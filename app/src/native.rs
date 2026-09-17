//! Windows integration: the target app, its icon, and inserting text.
use crate::hotkey::INJECT_TAG;
use base64::Engine as _;
use std::{collections::HashMap, sync::Mutex, time::Duration};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GlobalFree, ERROR_FILE_NOT_FOUND, HGLOBAL, HWND, RECT},
    Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetMonitorInfoW, GetObjectW,
        MonitorFromWindow, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, MONITORINFO,
        MONITOR_DEFAULTTOPRIMARY,
    },
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
    System::{
        DataExchange::{
            CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats,
            GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable,
            OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        Threading::{
            AttachThreadInput, GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
            OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        },
        Shell::ExtractIconExW,
        WindowsAndMessaging::{
            BringWindowToTop, DestroyIcon, GetForegroundWindow, GetIconInfo, GetWindowLongPtrW,
            GetWindowRect, GetWindowThreadProcessId, IsHungAppWindow, IsWindow,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HICON,
            HWND_TOPMOST, ICONINFO, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
        },
    },
};

const CF_UNICODETEXT: u32 = 13;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}
pub fn is_own(hwnd: HWND) -> bool {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid == unsafe { GetCurrentProcessId() }
}
pub fn is_window(hwnd: HWND) -> bool {
    hwnd != 0 && unsafe { IsWindow(hwnd) } != 0
}

#[derive(Clone, serde::Serialize, PartialEq)]
pub struct Target {
    pub name: String,
    /// PNG data URL of the program icon, or empty.
    pub icon: String,
}

static TARGETS: Mutex<Option<HashMap<String, Target>>> = Mutex::new(None);

/// Readable name and icon of the program behind a window, cached per executable.
pub fn target(hwnd: HWND) -> Option<Target> {
    let path = exe_path(hwnd)?;
    let mut cache = TARGETS.lock().ok()?;
    let cache = cache.get_or_insert_with(HashMap::new);
    if let Some(found) = cache.get(&path) {
        return Some(found.clone());
    }
    let name = file_description(&path)
        .or_else(|| {
            let stem = std::path::Path::new(&path)
                .file_stem()?
                .to_string_lossy()
                .to_string();
            let mut chars = stem.chars();
            let first = chars.next()?;
            Some(first.to_uppercase().collect::<String>() + chars.as_str())
        })
        .map(|n| n.chars().take(32).collect::<String>())?;
    let found = Target {
        name,
        icon: icon_data_url(&path).unwrap_or_default(),
    };
    cache.insert(path, found.clone());
    Some(found)
}

fn exe_path(hwnd: HWND) -> Option<String> {
    if !is_window(hwnd) {
        return None;
    }
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        (ok != 0 && len > 0).then(|| String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

/// "Google Chrome" instead of "chrome": the description Explorer shows.
fn file_description(path: &str) -> Option<String> {
    unsafe {
        let path = wide(path);
        let size = GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr() as *mut _) == 0 {
            return None;
        }
        let mut block = std::ptr::null_mut();
        let mut len = 0u32;
        let translation = wide(r"\VarFileInfo\Translation");
        if VerQueryValueW(
            data.as_ptr() as *const _,
            translation.as_ptr(),
            &mut block,
            &mut len,
        ) == 0
            || len < 4
        {
            return None;
        }
        let lang = *(block as *const u16);
        let codepage = *(block as *const u16).add(1);
        let query = wide(&format!(
            "\\StringFileInfo\\{lang:04x}{codepage:04x}\\FileDescription"
        ));
        let mut value = std::ptr::null_mut();
        let mut value_len = 0u32;
        if VerQueryValueW(
            data.as_ptr() as *const _,
            query.as_ptr(),
            &mut value,
            &mut value_len,
        ) == 0
            || value_len == 0
        {
            return None;
        }
        let text = std::slice::from_raw_parts(value as *const u16, value_len as usize);
        let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
        let name = String::from_utf16_lossy(&text[..end]).trim().to_string();
        (!name.is_empty()).then_some(name)
    }
}

fn icon_data_url(path: &str) -> Option<String> {
    unsafe {
        let path = wide(path);
        let mut large: HICON = 0;
        let mut small: HICON = 0;
        if ExtractIconExW(path.as_ptr(), 0, &mut large, &mut small, 1) == 0 {
            return None;
        }
        let png = icon_png(if large != 0 { large } else { small });
        if large != 0 {
            DestroyIcon(large);
        }
        if small != 0 {
            DestroyIcon(small);
        }
        png.map(|bytes| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        })
    }
}

