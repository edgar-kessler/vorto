//! OpenAI Whisper through whisper.cpp, on the graphics card (Vulkan) when the build and the PC
//! allow it, otherwise on the processor.
use anyhow::{Context, Result};
use std::{io::Write, path::Path};
use vorto::provider::{
    check_transfer, AudioRequest, Cancellation, ProcessingLocation, ProviderInfo, Transcript,
    TranscriptionProvider,
};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub struct LocalWhisper {
    /// Its buffers are allocated once: recreating them for every preview is slow, above all
    /// on the graphics card. It keeps the model loaded.
    state: WhisperState,
    threads: usize,
}
impl LocalWhisper {
    pub fn load(path: &Path, gpu: bool, threads: usize) -> Result<Self> {
        let mut options = WhisperContextParameters::default();
        options.use_gpu(gpu && gpu_available());
        let context =
            WhisperContext::new_with_params(path.to_str().context("Invalid model path")?, options)?;
        Ok(Self {
            state: context.create_state()?,
            threads,
        })
    }
}

/// This build has the graphics backend, and this PC has the Vulkan loader it needs.
pub fn gpu_available() -> bool {
    cfg!(feature = "vulkan") && crate::system::vulkan_loader_present()
}

impl TranscriptionProvider for LocalWhisper {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "whisper-local".into(),
            label: "Local Whisper".into(),
            location: ProcessingLocation::OnDevice,
        }
    }
    fn transcribe(&mut self, request: AudioRequest, cancel: Cancellation) -> Result<Transcript> {
        check_transfer(&self.info(), &request)?;
        anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
        let transcript = transcribe(
            &mut self.state,
            &request.samples,
            request.language.as_deref().unwrap_or("auto"),
            self.threads,
            &cancel,
        )?;
        anyhow::ensure!(!cancel.is_cancelled(), "Transcription cancelled");
        Ok(transcript)
    }
}

fn transcribe(
    state: &mut WhisperState,
    audio: &[f32],
    language: &str,
    threads: usize,
    cancel: &Cancellation,
) -> Result<Transcript> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(threads as i32);
    params.set_language(if language == "auto" {
        None
    } else {
        Some(language)
    });
    params.set_translate(false);
    params.set_no_context(true);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);
    unsafe extern "C" fn cancelled(data: *mut std::ffi::c_void) -> bool {
        // The Arc owner stays alive for the entire synchronous full() call.
        unsafe {
            (*(data as *const std::sync::atomic::AtomicBool))
                .load(std::sync::atomic::Ordering::Acquire)
        }
    }
    unsafe {
        params.set_abort_callback(Some(cancelled));
        params.set_abort_callback_user_data(
            std::sync::Arc::as_ptr(&cancel.0) as *mut std::ffi::c_void
        );
    }
    state.full(params, audio)?;
    let mut text = String::new();
    for segment in state.as_iter() {
        text.push_str(&segment.to_str_lossy()?);
    }
    let text = text.trim().to_owned();
    // A language guessed from silence must not decide the rest of the dictation.
    let detected = (language == "auto" && !text.is_empty())
        .then(|| whisper_rs::get_lang_str(state.full_lang_id_from_state()))
        .flatten()
        .map(str::to_owned);
    Ok(Transcript {
        text,
        language: detected,
    })
}

/// whisper.cpp and ggml describe the model, its path and the graphics device on every load
/// and every recognition. engine.log keeps only their warnings and errors.
unsafe extern "C" fn log_problems(
    level: whisper_rs::whisper_rs_sys::ggml_log_level,
    text: *const std::ffi::c_char,
    _: *mut std::ffi::c_void,
) {
    use whisper_rs::whisper_rs_sys as sys;
    let problem = level == sys::ggml_log_level_GGML_LOG_LEVEL_WARN
        || level == sys::ggml_log_level_GGML_LOG_LEVEL_ERROR;
    if problem && !text.is_null() {
        let text = unsafe { std::ffi::CStr::from_ptr(text) };
        let _ = std::io::stderr().write_all(text.to_bytes());
    }
}

pub fn quiet_logs() {
    // The callback never unwinds and uses no user data.
    unsafe { whisper_rs::set_log_callback(Some(log_problems), std::ptr::null_mut()) };
}
