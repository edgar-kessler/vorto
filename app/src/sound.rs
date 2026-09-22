//! Short feedback sounds, synthesized once at first use, modeled on a clicky (blue) key switch:
//! it goes down when dictation starts and comes back up when it ends, a quiet tick says the
//! text is in place, and dull keystrokes mean cancel or error. No tones, no sound files.
use std::{f32::consts::TAU, sync::OnceLock};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

const RATE: u32 = 48_000;

#[derive(Clone, Copy)]
pub enum Sound {
    Press,
    Release,
    Done,
    Cancel,
    Error,
}

/// Plays without waiting. A new sound cuts off the one still playing.
pub fn play(sound: Sound) {
    static WAVS: OnceLock<[Vec<u8>; 5]> = OnceLock::new();
    let wavs = WAVS.get_or_init(|| {
        [
            wav(&press()),
            wav(&release()),
            wav(&done()),
            wav(&cancel()),
            wav(&error()),
        ]
    });
    let bytes = &wavs[sound as usize];
    // The bytes live as long as the process, as SND_ASYNC with SND_MEMORY requires.
    unsafe {
        PlaySoundW(
            bytes.as_ptr().cast(),
            0,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

fn buffer(ms: u32) -> Vec<f32> {
    vec![0.0; (RATE * ms / 1000) as usize]
}

fn t(i: usize) -> f32 {
    i as f32 / RATE as f32
}

/// Deterministic white noise, so every click sounds the same.
struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1 << 23) as f32 - 1.0
    }
}

/// A burst of noise through a band-pass filter: the snap of plastic on plastic. `hz` sets how
/// bright it is, `decay` how long it rings, in seconds.
fn burst(out: &mut [f32], at: f32, hz: f32, q: f32, decay: f32, gain: f32, seed: u32) {
    // RBJ band-pass with 0 dB peak gain.
    let w = TAU * hz / RATE as f32;
    let alpha = w.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    let (b0, b2) = (alpha / a0, -alpha / a0);
    let (a1, a2) = (-2.0 * w.cos() / a0, (1.0 - alpha) / a0);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    let mut noise = Noise(seed);
    let start = (at * RATE as f32) as usize;
    for (i, sample) in out.iter_mut().enumerate().skip(start) {
        let x = t(i - start);
        if x > decay * 12.0 {
            break;
        }
        let n = noise.next();
        let y = b0 * n + b2 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, n, y1, y);
        let attack = (x / 0.00015).min(1.0);
        *sample += gain * attack * (-x / decay).exp() * y;
    }
}

/// A damped resonance of the switch housing or keycap.
fn ring(out: &mut [f32], at: f32, hz: f32, decay: f32, gain: f32) {
    let start = (at * RATE as f32) as usize;
    for (i, sample) in out.iter_mut().enumerate().skip(start) {
        let x = t(i - start);
        if x > decay * 12.0 {
            break;
        }
        *sample += gain * (TAU * hz * x).sin() * (-x / decay).exp();
    }
}

/// Scales a sound so its loudest sample is `peak`.
fn level(mut out: Vec<f32>, peak: f32) -> Vec<f32> {
    let max = out.iter().fold(0f32, |m, s| m.max(s.abs())).max(1e-6);
    for s in &mut out {
        *s *= peak / max;
    }
    out
}

/// A clicky switch going down: the click jacket snaps, bright and very short, then the stem
/// bottoms out a moment later with a lower plastic clack.
fn press() -> Vec<f32> {
    let mut out = buffer(90);
    burst(&mut out, 0.0, 4300.0, 1.4, 0.0011, 1.0, 7);
    burst(&mut out, 0.0, 7600.0, 1.8, 0.0006, 0.45, 13);
    ring(&mut out, 0.0, 3150.0, 0.0035, 0.18);
    burst(&mut out, 0.012, 1250.0, 1.0, 0.0028, 0.6, 29);
    ring(&mut out, 0.012, 540.0, 0.0055, 0.22);
    ring(&mut out, 0.012, 1900.0, 0.0035, 0.1);
    level(out, 0.62)
}

/// The same switch coming back up: a second, lighter click and the soft top-out.
fn release() -> Vec<f32> {
    let mut out = buffer(80);
    burst(&mut out, 0.0, 4900.0, 1.4, 0.0009, 1.0, 11);
    ring(&mut out, 0.0, 3400.0, 0.003, 0.14);
    burst(&mut out, 0.007, 1700.0, 1.0, 0.0022, 0.4, 31);
    ring(&mut out, 0.007, 720.0, 0.0045, 0.14);
    level(out, 0.45)
}

/// The text is in place: one quiet tick, no melody.
fn done() -> Vec<f32> {
    let mut out = buffer(60);
    burst(&mut out, 0.0, 3900.0, 1.3, 0.0008, 1.0, 17);
    ring(&mut out, 0.0, 950.0, 0.0035, 0.12);
    level(out, 0.22)
}

/// Discarded: a dull keystroke without the click.
fn cancel() -> Vec<f32> {
    let mut out = buffer(90);
    burst(&mut out, 0.0, 950.0, 0.9, 0.004, 1.0, 19);
    ring(&mut out, 0.0, 330.0, 0.009, 0.35);
    level(out, 0.4)
}

/// Something went wrong: two dull keystrokes.
fn error() -> Vec<f32> {
    let mut out = buffer(190);
    for (at, seed) in [(0.0, 23), (0.085, 37)] {
        burst(&mut out, at, 900.0, 0.9, 0.004, 1.0, seed);
        ring(&mut out, at, 310.0, 0.009, 0.35);
    }
    level(out, 0.42)
}

/// 16-bit mono PCM in a RIFF container, with a short fade so nothing ends in a click.
fn wav(samples: &[f32]) -> Vec<u8> {
    let fade = (RATE / 200) as usize;
    let len = samples.len();
    let pcm: Vec<i16> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let tail = ((len - i) as f32 / fade as f32).min(1.0);
            (s.tanh() * tail * i16::MAX as f32) as i16
        })
        .collect();
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_are_valid_and_never_clip() {
        for samples in [press(), release(), done(), cancel(), error()] {
            let bytes = wav(&samples);
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(bytes.len(), 44 + samples.len() * 2);
            let peak = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.05 && peak < 1.0, "peak {peak}");
        }
    }

    /// Writes the sounds as WAV files to listen to:
    /// `VORTO_SOUND_DIR=<folder> cargo test -p vorto-app -- --ignored write_sounds`
    #[test]
    #[ignore]
    fn write_sounds() {
        let dir =
            std::path::PathBuf::from(std::env::var("VORTO_SOUND_DIR").expect("VORTO_SOUND_DIR"));
        for (name, samples) in [
            ("press", press()),
            ("release", release()),
            ("done", done()),
            ("cancel", cancel()),
            ("error", error()),
        ] {
            std::fs::write(dir.join(format!("{name}.wav")), wav(&samples)).unwrap();
        }
    }
}
