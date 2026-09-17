use crate::protocol::{Command, Event, ENGINE_EXE, GPU_ENGINE_EXE};
use anyhow::{bail, Context, Result};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command as Process, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Setup,
    Starting,
    Ready,
    Sleeping,
    Downloading,
    Transcribing,
    Error,
}
impl Phase {
    pub fn busy(self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Downloading | Self::Transcribing
        )
    }
    pub fn timeout(self) -> Option<Duration> {
        match self {
            Self::Starting => Some(Duration::from_secs(90)),
            Self::Transcribing => Some(Duration::from_secs(180)),
            Self::Downloading => Some(Duration::from_secs(60)),
            _ => None,
        }
    }
}
pub struct Supervisor {
    child: Option<Child>,
    input: Option<ChildStdin>,
    events: Option<Receiver<Event>>,
    pub phase: Phase,
    pub detail: String,
    pub progress: f32,
    last_progress: Instant,
}
impl Default for Supervisor {
    fn default() -> Self {
        Self {
            child: None,
            input: None,
            events: None,
            phase: Phase::Setup,
            detail: String::new(),
            progress: -1.0,
            last_progress: Instant::now(),
        }
    }
}
/// The error when the engine process ended on its own, for example in a crash.
pub const CRASHED: &str = "The voice model stopped unexpectedly.";

/// The prebuilt ONNX Runtime and whisper.cpp are compiled for these instruction sets. Without
/// them the engine would crash on its first instruction instead of explaining anything.
pub fn processor_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::is_x86_feature_detected as has;
        has!("avx")
            && has!("avx2")
            && has!("fma")
            && has!("f16c")
            && has!("bmi1")
            && has!("bmi2")
            && has!("lzcnt")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        true
    }
}

/// The graphics card engine is installed, and this PC has the Vulkan loader it needs.
pub fn gpu_engine_available() -> bool {
    let installed =
        std::env::current_exe().is_ok_and(|exe| exe.with_file_name(GPU_ENGINE_EXE).is_file());
    let loader = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .is_some_and(|windows| windows.join("System32").join("vulkan-1.dll").is_file());
    installed && loader
}

/// Engines belong to a job that Windows ends together with the app, however the app ends: a
/// download or recognition never keeps running on its own or locks files for an update.
#[cfg(windows)]
fn join_app_job(child: &Child) {
    use std::os::windows::io::AsRawHandle;
    use std::sync::OnceLock;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    static JOB: OnceLock<isize> = OnceLock::new();
    let job = *JOB.get_or_init(|| unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job != 0 {
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
        }
        // The handle stays open for the app's lifetime; Windows closes it when the app ends.
        job
    });
    if job != 0 {
        unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as isize) };
    }
}

