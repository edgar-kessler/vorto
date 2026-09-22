<script>
  import { app, level, act, normalizeLevel, saveSettings } from "../lib/api.js";
  import { slide } from "svelte/transition";
  import Toggle from "../lib/Toggle.svelte";
  import Select from "../lib/Select.svelte";
  import Segmented from "../lib/Segmented.svelte";
  import ShortcutRecorder from "../lib/ShortcutRecorder.svelte";
  import ComboRecorder from "../lib/ComboRecorder.svelte";
  import Icon from "../lib/Icon.svelte";

  let s = $derived($app);
  let settings = $derived(s.settings);
  let model = $derived(s.models.find((m) => m.id === settings.model));
  let parakeet = $derived(model?.family === "parakeet");
  let update = $derived(s.update);
  let updateText = $derived(
    {
      checking: "Checking for updates…",
      latest: "You have the latest version.",
      available: `Version ${update.version} is available.`,
      downloading: `Downloading version ${update.version}${update.progress >= 0 ? ` · ${Math.round(update.progress * 100)} %` : "…"}`,
      ready: `Version ${update.version} is ready to install.`,
      installing: "Installing the update…",
      error: "Couldn't update. Check your internet connection and try again.",
    }[update.status] ?? "Open source. No account, no uploads, no telemetry.",
  );

  function set(key, value) {
    saveSettings({ [key]: value });
  }

  // In the order of Shortcuts::all in the app.
  const extras = [
    ["paste_last", "Paste last dictation", "Inserts your last dictation again, into the app you are in."],
    ["undo_last", "Undo last insertion", "Takes back what Vorto just wrote, while you are still in that app."],
    ["toggle_ai", "Turn AI editing on or off", "The pill says which it is now."],
    ["copy_last", "Copy last dictation", "Puts your last dictation on the clipboard."],
  ];
  function setExtra(key, keys) {
    saveSettings({ shortcuts: { ...settings.shortcuts, [key]: keys } });
  }

  const languages = [
    ["auto", "Detect automatically"],
    ["en", "English"],
    ["de", "German"],
    ["fr", "French"],
    ["es", "Spanish"],
    ["it", "Italian"],
    ["nl", "Dutch"],
    ["pt", "Portuguese"],
  ].map(([value, label]) => ({ value, label }));
  const idle = [
    [0, "Always"],
    [5, "5 minutes"],
    [15, "15 minutes"],
    [30, "30 minutes"],
  ].map(([value, label]) => ({ value, label }));
  let microphones = $derived([{ value: "", label: "Windows default" }, ...s.microphones.map((m) => ({ value: m, label: m }))]);
</script>

<header class="page-head">
  <h1>Settings</h1>
</header>