unsafe fn icon_png(icon: HICON) -> Option<Vec<u8>> {
    let mut info: ICONINFO = std::mem::zeroed();
    if GetIconInfo(icon, &mut info) == 0 {
        return None;
    }
    let result = (|| {
        let mut bitmap: BITMAP = std::mem::zeroed();
        if info.hbmColor == 0
            || GetObjectW(
                info.hbmColor,
                std::mem::size_of::<BITMAP>() as i32,
                &mut bitmap as *mut _ as *mut _,
            ) == 0
        {
            return None;
        }
        let (w, h) = (bitmap.bmWidth, bitmap.bmHeight);
        let mut header: BITMAPINFO = std::mem::zeroed();
        header.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            ..std::mem::zeroed()
        };
        let dc = CreateCompatibleDC(0);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let rows = GetDIBits(
            dc,
            info.hbmColor,
            0,
            h as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut header,
            DIB_RGB_COLORS,
        );
        DeleteDC(dc);
        if rows == 0 {
            return None;
        }
        let has_alpha = pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
        for p in pixels.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            }
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&pixels, w as u32, h as u32, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(png)
    })();
    if info.hbmColor != 0 {
        DeleteObject(info.hbmColor);
    }
    if info.hbmMask != 0 {
        DeleteObject(info.hbmMask);
    }
    result
}
use image::ImageEncoder as _;

/// Icons of every app seen this session, keyed by display name.
pub fn known_icons() -> HashMap<String, String> {
    TARGETS
        .lock()
        .ok()
        .and_then(|cache| {
            cache.as_ref().map(|c| {
                c.values()
                    .filter(|t| !t.icon.is_empty())
                    .map(|t| (t.name.clone(), t.icon.clone()))
                    .collect()
            })
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------- autostart

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// The installer's product name: its uninstaller removes the value with this name.
const RUN_VALUE: &str = "Vorto";
/// Where Task Manager and Settings record that the user turned a startup entry off.
const STARTUP_APPROVED: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

fn autostart_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\" --hidden", exe.display()))
}

/// A value under HKEY_CURRENT_USER as raw bytes, or None when it is missing.
fn registry_value(subkey: &str, name: &str, kind: u32) -> Option<Vec<u8>> {
    use windows_sys::Win32::System::Registry::*;
    let (subkey, name) = (wide(subkey), wide(name));
    unsafe {
        let mut len = 0u32;
        if RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            name.as_ptr(),
            kind,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut len,
        ) != 0
        {
            return None;
        }
        let mut data = vec![0u8; len as usize];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            name.as_ptr(),
            kind,
            std::ptr::null_mut(),
            data.as_mut_ptr() as *mut _,
            &mut len,
        ) != 0
        {
            return None;
        }
        data.truncate(len as usize);
        Some(data)
    }
}

/// On only when the entry starts this executable and Windows has not disabled it.
pub fn autostart_enabled() -> bool {
    use windows_sys::Win32::System::Registry::{RRF_RT_REG_BINARY, RRF_RT_REG_SZ};
    let Some(expected) = autostart_command() else {
        return false;
    };
    let Some(data) = registry_value(RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ) else {
        return false;
    };
    let units: Vec<u16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect();
    let stored = String::from_utf16_lossy(&units);
    if !stored
        .trim_end_matches('\0')
        .eq_ignore_ascii_case(&expected)
    {
        return false;
    }
    // An odd first byte (e.g. 03) means the user disabled the entry.
    !registry_value(STARTUP_APPROVED, RUN_VALUE, RRF_RT_REG_BINARY)
        .is_some_and(|d| d.first().is_some_and(|&b| b & 1 == 1))
}

