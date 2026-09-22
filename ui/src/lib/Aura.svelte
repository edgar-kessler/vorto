<script>
  // The light around Vorto's mark: a glow that breathes, an arc that circles and soft ripples.
  // With `live`, the ripples follow `level` (0 to 1), such as the voice while you speak.
  let { live = false, level = 0, small = false } = $props();
</script>

<div class="aura" class:live class:small style="--v:{live ? level : 0}" aria-hidden="true">
  <span class="glow"></span>
  <span class="spin"></span>
  <i></i><i></i><i></i>
</div>

<style>
  /* One fixed cell the size of the halo: everything is centered on the mark, even the parts
     larger than it. */
  .aura {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template: 100% / 100%;
    place-items: center;
    pointer-events: none;
    --v: 0;
  }
  .aura > * {
    grid-area: 1 / 1;
  }
  .glow {
    width: 230px;
    height: 230px;
    border-radius: 999px;
    background: conic-gradient(from 0deg, #ff6250, #ffb199, #e8375a, #ffc15e, #ff6250);
    filter: blur(44px);
    opacity: calc(0.16 + var(--v) * 0.4);
    animation:
      turn 18s linear infinite,
      breathe 6s ease-in-out infinite;
  }
  .spin {
    width: 196px;
    height: 196px;
    border-radius: 999px;
    background: conic-gradient(from 0deg, transparent 0 60%, color-mix(in srgb, var(--brand) 15%, transparent) 75%, color-mix(in srgb, var(--brand) 70%, transparent) 100%);
    -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 1.5px), #000 calc(100% - 1px));
    mask: radial-gradient(farthest-side, transparent calc(100% - 1.5px), #000 calc(100% - 1px));
    opacity: 0.8;
    animation: turn 9s linear infinite;
    transform: scale(calc(1 + var(--v) * 0.12));
  }
  .aura i {
    width: 170px;
    height: 170px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--brand) 35%, transparent);
    animation: ripple 5s cubic-bezier(0.16, 0.7, 0.3, 1) infinite;
  }
  .aura i:nth-of-type(2) {
    animation-delay: 2.5s;
  }
  .aura i:nth-of-type(3) {
    display: none;
  }
  @keyframes turn {
    to {
      rotate: 360deg;
    }
  }
  @keyframes breathe {
    50% {
      scale: 1.12;
    }
  }
  @keyframes ripple {
    from {
      transform: scale(0.85);
      opacity: 0.8;
    }
    to {
      transform: scale(1.7);
      opacity: 0;
    }
  }
  /* Voice-driven: ripples swell with how loud you are, the arc speeds up by glowing more. */
  .aura.live i {
    animation: none;
    transition:
      transform 110ms linear,
      opacity 110ms linear;
  }
  .aura.live i:nth-of-type(1) {
    transform: scale(calc(0.95 + var(--v) * 0.4));
    opacity: calc(0.2 + var(--v) * 0.8);
  }
  .aura.live i:nth-of-type(2) {
    transform: scale(calc(1.12 + var(--v) * 0.7));
    opacity: calc(0.12 + var(--v) * 0.55);
  }
  .aura.live i:nth-of-type(3) {
    display: block;
    transform: scale(calc(1.3 + var(--v) * 1));
    opacity: calc(0.06 + var(--v) * 0.35);
  }
  .aura.small .glow {
    width: 190px;
    height: 190px;
  }
  .aura.small .spin {
    width: 164px;
    height: 164px;
  }
</style>
