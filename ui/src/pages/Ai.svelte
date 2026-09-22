<script>
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { slide, fade, scale } from "svelte/transition";
  import { app, act, saveSettings, currentSettings } from "../lib/api.js";
  import Toggle from "../lib/Toggle.svelte";
  import Select from "../lib/Select.svelte";
  import SearchSelect from "../lib/SearchSelect.svelte";
  import Icon from "../lib/Icon.svelte";
  import AppIcon from "../lib/AppIcon.svelte";
  import ProviderLogo from "../lib/ProviderLogo.svelte";
  import ProviderDialog from "../lib/ProviderDialog.svelte";

  let s = $derived($app);
  let ai = $derived(s.settings.ai);

  const timeouts = [10, 20, 30, 60, 120].map((v) => ({ value: v, label: `${v} seconds` }));

  function saveAi(change) {
    const current = structuredClone(currentSettings().ai);
    change(current);
    saveSettings({ ai: current });
  }
  const setProvider = (id, patch) => saveAi((a) => Object.assign(a.providers.find((p) => p.id === id) ?? {}, patch));
  const setStyle = (id, patch) => saveAi((a) => Object.assign(a.profiles.find((p) => p.id === id) ?? {}, patch));
  function unique(base, taken) {
    let id = base;
    for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
    return id;
  }
  const localOf = (provider) => s.providersLocal[ai.providers.findIndex((p) => p.id === provider?.id)] ?? true;
  const modelsOf = (id) => s.aiModels[id]?.models ?? [];
  // A provider model as a picker option: its readable name, with context size and price.
  const modelOption = (m) => ({ value: m.id, label: m.name || m.id, hint: m.hint || (m.name ? m.id : "") });

  // Model lists for every provider, and whether Ollama or LM Studio run here.
  onMount(() => {
    act("findLocal");
    for (const p of get(app).settings.ai.providers) if (!get(app).aiModels[p.id]) act("listModels", { provider: p.id });
  });

  // ---------------------------------------------------------------- adding a provider
  let adding = $state(false);
  function useLocal(found) {
    const id = unique(found.id, currentSettings().ai.providers.map((p) => p.id));
    saveAi((a) => a.providers.push({ id, name: found.name, kind: "openai", base_url: found.baseUrl, model: found.models[0]?.id ?? "", allow_remote: false }));
    setTimeout(() => act("listModels", { provider: id }), 150);
  }
  // Until a provider is set up, look for Ollama and LM Studio every few seconds, so one that
  // starts later shows up without a reload.
  $effect(() => {
    if (ai.providers.length) return;
    const timer = setInterval(() => act("findLocal"), 3000);
    return () => clearInterval(timer);
  });

  // ---------------------------------------------------------------- one provider's details
  let editing = $state("");
  let keyDraft = $state("");
  function saveKey(id) {
    act("setApiKey", { provider: id, key: keyDraft.trim() });
    keyDraft = "";
    setTimeout(() => act("listModels", { provider: id }), 300);
  }
  function removeProvider(id) {
    editing = "";
    saveAi((a) => {
      a.providers = a.providers.filter((p) => p.id !== id);
      for (const p of a.profiles) if (p.provider === id) (p.provider = ""), (p.model = "");
    });
  }

  // ---------------------------------------------------------------- presets
  let open = $state("");
  const icons = { email: "mail", chat: "message", prompt: "bot", notes: "list", formal: "file", clean: "wand" };
  const presetOf = (id) => s.presets?.find((p) => p.id === id) ?? { name: id, about: "", instructions: "" };
  const appOf = (exe) => s.apps.find((a) => a.exe === exe);
  const appName = (exe) => appOf(exe)?.name ?? exe.replace(/\.exe$/i, "");
  // Apps a preset can still add: the ones you dictated into, minus those it has.
  const appOptions = (style) =>
    s.apps.filter((a) => !style.apps.includes(a.exe)).map((a) => ({ value: a.exe, label: a.name, hint: a.exe, icon: a.icon, app: a.name }));
  function where(style) {
    const parts = [...style.apps.map(appName), ...style.titles.map((t) => `“${t}”`)];
    if (style.everywhere) parts.push(parts.length ? "all other apps" : "All apps");
    return parts.join(", ");
  }
  function turn(style, on) {
    setStyle(style.id, { enabled: on });
    if (on) open = style.id;
  }
  // Only one preset covers the apps no other preset is for.
  function setEverywhere(style, on) {
    saveAi((a) => {
      for (const p of a.profiles) p.everywhere = on ? p.id === style.id : p.id === style.id ? false : p.everywhere;
    });
  }
  let defaultProvider = $derived(ai.providers[0]);
  // One list for a preset's model: the default, or any model of any provider.
  let modelChoices = $derived([
    { value: "", label: "Default model", hint: defaultProvider ? `${defaultProvider.model || "none chosen"} · ${defaultProvider.name}` : "" },
    ...ai.providers.flatMap((p) => modelsOf(p.id).map((m) => ({ value: `${p.id}\n${m.id}`, label: m.name || m.id, hint: p.name }))),
  ]);
  const modelValue = (style) => (style.provider && style.model ? `${style.provider}\n${style.model}` : "");
  function setModel(style, value) {
    const [provider, model] = value ? value.split("\n") : ["", ""];
    setStyle(style.id, { provider, model });
  }
  let websiteDraft = $state({});
  function addWebsite(style) {
    const title = (websiteDraft[style.id] ?? "").trim();
    if (!title) return;
    setStyle(style.id, { titles: [...style.titles.filter((t) => t.toLowerCase() !== title.toLowerCase()), title] });
    websiteDraft[style.id] = "";
  }

  // ---------------------------------------------------------------- try it
  let sample = $state("");
  let testStyle = $state("");
  let test = $derived(s.aiTest);
  let tryStyle = $derived(ai.profiles.find((p) => p.id === testStyle) ?? ai.profiles[0]);
  let ready = $derived(ai.providers.some((p) => p.model));
  let cloud = $derived([...new Set(ai.providers.filter((p) => !localOf(p)).map((p) => p.name))]);