/// Starts Vorto quietly in the tray when Windows signs in.
pub fn set_autostart(on: bool) -> bool {
    use windows_sys::Win32::System::Registry::*;
    let command = if on {
        match autostart_command() {
            Some(command) => Some(wide(&command)),
            None => return false,
        }
    } else {
        None
    };
    unsafe {
        let mut key = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        ) != 0
        {
            return false;
        }
        let ok = if let Some(command) = command {
            let written = RegSetValueExW(
                key,
                wide(RUN_VALUE).as_ptr(),
                0,
                REG_SZ,
                command.as_ptr() as *const u8,
                (command.len() * 2) as u32,
            ) == 0;
            if written {
                // Turning it on here overrides an earlier "disabled" from Task Manager.
                RegDeleteKeyValueW(
                    HKEY_CURRENT_USER,
                    wide(STARTUP_APPROVED).as_ptr(),
                    wide(RUN_VALUE).as_ptr(),
                );
            }
            written
        } else {
            matches!(
                RegDeleteValueW(key, wide(RUN_VALUE).as_ptr()),
                0 | ERROR_FILE_NOT_FOUND
            )
        };
        RegCloseKey(key);
        ok
    }
}

/// Puts the startup entry back when it went missing, for example because installing over an
/// older version uninstalled that one first. Leaves a "disabled" from Task Manager alone.
pub fn restore_autostart() {
    use windows_sys::Win32::System::Registry::RRF_RT_REG_SZ;
    let current = autostart_command().map(|c| wide(&c));
    let stored = registry_value(RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ);
    if let (Some(command), Some(stored)) = (&current, &stored) {
        let stored: Vec<u16> = stored
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .collect();
        if &stored == command {
            return;
        }
    }
    unsafe {
        use windows_sys::Win32::System::Registry::*;
        let Some(command) = current else { return };
        let mut key = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        ) == 0
        {
            RegSetValueExW(
                key,
                wide(RUN_VALUE).as_ptr(),
                0,
                REG_SZ,
                command.as_ptr() as *const u8,
                (command.len() * 2) as u32,
            );
            RegCloseKey(key);
        }
    }
}

// ---------------------------------------------------------------- inserting text

fn key(vk: u16, scan: u16, flags: u32) -> INPUT {
    let mut input: INPUT = unsafe { std::mem::zeroed() };
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: scan,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: INJECT_TAG,
    };
    input
}
fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
fn down(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}

