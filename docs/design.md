# Vorto design guide

One visual language for the app, the recording indicator and the website. When in
doubt, copy what `ui/src/app.css` and the existing components already do.

## Character

Dark and clear: neutral black, white and grey with no tint, and no color but status. One flat surface, hairlines instead of panels, small controls with
a thin ring, one family of type. No filled blocks, no pills, no glows, no gradients beyond the faint highlight on a control. Hover is instant;
motion is short and only where something moves.

## Brand mark

Two wings that meet in a V along one straight seam, with a shared tip: a word taking off.

- App icon: near-black tile with a soft gradient and a thin light edge. The left wing is bright silver and casts a slight shadow on the darker right wing, like a fold. No gap between the wings.
- SVG source: `assets/logo.svg` (icon) and `ui/src/lib/Mark.svelte` (flat, animated, right wing at 58 %).
- Listening: the wings open with the voice. Writing: a slow beat. Done: one short beat. Never moves while idle.

The recording indicator shows the mark only while the microphone is on or text is being written.
Outcomes use a small square badge: green check (inserted), red cross (failed), grey info (neutral).

## Color tokens

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#ffffff` | `#0d0d0d` | The page, everywhere |
| `--surface` | `#ffffff` | `#161616` | Menus and dialogs, raised above the page |
| `--group` | black 4 % | white 5 % | Table heads, segmented tracks, chips |
| `--row-hover` | `#f4f4f4` | `#1c1c1c` | A row under the pointer |
| `--border` | black 8 % | white 8 % | Hairlines |
| `--grid-line` | black 6 % | white 6 % | The two lines the page hangs between |
| `--text` | `#161616` | `#fafafa` | Primary text |
| `--muted` | `#5c5c5c` | `#aeaeae` | Secondary text |
| `--faint` | `#808080` | `#808080` | Hints, meta, table heads |
| `--ink` | `#242424` | `#fafafa` | The one primary button |
| `--brand` | `#161616` | `#fafafa` | Switches on and focus: the strongest neutral, not a color |
| `--green` | `#15803d` | `#22c55e` | Ready, success |
| `--red` | `#d92e3c` | `#f1484f` | Errors, destructive actions |

The recording indicator is always dark: `#1c1c1c` with a thin ring and white text.

## Typography

- Inter Variable everywhere, 13 px, weight 440, line height 20, tracking -0.04 px (`@fontsource-variable/inter`
  in the app; the website serves the same woff2 files from `site/fonts`, no font service). Fallback Segoe UI.
- Page titles: 28, weight 600, tracking -0.035 em. Section titles: 15, weight 600. Big figures: tight tracking.
  Emphasis in text: weight 530. Numbers are tabular.

## Shape and spacing

- Base unit 4 px; common gaps 8, 12, 16, 24, 40.
- Radii: 3 chips, 4 small controls and keycaps, 6 controls and rows, 8 menus and dialogs. Nothing rounder,
  dots excepted.
- Buttons: 28 high (24 small, 32 large). Secondary is the default: `--control` with `--control-shadow`,
  a ring thinner than a border. Primary marks at most one action per screen.
- Rows sit directly on the page with a hairline between them, 56 high. Lists are rows with a fill on hover,
  not cards. A table has a filled head row and bare rows, like Voice models.
- Settings rows: title + muted description left, control right. Rows that can't apply are hidden, not
  disabled. A row may skip its description, or its control when it states a fact.
- Navigation runs along the top. The page is at most 880 wide, between two hairlines.

## Motion

- Hover and page changes are instant. 85 to 180 ms for what moves (a switch, the pill).
  Easing `cubic-bezier(0.22, 1, 0.36, 1)`.
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

`site/index.html` is one screen and nothing more: the headline, the download and the show. It's dark only, neutral black with one
white sheet of paper, folded at the corner like the mark. It has one idea, which plays by itself: the pill, the same one the app shows, listens, the words leave it as they're
said, the filler burns up and every real word flies to its place on the sheet, turning from white to black as it crosses the paper. A misheard word is learned on the way. Browsers keep sound off until a click, so the show starts silent; a press replays the scene out loud.

- Five scenes in `scripts/voice-lines.json`, one of them German, each played in turn. `scripts/voice.mjs` makes the
  voices with Eleven v4 through OpenRouter, and `scripts/voice-times.py` measures when each word is said.
- The mark in the pill opens with the real loudness of the voice. Without sound or audio, the same scene plays on estimated times.
- A canvas behind the page draws words and sparks, only while something moves. `prefers-reduced-motion` shows the final sentence.

The README's screenshots of the app come from the mock UI (`ui/src/lib/mock.js`).
