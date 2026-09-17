<script>
  import { onMount } from "svelte";
  import { fly, scale } from "svelte/transition";
  import { app, route, act, connect, windowControl, startsHidden, isTauri } from "./lib/api.js";
  import Icon from "./lib/Icon.svelte";
  import Mark from "./lib/Mark.svelte";
  import Toast from "./lib/Toast.svelte";
  import Home from "./pages/Home.svelte";
  import Models from "./pages/Models.svelte";
  import History from "./pages/History.svelte";
  import Settings from "./pages/Settings.svelte";
  import Onboarding from "./pages/Onboarding.svelte";

  const nav = [
    { id: "home", label: "Dictate", icon: "dictate" },
    { id: "history", label: "History", icon: "history" },
    { id: "models", label: "Voice models", icon: "models" },
    { id: "settings", label: "Settings", icon: "settings" },
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
  // Dictation can't work. The Dictate page says why; elsewhere a dot points there.
  let attention = $derived(!!s && $route !== "home" && (s.phase === "error" || (!s.hookOk && $route !== "settings")));

  // The collapsed sidebar hides the labels, so the icons get a tooltip.
  const narrow = matchMedia("(max-width: 960px)");
  let collapsed = $state(narrow.matches);
  $effect(() => {
    const update = () => (collapsed = narrow.matches);
    narrow.addEventListener("change", update);
    return () => narrow.removeEventListener("change", update);
  });
</script>

<div class="shell" class:ready class:browser={!isTauri} inert={!!s && !s.settings.onboarded}>
  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region>
      <Mark size={26} />
      <span class="collapsible">Vorto</span>
    </div>
    <nav>
      {#each nav as item}
        <button
          class="nav-item"
          class:active={$route === item.id}
          aria-label={item.id === "home" && attention ? "Dictate, needs attention" : item.label}
          title={collapsed ? item.label : undefined}
          aria-current={$route === item.id ? "page" : undefined}
          onclick={() => route.set(item.id)}
        >
          <Icon name={item.icon} size={17} />
          <span class="collapsible">{item.label}</span>
          {#if item.id === "home" && attention}
            <span class="badge" aria-hidden="true" in:scale={{ start: 0.4, duration: 260 }}></span>
          {/if}
        </button>
      {/each}
    </nav>
    <div class="grow" data-tauri-drag-region></div>
    {#if s?.update?.status === "ready"}
      <button
        class="nav-item update"
        aria-label="Restart to update to version {s.update.version}"
        title={collapsed ? "Restart to update" : undefined}
        onclick={() => act("installUpdate")}
        in:scale={{ start: 0.9, duration: 260 }}
      >
        <Icon name="download" size={17} />
        <span class="collapsible">Restart to update</span>
      </button>
    {/if}
  </aside>

  <main>
    <header class="titlebar" data-tauri-drag-region>
      <div class="controls">
        <button aria-label="Minimize" onclick={() => windowControl("minimize")}><Icon name="minus" size={16} /></button>
        <button aria-label="Maximize" onclick={() => windowControl("maximize")}><Icon name="square" size={14} /></button>
        <button class="close" aria-label="Close" onclick={() => windowControl("close")}><Icon name="x" size={16} /></button>
      </div>
    </header>
    <div class="scroll" bind:this={scroller}>
      {#if s}
        {#key $route}
          <div class="page" in:fly={{ y: 10, duration: 280, delay: 40 }}>
            {#if $route === "home"}<Home />
            {:else if $route === "models"}<Models />
            {:else if $route === "history"}<History />
            {:else}<Settings />{/if}
          </div>
        {/key}
      {/if}
    </div>
  </main>
</div>
{#if s && !s.settings.onboarded}
  <Onboarding />
{/if}
<Toast centered={!!s && !s.settings.onboarded} />

<style>
  .shell {
    display: flex;
    height: 100%;
    opacity: 0;
    transform: scale(0.992);
    transition:
      opacity 320ms ease,
      transform 420ms var(--ease);
  }
  .shell.ready {
    opacity: 1;
    transform: none;
  }
  .shell.browser {
    border: 1px solid var(--border);
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    width: var(--sidebar-w);
    flex-shrink: 0;
    padding: 10px 12px 12px;
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    transition:
      width 260ms var(--ease),
      padding 260ms var(--ease);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 44px;
    padding: 0 8px;
    margin-bottom: 14px;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .brand :global(*) {
    pointer-events: none;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    height: 38px;
    padding: 0 11px;
    border-radius: 10px;
    color: var(--nav-text);
    font-size: 14px;
    font-weight: 450;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      padding 260ms var(--ease);
  }
  .nav-item :global(svg) {
    color: var(--muted);
    transition: color var(--fast) ease;
  }
  .nav-item:hover {
    background: var(--hover);
  }
  .nav-item.active {
    background: var(--nav-active);
    color: var(--text);
    font-weight: 560;
  }
  .nav-item.active :global(svg) {
    color: var(--text);
  }
  .badge {
    width: 7px;
    height: 7px;
    margin-left: auto;
    flex-shrink: 0;
    border-radius: 999px;
    background: var(--red);
  }
  .grow {
    flex: 1;
  }
  .nav-item.update,
  .nav-item.update :global(svg) {
    color: var(--brand);
    font-weight: 560;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .titlebar {
    display: flex;
    align-items: center;
    height: 44px;
    flex-shrink: 0;
  }
  .controls {
    display: flex;
    height: 100%;
    margin-left: auto;
  }
  .controls button {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    color: var(--titlebar-icon);
    transition:
      background 120ms ease,
      color 120ms ease;
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
    overflow-y: auto;
    overflow-x: hidden;
  }
  .page {
    max-width: 800px;
    margin: 0 auto;
    padding: 12px 36px 56px;
  }
  @media (max-width: 960px) {
    .sidebar {
      padding-left: 10px;
      padding-right: 10px;
    }
    .collapsible {
      display: none;
    }
    .brand,
    .nav-item {
      justify-content: center;
      padding-left: 0;
      padding-right: 0;
    }
    .badge {
      position: absolute;
      top: 9px;
      right: 15px;
      margin: 0;
    }
    .page {
      padding-left: 24px;
      padding-right: 24px;
    }
  }
</style>
