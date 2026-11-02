//! The dictation shortcut on macOS and Linux. Keys arrive as Windows virtual-key codes, which is
//! what settings.json stores on every system. Unlike the Windows hooks, these listeners only
//! watch: a key the shortcut uses still reaches the app in front.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Mutex, OnceLock,
};

#[derive(Debug)]
pub enum HookEvent {
    Pressed,
    Released,
    Escape,
    /// A shortcut recorded while capturing.
    Captured(Vec<u32>),
    /// One of the extra shortcuts, by its index in `Shortcuts::all`.
    Shortcut(usize),
}

const VK_ESCAPE: u32 = 0x1B;

#[derive(Default)]
struct State {
    down: Vec<u32>,
    combo: Vec<u32>,
    toggle: bool,
    active: bool,
    satisfied: bool,
    capturing: bool,
    captured: Vec<u32>,
    recording: bool,
    escapable: bool,
    extra: Vec<Vec<u32>>,
    extra_held: [bool; 4],
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static SENDER: OnceLock<Sender<HookEvent>> = OnceLock::new();
static INSTALLED: AtomicBool = AtomicBool::new(false);

fn with<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}
fn send(event: HookEvent) {
    if let Some(sender) = SENDER.get() {
        let _ = sender.send(event);
    }
}

pub fn is_modifier(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C)
}
/// The left and right key of a modifier count as one in a combination.
fn generic(vk: u32) -> u32 {
    match vk {
        0xA0 | 0xA1 => 0x10,
        0xA2 | 0xA3 => 0x11,
        0xA4 | 0xA5 => 0x12,
        0x5C => 0x5B,
        other => other,
    }
}
fn held(down: &[u32], wanted: u32) -> bool {
    down.iter().any(|&k| k == wanted || generic(k) == wanted)
}

/// A key went down or up, from the listener of this system.
fn key(vk: u32, is_down: bool) {
    let mut events = Vec::new();
    with(|s| {
        let repeat = is_down && s.down.contains(&vk);
        if is_down && !repeat {
            s.down.push(vk);
        } else if !is_down {
            s.down.retain(|&k| k != vk);
        }
        if s.capturing {
            if is_down && !repeat {
                if vk == VK_ESCAPE && s.captured.is_empty() {
                    s.capturing = false;
                    events.push(HookEvent::Captured(Vec::new()));
                } else if !s.captured.contains(&vk) {
                    s.captured.push(vk);
                }
            } else if !is_down
                && !s.captured.is_empty()
                && !s.captured.iter().any(|k| s.down.contains(k))
            {
                s.capturing = false;
                let mut combo = std::mem::take(&mut s.captured);
                vorto::data::normalize_hotkey(&mut combo);
                combo.truncate(4);
                events.push(HookEvent::Captured(combo));
            }
            return;
        }
        if repeat {
            return;
        }
        if vk == VK_ESCAPE && is_down && (s.recording || s.escapable) {
            events.push(HookEvent::Escape);
            return;
        }
        for index in 0..s.extra.len().min(4) {
            let keys = &s.extra[index];
            let all = !keys.is_empty() && keys.iter().all(|&k| held(&s.down, k));
            if all && !s.extra_held[index] {
                events.push(HookEvent::Shortcut(index));
            }
            s.extra_held[index] = all;
        }
        if s.combo.is_empty() {
            return;
        }
        // A modifier used for something else, like Ctrl+C, doesn't start a dictation.
        let in_combo = s.combo.iter().any(|&k| k == vk || k == generic(vk));
        if !in_combo {
            if is_down
                && !s.toggle
                && s.satisfied
                && s.combo.iter().all(|&k| is_modifier(k))
                && !s.recording
            {
                s.satisfied = false;
                s.active = false;
                events.push(HookEvent::Released);
            }
            return;
        }
        let satisfied = s.combo.iter().all(|&k| held(&s.down, k));
        let before = std::mem::replace(&mut s.satisfied, satisfied);
        if satisfied == before {
            return;
        }
        if s.toggle {
            if satisfied {
                s.active = !s.active;
                events.push(if s.active {
                    HookEvent::Pressed
                } else {
                    HookEvent::Released
                });
            }
        } else {
            s.active = satisfied;
            events.push(if satisfied {
                HookEvent::Pressed
            } else {
                HookEvent::Released
            });
        }
    });
    for event in events {
        send(event);
    }
}

