<script>
  import { app, act } from "./api.js";
  import Keys from "./Keys.svelte";
  import { vk, label } from "./keys.js";
  import { get } from "svelte/store";
  import { fade } from "svelte/transition";

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
