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
    padding: 3px;
    border-radius: 10px;
    background: var(--control);
    min-width: 200px;
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--count));
    border-radius: 7px;
    background: var(--surface);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.1),
      0 0 0 0.5px rgba(0, 0, 0, 0.04);
    transform: translateX(calc(100% * var(--index)));
    transition: transform 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  button {
    position: relative;
    height: 28px;
    padding: 0 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--muted);
    transition: color 160ms ease;
  }
  button.active {
    color: var(--text);
  }
</style>
