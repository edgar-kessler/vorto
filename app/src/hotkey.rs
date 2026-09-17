//! Global shortcut through low-level keyboard and mouse hooks.
//!
//! Any combination of up to four keys or mouse buttons works. The callbacks only
//! update atomics, send a message and at most inject one masking key: Windows
//! removes hooks that take longer than a few hundred milliseconds. Input that
//! Vorto injects itself carries `INJECT_TAG` and is ignored.
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    mpsc::Sender,
    OnceLock,
};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentProcessId},
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, SendInput, INPUT, INPUT_KEYBOARD,
            KEYBDINPUT, KEYEVENTF_KEYUP,
        },
        WindowsAndMessaging::*,
    },
};

pub const INJECT_TAG: usize = 0x4D55_524D;
const VK_ESCAPE: u32 = 0x1B;

#[derive(Debug)]
pub enum HookEvent {
    Pressed,
    Released,
    Escape,
    /// A shortcut recorded while capturing.
    Captured(Vec<u32>),
}

static SENDER: OnceLock<Sender<HookEvent>> = OnceLock::new();
static DOWN: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static COMBO: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static TOGGLE: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static SATISFIED: AtomicBool = AtomicBool::new(false);
static CAPTURING: AtomicBool = AtomicBool::new(false);
static SWALLOWED: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static CAPTURED: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static RECORDING: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Tick count of the last keyboard event, so the key state is not compared too early.
static LAST_KEY: AtomicU32 = AtomicU32::new(0);
const MODIFIER_CODES: [u32; 11] = [
    0x10, 0x11, 0x12, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5B, 0x5C,
];

fn bit(vk: u32) -> (usize, u64) {
    ((vk as usize / 64).min(3), 1u64 << (vk % 64))
}
fn set(bits: &[AtomicU64; 4], vk: u32, on: bool) {
    let (i, b) = bit(vk);
    if on {
        bits[i].fetch_or(b, Ordering::Relaxed);
    } else {
        bits[i].fetch_and(!b, Ordering::Relaxed);
    }
}
fn has(bits: &[AtomicU64; 4], vk: u32) -> bool {
    let (i, b) = bit(vk);
    bits[i].load(Ordering::Relaxed) & b != 0
}
fn keys(bits: &[AtomicU64; 4]) -> Vec<u32> {
    (1..255).filter(|&vk| has(bits, vk)).collect()
}
fn send(event: HookEvent) {
    if let Some(sender) = SENDER.get() {
        let _ = sender.send(event);
    }
}

/// The side-independent code for a modifier, e.g. Left Ctrl to Ctrl.
fn generic(vk: u32) -> u32 {
    match vk {
        0xA0 | 0xA1 => 0x10,
        0xA2 | 0xA3 => 0x11,
        0xA4 | 0xA5 => 0x12,
        0x5C => 0x5B,
        other => other,
    }
}
fn in_combo(vk: u32) -> bool {
    has(&COMBO, vk) || has(&COMBO, generic(vk))
}
fn is_held(key: u32) -> bool {
    match key {
        0x10 => has(&DOWN, 0xA0) || has(&DOWN, 0xA1) || has(&DOWN, 0x10),
        0x11 => has(&DOWN, 0xA2) || has(&DOWN, 0xA3) || has(&DOWN, 0x11),
        0x12 => has(&DOWN, 0xA4) || has(&DOWN, 0xA5) || has(&DOWN, 0x12),
        0x5B => has(&DOWN, 0x5B) || has(&DOWN, 0x5C),
        other => has(&DOWN, other),
    }
}

pub fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C)
}
/// Whether the shortcut consists only of modifiers, like the default Right Ctrl.
fn modifier_only() -> bool {
    let mut any = false;
    for vk in 1..255 {
        if has(&COMBO, vk) {
            if !is_modifier(vk) {
                return false;
            }
            any = true;
        }
    }
    any
}

/// Windows' own key state. It does not include the event being handled yet, and it misses
/// presses Vorto swallowed, so it only confirms keys whose earlier events passed through.
fn physically_down(vk: u32) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) < 0 }
}

