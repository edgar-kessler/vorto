<script>
  // A picker with a search field, for long lists such as a provider's models or your apps.
  // Options are { value, label, hint?, icon?, app? }; `app` shows the program icon (or its
  // letter tile). With `custom`, what you type can be used as is.
  import Icon from "./Icon.svelte";
  import AppIcon from "./AppIcon.svelte";
  import { fly } from "svelte/transition";

  let {
    value = "",
    options = [],
    onchange,
    onopen,
    placeholder = "Choose…",
    search = "Search",
    empty = "Nothing found",
    custom = false,
    loading = false,
    width = 260,
    align = "left",
    label,
    trigger,
    // Always open, as a list inside the page or a dialog, without a button.
    inline = false,
  } = $props();

  let open = $state(inline);
  let query = $state("");
  let active = $state(0);
  let root;
  let button;
  let current = $derived(options.find((o) => o.value === value));
  let shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const found = q ? options.filter((o) => `${o.label} ${o.hint ?? ""} ${o.value}`.toLowerCase().includes(q)) : options;
    if (custom && q && !options.some((o) => o.value.toLowerCase() === q)) {
      return [...found, { value: query.trim(), label: `Use “${query.trim()}”`, typed: true }];
    }
    return found;
  });

  function toggle() {
    open = !open;
    if (open) {
      query = "";
      active = Math.max(0, options.findIndex((o) => o.value === value));
      onopen?.();
    }
  }
  function pick(option) {
    open = inline;
    button?.focus();
    onchange?.(option.value);
  }
  function key(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(shown.length - 1, active + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(0, active - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (shown[active]) pick(shown[active]);
    } else if (e.key === "Escape" && !inline) {
      e.stopPropagation();
      open = false;
      button?.focus();
    }
  }
  $effect(() => {
    query;
    active = 0;
  });
  $effect(() => {
    if (!open || inline) return;
    const close = (e) => {
      if (!root?.contains(e.target)) open = false;
    };
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  });
  function scrollIntoView(node, on) {
    if (on) node.scrollIntoView({ block: "nearest" });
    return { update: (now) => now && node.scrollIntoView({ block: "nearest" }) };
  }
</script>

<div class="search-select" bind:this={root} style="width:{inline ? "100%" : width + "px"}">
  {#if inline}
    <!-- No button: the list is always shown. -->
  {:else if trigger}
    <button class="custom-trigger" bind:this={button} aria-label={label} aria-haspopup="listbox" aria-expanded={open} onclick={toggle}>{@render trigger()}</button>
  {:else}
    <button class="trigger" class:open bind:this={button} aria-label={label} aria-haspopup="listbox" aria-expanded={open} onclick={toggle}>
      {#if current?.icon}<img src={current.icon} alt="" />{/if}
      <span class="value" class:placeholder={!current && !value}>{current?.label ?? (value || placeholder)}</span>
      <Icon name="chevron" size={15} />
    </button>
  {/if}
  {#if open}
    <div class="menu" class:right={align === "right"} class:inline transition:fly={{ y: -4, duration: inline ? 0 : 160 }}>
      <div class="find">
        <Icon name="search" size={15} />
        <input {@attach (node) => node.focus()} placeholder={search} aria-label={search} bind:value={query} onkeydown={key} />
      </div>
      <div class="list" role="listbox" aria-label={label}>
        {#if loading && !options.length}
          <p class="note">Loading…</p>
        {:else if !shown.length}
          <p class="note">{empty}</p>
        {/if}
        {#each shown as option, i (option.value + (option.typed ? "*" : ""))}
          <button class="option" class:active={i === active} class:selected={option.value === value && !option.typed} role="option" aria-selected={option.value === value} use:scrollIntoView={i === active} onpointerenter={() => (active = i)} onclick={() => pick(option)}>
            {#if option.app}<AppIcon name={option.app} src={option.icon} size={22} />{:else if option.icon}<img src={option.icon} alt="" />{:else if option.typed}<Icon name="plus" size={14} />{/if}
            <span class="label">{option.label}</span>
            {#if option.hint}<span class="hint">{option.hint}</span>{/if}
            {#if option.value === value && !option.typed}<Icon name="check" size={14} stroke={2.2} />{/if}
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .search-select {
    position: relative;
    max-width: 100%;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 34px;
    padding: 0 10px 0 12px;
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    font-size: 13.5px;
    text-align: left;
    transition: border-color var(--fast) ease;
  }
  .trigger:hover,
  .trigger.open {
    border-color: var(--muted);
  }
  .trigger :global(svg) {
    color: var(--faint);
    flex-shrink: 0;
  }
  .custom-trigger {
    display: flex;
  }
  .value {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .placeholder {
    color: var(--faint);
  }
  img {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border-radius: 4px;
  }
  .menu {
    position: absolute;
    z-index: 40;
    top: calc(100% + 6px);
    left: 0;
    min-width: 100%;
    width: max(100%, 280px);
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  .menu.inline {
    position: static;
    width: 100%;
    box-shadow: none;
    border-color: var(--border-strong);
  }
  .menu.inline .list {
    max-height: 260px;
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    color: var(--faint);
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    font-size: 13.5px;
    color: var(--text);
    user-select: text;
  }
  .find input::placeholder {
    color: var(--faint);
  }
  .list {
    max-height: 280px;
    overflow-y: auto;
    padding: 5px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 34px;
    padding: 6px 9px;
    border-radius: 9px;
    font-size: 13.5px;
    text-align: left;
  }
  .option.active {
    background: var(--hover);
  }
  .option :global(svg) {
    flex-shrink: 0;
    color: var(--muted);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .hint {
    font-size: 12px;
    color: var(--faint);
    white-space: nowrap;
  }
  .note {
    padding: 10px;
    font-size: 13px;
    color: var(--faint);
  }
</style>
