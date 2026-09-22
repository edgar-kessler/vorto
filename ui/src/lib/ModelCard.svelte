<script>
  import { app, act, downloadText } from "./api.js";
  import Icon from "./Icon.svelte";
  import BrandLogo from "./BrandLogo.svelte";
  import Meter from "./Meter.svelte";

  let { model, featured = false } = $props();
  let s = $derived($app);
  let inUse = $derived(model.installed && s.settings.model === model.id);
  let downloading = $derived(s.downloading === model.id);
  let busy = $derived(s.recording || s.downloading || s.phase === "transcribing");
  let memory = $derived(model.memoryMb >= 1000 ? `${(model.memoryMb / 1000).toFixed(1)} GB` : `${model.memoryMb} MB`);
  let percent = $derived(Math.round(Math.max(0, s.progress) * 100));
</script>

<article class="card" class:in-use={inUse} class:featured>
  <header>
    <div class="avatar"><BrandLogo family={model.family} size={28} /></div>
    <div class="title">
      <h3>{model.name}</h3>
      <span class="sub">by {model.family === "parakeet" ? "NVIDIA" : "OpenAI"}</span>
    </div>
    {#if inUse}
      <span class="badge live"><span class="dot"></span> In use</span>
    {:else if model.recommended}
      <span class="badge">Recommended</span>
    {/if}
  </header>

  <div class="meters">
    <Meter label="Speed" value={model.speed} />
    <Meter label="Accuracy" value={model.accuracy} />
  </div>

  <div class="facts">
    <span><Icon name="globe" size={14} /> {model.languages}</span>
    <span><Icon name="cpu" size={14} /> {model.hardware}</span>
    <span><Icon name="memory" size={14} /> {memory} memory</span>
  </div>

  {#if s.downloadError?.id === model.id && !downloading}
    <p class="failed" role="alert"><Icon name="alert" size={14} /> {s.downloadError.text}</p>
  {/if}

  <footer>
    {#if downloading}
      <div class="download">
        <div class="bar"><span style="transform: scaleX({Math.max(0.03, s.progress)})"></span></div>
        <div class="download-row">
          <span>{s.progress >= 0 ? `${percent} % · ` : ""}{downloadText(s)}</span>
          <button class="btn ghost sm" onclick={() => act("cancel")}>Cancel</button>
        </div>
      </div>
    {:else if !model.installed}
      <button class="btn primary wide" disabled={busy} onclick={() => act("download", { id: model.id })}>
        <Icon name="download" size={15} /> Download · {model.downloadMb} MB
      </button>
    {:else if !inUse}
      <button class="btn secondary wide" disabled={busy} onclick={() => act("useModel", { id: model.id })}>Use this model</button>
    {:else}
      <span class="ready"><Icon name="check" size={14} stroke={2.4} /> Ready on this PC</span>
      <button class="btn ghost sm repair" disabled={busy} onclick={() => act("download", { id: model.id })}>Repair</button>
    {/if}
  </footer>
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 14px;
    width: 100%;
    padding: 20px;
    border-radius: 22px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-card);
    transition:
      border-color var(--base) ease,
      box-shadow var(--base) ease;
  }
  .card:hover {
    border-color: var(--border-strong);
    box-shadow: 0 6px 18px rgba(16, 16, 20, 0.06);
  }
  .card.in-use {
    border-color: var(--brand-line);
    box-shadow: 0 0 0 3px var(--brand-soft);
  }
  .card.featured {
    padding: 24px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border-radius: 15px;
    flex-shrink: 0;
    background: var(--group);
    border: 1px solid var(--border);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .sub {
    margin-top: 1px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 10px;
    border-radius: 999px;
    background: var(--brand-soft);
    color: var(--brand);
    font-size: 11.5px;
    font-weight: 620;
    white-space: nowrap;
    align-self: flex-start;
  }
  .badge.live {
    background: var(--green-soft);
    color: var(--green-text);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--green);
  }
  .meters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 22px;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .facts span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px 0 9px;
    border-radius: 999px;
    background: var(--group);
    font-size: 12.5px;
    font-weight: 500;
    color: var(--chip-text);
  }
  .facts :global(svg) {
    color: var(--faint);
  }
  .failed {
    display: flex;
    gap: 7px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--red);
  }
  .failed :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    margin-top: auto;
    padding-top: 2px;
  }
  .wide {
    width: 100%;
    height: 40px;
  }
  .repair {
    margin-left: auto;
  }
  .ready {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .download {
    width: 100%;
  }
  .bar {
    height: 7px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--ink);
    transform-origin: left;
    transition: transform 300ms var(--ease);
  }
  .download-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 8px;
    font-size: 12.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
