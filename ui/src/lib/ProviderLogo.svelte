<script>
  // The provider's own logo on a tile. `id` is a provider id such as "openai" or "openai-2";
  // unknown ones (a custom address) get a neutral plug.
  import Icon from "./Icon.svelte";

  const logos = import.meta.glob("../assets/brands/ai/*.svg", { query: "?raw", import: "default", eager: true });
  let { id = "", size = 36 } = $props();
  let brand = $derived(id.replace(/-\d+$/, ""));
  let svg = $derived(logos[`../assets/brands/ai/${brand}.svg`]);
</script>

<span class="provider-logo" style="--size:{size}px" aria-hidden="true">
  {#if svg}
    <!-- Bundled SVG files from assets/brands/ai, never user input. -->
    {@html svg}
  {:else}
    <Icon name="plug" size={Math.round(size * 0.5)} />
  {/if}
</span>

<style>
  .provider-logo {
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    flex-shrink: 0;
    border-radius: calc(var(--size) * 0.28);
    background: var(--logo-tile, #fff);
    color: #0b0b0c;
    box-shadow:
      inset 0 0 0 1px var(--border),
      0 1px 2px rgba(16, 16, 20, 0.05);
  }
  .provider-logo :global(svg) {
    width: 56%;
    height: 56%;
  }
  :global([data-theme="dark"]) .provider-logo {
    background: #26262b;
    color: #f2f2f4;
  }
</style>
