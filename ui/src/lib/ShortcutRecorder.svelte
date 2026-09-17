<script>
  import { app, act } from "./api.js";
  import Keys from "./Keys.svelte";
  import { get } from "svelte/store";
  import { fade } from "svelte/transition";

  // KeyboardEvent.code to Windows virtual-key codes.
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
  function vk(e) {
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
  const label = (k) => names[k] ?? (k >= 0x70 && k <= 0x87 ? `F${k - 0x6f}` : k >= 0x60 && k <= 0x69 ? `Num ${k - 0x60}` : String.fromCharCode(k));

  let s = $derived($app);
  // A primitive, so the effect below re-runs only when capture starts or stops.
  let capturing = $derived(!!$app?.capturing);
  let held = $state(new Set());
  let combo = $state([]);

  // Capture swallows keys system-wide: end it when this recorder goes away.
  $effect(() => () => {
    if (get(app)?.capturing) act("stopRecordingShortcut");
  });

  $effect(() => {
    if (!capturing) {
      held = new Set();
      combo = [];
      return;
    }
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      let keys = [...new Set(combo)];
      // AltGr arrives as Left Ctrl + Right Alt.
      if (keys.includes(0xa5)) keys = keys.filter((k) => k !== 0xa2);
      act("setShortcut", { keys: keys.slice(0, 4) });
    };
    const down = (key) => {
      if (!key) return;
      held = new Set([...held, key]);
      if (!combo.includes(key)) combo = [...combo, key];
    };
    const up = (key) => {
      clearTimeout(lonely);
      const next = new Set(held);
      next.delete(key);
      held = next;
      if (next.size === 0 && combo.length) finish();
    };
    // Windows does not always deliver the release of Menu or Print: finish shortly
    // after such a key goes down unless more keys join.
    let lonely;
    const keydown = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.repeat) return;
      if (e.code === "Escape" && combo.length === 0) return act("stopRecordingShortcut");
      clearTimeout(lonely);
      const key = vk(e);
      down(key);
      if (key === 0x5d || key === 0x2c) {
        lonely = setTimeout(() => {
          held = new Set();
          finish();
        }, 450);
      }
    };
    const contextmenu = (e) => e.preventDefault();
    // Leaving the window (click elsewhere, minimize, close to tray) ends capture.
    const blur = () => act("stopRecordingShortcut");
    const keyup = (e) => {
      e.preventDefault();
      up(vk(e));
    };
    const mouse = { 1: 0x04, 3: 0x05, 4: 0x06 };
    const pointerdown = (e) => {
      if (mouse[e.button]) {
        e.preventDefault();
        down(mouse[e.button]);
      }
    };
    const pointerup = (e) => {
      if (mouse[e.button]) {
        e.preventDefault();
        up(mouse[e.button]);
      }
    };
    window.addEventListener("contextmenu", contextmenu, true);
    window.addEventListener("keydown", keydown, true);
    window.addEventListener("keyup", keyup, true);
    window.addEventListener("mousedown", pointerdown, true);
    window.addEventListener("mouseup", pointerup, true);
    window.addEventListener("blur", blur);
    return () => {
      clearTimeout(lonely);
      window.removeEventListener("blur", blur);
      window.removeEventListener("contextmenu", contextmenu, true);
      window.removeEventListener("keydown", keydown, true);
      window.removeEventListener("keyup", keyup, true);
      window.removeEventListener("mousedown", pointerdown, true);
      window.removeEventListener("mouseup", pointerup, true);
    };
  });
</script>

{#if s.capturing}
  <div class="capture" in:fade={{ duration: 140 }}>
    {#if combo.length}
      <Keys keys={combo.map(label)} />
    {:else}
      <span class="rec-dot"></span> Press keys or a mouse button…
    {/if}
  </div>
  <button class="btn ghost sm" onclick={() => act("stopRecordingShortcut")}>Cancel</button>
{:else}
  <Keys keys={s.shortcut} />
  <button class="btn secondary sm" disabled={s.recording} onclick={() => act("recordShortcut")}>Change</button>
{/if}

<style>
  .capture {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 240px;
    height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--brand-line);
    box-shadow: 0 0 0 4px var(--brand-soft);
    font-size: 13px;
    font-weight: 500;
    animation: glow 1.6s ease-in-out infinite;
  }
  @keyframes glow {
    50% {
      box-shadow: 0 0 0 7px var(--brand-soft);
    }
  }
  .rec-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--brand);
  }
</style>
