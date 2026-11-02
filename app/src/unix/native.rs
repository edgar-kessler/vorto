//! What the app asks of macOS and Linux: the app in front, inserting text, the clipboard, starting
//! at sign-in and the overlay windows. A "window" here is the process id of the app in front.
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::{collections::HashMap, sync::OnceLock, time::Duration};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

pub type HWND = isize;
/// The overlay windows, in place of the handles Windows gives them.
pub const HUD: HWND = 1;
pub const FLASH: HWND = 2;

static APP: OnceLock<AppHandle> = OnceLock::new();
pub fn init(app: AppHandle) {
    let _ = APP.set(app);
}

fn overlay(handle: HWND) -> Option<tauri::WebviewWindow> {
    APP.get()?
        .get_webview_window(if handle == HUD { "hud" } else { "flash" })
}

pub fn message_box(text: &str, question: bool) -> bool {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    let shown = {
        let quoted = text.replace('\\', "\\\\").replace('"', "\\\"");
        let buttons = if question {
            "buttons {\"Cancel\", \"OK\"} default button \"OK\""
        } else {
            "buttons {\"OK\"} default button \"OK\""
        };
        Command::new("osascript")
            .args([
                "-e",
                &format!("display dialog \"{quoted}\" with title \"Vorto\" {buttons}"),
            ])
            .status()
    };
    #[cfg(not(target_os = "macos"))]
    let shown = Command::new("zenity")
        .args([
            if question { "--question" } else { "--info" },
            "--title=Vorto",
            &format!("--text={text}"),
        ])
        .status();
    shown.is_ok_and(|status| status.success())
}

fn front() -> Option<active_win_pos_rs::ActiveWindow> {
    active_win_pos_rs::get_active_window().ok()
}
pub fn foreground() -> HWND {
    front().map_or(0, |w| w.process_id as isize)
}
pub fn is_own(hwnd: HWND) -> bool {
    hwnd == std::process::id() as isize
}
pub fn is_window(hwnd: HWND) -> bool {
    hwnd > 2
}
pub fn is_shell(_: HWND) -> bool {
    false
}

#[derive(Clone, serde::Serialize, PartialEq)]
pub struct Target {
    pub name: String,
    /// PNG data URL of the program icon, or empty.
    pub icon: String,
}

pub fn target(hwnd: HWND) -> Option<Target> {
    let window = front().filter(|w| w.process_id as isize == hwnd)?;
    Some(Target {
        name: window.app_name,
        icon: String::new(),
    })
}
pub fn exe_name(hwnd: HWND) -> String {
    program_path(hwnd)
        .and_then(|p| {
            std::path::Path::new(&p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}
pub fn window_title(hwnd: HWND) -> String {
    front()
        .filter(|w| w.process_id as isize == hwnd)
        .map_or_else(String::new, |w| w.title)
}
pub fn program_path(hwnd: HWND) -> Option<String> {
    front()
        .filter(|w| w.process_id as isize == hwnd)
        .map(|w| w.process_path.to_string_lossy().into_owned())
}
pub fn icon_for(_: &str) -> String {
    String::new()
}
pub fn known_icons() -> HashMap<String, String> {
    HashMap::new()
}

fn starter() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.to_str()?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Vorto")
        .set_app_path(path)
        .set_args(&["--hidden"])
        .build()
        .ok()
}
pub fn autostart_enabled() -> bool {
    starter().is_some_and(|s| s.is_enabled().unwrap_or(false))
}
pub fn set_autostart(on: bool) -> bool {
    starter().is_some_and(|s| {
        if on {
            s.enable().is_ok()
        } else {
            s.disable().is_ok()
        }
    })
}
pub fn restore_autostart() {
    set_autostart(true);
}

pub enum Outcome {
    Inserted,
    FocusLost,
    /// Only Windows keeps input from apps running as administrator; here for the controller.
    #[allow(dead_code)]
    Elevated,
}

/// Cmd on a Mac, Ctrl elsewhere, plus a letter.
fn chord(letter: char) -> bool {
    let Ok(mut keys) = Enigo::new(&EnigoSettings::default()) else {
        return false;
    };
    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    let done = keys.key(modifier, Direction::Press).is_ok()
        && keys.key(Key::Unicode(letter), Direction::Click).is_ok();
    let _ = keys.key(modifier, Direction::Release);
    done
}

pub fn undo(hwnd: HWND) -> bool {
    foreground() == hwnd && chord('z')
}

pub fn set_clipboard(text: &str) -> bool {
    arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(text))
        .is_ok()
}
/// These systems have no shared rule for keeping text out of clipboard managers.
pub fn set_clipboard_private(text: &str) -> bool {
    set_clipboard(text)
}
pub fn finish_restore() {}

pub fn insert(text: &str, hwnd: HWND, paste: bool, restore: bool) -> Outcome {
    if foreground() != hwnd {
        return Outcome::FocusLost;
    }
    if !paste {
        let typed = Enigo::new(&EnigoSettings::default()).and_then(|mut keys| {
            keys.text(text)
                .map_err(|_| enigo::NewConError::NoPermission)
        });
        return if typed.is_ok() {
            Outcome::Inserted
        } else {
            Outcome::FocusLost
        };
    }
    let before = restore
        .then(|| {
            arboard::Clipboard::new()
                .and_then(|mut c| c.get_text())
                .ok()
        })
        .flatten();
    if !set_clipboard(text) {
        return Outcome::FocusLost;
    }
    // Long enough for clipboard owners on X11 and the Mac's pasteboard.
    std::thread::sleep(Duration::from_millis(60));
    if !chord('v') {
        return Outcome::FocusLost;
    }
    if let Some(previous) = before {
        let inserted = text.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1200));
            // Unless something else was copied in the meantime.
            let same = arboard::Clipboard::new()
                .and_then(|mut c| c.get_text())
                .is_ok_and(|now| now == inserted);
            if same {
                set_clipboard(&previous);
            }
        });
    }
    Outcome::Inserted
}

pub fn prepare_overlay(_: HWND) {}

/// Shows the overlay centered at the top or bottom of the main screen.
pub fn show_overlay(handle: HWND, _near: HWND, size: (i32, i32), bottom: bool) {
    let Some(window) = overlay(handle) else {
        return;
    };
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen = monitor.size();
        let place = monitor.position();
        let x = place.x + (screen.width as i32 - size.0) / 2;
        let y = if bottom {
            place.y + screen.height as i32 - size.1 - 48
        } else {
            place.y + 8
        };
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    let _ = window.show();
}
pub fn show_overlay_at(handle: HWND, x: i32, y: i32, w: i32, h: i32) {
    if let Some(window) = overlay(handle) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
        let _ = window.show();
    }
}
pub fn hide_overlay(handle: HWND) {
    if let Some(window) = overlay(handle) {
        let _ = window.hide();
    }
}
