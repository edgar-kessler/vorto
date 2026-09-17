# Brand assets

The brand mark, colors and type are defined in [design.md](design.md#brand-mark).

- `assets/logo.svg`: the mark as a static SVG.
- `ui/src/lib/Mark.svelte`: the animated mark used in the app.
- `app/icons/`: the app icons, with `source.png` as the 1024 px master. To regenerate
  them, run `npx tauri icon ../app/icons/source.png -o ../app/icons` in `ui/` and delete
  any mobile icon folders it adds.
