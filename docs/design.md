# Vorto design guide

One visual language for the app, the recording indicator and the website. When in
doubt, copy what `ui/src/app.css` and the existing components already do.

## Character

Quiet, light and precise, in the spirit of modern assistant apps: soft grey surfaces,
black pill buttons, generous whitespace, one warm accent. Nothing glossy, no heavy
shadows, no gradients on controls. Motion is short and purposeful.

## Brand mark

Two squared quote marks over a text caret: your words becoming text.

- Quotes: coral `#ff6250`. Caret: ink `#0b0b0c` on light, white on dark.
- App icon: dark tile `#141416`, radius ~27% of the tile, coral quotes, white caret.
- SVG source: `assets/logo.svg` (icon) and `ui/src/lib/Mark.svelte` (animated).
- Listening: quotes bob with the voice level, caret stretches like typing.
  Writing: three typing dots. Done: a short pop. Never blink while idle.
- The resting float plays twice, then holds still: endless animations keep the window drawing.

The recording pill shows the mark only while the microphone is on or text is being written.
Outcomes use a round badge: green check (inserted), red cross (failed), grey info (neutral).

## Color tokens

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#fbfbfb` | `#111113` | Page background |
| `--sidebar` | `#f5f5f6` | `#17171a` | Secondary panels |
| `--surface` | `#ffffff` | `#1c1c20` | Cards, inputs |
| `--group` | `#f3f3f4` | `#1a1a1d` | Grouped rows, chips |
| `--border` | `#e9e9ec` | `#26262b` | Hairlines |
| `--border-strong` | `#dcdce0` | `#323238` | Inputs, keycaps |
| `--text` | `#0b0b0c` | `#f2f2f4` | Primary text |
| `--muted` | `#6c6c74` | `#9b9ba4` | Secondary text |
| `--faint` | `#a2a2aa` | `#6a6a73` | Hints, meta |
| `--ink` | `#0b0b0c` | `#f4f4f6` | Primary buttons, toggles |
| `--brand` | `#ff6250` | `#ff6250` | Accent, sparingly |
| `--brand-soft` | `#fff0ec` | `#2a1a18` | Accent backgrounds |
| `--green` | `#15a34a` | `#15a34a` | Success |
| `--red` | `#ea2a42` | `#ea2a42` | Errors, recording |

The recording indicator is always dark: `rgba(17,17,19,.96)` with white text.

## Typography

- Family: Inter Variable, weight axis only (`@fontsource-variable/inter` in the app; the
  website serves the same woff2 files from `site/fonts`, no font service), features `cv11`,
  `ss01`, `ss03`. Fallback Segoe UI.
- Scale: 12.5 meta, 13.5 small UI, 14 body, 15–16 lead, 17 section, 26 page title,
  32–38 hero. Headings weight 600–620 with negative tracking (-0.02em to -0.03em).
- Line height 1.45 for UI text, 1.5–1.55 for paragraphs.

## Shape and spacing

- Base unit 4 px; common gaps 8, 12, 14, 16, 22, 28.
- Radii: 8 small, 10 inputs, 12 menus, 16 groups, 18–22 cards, 999 pills. Media inside a
  card sits on its padding with a concentric radius (20 px card, 8 px padding, 12 px media).
- Buttons are pills: primary `--ink` fill, secondary `#ebebee`, ghost transparent.
  Heights 30 (sm), 36, 42 (lg). Weight ~560.
- Cards: `--surface`, 1px `--border`, `0 1px 2px rgba(16,16,20,.04)` shadow.
- Settings rows: title + muted description left, control right, hairline between rows.
  Rows that can't apply are hidden with a slide, not disabled. A row may skip its description,
  or its control when it states a fact.

## Motion

- Easing `cubic-bezier(0.22, 1, 0.36, 1)`; 140 ms for hover, 220–320 ms for state,
  420–520 ms for entrances. Springy overshoot only for small elements appearing.
- Entrances grow from small to full size, never pop in instantly.
- Respect `prefers-reduced-motion`.

## Voice

English, short, friendly and concrete. Say what happens ("Kept in Vorto"), not what the
software is. Use contractions. Say "speak" and "voice model". No exclamation marks except
for real success moments. Privacy is a fact, not a slogan: "Your voice never leaves this
device." On the website the reader may be on another device, so it says "your PC" instead
of "this device" or "this PC".

## One place for each fact

Every fact has one home. Other screens show nothing about it, or at most a red dot on the
Dictate item when dictation can't work.

- Dictations into other apps belong to the pill. Dictations into the open Vorto window belong
  to the Dictate page.
- Confirm an action where it happened (a check icon, new keycaps), not with a toast. Toasts are
  for news the current screen doesn't show.
- Pages have a title and no subtitle or eyebrow.

## Website

`site/index.html` uses these tokens and components, with a few deliberate differences:

- Focus ring: a solid 2 px outline in the text color, not brand at 55 %, so it keeps 3:1 on
  every surface, including the dark privacy card.
- Segmented control: in dark mode the thumb is lighter (`#3a3a40`) than the app's
  `--surface`, it has a 1 px ring, and the chosen label is weight 600. Inside an inset panel
  the track is recessed (`#19191c` in dark mode). Selection never relies on the thumb alone.
- Settings rows inside cards are compact: gap 16, rows 60 px, padding 10, and a
  `--row-line` hairline (`#e6e6ea` / `#303036`) that stays visible on the inset panel.
- Header controls share one height of 30 px (`.btn.sm`, icon buttons, nav links).
- Large marks (40 px and up) show the typing dots while writing; smaller ones keep a steady
  caret, like `simple` in `Mark.svelte`.
