<script>
  import { app, act, downloadText } from "./api.js";
  import Icon from "./Icon.svelte";
  import BrandLogo from "./BrandLogo.svelte";

  let { model, featured = false } = $props();
  let s = $derived($app);
  let inUse = $derived(model.installed && s.settings.model === model.id);
  let downloading = $derived(s.downloading === model.id);
  let busy = $derived(s.recording || s.downloading || s.phase === "transcribing");
</script>

<article class="card" class:in-use={inUse} class:featured>
  <header>
    <div class="avatar"><BrandLogo family={model.family} size={26} /></div>
    <h3>{model.name}</h3>
    {#if model.recommended && !inUse}<span class="chip rec">Recommended</span>{/if}
  </header>

  <dl class="facts">
    <div><dt><Icon name="memory" size={14} /></dt><dd>~{model.memoryMb >= 1000 ? `${(model.memoryMb / 1000).toFixed(1)} GB` : `${model.memoryMb} MB`} memory</dd></div>
    <div><dt><Icon name="globe" size={14} /></dt><dd>{model.languages}</dd></div>
  </dl>

  <div class="meters">
    <div class="meter" role="img" aria-label="Speed {model.speed} of 5"><span aria-hidden="true">Speed</span><i aria-hidden="true" style="--v:{model.speed}"></i></div>
    <div class="meter" role="img" aria-label="Accuracy {model.accuracy} of 5"><span aria-hidden="true">Accuracy</span><i aria-hidden="true" style="--v:{model.accuracy}"></i></div>
  </div>
  <p class="hardware"><Icon name="cpu" size={13} /> {model.hardware}</p>

  {#if s.downloadError?.id === model.id && !downloading}
    <p class="failed" role="alert">{s.downloadError.text}</p>
  {/if}

  <footer>
    {#if downloading}
      <div class="download">
        <div class="bar"><span style="width:{Math.max(3, s.progress * 100)}%"></span></div>
        <div class="download-row">
          <span>{downloadText(s)}</span>
          <button class="btn ghost sm" onclick={() => act("cancel")}>Cancel</button>
        </div>
      </div>
    {:else if !model.installed}
      <button class="btn primary" disabled={busy} onclick={() => act("download", { id: model.id })}>
        <Icon name="download" size={15} /> Download · {model.downloadMb} MB
      </button>
    {:else if !inUse}
      <button class="btn secondary" disabled={busy} onclick={() => act("useModel", { id: model.id })}>Use this model</button>
    {:else}
      <span class="ready"><Icon name="check" size={14} stroke={2.4} /> In use</span>
      <button class="btn ghost sm repair" disabled={busy} onclick={() => act("download", { id: model.id })}>Repair</button>
    {/if}
  </footer>
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    width: 100%;
    padding: 18px 18px 16px;
    border-radius: var(--r-xl);
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
    padding: 22px;
  }
  header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 50px;
    height: 50px;
    border-radius: 15px;
    flex-shrink: 0;
    background: var(--group);
    border: 1px solid var(--border);
  }
  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.012em;
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 14px 0 0;
  }
  .facts div {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px 0 8px;
    border-radius: 999px;
    background: var(--group);
    font-size: 12.5px;
    color: var(--chip-text);
  }
  .facts dt {
    display: grid;
    color: var(--faint);
  }
  .facts dd {
    margin: 0;
    font-weight: 500;
  }
  .meters {
    display: flex;
    gap: 22px;
    margin-top: 14px;
  }
  .meter {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .meter i {
    width: 64px;
    height: 5px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--ink) calc(var(--v) * 20%), var(--bar-bg) 0);
  }
  .hardware {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
    font-size: 12.5px;
    color: var(--faint);
  }
  .failed {
    margin-top: 12px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--red);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    margin-top: 16px;
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
    height: 6px;
    border-radius: 999px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--ink);
    border-radius: inherit;
    transition: width 300ms var(--ease);
  }
  .download-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 12.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
