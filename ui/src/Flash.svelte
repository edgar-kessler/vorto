<script>
  // The glow over text Vorto just inserted. The window covers the text; each rectangle is a
  // line of it, in physical pixels relative to the window.
  import { onMount } from "svelte";
  import { isTauri } from "./lib/api.js";

  let flash = $state(null);

  onMount(() => {
    if (!isTauri) {
      // In the browser: ?flash shows a sample, #flash selects this page.
      flash = { id: 1, rects: [[18, 18, 420, 22], [18, 42, 260, 22]] };
      return;
    }
    let stop;
    import("@tauri-apps/api/event").then(({ listen }) =>
      listen("flash", (e) => (flash = JSON.parse(e.payload))).then((un) => (stop = un)),
    );
    return () => stop?.();
  });
  const px = (v) => `${v / devicePixelRatio}px`;
</script>

{#if flash}
  {#key flash.id}
    <div class="flash">
      {#each flash.rects as [x, y, w, h]}
        <!-- Some apps only say where the caret is: that glows as a bar. -->
        <span class="line" class:caret={w <= 3} style="left:{px(w <= 3 ? x - 2 : x)}; top:{px(y)}; width:{px(Math.max(w, 5))}; height:{px(h)}"></span>
      {/each}
    </div>
  {/key}
{/if}

<style>
  .flash {
    position: fixed;
    inset: 0;
    pointer-events: none;
  }
  .line {
    position: absolute;
    border-radius: 5px;
    background: rgba(255, 98, 80, 0.16);
    box-shadow:
      0 0 0 1.5px rgba(255, 98, 80, 0.55),
      0 0 16px 2px rgba(255, 98, 80, 0.35);
    opacity: 0;
    animation: glow 1150ms cubic-bezier(0.22, 1, 0.36, 1) forwards;
  }
  .line.caret {
    border-radius: 999px;
    background: rgba(255, 98, 80, 0.9);
    box-shadow:
      0 0 10px 3px rgba(255, 98, 80, 0.55),
      0 0 26px 8px rgba(255, 98, 80, 0.25);
  }
  @keyframes glow {
    0% {
      opacity: 0;
      transform: scale(0.96);
    }
    14% {
      opacity: 1;
      transform: scale(1.015);
    }
    30% {
      transform: scale(1);
    }
    62% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .line {
      animation: none;
      opacity: 1;
    }
  }
</style>
