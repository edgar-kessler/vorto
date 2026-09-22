use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Whisper,
    Parakeet,
}

/// One file of a model, as it exists at the model's pinned revision.
pub struct ModelFile {
    pub name: &'static str,
    pub size: u64,
    /// Lowercase hex SHA-256.
    pub sha256: &'static str,
}

/// A downloadable voice model. Figures are shown in the model picker.
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub family: Family,
    pub repo: &'static str,
    /// Full commit hash every file is fetched from. Moving it requires an app release.
    pub revision: &'static str,
    /// Files fetched from the repository into the model folder, checked against these values.
    pub files: &'static [ModelFile],
    pub languages: &'static str,
    pub download_mb: u32,
    /// Peak engine memory measured on CPU (Ryzen 7 5700X3D).
    pub memory_mb: u32,
    /// 1 to 5, relative to the other models.
    pub speed: u8,
    pub accuracy: u8,
    pub hardware: &'static str,
}

/// Recommended in the app.
pub const DEFAULT_MODEL: &str = "parakeet-v3";

/// Tauri's bundle identifier. The installer's "delete app data" option removes this folder in
/// %LOCALAPPDATA%, and WebView2 keeps its profile in it too.
pub const APP_ID: &str = "app.vorto.desktop";

pub const MODELS: &[Model] = &[
    Model {
        id: "parakeet-v3",
        name: "Parakeet v3",
        family: Family::Parakeet,
        repo: "istupakov/parakeet-tdt-0.6b-v3-onnx",
        revision: "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce",
        files: &[
            ModelFile {
                name: "encoder-model.int8.onnx",
                size: 652_183_999,
                sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
            },
            ModelFile {
                name: "decoder_joint-model.int8.onnx",
                size: 18_202_004,
                sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
            },
            ModelFile {
                name: "nemo128.onnx",
                size: 139_764,
                sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
            },
            ModelFile {
                name: "vocab.txt",
                size: 93_939,
                sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
            },
        ],
        languages: "25 European languages",
        download_mb: 670,
        memory_mb: 900,
        speed: 5,
        accuracy: 5,
        hardware: "Fast on any modern processor",
    },
    Model {
        id: "whisper-turbo",
        name: "Whisper Turbo",
        family: Family::Whisper,
        repo: "ggerganov/whisper.cpp",
        revision: "5359861c739e955e79d9a303bcbc70fb988958b1",
        files: &[ModelFile {
            name: "ggml-large-v3-turbo-q5_0.bin",
            size: 574_041_195,
            sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        }],
        languages: "99 languages",
        download_mb: 574,
        memory_mb: 1100,
        speed: 2,
        accuracy: 5,
        hardware: "Needs a graphics card",
    },
    Model {
        id: "whisper-small",
        name: "Whisper Small",
        family: Family::Whisper,
        repo: "ggerganov/whisper.cpp",
        revision: "5359861c739e955e79d9a303bcbc70fb988958b1",
        files: &[ModelFile {
            name: "ggml-small.bin",
            size: 487_601_967,
            sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        }],
        languages: "99 languages",
        download_mb: 488,
        memory_mb: 850,
        speed: 3,
        accuracy: 4,
        hardware: "Graphics card recommended",
    },
    Model {
        id: "whisper-base",
        name: "Whisper Base",
        family: Family::Whisper,
        repo: "ggerganov/whisper.cpp",
        revision: "5359861c739e955e79d9a303bcbc70fb988958b1",
        files: &[ModelFile {
            name: "ggml-base.bin",
            size: 147_951_465,
            sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        }],
        languages: "99 languages",
        download_mb: 148,
        memory_mb: 400,
        speed: 4,
        accuracy: 3,
        hardware: "Light on any processor",
    },
];

pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gpu: bool,
    pub model: String,
    pub language: String,
    pub microphone: String,
    /// Virtual-key codes that must all be held; mouse buttons use VK 0x04 to 0x06.
    pub hotkey: Vec<u32>,
    pub toggle: bool,
    pub idle_minutes: u64,
    /// Insert the text into the app that was focused when dictation began.
    pub paste: bool,
    /// "paste" through the clipboard, or "type" character by character.
    pub method: String,
    pub append_space: bool,
    /// Show words in the indicator while speaking.
    pub live_preview: bool,
    /// "top" or "bottom" of the screen.
    pub hud_position: String,
    /// The first-run introduction has been completed or skipped.
    pub onboarded: bool,
    pub history: bool,
    pub restore_clipboard: bool,
    pub overlay: bool,
    /// Look for new versions and download them in the background.
    pub auto_update: bool,
    /// Start with Windows, as last chosen in Vorto. Windows' own list decides whether it runs.
    pub autostart: bool,
    /// A short glow over the words just inserted, where the app can say where they are.
    pub highlight: bool,
    /// Names and terms to spell the user's way: Whisper hears them better, and AI editing
    /// keeps them.
    pub vocabulary: Vec<String>,
    /// Words suggested from History that the user turned down.
    pub dismissed: Vec<String>,
    /// Applied to every dictation, before AI editing.
    pub replacements: Vec<Replacement>,
    pub ai: AiSettings,
    /// Extra shortcuts, each a list of virtual-key codes like `hotkey`, empty when unset.
    pub shortcuts: Shortcuts,
    /// Key clicks when dictation starts and ends, and a chime when the text is in place.
    pub sounds: bool,
}
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Shortcuts {
    pub paste_last: Vec<u32>,
    pub undo_last: Vec<u32>,
    pub toggle_ai: Vec<u32>,
    pub copy_last: Vec<u32>,
}
impl Shortcuts {
    /// In the order of their ids for Windows' RegisterHotKey.
    pub fn all(&self) -> [&Vec<u32>; 4] {
        [
            &self.paste_last,
            &self.undo_last,
            &self.toggle_ai,
            &self.copy_last,
        ]
    }
}

