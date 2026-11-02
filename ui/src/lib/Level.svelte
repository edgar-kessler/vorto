<script>
  // How loud the microphone is right now, as five small bars.
  let { level = 0 } = $props();
  const shape = [0.5, 0.8, 1, 0.8, 0.5];
</script>

<span class="level" aria-hidden="true">
  {#each shape as k}<i style="transform: scaleY({Math.max(0.16, Math.min(1, level) * k)})"></i>{/each}
</span>

<style>
  .level {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 14px;
  }
  i {
    width: 2px;
    height: 100%;
    border-radius: 1px;
    background: currentColor;
    transition: transform 80ms linear;
  }
</style>