/// Clears modifiers Windows reports as released although their key-up never arrived,
/// e.g. after the lock screen or a UAC prompt. Outside shortcut recording modifiers are
/// never swallowed, so Windows' state for them is reliable.
fn resync_modifiers(skip: u32) {
    for code in MODIFIER_CODES {
        if code != skip && has(&DOWN, code) && !physically_down(code) {
            set(&DOWN, code, false);
        }
    }
}

/// Presses and releases an unassigned key, so a held Alt or Win whose partner key was
/// swallowed does not count as tapped alone (menu bar, Start, layout switch).
fn mask() {
    unsafe {
        let mut inputs: [INPUT; 2] = std::mem::zeroed();
        for (input, flags) in inputs.iter_mut().zip([0, KEYEVENTF_KEYUP]) {
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki = KEYBDINPUT {
                wVk: 0xE8,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECT_TAG,
            };
        }
        SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

/// A key or click while a modifier-only shortcut is held means the modifier was meant
/// for something else, such as Ctrl+C: discard the dictation that press started.
fn abort_chorded() {
    if SATISFIED.load(Ordering::Relaxed)
        && ACTIVE.load(Ordering::Relaxed)
        && modifier_only()
        && ACTIVE.swap(false, Ordering::Relaxed)
    {
        send(HookEvent::Escape);
    }
}

fn own_foreground() -> bool {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        pid == GetCurrentProcessId()
    }
}

pub fn configure(combo: &[u32], toggle: bool) {
    for word in &COMBO {
        word.store(0, Ordering::Relaxed);
    }
    for &vk in combo {
        set(&COMBO, vk, true);
    }
    TOGGLE.store(toggle, Ordering::Relaxed);
    ACTIVE.store(false, Ordering::Relaxed);
    SATISFIED.store(false, Ordering::Relaxed);
}
pub fn begin_capture() {
    for word in &CAPTURED {
        word.store(0, Ordering::Relaxed);
    }
    CAPTURING.store(true, Ordering::Relaxed);
}
pub fn end_capture() {
    CAPTURING.store(false, Ordering::Relaxed);
}
pub fn capturing() -> bool {
    CAPTURING.load(Ordering::Relaxed)
}
/// Lets Escape cancel only while a dictation is running. When a dictation ends, however
/// it ended, the next shortcut press starts a new one.
pub fn set_recording(on: bool) {
    RECORDING.store(on, Ordering::Relaxed);
    if !on {
        ACTIVE.store(false, Ordering::Relaxed);
    }
}
pub fn force_inactive() {
    ACTIVE.store(false, Ordering::Relaxed);
}
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// Returns true when the event must be swallowed.
fn handle(vk: u32, down: bool) -> bool {
    if CAPTURING.load(Ordering::Relaxed) {
        if !own_foreground() {
            // The user left Vorto: stop recording a shortcut and let the input through.
            CAPTURING.store(false, Ordering::Relaxed);
            send(HookEvent::Captured(Vec::new()));
            set(&DOWN, vk, down);
            return false;
        }
        if down {
            if vk == VK_ESCAPE && keys(&CAPTURED).is_empty() {
                CAPTURING.store(false, Ordering::Relaxed);
                send(HookEvent::Captured(Vec::new()));
                return true;
            }
            // Repeats of a key held before recording began are not part of the shortcut.
            if has(&DOWN, vk) && !has(&CAPTURED, vk) && physically_down(vk) {
                return false;
            }
            set(&CAPTURED, vk, true);
            set(&DOWN, vk, true);
            return true;
        }
        let captured = has(&CAPTURED, vk);
        set(&DOWN, vk, false);
        let still_down = keys(&CAPTURED).iter().any(|&k| has(&DOWN, k));
        if !still_down && !keys(&CAPTURED).is_empty() {
            CAPTURING.store(false, Ordering::Relaxed);
            let mut combo = keys(&CAPTURED);
            vorto::data::normalize_hotkey(&mut combo);
            combo.truncate(4);
            send(HookEvent::Captured(combo));
        }
        // Only releases of swallowed presses are swallowed, so no key stays down elsewhere.
        return captured;
    }
    let was_down = has(&DOWN, vk);
    set(&DOWN, vk, down);
    // Escape that cancels a dictation is not also delivered to the app, and neither are
    // its repeats and release.
    if vk == VK_ESCAPE {
        if down && RECORDING.load(Ordering::Relaxed) {
            send(HookEvent::Escape);
            set(&SWALLOWED, vk, true);
            return true;
        }
        if has(&SWALLOWED, vk) {
            set(&SWALLOWED, vk, down);
            return true;
        }
    }
    let combo = keys(&COMBO);
    if combo.is_empty() {
        return false;
    }
    if !in_combo(vk) {
        if down {
            abort_chorded();
        }
        return false;
    }
    if down {
        resync_modifiers(vk);
    }
    let satisfied = combo.iter().all(|&k| is_held(k));
    let before = SATISFIED.swap(satisfied, Ordering::Relaxed);
    if satisfied && !before {
        if TOGGLE.load(Ordering::Relaxed) {
            let now = !ACTIVE.load(Ordering::Relaxed);
            ACTIVE.store(now, Ordering::Relaxed);
            send(if now {
                HookEvent::Pressed
            } else {
                HookEvent::Released
            });
        } else if !ACTIVE.swap(true, Ordering::Relaxed) {
            send(HookEvent::Pressed);
        }
    } else if !satisfied
        && before
        && !TOGGLE.load(Ordering::Relaxed)
        && ACTIVE.swap(false, Ordering::Relaxed)
    {
        send(HookEvent::Released);
    }
    // Swallow the non-modifier part of the shortcut so it does not type or click.
    // Modifiers pass through so Right Ctrl keeps working as Ctrl elsewhere.
    if is_modifier(vk) {
        let alt_or_win = combo.iter().any(|&k| matches!(generic(k), 0x12 | 0x5B));
        if !down && before && !satisfied && (alt_or_win || combo.len() > 1) && modifier_only() {
            mask();
        }
        return false;
    }
    if down {
        // A swallowed key stays swallowed until released, and a key that was already down
        // before the shortcut passes, so every press the system saw also gets its release.
        if has(&SWALLOWED, vk) {
            return true;
        }
        if was_down && physically_down(vk) {
            return false;
        }
        let others = combo
            .iter()
            .filter(|&&k| k != vk && k != generic(vk))
            .all(|&k| is_held(k));
        if others {
            if is_held(0x12) || is_held(0x5B) || (is_held(0x10) && is_held(0x11)) {
                mask();
            }
            set(&SWALLOWED, vk, true);
        }
        others
    } else {
        let swallowed = has(&SWALLOWED, vk);
        set(&SWALLOWED, vk, false);
        swallowed
    }
}

unsafe extern "system" fn keyboard(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Keep this path allocation-free and fast: Windows silently removes slow hooks.
    if code == HC_ACTION as i32 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        LAST_KEY.store(kb.time, Ordering::Relaxed);
        if kb.dwExtraInfo != INJECT_TAG {
            let msg = wparam as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            if (down || up) && handle(kb.vkCode, down) {
                return 1;
            }
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}

unsafe extern "system" fn mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let msg = wparam as u32;
        let button = match msg {
            WM_MBUTTONDOWN | WM_MBUTTONUP => Some(0x04),
            WM_XBUTTONDOWN | WM_XBUTTONUP => Some(if (info.mouseData >> 16) & 0xFFFF == 1 {
                0x05
            } else {
                0x06
            }),
            _ => None,
        };
        let capturing = CAPTURING.load(Ordering::Relaxed);
        if info.dwExtraInfo != INJECT_TAG {
            match button {
                // Only take mouse buttons into a capture once a key or the button itself starts it.
                Some(vk) if capturing || in_combo(vk) => {
                    if handle(vk, matches!(msg, WM_MBUTTONDOWN | WM_XBUTTONDOWN)) {
                        return 1;
                    }
                }
                _ if !capturing
                    && matches!(
                        msg,
                        WM_LBUTTONDOWN
                            | WM_RBUTTONDOWN
                            | WM_MBUTTONDOWN
                            | WM_XBUTTONDOWN
                            | WM_MOUSEWHEEL
                            | WM_MOUSEHWHEEL
                    ) =>
                {
                    abort_chorded()
                }
                _ => {}
            }
        }
    }
    CallNextHookEx(0, code, wparam, lparam)
}

