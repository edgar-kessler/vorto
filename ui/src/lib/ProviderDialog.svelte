<script>
  // Adding an AI editing provider: pick one from the list, paste a key, choose a model. Used by
  // the AI editing page and the setup. `onadded` gets the new provider's id once it has a model.
  import { slide, fade, scale } from "svelte/transition";
  import { app, act, saveSettings, currentSettings } from "./api.js";
  import SearchSelect from "./SearchSelect.svelte";
  import Icon from "./Icon.svelte";
  import ProviderLogo from "./ProviderLogo.svelte";

  let { open = $bindable(false), onadded, online = false } = $props();
  let s = $derived($app);

  // Everything that speaks the OpenAI chat API uses "openai"; Anthropic has its own.
  const catalog = [
    { id: "ollama", name: "Ollama", kind: "openai", url: "http://localhost:11434/v1", local: true, about: "Free models on this PC", site: "https://ollama.com/download" },
    { id: "lmstudio", name: "LM Studio", kind: "openai", url: "http://localhost:1234/v1", local: true, about: "Free models on this PC", site: "https://lmstudio.ai" },
    { id: "openai", name: "OpenAI", kind: "openai", url: "https://api.openai.com/v1", about: "GPT models", keys: "https://platform.openai.com/api-keys" },
    { id: "anthropic", name: "Anthropic", kind: "anthropic", url: "https://api.anthropic.com", about: "Claude models", keys: "https://console.anthropic.com/settings/keys" },
    { id: "gemini", name: "Google Gemini", kind: "openai", url: "https://generativelanguage.googleapis.com/v1beta/openai", about: "Gemini models", keys: "https://aistudio.google.com/apikey" },
    { id: "groq", name: "Groq", kind: "openai", url: "https://api.groq.com/openai/v1", about: "Very fast open models", keys: "https://console.groq.com/keys" },
    { id: "mistral", name: "Mistral", kind: "openai", url: "https://api.mistral.ai/v1", about: "European models", keys: "https://console.mistral.ai/api-keys" },
    { id: "openrouter", name: "OpenRouter", kind: "openai", url: "https://openrouter.ai/api/v1", about: "Hundreds of models, one key", keys: "https://openrouter.ai/keys" },
    { id: "deepseek", name: "DeepSeek", kind: "openai", url: "https://api.deepseek.com/v1", about: "DeepSeek models", keys: "https://platform.deepseek.com/api_keys" },
    { id: "xai", name: "xAI", kind: "openai", url: "https://api.x.ai/v1", about: "Grok models", keys: "https://console.x.ai" },
    { id: "together", name: "Together AI", kind: "openai", url: "https://api.together.xyz/v1", about: "Open models in the cloud", keys: "https://api.together.ai/settings/api-keys" },
    { id: "custom", name: "Other provider", kind: "openai", url: "", about: "Any OpenAI-compatible address", custom: true },
  ];

  function saveAi(change) {
    const current = structuredClone(currentSettings().ai);
    change(current);
    saveSettings({ ai: current });
  }
  const setProvider = (id, patch) => saveAi((a) => Object.assign(a.providers.find((p) => p.id === id) ?? {}, patch));
  function unique(base, taken) {
    let id = base;
    for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
    return id;
  }
  const modelOption = (m) => ({ value: m.id, label: m.name || m.id, hint: m.hint || (m.name ? m.id : "") });

  let dialog = $state(null); // null, or { step: "pick" | "setup", entry, id, key, url, error }
  let providerQuery = $state("");
  // Names that start with the search first, then names containing it, then descriptions.
  let shownCatalog = $derived.by(() => {
    const q = providerQuery.trim().toLowerCase();
    const rank = (c) => {
      const name = c.name.toLowerCase();
      return name.startsWith(q) ? 0 : name.includes(q) ? 1 : c.about.toLowerCase().includes(q) ? 2 : 3;
    };
    return catalog.filter((c) => rank(c) < 3).sort((a, b) => rank(a) - rank(b));
  });
  const running = (entry) => s.localAi?.find((l) => l.id === entry.id);

  function openDialog() {
    providerQuery = "";
    dialog = { step: "pick" };
  }
  function choose(entry) {
    const id = unique(entry.id, currentSettings().ai.providers.map((p) => p.id));
    dialog = { step: "setup", entry, id, key: "", url: entry.url, error: "", connected: false };
    // Local servers need no key: connect right away.
    if (entry.local) connect();
  }
  function connect() {
    const { entry, id, key, url } = dialog;
    if (!url.trim()) return (dialog.error = "Enter the provider's address.");
    if (!entry.local && !key.trim() && !entry.custom) return (dialog.error = "Paste your API key.");
    if (key.trim()) act("setApiKey", { provider: id, key: key.trim() });
    saveAi((a) => {
      if (!a.providers.some((p) => p.id === id)) {
        a.providers.push({ id, name: entry.name, kind: entry.kind, base_url: url.trim(), model: "", allow_remote: !entry.local });
      }
    });
    dialog.error = "";
    dialog.connected = true;
    setTimeout(() => act("listModels", { provider: id }), 150);
  }
  function finish(model) {
    setProvider(dialog.id, { model });
    const id = dialog.id;
    close();
    onadded?.(id);
  }
  let setupModels = $derived(dialog?.step === "setup" ? s.aiModels[dialog.id] : null);

  // Esc closes the dialog.
  $effect(() => {
    if (!dialog) return;
    const key = (e) => e.key === "Escape" && close();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });

  // The parent opens it with `open`; every way out closes both, so it never reopens itself.
  // `online` leaves out the local servers.
  function close() {
    dialog = null;
    open = false;
  }
  let shown = false;
  $effect(() => {
    if (open && !shown) openDialog();
    if (!open) dialog = null;
    shown = open;
  });
  let listed = $derived(online ? shownCatalog.filter((c) => !c.local) : shownCatalog);
