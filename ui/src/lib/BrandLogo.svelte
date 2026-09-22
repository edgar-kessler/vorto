<script>
  import nvidia from "../assets/brands/nvidia.svg";
  import openai from "../assets/brands/openai.svg";

  let { family, size = 24 } = $props();
  let parakeet = $derived(family === "parakeet");
</script>

<img
  class="brand-logo"
  class:openai={!parakeet}
  src={parakeet ? nvidia : openai}
  alt={parakeet ? "NVIDIA" : "OpenAI"}
  width={size}
  height={size}
  draggable="false"
/>

<style>
  .brand-logo {
    display: block;
    flex-shrink: 0;
    user-select: none;
  }
  :global([data-theme="dark"]) .openai {
    filter: invert(1);
  }
</style>
