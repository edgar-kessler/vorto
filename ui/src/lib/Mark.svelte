<script>
  // Vorto's mark: two wings that meet in a V, a word taking off. It stays still; the level bars
  // show the voice. Moods: idle, sleep, worried.
  let { size = 48, mood = "idle", onDark = false, tile = false } = $props();
</script>

<svg
  viewBox={tile ? "-8 -8 40 40" : "0 0 24 24"}
  width={size}
  height={size}
  class="mark {mood}"
  class:on-dark={onDark}
  class:tile
  aria-hidden="true"
>
  {#if tile}
    <rect x="-8" y="-8" width="40" height="40" rx="9" class="tile-bg" />
  {/if}
  <path class="wing far" d="M24 .75C17.85 1.34 13.31 5 11.7 10.6V23L12 23.3C17.27 18.17 22.24 10.41 24 .75Z" />
  <path class="wing" d="M0 .75C6.15 1.34 10.69 5 12 10.85V23.3C6.73 18.17 1.76 10.41 0 .75Z" />
</svg>

<style>
  .mark {
    display: block;
    overflow: visible;
    color: var(--text);
  }
  .mark.on-dark,
  .mark.tile {
    color: #fafafa;
  }
  .tile-bg {
    fill: #0d0d0d;
  }
  .wing {
    fill: currentColor;
  }
  .far {
    opacity: 0.58;
  }
  .sleep {
    opacity: 0.45;
  }
  .worried {
    opacity: 0.7;
  }
</style>
