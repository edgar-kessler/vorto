<script>
  // A rating from 1 to 5 as five short segments.
  let { label, value = 0 } = $props();
</script>

<span class="meter" role="img" aria-label="{label} {value} of 5">
  <span class="name" aria-hidden="true">{label}</span>
  <span class="segments" aria-hidden="true">
    {#each [1, 2, 3, 4, 5] as i}<i class:on={i <= value}></i>{/each}
  </span>
</span>

<style>
  .meter {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .segments {
    display: flex;
    gap: 3px;
  }
  i {
    width: 12px;
    height: 5px;
    border-radius: 999px;
    background: var(--bar-bg);
  }
  i.on {
    background: var(--ink);
  }
</style>
