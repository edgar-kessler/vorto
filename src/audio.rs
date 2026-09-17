use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{FftFixedIn, Resampler};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Buffer {
    samples: Vec<f32>,
    level: f32,
    error: Option<String>,
}
pub struct Recorder {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Buffer>>,
    rate: u32,
}
pub fn devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|d| d.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}
impl Recorder {
    pub fn start(name: &str) -> Result<Self> {
        let host = cpal::default_host();
        let device = if name.is_empty() {
            host.default_input_device()
        } else {
            host.input_devices()?
                .find(|d| d.name().is_ok_and(|n| n == name))
        }
        .context("Microphone not found. Connect an input and choose it in Preferences.")?;
        let supported = device.default_input_config()?;
        let config: cpal::StreamConfig = supported.clone().into();
        let rate = config.sample_rate.0;
        let buffer = Arc::new(Mutex::new(Buffer::default()));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, buffer.clone())?,
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, buffer.clone())?,
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, buffer.clone())?,
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, buffer.clone())?,
            format => bail!("Unsupported microphone format: {format:?}"),
        };
        stream.play()?;
        Ok(Self {
            stream,
            buffer,
            rate,
        })
    }
    pub fn level(&self) -> f32 {
        self.buffer.lock().map(|b| b.level).unwrap_or(0.0)
    }
    /// A handle other threads can read the running recording through.
    pub fn tap(&self) -> Tap {
        Tap {
            buffer: self.buffer.clone(),
            rate: self.rate,
        }
    }
    pub fn seconds(&self) -> f32 {
        self.tap().seconds()
    }
    pub fn error(&self) -> Option<String> {
        self.buffer.lock().ok().and_then(|b| b.error.clone())
    }
    pub fn stop(self) -> Result<Vec<f32>> {
        self.stream.pause()?;
        drop(self.stream);
        let samples = {
            let mut buffer = self
                .buffer
                .lock()
                .map_err(|_| anyhow::anyhow!("Microphone buffer interrupted"))?;
            if let Some(error) = buffer.error.take() {
                bail!(error);
            }
            std::mem::take(&mut buffer.samples)
        };
        resample(&samples, self.rate)
    }
}

