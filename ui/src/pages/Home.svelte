<script>
  import { app, level, route, act, normalizeLevel, clock, downloadText } from "../lib/api.js";
  import { languages } from "../lib/languages.js";
  import AppIcon from "../lib/AppIcon.svelte";
  import Keys from "../lib/Keys.svelte";
  import Icon from "../lib/Icon.svelte";
  import Level from "../lib/Level.svelte";
  import Mark from "../lib/Mark.svelte";
  import ModelCard from "../lib/ModelCard.svelte";
  import { fade, slide } from "svelte/transition";

  let s = $derived($app);
  let model = $derived(s.models.find((m) => m.id === s.settings.model));
  let installed = $derived(model?.installed);
  let downloading = $derived(s.phase === "downloading");
  // Dictations into this window show here; dictations into other apps belong to the pill.
  let listening = $derived(s.dictatingHere && s.recording);
  let writing = $derived(s.dictatingHere && !s.recording);

  // The clock ticks while listening and keeps its last value while the words are written.
  let since = $derived(s.recordingSince);
  let elapsed = $state(0);
  $effect(() => {
    if (!listening) return;
    const start = since;
    const tick = () => (elapsed = Math.max(0, Math.floor((Date.now() - start) / 1000)));
    tick();
    const t = setInterval(tick, 250);
    return () => clearInterval(t);
  });

  let downloadingName = $derived(s.models.find((m) => m.id === s.downloading)?.name ?? "a voice model");
  let recommended = $derived(s.models.find((m) => m.recommended) ?? s.models[0]);
  // History already holds text that went into another app; the card is for what stayed here.
  let showLatest = $derived(
    s.latest && !s.dictatingHere && !(s.settings.history && s.history[0]?.text === s.latest && s.history[0].app),
  );

  let language = $derived(
    s.settings.language === "auto" ? "Any language" : (languages.find(([code]) => code === s.settings.language)?.[1] ?? s.settings.language),
  );

  // ---------------------------------------------------------------- today
  const iso = (d) => d.toLocaleDateString("sv-SE");
  let today = $derived(s.stats?.days?.[iso(new Date())] ?? { dictations: 0, words: 0, seconds: 0 });
  // Typing the same words at your typing speed, minus the time spent speaking them.
  let saved = $derived(Math.max(0, (today.words / (s.settings.typing_wpm || 40)) * 60 - today.seconds));
  // Days in a row with a dictation, counted back from today or, before the first one, yesterday.
  let streak = $derived.by(() => {
    const days = s.stats?.days ?? {};
    const d = new Date();
    if (!days[iso(d)]?.dictations) d.setDate(d.getDate() - 1);
    let count = 0;
    while (days[iso(d)]?.dictations) {
      count++;
      d.setDate(d.getDate() - 1);
    }
    return count;
  });
  function duration(seconds) {
    const rounded = Math.round(seconds);
    if (rounded < 60) return `${rounded} s`;
    const minutes = Math.round(rounded / 60);
    return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
  }
  let tiles = $derived([
    ["Time saved today", duration(saved)],
    ["Words today", today.words.toLocaleString("en-US")],
    ["Dictations today", today.dictations.toLocaleString("en-US")],
    ["Days in a row", streak.toLocaleString("en-US")],
  ]);

  // ---------------------------------------------------------------- recent
  let recent = $derived(s.settings.history ? s.history.slice(0, 5) : []);
  function when(at) {
    const [day, time] = at.split(" ");
    if (day === iso(new Date())) return time;
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    if (day === iso(yesterday)) return "Yesterday";
    return new Date(`${day}T00:00`).toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  let copied = $state(null);
  let copyTimer;
  function copy(text) {
    act("copy", { text });
    copied = text;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = null), 1400);
  }
</script>