pub fn configure(combo: &[u32], toggle: bool) {
    with(|s| {
        s.combo = combo.to_vec();
        s.toggle = toggle;
        s.active = false;
        s.satisfied = false;
    });
}
pub fn begin_capture() {
    with(|s| {
        s.captured.clear();
        s.capturing = true;
    });
}
pub fn end_capture() {
    with(|s| s.capturing = false);
}
pub fn capturing() -> bool {
    with(|s| s.capturing)
}
pub fn set_recording(on: bool) {
    with(|s| s.recording = on);
}
pub fn set_escapable(on: bool) {
    with(|s| s.escapable = on);
}
pub fn force_inactive() {
    with(|s| {
        s.active = false;
        s.satisfied = false;
    });
}
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}
pub fn configure_extra(shortcuts: &[&Vec<u32>]) {
    with(|s| {
        s.extra = shortcuts.iter().map(|k| (*k).clone()).collect();
        s.extra_held = [false; 4];
    });
}
pub fn extra_ok() -> [bool; 4] {
    [true; 4]
}

pub fn start(sender: Sender<HookEvent>) {
    let _ = SENDER.set(sender);
    std::thread::Builder::new()
        .name("vorto-keys".into())
        .spawn(listen)
        .ok();
}

/// Human-readable names, e.g. ["Ctrl", "Shift", "Space"].
pub fn names(combo: &[u32]) -> Vec<String> {
    combo.iter().map(|&vk| name(vk)).collect()
}
pub fn name(vk: u32) -> String {
    let mac = cfg!(target_os = "macos");
    let fixed = match vk {
        0x10 => "Shift",
        0x11 => "Ctrl",
        0x12 if mac => "Option",
        0x12 => "Alt",
        0xA0 => "Left Shift",
        0xA1 => "Right Shift",
        0xA2 => "Left Ctrl",
        0xA3 => "Right Ctrl",
        0xA4 if mac => "Left Option",
        0xA5 if mac => "Right Option",
        0xA4 => "Left Alt",
        0xA5 => "Right Alt",
        0x5B | 0x5C if mac => "Cmd",
        0x5B | 0x5C => "Super",
        0x20 => "Space",
        0x0D => "Enter",
        0x09 => "Tab",
        0x1B => "Esc",
        0x08 => "Backspace",
        0x14 => "Caps Lock",
        0x25 => "Left",
        0x26 => "Up",
        0x27 => "Right",
        0x28 => "Down",
        0x2E => "Delete",
        0x24 => "Home",
        0x23 => "End",
        0x21 => "Page Up",
        0x22 => "Page Down",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    match vk {
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        _ => char::from_u32(vk).map_or_else(|| vk.to_string(), |c| c.to_string()),
    }
}

/// macOS: a listening event tap. It needs the Accessibility permission, and asks by failing.
#[cfg(target_os = "macos")]
fn listen() {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        EventField,
    };
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ],
        |_, kind, event| {
            let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
            if let Some(vk) = mac_key(code) {
                match kind {
                    CGEventType::KeyDown => key(vk, true),
                    CGEventType::KeyUp => key(vk, false),
                    // Modifiers only say they changed: the state we track says which way.
                    CGEventType::FlagsChanged => {
                        let down = with(|s| !s.down.contains(&vk));
                        key(vk, down);
                    }
                    _ => {}
                }
            }
            None
        },
    );
    let Ok(tap) = tap else {
        crate::log::write("the shortcut needs the Accessibility permission");
        return;
    };
    let Ok(source) = tap.mach_port.create_runloop_source(0) else {
        return;
    };
    let run_loop = CFRunLoop::get_current();
    unsafe { run_loop.add_source(&source, kCFRunLoopCommonModes) };
    tap.enable();
    INSTALLED.store(true, Ordering::Relaxed);
    CFRunLoop::run_current();
}

/// Virtual keycodes of macOS keyboards (HIToolbox/Events.h) to Windows codes.
#[cfg(target_os = "macos")]
fn mac_key(code: u16) -> Option<u32> {
    const LETTERS: [(u16, u8); 26] = [
        (0x00, b'A'),
        (0x0B, b'B'),
        (0x08, b'C'),
        (0x02, b'D'),
        (0x0E, b'E'),
        (0x03, b'F'),
        (0x05, b'G'),
        (0x04, b'H'),
        (0x22, b'I'),
        (0x26, b'J'),
        (0x28, b'K'),
        (0x25, b'L'),
        (0x2E, b'M'),
        (0x2D, b'N'),
        (0x1F, b'O'),
        (0x23, b'P'),
        (0x0C, b'Q'),
        (0x0F, b'R'),
        (0x01, b'S'),
        (0x11, b'T'),
        (0x20, b'U'),
        (0x09, b'V'),
        (0x0D, b'W'),
        (0x07, b'X'),
        (0x10, b'Y'),
        (0x06, b'Z'),
    ];
    const DIGITS: [(u16, u8); 10] = [
        (0x1D, b'0'),
        (0x12, b'1'),
        (0x13, b'2'),
        (0x14, b'3'),
        (0x15, b'4'),
        (0x17, b'5'),
        (0x16, b'6'),
        (0x1A, b'7'),
        (0x1C, b'8'),
        (0x19, b'9'),
    ];
    const FUNCTION: [(u16, u32); 12] = [
        (0x7A, 1),
        (0x78, 2),
        (0x63, 3),
        (0x76, 4),
        (0x60, 5),
        (0x61, 6),
        (0x62, 7),
        (0x64, 8),
        (0x65, 9),
        (0x6D, 10),
        (0x67, 11),
        (0x6F, 12),
    ];
    if let Some((_, c)) = LETTERS.iter().chain(&DIGITS).find(|(k, _)| *k == code) {
        return Some(*c as u32);
    }
    if let Some((_, n)) = FUNCTION.iter().find(|(k, _)| *k == code) {
        return Some(0x6F + n);
    }
    Some(match code {
        0x38 => 0xA0,
        0x3C => 0xA1,
        0x3B => 0xA2,
        0x3E => 0xA3,
        0x3A => 0xA4,
        0x3D => 0xA5,
        0x37 => 0x5B,
        0x36 => 0x5C,
        0x31 => 0x20,
        0x24 => 0x0D,
        0x30 => 0x09,
        0x35 => 0x1B,
        0x33 => 0x08,
        0x39 => 0x14,
        0x7B => 0x25,
        0x7E => 0x26,
        0x7C => 0x27,
        0x7D => 0x28,
        0x75 => 0x2E,
        0x73 => 0x24,
        0x77 => 0x23,
        0x74 => 0x21,
        0x79 => 0x22,
        _ => return None,
    })
}

