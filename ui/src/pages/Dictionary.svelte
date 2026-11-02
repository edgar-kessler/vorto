<script>
  import { app, saveSettings, currentSettings } from "../lib/api.js";
  import Icon from "../lib/Icon.svelte";
  import Toggle from "../lib/Toggle.svelte";
  import { fade } from "svelte/transition";

  let s = $derived($app);
  let settings = $derived(s.settings);
  let whisper = $derived(s.models.find((m) => m.id === settings.model)?.family === "whisper");

  let word = $state("");
  let from = $state("");
  let to = $state("");

  function addWords(text) {
    const words = text
      .split(/[,;\n]/)
      .map((w) => w.trim())
      .filter(Boolean);
    const vocabulary = currentSettings().vocabulary;
    const known = new Set(vocabulary.map((w) => w.toLowerCase()));
    const fresh = words.filter((w) => !known.has(w.toLowerCase()));
    if (fresh.length) saveSettings({ vocabulary: [...vocabulary, ...fresh] });
  }
  function removeWord(w) {
    saveSettings({ vocabulary: currentSettings().vocabulary.filter((v) => v !== w) });
  }
  function dismiss(w) {
    saveSettings({ dismissed: [...currentSettings().dismissed, w] });
  }
  function addRule() {
    if (!from.trim()) return;
    const rules = currentSettings().replacements.filter((r) => r.from.toLowerCase() !== from.trim().toLowerCase());
    saveSettings({ replacements: [...rules, { from: from.trim(), to: to.trim() }] });
    from = "";
    to = "";
  }
  function removeRule(i) {
    saveSettings({ replacements: currentSettings().replacements.filter((_, j) => j !== i) });
  }
  function forget(l) {
    saveSettings({ learned: currentSettings().learned.filter((v) => v !== l && (v.from !== l.from || v.to !== l.to)) });
  }
  // Newest first.
  let learned = $derived([...settings.learned].reverse());
  const shown = (text) => (text ? text.replaceAll("\\n", "↵") : "(nothing)");
</script>

<header class="page-head">
  <h1>Dictionary</h1>
</header>

<h3 class="section-title">Learned from you</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Learn from my corrections</strong>
      <span>When you fix a word right after dictating, or in History, Vorto remembers it. Your words stay on this device.</span>
    </div>
    <div class="control"><Toggle label="Learn from my corrections" checked={settings.learn} onchange={(on) => saveSettings({ learn: on })} /></div>
  </div>
</div>
{#if learned.length}
  <div class="rules">
    {#each learned as l (l.from + l.to)}
      <div class="rule-row">
        <span class="from">{l.from}</span>
        <Icon name="arrow" size={14} />
        <span class="to">{l.to}</span>
        {#if l.count > 1}<span class="times" title="Corrected {l.count} times">{l.count}×</span>{/if}
        <button class="icon-btn" aria-label="Forget {l.from}" onclick={() => forget(l)}><Icon name="x" size={14} /></button>
      </div>
    {/each}
  </div>
{:else if settings.learn}
  <p class="hint">Nothing yet. Fix a misheard word in the app you dictated into, or use the pencil in History.</p>
{/if}

<h3 class="section-title">Your words</h3>
<div class="group pad">
  <p class="hint">
    Names and terms Vorto should spell your way.
    {whisper ? "Whisper listens for them" : "Parakeet checks them against what you said"}, and AI editing keeps them.
  </p>
  <form
    class="add"
    onsubmit={(e) => {
      e.preventDefault();
      addWords(word);
      word = "";
    }}
  >
    <input class="field" placeholder="For example Vorto, Kubernetes, Dr. Møller" aria-label="New word" bind:value={word} />
    <button class="btn secondary" disabled={!word.trim()}><Icon name="plus" size={14} /> Add</button>
  </form>
  {#if settings.vocabulary.length}
    <div class="chips">
      {#each settings.vocabulary as w (w)}
        <span class="word">
          {w}
          <button class="remove" aria-label="Remove {w}" onclick={() => removeWord(w)}><Icon name="x" size={12} stroke={2.2} /></button>
        </span>
      {/each}
    </div>
  {/if}
</div>

{#if s.suggestions.length}
  <h3 class="section-title" in:fade>From your dictations</h3>
  <div class="group pad" in:fade>
    <p class="hint">Words you use often that look like names or terms.</p>
    <div class="chips">
      {#each s.suggestions as w (w)}
        <span class="word suggestion">
          <button class="take" onclick={() => addWords(w)}><Icon name="plus" size={12} stroke={2.2} /> {w}</button>
          <button class="remove" aria-label="Don't suggest {w}" title="Don't suggest" onclick={() => dismiss(w)}><Icon name="x" size={12} stroke={2.2} /></button>
        </span>
      {/each}
    </div>
  </div>
{/if}

<h3 class="section-title">Replacements</h3>
<div class="group pad">
  <p class="hint">
    Vorto swaps these in every dictation, before AI editing. Whole words only, upper and lower case don't matter. Write <code>\n</code> for a line break, for example "new paragraph" → <code>\n\n</code>.
  </p>
  <form
    class="add rule"
    onsubmit={(e) => {
      e.preventDefault();
      addRule();
    }}
  >
    <input class="field" placeholder="When Vorto writes" aria-label="When Vorto writes" bind:value={from} />
    <Icon name="arrow" size={16} />
    <input class="field" placeholder="Write instead" aria-label="Write instead" bind:value={to} />
    <button class="btn secondary" disabled={!from.trim()}><Icon name="plus" size={14} /> Add</button>
  </form>
  {#if settings.replacements.length}
    <div class="rules">
      {#each settings.replacements as r, i (r.from)}
        <div class="rule-row" in:fade={{ duration: 160 }}>
          <span class="from">{r.from}</span>
          <Icon name="arrow" size={14} />
          <span class="to">{shown(r.to)}</span>
          <button class="icon-btn" aria-label="Remove {r.from}" onclick={() => removeRule(i)}><Icon name="trash" size={14} /></button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pad {
    padding-top: 6px;
  }
  .hint {
    padding: 6px 0;
    color: var(--muted);
  }
  .hint + .add {
    margin-top: 8px;
  }
  code {
    font-family: var(--mono);
    font-size: 12px;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--group);
  }
  .add {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .add :global(svg) {
    flex-shrink: 0;
    color: var(--faint);
  }
  .add .btn :global(svg) {
    color: inherit;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 12px;
  }
  .word {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 24px;
    padding: 0 3px 0 8px;
    border-radius: 4px;
    background: var(--control);
    box-shadow: var(--control-shadow);
  }
  .suggestion {
    padding-left: 3px;
  }
  .take {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 18px;
    padding: 0 4px;
    border-radius: 3px;
  }
  .take:hover {
    background: var(--hover);
  }
  .remove {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 3px;
    color: var(--faint);
  }
  .remove:hover {
    background: var(--hover);
    color: var(--text);
  }
  .rules {
    display: flex;
    flex-direction: column;
    margin-top: 6px;
  }
  .rule-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
  }
  .rule-row + .rule-row {
    border-top: 1px solid var(--border);
  }
  .rule-row :global(svg) {
    flex-shrink: 0;
    color: var(--faint);
  }
  .from,
  .to {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .from {
    color: var(--muted);
  }
  .times {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--faint);
  }
</style>
