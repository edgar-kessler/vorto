<script>
  let { keys = [], size = "md" } = $props();
</script>

<span class="keys {size}">
  {#each keys as key, i}
    {#if i > 0}<span class="plus">+</span>{/if}
    <kbd class="kbd">{key}</kbd>
  {/each}
</span>

<style>
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    vertical-align: middle;
  }
  .plus {
    color: var(--faint);
    font-size: 12px;
  }
  .lg :global(.kbd) {
    height: 30px;
    padding: 0 11px;
    font-size: 14px;
    border-radius: 8px;
  }
</style>