/// Linux: X11, and apps under XWayland. A Wayland desktop keeps keys from other windows to itself.
#[cfg(target_os = "linux")]
fn listen() {
    use rdev::{listen as watch, EventType};
    INSTALLED.store(true, Ordering::Relaxed);
    let result = watch(|event| match event.event_type {
        EventType::KeyPress(k) => {
            if let Some(vk) = linux_key(k) {
                key(vk, true);
            }
        }
        EventType::KeyRelease(k) => {
            if let Some(vk) = linux_key(k) {
                key(vk, false);
            }
        }
        _ => {}
    });
    if result.is_err() {
        INSTALLED.store(false, Ordering::Relaxed);
        crate::log::write("couldn't listen for the shortcut");
    }
}

#[cfg(target_os = "linux")]
fn linux_key(key: rdev::Key) -> Option<u32> {
    use rdev::Key::*;
    Some(match key {
        ShiftLeft => 0xA0,
        ShiftRight => 0xA1,
        ControlLeft => 0xA2,
        ControlRight => 0xA3,
        Alt => 0xA4,
        AltGr => 0xA5,
        MetaLeft => 0x5B,
        MetaRight => 0x5C,
        Space => 0x20,
        Return => 0x0D,
        Tab => 0x09,
        Escape => 0x1B,
        Backspace => 0x08,
        CapsLock => 0x14,
        LeftArrow => 0x25,
        UpArrow => 0x26,
        RightArrow => 0x27,
        DownArrow => 0x28,
        Delete => 0x2E,
        Home => 0x24,
        End => 0x23,
        PageUp => 0x21,
        PageDown => 0x22,
        F1 => 0x70,
        F2 => 0x71,
        F3 => 0x72,
        F4 => 0x73,
        F5 => 0x74,
        F6 => 0x75,
        F7 => 0x76,
        F8 => 0x77,
        F9 => 0x78,
        F10 => 0x79,
        F11 => 0x7A,
        F12 => 0x7B,
        KeyA => 0x41,
        KeyB => 0x42,
        KeyC => 0x43,
        KeyD => 0x44,
        KeyE => 0x45,
        KeyF => 0x46,
        KeyG => 0x47,
        KeyH => 0x48,
        KeyI => 0x49,
        KeyJ => 0x4A,
        KeyK => 0x4B,
        KeyL => 0x4C,
        KeyM => 0x4D,
        KeyN => 0x4E,
        KeyO => 0x4F,
        KeyP => 0x50,
        KeyQ => 0x51,
        KeyR => 0x52,
        KeyS => 0x53,
        KeyT => 0x54,
        KeyU => 0x55,
        KeyV => 0x56,
        KeyW => 0x57,
        KeyX => 0x58,
        KeyY => 0x59,
        KeyZ => 0x5A,
        Num0 => 0x30,
        Num1 => 0x31,
        Num2 => 0x32,
        Num3 => 0x33,
        Num4 => 0x34,
        Num5 => 0x35,
        Num6 => 0x36,
        Num7 => 0x37,
        Num8 => 0x38,
        Num9 => 0x39,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_follow_the_system() {
        assert_eq!(name(0x20), "Space");
        assert_eq!(name(0x78), "F9");
        assert_eq!(name(0x41), "A");
    }
    #[test]
    fn modifiers_count_on_either_side() {
        assert!(held(&[0xA3], 0x11));
        assert!(held(&[0xA2], 0x11));
        assert!(!held(&[0xA0], 0x11));
    }
}
