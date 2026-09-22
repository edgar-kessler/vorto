<script>
  import { app, saveSettings, currentSettings } from "../lib/api.js";
  import Icon from "../lib/Icon.svelte";
  import { fade, scale } from "svelte/transition";

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
  const shown = (text) => (text ? text.replaceAll("\\n", "↵") : "(nothing)");
</script>

<header class="page-head">
  <h1>Dictionary</h1>
</header>

<h3 class="section-title">Your words</h3>
<div class="group pad">
  <p class="hint">
    Names and terms Vorto should spell your way.
    {whisper ? "Whisper listens for them" : "Whisper listens for them, Parakeet doesn't"}, and AI editing keeps them.
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
    <button class="btn primary sm" disabled={!word.trim()}><Icon name="plus" size={15} /> Add</button>
  </form>
  {#if settings.vocabulary.length}
    <div class="chips">
      {#each settings.vocabulary as w (w)}
        <span class="word" in:scale={{ start: 0.8, duration: 180 }}>
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
    <button class="btn primary sm" disabled={!from.trim()}><Icon name="plus" size={15} /> Add</button>
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
    padding: 16px 18px;
  }
  .hint {
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.5;
    margin-bottom: 12px;
  }
  code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--surface);
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
    margin-top: 14px;
  }
  .word {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 4px 0 11px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    font-size: 13px;
  }
  .suggestion {
    padding-left: 4px;
    border-style: dashed;
  }
  .take {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 6px;
    border-radius: 999px;
    font-size: 13px;
  }
  .take:hover {
    background: var(--hover);
  }
  .remove {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 999px;
    color: var(--faint);
  }
  .remove:hover {
    background: var(--hover);
    color: var(--text);
  }
  .rules {
    display: flex;
    flex-direction: column;
    margin-top: 10px;
  }
  .rule-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    font-size: 13.5px;
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
</style>
