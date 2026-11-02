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
    width: 28px;
    height: 18px;
    border-radius: 4px;
    background: linear-gradient(180deg, transparent, rgba(255, 255, 255, 0.2)), var(--track);
    box-shadow:
      inset 0 -0.5px 0 rgba(255, 255, 255, 0.1),
      inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
    flex-shrink: 0;
  }
  .toggle:hover {
    background: linear-gradient(180deg, transparent, rgba(255, 255, 255, 0.2)), var(--track-hover);
  }
  /* The target is larger than what it shows. */
  .toggle::before {
    content: "";
    position: absolute;
    inset: -8px -6px;
  }
  .toggle.on {
    background: var(--brand);
    box-shadow:
      inset 0 0 0 0.5px rgba(0, 0, 0, 0.2),
      inset 0 1px 1px rgba(0, 0, 0, 0.1);
  }
  .toggle:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 2.5px;
    background: var(--knob);
    box-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.4);
    transition: transform 120ms var(--ease);
  }
  .on .knob {
    transform: translateX(10px);
    background: var(--knob-on);
  }
</style>