#[derive(Clone)]
pub struct Tap {
    buffer: Arc<Mutex<Buffer>>,
    rate: u32,
}
impl Tap {
    /// The last `seconds` of the recording so far, as 16 kHz mono, without stopping.
    pub fn snapshot(&self, seconds: f32) -> Result<Vec<f32>> {
        let tail = {
            let buffer = self
                .buffer
                .lock()
                .map_err(|_| anyhow::anyhow!("Microphone buffer interrupted"))?;
            let keep = (self.rate as f32 * seconds) as usize;
            buffer.samples[buffer.samples.len().saturating_sub(keep)..].to_vec()
        };
        resample(&tail, self.rate)
    }
    pub fn seconds(&self) -> f32 {
        self.len() as f32 / self.rate as f32
    }
    pub fn rate(&self) -> u32 {
        self.rate
    }
    /// Captured samples at the device rate.
    pub fn len(&self) -> usize {
        self.buffer.lock().map(|b| b.samples.len()).unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Device-rate samples `from..to`, converted to 16 kHz mono.
    pub fn segment(&self, from: usize, to: usize) -> Result<Vec<f32>> {
        let part = {
            let buffer = self
                .buffer
                .lock()
                .map_err(|_| anyhow::anyhow!("Microphone buffer interrupted"))?;
            let to = to.min(buffer.samples.len());
            buffer.samples[from.min(to)..to].to_vec()
        };
        resample(&part, self.rate)
    }
    /// Loudness of consecutive 30 ms frames in `from..to`, if that is longer than a pause.
    fn frames(&self, from: usize, to: usize) -> Option<(usize, Vec<f32>)> {
        let frame = (self.rate as usize * 30 / 1000).max(1);
        let buffer = self.buffer.lock().ok()?;
        let to = to.min(buffer.samples.len());
        if to <= from + frame * PAUSE_FRAMES {
            return None;
        }
        let energies = buffer.samples[from..to]
            .chunks(frame)
            .map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt())
            .collect();
        Some((frame, energies))
    }
    /// The middle of the latest 360 ms stretch between `from` and `to` that is quiet
    /// enough to be a pause between words.
    pub fn find_pause(&self, from: usize, to: usize) -> Option<usize> {
        let (frame, energies) = self.frames(from, to)?;
        let mut sorted = energies.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        // Measured from the background noise, so long pauses in a noisy room still count,
        // and kept below the speech in the same stretch. When there is hardly any speech,
        // quiet background noise is still a pause; steady loud sound never is.
        let floor = sorted[sorted.len() / 10];
        let speech = sorted[sorted.len() * 19 / 20];
        let threshold = (floor * 2.0)
            .min((speech * 0.5).max((floor * 1.5).min(0.02)))
            .max(0.004);
        let span = PAUSE_FRAMES;
        // The latest pause, so committed text grows as much as possible.
        (0..=energies.len() - span).rev().find_map(|start| {
            let loudest = energies[start..start + span]
                .iter()
                .cloned()
                .fold(0.0, f32::max);
            (loudest < threshold).then(|| from + (start + span / 2) * frame)
        })
    }
    /// The middle of the quietest 360 ms stretch between `from` and `to`, for when a
    /// segment has to be cut without a clear pause: it lands between words, not inside one.
    pub fn quietest(&self, from: usize, to: usize) -> usize {
        let Some((frame, energies)) = self.frames(from, to) else {
            return to.min(self.len()).max(from);
        };
        let span = PAUSE_FRAMES;
        let mut best = (f32::INFINITY, 0);
        for start in 0..=energies.len() - span {
            let loudest = energies[start..start + span]
                .iter()
                .cloned()
                .fold(0.0, f32::max);
            // Later wins a tie, like find_pause.
            if loudest <= best.0 {
                best = (loudest, start);
            }
        }
        from + (best.1 + span / 2) * frame
    }
}
/// A pause between words lasts at least this many 30 ms frames.
const PAUSE_FRAMES: usize = 12;

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Buffer>>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let cap = config.sample_rate.0 as usize * 120;
    let errors = buffer.clone();
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            if let Ok(mut b) = buffer.lock() {
                let mut energy = 0.0;
                let mut count = 0;
                for frame in data.chunks_exact(channels) {
                    let sample = frame
                        .iter()
                        .map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s))
                        .sum::<f32>()
                        / channels as f32;
                    let sample = if sample.is_finite() {
                        sample.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    };
                    if b.samples.len() < cap {
                        b.samples.push(sample);
                    }
                    energy += sample * sample;
                    count += 1;
                }
                b.level = (energy / count.max(1) as f32).sqrt();
            }
        },
        move |error| {
            if let Ok(mut b) = errors.lock() {
                b.error = Some(format!("Microphone disconnected or unavailable: {error}"));
            }
        },
        None,
    )?)
}
pub fn resample(input: &[f32], rate: u32) -> Result<Vec<f32>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    anyhow::ensure!(rate > 0, "Invalid sample rate");
    if rate == 16000 {
        return Ok(input.to_vec());
    }
    // FFT resampling is exact for fixed rates and many times faster than a long sinc filter.
    let mut resampler = FftFixedIn::<f32>::new(rate as usize, 16000, 1024, 2, 1)?;
    let delay = resampler.output_delay();
    let expected = (input.len() as u64 * 16000 / rate as u64) as usize;
    let mut output = Vec::new();
    for chunk in input.chunks(1024) {
        let block = if chunk.len() == 1024 {
            resampler.process(&[chunk], None)?
        } else {
            resampler.process_partial(Some(&[chunk]), None)?
        };
        output.extend_from_slice(&block[0]);
    }
    while output.len() < expected + delay {
        output.extend_from_slice(&resampler.process_partial::<&[f32]>(None, None)?[0]);
    }
    Ok(output[delay..delay + expected].to_vec())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resampling_preserves_duration() {
        for rate in [44100, 48000, 96000] {
            let samples = vec![0.1; rate];
            let out = resample(&samples, rate as u32).unwrap();
            assert_eq!(out.len(), 16000);
            assert!((out[8000] - 0.1).abs() < 0.001);
        }
    }
    #[test]
    fn pauses_are_found_between_words() {
        let rate = 16000;
        let mut samples: Vec<f32> = (0..rate * 2)
            .map(|i| (i as f32 * 0.07).sin() * 0.3)
            .collect();
        let pause_start = samples.len();
        samples.extend(std::iter::repeat_n(0.0, rate / 2));
        samples.extend((0..rate * 2).map(|i| (i as f32 * 0.05).sin() * 0.3));
        let tap = Tap {
            buffer: Arc::new(Mutex::new(Buffer {
                samples,
                ..Default::default()
            })),
            rate: rate as u32,
        };
        let cut = tap.find_pause(0, tap.len()).expect("pause");
        assert!(cut > pause_start && cut < pause_start + rate / 2);
        assert!(tap.find_pause(0, pause_start).is_none());
    }
    #[test]
    fn pauses_are_found_over_background_noise() {
        // Slow dictation in a room at about -46 dBFS: more pause than speech.
        let rate = 16000;
        let mut seed = 1u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed as f32 / u32::MAX as f32 * 2.0 - 1.0) * 0.0087
        };
        let mut samples = Vec::new();
        let mut pauses = Vec::new();
        for _ in 0..3 {
            samples.extend((0..rate).map(|i| (i as f32 * 0.07).sin() * 0.1 + noise()));
            pauses.push(samples.len()..samples.len() + rate * 2);
            samples.extend((0..rate * 2).map(|_| noise()));
        }
        let tap = Tap {
            buffer: Arc::new(Mutex::new(Buffer {
                samples,
                ..Default::default()
            })),
            rate: rate as u32,
        };
        let cut = tap.find_pause(0, tap.len()).expect("pause");
        assert!(pauses.iter().any(|p| p.contains(&cut)));
        let quiet = tap.quietest(rate / 2, rate * 5);
        assert!(pauses.iter().any(|p| p.contains(&quiet)));
    }
    #[test]
    fn empty_capture_is_safe() {
        assert!(resample(&[], 48000).unwrap().is_empty());
    }
}
