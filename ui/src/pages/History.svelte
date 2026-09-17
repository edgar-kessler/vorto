<script>
  import { app, act, saveSettings } from "../lib/api.js";
  import Icon from "../lib/Icon.svelte";
  import Mark from "../lib/Mark.svelte";
  import AppIcon from "../lib/AppIcon.svelte";
  import { fade, scale } from "svelte/transition";

  let s = $derived($app);
  let query = $state("");
  let confirm = $state(false);
  let copied = $state("");

  const label = (date) => {
    const today = new Date();
    const iso = (d) => d.toLocaleDateString("sv-SE");
    if (date === iso(today)) return "Today";
    if (date === iso(new Date(today.getTime() - 86400000))) return "Yesterday";
    const day = new Date(`${date}T12:00:00`);
    const year = day.getFullYear() !== today.getFullYear() ? { year: "numeric" } : {};
    return day.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric", ...year });
  };
  let groups = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const out = [];
    for (const entry of s.history) {
      if (q && !entry.text.toLowerCase().includes(q)) continue;
      const date = entry.at.split(" ")[0];
      let group = out.at(-1);
      if (!group || group.date !== date) out.push((group = { date, items: [] }));
      group.items.push(entry);
    }
    return out;
  });
  // Entries are not unique (same words in the same minute), so rows are tracked by position.
  function copy(entry, key) {
    act("copy", { text: entry.text });
    copied = key;
    setTimeout(() => (copied = ""), 1400);
  }
  function clear() {
    confirm = false;
    query = "";
    act("clearHistory");
  }

  let clearButton = $state();
  $effect(() => {
    if (!confirm) return;
    const key = (e) => {
      if (e.key === "Escape") confirm = false;
    };
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("keydown", key);
      clearButton?.focus();
    };
  });
</script>

<header class="page-head">
  <h1>History</h1>
  {#if s.history.length}
    <button class="btn ghost sm" bind:this={clearButton} onclick={() => (confirm = true)}><Icon name="trash" size={15} /> Clear</button>
  {/if}
</header>

{#if !s.settings.history && s.history.length}
  <p class="paused">History is off <button class="btn ghost sm" onclick={() => saveSettings({ history: true })}>Turn on</button></p>
{/if}

{#if s.history.length > 8 || query}
  <div class="search">
    <Icon name="search" size={16} />
    <input class="selectable" placeholder="Search" aria-label="Search" bind:value={query} />
    {#if query}<button class="icon-btn" aria-label="Clear search" onclick={() => (query = "")}><Icon name="x" size={14} /></button>{/if}
  </div>
{/if}

{#if !s.history.length}
  <div class="empty" in:fade>
    <Mark size={80} mood="sleep" />
    {#if s.settings.history}
      <h2>No dictations yet</h2>
    {:else}
      <h2>History is off</h2>
      <button class="btn secondary sm" onclick={() => saveSettings({ history: true })}>Turn on</button>
    {/if}
  </div>
{:else if !groups.length}
  <div class="empty small-empty">
    <p class="muted">No matches</p>
  </div>
{:else}
  {#each groups as group, gi (gi)}
    <h3 class="section-title">{label(group.date)}</h3>
    <div class="list">
      {#each group.items as entry, i (i)}
        <button class="entry" onclick={() => copy(entry, `${gi}:${i}`)}>
          <AppIcon name={entry.app} size={30} />
          <span class="body">
            <span class="text">{entry.text}</span>
            <span class="meta">{entry.at.split(" ")[1]} · {entry.app || "Kept in Vorto"}</span>
          </span>
          <span class="action">
            {#if copied === `${gi}:${i}`}
              <span class="done" in:scale={{ duration: 160, start: 0.8 }}><Icon name="check" size={15} stroke={2.2} /></span>
            {:else}
              <Icon name="copy" size={15} />
            {/if}
          </span>
        </button>
      {/each}
    </div>
  {/each}
{/if}

<!-- The check icon confirms a copy on screen; this says it to screen readers. -->
<p class="sr-only" role="status">{copied ? "Copied" : ""}</p>

{#if confirm}
  {@const n = s.history.length}
  <div class="scrim" transition:fade={{ duration: 160 }} onclick={() => (confirm = false)} role="presentation">
    <div class="dialog" transition:scale={{ duration: 200, start: 0.96 }} onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === "Escape" && (confirm = false)} role="dialog" aria-modal="true" aria-labelledby="clear-title" tabindex="-1">
      <h2 id="clear-title">Clear {n} {n === 1 ? "dictation" : "dictations"}?</h2>
      <p class="muted">This can't be undone.</p>
      <div class="dialog-actions">
        <button class="btn secondary" {@attach (node) => node.focus()} onclick={() => (confirm = false)}>Cancel</button>
        <button class="btn danger" onclick={clear}>Clear</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .paused {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: -10px 2px 12px;
    font-size: 12.5px;
    color: var(--faint);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 40px;
    padding: 0 8px 0 13px;
    margin-bottom: 6px;
    border-radius: 12px;
    background: var(--group);
    color: var(--faint);
    border: 1px solid transparent;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .search:focus-within {
    background: var(--surface);
    border-color: var(--border-strong);
  }
  .search input {
    flex: 1;
    border: 0;
    outline: 0;
    background: none;
    font-size: 14px;
  }
  .search input::placeholder {
    color: var(--faint);
  }
  .list {
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: var(--r-lg);
    background: var(--group);
  }
  .entry {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 12px;
    border-radius: 12px;
    text-align: left;
    transition: background var(--fast) ease;
  }
  .entry:hover {
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .text {
    line-height: 1.5;
  }
  .meta {
    font-size: 12px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .action {
    display: grid;
    place-items: center;
    width: 20px;
    height: 21px;
    color: var(--faint);
    opacity: 0;
    transition: opacity var(--fast) ease;
  }
  .entry:hover .action,
  .entry:focus-visible .action,
  .action:has(.done) {
    opacity: 1;
  }
  .done {
    display: grid;
    color: var(--green);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 70px 0;
    text-align: center;
  }
  .empty h2 {
    margin-top: 12px;
  }
  .empty .btn {
    margin-top: 10px;
  }
  .small-empty {
    padding: 40px 0;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: var(--scrim);
  }
  .dialog {
    width: 380px;
    padding: 22px;
    border-radius: 18px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }
  .dialog p {
    margin-top: 6px;
    line-height: 1.5;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }
</style>
