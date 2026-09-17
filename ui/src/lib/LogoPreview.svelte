<script>
  // Design-time sheet for the mark: open /?logos=1 in a browser.
  import Mark from "./Mark.svelte";
  const moods = ["idle", "listening", "thinking", "happy", "sleep", "worried"];
  let level = $state(0.4);
  $effect(() => {
    let t = 0;
    const timer = setInterval(() => {
      t += 0.18;
      level = Math.max(0, Math.sin(t) * 0.5 + Math.sin(t * 2.7) * 0.35 + 0.2);
    }, 60);
    return () => clearInterval(timer);
  });
</script>

<div class="sheet">
  <section>
    {#each moods as mood}
      <figure>
        <div class="stage"><Mark size={110} {mood} {level} /></div>
        <figcaption>{mood}</figcaption>
      </figure>
    {/each}
  </section>
  <section>
    <figure><div class="stage dark"><Mark size={110} mood="listening" {level} onDark /></div><figcaption>on dark</figcaption></figure>
    <figure><div class="stage"><Mark size={96} tile /></div><figcaption>app icon</figcaption></figure>
    <figure>
      <div class="stage row">
        <Mark size={24} />
        <Mark size={24} mood="listening" {level} />
        <div class="pill"><Mark size={20} mood="listening" {level} onDark /> Listening</div>
      </div>
      <figcaption>small</figcaption>
    </figure>
  </section>
</div>

<style>
  .sheet {
    height: 100%;
    overflow: auto;
    padding: 24px;
  }
  section {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 14px;
  }
  figure {
    margin: 0;
    padding: 10px;
    border-radius: 18px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .stage {
    display: grid;
    place-items: center;
    width: 150px;
    height: 150px;
    border-radius: 14px;
    background: var(--group);
  }
  .stage.dark {
    background: #111113;
  }
  .stage.row {
    display: flex;
    gap: 12px;
    justify-content: center;
    width: 260px;
  }
  .pill {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px 0 10px;
    border-radius: 999px;
    background: #111113;
    color: #fff;
    font-weight: 560;
    font-size: 13px;
  }
  figcaption {
    margin-top: 6px;
    text-align: center;
    font-weight: 560;
    text-transform: capitalize;
  }
</style>