<h3 class="section-title">General</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Dictation shortcut</strong>
      {#if !s.hookOk}<span class="fail">Your shortcut isn't working. Restart Vorto.</span>{/if}
    </div>
    <div class="control">
      <ShortcutRecorder />
    </div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Recording style</strong>
      <span>{settings.toggle ? "Press once to start, press again to finish." : "Hold the shortcut while you speak."}</span>
    </div>
    <div class="control">
      <Segmented
        label="Recording style"
        value={settings.toggle}
        options={[
          { value: false, label: "Hold" },
          { value: true, label: "Toggle" },
        ]}
        onchange={(v) => set("toggle", v)}
      />
    </div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Start with Windows</strong>
      <span>Your shortcut works right after sign-in.</span>
    </div>
    <div class="control"><Toggle label="Start with Windows" checked={s.autostart} onchange={(v) => act("setAutostart", { on: v })} /></div>
  </div>
</div>

<h3 class="section-title">More shortcuts</h3>
<div class="group">
  {#each extras as [key, title, text], i}
    <div class="row">
      <div class="label">
        <strong>{title}</strong>
        {#if !s.extraOk[i]}
          <span class="fail">Another app already uses this combination. Choose another one.</span>
        {:else}
          <span>{text}</span>
        {/if}
      </div>
      <div class="control">
        <ComboRecorder label={title} keys={settings.shortcuts[key]} names={s.extraShortcuts[i]} ok={s.extraOk[i]} onchange={(keys) => setExtra(key, keys)} />
      </div>
    </div>
  {/each}
</div>

<h3 class="section-title">Recognition</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Language</strong>
      <span>{parakeet ? "Parakeet detects the language automatically." : "Picking your language improves accuracy."}</span>
    </div>
    {#if !parakeet}
      <div class="control">
        <Select label="Language" value={settings.language} options={languages} onchange={(v) => set("language", v)} />
      </div>
    {/if}
  </div>
  <div class="row">
    <div class="label">
      <strong>Microphone</strong>
      {#if s.micTest}
        <span class="meter" aria-hidden="true"><i style="width:{Math.round(normalizeLevel($level) * 100)}%"></i></span>
        <span>Speak now.</span>
      {:else if !s.microphones.length}
        <span class="fail">No microphone found.</span>
      {/if}
    </div>
    <div class="control">
      <button class="btn sm" class:secondary={!s.micTest} class:primary={s.micTest} disabled={s.recording} onclick={() => act("testMicrophone", { on: !s.micTest })}>{s.micTest ? "Stop" : "Test"}</button>
      <Select label="Microphone" value={settings.microphone} options={microphones} width={230} onopen={() => act("refreshMicrophones")} onchange={(v) => set("microphone", v)} />
    </div>
  </div>
  {#if s.gpuBuild}
    <div class="row">
      <div class="label">
        <strong>Use graphics card</strong>
        <span>Makes Whisper models much faster. Parakeet runs on the processor.</span>
      </div>
      <div class="control"><Toggle label="Use graphics card" checked={settings.gpu} onchange={(v) => set("gpu", v)} /></div>
    </div>
  {/if}
  <div class="row">
    <div class="label">
      <strong>Live preview</strong>
      <span>{parakeet || (s.gpuBuild && settings.gpu) ? "Shows your words as you speak." : "Needs Parakeet or a graphics card."}</span>
    </div>
    <div class="control"><Toggle label="Live preview" checked={settings.live_preview} onchange={(v) => set("live_preview", v)} /></div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Sounds</strong>
      <span>A key click when you start and stop, and a chime when the text is in place.</span>
    </div>
    <div class="control"><Toggle label="Sounds" checked={settings.sounds} onchange={(v) => set("sounds", v)} /></div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Keep model ready</strong>
      <span>A timer frees memory; the next dictation then takes a moment to start. The microphone stays off until you record.</span>
    </div>
    <div class="control">
      <Select label="Keep model ready" value={settings.idle_minutes} options={idle} width={180} onchange={(v) => set("idle_minutes", v)} />
    </div>
  </div>
</div>

<h3 class="section-title">Output</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Insert automatically</strong>
      <span>{settings.paste ? "Puts the text into the app you were using." : "Text is kept in Vorto, ready to copy."}</span>
    </div>
    <div class="control"><Toggle label="Insert automatically" checked={settings.paste} onchange={(v) => set("paste", v)} /></div>
  </div>
  <!-- One block per row: a nested block would drop its row without the slide. -->
  {#if settings.paste}
    <div class="row" transition:slide={{ duration: 220 }}>
      <div class="label">
        <strong>Insertion method</strong>
        <span>{settings.method === "paste" ? "Pasting is instant and works in most apps." : "Typing works everywhere, a little slower."}</span>
      </div>
      <div class="control">
        <Segmented
          label="Insertion method"
          value={settings.method}
          options={[
            { value: "paste", label: "Paste" },
            { value: "type", label: "Type" },
          ]}
          onchange={(v) => set("method", v)}
        />
      </div>
    </div>
  {/if}
  {#if settings.paste && settings.method === "paste"}
    <div class="row" transition:slide={{ duration: 220 }}>
      <div class="label">
        <strong>Restore clipboard</strong>
        <span>Puts back what you had copied before.</span>
      </div>
      <div class="control"><Toggle label="Restore clipboard" checked={settings.restore_clipboard} onchange={(v) => set("restore_clipboard", v)} /></div>
    </div>
  {/if}
  {#if settings.paste}
    <div class="row" transition:slide={{ duration: 220 }}>
      <div class="label">
        <strong>Add a space after</strong>
        <span>Keep dictating without touching the keyboard.</span>
      </div>
      <div class="control"><Toggle label="Add a space after" checked={settings.append_space} onchange={(v) => set("append_space", v)} /></div>
    </div>
  {/if}
  {#if settings.paste}
    <div class="row" transition:slide={{ duration: 220 }}>
      <div class="label">
        <strong>Show where text lands</strong>
        <span>A short glow over the words Vorto just inserted, in apps that say where they are.</span>
      </div>
      <div class="control"><Toggle label="Show where text lands" checked={settings.highlight} onchange={(v) => set("highlight", v)} /></div>
    </div>
  {/if}
  <div class="row">
    <div class="label">
      <strong>Recording indicator</strong>
      <span>A small pill with the timer, your words and the result while you dictate into other apps.</span>
    </div>
    <div class="control"><Toggle label="Recording indicator" checked={settings.overlay} onchange={(v) => set("overlay", v)} /></div>
  </div>
  {#if settings.overlay}
    <div class="row" transition:slide={{ duration: 220 }}>
      <div class="label">
        <strong>Indicator position</strong>
      </div>
      <div class="control">
        <Segmented
          label="Indicator position"
          value={settings.hud_position}
          options={[
            { value: "top", label: "Top" },
            { value: "bottom", label: "Bottom" },
          ]}
          onchange={(v) => set("hud_position", v)}
        />
      </div>
    </div>
  {/if}
  <div class="row">
    <div class="label">
      <strong>Save history</strong>
      <span>Keeps your last 100 dictations.</span>
    </div>
    <div class="control"><Toggle label="Save history" checked={settings.history} onchange={(v) => set("history", v)} /></div>
  </div>
</div>

<h3 class="section-title">About</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Vorto {s.version}</strong>
      <span aria-live="polite">{updateText}</span>
    </div>
    <div class="control">
      {#if update.status === "ready" || update.status === "available"}
        <button class="btn primary sm" onclick={() => act("installUpdate")}>
          <Icon name="download" size={15} /> {update.status === "ready" ? "Restart to update" : "Update"}
        </button>
      {:else if !["checking", "downloading", "installing"].includes(update.status)}
        <button class="btn secondary sm" onclick={() => act("checkForUpdates")}><Icon name="refresh" size={15} /> Check for updates</button>
      {/if}
    </div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Update automatically</strong>
      <span>Downloads new versions in the background. You choose when to restart.</span>
    </div>
    <div class="control">
      <Toggle label="Update automatically" checked={settings.auto_update} onchange={(v) => set("auto_update", v)} />
    </div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Data folder</strong>
      <span class="path selectable">{s.dataDir}</span>
    </div>
    <div class="control">
      <button class="btn secondary sm" onclick={() => act("openDataFolder")}><Icon name="folder" size={15} /> Open</button>
    </div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Runs in the background</strong>
      <span>Closing the window keeps your shortcut working. Quit from the tray icon.</span>
    </div>
  </div>
</div>

<style>
  /* Beats the global `.row .label span` muted colour. */
  .label .fail {
    color: var(--red);
  }
  .meter {
    width: 220px;
    height: 6px;
    margin: 7px 0 5px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--green);
    transition: width 80ms linear;
  }
  .path {
    font-size: 12px !important;
    word-break: break-all;
  }
</style>
