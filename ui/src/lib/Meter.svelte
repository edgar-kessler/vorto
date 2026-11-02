<script>
  // A rating from 1 to 5 as five short segments.
  let { label, value = 0, hideName = false } = $props();
</script>

<span class="meter" role="img" aria-label="{label} {value} of 5">
  {#if !hideName}<span class="name" aria-hidden="true">{label}</span>{/if}
  <span class="segments" aria-hidden="true">
    {#each [1, 2, 3, 4, 5] as i}<i class:on={i <= value}></i>{/each}
  </span>
</span>

<style>
  .meter {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    color: var(--muted);
  }
  .segments {
    display: flex;
    gap: 2px;
  }
  i {
    width: 8px;
    height: 12px;
    border-radius: 1.5px;
    background: var(--bar-bg);
  }
  i.on {
    background: var(--ink);
  }
</style>
