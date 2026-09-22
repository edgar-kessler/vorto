<script>
  // Vorto's mark: a key with your voice on it. Hold the key, talk, and it types.
  // Moods: idle, listening, thinking, happy, sleep, worried.
  let { size = 48, mood = "idle", level = 0, onDark = false, tile = false } = $props();

  // Five bars read well from 40px up; below that three thicker ones stay sharp.
  const full = [
    [38.5, 12],
    [47.5, 22],
    [56.5, 32],
    [65.5, 22],
    [74.5, 12],
  ];
  const small = [
    [40, 16],
    [54.5, 30],
    [69, 16],
  ];
  let l = $derived(Math.min(1, Math.max(0, level)));
  let simple = $derived(size < 40);
  let bars = $derived(simple ? small : full);
  let w = $derived(simple ? 11 : 7);
</script>

<svg
  viewBox={tile ? "0 0 120 120" : "18 18 84 84"}
  width={size}
  height={size}
  class="mark {mood}"
  class:simple
  class:on-dark={onDark}
  class:tile
  style="--l:{l}"
  aria-hidden="true"
>
  {#if tile}
    <rect x="4" y="4" width="112" height="112" rx="30" class="tile-bg" />
  {/if}
  <rect class="lip" x="22" y="30" width="76" height="66" rx="18" />
  <g class="cap">
    <rect class="face" x="22" y="24" width="76" height="62" rx="18" />
    {#if !simple}
      <rect class="dish" x="28" y="30" width="64" height="48" rx="13" />
    {/if}
    <g class="bars">
      {#each bars as [x, h], i}
        <rect class="bar" style="--i:{i}" {x} y={54 - h / 2} width={w} height={h} rx={w / 2} />
      {/each}
    </g>
  </g>
</svg>

<style>
  .mark {
    display: block;
    overflow: visible;
  }
  .tile-bg {
    fill: #141416;
  }
  .lip {
    fill: #b0372a;
  }
  .face {
    fill: #ff6250;
  }
  .dish {
    fill: #ff7363;
  }
  .bar {
    fill: #ffffff;
    transform-box: fill-box;
    transform-origin: center;
    transition: transform 90ms linear;
  }
  .cap {
    transition: transform 90ms linear;
  }

  /* Resting: the bars hum for a moment, then hold still. An endless animation would keep
     the window drawing frames, and cost power, the whole time Vorto runs. */
  .idle .bar {
    animation: hum 2.4s ease-in-out 2;
    animation-delay: calc(var(--i) * 0.12s);
  }
  .simple.idle .bar {
    animation: none;
  }

  /* Listening: the key is held down and the bars follow your voice. */
  .listening .cap {
    transform: translateY(calc(var(--l) * 4px + 2px));
  }
  .listening .bar {
    transform: scaleY(calc(0.4 + var(--l) * 0.95));
  }
  .listening .bar:nth-child(even) {
    transform: scaleY(calc(0.5 + var(--l) * 0.7));
  }
  .listening .bar:nth-child(3) {
    transform: scaleY(calc(0.35 + var(--l) * 1));
  }

  /* Thinking: the key is let go and the bars ripple while the text is written. */
  .thinking .bar {
    animation: ripple 1s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.12s);
  }

  /* Happy: one firm key press. */
  .happy .cap {
    animation: press 480ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .sleep {
    opacity: 0.55;
  }
  .sleep .bar {
    transform: scaleY(0.35);
  }

  .worried .bar {
    transform: scaleY(0.3);
  }
  .worried .cap {
    animation: shake 420ms ease-in-out;
  }

  @keyframes hum {
    0%,
    100% {
      transform: scaleY(1);
    }
    50% {
      transform: scaleY(0.6);
    }
  }
  @keyframes ripple {
    0%,
    100% {
      transform: scaleY(0.35);
    }
    40% {
      transform: scaleY(1.1);
    }
  }
  @keyframes press {
    0%,
    100% {
      transform: translateY(0);
    }
    35% {
      transform: translateY(6px);
    }
  }
  @keyframes shake {
    0%,
    100% {
      transform: translateX(0);
    }
    25% {
      transform: translateX(-3px);
    }
    75% {
      transform: translateX(3px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar,
    .cap {
      animation: none !important;
    }
  }
</style>
