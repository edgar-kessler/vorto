// Keyboard events to Windows virtual-key codes and their names, for recording shortcuts.
const codes = {
  ControlLeft: 0xa2, ControlRight: 0xa3, ShiftLeft: 0xa0, ShiftRight: 0xa1,
  AltLeft: 0xa4, AltRight: 0xa5, MetaLeft: 0x5b, MetaRight: 0x5c, ContextMenu: 0x5d,
  Space: 0x20, Enter: 0x0d, NumpadEnter: 0x0d, Tab: 0x09, Backspace: 0x08, Escape: 0x1b,
  CapsLock: 0x14, ArrowLeft: 0x25, ArrowUp: 0x26, ArrowRight: 0x27, ArrowDown: 0x28,
  Insert: 0x2d, Delete: 0x2e, Home: 0x24, End: 0x23, PageUp: 0x21, PageDown: 0x22,
  PrintScreen: 0x2c, Pause: 0x13, ScrollLock: 0x91, NumLock: 0x90,
  NumpadMultiply: 0x6a, NumpadAdd: 0x6b, NumpadSubtract: 0x6d, NumpadDecimal: 0x6e, NumpadDivide: 0x6f,
  Backquote: 0xc0, Minus: 0xbd, Equal: 0xbb, BracketLeft: 0xdb, BracketRight: 0xdd,
  Backslash: 0xdc, Semicolon: 0xba, Quote: 0xde, Comma: 0xbc, Period: 0xbe, Slash: 0xbf, IntlBackslash: 0xe2,
};
const names = {
  0xa2: "Left Ctrl", 0xa3: "Right Ctrl", 0xa0: "Left Shift", 0xa1: "Right Shift", 0xa4: "Left Alt",
  0xa5: "Right Alt", 0x5b: "Win", 0x5c: "Win", 0x5d: "Menu", 0x20: "Space", 0x0d: "Enter", 0x09: "Tab",
  0x08: "Backspace", 0x14: "Caps Lock", 0x25: "Left", 0x26: "Up", 0x27: "Right", 0x28: "Down",
  0x2d: "Insert", 0x2e: "Delete", 0x24: "Home", 0x23: "End", 0x21: "Page Up", 0x22: "Page Down",
  0x2c: "Print", 0x13: "Pause", 0x91: "Scroll Lock", 0x04: "Middle mouse", 0x05: "Mouse 4", 0x06: "Mouse 5",
};
export function vk(e) {
  if (codes[e.code]) return codes[e.code];
  let m = e.code.match(/^Key([A-Z])$/);
  if (m) return m[1].charCodeAt(0);
  m = e.code.match(/^Digit(\d)$/);
  if (m) return 0x30 + Number(m[1]);
  m = e.code.match(/^Numpad(\d)$/);
  if (m) return 0x60 + Number(m[1]);
  m = e.code.match(/^F(\d+)$/);
  if (m) return 0x6f + Number(m[1]);
  return 0;
}
export const label = (k) => names[k] ?? (k >= 0x70 && k <= 0x87 ? `F${k - 0x6f}` : k >= 0x60 && k <= 0x69 ? `Num ${k - 0x60}` : String.fromCharCode(k));
