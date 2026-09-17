<script>
  // Vorto's mark: two quote marks over a text caret, your words becoming text.
  // Moods: idle, listening, thinking, happy, sleep, worried.
  let { size = 48, mood = "idle", level = 0, onDark = false, tile = false } = $props();

  // A squared quote mark with a rounded shoulder.
  const quote = "M26 46c0-11 8.5-20 19.5-20H49v14.5h-3.5c-4.4 0-7.5 3.2-7.5 7.5v1.5h11V80H26V46Z";
  let l = $derived(Math.min(1, Math.max(0, level)));
  let simple = $derived(size < 40);
</script>

<svg
  viewBox="0 0 120 120"
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
  <g class="quotes">
    <g class="q left"><path d={quote} /></g>
    <g class="q right"><path d={quote} transform="translate(45 0)" /></g>
  </g>
  {#if mood === "thinking" && !simple}
    <g class="dots">
      <circle cx="44" cy="96" r="5" />
      <circle cx="60" cy="96" r="5" />
      <circle cx="76" cy="96" r="5" />
    </g>
  {:else}
    <rect class="caret" x="26" y="91" width="68" height="9" rx="4.5" />
  {/if}
</svg>

<style>
  .mark {
    display: block;
    overflow: visible;
    --quote: #ff6250;
    --caret: var(--ink, #111113);
  }
  .on-dark {
    --caret: #ffffff;
  }
  .tile {
    --quote: #ff6250;
    --caret: #ffffff;
  }
  .tile-bg {
    fill: #141416;
  }
  .q path {
    fill: var(--quote);
  }
  .q {
    transform-box: view-box;
    transform-origin: 60px 80px;
    transition: transform 90ms linear;
  }
  .caret {
    fill: var(--caret);
    transform-box: fill-box;
    transform-origin: left center;
    transition: transform 120ms ease-out;
  }
  .dots circle {
    fill: var(--caret);
    animation: typing 1s ease-in-out infinite;
  }
  .dots circle:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots circle:nth-child(3) {
    animation-delay: 0.3s;
  }

  /* Resting: the quotes float gently for a moment, then hold still. An endless animation
     would keep the window drawing frames, and cost power, the whole time Vorto runs. */
  .idle .left {
    animation: float-a 4.8s ease-in-out 2;
  }
  .idle .right {
    animation: float-b 4.8s ease-in-out 2;
  }
  .simple .caret {
    animation: none !important;
  }
  .simple.idle .q {
    animation: none;
  }

  /* Listening: the quotes answer your voice and the caret types along. */
  .listening .left {
    transform: translateY(calc(var(--l) * -9px)) rotate(calc(var(--l) * -6deg));
  }
  .listening .right {
    transform: translateY(calc(var(--l) * -5px)) rotate(calc(var(--l) * 5deg));
  }
  .listening .caret {
    transform: scaleX(calc(0.28 + var(--l) * 0.72));
  }

  .thinking .quotes {
    transform-origin: 60px 60px;
    animation: ponder 1.4s ease-in-out infinite;
  }

  .happy .quotes {
    transform-origin: 60px 64px;
    animation: pop 520ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .sleep {
    opacity: 0.55;
  }

  .worried .left {
    transform: rotate(-10deg) translateX(-2px);
  }
  .worried .right {
    transform: rotate(10deg) translateX(2px);
  }
  .worried .caret {
    fill: #ea2a42;
  }

  @keyframes float-a {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-3px);
    }
  }
  @keyframes float-b {
    0%,
    100% {
      transform: translateY(-2px);
    }
    50% {
      transform: translateY(1px);
    }
  }
  @keyframes typing {
    0%,
    70%,
    100% {
      transform: translateY(0);
      opacity: 0.35;
    }
    35% {
      transform: translateY(-6px);
      opacity: 1;
    }
  }
  @keyframes ponder {
    0%,
    100% {
      transform: rotate(-4deg);
    }
    50% {
      transform: rotate(4deg);
    }
  }
  @keyframes pop {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.18) translateY(-4px);
    }
    100% {
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .q,
    .caret,
    .quotes,
    .dots circle {
      animation: none !important;
    }
  }
</style>