/// Waits until the user lets go of modifier keys, so Ctrl+V is not Ctrl+Shift+V.
pub fn wait_for_modifiers(limit: Duration) {
    let start = std::time::Instant::now();
    while [0x10, 0x11, 0x12, 0x5B, 0x5C].iter().any(|&k| down(k)) && start.elapsed() < limit {
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn focus(hwnd: HWND) {
    unsafe {
        if !is_window(hwnd) || GetForegroundWindow() == hwnd {
            return;
        }
        if SetForegroundWindow(hwnd) != 0 {
            return;
        }
        // Share this thread's input state with the foreground window's thread, never two
        // other apps' threads with each other, and skip hung windows that would block.
        let fg = GetForegroundWindow();
        let me = GetCurrentThreadId();
        let fg_thread = if fg != 0 {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        } else {
            0
        };
        if fg_thread != 0
            && fg_thread != me
            && IsHungAppWindow(fg) == 0
            && IsHungAppWindow(hwnd) == 0
            && AttachThreadInput(me, fg_thread, 1) != 0
        {
            SetForegroundWindow(hwnd);
            BringWindowToTop(hwnd);
            AttachThreadInput(me, fg_thread, 0);
        }
    }
}

pub fn type_text(text: &str) {
    let mut inputs = Vec::new();
    for unit in text.encode_utf16() {
        inputs.push(key(0, unit, KEYEVENTF_UNICODE));
        inputs.push(key(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    for chunk in inputs.chunks(200) {
        send(chunk);
        std::thread::sleep(Duration::from_millis(4));
    }
}

fn paste_keys() {
    let v_scan = unsafe { MapVirtualKeyW(0x56, 0) } as u16;
    let ctrl_scan = unsafe { MapVirtualKeyW(0xA2, 0) } as u16;
    send(&[
        key(0xA2, ctrl_scan, 0),
        key(0x56, v_scan, 0),
        key(0x56, v_scan, KEYEVENTF_KEYUP),
        key(0xA2, ctrl_scan, KEYEVENTF_KEYUP),
    ]);
}

pub enum Outcome {
    Inserted,
    FocusLost,
    /// The app runs as administrator, and Windows ignores input sent to it from Vorto.
    Elevated,
}

/// Whether a process token is elevated; None when Windows won't say.
fn token_elevated(process: isize) -> Option<bool> {
    unsafe {
        let mut token = 0;
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        ) != 0;
        CloseHandle(token);
        ok.then_some(elevation.TokenIsElevated != 0)
    }
}

/// Windows drops simulated keys that go from a normal app to one running as administrator,
/// without reporting it.
fn blocks_input(hwnd: HWND) -> bool {
    if token_elevated(unsafe { GetCurrentProcess() }) == Some(true) {
        return false;
    }
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return false;
        }
        let elevated = token_elevated(process);
        CloseHandle(process);
        // A token Windows won't show belongs to a protected process, such as a game with
        // anti-cheat, as often as to an elevated one: insert as usual then.
        elevated == Some(true)
    }
}

/// The clipboard restore after the last paste. Holding this lock for a whole insertion
/// keeps a second paste from reading the first transcript as the clipboard to restore.
static RESTORE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

/// Puts `text` into `hwnd`; call off the UI thread. With `restore`, a plain-text clipboard
/// is put back after the paste unless something else was copied meanwhile.
pub fn insert(text: &str, hwnd: HWND, paste: bool, restore: bool) -> Outcome {
    let mut pending = RESTORE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(previous) = pending.take() {
        let _ = previous.join();
    }
    wait_for_modifiers(Duration::from_millis(1500));
    if unsafe { GetForegroundWindow() } != hwnd {
        focus(hwnd);
        std::thread::sleep(Duration::from_millis(40));
    }
    if unsafe { GetForegroundWindow() } != hwnd {
        return Outcome::FocusLost;
    }
    if blocks_input(hwnd) {
        return Outcome::Elevated;
    }
    // Only plain text can be put back exactly. Images, files, rich text or data a password
    // manager hid from clipboard monitors stay untouched: type instead.
    let (has_text, has_other) = clipboard_contents();
    if !paste || (has_other && (restore || !has_text)) {
        type_text(text);
        return Outcome::Inserted;
    }
    let restore = restore && !reads_clipboard_late(hwnd);
    let saved = if restore { clipboard_text() } else { None };
    if restore && has_text && saved.is_none() {
        type_text(text);
        return Outcome::Inserted;
    }
    if !set_clipboard_private(text) {
        if unsafe { CountClipboardFormats() } == 0 {
            if let Some(old) = &saved {
                set_clipboard_private(old);
            }
        }
        type_text(text);
        return Outcome::Inserted;
    }
    let sequence = unsafe { GetClipboardSequenceNumber() };
    std::thread::sleep(Duration::from_millis(10));
    paste_keys();
    if let Some(old) = saved {
        *pending = Some(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1200));
            // A newer copy by the user or another app wins.
            if unsafe { GetClipboardSequenceNumber() } == sequence {
                set_clipboard_private(&old);
            }
        }));
    }
    Outcome::Inserted
}

/// Remote desktop and virtual machine windows fetch pasted data over their connection,
/// possibly after a restore, which would paste the old clipboard instead.
fn reads_clipboard_late(hwnd: HWND) -> bool {
    const CLIENTS: [&str; 7] = [
        "mstsc.exe",
        "msrdc.exe",
        "vmconnect.exe",
        "wfica32.exe",
        "cdviewer.exe",
        "vmware.exe",
        "virtualboxvm.exe",
    ];
    exe_path(hwnd).is_some_and(|path| {
        let name = path
            .rsplit('\\')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        CLIENTS.contains(&name.as_str())
    })
}

