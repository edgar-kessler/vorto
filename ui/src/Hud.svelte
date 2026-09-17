<script>
  import { onMount } from "svelte";
  import { app, level, connect, normalizeLevel, clock } from "./lib/api.js";
  import Icon from "./lib/Icon.svelte";
  import Mark from "./lib/Mark.svelte";

  let voice = $state(0);
  let now = $state(Date.now());
  let s = $derived($app);
  let hud = $derived(s?.hud ?? { mode: "hidden", text: "", live: "" });
  // Keep the last visible content while the pill animates out.
  let shown = $state({ mode: "hidden", text: "", live: "" });
  let visible = $derived(hud.mode !== "hidden");
  let bottom = $derived(s?.settings?.hud_position === "bottom");
  $effect(() => {
    if (hud.mode !== "hidden") {
      shown = hud;
      return;
    }
    // Once the pill has animated out, drop its content: a hidden "Writing" would keep animating.
    const t = setTimeout(() => (shown = { mode: "hidden", text: "", live: "" }), 400);
    return () => clearTimeout(t);
  });

  onMount(() => {
    connect();
    const unsub = level.subscribe((v) => {
      // Rise quickly, fall gently, so the mark moves like a voice and not like noise.
      const target = normalizeLevel(v);
      voice = target > voice ? target : voice * 0.82 + target * 0.18;
    });
    return unsub;
  });
  // The clock only runs while recording, so the hidden pill does no work.
  $effect(() => {
    if (!s?.recording) return;
    now = Date.now();
    const t = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(t);
  });
  let elapsed = $derived(s?.recording ? Math.max(0, Math.floor((now - s.recordingSince) / 1000)) : 0);
  let target = $derived(s?.target);
  // Caption style: the newest words, up to four lines, cut at a word boundary.
  let caption = $derived.by(() => {
    const text = shown.live ?? "";
    if (text.length <= 240) return text;
    const tail = text.slice(-240);
    return "…" + tail.slice(tail.indexOf(" ") + 1);
  });
  let live = $derived(Boolean(caption) && (shown.mode === "listening" || shown.mode === "writing"));
  let words = $derived(caption.split(" ").filter(Boolean));
</script>

<div class="wrap" class:bottom>
  <div class="pill {shown.mode}" class:visible class:has-live={live} role="status">
    <div class="head">
      {#if shown.mode === "listening"}
        <span class="mark"><Mark size={24} mood="listening" level={voice} onDark /></span>
        <span class="status">Listening</span>
      {:else if shown.mode === "writing"}
        <span class="mark"><Mark size={24} mood="thinking" onDark /></span>
        <span class="status shimmer">Writing</span>
      {:else if shown.mode === "done"}
        <span class="badge ok"><Icon name="check" size={12} stroke={3} /></span>
        <span class="status">{shown.text}</span>
      {:else if shown.mode === "notice"}
        <span class="badge info"><Icon name="info" size={12} stroke={3} /></span>
        <span class="status">{shown.text}</span>
      {:else if shown.mode === "error"}
        <span class="badge bad"><Icon name="x" size={12} stroke={3} /></span>
        <span class="status">{shown.text}</span>
      {/if}

      {#if (shown.mode === "listening" || shown.mode === "writing") && shown.text}
        <span class="divider"></span>
        <span class="app">
          {#if target?.icon}<img src={target.icon} alt="" />{/if}
          <span>{shown.text}</span>
        </span>
      {/if}

      {#if shown.mode === "listening"}
        <span class="time">{clock(elapsed, s.recordingLimit)}</span>
      {/if}
    </div>
    {#if live}
      <div class="live">
        <p>{#each words as word, i (i)}<span class="word">{word + " "}</span>{/each}</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    height: 100vh;
    padding: 8px 0;
  }
  .wrap.bottom {
    justify-content: flex-end;
  }
  .pill {
    display: flex;
    flex-direction: column;
    max-width: 560px;
    padding: 0 18px 0 12px;
    border-radius: 24px;
    background: rgba(17, 17, 19, 0.96);
    color: #fff;
    box-shadow:
      0 10px 26px rgba(0, 0, 0, 0.28),
      inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    font-size: 13.5px;
    opacity: 0;
    transform: translateY(-10px) scale(0.55);
    transform-origin: top center;
    interpolate-size: allow-keywords;
    transition:
      opacity 200ms ease,
      transform 460ms cubic-bezier(0.34, 1.4, 0.64, 1),
      width 420ms cubic-bezier(0.22, 1, 0.36, 1),
      border-radius 300ms ease;
  }
  .bottom .pill {
    transform: translateY(10px) scale(0.55);
    transform-origin: bottom center;
  }
  .pill.visible {
    opacity: 1;
    transform: none;
  }
  .pill:not(.has-live) {
    width: auto;
  }
  .pill.has-live {
    width: 540px;
    padding-left: 16px;
    border-radius: 22px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    white-space: nowrap;
  }
  .live {
    interpolate-size: allow-keywords;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    max-height: calc(4 * 1.5em);
    margin: -2px 0 14px;
    overflow: hidden;
    font-size: 15px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.92);
    white-space: normal;
    animation: unfold 420ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes unfold {
    from {
      max-height: 0;
      opacity: 0;
      margin-bottom: 0;
    }
  }
  .live p {
    margin: 0;
  }
  /* New words glide in; corrected words change in place without flicker. */
  .word {
    display: inline;
    animation: word-in 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @keyframes word-in {
    from {
      opacity: 0;
      filter: blur(4px);
    }
    to {
      opacity: 1;
      filter: blur(0);
    }
  }
  .mark {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
  }
  .status {
    font-weight: 560;
    letter-spacing: -0.005em;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .divider {
    width: 1px;
    height: 16px;
    background: rgba(255, 255, 255, 0.16);
  }
  .app {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 180px;
    color: rgba(255, 255, 255, 0.78);
  }
  .app span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .app img {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
  .time {
    margin-left: auto;
    padding-left: 12px;
    min-width: 30px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: rgba(255, 255, 255, 0.6);
    font-size: 12.5px;
  }
  .shimmer {
    background: linear-gradient(90deg, #8f8f98 0%, #fff 40%, #8f8f98 80%);
    background-size: 200% 100%;
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
  .badge {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .badge.ok {
    background: #22c55e;
  }
  .badge.bad {
    background: #ff4757;
  }
  .badge.info {
    background: rgba(255, 255, 255, 0.16);
  }
</style>