/// AI editing: a language model rewrites the dictation before it's inserted.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub enabled: bool,
    pub providers: Vec<AiProvider>,
    /// Tried in order; the first whose apps or window titles match is used. One without any
    /// is used everywhere else.
    pub profiles: Vec<AiProfile>,
    /// After this long the dictation is inserted as spoken.
    pub timeout_secs: u64,
}
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    /// "openai" for every OpenAI-compatible API, including Ollama and LM Studio, or "anthropic".
    pub kind: String,
    pub base_url: String,
    pub model: String,
    /// The user agreed to send dictated text to this provider. Only needed for providers
    /// outside this PC.
    pub allow_remote: bool,
}
impl AiProvider {
    /// Runs on this PC, so text never leaves it.
    pub fn local(&self) -> bool {
        let url = self.base_url.trim();
        let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = match host.strip_prefix('[') {
            Some(v6) => v6.split(']').next().unwrap_or(""),
            None => host.rsplit_once(':').map_or(host, |(h, _)| h),
        };
        let host = host.to_ascii_lowercase();
        host == "localhost" || host.starts_with("127.") || host == "::1"
    }
}
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct AiProfile {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// What to do with the dictation, in the user's words.
    pub prompt: String,
    /// Provider id; empty uses the first provider.
    pub provider: String,
    /// Overrides the provider's model when set.
    pub model: String,
    /// Program file names such as "outlook.exe".
    pub apps: Vec<String>,
    /// Parts of window titles, such as "Gmail" for a browser tab.
    pub titles: Vec<String>,
}
impl AiProfile {
    fn everywhere(&self) -> bool {
        self.apps.iter().all(|a| a.trim().is_empty())
            && self.titles.iter().all(|t| t.trim().is_empty())
    }
    fn matches(&self, exe: &str, title: &str) -> bool {
        let title = title.to_lowercase();
        self.apps
            .iter()
            .map(|a| a.trim())
            .any(|a| !a.is_empty() && a.eq_ignore_ascii_case(exe))
            || self
                .titles
                .iter()
                .map(|t| t.trim().to_lowercase())
                .any(|t| !t.is_empty() && title.contains(&t))
    }
}
impl AiSettings {
    /// The style for a window: the first matching one, else the first that applies everywhere.
    pub fn profile_for(&self, exe: &str, title: &str) -> Option<&AiProfile> {
        let enabled = || self.profiles.iter().filter(|p| p.enabled);
        enabled()
            .find(|p| !p.everywhere() && p.matches(exe, title))
            .or_else(|| enabled().find(|p| p.everywhere()))
    }
    /// A profile's provider; an empty id means the first one.
    pub fn provider(&self, id: &str) -> Option<&AiProvider> {
        if id.is_empty() {
            return self.providers.first();
        }
        self.providers.iter().find(|p| p.id == id)
    }
}
impl Default for AiSettings {
    fn default() -> Self {
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        Self {
            enabled: false,
            providers: vec![AiProvider {
                id: "ollama".into(),
                name: "Ollama".into(),
                kind: "openai".into(),
                base_url: "http://localhost:11434/v1".into(),
                model: String::new(),
                allow_remote: false,
            }],
            profiles: vec![
                AiProfile {
                    id: "email".into(),
                    name: "Email".into(),
                    enabled: true,
                    prompt: "Write this as a polished, polite email body: a greeting line, short paragraphs and a closing line. Remove filler words, fix grammar and punctuation. Match the formality of the transcript (for example Sie or du in German). Don't add a subject line, a signature name or facts that weren't spoken.".into(),
                    apps: list(&["outlook.exe", "olk.exe", "thunderbird.exe"]),
                    titles: list(&["Outlook", "Gmail", "Thunderbird"]),
                    ..Default::default()
                },
                AiProfile {
                    id: "prompt".into(),
                    name: "AI prompt".into(),
                    enabled: true,
                    prompt: "The user is dictating a prompt for an AI assistant. Clean it up: remove filler words, false starts and repetitions, fix grammar, and give it a clear structure, with short paragraphs or a list when several points are made. Keep every requirement and detail. Don't answer or carry out the prompt.".into(),
                    apps: list(&["claude.exe", "chatgpt.exe"]),
                    titles: list(&["Claude", "ChatGPT", "Gemini", "Perplexity", "Copilot"]),
                    ..Default::default()
                },
                AiProfile {
                    id: "chat".into(),
                    name: "Chat".into(),
                    enabled: true,
                    prompt: "This is a casual chat message. Fix grammar and punctuation and remove filler words. Keep it short and informal, with no greeting or sign-off unless one was spoken.".into(),
                    apps: list(&[
                        "slack.exe",
                        "ms-teams.exe",
                        "whatsapp.exe",
                        "discord.exe",
                        "telegram.exe",
                        "signal.exe",
                    ]),
                    titles: list(&["WhatsApp", "Slack", "Discord", "Teams"]),
                    ..Default::default()
                },
                AiProfile {
                    id: "clean".into(),
                    name: "Clean up".into(),
                    enabled: true,
                    prompt: "Fix grammar, spelling and punctuation. Remove filler words (such as um, uh, äh, ähm, also, halt, sozusagen, like, you know), false starts and repetitions. Otherwise keep the wording and tone.".into(),
                    ..Default::default()
                },
            ],
            timeout_secs: 20,
        }
    }
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            gpu: true,
            model: DEFAULT_MODEL.into(),
            language: "auto".into(),
            microphone: String::new(),
            hotkey: vec![VK_RCONTROL],
            toggle: false,
            idle_minutes: 0,
            paste: true,
            method: "paste".into(),
            append_space: false,
            live_preview: true,
            hud_position: "top".into(),
            onboarded: false,
            history: true,
            restore_clipboard: true,
            overlay: true,
            auto_update: true,
            autostart: false,
            vocabulary: Vec::new(),
            dismissed: Vec::new(),
            replacements: Vec::new(),
            ai: AiSettings::default(),
            shortcuts: Shortcuts::default(),
            sounds: true,
            highlight: true,
        }
    }
}
impl Settings {
    pub fn validate(&mut self) {
        if model(&self.model).is_none() {
            self.model = DEFAULT_MODEL.into();
        }
        if !["auto", "en", "de", "fr", "es", "it", "nl", "pt"].contains(&self.language.as_str()) {
            self.language = "auto".into();
        }
        normalize_hotkey(&mut self.hotkey);
        if self.hotkey.is_empty()
            || self.hotkey.len() > 4
            || self.hotkey.iter().any(|&k| k == 0 || k > 0xFE)
        {
            self.hotkey = vec![VK_RCONTROL];
        }
        if !["top", "bottom"].contains(&self.hud_position.as_str()) {
            self.hud_position = "top".into();
        }
        if !["paste", "type"].contains(&self.method.as_str()) {
            self.method = "paste".into();
        }
        if ![0, 5, 15, 30].contains(&self.idle_minutes) {
            self.idle_minutes = 0;
        }
        tidy(&mut self.vocabulary, 500);
        tidy(&mut self.dismissed, 500);
        for r in &mut self.replacements {
            r.from = r.from.trim().chars().take(80).collect();
            r.to = r.to.trim().chars().take(400).collect();
        }
        self.replacements.retain(|r| !r.from.is_empty());
        self.replacements.truncate(500);
        for keys in [
            &mut self.shortcuts.paste_last,
            &mut self.shortcuts.undo_last,
            &mut self.shortcuts.toggle_ai,
            &mut self.shortcuts.copy_last,
        ] {
            normalize_hotkey(keys);
            if keys.len() > 4 || keys.iter().any(|&k| k == 0 || k > 0xFE) {
                keys.clear();
            }
        }
        let ai = &mut self.ai;
        ai.timeout_secs = ai.timeout_secs.clamp(3, 120);
        for p in &mut ai.providers {
            if !["openai", "anthropic"].contains(&p.kind.as_str()) {
                p.kind = "openai".into();
            }
            p.base_url = p.base_url.trim().trim_end_matches('/').to_string();
            p.model = p.model.trim().to_string();
        }
        for p in &mut ai.profiles {
            p.model = p.model.trim().to_string();
            tidy(&mut p.apps, 50);
            tidy(&mut p.titles, 50);
        }
    }
}
/// Trimmed, without empty entries or case-insensitive duplicates, at most `max`.
fn tidy(list: &mut Vec<String>, max: usize) {
    for item in list.iter_mut() {
        *item = item.trim().chars().take(80).collect();
    }
    list.retain(|w| !w.is_empty());
    let mut seen = std::collections::HashSet::new();
    list.retain(|w| seen.insert(w.to_lowercase()));
    list.truncate(max);
}
impl Settings {
    /// AI editing is on and has somewhere to send text.
    pub fn ai_ready(&self) -> bool {
        self.ai.enabled && !self.ai.providers.is_empty()
    }
}
pub const VK_RCONTROL: u32 = 0xA3;

