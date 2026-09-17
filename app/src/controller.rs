//! The single owner of app state. It runs on its own thread, receives hotkey events and UI
//! actions, and publishes what the two windows render: the full state for the main window
//! while it is on screen, and a small one for the recording pill.
use crate::{
    hotkey::{self, HookEvent},
    native::{self, Target},
    update::{self, UpdateMsg, UpdateView},
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        mpsc::{Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use vorto::{
    audio::{self, Recorder},
    data::{self, Family, Settings, Store},
    protocol::{Command, Event},
    supervisor::{Phase, Supervisor},
};
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

/// Recording stops on its own after this long.
const MAX_RECORDING_SECS: u64 = 120;
/// How often Vorto looks for a new version while it runs.
const UPDATE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    StartDictation,
    StopDictation,
    Cancel,
    Download {
        id: String,
    },
    UseModel {
        id: String,
    },
    Retry,
    UseCpu,
    SaveSettings {
        settings: Settings,
    },
    RecordShortcut,
    StopRecordingShortcut,
    /// Shortcut recorded by the window itself, used when the hook does not report it.
    SetShortcut {
        keys: Vec<u32>,
    },
    ClearHistory,
    Copy {
        text: String,
    },
    OpenDataFolder,
    RefreshMicrophones,
    TestMicrophone {
        on: bool,
    },
    SetAutostart {
        on: bool,
    },
    CheckForUpdates,
    InstallUpdate,
}

impl Action {
    fn name(&self) -> &'static str {
        match self {
            Action::StartDictation => "startDictation",
            Action::StopDictation => "stopDictation",
            Action::Cancel => "cancel",
            Action::Download { .. } => "download",
            Action::UseModel { .. } => "useModel",
            Action::Retry => "retry",
            Action::UseCpu => "useCpu",
            Action::SaveSettings { .. } => "saveSettings",
            Action::RecordShortcut => "recordShortcut",
            Action::StopRecordingShortcut => "stopRecordingShortcut",
            Action::SetShortcut { .. } => "setShortcut",
            Action::ClearHistory => "clearHistory",
            Action::Copy { .. } => "copy",
            Action::OpenDataFolder => "openDataFolder",
            Action::RefreshMicrophones => "refreshMicrophones",
            Action::TestMicrophone { .. } => "testMicrophone",
            Action::SetAutostart { .. } => "setAutostart",
            Action::CheckForUpdates => "checkForUpdates",
            Action::InstallUpdate => "installUpdate",
        }
    }
}

pub enum Msg {
    Hook(HookEvent),
    Action(Action),
    Inserted {
        outcome: native::Outcome,
        text: String,
    },
    /// Audio prepared while recording, with the dictation it was prepared for.
    PreviewReady(u64, Option<Prepared>),
    /// The main window was shown or hidden (or minimized).
    MainWindow {
        visible: bool,
    },
    Update(UpdateMsg),
}

/// The last snapshot of each window, for a page that loads or reloads.
#[derive(Default)]
pub struct Published {
    main: String,
    hud: String,
}

pub struct Handle {
    pub tx: Mutex<Sender<Msg>>,
    pub published: Arc<Mutex<Published>>,
}

#[tauri::command]
pub fn get_state(handle: tauri::State<Handle>, window: tauri::WebviewWindow) -> Box<RawValue> {
    let json = handle
        .published
        .lock()
        .map(|p| {
            if window.label() == "hud" {
                p.hud.clone()
            } else {
                p.main.clone()
            }
        })
        .unwrap_or_default();
    RawValue::from_string(json)
        .unwrap_or_else(|_| RawValue::from_string("null".into()).expect("valid JSON"))
}
#[tauri::command]
pub fn action(handle: tauri::State<Handle>, action: Action) {
    if let Ok(tx) = handle.tx.lock() {
        let _ = tx.send(Msg::Action(action));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelView {
    id: &'static str,
    name: &'static str,
    family: &'static str,
    languages: &'static str,
    download_mb: u32,
    memory_mb: u32,
    speed: u8,
    accuracy: u8,
    hardware: &'static str,
    installed: bool,
    recommended: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot<'a> {
    phase: &'static str,
    detail: &'a str,
    progress: f32,
    downloading: Option<&'a str>,
    download_error: Option<&'a DownloadError>,
    recording: bool,
    recording_since: u64,
    recording_limit: u64,
    dictating_here: bool,
    /// Words recognized so far in a dictation into the window.
    live: &'a str,
    dictations: u64,
    settings: &'a Settings,
    shortcut: Vec<String>,
    capturing: bool,
    hook_ok: bool,
    models: Vec<ModelView>,
    history: &'a [data::Entry],
    words_today: usize,
    latest: &'a str,
    notice: &'a Notice,
    microphones: &'a [String],
    data_dir: String,
    version: &'static str,
    /// The graphics card engine can run on this PC.
    gpu_build: bool,
    mic_test: bool,
    autostart: bool,
    /// Icons of the apps in History. Sorted, so an unchanged state serializes the same way.
    app_icons: BTreeMap<&'a str, &'a str>,
    update: &'a UpdateView,
}

/// Only what the recording pill shows.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HudSnapshot<'a> {
    hud: HudView<'a>,
    recording: bool,
    recording_since: u64,
    recording_limit: u64,
    settings: HudSettings<'a>,
    target: Option<&'a Target>,
}
/// The pill's copy of `Hud`, with words only while it shows.
#[derive(Serialize)]
struct HudView<'a> {
    mode: &'static str,
    text: &'a str,
    live: &'a str,
}
#[derive(Serialize)]
struct HudSettings<'a> {
    hud_position: &'a str,
}

#[derive(Serialize, Clone, Default)]
struct Notice {
    id: u64,
    text: String,
    kind: &'static str,
    /// What the notice is about, so a screen that already shows it can skip the toast:
    /// "", "engine", "download" or "downloaded".
    topic: &'static str,
}
#[derive(Serialize)]
struct DownloadError {
    id: &'static str,
    text: String,
}
#[derive(Serialize, Clone, Default, PartialEq)]
struct Hud {
    /// hidden, listening, writing, done, notice or error
    mode: &'static str,
    text: String,
    /// Words recognized so far while speaking.
    live: String,
}

/// A recognition request sent while recording.
pub enum Draft {
    Preview,
    Commit { end: usize },
}
pub struct Prepared {
    file: tempfile::NamedTempFile,
    draft: Draft,
    seconds: f32,
}
struct Flight {
    // Kept alive until the engine has read it.
    _file: tempfile::NamedTempFile,
    draft: Draft,
    /// Replies for an earlier dictation are ignored.
    dictation: u64,
    sent: Instant,
    seconds: f32,
}

