//! Integration seam for separately distributed local or online providers.
//! No account, billing, credentials, or cloud implementation belongs in this crate.
use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessingLocation {
    OnDevice,
    Remote,
}

#[derive(Clone, Debug)]
pub struct ProviderInfo {
    pub id: String,
    pub label: String,
    pub location: ProcessingLocation,
}
#[derive(Clone, Default)]
pub struct Cancellation(pub Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
pub struct AudioRequest {
    /// Normalized 16 kHz mono PCM. Providers must not retain it after completion.
    pub samples: Vec<f32>,
    pub language: Option<String>,
    /// Remote providers must reject requests without explicit transfer consent.
    pub allow_remote: bool,
}
pub struct Transcript {
    pub text: String,
    /// The language the provider detected, when it was asked to detect one.
    pub language: Option<String>,
}
pub trait TranscriptionProvider: Send {
    fn info(&self) -> ProviderInfo;
    /// Call off the UI thread; implementations must honor cancellation.
    fn transcribe(&mut self, request: AudioRequest, cancel: Cancellation) -> Result<Transcript>;
}
pub fn check_transfer(info: &ProviderInfo, request: &AudioRequest) -> Result<()> {
    anyhow::ensure!(
        info.location != ProcessingLocation::Remote || request.allow_remote,
        "Online transcription needs your permission to send audio."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_audio_requires_explicit_consent() {
        let p = ProviderInfo {
            id: "test".into(),
            label: "test".into(),
            location: ProcessingLocation::Remote,
        };
        let mut r = AudioRequest {
            samples: vec![],
            language: None,
            allow_remote: false,
        };
        assert!(check_transfer(&p, &r).is_err());
        r.allow_remote = true;
        assert!(check_transfer(&p, &r).is_ok());
    }
}
