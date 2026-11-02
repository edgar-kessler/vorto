<script>
  import { app, act, downloadText } from "../lib/api.js";
  import Icon from "../lib/Icon.svelte";
  import BrandLogo from "../lib/BrandLogo.svelte";
  import Meter from "../lib/Meter.svelte";
  import Select from "../lib/Select.svelte";
  import { languageOptions as options, bestFor, spokenLanguage, rememberLanguage } from "../lib/languages.js";

  let s = $derived($app);
  let busy = $derived(s.recording || s.downloading || s.phase === "transcribing");
  const memory = (mb) => (mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`);
  const maker = (m) => (m.id === "parakeet-de" ? "primeLine, on NVIDIA Parakeet" : m.id === "parakeet-ultra" ? "Moondream, on NVIDIA Parakeet" : m.family === "parakeet" ? "NVIDIA" : "OpenAI");
  let percent = $derived(Math.round(Math.max(0, s.progress) * 100));

  let speak = $state(spokenLanguage());
  $effect(() => rememberLanguage(speak));
  let pick = $derived(bestFor(speak));
  // Best first, then what's on this PC, then the rest in catalog order.
  let sorted = $derived(
    [...s.models].sort((a, b) => {
      const rank = (m) => (m.id === pick.id ? 0 : m.installed ? 1 : 2);
      return rank(a) - rank(b);
    }),
  );
  let chosen = $derived(s.models.find((m) => m.id === pick.id));
  let using = $derived(chosen?.installed && s.settings.model === chosen.id);
</script>

<header class="page-head">
  <h1>Voice models</h1>
  <div class="speak">
    <span>I mostly speak</span>
    <Select label="What you mostly speak" bind:value={speak} {options} width={170} />
  </div>
</header>

{#if chosen && !using}
  <div class="advice">
    <div>
      <strong>{chosen.name} fits you best</strong>
      <span>{pick.why}</span>
    </div>
    {#if chosen.installed}
      <button class="btn primary" disabled={busy} onclick={() => act("useModel", { id: chosen.id })}>Use it</button>
    {:else}
      <button class="btn primary" disabled={busy} onclick={() => act("download", { id: chosen.id })}><Icon name="download" size={14} /> Get it · {chosen.downloadMb} MB</button>
    {/if}
  </div>
{/if}

<div class="table" role="table" aria-label="Voice models">
  <div class="head" role="row">
    <span role="columnheader">Model</span>
    <span role="columnheader">Speed</span>
    <span role="columnheader">Accuracy</span>
    <span role="columnheader" class="num">Memory</span>
    <span role="columnheader" class="num"><span class="sr-only">Action</span></span>
  </div>
  {#each sorted as model (model.id)}
    {@const inUse = model.installed && s.settings.model === model.id}
    {@const downloading = s.downloading === model.id}
    <div class="model" class:in-use={inUse} role="row">
      <div class="name" role="cell">
        <span class="avatar"><BrandLogo family={model.family} size={16} /></span>
        <span class="title">
          <strong>
            {model.name}
            {#if model.id === pick.id}<span class="chip rec">Best for you</span>{/if}
          </strong>
          <span class="sub">{maker(model)} · {model.languages}</span>
        </span>
      </div>
      <div role="cell"><Meter label="Speed" value={model.speed} hideName /></div>
      <div role="cell"><Meter label="Accuracy" value={model.accuracy} hideName /></div>
      <div class="num" role="cell" title={model.hardware}>{memory(model.memoryMb)}</div>
      <div class="action num" role="cell">
        {#if downloading}
          <span class="percent">{s.progress >= 0 ? `${percent} %` : downloadText(s)}</span>
          <button class="btn ghost sm" onclick={() => act("cancel")}>Cancel</button>
        {:else if !model.installed}
          <button class="btn secondary sm" disabled={busy} onclick={() => act("download", { id: model.id })}><Icon name="download" size={13} /> {model.downloadMb} MB</button>
        {:else if !inUse}
          <button class="btn secondary sm" disabled={busy} onclick={() => act("useModel", { id: model.id })}>Use</button>
        {:else}
          <span class="live"><span class="dot"></span> In use</span>
          <button class="icon-btn repair" title="Repair the download" aria-label="Repair {model.name}" disabled={busy} onclick={() => act("download", { id: model.id })}><Icon name="refresh" size={13} /></button>
        {/if}
      </div>
      {#if downloading}
        <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={percent}><span style="transform: scaleX({Math.max(0.02, s.progress)})"></span></div>
      {/if}
      {#if s.downloadError?.id === model.id && !downloading}
        <p class="failed" role="alert"><Icon name="alert" size={13} /> {s.downloadError.text}</p>
      {/if}
    </div>
  {/each}
</div>

<p class="note">
  Models are downloaded from Hugging Face and checked before use. Your voice never leaves this device. Memory is measured on the processor; with a graphics card, most of it moves to video memory.
</p>

<style>
  .speak {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
  }
  .advice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 24px;
    padding: 12px 12px 12px 16px;
    border-radius: 6px;
    box-shadow: var(--shadow-card);
  }
  .advice strong {
    display: block;
    font-weight: 530;
  }
  .advice span {
    color: var(--muted);
  }
  .table {
    --columns: minmax(0, 1fr) 76px 76px 72px 124px;
    margin: 0 -16px;
  }
  .head,
  .model {
    display: grid;
    grid-template-columns: var(--columns);
    align-items: center;
    column-gap: 16px;
    padding: 0 16px;
  }
  .head {
    height: 32px;
    margin-bottom: 8px;
    border-radius: 6px;
    background: var(--group);
    color: var(--faint);
    font-weight: 530;
  }
  .model {
    position: relative;
    min-height: 56px;
    padding-block: 8px;
    border-radius: 6px;
  }
  .model:hover {
    background: var(--row-hover);
  }
  .num {
    text-align: right;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    border-radius: 6px;
    background: var(--control);
    box-shadow: var(--control-shadow);
  }
  .title {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title strong {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 530;
    white-space: nowrap;
  }
  .sub {
    overflow: hidden;
    color: var(--muted);
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .action {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    font-weight: 530;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--green);
  }
  .repair {
    width: 24px;
    height: 24px;
  }
  .percent {
    color: var(--muted);
  }
  .bar {
    grid-column: 1 / -1;
    height: 2px;
    margin-top: 8px;
    border-radius: 2px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--text);
    transform-origin: left;
    transition: transform 200ms linear;
  }
  .failed {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    color: var(--red-text);
  }
  .note {
    margin-top: 28px;
    max-width: 560px;
    color: var(--faint);
  }
  @media (max-width: 880px) {
    .table {
      --columns: minmax(0, 1fr) 64px 64px 124px;
    }
    .head > :nth-child(4),
    .model > :nth-child(4) {
      display: none;
    }
  }
</style>
