<script>
  import { app, act, saveSettings, currentSettings } from "../lib/api.js";
  import { slide, fade } from "svelte/transition";
  import Toggle from "../lib/Toggle.svelte";
  import Select from "../lib/Select.svelte";
  import Icon from "../lib/Icon.svelte";
  import { get } from "svelte/store";

  let s = $derived($app);
  let ai = $derived(s.settings.ai);

  // Everything that speaks the OpenAI chat API uses "openai"; Anthropic has its own.
  const presets = [
    { id: "ollama", name: "Ollama", kind: "openai", base_url: "http://localhost:11434/v1", note: "On this PC" },
    { id: "lmstudio", name: "LM Studio", kind: "openai", base_url: "http://localhost:1234/v1", note: "On this PC" },
    { id: "openai", name: "OpenAI", kind: "openai", base_url: "https://api.openai.com/v1" },
    { id: "anthropic", name: "Anthropic", kind: "anthropic", base_url: "https://api.anthropic.com" },
    { id: "gemini", name: "Google Gemini", kind: "openai", base_url: "https://generativelanguage.googleapis.com/v1beta/openai" },
    { id: "groq", name: "Groq", kind: "openai", base_url: "https://api.groq.com/openai/v1" },
    { id: "mistral", name: "Mistral", kind: "openai", base_url: "https://api.mistral.ai/v1" },
    { id: "openrouter", name: "OpenRouter", kind: "openai", base_url: "https://openrouter.ai/api/v1" },
    { id: "deepseek", name: "DeepSeek", kind: "openai", base_url: "https://api.deepseek.com/v1" },
    { id: "xai", name: "xAI", kind: "openai", base_url: "https://api.x.ai/v1" },
    { id: "together", name: "Together AI", kind: "openai", base_url: "https://api.together.xyz/v1" },
    { id: "custom", name: "Custom provider", kind: "openai", base_url: "", note: "Any OpenAI-compatible address" },
  ];
  const timeouts = [10, 20, 30, 60, 120].map((v) => ({ value: v, label: `${v} seconds` }));

  function saveAi(change) {
    const current = structuredClone(currentSettings().ai);
    change(current);
    saveSettings({ ai: current });
  }
  const setProvider = (id, patch) => saveAi((a) => Object.assign(a.providers.find((p) => p.id === id) ?? {}, patch));
  const setProfile = (id, patch) => saveAi((a) => Object.assign(a.profiles.find((p) => p.id === id) ?? {}, patch));
  const list = (text) => text.split(",").map((t) => t.trim()).filter(Boolean);
  function unique(base, taken) {
    let id = base;
    for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
    return id;
  }

  function addProvider(presetId) {
    const preset = presets.find((p) => p.id === presetId);
    if (!preset) return;
    let id;
    saveAi((a) => {
      id = unique(preset.id, a.providers.map((p) => p.id));
      a.providers.push({ id, name: preset.name, kind: preset.kind, base_url: preset.base_url, model: "", allow_remote: false });
    });
    open = id;
    if (preset.note === "On this PC") act("listModels", { provider: id });
  }
  function removeProvider(id) {
    saveAi((a) => {
      a.providers = a.providers.filter((p) => p.id !== id);
      for (const p of a.profiles) if (p.provider === id) p.provider = "";
    });
  }

  let open = $state(get(app).settings.ai.providers[0]?.id ?? "");
  let keys = $state({});
  function saveKey(id) {
    act("setApiKey", { provider: id, key: keys[id] ?? "" });
    keys[id] = "";
    setTimeout(() => act("listModels", { provider: id }), 300);
  }

  let openStyle = $state("");
  function addStyle() {
    let id;
    saveAi((a) => {
      id = unique("style", a.profiles.map((p) => p.id));
      // Before the style that applies everywhere, so its apps are tried first.
      const at = a.profiles.findIndex((p) => !p.apps.length && !p.titles.length);
      const style = { id, name: "New style", enabled: true, prompt: "", provider: "", model: "", apps: [], titles: [] };
      a.profiles.splice(at < 0 ? a.profiles.length : at, 0, style);
    });
    openStyle = id;
  }
  function moveStyle(id, by) {
    saveAi((a) => {
      const i = a.profiles.findIndex((p) => p.id === id);
      const j = i + by;
      if (i < 0 || j < 0 || j >= a.profiles.length) return;
      [a.profiles[i], a.profiles[j]] = [a.profiles[j], a.profiles[i]];
    });
  }
  const where = (p) => {
    const places = [...p.apps.map((a) => a.replace(/\.exe$/i, "")), ...p.titles.map((t) => `“${t}”`)];
    return places.length ? places.join(", ") : "Everywhere else";
  };
  const providerOf = (p) => ai.providers.find((x) => x.id === p.provider) ?? ai.providers[0];
  const localOf = (provider) => s.providersLocal[ai.providers.findIndex((p) => p.id === provider?.id)] ?? true;
  // Cloud providers that at least one enabled style sends text to.
  let cloud = $derived(
    [...new Set(ai.profiles.filter((p) => p.enabled).map(providerOf).filter((p) => p && !localOf(p)).map((p) => p.name))],
  );

  let sample = $state("ähm also ich wollte halt fragen ob wir das meeting morgen auf zehn uhr verschieben können danke");
  let testStyle = $state(get(app).settings.ai.profiles.find((p) => p.enabled)?.id ?? "");
  let test = $derived(s.aiTest);
