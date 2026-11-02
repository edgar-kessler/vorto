<script>
  import { onMount } from "svelte";
  import { scale } from "svelte/transition";
  import { app, route, act, connect, windowControl, startsHidden, isTauri, applyTheme } from "./lib/api.js";
  import Icon from "./lib/Icon.svelte";
  import Mark from "./lib/Mark.svelte";
  import Toast from "./lib/Toast.svelte";
  import Home from "./pages/Home.svelte";
  import Models from "./pages/Models.svelte";
  import History from "./pages/History.svelte";
  import Settings from "./pages/Settings.svelte";
  import Onboarding from "./pages/Onboarding.svelte";
  import Dictionary from "./pages/Dictionary.svelte";
  import Ai from "./pages/Ai.svelte";
  import Stats from "./pages/Stats.svelte";

  const nav = [
    { id: "home", label: "Dictate" },
    { id: "history", label: "History" },
    { id: "stats", label: "Stats" },
    { id: "dictionary", label: "Dictionary" },
    { id: "ai", label: "AI", name: "AI editing" },
    { id: "models", label: "Models", name: "Voice models" },
    { id: "settings", label: "Settings" },
  ];

  let ready = $state(false);
  let scroller;

  onMount(async () => {
    await connect();
    const hidden = await startsHidden();
    // Show the window only once the first frame is ready: no white flash.
    requestAnimationFrame(() => {
      ready = true;
      if (!hidden) windowControl("show");
    });
  });

  $effect(() => {
    $route;
    scroller?.scrollTo({ top: 0 });
  });

  let s = $derived($app);
  $effect(() => {
    if (s?.settings?.theme) applyTheme(s.settings.theme);
  });
  // Dictation can't work. The Dictate page says why; elsewhere a dot points there.
  let attention = $derived(!!s && $route !== "home" && (s.phase === "error" || (!s.hookOk && $route !== "settings")));

</script>

<div class="shell" class:ready class:browser={!isTauri} inert={!!s && !s.settings.onboarded}>
  <header class="top" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <Mark size={18} />
      <span>Vorto</span>
    </div>
    <nav>
      {#each nav as item}
        <button
          class="nav-item"
          class:active={$route === item.id}
          aria-label={item.id === "home" && attention ? "Dictate, needs attention" : (item.name ?? item.label)}
          aria-current={$route === item.id ? "page" : undefined}
          onclick={() => route.set(item.id)}
        >
          {item.label}
          {#if item.id === "home" && attention}
            <span class="badge" aria-hidden="true" in:scale={{ start: 0.4, duration: 160 }}></span>
          {/if}
        </button>
      {/each}
    </nav>
    <div class="grow" data-tauri-drag-region></div>
    {#if s?.update?.status === "ready"}
      <button
        class="btn secondary sm update"
        aria-label="Restart to update to version {s.update.version}"
        onclick={() => act("installUpdate")}
      >
        <Icon name="download" size={14} /> Restart to update
      </button>
    {/if}
    <div class="controls">
      <button aria-label="Minimize" onclick={() => windowControl("minimize")}><Icon name="minus" size={15} /></button>
      <button aria-label="Maximize" onclick={() => windowControl("maximize")}><Icon name="square" size={13} /></button>
      <button class="close" aria-label="Close" onclick={() => windowControl("close")}><Icon name="x" size={15} /></button>
    </div>
  </header>

  <main class="scroll" bind:this={scroller}>
    {#if s}
      {#key $route}
        <div class="page">
          {#if $route === "home"}<Home />
          {:else if $route === "models"}<Models />
          {:else if $route === "history"}<History />
          {:else if $route === "dictionary"}<Dictionary />
          {:else if $route === "ai"}<Ai />
          {:else if $route === "stats"}<Stats />
          {:else}<Settings />{/if}
        </div>
      {/key}
    {/if}
  </main>
</div>
{#if s && !s.settings.onboarded}
  <Onboarding />
{/if}
<Toast centered={!!s && !s.settings.onboarded} />

<style>
  .shell {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .shell.ready {
    opacity: 1;
  }
  .top {
    display: flex;
    align-items: center;
    height: 44px;
    flex-shrink: 0;
    padding-left: 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-right: 24px;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .brand :global(*) {
    pointer-events: none;
  }
  nav {
    display: flex;
    align-self: stretch;
    gap: 16px;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 2px;
    color: var(--nav-text);
    font-size: 13px;
    font-weight: 500;
  }
  .nav-item:hover {
    color: var(--text);
  }
  .nav-item.active {
    color: var(--text);
    box-shadow: inset 0 -1.5px 0 var(--text);
  }
  .badge {
    width: 6px;
    height: 6px;
    flex-shrink: 0;
    border-radius: 999px;
    background: var(--red);
  }
  .grow {
    flex: 1;
    align-self: stretch;
  }
  .update {
    margin-right: 8px;
  }
  .controls {
    display: flex;
    height: 100%;
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
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    /* The scrollbar coming and going between pages doesn't shift the content sideways. */
    scrollbar-gutter: stable both-edges;
    overflow-x: hidden;
  }
  /* The page hangs between two hairlines that run the height of the window. */
  .page {
    position: relative;
    min-height: 100%;
    max-width: 880px;
    margin: 0 auto;
    padding: 40px 48px 64px;
    border-inline: 1px solid var(--grid-line);
  }
</style>
