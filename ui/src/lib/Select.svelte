<script>
  import Icon from "./Icon.svelte";
  import { fly } from "svelte/transition";
  // `onopen` runs each time the menu opens, e.g. to refresh the options.
  let { value = $bindable(), options, onchange, onopen, disabled = false, width = 210, placeholder = "", label, align = "right" } = $props();
  let open = $state(false);
  let root;
  let trigger;
  let current = $derived(options.find((o) => o.value === value));

  $effect(() => {
    if (!open) return;
    const close = (e) => {
      if (!root?.contains(e.target)) open = false;
    };
    const key = (e) => {
      if (e.key === "Escape") {
        open = false;
        trigger?.focus();
      }
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", key);
    };
  });
  function pick(option) {
    value = option.value;
    open = false;
    trigger?.focus();
    onchange?.(option.value);
  }
</script>

<div class="select" bind:this={root} style="width:{width}px">
  <button class="trigger" class:open {disabled} bind:this={trigger} aria-label={label} aria-haspopup="listbox" aria-expanded={open} onclick={() => {
      open = !open;
      if (open) onopen?.();
    }}>
    <span class="value">{current?.label ?? placeholder}</span>
    <Icon name="chevron" size={15} />
  </button>
  {#if open}
    <div class="menu" class:left={align === "left"} role="listbox" aria-label={label} transition:fly={{ y: -4, duration: 160 }}>
      {#each options as option}
        <button class="item" class:selected={option.value === value} role="option" aria-selected={option.value === value} onclick={() => pick(option)}>
          <span>{option.label}</span>
          {#if option.value === value}<Icon name="check" size={15} stroke={2} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 34px;
    padding: 0 10px 0 12px;
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    font-size: 13.5px;
    transition:
      border-color 140ms ease,
      box-shadow 140ms ease;
  }
  .trigger:hover {
    border-color: var(--border-hover);
  }
  .trigger.open {
    border-color: var(--border-focus);
    box-shadow: 0 0 0 3px var(--hover);
  }
  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .trigger :global(svg) {
    color: var(--muted);
    flex-shrink: 0;
    transition: transform 200ms ease;
  }
  .trigger.open :global(svg) {
    transform: rotate(180deg);
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .menu {
    position: absolute;
    z-index: 50;
    top: calc(100% + 6px);
    right: 0;
    min-width: 100%;
    padding: 5px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-pop);
  }
  .menu.left {
    right: auto;
    left: 0;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    min-height: 32px;
    padding: 6px 10px;
    border-radius: 8px;
    text-align: left;
    font-size: 13.5px;
    white-space: nowrap;
  }
  .item:hover {
    background: var(--hover);
  }
  .item.selected {
    font-weight: 550;
  }
</style>
