<script>
  import { fly, fade } from "svelte/transition";
  import { app, level, act, normalizeLevel, windowControl, windowVisible, saveSettings, onboardingStep } from "../lib/api.js";
  import Mark from "../lib/Mark.svelte";
  import Icon from "../lib/Icon.svelte";
  import Select from "../lib/Select.svelte";
  import BrandLogo from "../lib/BrandLogo.svelte";
  import ShortcutRecorder from "../lib/ShortcutRecorder.svelte";

  let s = $derived($app);
  const steps = ["welcome", "model", "microphone", "shortcut", "done"];
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

  let voice = $derived(normalizeLevel($level));
  // A gentle speaking rhythm for the welcome screen.
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
  let selected = $derived(s.models.find((m) => m.id === choice));
  // A resting model wakes up when dictation starts.
  let ready = $derived(selected?.installed && (s.phase === "ready" || s.phase === "sleeping"));
  let microphones = $derived([{ value: "", label: "Windows default" }, ...s.microphones.map((m) => ({ value: m, label: m }))]);
  const choices = ["parakeet-v3", "whisper-turbo"]; // first-run pair: the recommended model and the multilingual one
  let options = $derived(choices.map((id) => s.models.find((m) => m.id === id)).filter(Boolean));
  // A failed download of the chosen model, or an engine error once nothing is downloading.
  let problem = $derived(s.downloadError?.id === choice ? s.downloadError.text : s.phase === "error" && !s.downloading ? s.detail : "");
</script>