/// Runs on the hook thread between events: drops modifiers that were released unseen and
/// ends a hold-mode dictation whose shortcut is no longer physically held.
fn resync_hold(now: u32) {
    // Right after a key event Windows may not have updated its key state yet.
    if CAPTURING.load(Ordering::Relaxed) || now.wrapping_sub(LAST_KEY.load(Ordering::Relaxed)) < 250
    {
        return;
    }
    resync_modifiers(0);
    let combo = keys(&COMBO);
    if combo.is_empty() || combo.iter().all(|&k| is_held(k)) {
        return;
    }
    if SATISFIED.swap(false, Ordering::Relaxed)
        && !TOGGLE.load(Ordering::Relaxed)
        && ACTIVE.swap(false, Ordering::Relaxed)
    {
        send(HookEvent::Released);
    }
}

/// Installs both hooks on a dedicated, high-priority thread with its own message loop.
///
/// Windows drops low-level hooks without notice when a callback is late, which
/// happens under heavy load such as games. The hooks are therefore renewed every
/// few seconds; the key state lives in statics and survives the renewal.
pub fn start(sender: Sender<HookEvent>) {
    let _ = SENDER.set(sender);
    std::thread::Builder::new()
        .name("vorto-hotkey".into())
        .spawn(|| unsafe {
            use windows_sys::Win32::System::Threading::{
                GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
            };
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
            let module = GetModuleHandleW(std::ptr::null());
            let install = || {
                (
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), module, 0),
                    SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), module, 0),
                )
            };
            let mut hooks = install();
            INSTALLED.store(hooks.0 != 0, Ordering::Relaxed);
            crate::log::write(format!(
                "hooks installed: keyboard={} mouse={}",
                hooks.0 != 0,
                hooks.1 != 0
            ));
            SetTimer(0, 0, 4000, None);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                if msg.message == WM_TIMER {
                    // Renew each hook on its own: keep the old one if its replacement failed.
                    let fresh = install();
                    if fresh.0 != 0 {
                        if hooks.0 != 0 {
                            UnhookWindowsHookEx(hooks.0);
                        }
                        hooks.0 = fresh.0;
                    }
                    if fresh.1 != 0 {
                        if hooks.1 != 0 {
                            UnhookWindowsHookEx(hooks.1);
                        }
                        hooks.1 = fresh.1;
                    }
                    INSTALLED.store(hooks.0 != 0, Ordering::Relaxed);
                    resync_hold(msg.time);
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hooks.0);
            UnhookWindowsHookEx(hooks.1);
        })
        .expect("hotkey thread");
}

