<script>
  // Records a key combination such as Ctrl+Alt+V inside the window, for the extra shortcuts.
  // Windows registers them for Vorto, so they need one key besides Ctrl, Alt, Shift or Win.
  import Keys from "./Keys.svelte";
  import Icon from "./Icon.svelte";
  import { vk } from "./keys.js";
  import { fade } from "svelte/transition";

  let { keys = [], names = [], ok = true, label, onchange } = $props();
  let recording = $state(false);
  let held = $state([]);

  const modifier = { 0xa0: 0x10, 0xa1: 0x10, 0xa2: 0x11, 0xa3: 0x11, 0xa4: 0x12, 0xa5: 0x12, 0x5b: 0x5b, 0x5c: 0x5b };
  const modifierName = { 0x10: "Shift", 0x11: "Ctrl", 0x12: "Alt", 0x5b: "Win" };

  $effect(() => {
    if (!recording) return;
    held = [];
    const keydown = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.repeat) return;
      const key = vk(e);
      if (!key) return;
      if (key === 0x1b && !held.length) return (recording = false);
      if (modifier[key]) {
        if (!held.includes(modifier[key])) held = [...held, modifier[key]];
        return;
      }
      // Ctrl, Alt, Shift, Win first, then the key, as Vorto names combinations.
      const mods = [[e.ctrlKey, 0x11], [e.altKey, 0x12], [e.shiftKey, 0x10], [e.metaKey, 0x5b]]
        .filter(([on, m]) => on || held.includes(m))
        .map(([, m]) => m);
      recording = false;
      onchange?.([...mods, key]);
    };
    const keyup = (e) => {
      const key = modifier[vk(e)];
      if (key) held = held.filter((k) => k !== key);
    };
    const blur = () => (recording = false);
    window.addEventListener("keydown", keydown, true);
    window.addEventListener("keyup", keyup, true);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", keydown, true);
      window.removeEventListener("keyup", keyup, true);
      window.removeEventListener("blur", blur);
    };
  });
</script>

{#if recording}
  <div class="capture" in:fade={{ duration: 140 }}>
    {#if held.length}
      <Keys keys={held.map((k) => modifierName[k])} />
    {:else}
      <span class="rec-dot"></span> Press a combination…
    {/if}
  </div>
  <button class="btn ghost sm" onclick={() => (recording = false)}>Cancel</button>
{:else}
  {#if keys.length}
    <Keys keys={names} />
    <button class="icon-btn" aria-label="Remove the shortcut for {label}" onclick={() => onchange?.([])}><Icon name="x" size={14} /></button>
  {/if}
  <button class="btn secondary sm" onclick={() => (recording = true)}>{keys.length ? "Change" : "Set"}</button>
{/if}

<style>
  .capture {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 200px;
    height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--brand-line);
    box-shadow: 0 0 0 4px var(--brand-soft);
    font-size: 13px;
    font-weight: 500;
  }
  .rec-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--brand);
  }
</style>
