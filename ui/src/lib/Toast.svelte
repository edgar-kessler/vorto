<script>
  import { app, route, onboardingStep } from "./api.js";
  import Icon from "./Icon.svelte";
  import { fly } from "svelte/transition";
  // Centered above the footer while onboarding covers the sidebar.
  let { centered = false } = $props();
  let visible = $state(null);
  // The startup warning arrives as notice 0.
  let last = -1;
  let timer;
  const icons = { error: "alert", info: "info", success: "check" };
  $effect(() => {
    const notice = $app?.notice;
    if (!notice || notice.id === last || !notice.text) return;
    last = notice.id;
    // Skip news the current screen already shows.
    const s = $app;
    const where = $onboardingStep ? `onboarding:${$onboardingStep}` : $route;
    const noModel = !s.models.find((m) => m.id === s.settings.model)?.installed;
    const shownOn =
      {
        engine: ["home", "onboarding:shortcut"],
        download: ["models", "onboarding:shortcut", ...(noModel ? ["home"] : [])],
        downloaded: ["home", "models", "onboarding:shortcut"],
      }[notice.topic] ?? [];
    if (shownOn.includes(where)) return;
    visible = { ...notice };
    clearTimeout(timer);
    timer = setTimeout(() => (visible = null), notice.kind === "error" ? 6000 : 3200);
  });
</script>

{#if visible}
  {#key visible.id}
    <div class="toast {visible.kind}" class:centered in:fly={{ y: 12, duration: 260 }} out:fly={{ y: 8, duration: 180 }}>
      <span class="icon"><Icon name={icons[visible.kind] ?? "check"} size={13} stroke={2.4} /></span>
      <span class="text">{visible.text}</span>
      <button class="close" onclick={() => (visible = null)} aria-label="Dismiss"><Icon name="x" size={14} /></button>
    </div>
  {/key}
{/if}

<style>
  .toast {
    position: fixed;
    left: calc(50% + var(--sidebar-w) / 2);
    bottom: 22px;
    translate: -50% 0;
    z-index: 300;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(560px, calc(100vw - var(--sidebar-w) - 48px));
    padding: 8px 8px 8px 10px;
    border-radius: 14px;
    background: var(--toast);
    border: 1px solid var(--toast-border);
    color: #fff;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
    font-size: 13.5px;
  }
  .toast.centered {
    left: 50%;
    bottom: 96px;
    max-width: min(560px, calc(100vw - 48px));
  }
  .icon {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 999px;
    background: var(--green);
    flex-shrink: 0;
  }
  .error .icon {
    background: var(--red);
  }
  .info .icon {
    background: #3a3a40;
  }
  .text {
    flex: 1;
    line-height: 1.35;
  }
  .close {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: #9d9da5;
  }
  .close:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
</style>