/// In combinations either side of a modifier works, and keys read Ctrl, Alt, Shift, Win, key.
/// A single modifier such as Right Ctrl keeps its side.
pub fn normalize_hotkey(keys: &mut Vec<u32>) {
    // AltGr arrives as Left Ctrl + Right Alt.
    if keys.contains(&0xA5) && keys.contains(&0xA2) {
        keys.retain(|&k| k != 0xA2);
    }
    if keys.len() > 1 {
        for key in keys.iter_mut() {
            *key = match *key {
                0xA0 | 0xA1 => 0x10,
                0xA2 | 0xA3 => 0x11,
                0xA4 | 0xA5 => 0x12,
                0x5C => 0x5B,
                other => other,
            };
        }
    }
    let rank = |k: u32| match k {
        0x11 | 0xA2 | 0xA3 => 0,
        0x12 | 0xA4 | 0xA5 => 1,
        0x10 | 0xA0 | 0xA1 => 2,
        0x5B | 0x5C => 3,
        _ => 4,
    };
    keys.sort_by_key(|&k| (rank(k), k));
    keys.dedup();
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub text: String,
    /// The app the text went into; empty for dictations kept in Vorto.
    #[serde(default)]
    pub app: String,
    pub at: String,
    pub seconds: f32,
    /// The words as spoken, when AI editing changed them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw: String,
}