impl Supervisor {
    /// Starts an engine; `gpu` picks the graphics card engine, which only runs Whisper.
    pub fn launch(&mut self, root: &std::path::Path, gpu: bool) -> Result<()> {
        self.stop();
        if !processor_supported() {
            bail!("This PC's processor isn't supported yet. Vorto needs a processor with AVX2, which most PCs made since 2015 have.");
        }
        let exe =
            std::env::current_exe()?.with_file_name(if gpu { GPU_ENGINE_EXE } else { ENGINE_EXE });
        if !exe.is_file() {
            bail!("The voice engine is missing. Reinstall Vorto.");
        }
        let path = root.join("engine.log");
        // Keep the file small like vorto.log: start fresh when it grows past 1 MB.
        // Safe to truncate because stop() has already waited for the previous worker.
        let fresh = path.metadata().is_ok_and(|m| m.len() > 1_000_000);
        let log = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(!fresh)
            .truncate(fresh)
            .open(&path)?;
        let mut command = Process::new(exe);
        command
            // Overlay layers of recording and monitoring tools are a common cause of Vulkan crashes.
            .env("VK_LOADER_LAYERS_DISABLE", "~implicit~")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command.spawn().context("The voice engine couldn't start")?;
        #[cfg(windows)]
        join_app_job(&child);
        let output = child.stdout.take().context("Engine output unavailable")?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            let mut line = Vec::new();
            // Native libraries may print to stdout too: skip what isn't an event, even invalid UTF-8.
            while matches!(reader.read_until(b'\n', &mut line), Ok(n) if n > 0) {
                if let Ok(event) =
                    serde_json::from_str::<Event>(String::from_utf8_lossy(&line).trim())
                {
                    if tx.send(event).is_err() {
                        break;
                    }
                }
                line.clear();
            }
        });
        self.input = child.stdin.take();
        self.child = Some(child);
        self.events = Some(rx);
        self.last_progress = Instant::now();
        Ok(())
    }
    pub fn send(&mut self, command: Command, phase: Phase) -> Result<()> {
        let input = self
            .input
            .as_mut()
            .context("The voice model isn't running.")?;
        writeln!(input, "{}", serde_json::to_string(&command)?)?;
        input.flush()?;
        self.phase = phase;
        self.last_progress = Instant::now();
        self.progress = -1.0;
        if matches!(phase, Phase::Starting | Phase::Downloading) {
            self.detail.clear();
        }
        Ok(())
    }
    pub fn poll(&mut self) -> Vec<Event> {
        let events: Vec<Event> = self
            .events
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for event in &events {
            self.last_progress = Instant::now();
            match event {
                Event::Progress { detail, fraction } => {
                    self.detail = detail.clone();
                    self.progress = *fraction;
                }
                Event::Ready => {
                    self.phase = Phase::Ready;
                    self.detail.clear();
                }
                Event::Downloaded => {
                    self.phase = Phase::Sleeping;
                }
                Event::Result { .. } => {
                    self.phase = Phase::Ready;
                }
                Event::Preview { .. } => {}
                Event::Error { detail } => {
                    self.phase = Phase::Error;
                    self.detail = detail.clone();
                }
            }
        }
        let mut events = events;
        let exited = self
            .child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten());
        if exited.is_some() && self.phase != Phase::Error {
            self.fail(CRASHED);
            events.push(Event::Error {
                detail: self.detail.clone(),
            });
        } else if self
            .phase
            .timeout()
            .is_some_and(|limit| self.last_progress.elapsed() > limit)
        {
            self.fail("The voice model took too long and was stopped.");
            events.push(Event::Error {
                detail: self.detail.clone(),
            });
        }
        events
    }
    pub fn fail(&mut self, message: &str) {
        self.stop();
        self.phase = Phase::Error;
        self.detail = message.into();
    }
    pub fn stop(&mut self) {
        self.input.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            // A process stuck in a driver call may take a moment to end. Don't wait forever:
            // the job ends it with the app at the latest.
            let started = Instant::now();
            while matches!(child.try_wait(), Ok(None)) && started.elapsed() < Duration::from_secs(3)
            {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        self.events = None;
    }
    pub fn sleep(&mut self) {
        self.stop();
        self.phase = Phase::Sleeping;
        self.detail.clear();
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_busy_state_has_a_deadline() {
        for p in [Phase::Starting, Phase::Transcribing, Phase::Downloading] {
            assert!(p.busy());
            assert!(p.timeout().is_some());
        }
    }
    #[test]
    fn recovery_leaves_an_actionable_error() {
        let mut s = Supervisor::default();
        s.fail("retry");
        assert_eq!(s.phase, Phase::Error);
        assert_eq!(s.detail, "retry");
        s.sleep();
        assert_eq!(s.phase, Phase::Sleeping);
    }
    #[test]
    fn startup_timeout_becomes_retryable() {
        let mut s = Supervisor::default();
        s.phase = Phase::Starting;
        s.last_progress = Instant::now() - Duration::from_secs(91);
        let events = s.poll();
        assert_eq!(s.phase, Phase::Error);
        assert!(matches!(events.first(), Some(Event::Error { .. })));
    }
    #[test]
    fn replacing_worker_drops_stale_ready_events() {
        let (tx, rx) = mpsc::channel();
        let mut s = Supervisor::default();
        s.events = Some(rx);
        s.phase = Phase::Starting;
        tx.send(Event::Ready).unwrap();
        s.sleep();
        assert!(s.poll().is_empty());
        assert_eq!(s.phase, Phase::Sleeping);
    }
}
