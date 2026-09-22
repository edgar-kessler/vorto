//! Short feedback sounds, synthesized once at first use: a mechanical key switch going down when
//! dictation starts and coming back up when it ends, a bright two-note chime when the text is in
//! place, and softer tones for cancel and errors. No sound files ship with Vorto.
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

/// A switch: the bright click of the contact, the hollow body of the case and the low thock
/// of the keycap bottoming out. `pitch` > 1 is lighter, like the upstroke.
fn switch(out: &mut [f32], at: f32, pitch: f32, gain: f32, seed: u32) {
    let mut noise = Noise(seed);
    let mut hp = 0.0;
    let mut last = 0.0;
    let start = (at * RATE as f32) as usize;
    for (i, sample) in out.iter_mut().enumerate().skip(start) {
        let x = t(i - start);
        // Contact: high-passed noise, over in a few milliseconds.
        let n = noise.next();
        hp = 0.72 * (hp + n - last);
        last = n;
        let contact = hp * (-x / 0.0022).exp();
        // Case resonance and a second, slightly detuned mode for a plastic, "creamy" color.
        let body = (TAU * 2350.0 * pitch * x).sin() * (-x / 0.012).exp() * 0.55
            + (TAU * 3710.0 * pitch * x).sin() * (-x / 0.006).exp() * 0.3;
        // Bottom-out thock, falling in pitch like a real keycap.
        let f = 170.0 * pitch * (1.0 + 0.6 * (-x / 0.01).exp());
        let thock = (TAU * f * x).sin() * (-x / 0.028).exp() * 0.9;
        *sample += gain * (contact * 0.9 + body * 0.5 + thock * (2.0 - pitch));
    }
}

/// A soft sine with a touch of its octave, gliding from `f0` to `f1`.
fn tone(out: &mut [f32], at: f32, f0: f32, f1: f32, len: f32, gain: f32) {
    let start = (at * RATE as f32) as usize;
    let mut phase = 0.0;
    for (i, sample) in out.iter_mut().enumerate().skip(start) {
        let x = t(i - start);
        if x > len * 3.0 {
            break;
        }
        let k = (x / len).min(1.0);
        let f = f0 + (f1 - f0) * (1.0 - (1.0 - k).powi(3));
        phase += TAU * f / RATE as f32;
        let attack = (x / 0.004).min(1.0);
        let env = attack * (-x / len).exp();
        *sample += gain * env * (phase.sin() + 0.18 * (2.0 * phase).sin());
    }
}

fn press() -> Vec<f32> {
    let mut out = buffer(160);
    switch(&mut out, 0.0, 1.0, 0.55, 7);
    // A small rising blip: the microphone is on.
    tone(&mut out, 0.012, 740.0, 1180.0, 0.045, 0.16);
    out
}

fn release() -> Vec<f32> {
    let mut out = buffer(150);
    switch(&mut out, 0.0, 1.35, 0.42, 11);
    tone(&mut out, 0.01, 1180.0, 820.0, 0.04, 0.13);
    out
}

fn done() -> Vec<f32> {
    let mut out = buffer(420);
    // A major sixth, bright and short: "there it is".
    tone(&mut out, 0.0, 1318.5, 1318.5, 0.07, 0.2);
    tone(&mut out, 0.075, 2217.5, 2217.5, 0.11, 0.17);
    out
}

fn cancel() -> Vec<f32> {
    let mut out = buffer(220);
    tone(&mut out, 0.0, 520.0, 300.0, 0.06, 0.2);
    out
}

fn error() -> Vec<f32> {
    let mut out = buffer(380);
    tone(&mut out, 0.0, 392.0, 392.0, 0.06, 0.2);
    tone(&mut out, 0.11, 311.1, 311.1, 0.09, 0.2);
    out
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
}
