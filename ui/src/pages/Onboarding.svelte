<script>
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { app, level, act, route, normalizeLevel, windowControl, windowVisible, saveSettings, currentSettings, onboardingStep } from "../lib/api.js";
  import Mark from "../lib/Mark.svelte";
  import Icon from "../lib/Icon.svelte";
  import Select from "../lib/Select.svelte";
  import BrandLogo from "../lib/BrandLogo.svelte";
  import ProviderLogo from "../lib/ProviderLogo.svelte";
  import ShortcutRecorder from "../lib/ShortcutRecorder.svelte";
  import ProviderDialog from "../lib/ProviderDialog.svelte";
  import Aura from "../lib/Aura.svelte";
  import Meter from "../lib/Meter.svelte";

  let s = $derived($app);
  const steps = ["welcome", "model", "microphone", "shortcut", "ai", "done"];
  const titles = ["Welcome", "Voice model", "Microphone", "Shortcut", "AI editing", "Done"];
  let step = $state(0);
  let direction = $state(1);
  // Picked once when onboarding opens: the installed model, else the recommended one.
  const initialChoice = () => s.models.find((m) => m.installed)?.id ?? s.models.find((m) => m.recommended)?.id;
  let choice = $state(initialChoice());
  let tried = $state(false);
  const dictations = () => s.dictations;
  let dictationsAtStart = dictations();

  // Toast skips news that the open step already shows.
  $effect(() => {
    onboardingStep.set(steps[step]);
  });
  $effect(() => () => onboardingStep.set(null));

  function go(to) {
    direction = to > step ? 1 : -1;
    step = Math.max(0, Math.min(steps.length - 1, to));
  }
  function next() {
    if (steps[step] === "model") {
      const model = s.models.find((m) => m.id === choice);
      if (model?.installed) act("useModel", { id: choice });
      else if (s.downloading !== choice) act("download", { id: choice });
    }
    if (steps[step] === "ai") applyAi();
    if (steps[step] === "done") return finish();
    go(step + 1);
  }
  function finish() {
    act("testMicrophone", { on: false });
    saveSettings({ onboarded: true });
  }

  // Keep the microphone level alive while its step is open and the window is on screen.
  // Closing to the tray stops the renewals, and the backend ends the test on its own.
  $effect(() => {
    if (steps[step] !== "microphone") return;
    let active = true;
    const renew = async () => {
      if ((await windowVisible()) && active) act("testMicrophone", { on: true });
    };
    act("testMicrophone", { on: true });
    const timer = setInterval(renew, 9000);
    window.addEventListener("focus", renew);
    return () => {
      active = false;
      clearInterval(timer);
      window.removeEventListener("focus", renew);
      act("testMicrophone", { on: false });
    };
  });

  // Celebrate the first real dictation on the shortcut step.
  $effect(() => {
    if (steps[step] === "shortcut" && dictations() > dictationsAtStart) tried = true;
  });
  function retry() {
    if (s.downloadError?.id === choice || !selected?.installed) act("download", { id: choice });
    else act("retry");
  }

  // ---------------------------------------------------------------- AI editing, optional
  let aiChoice = $state("later"); // later, local or online
  let local = $derived(s.localAi?.find((l) => l.models.length));
  let localModel = $state("");
  // Look for Ollama and LM Studio while this step is open, so one that starts now shows up.
  $effect(() => {
    if (steps[step] !== "ai" || local) return;
    act("findLocal");
    const timer = setInterval(() => act("findLocal"), 3000);
    return () => clearInterval(timer);
  });
  // An online provider, set up right here in the provider dialog.
  let adding = $state(false);
  let online = $state(""); // the new provider's id
  let onlineProvider = $derived(s.settings.ai.providers.find((p) => p.id === online));
  function chooseOnline() {
    aiChoice = "online";
    if (!onlineProvider) adding = true;
  }
  function addedOnline(id) {
    online = id;
    aiChoice = "online";
  }
  // Nothing was connected: back to "Not now".
  $effect(() => {
    if (!adding && aiChoice === "online" && !onlineProvider) aiChoice = "later";
  });
  $effect(() => {
    // The first model Ollama or LM Studio has, once they're found; the bigger qwen when there.
    if (local && !localModel) localModel = local.models.find((m) => /3b|7b|8b/.test(m.id))?.id ?? local.models[0].id;
  });
  // Connects the chosen model as the default and turns on Clean up for every app.
  function applyAi() {
    if (aiChoice === "later" || (aiChoice === "local" && !local) || (aiChoice === "online" && !onlineProvider)) return;
    const ai = structuredClone(currentSettings().ai);
    if (aiChoice === "local" && !ai.providers.some((p) => p.id === local.id)) {
      ai.providers.unshift({ id: local.id, name: local.name, kind: "openai", base_url: local.baseUrl, model: localModel, allow_remote: false });
    }
    if (aiChoice === "online") {
      const i = ai.providers.findIndex((p) => p.id === online);
      if (i > 0) ai.providers.unshift(...ai.providers.splice(i, 1));
    }
    ai.enabled = true;
    for (const p of ai.profiles) if (p.id === "clean") (p.enabled = true), (p.everywhere = true);
    saveSettings({ ai });
  }

  let voice = $derived(normalizeLevel($level));
  // A gentle speaking rhythm for the welcome screen.
  let demo = $state(0);
  $effect(() => {
    if (steps[step] !== "welcome" && steps[step] !== "ai") return;
    let t = 0;
    const timer = setInterval(() => {
      t += 0.2;
      demo = Math.max(0, Math.sin(t) * 0.4 + Math.sin(t * 2.3) * 0.3 + 0.2);
    }, 60);
    return () => clearInterval(timer);
  });
  let selected = $derived(s.models.find((m) => m.id === choice));
  // A resting model wakes up when dictation starts.
  let ready = $derived(selected?.installed && (s.phase === "ready" || s.phase === "sleeping"));
  let microphones = $derived([{ value: "", label: "Windows default" }, ...s.microphones.map((m) => ({ value: m, label: m }))]);
  const choices = ["parakeet-v3", "whisper-turbo"]; // first-run pair: the recommended model and the multilingual one
  let options = $derived(choices.map((id) => s.models.find((m) => m.id === id)).filter(Boolean));
  // A failed download of the chosen model, or an engine error once nothing is downloading.
  let problem = $derived(s.downloadError?.id === choice ? s.downloadError.text : s.phase === "error" && !s.downloading ? s.detail : "");

  // ---------------------------------------------------------------- motion
  // The step moves in from the side it's coming from, out of focus, and settles.
  function enter(node, { delay = 0 } = {}) {
    return {
      delay,
      duration: 620,
      easing: cubicOut,
      css: (t, u) => `opacity:${t}; filter:blur(${u * 14}px); transform:translate(${u * 60 * direction}px, ${u * 8}px) scale(${0.97 + t * 0.03});`,
    };
  }
  function leave(node) {
    return {
      duration: 280,
      easing: cubicOut,
      css: (t, u) => `opacity:${t}; filter:blur(${u * 10}px); transform:translate(${-u * 40 * direction}px, 0) scale(${1 - u * 0.03});`,
    };
  }
  // The background leans a little toward the pointer.
  let px = $state(0);
  let py = $state(0);
  function pointer(e) {
    px = e.clientX / window.innerWidth - 0.5;
    py = e.clientY / window.innerHeight - 0.5;
  }
  let progress = $derived(step / (steps.length - 1));
  // Confetti for the last step: fixed pieces, so it looks the same every time.
  const confetti = Array.from({ length: 26 }, (_, i) => {
    const angle = (i / 26) * Math.PI * 2 + (i % 3) * 0.2;
    const dist = 150 + ((i * 37) % 110);
    return { x: Math.cos(angle) * dist, y: Math.sin(angle) * dist * 0.75 - 40, r: (i * 53) % 360, d: (i % 5) * 40, c: i % 3 };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="onboarding" onpointermove={pointer} style="--px:{px}; --py:{py}">
  <div class="aurora" aria-hidden="true"><span></span><span></span><span></span><span></span></div>
  <div class="grain" aria-hidden="true"></div>

  <header class="bar" data-tauri-drag-region>
    <div class="progress" data-tauri-drag-region role="progressbar" aria-valuemin="1" aria-valuemax={steps.length} aria-valuenow={step + 1} aria-valuetext="{titles[step]}, step {step + 1} of {steps.length}" aria-label="Setup progress">
      <div class="track"><div class="fill" style="transform: scaleX({Math.max(0.04, progress)})"></div></div>
      <span class="step-name" data-tauri-drag-region>{#key step}<span in:fade={{ duration: 260 }}>{titles[step]}</span>{/key}</span>
    </div>
    <div class="controls">
      {#if steps[step] !== "done"}
        <button class="skip" onclick={finish}>Skip</button>
      {/if}
      <button aria-label="Minimize" onclick={() => windowControl("minimize")}><Icon name="minus" size={16} /></button>
      <button class="close" aria-label="Close" onclick={() => windowControl("close")}><Icon name="x" size={16} /></button>
    </div>
  </header>

  <main>
    {#key step}
      <section class="step" in:enter={{ delay: 180 }} out:leave>
        {#if steps[step] === "welcome"}
          <div class="hero rise" style="--d:0">
            <Aura />
            <div class="halo"><Mark size={150} mood="listening" level={demo} /></div>
          </div>
          <h1 class="rise" style="--d:1">Say it.<br /><span class="gradient">Vorto writes it.</span></h1>
          <p class="lead rise" style="--d:2">Speak into any app. Your voice never leaves this device.</p>
        {:else if steps[step] === "model"}
          <h1 class="rise" style="--d:0">Choose a voice model</h1>
          <p class="lead rise" style="--d:1">It runs on this PC. You can switch any time.</p>
          <div class="choices">
            {#each options as model, i (model.id)}
              <button class="choice rise" style="--d:{2 + i}" class:picked={choice === model.id} aria-pressed={choice === model.id} onclick={() => (choice = model.id)}>
                <span class="logo"><BrandLogo family={model.family} size={28} /></span>
                <span class="choice-text">
                  <span class="choice-title">{model.name}{#if model.recommended}<span class="chip rec">Recommended</span>{/if}</span>
                  <span class="choice-body">{model.languages} · {model.hardware}</span>
                  <span class="choice-meters">
                    <Meter label="Speed" value={model.speed} />
                    <Meter label="Accuracy" value={model.accuracy} />
                    <span class="choice-meta">{model.installed ? "Downloaded" : `${model.downloadMb} MB`}</span>
                  </span>
                </span>
                <span class="radio" aria-hidden="true"><Icon name="check" size={14} stroke={3} /></span>
              </button>
            {/each}
          </div>
        {:else if steps[step] === "microphone"}
          <div class="hero small rise" style="--d:0">
            <Aura live level={voice} small />
            <div class="halo" style="--v:{voice}"><Mark size={120} mood="listening" level={voice} /></div>
          </div>
          <h1 class="rise" style="--d:1">Say something</h1>
          <p class="lead rise" style="--d:2">The rings move when Vorto hears you.</p>
          <div class="mic rise" style="--d:3">
            <Select label="Microphone" value={s.settings.microphone} options={microphones} width={320} onchange={(v) => saveSettings({ microphone: v })} />
          </div>
        {:else if steps[step] === "shortcut"}
          <h1 class="rise" style="--d:0">Your shortcut</h1>
          <p class="lead rise" style="--d:1">{s.settings.toggle ? "Press it, speak, then press it again." : "Hold it, speak, and let go."}</p>
          <div class="shortcut-card rise" style="--d:2">
            <ShortcutRecorder />
          </div>
          {#if !s.capturing}
            {#if problem}
              <div class="problem" in:fade={{ delay: 300 }}>
                <p role="alert"><Icon name="alert" size={16} /> {problem}</p>
                <div class="problem-actions">
                  <button class="btn secondary" onclick={retry}><Icon name="refresh" size={15} /> Try again</button>
                  <button class="btn ghost" onclick={() => go(steps.indexOf("model"))}>Choose another model</button>
                </div>
              </div>
            {:else}
              <div class="try rise" style="--d:3" class:ok={tried} class:fail={!tried && !s.hookOk}>
                {#if tried}
                  <Icon name="check" size={16} stroke={2.4} /> Vorto wrote your first words.
                {:else if !s.hookOk}
                  <Icon name="alert" size={16} /> Your shortcut isn't working. Restart Vorto.
                {:else if s.downloading}
                  <span class="spinner"></span> {s.detail || "Starting the download"}
                {:else if !ready}
                  <span class="spinner"></span> {s.detail || "Getting ready"}
                {:else}
                  <span class="pulse"></span> Try it now in any text field
                {/if}
              </div>
            {/if}
          {/if}
        {:else if steps[step] === "ai"}
          <div class="hero tiny rise" style="--d:0">
            <div class="ai-orb"><Icon name="pen" size={30} /></div>
          </div>
          <h1 class="rise" style="--d:1">Polish your words with AI</h1>
          <p class="lead rise" style="--d:2">Optional. A language model can clean up what you said, write it as an email or tidy up a prompt, before Vorto inserts it.</p>
          <div class="choices">
            <!-- A div, not a button: it holds a model picker of its own. -->
            <div class="choice rise" style="--d:3" class:picked={aiChoice === "local"} class:unavailable={!local} role="radio" aria-checked={aiChoice === "local"} aria-disabled={!local} tabindex={local ? 0 : -1} onclick={() => local && (aiChoice = "local")} onkeydown={(e) => local && (e.key === "Enter" || e.key === " ") && (e.preventDefault(), (aiChoice = "local"))}>
              <span class="logo"><ProviderLogo id={local?.id ?? "ollama"} size={40} /></span>
              <span class="choice-text">
                <span class="choice-title">{local ? `${local.name} on this PC` : "A free model on this PC"}{#if local}<span class="chip rec">Found</span>{/if}</span>
                <span class="choice-body">{#if local}Private and free. Vorto cleans up every dictation.{:else}<span class="looking"><span class="spinner"></span> Looking for Ollama or LM Studio…</span>{/if}</span>
                {#if local && aiChoice === "local"}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <span class="model-pick" onclick={(e) => e.stopPropagation()} role="presentation">
                    <Select label="Model" value={localModel} options={local.models.map((m) => ({ value: m.id, label: m.id }))} width={220} onchange={(v) => (localModel = v)} />
                  </span>
                {:else if !local}
                  <span class="choice-links">
                    <span class="link" role="link" tabindex="0" onclick={(e) => (e.stopPropagation(), act("openUrl", { url: "https://ollama.com/download" }))} onkeydown={(e) => e.key === "Enter" && act("openUrl", { url: "https://ollama.com/download" })}>Get Ollama <Icon name="external" size={12} /></span>
                  </span>
                {/if}
              </span>
              <span class="radio" aria-hidden="true"><Icon name="check" size={14} stroke={3} /></span>
            </div>
            <button class="choice rise" style="--d:4" class:picked={aiChoice === "online"} aria-pressed={aiChoice === "online"} onclick={chooseOnline}>
              {#if onlineProvider}
                <span class="logo"><ProviderLogo id={onlineProvider.id} size={40} /></span>
                <span class="choice-text">
                  <span class="choice-title">{onlineProvider.name}<span class="chip rec">Connected</span></span>
                  <span class="choice-body">{onlineProvider.model || "No model chosen"} · <span class="link" role="button" tabindex="0" onclick={(e) => (e.stopPropagation(), (adding = true))} onkeydown={(e) => e.key === "Enter" && (e.stopPropagation(), (adding = true))}>Choose another</span></span>
                </span>
              {:else}
                <span class="logo logos"><ProviderLogo id="openai" size={24} /><ProviderLogo id="anthropic" size={24} /><ProviderLogo id="gemini" size={24} /></span>
                <span class="choice-text">
                  <span class="choice-title">An online provider</span>
                  <span class="choice-body">OpenAI, Anthropic, Gemini and more, with your own key. Fastest on most PCs.</span>
                </span>
              {/if}
              <span class="radio" aria-hidden="true"><Icon name="check" size={14} stroke={3} /></span>
            </button>
            <button class="choice plain rise" style="--d:5" class:picked={aiChoice === "later"} aria-pressed={aiChoice === "later"} onclick={() => (aiChoice = "later")}>
              <span class="choice-text">
                <span class="choice-title">Not now</span>
                <span class="choice-body">Vorto inserts your words as spoken. You can turn AI editing on later.</span>
              </span>
              <span class="radio" aria-hidden="true"><Icon name="check" size={14} stroke={3} /></span>
            </button>
          </div>
        {:else}
          <div class="hero rise" style="--d:0">
            <Aura />
            <div class="confetti" aria-hidden="true">
              {#each confetti as c}<i class="c{c.c}" style="--x:{c.x}px; --y:{c.y}px; --r:{c.r}deg; --dl:{c.d}ms"></i>{/each}
            </div>
            <div class="halo"><Mark size={140} mood="happy" /></div>
          </div>
          <h1 class="rise" style="--d:1">All set</h1>
          <ul class="tips">
            <li class="rise" style="--d:2"><kbd class="kbd">Esc</kbd> <span>Discards a dictation while you speak.</span></li>
            <li class="rise" style="--d:3"><Icon name="app" size={18} /> <span>Closing the window keeps Vorto ready in the tray.</span></li>
            {#if aiChoice === "local" && local}
              <li class="rise" style="--d:4"><Icon name="pen" size={18} /> <span>AI editing cleans up every dictation with {localModel}.</span></li>
            {:else if aiChoice === "online" && onlineProvider}
              <li class="rise" style="--d:4"><Icon name="pen" size={18} /> <span>AI editing cleans up every dictation with {onlineProvider.name}.</span></li>
            {/if}
          </ul>
        {/if}
      </section>
    {/key}
  </main>

  <ProviderDialog bind:open={adding} online onadded={addedOnline} />

  <footer>
    {#if step > 0}
      <button class="btn ghost lg" onclick={() => go(step - 1)} transition:fade={{ duration: 150 }}>Back</button>
    {:else}
      <span></span>
    {/if}
    <button class="btn primary lg next" onclick={next}>
      <span class="shine" aria-hidden="true"></span>
      {#if steps[step] === "welcome"}Get started{:else if steps[step] === "model" && !selected?.installed && s.downloading !== choice}Download{:else if steps[step] === "shortcut" && !tried}I'll try later{:else if steps[step] === "ai" && aiChoice === "local"}Use {local?.name ?? "it"}{:else if steps[step] === "ai" && aiChoice === "online"}Use {onlineProvider?.name ?? "it"}{:else if steps[step] === "done"}Done{:else}Continue{/if}
      <Icon name="arrow" size={17} />
    </button>
  </footer>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    overflow: hidden;
    animation: enter 700ms var(--ease);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: scale(1.03);
      filter: blur(8px);
    }
  }
  /* Four soft blobs drift slowly and lean toward the pointer. */
  .aurora {
    position: absolute;
    inset: -25%;
    pointer-events: none;
    filter: blur(80px);
    opacity: 0.6;
    transform: translate(calc(var(--px) * -40px), calc(var(--py) * -30px));
    transition: transform 900ms var(--ease);
  }
  .aurora span {
    position: absolute;
    width: 44%;
    height: 44%;
    border-radius: 999px;
    mix-blend-mode: multiply;
  }
  .aurora span:nth-child(1) {
    left: 10%;
    top: 6%;
    background: #ffb199;
    animation: drift-a 16s ease-in-out infinite;
  }
  .aurora span:nth-child(2) {
    right: 6%;
    top: 14%;
    background: #ffd6a8;
    animation: drift-b 21s ease-in-out infinite;
  }
  .aurora span:nth-child(3) {
    left: 30%;
    bottom: 2%;
    background: #ffc2d1;
    animation: drift-a 25s ease-in-out infinite reverse;
  }
  .aurora span:nth-child(4) {
    right: 24%;
    bottom: 18%;
    width: 30%;
    height: 30%;
    background: #ff8a70;
    opacity: 0.55;
    animation: drift-b 19s ease-in-out infinite reverse;
  }
  :global([data-theme="dark"]) .aurora {
    opacity: 0.22;
  }
  :global([data-theme="dark"]) .aurora span {
    mix-blend-mode: normal;
  }
  :global([data-theme="dark"]) .aurora span:nth-child(2) {
    background: #ff7a5c;
  }
  :global([data-theme="dark"]) .aurora span:nth-child(3) {
    background: #c2185b;
  }
  @keyframes drift-a {
    50% {
      transform: translate(14%, 12%) scale(1.18) rotate(20deg);
    }
  }
  @keyframes drift-b {
    50% {
      transform: translate(-16%, 9%) scale(0.88) rotate(-16deg);
    }
  }
  /* A whisper of film grain keeps the gradient from banding. */
  .grain {
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: 0.05;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='140' height='140'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  }
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    height: 48px;
    padding-left: 24px;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 14px;
    flex: 1;
  }
  .track {
    width: 140px;
    height: 5px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 10%, transparent);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, #ff8a70, var(--brand), #ff4d6d);
    background-size: 200% 100%;
    transform-origin: left;
    transition: transform 700ms var(--ease);
    animation: flow 3s linear infinite;
  }
  @keyframes flow {
    to {
      background-position: -200% 0;
    }
  }
  .step-name {
    display: grid;
    font-size: 12.5px;
    font-weight: 540;
    color: var(--muted);
  }
  .step-name > span {
    grid-area: 1 / 1;
  }
  .controls {
    display: flex;
    align-items: center;
    height: 100%;
  }
  .controls button {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    color: var(--titlebar-icon);
  }
  .controls button:hover {
    background: var(--hover);
  }
  .controls .close:hover {
    background: #e81123;
    color: #fff;
  }
  .controls .skip {
    width: auto;
    padding: 0 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
  }
  main {
    position: relative;
    flex: 1;
    display: grid;
    min-height: 0;
  }
  .step {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0 40px;
    text-align: center;
    will-change: transform, filter, opacity;
  }
  /* Parts of a step rise in one after another. */
  .rise {
    animation: rise 720ms var(--ease) both;
    animation-delay: calc(240ms + var(--d, 0) * 70ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.985);
      filter: blur(6px);
    }
  }
  .hero {
    position: relative;
    margin-bottom: 28px;
  }
  .hero.small {
    margin-bottom: 18px;
  }
  .hero.tiny {
    margin-bottom: 20px;
  }
  .halo {
    position: relative;
    display: grid;
    place-items: center;
    width: 210px;
    height: 210px;
    border-radius: 999px;
    background: radial-gradient(circle, color-mix(in srgb, var(--surface) 90%, transparent) 30%, transparent 70%);
    transform: scale(calc(1 + var(--v, 0) * 0.08));
    transition: transform 90ms linear;
  }
  .small .halo {
    width: 170px;
    height: 170px;
  }
  @keyframes float {
    50% {
      transform: translateY(-6px);
    }
  }
  .ai-orb {
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
    border-radius: 26px;
    color: #fff;
    background: linear-gradient(135deg, #ff8a70, var(--brand) 55%, #e8375a);
    box-shadow:
      0 18px 40px color-mix(in srgb, var(--brand) 35%, transparent),
      inset 0 1px 0 rgba(255, 255, 255, 0.35);
    animation: float 5s ease-in-out infinite;
  }
  h1 {
    margin-top: 8px;
    font-size: 40px;
    line-height: 1.08;
    letter-spacing: -0.035em;
  }
  .gradient {
    background: linear-gradient(90deg, var(--brand), #ff8a70, #e8375a, var(--brand));
    background-size: 300% 100%;
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    animation: flow 6s linear infinite;
  }
  .lead {
    max-width: 460px;
    margin-top: 12px;
    font-size: 16px;
    line-height: 1.55;
    color: var(--muted);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(540px, 100%);
    margin-top: 28px;
  }
  .choice {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(12px);
    border: 1.5px solid var(--border);
    text-align: left;
    box-shadow: var(--shadow-card);
    transition:
      border-color 200ms ease,
      box-shadow 260ms ease,
      transform 300ms var(--ease);
  }
  .choice {
    cursor: pointer;
  }
  .choice.unavailable {
    cursor: default;
  }
  .choice:hover:not(:disabled):not(.unavailable) {
    transform: translateY(-2px);
    box-shadow:
      var(--shadow-card),
      0 12px 28px rgba(16, 16, 20, 0.08);
  }
  .choice:disabled {
    cursor: default;
  }
  .choice.unavailable .choice-title,
  .choice.unavailable .logo {
    opacity: 0.75;
  }
  /* A card with an open picker sits above the cards after it, so its menu isn't covered. */
  .choice:focus-within,
  .choice:has(:global([aria-expanded="true"])) {
    z-index: 5;
  }
  .choice.picked {
    z-index: 2;
    border-color: var(--brand);
    box-shadow: 0 0 0 4px var(--brand-soft);
  }
  .choice.plain {
    padding-left: 22px;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    flex-shrink: 0;
    border-radius: 14px;
    background: var(--group);
  }
  .logos {
    display: flex;
    gap: 0;
    background: none;
    width: 64px;
  }
  .logos :global(.provider-logo) {
    margin-right: -8px;
    box-shadow: 0 0 0 2px var(--surface);
  }
  .choice-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  .choice-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 600;
  }
  .choice-body {
    font-size: 13.5px;
    color: var(--muted);
    line-height: 1.45;
  }
  .choice-meta {
    margin-top: 2px;
    font-size: 12px;
    color: var(--faint);
  }
  .choice-meters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 18px;
    margin-top: 8px;
  }
  .choice-meters .choice-meta {
    margin: 0 0 0 auto;
  }
  .model-pick {
    margin-top: 8px;
  }
  .choice-links {
    display: flex;
    gap: 14px;
    margin-top: 6px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    font-weight: 560;
    color: var(--brand);
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .radio {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    border-radius: 999px;
    border: 1.5px solid var(--border-strong);
    color: transparent;
    transition: all 220ms var(--ease);
  }
  .picked .radio {
    background: var(--brand);
    border-color: var(--brand);
    color: #fff;
    transform: scale(1.08);
  }
  .mic {
    margin-top: 26px;
  }
  .shortcut-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    margin-top: 28px;
    padding: 26px 34px;
    border-radius: 22px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(12px);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-card);
  }
  /* The recorder's keycaps and capture field, at hero size. */
  .shortcut-card :global(.kbd) {
    height: 46px;
    padding: 0 18px;
    font-size: 19px;
    border-radius: 11px;
    box-shadow: 0 3px 0 var(--kbd-shadow);
    animation: key-press 2.4s ease-in-out infinite;
  }
  @keyframes key-press {
    0%,
    70%,
    100% {
      transform: none;
      box-shadow: 0 3px 0 var(--kbd-shadow);
    }
    80% {
      transform: translateY(3px);
      box-shadow: 0 0 0 var(--kbd-shadow);
    }
  }
  .shortcut-card :global(.capture) {
    height: auto;
    min-height: 58px;
  }
  .try {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 22px;
    font-size: 14px;
    color: var(--muted);
  }
  .try.ok {
    color: var(--green-text);
  }
  .try.fail {
    color: var(--red);
  }
  .problem {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 480px;
    margin-top: 22px;
  }
  .problem p {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 14px;
    line-height: 1.45;
    color: var(--red);
    text-align: left;
  }
  .problem p :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
  }
  .problem-actions {
    display: flex;
    gap: 8px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 999px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--brand);
    animation: spin 800ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--green);
    box-shadow: 0 0 0 4px var(--green-soft);
    animation: beat 1.6s ease-in-out infinite;
  }
  @keyframes beat {
    50% {
      box-shadow: 0 0 0 8px transparent;
    }
  }
  /* A burst of small pieces from behind the mark, once. */
  .confetti {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
  }
  .confetti i {
    grid-area: 1 / 1;
    width: 8px;
    height: 12px;
    border-radius: 2px;
    opacity: 0;
    animation: burst 1.4s cubic-bezier(0.15, 0.7, 0.3, 1) both;
    animation-delay: calc(420ms + var(--dl));
  }
  .confetti .c0 {
    background: var(--brand);
  }
  .confetti .c1 {
    background: #ffc15e;
  }
  .confetti .c2 {
    background: var(--text);
    width: 6px;
    height: 6px;
    border-radius: 999px;
  }
  @keyframes burst {
    0% {
      opacity: 1;
      transform: translate(0, 0) rotate(0) scale(0.4);
    }
    70% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(var(--x), calc(var(--y) + 60px)) rotate(var(--r)) scale(1);
    }
  }
  .tips {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 28px 0 0;
    padding: 0;
    list-style: none;
    text-align: left;
  }
  .tips li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(12px);
    border: 1px solid var(--border);
    color: var(--text);
  }
  .looking {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  footer {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 28px 26px;
  }
  .next {
    position: relative;
    min-width: 180px;
    overflow: hidden;
    transition:
      transform 200ms var(--ease),
      box-shadow 260ms ease;
  }
  .next:hover {
    transform: translateY(-1px);
    box-shadow: 0 10px 24px rgba(16, 16, 20, 0.18);
  }
  /* A light sweep across the button every few seconds. */
  .shine {
    position: absolute;
    inset: 0;
    background: linear-gradient(110deg, transparent 30%, rgba(255, 255, 255, 0.28) 50%, transparent 70%);
    transform: translateX(-120%);
    animation: sweep 4.5s ease-in-out infinite;
    animation-delay: 1.2s;
    pointer-events: none;
  }
  @keyframes sweep {
    0%,
    60% {
      transform: translateX(-120%);
    }
    85%,
    100% {
      transform: translateX(120%);
    }
  }
</style>
