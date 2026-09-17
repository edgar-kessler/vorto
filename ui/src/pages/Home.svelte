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
    <div class="hero-mascot"><Mark size={116} /></div>
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
      {#if listening}
        <span class="ring" style="--l:{normalizeLevel($level)}"></span>
        <span class="ring two" style="--l:{normalizeLevel($level)}"></span>
      {/if}
      <div class="mascot-layer"><Mark size={124} {mood} level={normalizeLevel($level)} /></div>
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
    padding: 18px 0 8px;
  }
  .stage {
    position: relative;
    display: grid;
    place-items: center;
    width: 200px;
    height: 170px;
  }
  .stage::before {
    content: "";
    position: absolute;
    width: 150px;
    height: 150px;
    border-radius: 999px;
    background: radial-gradient(circle, var(--glow) 0%, transparent 70%);
    transition: transform 400ms var(--ease);
  }
  .stage.live::before {
    transform: scale(1.25);
  }
  .mascot-layer {
    position: relative;
    z-index: 1;
  }
  .ring {
    position: absolute;
    width: 132px;
    height: 132px;
    border-radius: 999px;
    border: 2px solid rgba(255, 98, 80, 0.35);
    transform: scale(calc(1 + var(--l) * 0.35));
    transition: transform 90ms linear;
  }
  .ring.two {
    border-color: rgba(255, 98, 80, 0.16);
    transform: scale(calc(1.12 + var(--l) * 0.6));
  }
  .headline {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .title {
    margin-top: 10px;
    font-size: 26px;
    line-height: 1.2;
    font-weight: 620;
    letter-spacing: -0.022em;
  }
  .instruction {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: 8px;
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
    font-size: 15.5px;
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
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
  .word {
    animation: word-in 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
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
    font-size: 17px;
    line-height: 1.5;
    color: var(--text);
    transition: color 220ms ease;
  }
  .live.frozen {
    color: var(--muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    margin-top: 22px;
  }
  .rec {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: #ff4d5e;
  }
  .progress {
    width: 300px;
    height: 6px;
    border-radius: 999px;
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
    margin-top: 26px;
    padding: 16px 12px 16px 18px;
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-card);
  }
  .latest p {
    flex: 1;
    font-size: 15px;
    line-height: 1.55;
  }
  .today {
    margin-top: 30px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--group);
    text-align: left;
  }
  .stat-label {
    font-size: 12.5px;
    color: var(--muted);
  }
  .stat strong {
    font-size: 20px;
    font-weight: 620;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
</style>
