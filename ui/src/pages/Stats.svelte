<script>
  import { app, act, saveSettings } from "../lib/api.js";
  import { fade, scale } from "svelte/transition";
  import Select from "../lib/Select.svelte";
  import Segmented from "../lib/Segmented.svelte";
  import AppIcon from "../lib/AppIcon.svelte";
  import Icon from "../lib/Icon.svelte";
  import Mark from "../lib/Mark.svelte";

  let s = $derived($app);
  let stats = $derived(s.stats ?? { days: {}, apps: {}, since: "" });
  let wpm = $derived(s.settings.typing_wpm || 40);

  // Typing the same words at your typing speed, minus the time spent speaking them.
  const saved = (t) => Math.max(0, (t.words / wpm) * 60 - t.seconds);
  const sum = (list) =>
    list.reduce((a, t) => ({ dictations: a.dictations + t.dictations, words: a.words + t.words, seconds: a.seconds + t.seconds }), { dictations: 0, words: 0, seconds: 0 });
  let total = $derived(sum(Object.values(stats.days)));
  let totalSaved = $derived(saved(total));

  function duration(seconds, short = false) {
    const s = Math.round(seconds);
    if (s < 60) return `${s} s`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m} min`;
    const h = Math.floor(m / 60);
    const rest = m % 60;
    return short || !rest ? `${h} h` : `${h} h ${rest} min`;
  }
  const number = (n) => (n >= 10000 ? `${(n / 1000).toFixed(n >= 100000 ? 0 : 1)}K` : n.toLocaleString("en-US"));
  const iso = (d) => d.toLocaleDateString("sv-SE");

  // ---------------------------------------------------------------- the last 30 days
  let days = $derived.by(() => {
    const out = [];
    const today = new Date();
    for (let i = 29; i >= 0; i--) {
      const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
      const t = stats.days[iso(d)] ?? { dictations: 0, words: 0, seconds: 0 };
      out.push({ date: d, key: iso(d), ...t, saved: saved(t) });
    }
    return out;
  });
  // Clean ticks in minutes: 0, then a round step so there are three or four lines.
  let scaleDays = $derived.by(() => {
    const max = Math.max(...days.map((d) => d.saved / 60), 0);
    const steps = [1, 2, 5, 10, 15, 20, 30, 60, 120, 240];
    const step = steps.find((st) => max / st <= 3) ?? 480;
    const top = Math.max(step, Math.ceil(max / step) * step);
    return { top, ticks: Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step) };
  });
  let best = $derived(days.reduce((b, d) => (d.saved > (b?.saved ?? 0) ? d : b), null));
  const dayLabel = (d) => d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });

  // ---------------------------------------------------------------- by app
  let apps = $derived.by(() => {
    const list = Object.entries(stats.apps)
      .map(([name, t]) => ({ name, ...t }))
      .sort((a, b) => b.seconds - a.seconds);
    // Eight apps at most; the rest share one bar.
    if (list.length <= 8) return list;
    const rest = sum(list.slice(7));
    return [...list.slice(0, 7), { name: "Other apps", other: true, ...rest }];
  });
  let appMax = $derived(Math.max(...apps.map((a) => a.seconds), 1));
  const iconOf = (name) => s.apps?.find((a) => a.name === name)?.icon ?? "";

  // ---------------------------------------------------------------- hover and table views
  let tip = $state(null); // { x, y, title, rows }
  function show(e, title, rows) {
    const box = e.currentTarget.getBoundingClientRect();
    tip = { x: box.left + box.width / 2, y: box.top, title, rows };
  }
  let dayView = $state("chart");
  let appView = $state("chart");

  const speeds = [20, 30, 40, 50, 60, 80, 100].map((v) => ({ value: v, label: `${v} words a minute` }));
  let confirmReset = $state(false);
  let spoken = $derived(total.seconds > 0 ? Math.round(total.words / (total.seconds / 60)) : 0);
  let since = $derived(stats.since ? new Date(`${stats.since}T12:00:00`).toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" }) : "");
</script>

<header class="page-head">
  <h1>Stats</h1>
  {#if total.dictations}
    <button class="btn ghost sm" onclick={() => (confirmReset = true)}><Icon name="refresh" size={15} /> Reset</button>
  {/if}
</header>

{#if !total.dictations}
  <div class="empty" in:fade>
    <Mark size={80} mood="sleep" />
    <h2>Nothing to count yet</h2>
    <p class="muted">Dictate something, and Vorto shows here how much time it saved you.</p>
  </div>
{:else}
  <section class="hero" in:fade={{ duration: 300 }}>
    <div>
      <span class="hero-label">Time saved</span>
      <strong class="hero-value">{duration(totalSaved)}</strong>
      <span class="hero-note">compared with typing {number(total.words)} words{since ? ` since ${since}` : ""}</span>
    </div>
    <label class="speed">
      <span>Your typing speed</span>
      <Select label="Your typing speed" value={wpm} options={speeds} width={200} onchange={(v) => saveSettings({ typing_wpm: v })} />
    </label>
  </section>

  <div class="tiles">
    {#each [["Words dictated", number(total.words)], ["Dictations", number(total.dictations)], ["Time spoken", duration(total.seconds)], ["Speaking speed", `${spoken} wpm`]] as [label, value], i}
      <div class="tile" in:scale={{ start: 0.96, duration: 320, delay: 60 + i * 50 }}>
        <span class="tile-label">{label}</span>
        <strong class="tile-value">{value}</strong>
      </div>
    {/each}
  </div>

  <!-- ------------------------------------------------------------ last 30 days -->
  <section class="card">
    <header class="card-head">
      <div>
        <h2>Time saved, last 30 days</h2>
        <p>Minutes a day{best && best.saved ? `, most on ${dayLabel(best.date)}` : ""}</p>
      </div>
      <Segmented label="Show time saved as" value={dayView} options={[{ value: "chart", label: "Chart" }, { value: "table", label: "Table" }]} onchange={(v) => (dayView = v)} />
    </header>
    {#if dayView === "chart"}
      <div class="columns-chart" role="img" aria-label="Minutes saved on each of the last 30 days">
        <div class="grid-lines" aria-hidden="true">
          {#each scaleDays.ticks as t}
            <div class="grid-line" style="bottom:{(t / scaleDays.top) * 100}%"><span>{t}</span></div>
          {/each}
        </div>
        <div class="columns">
          {#each days as d, i (d.key)}
            {@const h = (d.saved / 60 / scaleDays.top) * 100}
            <button
              class="column"
              aria-label="{dayLabel(d.date)}: {duration(d.saved)} saved"
              onpointerenter={(e) => show(e, dayLabel(d.date), [[duration(d.saved), "saved"], [number(d.words), "words"], [duration(d.seconds), "spoken"]])}
              onpointerleave={() => (tip = null)}
              onfocus={(e) => show(e, dayLabel(d.date), [[duration(d.saved), "saved"], [number(d.words), "words"], [duration(d.seconds), "spoken"]])}
              onblur={() => (tip = null)}
            >
              {#if d === best && d.saved}<span class="cap-label" style="bottom:calc({h}% + 6px)">{duration(d.saved, true)}</span>{/if}
              <span class="bar" style="height:{h}%; --i:{i}"></span>
            </button>
          {/each}
        </div>
        <div class="x-axis" aria-hidden="true">
          <span>{dayLabel(days[0].date)}</span>
          <span>{dayLabel(days[14].date)}</span>
          <span>Today</span>
        </div>
      </div>
    {:else}
      <div class="table-wrap" in:fade>
        <table>
          <thead><tr><th>Day</th><th>Saved</th><th>Words</th><th>Spoken</th></tr></thead>
          <tbody>
            {#each [...days].reverse().filter((d) => d.dictations) as d (d.key)}
              <tr><td>{dayLabel(d.date)}</td><td>{duration(d.saved)}</td><td>{number(d.words)}</td><td>{duration(d.seconds)}</td></tr>
            {:else}
              <tr><td colspan="4" class="muted">No dictations in the last 30 days</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>

  <!-- ------------------------------------------------------------ by app -->
  <section class="card">
    <header class="card-head">
      <div>
        <h2>Time spoken by app</h2>
        <p>Every dictation{since ? ` since ${since}` : ""}</p>
      </div>
      <Segmented label="Show apps as" value={appView} options={[{ value: "chart", label: "Chart" }, { value: "table", label: "Table" }]} onchange={(v) => (appView = v)} />
    </header>
    {#if appView === "chart"}
      <div class="bars" role="list">
        {#each apps as a, i (a.name)}
          <div class="bar-row" role="listitem">
            <span class="bar-name">
              {#if a.other}<span class="other-icon"><Icon name="more" size={14} stroke={2.4} /></span>{:else}<AppIcon name={a.name === "Vorto" ? "" : a.name} src={iconOf(a.name)} size={22} />{/if}
              <span>{a.name === "Vorto" ? "Kept in Vorto" : a.name}</span>
            </span>
            <button
              class="bar-track"
              aria-label="{a.name}: {duration(a.seconds)} spoken"
              onpointerenter={(e) => show(e, a.name, [[duration(a.seconds), "spoken"], [number(a.dictations), a.dictations === 1 ? "dictation" : "dictations"], [duration(saved(a)), "saved"]])}
              onpointerleave={() => (tip = null)}
              onfocus={(e) => show(e, a.name, [[duration(a.seconds), "spoken"], [number(a.dictations), a.dictations === 1 ? "dictation" : "dictations"], [duration(saved(a)), "saved"]])}
              onblur={() => (tip = null)}
            >
              <span class="hbar" style="width:{Math.max(1.5, (a.seconds / appMax) * 100)}%; --i:{i}"></span>
              <span class="tip-label">{duration(a.seconds)}</span>
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="table-wrap" in:fade>
        <table>
          <thead><tr><th>App</th><th>Spoken</th><th>Dictations</th><th>Words</th><th>Saved</th></tr></thead>
          <tbody>
            {#each apps as a (a.name)}
              <tr><td>{a.name === "Vorto" ? "Kept in Vorto" : a.name}</td><td>{duration(a.seconds)}</td><td>{number(a.dictations)}</td><td>{number(a.words)}</td><td>{duration(saved(a))}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
  <p class="footnote">Time saved is the time typing these words would take at your typing speed, minus the time you spent speaking. Stats keep only numbers, never what you said, and count even with History off.</p>
{/if}

{#if tip}
  <div class="tooltip" style="left:{tip.x}px; top:{tip.y}px" role="tooltip" transition:fade={{ duration: 100 }}>
    <span class="tip-title">{tip.title}</span>
    {#each tip.rows as [value, label]}
      <span class="tip-row"><strong>{value}</strong> {label}</span>
    {/each}
  </div>
{/if}

{#if confirmReset}
  <div class="scrim" transition:fade={{ duration: 160 }} onclick={() => (confirmReset = false)} role="presentation">
    <div class="dialog" transition:scale={{ duration: 200, start: 0.96 }} onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === "Escape" && (confirmReset = false)} role="dialog" aria-modal="true" aria-labelledby="reset-title" tabindex="-1">
      <h2 id="reset-title">Reset your stats?</h2>
      <p class="muted">Vorto starts counting again from zero. History stays.</p>
      <div class="dialog-actions">
        <button class="btn secondary" {@attach (node) => node.focus()} onclick={() => (confirmReset = false)}>Cancel</button>
        <button class="btn danger" onclick={() => ((confirmReset = false), act("resetStats"))}>Reset</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 70px 0;
    text-align: center;
  }
  .empty h2 {
    margin-top: 12px;
  }
  .hero {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    flex-wrap: wrap;
    padding: 26px 26px 24px;
    border-radius: var(--r-xl);
    background: var(--group);
  }
  .hero > div {
    display: flex;
    flex-direction: column;
  }
  .hero-label {
    font-size: 13px;
    font-weight: 540;
    color: var(--muted);
  }
  .hero-value {
    margin-top: 4px;
    font-size: 52px;
    font-weight: 620;
    line-height: 1.05;
    letter-spacing: -0.035em;
  }
  .hero-note {
    margin-top: 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .speed {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    margin-top: 8px;
  }
  @media (max-width: 760px) {
    .tiles {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 16px;
    border-radius: var(--r-lg);
    background: var(--group);
  }
  .tile-label {
    font-size: 12.5px;
    color: var(--muted);
  }
  .tile-value {
    font-size: 19px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .card {
    margin-top: 16px;
    padding: 18px 20px 20px;
    border-radius: var(--r-xl);
    background: var(--group);
  }
  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }
  .card-head h2 {
    font-size: 15px;
  }
  .card-head p {
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--muted);
  }
  /* Columns: 4px rounded tops on a shared baseline, recessive hairline grid. */
  .columns-chart {
    position: relative;
    height: 200px;
    padding-left: 34px;
  }
  .grid-lines {
    position: absolute;
    inset: 0 0 22px 34px;
  }
  .grid-line {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border);
  }
  .grid-line span {
    position: absolute;
    left: -34px;
    top: -8px;
    width: 26px;
    text-align: right;
    font-size: 11px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .columns {
    position: absolute;
    inset: 0 0 22px 34px;
    display: flex;
    align-items: flex-end;
    gap: 2px;
  }
  .column {
    position: relative;
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    border-radius: 6px;
  }
  .column:hover,
  .column:focus-visible {
    background: var(--hover);
  }
  .bar {
    display: block;
    width: min(16px, 70%);
    min-height: 0;
    border-radius: 4px 4px 0 0;
    background: var(--chart);
    transform-origin: bottom;
    animation: grow 620ms var(--ease) both;
    animation-delay: calc(var(--i) * 14ms);
  }
  .column:hover .bar,
  .column:focus-visible .bar {
    filter: brightness(1.08);
  }
  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }
  .cap-label {
    position: absolute;
    font-size: 11.5px;
    font-weight: 560;
    color: var(--text);
    white-space: nowrap;
  }
  .x-axis {
    position: absolute;
    left: 34px;
    right: 0;
    bottom: 0;
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--faint);
  }
  /* Horizontal bars: name, then the bar with its value at the tip. */
  .bars {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .bar-row {
    display: grid;
    grid-template-columns: 180px 1fr;
    align-items: center;
    gap: 12px;
  }
  .bar-name {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    font-size: 13px;
  }
  .bar-name > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .other-icon {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--control);
    color: var(--muted);
  }
  .bar-track {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 6px 0 0;
    border-radius: 8px;
    text-align: left;
  }
  .bar-track:hover,
  .bar-track:focus-visible {
    background: var(--hover);
  }
  .hbar {
    display: block;
    height: 14px;
    border-radius: 0 4px 4px 0;
    background: var(--chart);
    transform-origin: left;
    animation: grow-x 620ms var(--ease) both;
    animation-delay: calc(80ms + var(--i) * 40ms);
  }
  @keyframes grow-x {
    from {
      transform: scaleX(0);
    }
  }
  .tip-label {
    font-size: 12.5px;
    font-weight: 540;
    color: var(--muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .table-wrap {
    max-height: 320px;
    overflow-y: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: left;
    font-weight: 540;
    color: var(--muted);
    font-size: 12px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .footnote {
    margin: 14px 4px 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--faint);
  }
  .tooltip {
    position: fixed;
    z-index: 90;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 12px;
    border-radius: 10px;
    background: var(--surface);
    box-shadow:
      var(--shadow-pop),
      0 0 0 1px var(--border);
    transform: translate(-50%, calc(-100% - 8px));
    pointer-events: none;
    white-space: nowrap;
  }
  .tip-title {
    font-size: 12px;
    color: var(--muted);
  }
  .tip-row {
    font-size: 12.5px;
    color: var(--muted);
  }
  .tip-row strong {
    color: var(--text);
    font-weight: 600;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: var(--scrim);
  }
  .dialog {
    width: 380px;
    padding: 22px;
    border-radius: 18px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }
  .dialog p {
    margin-top: 6px;
    line-height: 1.5;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }
</style>
