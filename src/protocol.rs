//! Messages between the app and the voice engine: one JSON object per line, commands on the
//! engine's stdin and events on its stdout.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The engine executable, next to the app's own: Parakeet, and Whisper on the processor.
pub const ENGINE_EXE: &str = "vorto-engine.exe";
/// Whisper on the graphics card. whisper.cpp's Vulkan backend loads the Vulkan loader even to
/// run on the processor, so this engine only runs where a graphics driver installed it.
pub const GPU_ENGINE_EXE: &str = "vorto-engine-gpu.exe";

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    Load {
        root: PathBuf,
        model: String,
        #[serde(default = "default_gpu")]
        gpu: bool,
    },
    Transcribe {
        audio: PathBuf,
        language: String,
    },
    /// A quick look at the recording so far; failures never become errors.
    Preview {
        audio: PathBuf,
        language: String,
    },
    Download {
        root: PathBuf,
        model: String,
    },
}
fn default_gpu() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Progress {
        detail: String,
        fraction: f32,
    },
    Ready,
    Downloaded,
    Result {
        text: String,
    },
    /// `ok` is false when recognition failed; the audio was not recognized, not silent.
    Preview {
        text: String,
        ok: bool,
        /// Set when the request asked for detection, so later requests can reuse it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    Error {
        detail: String,
    },
}