<div class="onboarding">
  <div class="aurora" aria-hidden="true"><span></span><span></span><span></span></div>

  <header class="bar" data-tauri-drag-region>
    <div class="progress" data-tauri-drag-region role="progressbar" aria-valuemin="1" aria-valuemax={steps.length} aria-valuenow={step + 1} aria-label="Setup progress">
      {#each steps as _, i}
        <span class="dot" class:active={i === step} class:done={i < step}></span>
      {/each}
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
      <section class="step" in:fly={{ x: 48 * direction, duration: 460, delay: 120, opacity: 0 }} out:fly={{ x: -36 * direction, duration: 220, opacity: 0 }}>
        {#if steps[step] === "welcome"}
          <div class="hero">
            <div class="halo"><Mark size={150} mood="listening" level={demo} /></div>
          </div>
          <h1>Say it.<br />Vorto writes it.</h1>
          <p class="lead">Speak into any app. Your voice never leaves this device.</p>
        {:else if steps[step] === "model"}
          <h1>Choose a voice model</h1>
          <p class="lead">You can switch any time.</p>
          <div class="choices">
            {#each options as model, i (model.id)}
              <button class="choice" class:picked={choice === model.id} aria-pressed={choice === model.id} onclick={() => (choice = model.id)} in:fly={{ y: 16, duration: 420, delay: 220 + i * 90 }}>
                <span class="logo"><BrandLogo family={model.family} size={28} /></span>
                <span class="choice-text">
                  <span class="choice-title">{model.name}{#if model.recommended}<span class="chip rec">Recommended</span>{/if}</span>
                  <span class="choice-body">{model.languages} · {model.hardware}</span>
                  <span class="choice-meta">{model.installed ? "Downloaded" : `${model.downloadMb} MB`}</span>
                </span>
                <span class="radio" aria-hidden="true"><Icon name="check" size={14} stroke={3} /></span>
              </button>
            {/each}
          </div>
        {:else if steps[step] === "microphone"}
          <div class="hero small">
            <div class="halo" style="--v:{voice}"><Mark size={120} mood="listening" level={voice} /></div>
          </div>
          <h1>Say something</h1>
          <p class="lead">The quotes move when Vorto hears you.</p>
          <div class="mic">
            <Select label="Microphone" value={s.settings.microphone} options={microphones} width={320} onchange={(v) => saveSettings({ microphone: v })} />
          </div>
        {:else if steps[step] === "shortcut"}
          <h1>Your shortcut</h1>
          <p class="lead">{s.settings.toggle ? "Press it, speak, then press it again." : "Hold it, speak, and let go."}</p>
          <div class="shortcut-card">
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
              <div class="try" class:ok={tried} class:fail={!tried && !s.hookOk} in:fade={{ delay: 300 }}>
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
        {:else}
          <div class="hero">
            <div class="halo"><Mark size={140} mood="happy" /></div>
          </div>
          <h1>All set</h1>
          <ul class="tips">
            <li in:fly={{ y: 12, delay: 250, duration: 380 }}><kbd class="kbd">Esc</kbd> <span>Discards a dictation while you speak.</span></li>
            <li in:fly={{ y: 12, delay: 330, duration: 380 }}><Icon name="app" size={18} /> <span>Closing the window keeps Vorto ready in the tray.</span></li>
          </ul>
        {/if}
      </section>
    {/key}
  </main>

  <footer>
    {#if step > 0}
      <button class="btn ghost lg" onclick={() => go(step - 1)} transition:fade={{ duration: 150 }}>Back</button>
    {:else}
      <span></span>
    {/if}
    <button class="btn primary lg next" onclick={next}>
      {#if steps[step] === "welcome"}Get started{:else if steps[step] === "model" && !selected?.installed && s.downloading !== choice}Download{:else if steps[step] === "shortcut" && !tried}I'll try later{:else if steps[step] === "done"}Done{:else}Continue{/if}
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
    animation: enter 520ms var(--ease);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: scale(1.02);
    }
  }
  .aurora {
    position: absolute;
    inset: -20%;
    pointer-events: none;
    filter: blur(70px);
    opacity: 0.55;
  }
  .aurora span {
    position: absolute;
    width: 46%;
    height: 46%;
    border-radius: 999px;
  }
  .aurora span:nth-child(1) {
    left: 8%;
    top: 4%;
    background: #ffb199;
    animation: drift-a 18s ease-in-out infinite;
  }
  .aurora span:nth-child(2) {
    right: 4%;
    top: 16%;
    background: #ffd6a8;
    animation: drift-b 22s ease-in-out infinite;
  }
  .aurora span:nth-child(3) {
    left: 30%;
    bottom: 0;
    background: #ffc2d1;
    animation: drift-a 26s ease-in-out infinite reverse;
  }
  @media (prefers-color-scheme: dark) {
    .aurora {
      opacity: 0.16;
    }
  }
  @keyframes drift-a {
    50% {
      transform: translate(12%, 10%) scale(1.15);
    }
  }
  @keyframes drift-b {
    50% {
      transform: translate(-14%, 8%) scale(0.9);
    }
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
    gap: 6px;
    flex: 1;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--border-strong);
    transition:
      width 360ms var(--ease),
      background 240ms ease;
  }
  .dot.done {
    background: var(--text);
  }
  .dot.active {
    width: 26px;
    background: var(--brand);
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
  }
  .hero {
    margin-bottom: 28px;
  }
  .hero.small {
    margin-bottom: 18px;
  }
  .halo {
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
  h1 {
    margin-top: 8px;
    font-size: 38px;
    line-height: 1.1;
    letter-spacing: -0.03em;
  }
  .lead {
    max-width: 440px;
    margin-top: 12px;
    font-size: 16px;
    line-height: 1.55;
    color: var(--muted);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(520px, 100%);
    margin-top: 28px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
    border-radius: 18px;
    background: var(--surface);
    border: 1.5px solid var(--border);
    text-align: left;
    box-shadow: var(--shadow-card);
    transition:
      border-color 200ms ease,
      box-shadow 240ms ease,
      transform 240ms var(--ease);
  }
  .choice:hover {
    transform: translateY(-2px);
  }
  .choice.picked {
    border-color: var(--brand);
    box-shadow: 0 0 0 4px var(--brand-soft);
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
  .radio {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    border-radius: 999px;
    border: 1.5px solid var(--border-strong);
    color: transparent;
    transition: all 200ms ease;
  }
  .picked .radio {
    background: var(--brand);
    border-color: var(--brand);
    color: #fff;
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
    background: var(--surface);
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
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
  }
  footer {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 28px 26px;
  }
  .next {
    min-width: 170px;
  }
</style>