pub struct Store {
    pub root: PathBuf,
    pub settings: Settings,
    pub history: Vec<Entry>,
    pub warning: Option<String>,
    /// No preferences were saved before: this is the first start.
    pub fresh: bool,
}
impl Store {
    pub fn open() -> Result<Self> {
        let root = std::env::var_os("VORTO_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
                    .join(APP_ID)
            });
        fs::create_dir_all(root.join("models"))?;
        let path = root.join("settings.json");
        let fresh = !path.exists();
        let (mut settings, warning) = if path.exists() {
            match fs::read(&path)
                .map_err(anyhow::Error::from)
                .and_then(|b| Ok(serde_json::from_slice::<Settings>(&b)?))
            {
                Ok(s) => (s, None),
                Err(_) => (
                    Settings::default(),
                    Some(
                        "Your saved preferences couldn't be read, so Vorto uses its defaults."
                            .into(),
                    ),
                ),
            }
        } else {
            (Settings::default(), None)
        };
        settings.validate();
        let mut history: Vec<Entry> = fs::read(root.join("history.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        history.truncate(100);
        Ok(Self {
            root,
            settings,
            history,
            warning,
            fresh,
        })
    }
    pub fn save(&self) -> Result<()> {
        atomic_json(&self.root.join("settings.json"), &self.settings)
    }
    pub fn add(&mut self, text: String, seconds: f32, app: String) -> Result<()> {
        self.add_edited(text, String::new(), seconds, app)
    }
    /// `raw` is the dictation as spoken when AI editing changed it, else empty.
    pub fn add_edited(
        &mut self,
        text: String,
        raw: String,
        seconds: f32,
        app: String,
    ) -> Result<()> {
        if self.settings.history {
            self.history.insert(
                0,
                Entry {
                    raw: if raw == text { String::new() } else { raw },
                    text,
                    app,
                    at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                    seconds,
                },
            );
            self.history.truncate(100);
            atomic_json(&self.root.join("history.json"), &self.history)?;
        }
        Ok(())
    }
    /// The newest entry never reached its app: show it as kept in Vorto.
    pub fn forget_app(&mut self, text: &str) -> Result<()> {
        match self.history.first_mut() {
            Some(e) if e.text == text && !e.app.is_empty() => {
                e.app.clear();
                atomic_json(&self.root.join("history.json"), &self.history)
            }
            _ => Ok(()),
        }
    }
    pub fn clear(&mut self) -> Result<()> {
        atomic_json(&self.root.join("history.json"), &Vec::<Entry>::new())?;
        self.history.clear();
        Ok(())
    }
}
pub fn atomic_json(path: &Path, data: &impl Serialize) -> Result<()> {
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing data directory")?)?;
    file.write_all(&serde_json::to_vec_pretty(data)?)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub fn model_dir(root: &Path, model: &str) -> PathBuf {
    root.join("models").join(model)
}
/// Complete only after every file arrived and passed verification.
pub fn installed(root: &Path, id: &str) -> bool {
    let Some(model) = model(id) else {
        return false;
    };
    let dir = model_dir(root, id);
    dir.join("verified").is_file()
        && model
            .files
            .iter()
            .all(|f| fs::metadata(dir.join(f.name)).is_ok_and(|m| m.is_file() && m.len() == f.size))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_preferences_are_normalized() {
        let mut s = Settings {
            model: "../../bad".into(),
            hotkey: vec![],
            idle_minutes: u64::MAX,
            ..Default::default()
        };
        s.validate();
        assert_eq!(s.model, DEFAULT_MODEL);
        assert_eq!(s.hotkey, vec![VK_RCONTROL]);
        assert_eq!(s.idle_minutes, 0);
    }
    #[test]
    fn combinations_use_either_modifier_side() {
        let mut keys = vec![0x20, 0xA0, 0xA3];
        normalize_hotkey(&mut keys);
        assert_eq!(keys, vec![0x11, 0x10, 0x20]);
        let mut single = vec![0xA3];
        normalize_hotkey(&mut single);
        assert_eq!(single, vec![0xA3]);
        let mut altgr = vec![0xA2, 0xA5];
        normalize_hotkey(&mut altgr);
        assert_eq!(altgr, vec![0xA5]);
    }
    #[test]
    fn missing_preferences_use_defaults() {
        let s: Settings = serde_json::from_str(r#"{"toggle":true}"#).unwrap();
        assert!(s.toggle);
        assert_eq!(s.hotkey, vec![VK_RCONTROL]);
        assert_eq!(s.model, DEFAULT_MODEL);
    }
    #[test]
    fn styles_match_apps_and_titles() {
        let ai = AiSettings::default();
        let id = |exe, title| ai.profile_for(exe, title).map(|p| p.id.as_str());
        assert_eq!(id("OUTLOOK.EXE", "Inbox"), Some("email"));
        assert_eq!(id("chrome.exe", "Posteingang - Gmail"), Some("email"));
        assert_eq!(id("chrome.exe", "New chat - Claude"), Some("prompt"));
        assert_eq!(id("notepad.exe", "Untitled"), Some("clean"));
        let mut off = ai.clone();
        off.profiles
            .iter_mut()
            .for_each(|p| p.enabled = p.id != "clean");
        assert!(off.profile_for("notepad.exe", "").is_none());
    }
    #[test]
    fn local_providers_are_recognized() {
        let p = |url: &str| AiProvider {
            base_url: url.into(),
            ..Default::default()
        };
        assert!(p("http://localhost:11434/v1").local());
        assert!(p("http://127.0.0.1:1234/v1").local());
        assert!(p("http://[::1]:8080").local());
        assert!(!p("https://api.openai.com/v1").local());
        assert!(!p("https://localhost.evil.com/v1").local());
    }
    #[test]
    fn atomic_save_replaces_existing_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("config.json");
        atomic_json(&p, &1).unwrap();
        atomic_json(&p, &2).unwrap();
        assert_eq!(fs::read_to_string(p).unwrap(), "2");
    }
    #[test]
    fn every_model_file_is_pinned() {
        let hex = |s: &str, len: usize| {
            s.len() == len && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        };
        for m in MODELS {
            assert!(hex(m.revision, 40), "{} revision", m.id);
            assert!(!m.files.is_empty());
            for f in m.files {
                assert!(f.size > 0 && hex(f.sha256, 64), "{} {}", m.id, f.name);
            }
        }
    }
    #[test]
    fn partial_download_is_never_installed() {
        let d = tempfile::tempdir().unwrap();
        let dir = model_dir(d.path(), "whisper-base");
        fs::create_dir_all(&dir).unwrap();
        let file = fs::File::create(dir.join("ggml-base.bin")).unwrap();
        file.set_len(147_951_465).unwrap();
        assert!(!installed(d.path(), "whisper-base"));
        fs::write(dir.join("verified"), "x").unwrap();
        assert!(installed(d.path(), "whisper-base"));
        file.set_len(1_000).unwrap();
        assert!(
            !installed(d.path(), "whisper-base"),
            "a file of the wrong size is not installed"
        );
    }
}
