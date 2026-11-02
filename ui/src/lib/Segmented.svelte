<script>
  let { value = $bindable(), options, onchange, label } = $props();
  let index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
</script>

<div class="segmented" role="group" aria-label={label} style="--count:{options.length}; --index:{index}">
  <span class="thumb"></span>
  {#each options as option}
    <button
      class:active={option.value === value}
      aria-pressed={option.value === value}
      onclick={() => {
        value = option.value;
        onchange?.(option.value);
      }}>{option.label}</button
    >
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--count), 1fr);
    padding: 2px;
    border-radius: 6px;
    background: var(--group);
    min-width: 160px;
  }
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc((100% - 4px) / var(--count));
    border-radius: 4px;
    background: var(--control);
    box-shadow: var(--control-shadow);
    transform: translateX(calc(100% * var(--index)));
    transition: transform 120ms var(--ease);
  }
  :global(:root[data-theme="light"]) .thumb {
    background: #ffffff;
  }
  button {
    position: relative;
    height: 24px;
    padding: 0 10px;
    font-size: 12.5px;
    font-weight: 530;
    color: var(--muted);
  }
  button.active {
    color: var(--text);
  }
</style>
