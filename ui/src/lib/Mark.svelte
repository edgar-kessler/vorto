<script>
  // Vorto's mark: two wings that meet in a V, a word taking off.
  // Moods: idle, listening, thinking, happy, sleep, worried.
  let { size = 48, mood = "idle", level = 0, onDark = false, tile = false } = $props();
  let l = $derived(Math.min(1, Math.max(0, level)));
</script>

<svg
  viewBox={tile ? "-8 -8 40 40" : "0 0 24 24"}
  width={size}
  height={size}
  class="mark {mood}"
  class:on-dark={onDark}
  class:tile
  style="--l:{l}"
  aria-hidden="true"
>
  {#if tile}
    <rect x="-8" y="-8" width="40" height="40" rx="9" class="tile-bg" />
  {/if}
  <path class="wing far" d="M24 .75C17.85 1.34 13.31 5 11.7 10.6V23L12 23.3C17.27 18.17 22.24 10.41 24 .75Z" />
  <path class="wing near" d="M0 .75C6.15 1.34 10.69 5 12 10.85V23.3C6.73 18.17 1.76 10.41 0 .75Z" />
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
    /* Both turn around the shared tip. */
    transform-origin: 12px 23.3px;
    transition: transform 80ms linear;
  }
  .far {
    opacity: 0.58;
  }

  /* Listening: the wings open with your voice. */
  .listening .near {
    transform: rotate(calc(var(--l) * -9deg));
  }
  .listening .far {
    transform: rotate(calc(var(--l) * 9deg));
  }

  /* Thinking: one slow beat of the wings while the text is written. */
  .thinking .near {
    animation: beat-near 900ms ease-in-out infinite;
  }
  .thinking .far {
    animation: beat-far 900ms ease-in-out infinite;
  }

  /* Happy: one short beat. */
  .happy .near {
    animation: beat-near 360ms ease-out;
  }
  .happy .far {
    animation: beat-far 360ms ease-out;
  }

  .sleep {
    opacity: 0.45;
  }
  .worried {
    opacity: 0.7;
  }

  @keyframes beat-near {
    50% {
      transform: rotate(-8deg);
    }
  }
  @keyframes beat-far {
    50% {
      transform: rotate(8deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .wing {
      animation: none !important;
    }
  }
</style>