</script>

<header class="page-head">
  <h1>AI editing</h1>
  <Toggle label="AI editing" checked={ai.enabled} onchange={(v) => saveAi((a) => (a.enabled = v))} />
</header>
<p class="lead">
  A language model rewrites what you said before Vorto inserts it, the way you describe for each app. Press <kbd class="kbd">Esc</kbd> while the pill says Polishing to insert your words as spoken.
</p>

<!-- ------------------------------------------------------------------ models -->
<h3 class="section-title">Models</h3>
{#if ai.providers.length}
  <p class="section-lead">Presets use the default model unless you choose another one for them.</p>
{/if}
{#if !ai.providers.length}
  <div class="empty-card" in:fade>
    <div class="empty-icon"><Icon name="pen" size={22} /></div>
    <div class="empty-text">
      <strong>Connect a language model</strong>
      <span>A free model on this PC keeps everything private. An online provider is faster on most PCs.</span>
    </div>
    {#each s.localAi ?? [] as found (found.id)}
      <div class="found" transition:slide={{ duration: 200 }}>
        <ProviderLogo id={found.id} size={36} />
        <span class="found-text"><strong>{found.name} is running on this PC</strong><span>{found.models.length} {found.models.length === 1 ? "model" : "models"} installed</span></span>
        <button class="btn primary sm" disabled={!found.models.length} onclick={() => useLocal(found)}>Use {found.name}</button>
      </div>
    {/each}
    <button class="btn secondary" onclick={() => (adding = true)}><Icon name="plus" size={15} /> Choose a provider</button>
  </div>
{:else}
  <div class="list">
    {#each ai.providers as provider, i (provider.id)}
      {@const list = s.aiModels[provider.id]}
      {@const local = localOf(provider)}
      <div class="provider" class:open={editing === provider.id}>
        <div class="provider-row">
          <ProviderLogo id={provider.id} size={36} />
          <span class="provider-name">
            <strong>{provider.name}{#if i === 0}<span class="default-chip">Default</span>{/if}</strong>
            <span>{local ? "On this PC" : "Online"}{list?.models?.length ? ` · ${list.models.length} models` : ""}</span>
          </span>
          <SearchSelect
            label="Model for {provider.name}"
            value={provider.model}
            placeholder="Choose a model"
            search="Search, or type a model ID"
            empty={list?.status === "error" ? list.error : "No models found"}
            custom
            loading={list?.status === "loading"}
            width={250}
            options={modelsOf(provider.id).map(modelOption)}
            onopen={() => list?.status !== "loading" && act("listModels", { provider: provider.id })}
            onchange={(m) => setProvider(provider.id, { model: m })}
          />
          <button class="icon-btn" aria-label="Settings for {provider.name}" aria-expanded={editing === provider.id} onclick={() => ((editing = editing === provider.id ? "" : provider.id), (keyDraft = ""))}><Icon name="more" size={18} stroke={2.4} /></button>
        </div>
        {#if list?.status === "error" && editing !== provider.id}
          <p class="problem" transition:slide={{ duration: 180 }}>{list.error}</p>
        {/if}
        {#if editing === provider.id}
          <div class="details" transition:slide={{ duration: 220 }}>
            {#if !local && !provider.allow_remote}
              <div class="consent">
                <span>Vorto sends the text of your dictations to {provider.name}. Your voice stays on this device.</span>
                <button class="btn primary sm" onclick={() => setProvider(provider.id, { allow_remote: true })}>Allow</button>
              </div>
            {/if}
            <label class="field-row"><span>Name</span><input class="field" value={provider.name} onchange={(e) => setProvider(provider.id, { name: e.target.value.trim() || provider.name })} /></label>
            <div class="field-row">
              <span>API key</span>
              {#if s.apiKeys[provider.id]}
                <span class="saved"><Icon name="check" size={14} stroke={2.2} /> Saved securely in Windows</span>
                <button class="btn ghost sm" onclick={() => act("setApiKey", { provider: provider.id, key: "" })}>Remove</button>
              {:else}
                <input class="field" type="password" placeholder={local ? "Not needed" : "Paste your key"} bind:value={keyDraft} onkeydown={(e) => e.key === "Enter" && keyDraft.trim() && saveKey(provider.id)} />
                <button class="btn secondary sm" disabled={!keyDraft.trim()} onclick={() => saveKey(provider.id)}>Save</button>
              {/if}
            </div>
            <label class="field-row"><span>Address</span><input class="field mono" value={provider.base_url} onchange={(e) => setProvider(provider.id, { base_url: e.target.value, allow_remote: false })} /></label>
            <div class="details-foot">
              {#if i > 0}<button class="btn ghost sm" onclick={() => saveAi((a) => a.providers.unshift(...a.providers.splice(i, 1)))}>Make default</button>{/if}
              <span class="grow"></span>
              <button class="btn ghost sm danger-text" onclick={() => removeProvider(provider.id)}><Icon name="trash" size={14} /> Remove</button>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
  <button class="btn ghost sm add-link" onclick={() => (adding = true)}><Icon name="plus" size={15} /> Add a provider</button>
  {#if cloud.length && ai.enabled}
    <p class="privacy"><Icon name="globe" size={14} /> The text you dictate can go to {cloud.join(" and ")}. Your voice stays on this device.</p>
  {/if}
{/if}

<!-- ------------------------------------------------------------------ presets -->
<h3 class="section-title">Presets</h3>
<p class="section-lead">Turn on what you need and choose the apps for each. When several fit, the one higher up wins.</p>
<div class="presets">
  {#each ai.profiles as style (style.id)}
    {@const preset = presetOf(style.id)}
    {@const place = where(style)}
    <div class="preset" class:on={style.enabled} class:open={open === style.id}>
      <div class="preset-row">
        <span class="preset-icon"><Icon name={icons[style.id] ?? "pen"} size={18} /></span>
        <button class="preset-head" aria-expanded={open === style.id} onclick={() => (open = open === style.id ? "" : style.id)}>
          <strong>{preset.name}</strong>
          <span class:warn={style.enabled && !place}>{style.enabled ? place || "Choose where to use it" : preset.about}</span>
        </button>
        {#if style.enabled && style.apps.length}
          <span class="icons" aria-hidden="true">
            {#each style.apps.slice(0, 4) as exe (exe)}<AppIcon name={appName(exe)} src={appOf(exe)?.icon} size={22} />{/each}
          </span>
        {/if}
        <Toggle label="Use {preset.name}" checked={style.enabled} onchange={(v) => turn(style, v)} />
      </div>
      {#if open === style.id}
        <div class="details" transition:slide={{ duration: 240 }}>
          <p class="does"><Icon name={icons[style.id] ?? "pen"} size={14} /> {preset.instructions}</p>

          <div class="block">
            <span class="block-title">Where</span>
            <div class="chips">
              {#each style.apps as exe (exe)}
                <span class="chip-app" in:scale={{ start: 0.85, duration: 160 }}>
                  <AppIcon name={appName(exe)} src={appOf(exe)?.icon} size={18} />
                  {appName(exe)}
                  <button aria-label="Remove {appName(exe)}" onclick={() => setStyle(style.id, { apps: style.apps.filter((a) => a !== exe) })}><Icon name="x" size={12} stroke={2.2} /></button>
                </span>
              {/each}
              {#each style.titles as title (title)}
                <span class="chip-app" in:scale={{ start: 0.85, duration: 160 }}>
                  <Icon name="globe" size={14} />
                  {title}
                  <button aria-label="Remove {title}" onclick={() => setStyle(style.id, { titles: style.titles.filter((t) => t !== title) })}><Icon name="x" size={12} stroke={2.2} /></button>
                </span>
              {/each}
              <SearchSelect
                label="Add an app"
                search="Search your apps"
                empty={s.apps.length ? "No other apps yet" : "Dictate into an app once, and it shows up here"}
                width={150}
                options={appOptions(style)}
                onchange={(exe) => setStyle(style.id, { apps: [...style.apps, exe], enabled: true })}
              >
                {#snippet trigger()}<span class="add-chip"><Icon name="plus" size={14} /> Add an app</span>{/snippet}
              </SearchSelect>
            </div>
            <form
              class="website"
              onsubmit={(e) => {
                e.preventDefault();
                addWebsite(style);
              }}
            >
              <input class="field" placeholder="Website, for example Gmail (matches the browser tab's title)" bind:value={websiteDraft[style.id]} />
              <button class="btn secondary sm" disabled={!websiteDraft[style.id]?.trim()}>Add</button>
            </form>
            <label class="check">
              <Toggle label="Use in all other apps" checked={style.everywhere} onchange={(v) => setEverywhere(style, v)} />
              <span>Also use it in every app no other preset is for</span>
            </label>
          </div>

          <div class="block">
            <span class="block-title">Your additions</span>
            <textarea class="field" rows="2" placeholder={style.id === "email" ? "Optional. For example: Always use Sie." : "Optional. For example: Always use British spelling."} value={style.prompt} onchange={(e) => setStyle(style.id, { prompt: e.target.value })}></textarea>
          </div>

          <div class="block">
            <span class="block-title">Model</span>
            <SearchSelect label="Model for {preset.name}" value={modelValue(style)} search="Search, or type a model ID" width={340} options={modelChoices} onchange={(v) => setModel(style, v)} />
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<!-- ------------------------------------------------------------------ try it -->
{#if ready}
  <h3 class="section-title">Try it</h3>
  <div class="try-card">
    <textarea class="field" rows="3" aria-label="Something to say" placeholder="Type what you might say, filler words and all." bind:value={sample}></textarea>
    <div class="try-row">
      <Select label="Preset" value={tryStyle?.id} width={200} options={ai.profiles.map((p) => ({ value: p.id, label: presetOf(p.id).name }))} onchange={(v) => (testStyle = v)} />
      <button class="btn primary sm" disabled={!sample.trim() || test.status === "running"} onclick={() => act("testAi", { profile: tryStyle.id, text: sample })}>
        {test.status === "running" ? "Polishing…" : "Try"}
      </button>
      {#if test.status === "done"}<span class="faint small" in:fade>{(test.millis / 1000).toFixed(1)} s</span>{/if}
    </div>
    {#if test.status === "done" || test.status === "error"}
      <p class="result selectable" class:error={test.status === "error"} transition:slide={{ duration: 200 }}>{test.text}</p>
    {/if}
  </div>
{/if}

<h3 class="section-title">When the model is slow</h3>
<div class="group">
  <div class="row">
    <div class="label">
      <strong>Wait at most</strong>
      <span>Then Vorto inserts your words as spoken.</span>
    </div>
    <div class="control"><Select label="Wait at most" value={ai.timeout_secs} options={timeouts} width={150} onchange={(v) => saveAi((a) => (a.timeout_secs = v))} /></div>
  </div>
</div>

<!-- ------------------------------------------------------------------ add a provider -->
<ProviderDialog bind:open={adding} />

<style>
  .section-lead {
    margin: -2px 2px 10px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .default-chip {
    display: inline-flex;
    align-items: center;
    height: 19px;
    margin-left: 8px;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--brand-soft);
    color: var(--brand);
    font-size: 11px;
    font-weight: 620;
    vertical-align: 1px;
  }
  .presets {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .preset {
    border-radius: var(--r-lg);
    background: var(--group);
    transition:
      background var(--fast) ease,
      box-shadow 240ms ease;
  }
  .preset.open {
    background: var(--surface);
    box-shadow:
      var(--shadow-card),
      0 0 0 1px var(--border);
  }
  .preset-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 62px;
    padding: 0 16px 0 14px;
  }
  .preset-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 10px;
    background: var(--surface);
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--border);
    transition:
      background 240ms ease,
      color 240ms ease;
  }
  .preset.on .preset-icon {
    background: var(--brand-soft);
    color: var(--brand);
    box-shadow: none;
  }
  .preset-head {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-self: stretch;
    justify-content: center;
    text-align: left;
  }
  .preset-head strong {
    font-weight: 560;
    font-size: 14px;
  }
  .preset-head span {
    font-size: 12.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preset-head span.warn {
    color: var(--brand);
  }
  .does {
    display: flex;
    gap: 9px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--group);
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--muted);
  }
  .does :global(svg) {
    flex-shrink: 0;
    margin-top: 3px;
  }
  .lead {
    margin: -12px 2px 24px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--muted);
  }
  .lead .kbd {
    height: 20px;
    padding: 0 6px;
    font-size: 11.5px;
  }
  .list {
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: var(--r-lg);
    background: var(--group);
  }
  .provider,
  .style {
    border-radius: 12px;
    transition: background var(--fast) ease;
  }
  .provider.open,
  .style.open {
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }
  .provider + .provider,
  .style + .style {
    margin-top: 2px;
  }
  .provider-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 58px;
    padding: 8px 8px 8px 12px;
  }
  .provider-name,
  .style-text,
  .entry-text,
  .found-text,
  .empty-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .provider-name strong,
  .style-text strong,
  .entry-text strong,
  .found-text strong,
  .empty-text strong {
    font-weight: 540;
    font-size: 14px;
  }
  .provider-name span,
  .style-text span,
  .entry-text span,
  .found-text span,
  .empty-text span {
    font-size: 12.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty-text span {
    white-space: normal;
    line-height: 1.5;
    margin-top: 2px;
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 4px 16px 14px;
  }
  .field-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field-row > span:first-child {
    width: 72px;
    flex-shrink: 0;
    font-size: 13px;
    color: var(--muted);
  }
  .mono {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
  }
  .saved {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--green);
  }
  .details-foot {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .grow {
    flex: 1;
  }
  .danger-text:hover {
    color: var(--red);
  }
  .consent {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--brand-soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .consent span {
    flex: 1;
  }
  .problem {
    color: var(--red);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .provider > .problem {
    padding: 0 16px 12px 56px;
  }
  .add-link {
    margin: 8px 0 0 4px;
  }
  .privacy {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 8px 6px 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
    padding: 20px;
    border-radius: var(--r-lg);
    background: var(--group);
  }
  .empty-card.small {
    flex-direction: row;
    align-items: center;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 13px;
    background: var(--surface);
    color: var(--brand);
    box-shadow: var(--shadow-card);
  }
  .found {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 10px 10px 12px;
    border-radius: 12px;
    background: var(--surface);
    box-shadow: var(--shadow-card);
  }
  .style.off .style-text,
  .style.off .icons {
    opacity: 0.5;
  }
  .style-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-left: 12px;
  }
  .style-head {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 58px;
    padding: 8px 14px 8px 8px;
    text-align: left;
  }
  .icons {
    display: flex;
    align-items: center;
  }
  .icons > :global(*) {
    margin-right: -5px;
    box-shadow: 0 0 0 2px var(--group);
    border-radius: 6px;
  }
  .style.open .icons > :global(*) {
    box-shadow: 0 0 0 2px var(--surface);
  }
  .icons:not(:empty) {
    margin-right: 8px;
  }
  .everywhere {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--control);
    color: var(--muted);
  }
  .chev {
    display: grid;
    color: var(--faint);
    transition: transform 220ms var(--ease);
  }
  .chev.turned {
    transform: rotate(180deg);
  }
  .title-field {
    height: 38px;
    font-size: 15px;
    font-weight: 540;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .block-title {
    font-size: 12.5px;
    font-weight: 540;
    color: var(--muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip-app {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 4px 0 6px;
    border-radius: 999px;
    background: var(--group);
    font-size: 13px;
  }
  .chip-app :global(svg) {
    color: var(--muted);
  }
  .chip-app button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 999px;
    color: var(--faint);
  }
  .chip-app button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .add-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 12px 0 9px;
    border-radius: 999px;
    border: 1px dashed var(--border-strong);
    font-size: 13px;
    color: var(--muted);
    white-space: nowrap;
  }
  .add-chip:hover {
    color: var(--text);
    border-color: var(--muted);
  }
  .website {
    display: flex;
    gap: 8px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--muted);
  }
  .try-card {
    padding: 14px;
    border-radius: var(--r-lg);
    background: var(--group);
  }
  .try-row {
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
  }
  .icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
</style>
