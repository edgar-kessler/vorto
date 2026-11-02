<script>
  import { fade, fly } from "svelte/transition";
  import { app, level, act, route, normalizeLevel, windowControl, windowVisible, saveSettings, currentSettings, onboardingStep } from "../lib/api.js";
  import { languageOptions, bestFor, spokenLanguage, rememberLanguage } from "../lib/languages.js";
  import Mark from "../lib/Mark.svelte";
  import Icon from "../lib/Icon.svelte";
  import Keys from "../lib/Keys.svelte";
  import Select from "../lib/Select.svelte";
  import BrandLogo from "../lib/BrandLogo.svelte";
  import ProviderLogo from "../lib/ProviderLogo.svelte";
  import ShortcutRecorder from "../lib/ShortcutRecorder.svelte";
  import ProviderDialog from "../lib/ProviderDialog.svelte";

  let s = $derived($app);
  const steps = ["welcome", "model", "microphone", "shortcut", "ai", "done"];
  let step = $state(0);
  let direction = $state(1);
  let name = $derived(steps[step]);

  // What the user speaks picks the model, until they pick one themselves.
  let speak = $state(spokenLanguage());
  let picked = $state(null);
  let more = $state(false);
  $effect(() => rememberLanguage(speak));
  let best = $derived(bestFor(speak));
  // Already downloaded and in use wins over a suggestion: nothing to wait for.
  let choice = $derived(picked ?? best.id);
  let selected = $derived(s.models.find((m) => m.id === choice));
  let recommended = $derived(s.models.find((m) => m.id === best.id));
  let others = $derived(s.models.filter((m) => m.id !== best.id));

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
    if (name === "model") {
      if (selected?.installed) act("useModel", { id: choice });
      else if (s.downloading !== choice) act("download", { id: choice });
    }
    if (name === "ai") applyAi();
    if (name === "done") return finish();
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

  // The first real dictation lands on the sheet of the shortcut step.
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
  // A speaking rhythm for the welcome screen, so the mark is alive before there's a voice.
  let demo = $state(0);
  $effect(() => {
    if (steps[step] !== "welcome") return;
    let t = 0;
    const timer = setInterval(() => {
      t += 0.2;
      demo = Math.max(0, Math.sin(t) * 0.4 + Math.sin(t * 2.3) * 0.3 + 0.2);
    }, 60);
    return () => clearInterval(timer);
  });
  // A resting model wakes up when dictation starts.
  let ready = $derived(selected?.installed && (s.phase === "ready" || s.phase === "sleeping"));
  let microphones = $derived([{ value: "", label: "Windows default" }, ...s.microphones.map((m) => ({ value: m, label: m }))]);
  // A failed download of the chosen model, or an engine error once nothing is downloading.
  let problem = $derived(s.downloadError?.id === choice ? s.downloadError.text : s.phase === "error" && !s.downloading ? s.detail : "");
  const maker = (m) => (m.id === "parakeet-de" ? "primeLine" : m.id === "parakeet-ultra" ? "Moondream" : m.family === "parakeet" ? "NVIDIA" : "OpenAI");
  const size = (m) => (m.installed ? "Downloaded" : `${m.downloadMb} MB`);
  // Percent of the download, while the chosen model is the one coming in.
  let downloading = $derived(s.downloading === choice && s.progress >= 0 ? Math.round(s.progress * 100) : -1);
  let nextLabel = $derived(
    name === "welcome" ? "Get started" : name === "model" && !selected?.installed && s.downloading !== choice ? "Download" : name === "done" ? "Start dictating" : "Continue",
  );
</script>

