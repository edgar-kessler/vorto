<script>
  import { app } from "../lib/api.js";
  import ModelCard from "../lib/ModelCard.svelte";
  let s = $derived($app);
</script>

<header class="page-head"><h1>Voice models</h1></header>

<div class="grid">
  {#each s.models as model (model.id)}
    <ModelCard {model} />
  {/each}
</div>

<p class="note">
  Models are downloaded from Hugging Face and verified before use. Memory is measured on the processor; with a graphics card most of it moves to video memory.
</p>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 14px;
  }
  .note {
    margin-top: 20px;
    font-size: 12.5px;
    color: var(--faint);
    line-height: 1.55;
  }
</style>
