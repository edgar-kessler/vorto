//! Vorto's voice engine. The app starts it, sends one JSON command per line on stdin and reads
//! events from stdout (see `vorto::protocol`). The app kills it when it hangs, and it exits on
//! its own when stdin closes.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod download;
#[cfg(feature = "parakeet")]
mod parakeet;
mod proxy;
mod system;
mod whisper;

use anyhow::{bail, Context, Result};
use std::{
    io::{BufRead, Write},
    path::Path,
};
use vorto::{
    data::{self, Family, Model},
    protocol::{Command, Event},
    provider::{AudioRequest, Cancellation, Transcript, TranscriptionProvider},
};

pub(crate) fn emit(event: Event) {
    if let Ok(json) = serde_json::to_string(&event) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{json}");
        let _ = out.flush();
    }
}

fn main() {
    system::safe_dll_search();
    system::report_crashes();
    system::prefer_speed();
    whisper::quiet_logs();
    if let Err(error) = serve() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn serve() -> Result<()> {
    let threads = system::inference_threads();
    let mut model: Option<Box<dyn TranscriptionProvider>> = None;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let outcome = (|| -> Result<()> {
            match serde_json::from_str::<Command>(&line)? {
                Command::Load {
                    root,
                    model: id,
                    gpu,
                } => {
                    // A failed load must not leave the previous model answering.
                    model = None;
                    let entry = data::model(&id).context("Unknown voice model")?;
                    if !data::installed(&root, &id) {
                        bail!("{} isn't downloaded yet.", entry.name);
                    }
                    let dir = data::model_dir(&root, &id);
                    let mut context = open(entry, &dir, gpu, threads)?;
                    if let Err(error) = warm_up(context.as_mut()) {
                        if !(gpu && entry.family == Family::Whisper) {
                            return Err(error);
                        }
                        emit(Event::Progress {
                            detail: "Graphics card unavailable. Switching to the processor".into(),
                            fraction: -1.0,
                        });
                        // Free the graphics card copy first, so the model isn't in memory twice.
                        drop(context);
                        context = open(entry, &dir, false, threads)?;
                        warm_up(context.as_mut())?;
                    }
                    model = Some(context);
                    emit(Event::Ready);
                }
                Command::Transcribe {
                    audio,
                    language,
                    prompt,
                } => {
                    let context = model.as_mut().context("The voice model isn't ready.")?;
                    let text = transcribe_file(context.as_mut(), &audio, language, prompt)?.text;
                    emit(Event::Result { text });
                }
                Command::Preview {
                    audio,
                    language,
                    prompt,
                } => {
                    let outcome = model
                        .as_mut()
                        .context("The voice model isn't ready.")
                        .and_then(|context| {
                            transcribe_file(context.as_mut(), &audio, language, prompt)
                        });
                    // Reported in the event rather than as an error, which would end the dictation.
                    emit(match outcome {
                        Ok(transcript) => Event::Preview {
                            text: transcript.text,
                            ok: true,
                            language: transcript.language,
                        },
                        Err(error) => {
                            eprintln!("Preview failed: {error:#}");
                            Event::Preview {
                                text: String::new(),
                                ok: false,
                                language: None,
                            }
                        }
                    });
                }
                Command::Download { root, model } => download::download(&root, &model)?,
            }
            Ok(())
        })();
        if let Err(error) = outcome {
            emit(Event::Error {
                detail: format!("{error:#}"),
            });
        }
    }
    Ok(())
}

fn open(
    entry: &Model,
    dir: &Path,
    gpu: bool,
    threads: usize,
) -> Result<Box<dyn TranscriptionProvider>> {
    Ok(match entry.family {
        Family::Whisper => {
            let path = dir.join(entry.files[0].name);
            match whisper::LocalWhisper::load(&path, gpu, threads) {
                Ok(context) => Box::new(context),
                Err(_) if gpu => Box::new(whisper::LocalWhisper::load(&path, false, threads)?),
                Err(error) => return Err(error),
            }
        }
        #[cfg(feature = "parakeet")]
        Family::Parakeet => Box::new(parakeet::Parakeet::load(dir, threads)?),
        #[cfg(not(feature = "parakeet"))]
        Family::Parakeet => bail!("This voice engine can't run {}.", entry.name),
    })
}

/// The first run pays for kernel compilation and allocations; do it before the user speaks.
fn warm_up(provider: &mut dyn TranscriptionProvider) -> Result<()> {
    provider
        .transcribe(
            AudioRequest {
                samples: vec![0.0; 16_000],
                language: Some("en".into()),
                prompt: String::new(),
                allow_remote: false,
            },
            Cancellation::default(),
        )
        .map(|_| ())
}

fn transcribe_file(
    provider: &mut dyn TranscriptionProvider,
    audio: &Path,
    language: String,
    prompt: String,
) -> Result<Transcript> {
    let mut reader = hound::WavReader::open(audio)?;
    let spec = reader.spec();
    if spec.sample_rate != 16000 || spec.channels != 1 {
        bail!("Expected 16 kHz mono audio");
    }
    let samples: Vec<f32> = reader
        .samples::<f32>()
        .collect::<std::result::Result<_, _>>()?;
    // The app stops recording at two minutes; the margin covers its timing.
    if samples.len() > 125 * 16_000 {
        bail!("Recording exceeds the two-minute limit");
    }
    provider.transcribe(
        AudioRequest {
            samples,
            language: if language == "auto" {
                None
            } else {
                Some(language)
            },
            prompt,
            allow_remote: false,
        },
        Cancellation::default(),
    )
}
