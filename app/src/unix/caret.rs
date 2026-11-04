//! Where text landed, and what the focused field says, need the accessibility APIs of each desktop.
//! Until then there's no glow over inserted text here, and corrections are learned in History.

/// A rectangle on screen in physical pixels: left, top, width, height.
pub type Rect = (i32, i32, i32, i32);

pub fn inserted(_hwnd: isize, _chars: usize) -> Vec<Rect> {
    Vec::new()
}

pub fn field_text() -> Option<String> {
    None
}