fn open_clipboard() -> bool {
    for _ in 0..10 {
        if unsafe { OpenClipboard(0) } != 0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    false
}
/// Whether the clipboard holds text, and whether it holds anything besides plain text
/// (also true when it cannot be inspected).
fn clipboard_contents() -> (bool, bool) {
    unsafe {
        let text = IsClipboardFormatAvailable(CF_UNICODETEXT) != 0;
        if CountClipboardFormats() == 0 {
            return (false, false);
        }
        if !open_clipboard() {
            return (text, true);
        }
        let private = privacy_formats();
        let mut other = false;
        let mut format = 0;
        loop {
            format = EnumClipboardFormats(format);
            if format == 0 {
                break;
            }
            // CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT, CF_LOCALE, and the privacy flags that
            // a restore writes back.
            if !matches!(format, 1 | 7 | 13 | 16) && !private.contains(&format) {
                other = true;
                break;
            }
        }
        CloseClipboard();
        (text, other)
    }
}
fn clipboard_text() -> Option<String> {
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 || !open_clipboard() {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        let text = if handle == 0 {
            None
        } else {
            let ptr = GlobalLock(handle as _) as *const u16;
            if ptr.is_null() {
                None
            } else {
                // Another process may have left out the terminator: stay inside the block.
                let units = std::slice::from_raw_parts(ptr, GlobalSize(handle as _) / 2);
                let len = units.iter().position(|&c| c == 0).unwrap_or(units.len());
                let text = String::from_utf16_lossy(&units[..len]);
                GlobalUnlock(handle as _);
                Some(text)
            }
        };
        CloseClipboard();
        text
    }
}
/// Registered formats that, set to 0, keep clipboard data out of Windows clipboard
/// history and cloud sync.
fn privacy_formats() -> [u32; 2] {
    ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"]
        .map(|name| unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) })
}
/// A movable memory block holding a copy of `data`, or null.
unsafe fn global_copy<T: Copy>(data: &[T]) -> HGLOBAL {
    let handle = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of_val(data));
    if handle.is_null() {
        return handle;
    }
    let ptr = GlobalLock(handle) as *mut T;
    if ptr.is_null() {
        GlobalFree(handle);
        return std::ptr::null_mut();
    }
    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    GlobalUnlock(handle);
    handle
}
/// Puts text on the clipboard as a normal entry, for text the user chose to copy.
pub fn set_clipboard(text: &str) -> bool {
    write_clipboard(text, false)
}
/// Puts text on the clipboard but keeps it out of Windows clipboard history and cloud
/// sync: for a transcript that is only there to be pasted, and for restoring.
pub fn set_clipboard_private(text: &str) -> bool {
    write_clipboard(text, true)
}
fn write_clipboard(text: &str, private: bool) -> bool {
    unsafe {
        // Prepare the data before emptying the clipboard, so a failure leaves it as it was.
        let handle = global_copy(&wide(text));
        if handle.is_null() {
            return false;
        }
        if !open_clipboard() {
            GlobalFree(handle);
            return false;
        }
        EmptyClipboard();
        let ok = SetClipboardData(CF_UNICODETEXT, handle as _) != 0;
        if !ok {
            GlobalFree(handle);
        } else if private {
            for format in privacy_formats() {
                let flag = global_copy(&[0u32]);
                if !flag.is_null() && (format == 0 || SetClipboardData(format, flag as _) == 0) {
                    GlobalFree(flag);
                }
            }
        }
        CloseClipboard();
        ok
    }
}

// ---------------------------------------------------------------- indicator window

/// Never takes focus, ignores the mouse, and stays out of Alt+Tab. tao rewrites the
/// extended styles whenever it changes the window's flags, so showing checks them again.
pub fn prepare_overlay(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let wanted = (style
            | (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW) as isize)
            & !(WS_EX_APPWINDOW as isize);
        if wanted != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}
/// Shows the overlay centered at the top or bottom of the monitor that holds `near`.
/// `size` is only a fallback: on a monitor with another scale the window changes size.
pub fn show_overlay(hwnd: HWND, near: HWND, size: (i32, i32), bottom: bool) {
    unsafe {
        prepare_overlay(hwnd);
        let monitor = MonitorFromWindow(
            if is_window(near) { near } else { foreground() },
            MONITOR_DEFAULTTOPRIMARY,
        );
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let area: RECT = if GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        };
        let current = || {
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) != 0 {
                (rect.right - rect.left, rect.bottom - rect.top)
            } else {
                size
            }
        };
        let place = |flags: u32| {
            let (w, h) = current();
            let x = area.left + (area.right - area.left - w) / 2;
            let y = if bottom {
                area.bottom - h - 10
            } else {
                area.top + 10
            };
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | flags,
            );
            (w, h)
        };
        // The first move lets Windows rescale the window for that monitor, the second
        // centers it with its real size; recenter if the scale only applied once shown.
        place(0);
        let placed = place(SWP_SHOWWINDOW);
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        if current() != placed {
            place(0);
        }
    }
}
pub fn hide_overlay(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }
}