</script>

{#if dialog}
  <div class="scrim" transition:fade={{ duration: 180 }} onclick={close} role="presentation">
    <div class="modal" class:wide={dialog.step === "pick"} transition:scale={{ duration: 240, start: 0.96, opacity: 0 }} onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="provider-title" tabindex="-1">
      {#if dialog.step === "pick"}
        <header class="modal-head">
          <div>
            <h2 id="provider-title">Add a provider</h2>
            <p>Pick where your words are edited. You can add more later.</p>
          </div>
          <button class="icon-btn" aria-label="Close" onclick={close}><Icon name="x" size={16} /></button>
        </header>
        <div class="find">
          <Icon name="search" size={16} />
          <input {@attach (node) => node.focus()} placeholder="Search providers" aria-label="Search providers" bind:value={providerQuery} onkeydown={(e) => e.key === "Enter" && listed[0] && choose(listed[0])} />
          {#if providerQuery}<button class="icon-btn small" aria-label="Clear search" onclick={() => (providerQuery = "")}><Icon name="x" size={13} /></button>{/if}
        </div>
        <div class="catalog">
          {#each [["On this PC", listed.filter((c) => c.local)], ["Online", listed.filter((c) => !c.local)]] as [group, entries] (group)}
            {#if entries.length}
              <h3 class="group-title">{group}</h3>
              <div class="grid">
                {#each entries as entry (entry.id)}
                  <button class="tile" onclick={() => choose(entry)}>
                    <ProviderLogo id={entry.id} size={40} />
                    <span class="tile-text">
                      <strong>{entry.name}</strong>
                      <span>{entry.about}</span>
                    </span>
                    {#if running(entry)}<span class="dot" title="Running on this PC"></span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          {:else}
            <p class="none">No provider called “{providerQuery}”. <button class="inline-link" onclick={() => choose(catalog.at(-1))}>Add one by its address</button></p>
          {/each}
        </div>
      {:else}
        {@const entry = dialog.entry}
        <header class="modal-head setup">
          <button class="icon-btn" aria-label="Back to all providers" onclick={() => (dialog = { step: "pick" })}><Icon name="right" size={16} /></button>
          <ProviderLogo id={entry.id} size={44} />
          <div>
            <h2 id="provider-title">{entry.name}</h2>
            <p>{entry.about}</p>
          </div>
          <button class="icon-btn" aria-label="Close" onclick={close}><Icon name="x" size={16} /></button>
        </header>
        <div class="modal-body">
          {#if !dialog.connected}
            {#if entry.custom}
              <label class="stack-field"><span>Address</span><input class="field mono" placeholder="https://example.com/v1" bind:value={dialog.url} /></label>
            {/if}
            <label class="stack-field">
              <span class="label-row">API key {entry.custom ? "(if it needs one)" : ""}
                {#if entry.keys}<button class="inline-link" onclick={() => act("openUrl", { url: entry.keys })}>Get a key <Icon name="external" size={12} /></button>{/if}
              </span>
              <input class="field big" type="password" {@attach (node) => node.focus()} placeholder="Paste your key" bind:value={dialog.key} onkeydown={(e) => e.key === "Enter" && connect()} />
            </label>
            {#if dialog.error}<p class="problem" transition:slide={{ duration: 160 }}>{dialog.error}</p>{/if}
            <div class="facts">
              <p><Icon name="globe" size={15} /> {entry.custom ? "This provider" : entry.name} receives the text of your dictations. Your voice stays on this device.</p>
              <p><Icon name="key" size={15} /> The key is kept in Windows Credential Manager, not in a file.</p>
            </div>
          {:else if setupModels?.status === "error"}
            <p class="problem">{setupModels.error}</p>
            {#if entry.local}<p class="muted small">Start {entry.name}, or install it first. Then try again.</p>{/if}
          {:else if setupModels?.status === "done"}
            <label class="stack-field">
              <span>Model</span>
              <SearchSelect inline label="Model" search="Search, or type a model ID" custom options={setupModels.models.map(modelOption)} onchange={finish} />
            </label>
            {#if entry.local && !setupModels.models.length}
              <p class="muted small">No models yet. In a terminal, run <code class="selectable">ollama pull qwen2.5:3b</code>, then try again.</p>
            {:else}
              <p class="muted small">{setupModels.models.length} {setupModels.models.length === 1 ? "model" : "models"} available. You can change it any time.</p>
            {/if}
          {:else}
            <p class="connecting"><span class="spinner"></span> Connecting to {entry.name}…</p>
          {/if}
        </div>
        <footer class="modal-foot">
          {#if !dialog.connected}
            <button class="btn secondary" onclick={close}>Cancel</button>
            <button class="btn primary" onclick={connect}>Connect</button>
          {:else if setupModels?.status === "error"}
            {#if entry.local}
              <button class="btn secondary" onclick={() => act("openUrl", { url: entry.site })}>Get {entry.name} <Icon name="external" size={13} /></button>
            {:else}
              <button class="btn secondary" onclick={() => (dialog.connected = false)}>Change key</button>
            {/if}
            <button class="btn primary" onclick={() => act("listModels", { provider: dialog.id })}>Try again</button>
          {:else}
            <button class="btn secondary" onclick={close}>{setupModels?.status === "done" ? "Choose later" : "Close"}</button>
          {/if}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--scrim);
    backdrop-filter: blur(6px);
  }
  .modal {
    display: flex;
    flex-direction: column;
    width: 460px;
    max-width: 100%;
    max-height: min(680px, calc(100vh - 48px));
    border-radius: 22px;
    background: var(--surface);
    box-shadow:
      var(--shadow-pop),
      0 0 0 1px var(--border);
    overflow: hidden;
  }
  .modal.wide {
    width: 620px;
  }
  .modal-head {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 22px 22px 16px 24px;
  }
  .modal-head > div {
    flex: 1;
    min-width: 0;
  }
  .modal-head h2 {
    font-size: 18px;
    letter-spacing: -0.02em;
  }
  .modal-head p {
    margin-top: 3px;
    font-size: 13px;
    color: var(--muted);
  }
  .modal-head.setup {
    align-items: center;
    padding-left: 14px;
  }
  .modal-head.setup > .icon-btn:first-child :global(svg) {
    transform: rotate(180deg);
  }
  .modal-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 4px 24px 20px;
  }
  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
    background: var(--group);
  }
  .find {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    margin: 0 24px 6px;
    padding: 0 8px 0 14px;
    border-radius: 12px;
    background: var(--group);
    border: 1px solid transparent;
    color: var(--faint);
    flex-shrink: 0;
    transition:
      border-color var(--fast) ease,
      background var(--fast) ease;
  }
  .find:focus-within {
    background: var(--surface);
    border-color: var(--border-strong);
  }
  .find input {
    flex: 1;
    border: 0;
    outline: 0;
    background: none;
    font-size: 14px;
    color: var(--text);
    user-select: text;
  }
  .icon-btn.small {
    width: 24px;
    height: 24px;
  }
  .catalog {
    padding: 4px 16px 20px;
    overflow-y: auto;
  }
  .group-title {
    margin: 14px 8px 8px;
    font-size: 12px;
    font-weight: 560;
    color: var(--muted);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .tile {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border-radius: 14px;
    border: 1px solid var(--border);
    text-align: left;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .tile:hover,
  .tile:focus-visible {
    background: var(--group);
    border-color: var(--border-strong);
  }
  .tile:active {
    transform: scale(0.985);
  }
  .tile-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .tile-text strong {
    font-weight: 560;
    font-size: 14px;
  }
  .tile-text span {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--green);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--green) 20%, transparent);
  }
  .none {
    padding: 20px 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .inline-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--brand);
    font-weight: 540;
  }
  .inline-link:hover {
    text-decoration: underline;
  }
  .stack-field {
    display: flex;
    flex-direction: column;
    gap: 7px;
    font-size: 12.5px;
    font-weight: 540;
    color: var(--muted);
  }
  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .field.big {
    height: 40px;
    font-size: 14px;
  }
  .facts {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--group);
  }
  .facts p {
    display: flex;
    gap: 9px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .facts :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }
  .connecting {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13.5px;
    color: var(--muted);
    padding: 10px 0;
  }
  .spinner {
    width: 16px;
    height: 16px;
    border-radius: 999px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--brand);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--group);
  }
</style>