/// Whether any 30 ms of 16 kHz audio is loud enough to hold speech.
fn voiced(samples: &[f32]) -> bool {
    samples
        .chunks(480)
        .any(|c| c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32 >= 0.006 * 0.006)
}

/// One insertion at a time, so a clipboard restore never lands inside the next paste.
static INSERTING: Mutex<()> = Mutex::new(());

fn join_text(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a} {b}"),
    }
}

fn write_wav(samples: &[f32]) -> anyhow::Result<tempfile::NamedTempFile> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_DELETE_ON_CLOSE,
    };
    // Windows deletes the recording when the last handle closes, even if Vorto is killed.
    let file = tempfile::Builder::new()
        .prefix("vorto-")
        .suffix(".wav")
        .make(|path| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .custom_flags(FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE)
                .open(path)
        })?;
    let mut writer = hound::WavWriter::new(
        file.reopen()?,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(file)
}

struct Pending {
    /// The whole recording at 16 kHz, until the part still to recognize is known.
    samples: Vec<f32>,
    /// Device sample rate, which `committed_end` counts in.
    rate: u64,
    file: Option<tempfile::NamedTempFile>,
    seconds: f32,
    /// Length of the part the final pass recognizes.
    tail_seconds: f32,
}

pub struct Controller {
    app: AppHandle,
    tx: Sender<Msg>,
    published: Arc<Mutex<Published>>,
    store: Store,
    engine: Supervisor,
    recorder: Option<Recorder>,
    started: Instant,
    started_epoch: u64,
    pending: Option<Pending>,
    target_hwnd: isize,
    target: Option<Target>,
    last_foreign: isize,
    latest: String,
    notice: Notice,
    hud: Hud,
    hud_until: Option<Instant>,
    hud_hide_at: Option<Instant>,
    overlay: (isize, (i32, i32)),
    downloading: Option<&'static str>,
    download_error: Option<DownloadError>,
    microphones: Vec<String>,
    capturing: bool,
    last_used: Instant,
    last_main: String,
    last_hud: String,
    last_level: Instant,
    mic_test: Option<(Recorder, Instant)>,
    autostart: bool,
    flight: Option<Flight>,
    preview_preparing: bool,
    /// Text of finished sentences recognized while the user is still speaking.
    committed: String,
    /// Device-rate sample where `committed` ends.
    committed_end: usize,
    /// The language Whisper heard earlier in this dictation, so a short final part in auto
    /// mode isn't taken for another language.
    detected_language: Option<String>,
    last_preview: Instant,
    stopped_at: Instant,
    /// Counts dictations, so late replies for one never change the next.
    dictation: u64,
    /// Dictations delivered since launch.
    dictations: u64,
    /// The current dictation goes into the open Vorto window rather than another app.
    here: bool,
    /// Previews took longer than they save, for example Whisper that fell back to the processor.
    slow_engine: bool,
    /// A model or graphics change is waiting for the engine to be free.
    reload_needed: bool,
    /// The graphics card crashed the engine this session: Whisper stays on the processor.
    gpu_failed: bool,
    /// The running engine is the graphics card one. Settings can change while it works.
    engine_gpu: bool,
    /// Insertions into other apps not finished yet.
    inserting: u32,
    /// Loading waits until then after a start at sign-in, unless a dictation needs the model.
    load_at: Option<Instant>,
    models_installed: Vec<bool>,
    installed_checked: Instant,
    app_icons: HashMap<String, String>,
    main_visible: bool,
    /// Whisper can run on the graphics card here: see `supervisor::gpu_engine_available`.
    gpu_engine: bool,
    update: UpdateView,
    update_found: Option<Box<tauri_plugin_updater::Update>>,
    update_bytes: Option<Vec<u8>>,
    /// Install as soon as the download finishes: the user asked for it.
    install_when_ready: bool,
    next_update_check: Instant,
}

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Controller {
    pub fn new(
        app: AppHandle,
        tx: Sender<Msg>,
        published: Arc<Mutex<Published>>,
        store: Store,
        overlay: (isize, (i32, i32)),
        hidden: bool,
    ) -> Self {
        let mut controller = Self {
            app,
            tx,
            published,
            notice: Notice {
                id: 0,
                text: store.warning.clone().unwrap_or_default(),
                kind: "info",
                topic: "",
            },
            store,
            engine: Supervisor::default(),
            recorder: None,
            started: Instant::now(),
            started_epoch: 0,
            pending: None,
            target_hwnd: 0,
            target: None,
            last_foreign: 0,
            latest: String::new(),
            hud: Hud {
                mode: "hidden",
                text: String::new(),
                live: String::new(),
            },
            hud_until: None,
            hud_hide_at: None,
            overlay,
            downloading: None,
            download_error: None,
            microphones: audio::devices(),
            capturing: false,
            last_used: Instant::now(),
            last_main: String::new(),
            last_hud: String::new(),
            last_level: Instant::now(),
            mic_test: None,
            autostart: native::autostart_enabled(),
            flight: None,
            preview_preparing: false,
            committed: String::new(),
            committed_end: 0,
            detected_language: None,
            last_preview: Instant::now(),
            stopped_at: Instant::now(),
            dictation: 0,
            dictations: 0,
            here: false,
            slow_engine: false,
            reload_needed: false,
            gpu_failed: false,
            engine_gpu: false,
            inserting: 0,
            load_at: None,
            models_installed: Vec::new(),
            installed_checked: Instant::now(),
            app_icons: HashMap::new(),
            main_visible: !hidden,
            gpu_engine: vorto::supervisor::gpu_engine_available(),
            update: UpdateView::default(),
            update_found: None,
            update_bytes: None,
            install_when_ready: false,
            next_update_check: Instant::now() + Duration::from_secs(20),
        };
        controller.refresh_installed();
        if controller.installed() {
            if hidden {
                // Started with Windows: let sign-in finish before reading the model from disk.
                controller.load_at = Some(Instant::now() + Duration::from_secs(30));
            } else {
                controller.load();
            }
        }
        controller
    }

    pub fn run(mut self, rx: Receiver<Msg>) {
        self.publish(true);
        let mut published = Instant::now();
        loop {
            // Recording needs level updates and previews; otherwise nothing changes quickly.
            let wait = if self.recorder.is_some() || self.mic_test.is_some() {
                15
            } else if self.active() {
                40
            } else {
                200
            };
            let mut handled = match rx.recv_timeout(Duration::from_millis(wait)) {
                Ok(msg) => {
                    self.handle(msg);
                    true
                }
                Err(RecvTimeoutError::Timeout) => false,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            while let Ok(msg) = rx.try_recv() {
                self.handle(msg);
                handled = true;
            }
            let was_active = self.active();
            self.tick();
            let due = if was_active || self.active() { 50 } else { 500 };
            if handled || published.elapsed() >= Duration::from_millis(due) {
                self.publish(false);
                published = Instant::now();
            }
        }
    }

    /// Something on screen may change from one pass to the next.
    fn active(&self) -> bool {
        self.recorder.is_some()
            || self.pending.is_some()
            || self.flight.is_some()
            || self.engine.phase.busy()
            || self.mic_test.is_some()
            || self.capturing
            || self.hud.mode != "hidden"
            || self.hud_hide_at.is_some()
            || matches!(self.update.status, "checking" | "downloading")
    }
    fn installed(&self) -> bool {
        data::installed(&self.store.root, &self.store.settings.model)
    }
    fn refresh_installed(&mut self) {
        self.models_installed = data::MODELS
            .iter()
            .map(|m| data::installed(&self.store.root, m.id))
            .collect();
        self.installed_checked = Instant::now();
    }
    fn set_target(&mut self, hwnd: isize) {
        self.target = native::target(hwnd);
        if self
            .target
            .as_ref()
            .is_some_and(|t| !self.app_icons.contains_key(&t.name))
        {
            self.app_icons = native::known_icons();
        }
    }
    fn say(&mut self, kind: &'static str, text: impl Into<String>) {
        self.say_about("", kind, text)
    }
    fn say_about(&mut self, topic: &'static str, kind: &'static str, text: impl Into<String>) {
        self.notice = Notice {
            id: self.notice.id + 1,
            text: text.into(),
            kind,
            topic,
        };
    }
    /// The pill shows dictations into other apps. Dictations into the open Vorto window show on
    /// its Dictate page instead, except during onboarding, which has no Dictate page.
    fn pill(&self) -> bool {
        self.store.settings.overlay && !self.here
    }
    /// A neutral outcome: on the pill when it shows, otherwise as a toast. Same words either way.
    fn notify(&mut self, text: &str, lasts: Duration) {
        if self.pill() {
            self.show_hud("notice", text, Some(lasts))
        } else {
            self.say("info", text)
        }
    }
    /// Engine failures are shown by the Dictate page, which skips their toast.
    fn error_topic(&self) -> &'static str {
        if self.engine.phase == Phase::Error {
            "engine"
        } else {
            ""
        }
    }
    fn fail_download(&mut self, id: &'static str, text: String) {
        self.download_error = Some(DownloadError {
            id,
            text: text.clone(),
        });
        self.say_about("download", "error", text);
        // Reloads the model in use, or shows setup when none is installed.
        self.load();
    }
    fn show_hud(&mut self, mode: &'static str, text: impl Into<String>, lasts: Option<Duration>) {
        if !self.pill() {
            // For example the indicator was turned off mid-dictation: don't leave the pill behind.
            self.hide_hud();
            return;
        }
        if self.hud.mode == "hidden" || self.hud_hide_at.is_some() {
            native::show_overlay(
                self.overlay.0,
                self.target_hwnd,
                self.overlay.1,
                self.store.settings.hud_position == "bottom",
            );
        }
        self.hud_hide_at = None;
        let live = if matches!(mode, "listening" | "writing") {
            std::mem::take(&mut self.hud.live)
        } else {
            String::new()
        };
        self.hud = Hud {
            mode,
            text: text.into(),
            live,
        };
        self.hud_until = lasts.map(|d| Instant::now() + d);
    }
    fn hide_hud(&mut self) {
        if self.hud.mode != "hidden" {
            self.hud = Hud {
                mode: "hidden",
                text: String::new(),
                live: String::new(),
            };
            // Let the pill animate out before the window disappears.
            self.hud_hide_at = Some(Instant::now() + Duration::from_millis(260));
        }
        self.hud_until = None;
    }
    fn target_name(&self) -> String {
        self.target
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_default()
    }
    fn busy_dictating(&self) -> bool {
        self.recorder.is_some() || self.pending.is_some()
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Hook(HookEvent::Pressed) => {
                if self.recorder.is_none() {
                    self.start(false);
                } else if self.store.settings.toggle {
                    self.stop_recording();
                }
            }
            Msg::Hook(HookEvent::Released) => self.stop_recording(),
            Msg::Hook(HookEvent::Escape) => {
                if self.recorder.is_some() {
                    self.cancel();
                }
            }
            Msg::Hook(HookEvent::Captured(keys)) => self.set_shortcut(keys),
            Msg::PreviewReady(dictation, prepared) => self.send_preview(dictation, prepared),
            Msg::Inserted { outcome, text } => self.inserted(outcome, text),
            Msg::MainWindow { visible } => {
                let shown = visible && !self.main_visible;
                self.main_visible = visible;
                if shown {
                    self.publish(true);
                }
                if !visible {
                    // Nobody sees the Dictate page or the microphone test any more.
                    if self.here && self.recorder.is_some() {
                        self.cancel();
                    }
                    self.mic_test = None;
                }
            }
            Msg::Update(msg) => self.update_event(msg),
            Msg::Action(action) => self.act(action),
        }
    }

    fn inserted(&mut self, outcome: native::Outcome, text: String) {
        self.inserting = self.inserting.saturating_sub(1);
        crate::log::write(format!(
            "inserted {} ms after release",
            self.stopped_at.elapsed().as_millis()
        ));
        // A later dictation may already own the pill.
        let busy = self.busy_dictating();
        let problem = match outcome {
            native::Outcome::Inserted => {
                if !busy {
                    self.show_hud("done", "Inserted", Some(Duration::from_millis(1100)));
                }
                return;
            }
            native::Outcome::FocusLost => "Couldn't insert.",
            native::Outcome::Elevated => "That app runs as administrator.",
        };
        // Out of clipboard history: the text is only there to be pasted, and it's in Vorto's History.
        let copied = native::set_clipboard_private(&text);
        if let Err(e) = self.store.forget_app(&text) {
            crate::log::write(format!("history not updated: {e}"));
        }
        // Another app can hold the clipboard open. The Dictate page shows the text either way.
        let message = if copied {
            format!("{problem} Press Ctrl+V to paste.")
        } else {
            format!("{problem} Open Vorto to copy the text.")
        };
        if !busy && self.pill() {
            self.show_hud("notice", message, Some(Duration::from_millis(2600)));
        } else {
            self.say("info", message);
        }
    }

    fn act(&mut self, action: Action) {
        crate::log::write(format!("action: {}", action.name()));
        match action {
            Action::StartDictation => self.start(true),
            Action::StopDictation => self.stop_recording(),
            Action::Cancel => self.cancel(),
            Action::Download { id } => self.download(&id),
            Action::UseModel { id } => {
                let loaded = id == self.store.settings.model
                    && matches!(self.engine.phase, Phase::Ready | Phase::Starting);
                if data::installed(&self.store.root, &id) && !loaded {
                    self.store.settings.model = id;
                    self.save();
                    self.load();
                }
            }
            Action::Retry => self.load(),
            Action::UseCpu => {
                self.store.settings.gpu = false;
                self.save();
                self.load();
            }
            Action::SaveSettings { mut settings } => {
                settings.validate();
                let reload = settings.gpu != self.store.settings.gpu
                    || settings.model != self.store.settings.model;
                let check_now = settings.auto_update && !self.store.settings.auto_update;
                // Reconfiguring resets the shortcut's pressed state, which a running dictation needs.
                let toggle_changed = settings.toggle != self.store.settings.toggle;
                // The shortcut only changes through recording.
                settings.hotkey = self.store.settings.hotkey.clone();
                settings.autostart = self.store.settings.autostart;
                self.store.settings = settings;
                if toggle_changed {
                    hotkey::configure(&self.store.settings.hotkey, self.store.settings.toggle);
                }
                self.save();
                if check_now {
                    self.next_update_check = Instant::now();
                }
                if reload && self.installed() {
                    if !self.busy_dictating() && !self.engine.phase.busy() {
                        self.load();
                    } else {
                        // Applied by tick() once the engine is free, without losing a dictation.
                        self.reload_needed = true;
                    }
                }
            }
            Action::RecordShortcut => {
                if self.recorder.is_none() {
                    self.capturing = true;
                    hotkey::begin_capture();
                }
            }
            Action::StopRecordingShortcut => {
                self.capturing = false;
                hotkey::end_capture();
            }
            Action::SetShortcut { keys } => {
                if self.capturing {
                    self.set_shortcut(keys);
                }
            }
            Action::ClearHistory => {
                if let Err(e) = self.store.clear() {
                    self.say("error", format!("History couldn't be cleared: {e}"));
                }
            }
            Action::Copy { text } => {
                if !native::set_clipboard(&text) {
                    self.say("error", "Couldn't copy to the clipboard.");
                }
            }
            Action::OpenDataFolder => {
                use std::os::windows::ffi::OsStrExt;
                let folder: Vec<u16> = self
                    .store
                    .root
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                let open: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
                // Through the shell: a bare "explorer.exe" is looked up next to vorto.exe first.
                std::thread::spawn(move || unsafe {
                    ShellExecuteW(
                        0,
                        open.as_ptr(),
                        folder.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null(),
                        SW_SHOWNORMAL,
                    );
                });
            }
            Action::RefreshMicrophones => self.microphones = audio::devices(),
            Action::TestMicrophone { on } => {
                self.mic_test = None;
                if on && self.recorder.is_none() {
                    match Recorder::start(&self.store.settings.microphone) {
                        Ok(recorder) => self.mic_test = Some((recorder, Instant::now())),
                        Err(e) => self.say("error", format!("Microphone unavailable: {e:#}")),
                    }
                }
            }
            Action::SetAutostart { on } => {
                if native::set_autostart(on) {
                    self.autostart = on;
                    self.store.settings.autostart = on;
                    self.save();
                } else {
                    self.say(
                        "error",
                        "Windows didn't allow changing the startup setting.",
                    );
                }
            }
            Action::CheckForUpdates => {
                if !matches!(
                    self.update.status,
                    "checking" | "downloading" | "installing" | "ready"
                ) {
                    self.update.status = "checking";
                    update::check(&self.app, self.tx.clone(), true);
                }
            }
            Action::InstallUpdate => match self.update.status {
                "ready" => self.install_update(),
                "available" | "error" if self.update_found.is_some() => {
                    self.install_when_ready = true;
                    self.download_update();
                }
                _ => {}
            },
        }
    }

    fn update_event(&mut self, msg: UpdateMsg) {
        match msg {
            UpdateMsg::Checked { found, asked } => match found {
                Ok(Some(found))
                    if self
                        .update_found
                        .as_ref()
                        .is_some_and(|f| f.version == found.version)
                        && self.update_bytes.is_some() =>
                {
                    // Already downloaded this session.
                    self.update.status = "ready";
                }
                Ok(Some(found)) => {
                    crate::log::write(format!("update available: {}", found.version));
                    self.update.version = found.version.clone();
                    self.update_found = Some(found);
                    self.update_bytes = None;
                    if self.store.settings.auto_update || asked {
                        self.download_update();
                    } else {
                        self.update.status = "available";
                    }
                }
                Ok(None) => self.update.status = if asked { "latest" } else { "idle" },
                Err(e) => {
                    crate::log::write(format!("update check failed: {e}"));
                    // A check in the background fails quietly, for example while offline.
                    self.update.status = if asked { "error" } else { "idle" };
                }
            },
            UpdateMsg::Progress(fraction) => {
                if self.update.status == "downloading" {
                    self.update.progress = fraction;
                }
            }
            UpdateMsg::Downloaded(Ok(bytes)) => {
                self.update_bytes = Some(bytes);
                self.update.status = "ready";
                if self.install_when_ready {
                    self.install_update();
                }
            }
            UpdateMsg::Downloaded(Err(e)) => {
                crate::log::write(format!("update download failed: {e}"));
                self.update.status = "error";
                self.install_when_ready = false;
            }
        }
    }
    fn download_update(&mut self) {
        let Some(found) = self.update_found.clone() else {
            return;
        };
        self.update.status = "downloading";
        self.update.progress = -1.0;
        update::download(*found, self.tx.clone());
    }
    fn install_update(&mut self) {
        // Never in the middle of a dictation, an insertion or a model download: it runs when the
        // user is done.
        if self.busy_dictating() || self.inserting > 0 || self.downloading.is_some() {
            self.install_when_ready = true;
            return;
        }
        let (Some(found), Some(bytes)) = (self.update_found.clone(), self.update_bytes.take())
        else {
            return;
        };
        crate::log::write(format!("installing update {}", found.version));
        self.update.status = "installing";
        self.publish(true);
        // The installer replaces the engine, so it must not be running.
        self.engine.stop();
        // Otherwise the icon stays in the tray until the mouse passes over it.
        drop(self.app.remove_tray_by_id("vorto"));
        // The installer exits Vorto, which would cut short putting back the user's clipboard.
        native::finish_restore();
        if let Err(e) = update::install(&found, &bytes) {
            // The updater already removed the tray icon and hid the windows: start over.
            crate::log::write(format!("update install failed: {e}"));
            native::message_box(
                &format!(
                    "Vorto couldn't install the update to version {}.\n\n{e}\n\nIt keeps running the current version.",
                    found.version
                ),
                false,
            );
            self.app.restart();
        }
    }

    fn set_shortcut(&mut self, keys: Vec<u32>) {
        crate::log::write(format!("shortcut captured: {keys:?}"));
        self.capturing = false;
        hotkey::end_capture();
        if !keys.is_empty() {
            self.store.settings.hotkey = keys;
            self.store.settings.validate();
            self.save();
        }
        hotkey::configure(&self.store.settings.hotkey, self.store.settings.toggle);
    }
    fn save(&mut self) {
        if let Err(e) = self.store.save() {
            self.say("error", format!("Settings couldn't be saved: {e}"));
        }
    }
    fn gpu(&self) -> bool {
        self.store.settings.gpu && !self.gpu_failed && self.gpu_engine
    }
    /// In auto mode, clips too short to tell languages apart get the language heard last in this
    /// dictation. Longer ones are detected on their own, so one misheard phrase can't set the
    /// language of everything after it.
    fn language(&self, seconds: f32) -> String {
        match &self.detected_language {
            Some(language) if self.store.settings.language == "auto" && seconds < 3.0 => {
                language.clone()
            }
            _ => self.store.settings.language.clone(),
        }
    }
    /// Whisper with the graphics card on runs in the graphics card engine; everything else,
    /// including downloads, in the processor engine.
    fn gpu_model(&self) -> bool {
        self.gpu()
            && data::model(&self.store.settings.model).is_some_and(|m| m.family == Family::Whisper)
    }
    fn load(&mut self) {
        self.flight = None;
        self.reload_needed = false;
        self.slow_engine = false;
        self.load_at = None;
        // Starting a model ends any download still running in the old worker.
        self.downloading = None;
        if !self.installed() {
            self.engine.phase = Phase::Setup;
            return;
        }
        let gpu = self.gpu_model();
        self.engine_gpu = gpu;
        let result = self.engine.launch(&self.store.root, gpu).and_then(|_| {
            self.engine.send(
                Command::Load {
                    root: self.store.root.clone(),
                    model: self.store.settings.model.clone(),
                    gpu,
                },
                Phase::Starting,
            )
        });
        if let Err(e) = result {
            self.engine
                .fail(&format!("Couldn't start the voice model: {e:#}"));
        }
    }
    fn download(&mut self, id: &str) {
        let Some(model) = data::model(id) else { return };
        if self.busy_dictating() {
            return;
        }
        self.download_error = None;
        // The model in use only changes once the download is complete.
        self.downloading = Some(model.id);
        self.engine_gpu = false;
        let result = self.engine.launch(&self.store.root, false).and_then(|_| {
            self.engine.send(
                Command::Download {
                    root: self.store.root.clone(),
                    model: model.id.into(),
                },
                Phase::Downloading,
            )
        });
        if let Err(e) = result {
            self.downloading = None;
            self.fail_download(model.id, format!("The download couldn't start: {e:#}"));
        }
    }

    fn start(&mut self, from_app: bool) {
        if self.recorder.is_some()
            || self.pending.is_some()
            || self.capturing
            || self.engine.phase == Phase::Downloading
        {
            hotkey::force_inactive();
            return;
        }
        if !self.installed() {
            hotkey::force_inactive();
            self.say("info", "Download a voice model to start dictating.");
            let _ = self.app.emit("navigate", "models");
            crate::show_main(&self.app);
            return;
        }
        self.mic_test = None;
        let foreground = native::foreground();
        self.target_hwnd = if from_app || native::is_own(foreground) {
            0
        } else {
            foreground
        };
        if self.target_hwnd != 0 {
            self.set_target(self.target_hwnd);
        }
        self.here = self.target_hwnd == 0 && self.main_visible && self.store.settings.onboarded;
        if self.here {
            let _ = self.app.emit("navigate", "home");
        }
        self.hud.live.clear();
        match Recorder::start(&self.store.settings.microphone) {
            Ok(recorder) => {
                self.dictation += 1;
                self.committed.clear();
                self.committed_end = 0;
                self.detected_language = None;
                self.recorder = Some(recorder);
                self.started = Instant::now();
                self.started_epoch = epoch_ms();
                hotkey::set_recording(true);
                let name = if self.target_hwnd != 0 {
                    self.target_name()
                } else {
                    String::new()
                };
                self.show_hud("listening", name, None);
                if !matches!(self.engine.phase, Phase::Ready | Phase::Starting) {
                    self.load();
                    // Stop now rather than let the words wait for an engine that is not coming.
                    if self.engine.phase != Phase::Starting {
                        self.recorder = None;
                        hotkey::set_recording(false);
                        let detail = self.engine.detail.clone();
                        self.say_about(self.error_topic(), "error", detail);
                        self.show_hud(
                            "error",
                            "Voice model unavailable",
                            Some(Duration::from_secs(2)),
                        );
                        self.here = false;
                    }
                }
            }
            Err(e) => {
                hotkey::force_inactive();
                self.say("error", format!("Microphone unavailable: {e:#}"));
                self.show_hud(
                    "error",
                    "Microphone unavailable",
                    Some(Duration::from_secs(2)),
                );
                self.here = false;
            }
        }
    }

    fn stop_recording(&mut self) {
        let Some(recorder) = self.recorder.take() else {
            return;
        };
        self.stopped_at = Instant::now();
        hotkey::set_recording(false);
        self.last_used = Instant::now();
        let tap = recorder.tap();
        let rate = tap.rate().max(1) as u64;
        // A microphone that failed mid-dictation still leaves what it captured.
        let samples = recorder.stop().or_else(|e| match tap.len() {
            0 => Err(e),
            len => tap.segment(0, len),
        });
        let samples = match samples {
            Ok(samples) => samples,
            Err(e) => {
                self.say("error", format!("The recording couldn't be read: {e:#}"));
                self.show_hud("error", "Microphone stopped", Some(Duration::from_secs(2)));
                self.here = false;
                return;
            }
        };
        let energy = samples.iter().map(|v| v * v).sum::<f32>() / samples.len().max(1) as f32;
        if samples.len() < 4800 || energy < 0.000001 {
            let text = if self.store.settings.toggle || self.here {
                "Too short. Speak a little longer."
            } else {
                "Too short. Hold the shortcut while you speak."
            };
            self.notify(text, Duration::from_millis(1800));
            self.here = false;
            return;
        }
        let seconds = samples.len() as f32 / 16000.0;
        self.pending = Some(Pending {
            samples,
            rate,
            file: None,
            seconds,
            tail_seconds: seconds,
        });
        let name = if self.target_hwnd != 0 {
            self.target_name()
        } else {
            String::new()
        };
        self.show_hud("writing", name, None);
        if self.engine.phase == Phase::Ready {
            self.transcribe();
        }
    }
    /// Sends the part of the recording not yet recognized. Waits for a commit still being
    /// recognized, because its result decides where that part starts.
    fn transcribe(&mut self) {
        let dictation = self.dictation;
        if self
            .flight
            .as_ref()
            .is_some_and(|f| f.dictation == dictation && matches!(f.draft, Draft::Commit { .. }))
        {
            return;
        }
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        if pending.file.is_none() {
            let from = ((self.committed_end as u64 * 16000 / pending.rate) as usize)
                .min(pending.samples.len());
            let mut tail = std::mem::take(&mut pending.samples).split_off(from);
            crate::log::write(format!(
                "final pass: {:.1} s of {:.1} s",
                tail.len() as f32 / 16000.0,
                pending.seconds
            ));
            // Whisper tends to invent words such as "Thank you." for silence.
            if !voiced(&tail) {
                let seconds = pending.seconds;
                self.pending = None;
                let text = std::mem::take(&mut self.committed);
                self.committed_end = 0;
                self.deliver(text, seconds);
                return;
            }
            if tail.len() < 8000 {
                tail.resize(8000, 0.0);
            }
            pending.tail_seconds = tail.len() as f32 / 16000.0;
            match write_wav(&tail) {
                Ok(file) => pending.file = Some(file),
                Err(e) => {
                    self.abandon(format!("The recording couldn't be saved: {e:#}"));
                    return;
                }
            }
        }
        let Some(pending) = self.pending.as_ref() else {
            return;
        };
        let Some(file) = pending.file.as_ref() else {
            return;
        };
        let command = Command::Transcribe {
            audio: file.path().into(),
            language: self.language(pending.tail_seconds),
        };
        if let Err(e) = self.engine.send(command, Phase::Transcribing) {
            self.engine.fail(&e.to_string());
            self.flight = None;
            self.abandon(e.to_string());
        }
    }
    fn cancel(&mut self) {
        let was_recording = self.recorder.take().is_some();
        hotkey::set_recording(false);
        hotkey::force_inactive();
        self.dictation += 1;
        self.detected_language = None;
        let was_pending = self.pending.take().is_some();
        let was_downloading = self.downloading.take().is_some();
        // A preview still in flight stays, so its reply is not taken for the next one.
        if was_pending || was_downloading || self.engine.phase.busy() {
            self.engine.sleep();
            self.flight = None;
        }
        if was_recording && self.pill() {
            self.show_hud("notice", "Discarded", Some(Duration::from_millis(900)));
        } else {
            self.hide_hud();
        }
        self.here = false;
        self.last_used = Instant::now();
    }
    /// Ends a dictation the engine could not finish. Words recognized before the failure go
    /// to History and the clipboard, not into whatever app has focus by now.
    fn abandon(&mut self, detail: String) {
        if self.recorder.take().is_some() {
            hotkey::set_recording(false);
        }
        let seconds = match self.pending.take() {
            Some(pending) => pending.seconds,
            None => self.started.elapsed().as_secs_f32(),
        };
        let partial = std::mem::take(&mut self.committed).trim().to_string();
        self.committed_end = 0;
        self.detected_language = None;
        let topic = self.error_topic();
        if partial.is_empty() {
            self.show_hud(
                "error",
                "Couldn't write this down",
                Some(Duration::from_secs(2)),
            );
            self.say_about(topic, "error", detail);
        } else {
            if let Err(e) = self.store.add(partial.clone(), seconds, String::new()) {
                crate::log::write(format!("history not saved: {e}"));
            }
            let copied = native::set_clipboard_private(&partial);
            self.latest = partial;
            // Another app can hold the clipboard open. The Dictate page shows the words either way.
            let (pill, place) = if copied {
                ("Partly copied. Press Ctrl+V to paste", "on your clipboard")
            } else {
                ("Partly kept in Vorto", "on the Dictate page")
            };
            self.show_hud("error", pill, Some(Duration::from_millis(2600)));
            self.say_about(
                topic,
                "error",
                format!("{detail} The words heard before that are {place}."),
            );
        }
        self.here = false;
    }

    /// While speaking: finished sentences are committed at pauses, and the words after
    /// the last commit are previewed. Audio is prepared on a worker thread so levels and
    /// hotkeys never stall, and the final pass only has to handle the last few seconds.
    fn request_preview(&mut self) {
        let Some(recorder) = &self.recorder else {
            return;
        };
        let fast_enough = !self.slow_engine
            && data::model(&self.store.settings.model)
                .is_some_and(|m| m.family == Family::Parakeet || self.gpu());
        if !fast_enough
            || self.flight.is_some()
            || self.preview_preparing
            || self.engine.phase != Phase::Ready
            || self.last_preview.elapsed() < Duration::from_millis(400)
            || recorder.seconds() < 0.7
        {
            return;
        }
        self.last_preview = Instant::now();
        self.preview_preparing = true;
        let tap = recorder.tap();
        let tx = self.tx.clone();
        let from = self.committed_end;
        let live = self.store.settings.live_preview;
        let dictation = self.dictation;
        std::thread::spawn(move || {
            let rate = tap.rate() as usize;
            let len = tap.len();
            let waiting = len.saturating_sub(from);
            let cut = if waiting > rate * 9 {
                tap.find_pause(from + rate * 3, len.saturating_sub(rate))
                    .or_else(|| {
                        // No clear pause: cut where it is quietest, so no word is split in two.
                        (waiting > rate * 20)
                            .then(|| tap.quietest(from + rate * 12, from + rate * 18))
                    })
            } else {
                None
            };
            let (range, draft) = match cut {
                Some(end) => ((from, end), Draft::Commit { end }),
                None if live => ((from, len), Draft::Preview),
                None => {
                    let _ = tx.send(Msg::PreviewReady(dictation, None));
                    return;
                }
            };
            // Whisper tends to invent words such as "Thank you." for silence. Silence left
            // uncommitted is covered by a later commit or the final pass.
            let prepared = tap
                .segment(range.0, range.1)
                .ok()
                .filter(|s| voiced(s))
                .and_then(|samples| {
                    let file = write_wav(&samples).ok()?;
                    Some(Prepared {
                        file,
                        draft,
                        seconds: samples.len() as f32 / 16000.0,
                    })
                });
            let _ = tx.send(Msg::PreviewReady(dictation, prepared));
        });
    }
    fn send_preview(&mut self, dictation: u64, prepared: Option<Prepared>) {
        self.preview_preparing = false;
        let Some(Prepared {
            file,
            draft,
            seconds,
        }) = prepared
        else {
            return;
        };
        // The dictation may have ended, or another begun, while the audio was prepared.
        if dictation != self.dictation
            || self.recorder.is_none()
            || self.engine.phase != Phase::Ready
            || self.flight.is_some()
        {
            return;
        }
        let command = Command::Preview {
            audio: file.path().into(),
            language: self.language(seconds),
        };
        let phase = self.engine.phase;
        if self.engine.send(command, phase).is_ok() {
            self.flight = Some(Flight {
                _file: file,
                draft,
                dictation,
                sent: Instant::now(),
                seconds,
            });
        }
    }

    fn deliver(&mut self, text: String, seconds: f32) {
        self.last_used = Instant::now();
        let text = text.trim().to_string();
        if text.is_empty() {
            self.notify("No words heard", Duration::from_millis(1600));
            self.here = false;
            return;
        }
        self.dictations += 1;
        self.latest = text.clone();
        let pasting = self.store.settings.paste && native::is_window(self.target_hwnd);
        let app = if pasting {
            self.target_name()
        } else {
            String::new()
        };
        if let Err(e) = self.store.add(text.clone(), seconds, app) {
            self.say("error", format!("History couldn't be saved: {e}"));
        }
        let hwnd = self.target_hwnd;
        if pasting {
            let delivered = text.clone();
            let mut text = text;
            if self.store.settings.append_space {
                text.push(' ');
            }
            let paste = self.store.settings.method == "paste";
            let restore = self.store.settings.restore_clipboard;
            let tx = self.tx.clone();
            // Counted until Inserted arrives, so an update waits for the text to land.
            self.inserting += 1;
            std::thread::spawn(move || {
                let _turn = INSERTING.lock().unwrap_or_else(|e| e.into_inner());
                let outcome = native::insert(&text, hwnd, paste, restore);
                let _ = tx.send(Msg::Inserted {
                    outcome,
                    text: delivered,
                });
            });
        } else {
            // Does nothing for a dictation into the Vorto window, which shows the text itself.
            self.show_hud("notice", "Kept in Vorto", Some(Duration::from_millis(1400)));
        }
        self.here = false;
    }

    fn tick(&mut self) {
        for event in self.engine.poll() {
            match event {
                Event::Ready => {
                    self.last_used = Instant::now();
                    if self.pending.is_some() {
                        self.transcribe();
                    }
                }
                Event::Downloaded => {
                    if let Some(id) = self.downloading.take() {
                        let name = data::model(id).map_or("Voice model", |m| m.name);
                        self.store.settings.model = id.into();
                        self.save();
                        self.download_error = None;
                        self.say_about("downloaded", "success", format!("{name} downloaded"));
                    }
                    self.refresh_installed();
                    self.load();
                }
                Event::Result { text } => {
                    crate::log::write(format!(
                        "recognized {} ms after release",
                        self.stopped_at.elapsed().as_millis()
                    ));
                    let seconds = self.pending.take().map(|p| p.seconds).unwrap_or(0.0);
                    let text = join_text(&std::mem::take(&mut self.committed), text.trim());
                    self.committed_end = 0;
                    self.deliver(text, seconds);
                }
                Event::Error { detail } => {
                    self.flight = None;
                    let failed = self.downloading.take();
                    self.refresh_installed();
                    let crashed_on_gpu =
                        failed.is_none() && detail == vorto::supervisor::CRASHED && self.engine_gpu;
                    if crashed_on_gpu {
                        // A graphics driver took the engine down. The processor is slower but works.
                        crate::log::write(
                            "engine crashed with the graphics card; switching to the processor",
                        );
                        self.gpu_failed = true;
                        self.say("info", "Whisper stopped working on the graphics card, so Vorto uses the processor now.");
                        // A dictation stays: its audio is still here and goes to the new engine once it's ready.
                        self.load();
                    } else if self.busy_dictating() {
                        self.abandon(detail);
                    } else if let Some(id) = failed {
                        self.fail_download(id, detail);
                    } else {
                        self.say_about("engine", "error", detail);
                    }
                }
                Event::Progress { .. } => {}
                Event::Preview { text, ok, language } => {
                    let Some(flight) = self.flight.take() else {
                        continue;
                    };
                    // Slower than this, previews delay the final pass more than they help.
                    if flight.sent.elapsed() > Duration::from_secs_f32(1.0 + flight.seconds * 0.25)
                    {
                        self.slow_engine = true;
                    }
                    if flight.dictation != self.dictation {
                        continue;
                    }
                    // The latest language heard in at least two seconds of speech; shorter clips
                    // are too little to tell languages apart reliably.
                    if ok && language.is_some() && flight.seconds >= 2.0 {
                        self.detected_language = language;
                    }
                    let text = text.trim();
                    match flight.draft {
                        // A failed commit stays uncommitted, so the final pass covers that audio.
                        Draft::Commit { end } if ok => {
                            self.committed = join_text(&self.committed, text);
                            self.committed_end = end;
                            if self.recorder.is_some() && self.store.settings.live_preview {
                                self.hud.live = self.committed.clone();
                            }
                        }
                        Draft::Preview if ok && self.recorder.is_some() && !text.is_empty() => {
                            self.hud.live = join_text(&self.committed, text);
                        }
                        _ => {}
                    }
                    // The final pass may have been waiting for this commit.
                    if self.pending.is_some() && self.engine.phase == Phase::Ready {
                        self.transcribe();
                    }
                }
            }
        }
        if self
            .flight
            .as_ref()
            .is_some_and(|f| f.sent.elapsed() > Duration::from_secs(45))
        {
            self.flight = None;
            // A hung worker: restart it without losing the dictation, whose audio is still here.
            if matches!(self.engine.phase, Phase::Ready | Phase::Transcribing) {
                crate::log::write("preview stalled; restarting the voice engine");
                if self.busy_dictating() {
                    self.load();
                } else {
                    self.engine.sleep();
                }
                self.slow_engine = true;
            }
        }
        // Nothing would ever finish a dictation the engine cannot take, such as after a failed start.
        if self.pending.is_some()
            && matches!(
                self.engine.phase,
                Phase::Error | Phase::Sleeping | Phase::Setup
            )
        {
            let detail = if self.engine.phase == Phase::Error {
                self.engine.detail.clone()
            } else {
                "The voice model stopped before it could write this down.".to_string()
            };
            self.abandon(detail);
        }
        if self.reload_needed
            && !self.busy_dictating()
            && self.flight.is_none()
            && self.engine.phase == Phase::Ready
        {
            self.load();
        }
        if self.load_at.is_some_and(|at| Instant::now() >= at)
            && matches!(self.engine.phase, Phase::Setup)
        {
            self.load();
        }
        if self.installed_checked.elapsed() > Duration::from_secs(5) {
            self.refresh_installed();
        }
        let foreground = native::foreground();
        if foreground != 0 && foreground != self.last_foreign && !native::is_own(foreground) {
            self.last_foreign = foreground;
            if !self.busy_dictating() {
                self.set_target(foreground);
            }
        }
        if let Some(error) = self.recorder.as_ref().and_then(|r| r.error()) {
            // Keep what was captured before the microphone went away.
            self.stop_recording();
            self.say("error", error);
        }
        if self.recorder.is_some()
            && self.started.elapsed() >= Duration::from_secs(MAX_RECORDING_SECS)
        {
            self.stop_recording();
        }
        self.request_preview();
        if self
            .mic_test
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(10))
        {
            self.mic_test = None;
        }
        if let Some(recorder) = self
            .recorder
            .as_ref()
            .or(self.mic_test.as_ref().map(|(r, _)| r))
        {
            if self.last_level.elapsed() >= Duration::from_millis(33) {
                self.last_level = Instant::now();
                let level = recorder.level().to_string();
                // Each window gets its own event name: a page's listeners receive every event
                // with their name, whichever window it was sent to.
                if self.main_visible && (self.here || self.mic_test.is_some()) {
                    let _ = self.app.emit_str_to("main", "level", level.clone());
                }
                if self.hud.mode == "listening" {
                    let _ = self.app.emit_str_to("hud", "hud-level", level);
                }
            }
        }
        if self.store.settings.idle_minutes > 0
            && !self.busy_dictating()
            && self.engine.phase == Phase::Ready
            && self.last_used.elapsed() > Duration::from_secs(self.store.settings.idle_minutes * 60)
        {
            self.engine.sleep();
        }
        if self.hud_until.is_some_and(|t| Instant::now() >= t) {
            self.hide_hud();
        }
        if self.hud_hide_at.is_some_and(|t| Instant::now() >= t) {
            self.hud_hide_at = None;
            native::hide_overlay(self.overlay.0);
        }
        if self.store.settings.auto_update
            && Instant::now() >= self.next_update_check
            && matches!(self.update.status, "idle" | "latest" | "error")
        {
            self.next_update_check = Instant::now() + UPDATE_INTERVAL;
            self.update.status = "checking";
            update::check(&self.app, self.tx.clone(), false);
        }
        if self.install_when_ready
            && self.update.status == "ready"
            && !self.busy_dictating()
            && self.inserting == 0
            && self.downloading.is_none()
        {
            self.install_update();
        }
    }

    /// Sends what changed. The main window only gets its state while it is on screen, unless
    /// `force`: serializing the history and app icons for a hidden window is wasted work.
    fn publish(&mut self, force: bool) {
        // The hidden pill gets nothing that changes, so it has nothing to redraw.
        let showing = self.hud.mode != "hidden";
        let hud = HudSnapshot {
            // Previews keep changing `live` for a dictation the pill doesn't show.
            hud: HudView {
                mode: self.hud.mode,
                text: if showing { &self.hud.text } else { "" },
                live: if showing { &self.hud.live } else { "" },
            },
            recording: showing && self.recorder.is_some(),
            recording_since: self.started_epoch,
            recording_limit: MAX_RECORDING_SECS,
            settings: HudSettings {
                hud_position: &self.store.settings.hud_position,
            },
            target: if showing { self.target.as_ref() } else { None },
        };
        if let Ok(json) = serde_json::to_string(&hud) {
            if json != self.last_hud {
                if let Ok(mut published) = self.published.lock() {
                    published.hud = json.clone();
                }
                let _ = self.app.emit_str_to("hud", "hud-state", json.clone());
                self.last_hud = json;
            }
        }
        if !self.main_visible && !force {
            return;
        }
        let settings = &self.store.settings;
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let snapshot = Snapshot {
            phase: match self.engine.phase {
                Phase::Setup => "setup",
                Phase::Starting => "starting",
                Phase::Ready => "ready",
                Phase::Sleeping => "sleeping",
                Phase::Downloading => "downloading",
                Phase::Transcribing => "transcribing",
                Phase::Error => "error",
            },
            detail: &self.engine.detail,
            progress: self.engine.progress,
            downloading: self.downloading,
            download_error: self.download_error.as_ref(),
            recording: self.recorder.is_some(),
            recording_since: self.started_epoch,
            recording_limit: MAX_RECORDING_SECS,
            dictating_here: self.here && self.busy_dictating(),
            live: if self.here { &self.hud.live } else { "" },
            dictations: self.dictations,
            settings,
            shortcut: hotkey::names(&settings.hotkey),
            capturing: self.capturing,
            hook_ok: hotkey::installed(),
            models: data::MODELS
                .iter()
                .zip(&self.models_installed)
                .map(|(m, &installed)| ModelView {
                    id: m.id,
                    name: m.name,
                    family: match m.family {
                        Family::Parakeet => "parakeet",
                        Family::Whisper => "whisper",
                    },
                    languages: m.languages,
                    download_mb: m.download_mb,
                    memory_mb: m.memory_mb,
                    speed: m.speed,
                    accuracy: m.accuracy,
                    hardware: m.hardware,
                    installed,
                    recommended: m.id == data::DEFAULT_MODEL,
                })
                .collect(),
            history: &self.store.history,
            words_today: self
                .store
                .history
                .iter()
                .filter(|e| e.at.starts_with(&today))
                .map(|e| e.text.split_whitespace().count())
                .sum(),
            latest: &self.latest,
            notice: &self.notice,
            microphones: &self.microphones,
            data_dir: self.store.root.display().to_string(),
            version: env!("CARGO_PKG_VERSION"),
            gpu_build: self.gpu_engine,
            mic_test: self.mic_test.is_some(),
            autostart: self.autostart,
            app_icons: self
                .app_icons
                .iter()
                .filter(|(name, _)| self.store.history.iter().any(|e| &e.app == *name))
                .map(|(name, icon)| (name.as_str(), icon.as_str()))
                .collect(),
            update: &self.update,
        };
        let Ok(json) = serde_json::to_string(&snapshot) else {
            return;
        };
        if json != self.last_main {
            if let Ok(mut published) = self.published.lock() {
                published.main = json.clone();
            }
            let _ = self.app.emit_str_to("main", "state", json.clone());
            self.last_main = json;
        }
    }
}
