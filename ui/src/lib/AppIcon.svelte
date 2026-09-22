<script>
  import { app } from "./api.js";
  // The real program icon when Vorto has seen the app, otherwise a letter tile.
  let { name = "", size = 28, src = "" } = $props();
  let icon = $derived(src || $app?.appIcons?.[name]);
  const palette = ["#ff8a65", "#4f9dff", "#34c38f", "#f5b73b", "#a78bfa", "#f06292", "#26c6da"];
  let tint = $derived(palette[[...name].reduce((h, c) => h + c.charCodeAt(0), 0) % palette.length]);
</script>

<span class="app-icon" class:real={icon} style="--size:{size}px; --tint:{tint}" aria-hidden="true">
  {#if icon}
    <img src={icon} alt="" />
  {:else if name}
    <span class="letter">{name[0].toUpperCase()}</span>
  {:else}
    <svg viewBox="0 0 24 24" width={size * 0.5} height={size * 0.5} aria-hidden="true"
      ><path d="M5 4h14a3 3 0 0 1 3 3v8a3 3 0 0 1-3 3h-7l-5 3v-3H5a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3Z" fill="currentColor" /></svg
    >
  {/if}
</span>

<style>
  .app-icon {
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    flex-shrink: 0;
    border-radius: calc(var(--size) * 0.28);
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--brand);
    overflow: hidden;
  }
  .app-icon.real {
    background: none;
    border: 0;
    border-radius: 0;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .letter {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    background: var(--tint);
    color: #fff;
    font-size: calc(var(--size) * 0.46);
    font-weight: 650;
  }
</style>
