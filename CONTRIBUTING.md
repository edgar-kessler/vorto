# Contributing to Vorto

Thanks for helping. Bug reports, testing on hardware the maintainer doesn't have, fixes and ideas all make Vorto better.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go through private reporting, not public issues: see [SECURITY.md](SECURITY.md).

## Before you start

- **Bugs:** [open an issue](https://github.com/edgar-kessler/vorto/issues/new/choose) with the bug report form. The logs and hardware details it asks for usually decide whether a problem can be fixed.
- **Small fixes** (typos, clear bugs, docs): send a pull request.
- **Larger changes** (new features, new voice models, changes to how text is inserted): open an issue first, so we can agree on the approach before you spend time on it.

Every change keeps Vorto's promises: audio and text stay on the PC, there's no account and no telemetry, and transcripts are never logged. The network is only for voice model downloads and update checks.

## Repository layout

| Path | What's there |
|---|---|
| `src/` | The `vorto` library: recording, settings and history, the voice model catalog, the engine protocol and supervisor |
| `engine/` | `vorto-engine`, the recognition process: Parakeet through ONNX Runtime, Whisper through whisper.cpp, model downloads |
| `app/` | `vorto-app`, the Tauri 2 desktop app: shortcut hooks, text insertion, windows, tray, updates |
| `ui/` | The interface, Svelte 5 and Vite, rendered in WebView2 |
| `site/` | The landing page, published with GitHub Pages |
| `scripts/` | Build, benchmark and third-party notice scripts |
| `docs/` | [Architecture](docs/architecture.md), [design guide](docs/design.md) and [brand assets](docs/brand.md) |

[docs/architecture.md](docs/architecture.md) explains how the pieces fit together.

## Setting up

You need:

- Windows 10 or 11, x64
- [Rust](https://rustup.rs) 1.98.1 or newer, stable
- [Node.js](https://nodejs.org) 24 with npm
- [Visual Studio](https://visualstudio.microsoft.com) 2022 or newer with the **Desktop development with C++** workload, which includes CMake and Ninja (`scripts\build.cmd` uses Ninja). A separate [CMake](https://cmake.org) install works too.
- [LLVM](https://github.com/llvm/llvm-project/releases), for bindgen. `LIBCLANG_PATH` defaults to `C:/Program Files/LLVM/bin` (see `.cargo/config.toml`); set it yourself if LLVM lives elsewhere.
- The [Vulkan SDK](https://vulkan.lunarg.com), only for the build with Whisper on the graphics card

Then, from the repository root:

```bat
npm ci --prefix ui
npm --prefix ui run build
cargo test --workspace
```

The app embeds the built UI from `ui\dist`, so build it before `cargo test`. `cargo test` builds the engine for the processor only. That's all you need for most changes.

## Building

```bat
scripts\build.cmd
```

This builds the UI, then both voice engines and the app in release mode. The result lands in `C:\vt\release`: `vorto.exe`, `vorto-engine.exe` and `vorto-engine-gpu.exe` (Whisper on the graphics card, built with the `vulkan` feature in `C:\vtg`). Set `VORTO_TARGET` and `VORTO_GPU_TARGET` to use other target folders. Keep their paths short: whisper.cpp's Vulkan shader build otherwise runs into Windows' 260-character path limit.

- `scripts\build.cmd --cpu` leaves out the graphics card engine, so it doesn't need the Vulkan SDK.
- `scripts\build.cmd --installer` builds the NSIS installer into `target\release\bundle\nsis\`. With `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set, it also writes the update signature (`.sig`).

whisper.cpp is compiled for a fixed AVX2 baseline (see `.cargo/config.toml`), not for the CPU of the machine that builds it, so a build runs on every supported PC. `whisper-rs-sys` doesn't notice changes to those settings: after changing them, run `cargo clean -p whisper-rs-sys`.

## Working on the interface

The UI runs in a normal browser against a stand-in for the Rust side, `ui/src/lib/mock.js`:

```bat
npm --prefix ui run dev
```

Open the address Vite prints. URL parameters listed at the top of `mock.js` show specific states, for example `?route=models`, `?onboarding=1`, `?error=1` or the recording pill with `?hud=listening&live=Hello#hud`.

To try the UI inside the real app, keep the Vite server running on port 5173 and start the app without the embedded UI:

```bat
cargo build -p vorto-engine
cargo run -p vorto-app --no-default-features
```

This build loads its interface from the Vite server. It's for development only: never distribute it.

Interface changes follow [docs/design.md](docs/design.md): its tokens, components, motion and voice. Please add screenshots in light and dark mode to the pull request.

## Voice models

The catalog lives in `src/data.rs`. Each model is pinned to a full commit of its Hugging Face repository, and every file has its size and SHA-256 recorded. The engine refuses anything that doesn't match, so moving a model to a new revision means updating those values and shipping a release.

To measure the engine, run `node scripts/benchmark.mjs`. It needs your own 16 kHz WAV clips, which aren't part of the repository; the script explains its options.

## Checks

Run these before opening a pull request:

```bat
cargo fmt --check
npm --prefix ui run build
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Pull requests

- Keep each pull request focused on one change, and explain what it changes and why.
- Say how you tested it: which Windows version, processor, graphics card and voice model.
- Comments in code explain why something is done, not what the next line does.
- User-facing text is English, short and concrete. Use contractions. Say "speak" and "voice model".
- Add a line to the `Unreleased` section of [CHANGELOG.md](CHANGELOG.md) for changes people will notice.

Contributions are released under the [MIT License](LICENSE), like the rest of Vorto.

## Releases

For maintainers:

1. Set the version in `Cargo.toml` (`[workspace.package] version`) and in `ui/package.json`.
2. Run `cargo check --workspace` (without `--locked`) and `npm install --prefix ui --package-lock-only`, so `Cargo.lock` and `ui/package-lock.json` carry the new version. The release build runs `cargo test --locked`, which stops if `Cargo.lock` is out of date.
3. Move the `Unreleased` entries in `CHANGELOG.md` under the new version.
4. Commit the version bump together with both lock files and the changelog, then push a tag `vX.Y.Z`.

The tag runs `.github/workflows/release.yml` in two jobs. The `build` job has read-only access and no secrets: it runs the tests, builds the installer and uploads it as a workflow artifact. The `release` job then signs the installer with the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, writes `latest.json` and publishes `Vorto_X.Y.Z_x64-setup.exe`, `Vorto-Setup.exe`, the `.sig` signature and `latest.json`, which installed copies of Vorto check for updates. Keeping the two apart means the build and its dependencies never see the signing key or get write access to the repository.