/// Human-readable names, e.g. ["Ctrl", "Shift", "Space"].
pub fn names(combo: &[u32]) -> Vec<String> {
    combo.iter().map(|&vk| name(vk)).collect()
}
pub fn name(vk: u32) -> String {
    let fixed = match vk {
        0x04 => "Middle mouse",
        0x05 => "Mouse 4",
        0x06 => "Mouse 5",
        0x10 => "Shift",
        0x11 => "Ctrl",
        0x12 => "Alt",
        0xA0 => "Left Shift",
        0xA1 => "Right Shift",
        0xA2 => "Left Ctrl",
        0xA3 => "Right Ctrl",
        0xA4 => "Left Alt",
        0xA5 => "Right Alt",
        0x5B | 0x5C => "Win",
        0x20 => "Space",
        0x14 => "Caps Lock",
        0x0D => "Enter",
        0x08 => "Backspace",
        0x09 => "Tab",
        0x1B => "Esc",
        0x21 => "Page Up",
        0x22 => "Page Down",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2D => "Insert",
        0x2E => "Delete",
        0x2C => "Print",
        0x13 => "Pause",
        0x91 => "Scroll Lock",
        0x5D => "Menu",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.into();
    }
    if (0x70..=0x87).contains(&vk) {
        return format!("F{}", vk - 0x6F);
    }
    unsafe {
        let scan = MapVirtualKeyW(vk, 0);
        let mut buffer = [0u16; 64];
        let len = GetKeyNameTextW((scan << 16) as i32, buffer.as_mut_ptr(), 64);
        if len > 0 {
            return String::from_utf16_lossy(&buffer[..len as usize]);
        }
    }
    format!("Key {vk}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modifiers_are_recognized() {
        assert!(is_modifier(0xA3));
        assert!(!is_modifier(0x20));
    }
    #[test]
    fn known_names_are_readable() {
        assert_eq!(names(&[0xA2, 0x20]), ["Left Ctrl", "Space"]);
        assert_eq!(name(0x78), "F9");
    }
}