<div class="onboarding">
  <header class="bar" data-tauri-drag-region>
    <div class="progress" role="progressbar" aria-label="Setup" aria-valuemin="1" aria-valuemax={steps.length} aria-valuenow={step + 1}>
      {#each steps as _, i}<i class:on={i <= step}></i>{/each}
    </div>
    <div class="controls">
      {#if name !== "done"}<button class="skip" onclick={finish}>Skip</button>{/if}
      <button aria-label="Minimize" onclick={() => windowControl("minimize")}><Icon name="minus" size={15} /></button>
      <button class="close" aria-label="Close" onclick={() => windowControl("close")}><Icon name="x" size={15} /></button>
    </div>
  </header>

  <main>
    {#key step}
      <section class="step" in:fly={{ x: 22 * direction, duration: 300, delay: 90 }} out:fade={{ duration: 90 }}>
        {#if name === "welcome"}
          <div class="mark rise" style="--d:0"><Mark size={84} mood="listening" level={demo} /></div>
          <h1 class="big rise" style="--d:1">Say it.<br />Vorto writes it.</h1>
          <p class="lead rise" style="--d:2">In any app. It learns how you talk, and your voice never leaves this PC.</p>
        {:else if name === "model"}
          <h1 class="rise" style="--d:0">What do you mostly speak?</h1>
          <div class="rise" style="--d:1">
            <Select label="What you mostly speak" bind:value={speak} options={languageOptions} width={260} onchange={() => (picked = null)} />
          </div>
          {#if recommended}
            <button class="model best rise" style="--d:2" class:on={choice === recommended.id} aria-pressed={choice === recommended.id} onclick={() => (picked = null)}>
              <span class="logo"><BrandLogo family={recommended.family} size={18} /></span>
              <span class="model-text">
                <strong>{recommended.name}<span class="chip">Best for you</span></strong>
                <span>{best.why}</span>
              </span>
              <span class="model-size">{size(recommended)}</span>
            </button>
          {/if}
          {#if more}
            <div class="others" in:fade={{ duration: 160 }}>
              {#each others as model (model.id)}
                <button class="model" class:on={choice === model.id} aria-pressed={choice === model.id} onclick={() => (picked = model.id)}>
                  <span class="logo"><BrandLogo family={model.family} size={18} /></span>
                  <span class="model-text">
                    <strong>{model.name}</strong>
                    <span>{maker(model)} · {model.languages}</span>
                  </span>
                  <span class="model-size">{size(model)}</span>
                </button>
              {/each}
            </div>
          {:else}
            <button class="btn ghost sm rise" style="--d:3" onclick={() => (more = true)}>Other models</button>
          {/if}
        {:else if name === "microphone"}
          <div class="mark rise" style="--d:0"><Mark size={84} mood="listening" level={voice} /></div>
          <h1 class="rise" style="--d:1">Say something</h1>
          <div class="level rise" style="--d:2" aria-hidden="true"><i style="transform: scaleX({Math.max(0.02, voice)})"></i></div>
          <p class="lead rise" style="--d:2">The line moves when Vorto hears you.</p>
          <div class="rise" style="--d:3">
            <Select label="Microphone" value={s.settings.microphone} options={microphones} width={320} onchange={(v) => saveSettings({ microphone: v })} />
          </div>
        {:else if name === "shortcut"}
          <h1 class="rise" style="--d:0">{s.settings.toggle ? "Press, speak, press again" : "Hold, speak, let go"}</h1>
          <div class="rise" style="--d:1"><ShortcutRecorder /></div>
          {#if !s.capturing}
            <div class="sheet rise" style="--d:2">
              <i class="fold"></i>
              {#if tried && s.latest}
                <p class="words" in:fade={{ duration: 200 }}>{s.latest}</p>
              {:else}
                <p class="placeholder">Your first words land here. In any other app, they land there.</p>
              {/if}
            </div>
            {#if problem}
              <div class="problem" in:fade={{ duration: 160 }}>
                <p role="alert"><Icon name="alert" size={15} /> {problem}</p>
                <div class="problem-actions">
                  <button class="btn secondary sm" onclick={retry}><Icon name="refresh" size={14} /> Try again</button>
                  <button class="btn ghost sm" onclick={() => go(steps.indexOf("model"))}>Choose another model</button>
                </div>
              </div>
            {:else}
              <p class="status rise" style="--d:3" class:ok={tried} class:fail={!tried && !s.hookOk}>
                {#if tried}
                  <Icon name="check" size={15} stroke={2.4} /> That's it. It works everywhere like this.
                {:else if !s.hookOk}
                  <Icon name="alert" size={15} /> Your shortcut isn't working. Restart Vorto.
                {:else if s.downloading}
                  <span class="spinner"></span> Downloading your voice model{downloading >= 0 ? ` · ${downloading} %` : ""}
                {:else if !ready}
                  <span class="spinner"></span> {s.detail || "Getting ready"}
                {:else}
                  <span class="dot"></span> Ready. Try it now.
                {/if}
              </p>
            {/if}
          {/if}
        {:else if name === "ai"}
          <h1 class="rise" style="--d:0">Polish your words with AI?</h1>
          <p class="lead rise" style="--d:1">Optional. A language model fixes what you said before Vorto inserts it.</p>
          <div class="choices rise" style="--d:2">
            <button class="model" class:on={aiChoice === "later"} aria-pressed={aiChoice === "later"} onclick={() => (aiChoice = "later")}>
              <span class="model-text">
                <strong>Not now</strong>
                <span>Your words go in as spoken. You can turn this on later.</span>
              </span>
            </button>
            <!-- A div, not a button: it holds a model picker of its own. -->
            <div class="model" class:on={aiChoice === "local"} class:off={!local} role="button" tabindex={local ? 0 : -1} aria-pressed={aiChoice === "local"} onclick={() => local && (aiChoice = "local")} onkeydown={(e) => local && e.key === "Enter" && (aiChoice = "local")}>
              <span class="logo"><ProviderLogo id={local?.id ?? "ollama"} size={18} /></span>
              <span class="model-text">
                <strong>{local ? `${local.name} on this PC` : "A free model on this PC"}</strong>
                <span>
                  {#if local}Private and free.{:else}<span class="spinner small"></span> Looking for Ollama or LM Studio.
                    <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
                    <span class="link" onclick={(e) => (e.stopPropagation(), act("openUrl", { url: "https://ollama.com/download" }))}>Get Ollama</span>{/if}
                </span>
              </span>
              {#if local && aiChoice === "local"}
                <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                <span onclick={(e) => e.stopPropagation()}>
                  <Select label="Model" value={localModel} options={local.models.map((m) => ({ value: m.id, label: m.id }))} width={180} onchange={(v) => (localModel = v)} />
                </span>
              {/if}
            </div>
            <button class="model" class:on={aiChoice === "online"} aria-pressed={aiChoice === "online"} onclick={chooseOnline}>
              <span class="logo"><ProviderLogo id={onlineProvider?.id ?? "openai"} size={18} /></span>
              <span class="model-text">
                <strong>{onlineProvider ? onlineProvider.name : "An online provider"}</strong>
                <span>{onlineProvider ? onlineProvider.model || "No model chosen" : "OpenAI, Anthropic, Gemini and more, with your own key."}</span>
              </span>
            </button>
          </div>
        {:else}
          <div class="mark rise" style="--d:0"><Mark size={84} mood="happy" /></div>
          <h1 class="big rise" style="--d:1">You're ready.</h1>
          <p class="lead rise" style="--d:2">{s.settings.toggle ? "Press" : "Hold"} <Keys keys={s.shortcut} /> in any app and speak. Fix a word once, and Vorto remembers it.</p>
          <p class="note rise" style="--d:3">Closing the window keeps Vorto ready in the tray.</p>
        {/if}
      </section>
    {/key}
  </main>

  <ProviderDialog bind:open={adding} online onadded={addedOnline} />

  <footer>
    {#if step > 0}
      <button class="btn ghost" onclick={() => go(step - 1)} in:fade={{ duration: 120 }}>Back</button>
    {:else}
      <span></span>
    {/if}
    <button class="btn primary lg" onclick={next}>{nextLabel} <Icon name="arrow" size={15} /></button>
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
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 44px;
    flex-shrink: 0;
    padding-left: 20px;
  }
  .progress {
    display: flex;
    gap: 4px;
    width: 180px;
  }
  .progress i {
    flex: 1;
    height: 2px;
    border-radius: 2px;
    background: var(--bar-bg);
    transition: background 240ms ease;
  }
  .progress i.on {
    background: var(--text);
  }
  .controls {
    display: flex;
    align-items: center;
    height: 100%;
    margin-left: auto;
  }
  .controls button {
    display: grid;
    place-items: center;
    width: 44px;
    height: 100%;
    color: var(--titlebar-icon);
  }
  .controls button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .controls .close:hover {
    background: #e81123;
    color: #fff;
  }
  .controls .skip {
    width: auto;
    padding: 0 12px;
    font-weight: 500;
    color: var(--muted);
  }
  main {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  /* The old and the new step share one place, and the column stays put. */
  .step {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 14px;
    position: absolute;
    inset: 0;
    width: min(560px, 100% - 48px);
    margin: 0 auto;
    padding: 16px 0;
  }
  .rise {
    animation: rise 460ms cubic-bezier(0.16, 1, 0.3, 1) both;
    animation-delay: calc(90ms + var(--d) * 55ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
  }
  .mark {
    margin-bottom: 14px;
  }
  h1 {
    font-size: 32px;
    letter-spacing: -0.04em;
    line-height: 1.08;
  }
  h1.big {
    font-size: 52px;
    letter-spacing: -0.05em;
    line-height: 1;
  }
  .lead {
    max-width: 460px;
    color: var(--muted);
    font-size: 15px;
    line-height: 1.5;
  }
  .note {
    color: var(--faint);
  }
  .choices,
  .others {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .others {
    max-height: 230px;
    overflow-y: auto;
  }
  .model {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 60px;
    padding: 10px 14px;
    border-radius: 8px;
    text-align: left;
    box-shadow: inset 0 0 0 1px var(--border);
    cursor: pointer;
  }
  .model:hover {
    background: var(--row-hover);
  }
  .model.on {
    box-shadow: inset 0 0 0 1.5px var(--text);
  }
  .model.off {
    cursor: default;
    background: none;
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .logo {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: 7px;
    background: var(--control);
    box-shadow: var(--control-shadow);
  }
  .model-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .model-text strong {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 560;
  }
  .model-text > span {
    color: var(--muted);
  }
  .model-size {
    flex-shrink: 0;
    color: var(--faint);
  }
  .link {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .level {
    width: 100%;
    max-width: 360px;
    height: 3px;
    border-radius: 3px;
    background: var(--bar-bg);
    overflow: hidden;
  }
  .level i {
    display: block;
    height: 100%;
    background: var(--text);
    transform-origin: left;
    transition: transform 70ms linear;
  }
  /* The same sheet as on the website: paper in both themes, folded at the corner. */
  .sheet {
    position: relative;
    width: 100%;
    min-height: 118px;
    padding: 16px 44px 16px 18px;
    color: #111111;
    background: #ffffff;
    border-radius: 6px;
    clip-path: polygon(0 0, calc(100% - 26px) 0, 100% 26px, 100% 100%, 0 100%);
    font-size: 16px;
    line-height: 1.45;
    letter-spacing: -0.012em;
  }
  :global(:root[data-theme="light"]) .sheet {
    background: #f4f4f4;
  }
  .fold {
    position: absolute;
    top: 0;
    right: 0;
    width: 26px;
    height: 26px;
    background: #c4c4c4;
    clip-path: polygon(0 0, 100% 100%, 0 100%);
  }
  .placeholder {
    color: #8a8a8a;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }
  .status.ok {
    color: var(--green-text);
  }
  .status.fail {
    color: var(--red-text);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--green);
    animation: breathe 1.6s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      opacity: 0.35;
    }
  }
  .spinner {
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 1.5px solid var(--bar-bg);
    border-top-color: var(--text);
    border-radius: 999px;
    animation: turn 800ms linear infinite;
  }
  .spinner.small {
    width: 10px;
    height: 10px;
    vertical-align: -1px;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  .problem p {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--red-text);
  }
  .problem-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: min(560px, 100% - 48px);
    flex-shrink: 0;
    margin: 0 auto;
    padding: 12px 0 26px;
  }
  @media (prefers-reduced-motion: reduce) {
    .dot,
    .rise {
      animation: none;
    }
  }
</style>
