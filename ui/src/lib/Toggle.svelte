<script>
  let { checked = $bindable(false), disabled = false, onchange, label } = $props();
</script>

<button
  class="toggle"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => {
    checked = !checked;
    onchange?.(checked);
  }}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    border-radius: 999px;
    background: var(--track);
    transition: background 180ms ease;
    flex-shrink: 0;
  }
  .toggle:hover {
    background: var(--track-hover);
  }
  .toggle.on {
    background: var(--ink);
  }
  .on .knob {
    background: var(--knob-on);
  }
  .toggle:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 999px;
    background: var(--knob);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.22);
    transition:
      transform 220ms cubic-bezier(0.22, 1, 0.36, 1),
      width 160ms ease;
  }
  .on .knob {
    transform: translateX(16px);
  }
  .toggle:active .knob {
    width: 21px;
  }
  .on:active .knob {
    transform: translateX(13px);
  }
</style>
