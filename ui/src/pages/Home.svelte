<script>
  import { app, level, route, act, normalizeLevel, clock, downloadText } from "../lib/api.js";
  import Mark from "../lib/Mark.svelte";
  import Keys from "../lib/Keys.svelte";
  import Icon from "../lib/Icon.svelte";
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

  // Celebrate each new dictation for a moment.
  const dictations = () => s.dictations;
  let celebrate = $state(false);
  let seen = dictations();
  $effect(() => {
    const count = dictations();
    if (count > seen) {
      celebrate = true;
      setTimeout(() => (celebrate = false), 1800);
    }
    seen = count;
  });
  let mood = $derived(
    listening
      ? "listening"
      : writing || s.phase === "transcribing" || s.phase === "starting" || downloading
        ? "thinking"
        : s.phase === "error" || !s.hookOk
          ? "worried"
          : celebrate
            ? "happy"
            : s.phase === "sleeping"
              ? "sleep"
              : "idle",
  );
  // Fades the headline when its meaning changes, not on every clock tick.
  let headlineKey = $derived(
    s.dictatingHere ? "dictation" : s.phase === "error" ? "error" : downloading ? "download" : !s.hookOk ? "hook" : "idle",
  );

  let downloadingName = $derived(s.models.find((m) => m.id === s.downloading)?.name ?? "a voice model");
  let recommended = $derived(s.models.find((m) => m.recommended) ?? s.models[0]);
  let todayCount = $derived(s.history.filter((e) => e.at.startsWith(new Date().toLocaleDateString("sv-SE"))).length);
  // History already holds text that went into another app; the card is for what stayed here.
  let showLatest = $derived(
    s.latest && !s.dictatingHere && !(s.settings.history && s.history[0]?.text === s.latest && s.history[0].app),
  );

  let copied = $state(false);
  let copyTimer;
  function copy(text) {
    act("copy", { text });
    copied = true;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = false), 1400);
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
  <section class="hero">
    <h1 class="sr-only">Dictate</h1>
    <div class="stage" class:live={listening}>
      <div class="mascot-layer"><Mark size={64} {mood} level={normalizeLevel($level)} /></div>
    </div>

    {#key headlineKey}
      <div class="headline" in:fade={{ duration: 180 }}>
        {#if listening || writing}
          <p class="title timer" class:frozen={writing}>{clock(elapsed, s.recordingLimit)}</p>
        {:else if s.phase === "error"}
          <p class="message">{s.detail}</p>
        {:else if downloading}
          <p class="title">Downloading {downloadingName}</p>
        {:else if !s.hookOk}
          <p class="message">Your shortcut isn't working. Restart Vorto.</p>
        {:else}
          <p class="title instruction">{s.settings.toggle ? "Press" : "Hold"} <Keys keys={s.shortcut} size="lg" /> and speak in any app</p>
        {/if}
      </div>
    {/key}

    <div class="sub">
      {#if downloading}
        <span class="muted">{downloadText(s)}</span>
      {:else if s.phase === "starting" && !s.dictatingHere}
        <span class="muted">{s.detail}</span>
      {/if}
    </div>

    {#if s.dictatingHere && s.live}
      <p class="live selectable" class:frozen={writing}>{#each s.live.split(" ").filter(Boolean) as word, i (i)}<span class="word">{word + " "}</span>{/each}</p>
    {/if}

    <div class="actions">
      {#if listening}
        <button class="btn primary lg" onclick={() => act("stopDictation")}><span class="rec"></span> Finish</button>
        <button class="btn secondary lg" aria-keyshortcuts="Escape" onclick={() => act("cancel")}>Discard <kbd class="kbd">Esc</kbd></button>
      {:else if s.phase === "error"}
        <button class="btn primary lg" onclick={() => act("retry")}><Icon name="refresh" size={16} /> Try again</button>
        {#if s.gpuBuild && s.settings.gpu && model?.family === "whisper"}
          <button class="btn secondary lg" onclick={() => act("useCpu")}>Use processor</button>
        {/if}
        <button class="btn ghost lg" onclick={() => route.set("models")}>Choose another model</button>
      {:else if downloading}
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
        <button class="btn ghost" onclick={() => act("cancel")}>Cancel</button>
      {:else if writing || s.recording || s.phase === "starting" || s.phase === "transcribing"}
        <!-- Busy: the headline or the pill already shows what's happening. -->
      {:else}
        <button class="btn secondary lg" onclick={() => act("startDictation")}>Dictate here</button>
      {/if}
    </div>
  </section>

  {#if showLatest}
    <section class="latest" transition:slide={{ duration: 240 }}>
      <p class="selectable">{s.latest}</p>
      <button class="icon-btn" aria-label={copied ? "Copied" : "Copy"} onclick={() => copy(s.latest)}>
        <Icon name={copied ? "check" : "copy"} size={16} />
      </button>
    </section>
  {/if}

  {#if s.settings.history && todayCount}
    <section class="today">
      <h2 class="section-title">Today</h2>
      <div class="stats">
        <div class="stat">
          <span class="stat-label">Words</span>
          <strong>{s.wordsToday.toLocaleString("en-US")}</strong>
        </div>
        <div class="stat">
          <span class="stat-label">Dictations</span>
          <strong>{todayCount}</strong>
        </div>
      </div>
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
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 56px 0 16px;
  }
  .stage {
    display: grid;
    place-items: center;
    height: 88px;
  }
  .headline {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .title {
    margin-top: 12px;
    font-family: var(--mono);
    font-size: 24px;
    line-height: 1.25;
    font-weight: 400;
    letter-spacing: -0.05em;
  }
  .instruction {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }
  .instruction :global(.kbd) {
    letter-spacing: 0;
  }
  .timer {
    font-variant-numeric: tabular-nums;
    transition: color 220ms ease;
  }
  .timer.frozen {
    color: var(--faint);
  }
  .message {
    margin-top: 10px;
    max-width: 520px;
    font-size: 14px;
    color: var(--red);
  }
  .sub {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 7px;
    min-height: 30px;
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
    max-width: 560px;
    margin-top: 14px;
    font-size: 15px;
    line-height: 1.55;
    color: var(--text);
  }
  .live.frozen {
    color: var(--muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    margin-top: 20px;
  }
  .rec {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: #ff4d5e;
  }
  .progress {
    width: 300px;
    height: 2px;
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
  .today {
    margin-top: 48px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
  .stats {
    display: flex;
    gap: 56px;
    margin-top: 12px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
  }
  .stat-label {
    color: var(--muted);
  }
  .stat strong {
    font-family: var(--mono);
    font-size: 24px;
    font-weight: 400;
    line-height: 1.2;
    letter-spacing: -0.04em;
  }
</style>