{#if !installed && !s.downloading}
  <section class="welcome">
    <div class="hero-mascot"><Mark size={56} /></div>
    <h1>Choose a voice model</h1>
    <div class="welcome-card">
      <ModelCard model={recommended} featured />
      <button class="btn ghost" onclick={() => route.set("models")}>All models <Icon name="arrow" size={15} /></button>
    </div>
  </section>
{:else}
  <section class="hero" class:busy={s.dictatingHere}>
    <h1 class="sr-only">Dictate</h1>
    <div class="line">
      {#if listening || writing}
        <div class="now" in:fade={{ duration: 160 }}>
          {#if listening}<Level level={normalizeLevel($level)} />{/if}
          <span class="title timer" class:frozen={writing}>{clock(elapsed, s.recordingLimit)}</span>
        </div>
      {:else if s.phase === "error"}
        <p class="message">{s.detail}</p>
      {:else if downloading}
        <p class="title">Downloading {downloadingName}</p>
      {:else if !s.hookOk}
        <p class="message">Your shortcut isn't working. Restart Vorto.</p>
      {:else}
        <p class="title instruction">{s.settings.toggle ? "Press" : "Hold"} <Keys keys={s.shortcut} size="lg" /> and speak in any app</p>
      {/if}

      <div class="actions">
        {#if listening}
          <button class="btn secondary" aria-keyshortcuts="Escape" onclick={() => act("cancel")}>Discard <kbd class="kbd">Esc</kbd></button>
          <button class="btn primary" onclick={() => act("stopDictation")}>Finish</button>
        {:else if s.phase === "error"}
          <button class="btn ghost" onclick={() => route.set("models")}>Choose another model</button>
          {#if s.gpuBuild && s.settings.gpu && model?.family === "whisper"}
            <button class="btn secondary" onclick={() => act("useCpu")}>Use processor</button>
          {/if}
          <button class="btn primary" onclick={() => act("retry")}><Icon name="refresh" size={15} /> Try again</button>
        {:else if downloading}
          <button class="btn ghost" onclick={() => act("cancel")}>Cancel</button>
        {:else if writing || s.recording || s.phase === "starting" || s.phase === "transcribing"}
          <!-- Busy: the line or the pill already shows what's happening. -->
        {:else}
          <button class="btn secondary" onclick={() => act("startDictation")}>Dictate here</button>
        {/if}
      </div>
    </div>

    {#if downloading}
      <div
        class="progress"
        role="progressbar"
        aria-label="Download progress"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(s.progress * 100)}
      >
        <span style="width:{Math.max(3, s.progress * 100)}%"></span>
      </div>
      <p class="muted detail">{downloadText(s)}</p>
    {:else if s.phase === "starting" && !s.dictatingHere}
      <p class="muted detail">{s.detail}</p>
    {:else if !s.dictatingHere}
      <div class="setup">
        <button onclick={() => route.set("models")}>{model?.name}</button>
        <button onclick={() => route.set("settings")}>{language}</button>
        <button onclick={() => route.set("settings")}>{s.settings.microphone || "Default microphone"}</button>
      </div>
    {/if}

    {#if s.dictatingHere && s.live}
      <p class="live selectable" class:frozen={writing}>{#each s.live.split(" ").filter(Boolean) as word, i (i)}<span class="word">{word + " "}</span>{/each}</p>
    {/if}
  </section>

  {#if showLatest}
    <section class="latest" transition:slide={{ duration: 240 }}>
      <p class="selectable">{s.latest}</p>
      <button class="icon-btn" aria-label={copied === s.latest ? "Copied" : "Copy"} onclick={() => copy(s.latest)}>
        <Icon name={copied === s.latest ? "check" : "copy"} size={16} />
      </button>
    </section>
  {/if}

  <section class="tiles" aria-label="Today">
    {#each tiles as [label, value]}
      <div class="tile">
        <span class="tile-label">{label}</span>
        <strong>{value}</strong>
      </div>
    {/each}
  </section>

  {#if s.settings.history}
    <section class="recent">
      <div class="recent-head">
        <h2 class="section-title">Recent</h2>
        {#if recent.length}
          <button class="btn ghost sm" onclick={() => route.set("history")}>All history <Icon name="arrow" size={14} /></button>
        {/if}
      </div>
      {#if recent.length}
        <ul>
          {#each recent as entry (entry.at + entry.text)}
            <li>
              <AppIcon name={entry.app} size={26} />
              <p class="selectable" title={entry.app || undefined}>{entry.text}</p>
              <span class="time">{when(entry.at)}</span>
              <button class="icon-btn" aria-label={copied === entry.text ? "Copied" : "Copy"} onclick={() => copy(entry.text)}>
                <Icon name={copied === entry.text ? "check" : "copy"} size={15} />
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted empty">What you dictate shows up here.</p>
      {/if}
    </section>
  {/if}
{/if}

<style>
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding-top: 28px;
  }
  .hero-mascot {
    margin-bottom: 18px;
  }
  .welcome h1 {
    margin-top: 10px;
    font-size: 32px;
  }
  .welcome-card {
    width: min(560px, 100%);
    margin-top: 28px;
    text-align: left;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px 24px;
    min-height: 40px;
  }
  .title {
    font-size: 24px;
    line-height: 1.25;
    font-weight: 400;
    letter-spacing: -0.04em;
  }
  .instruction {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .instruction :global(.kbd) {
    letter-spacing: 0;
  }
  .now {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .timer {
    font-variant-numeric: tabular-nums;
    transition: color 220ms ease;
  }
  .timer.frozen {
    color: var(--faint);
  }
  .message {
    max-width: 520px;
    font-size: 14px;
    color: var(--red);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .setup {
    display: flex;
    flex-wrap: wrap;
    margin: 8px 0 0 -6px;
  }
  .setup button {
    position: relative;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 13px;
  }
  .setup button:hover {
    color: var(--text);
  }
  .setup button + button {
    margin-left: 9px;
  }
  .setup button + button::before {
    content: "";
    position: absolute;
    left: -6px;
    top: 50%;
    width: 3px;
    height: 3px;
    margin-top: -1.5px;
    border-radius: 50%;
    background: var(--faint);
  }
  .detail {
    margin-top: 8px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .word {
    animation: word-in 160ms ease-out both;
  }
  @keyframes word-in {
    from {
      opacity: 0;
      filter: blur(4px);
    }
  }
  .live {
    margin-top: 16px;
    font-size: 15px;
    line-height: 1.55;
    color: var(--text);
  }
  .live.frozen {
    color: var(--muted);
  }
  .progress {
    height: 2px;
    margin-top: 12px;
    border-radius: 2px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .progress span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--ink);
    transition: width 300ms var(--ease);
  }
  .latest {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-top: 24px;
    padding: 12px 8px 12px 16px;
    border-radius: 6px;
    box-shadow: var(--shadow-card);
  }
  .latest p {
    flex: 1;
    font-size: 14px;
    line-height: 1.55;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    margin-top: 32px;
    border-block: 1px solid var(--border);
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 16px 16px 0;
  }
  .tile + .tile {
    padding-left: 16px;
    border-left: 1px solid var(--border);
  }
  .tile-label {
    color: var(--muted);
    font-size: 13px;
  }
  .tile strong {
    font-size: 22px;
    font-weight: 400;
    line-height: 1.2;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .recent {
    margin-top: 32px;
  }
  .recent-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .recent-head .section-title {
    margin: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 0;
    border-top: 1px solid var(--border);
  }
  li p {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
  }
  .time {
    flex-shrink: 0;
    color: var(--muted);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .empty {
    padding: 12px 0;
    font-size: 14px;
    border-top: 1px solid var(--border);
  }
  @media (max-width: 760px) {
    .tiles {
      grid-template-columns: repeat(2, 1fr);
    }
    .tile:nth-child(3) {
      padding-left: 0;
      border-left: 0;
    }
    .tile:nth-child(n + 3) {
      border-top: 1px solid var(--border);
    }
  }
</style>
