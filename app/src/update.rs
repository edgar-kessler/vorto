//! Updates from GitHub Releases through Tauri's updater. Installers are signed; the updater
//! refuses any that doesn't match the public key built into the app.
use crate::controller::Msg;
use serde::Serialize;
use std::sync::mpsc::Sender;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    /// idle, checking, latest, available, downloading, ready, installing or error
    pub status: &'static str,
    /// The version on offer.
    pub version: String,
    /// 0 to 1 while downloading, -1 when the size is unknown.
    pub progress: f32,
}
impl Default for UpdateView {
    fn default() -> Self {
        Self {
            status: "idle",
            version: String::new(),
            progress: -1.0,
        }
    }
}

pub enum UpdateMsg {
    /// The result of a check; `asked` when the user clicked the button.
    Checked {
        found: Result<Option<Box<Update>>, String>,
        asked: bool,
    },
    Progress(f32),
    Downloaded(Result<Vec<u8>, String>),
}

pub fn check(app: &AppHandle, tx: Sender<Msg>, asked: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // The installer restarts Vorto without the arguments this process got: after
        // "Restart to update" the window opens even if Vorto started hidden at sign-in.
        let found = match app.updater_builder().installer_arg("/R").build() {
            Ok(updater) => updater
                .check()
                .await
                .map(|u| u.map(|u| Box::new(u.restart_after_install(false))))
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        let _ = tx.send(Msg::Update(UpdateMsg::Checked { found, asked }));
    });
}

pub fn download(update: Update, tx: Sender<Msg>) {
    tauri::async_runtime::spawn(async move {
        let mut received = 0u64;
        let mut reported = -1.0f32;
        let progress = tx.clone();
        let result = update
            .download(
                move |chunk, total| {
                    received += chunk as u64;
                    let fraction =
                        total.map_or(-1.0, |t| (received as f64 / t.max(1) as f64) as f32);
                    // Whole percents are enough for a progress bar.
                    if fraction < 0.0 || fraction - reported >= 0.01 {
                        reported = fraction;
                        let _ = progress.send(Msg::Update(UpdateMsg::Progress(fraction)));
                    }
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(Msg::Update(UpdateMsg::Downloaded(result)));
    });
}

/// Starts the installer. On Windows it runs the setup and this process exits.
pub fn install(update: &Update, bytes: &[u8]) -> Result<(), String> {
    update.install(bytes).map_err(|e| e.to_string())
}