</script>

<header class="page-head">
  <h1>AI editing</h1>
</header>

<div class="group">
  <div class="row">
    <div class="label">
      <strong>AI editing</strong>
      <span>A language model polishes each dictation before Vorto inserts it, in the style that fits the app. Press Esc while the pill says Polishing to insert your words as spoken.</span>
    </div>
    <div class="control"><Toggle label="AI editing" checked={ai.enabled} onchange={(v) => saveAi((a) => (a.enabled = v))} /></div>
  </div>
  <div class="row">
    <div class="label">
      <strong>Wait at most</strong>
      <span>After that, Vorto inserts your words as spoken.</span>
    </div>
    <div class="control">
      <Select label="Wait at most" value={ai.timeout_secs} options={timeouts} width={160} onchange={(v) => saveAi((a) => (a.timeout_secs = v))} />
    </div>
  </div>
</div>
{#if ai.enabled && cloud.length}
  <p class="privacy" transition:slide={{ duration: 220 }}>
    <Icon name="globe" size={15} /> The text you dictate goes to {cloud.join(" and ")}. Your voice stays on this device.
  </p>
{/if}

<h3 class="section-title">Providers</h3>
<div class="stack">
  {#each ai.providers as provider (provider.id)}
    {@const local = localOf(provider)}
    {@const models = s.aiModels[provider.id]}
    <div class="card">
      <button class="card-head" aria-expanded={open === provider.id} onclick={() => (open = open === provider.id ? "" : provider.id)}>
        <span class="title">{provider.name}</span>
        <span class="chip">{local ? "On this PC" : "Online"}</span>
        <span class="meta">{provider.model || "No model chosen"}</span>
        <span class="chev" class:turned={open === provider.id}><Icon name="chevron" size={16} /></span>
      </button>
      {#if open === provider.id}
        <div class="card-body" transition:slide={{ duration: 220 }}>
          {#if !local && !provider.allow_remote}
            <div class="consent">
              <p>Vorto will send the text of your dictations to {provider.name} for editing. Their terms and privacy policy apply. Your voice recording stays on this device.</p>
              <button class="btn primary sm" onclick={() => setProvider(provider.id, { allow_remote: true })}>Allow</button>
            </div>
          {/if}
          <label class="field-row">
            <span>Name</span>
            <input class="field" value={provider.name} onchange={(e) => setProvider(provider.id, { name: e.target.value.trim() || provider.name })} />
          </label>
          <label class="field-row">
            <span>Address</span>
            <input class="field" value={provider.base_url} placeholder="https://…/v1" onchange={(e) => setProvider(provider.id, { base_url: e.target.value, allow_remote: false })} />
          </label>
          <div class="field-row">
            <span>API key</span>
            {#if s.apiKeys[provider.id]}
              <span class="saved"><Icon name="key" size={14} /> Saved in Windows Credential Manager</span>
              <button class="btn ghost sm" onclick={() => act("setApiKey", { provider: provider.id, key: "" })}>Remove</button>
            {:else}
              <input class="field" type="password" placeholder={local ? "Usually not needed" : "Paste your key"} bind:value={keys[provider.id]} onkeydown={(e) => e.key === "Enter" && saveKey(provider.id)} />
              <button class="btn secondary sm" disabled={!keys[provider.id]?.trim()} onclick={() => saveKey(provider.id)}>Save</button>
            {/if}
          </div>
          <div class="field-row">
            <span>Model</span>
            <input class="field" list="models-{provider.id}" value={provider.model} placeholder="Type or pick a model" onchange={(e) => setProvider(provider.id, { model: e.target.value.trim() })} />
            <datalist id="models-{provider.id}">
              {#each models?.models ?? [] as m}<option value={m}></option>{/each}
            </datalist>
            <button class="btn secondary sm" disabled={models?.status === "loading"} onclick={() => act("listModels", { provider: provider.id })}>
              <Icon name="refresh" size={14} /> {models?.status === "loading" ? "Loading…" : "Load models"}
            </button>
          </div>
          {#if models?.status === "error"}
            <p class="error" transition:fade>{models.error}</p>
          {:else if models?.status === "done"}
            <p class="ok" transition:fade>{models.models.length} {models.models.length === 1 ? "model" : "models"} available. Pick one above.</p>
          {/if}
          <div class="card-foot">
            <span class="faint small">{provider.kind === "anthropic" ? "Anthropic Messages API" : "OpenAI-compatible API"}</span>
            <button class="btn ghost sm" onclick={() => removeProvider(provider.id)}><Icon name="trash" size={14} /> Remove</button>
          </div>
        </div>
      {/if}
    </div>
  {/each}
  <div class="add-provider">
    <!-- Recreated after each pick, so it reads "Add a provider…" again. -->
    {#key ai.providers.length}
    <Select label="Add a provider" value="" placeholder="Add a provider…" width={240} align="left" options={presets.map((p) => ({ value: p.id, label: p.note ? `${p.name} · ${p.note}` : p.name }))} onchange={addProvider} />
    {/key}
  </div>
</div>

<h3 class="section-title">Styles</h3>
<p class="lead">Vorto uses the first style whose apps or window titles match where you dictate. A window title works for websites, since browsers show the page title.</p>
<div class="stack">
  {#each ai.profiles as profile, i (profile.id)}
    <div class="card" class:off={!profile.enabled}>
      <div class="card-head as-row">
        <Toggle label="Use {profile.name}" checked={profile.enabled} onchange={(v) => setProfile(profile.id, { enabled: v })} />
        <button class="head-button" aria-expanded={openStyle === profile.id} onclick={() => (openStyle = openStyle === profile.id ? "" : profile.id)}>
          <span class="title">{profile.name}</span>
          <span class="meta">{where(profile)}</span>
          <span class="chev" class:turned={openStyle === profile.id}><Icon name="chevron" size={16} /></span>
        </button>
      </div>
      {#if openStyle === profile.id}
        <div class="card-body" transition:slide={{ duration: 220 }}>
          <label class="field-row">
            <span>Name</span>
            <input class="field" value={profile.name} onchange={(e) => setProfile(profile.id, { name: e.target.value.trim() || profile.name })} />
          </label>
          <label class="field-row top">
            <span>Instructions</span>
            <textarea class="field" rows="4" value={profile.prompt} placeholder="What should happen to your words? For example: Make it polite and formal." onchange={(e) => setProfile(profile.id, { prompt: e.target.value })}></textarea>
          </label>
          <label class="field-row">
            <span>Apps</span>
            <input class="field" value={profile.apps.join(", ")} placeholder="outlook.exe, slack.exe" onchange={(e) => setProfile(profile.id, { apps: list(e.target.value) })} />
          </label>
          <label class="field-row">
            <span>Window titles</span>
            <input class="field" value={profile.titles.join(", ")} placeholder="Gmail, ChatGPT" onchange={(e) => setProfile(profile.id, { titles: list(e.target.value) })} />
          </label>
          <div class="field-row">
            <span>Provider</span>
            <Select
              label="Provider"
              value={profile.provider}
              width={220}
              options={[{ value: "", label: `Default (${ai.providers[0]?.name ?? "none"})` }, ...ai.providers.map((p) => ({ value: p.id, label: p.name }))]}
              onchange={(v) => setProfile(profile.id, { provider: v })}
            />
            <input class="field" list="models-{providerOf(profile)?.id}" value={profile.model} placeholder="Provider's model" aria-label="Model for this style" onchange={(e) => setProfile(profile.id, { model: e.target.value.trim() })} />
          </div>
          <div class="card-foot">
            <span class="faint small">{profile.apps.length || profile.titles.length ? "Leave apps and titles empty to use this style everywhere else." : "Used everywhere no other style matches."}</span>
            <div class="actions">
              <button class="icon-btn" aria-label="Move up" disabled={i === 0} onclick={() => moveStyle(profile.id, -1)}><Icon name="up" size={15} /></button>
              <button class="icon-btn" aria-label="Move down" disabled={i === ai.profiles.length - 1} onclick={() => moveStyle(profile.id, 1)}><Icon name="down" size={15} /></button>
              <button class="btn ghost sm" onclick={() => saveAi((a) => (a.profiles = a.profiles.filter((p) => p.id !== profile.id)))}><Icon name="trash" size={14} /> Delete</button>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/each}
  <div><button class="btn secondary sm" onclick={addStyle}><Icon name="plus" size={15} /> Add a style</button></div>
</div>

<h3 class="section-title">Try it</h3>
<div class="group pad">
  <textarea class="field" rows="3" aria-label="Sample dictation" bind:value={sample}></textarea>
  <div class="try">
    <Select label="Style" value={testStyle} width={200} options={ai.profiles.map((p) => ({ value: p.id, label: p.name }))} onchange={(v) => (testStyle = v)} />
    <button class="btn primary sm" disabled={!sample.trim() || test.status === "running"} onclick={() => act("testAi", { profile: testStyle, text: sample })}>
      <Icon name="play" size={13} /> {test.status === "running" ? "Polishing…" : "Try"}
    </button>
    {#if test.status === "done"}<span class="faint small">{(test.millis / 1000).toFixed(1)} s</span>{/if}
  </div>
  {#if test.status === "done" || test.status === "error"}
    <p class="result selectable" class:error={test.status === "error"} transition:slide={{ duration: 200 }}>{test.text}</p>
  {/if}
</div>

<style>
  .lead {
    margin: -2px 2px 10px;
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.5;
  }
  .privacy {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 2px 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    border-radius: var(--r-lg);
    background: var(--group);
    transition: opacity var(--fast) ease;
  }
  .card.off .title,
  .card.off .meta {
    opacity: 0.55;
  }
  .card-head,
  .head-button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 52px;
    padding: 0 16px;
    text-align: left;
  }
  .card-head.as-row {
    padding-right: 0;
    gap: 4px;
  }
  .head-button {
    flex: 1;
    min-width: 0;
    padding-left: 8px;
  }
  .title {
    font-weight: 540;
    font-size: 14px;
    white-space: nowrap;
  }
  .meta {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-head .chip + .meta {
    margin-left: 2px;
  }
  .chev {
    display: grid;
    color: var(--faint);
    transition: transform 220ms var(--ease);
  }
  .chev.turned {
    transform: rotate(180deg);
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 16px 14px;
  }
  .field-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field-row.top {
    align-items: flex-start;
  }
  .field-row.top > span {
    padding-top: 8px;
  }
  .field-row > span:first-child {
    width: 96px;
    flex-shrink: 0;
    font-size: 13px;
    color: var(--muted);
  }
  .saved {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--green-text, var(--green));
  }
  .consent {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--brand-soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .error {
    color: var(--red);
    font-size: 12.5px;
    margin-left: 106px;
  }
  .ok {
    color: var(--muted);
    font-size: 12.5px;
    margin-left: 106px;
  }
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .add-provider {
    display: flex;
  }
  .pad {
    padding: 14px 16px;
  }
  .try {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
  }
  .result {
    margin-top: 12px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface);
    font-size: 14px;
    line-height: 1.55;
    white-space: pre-wrap;
  }
  .result.error {
    color: var(--red);
    font-size: 13px;
    margin-left: 0;
  }
</style>
