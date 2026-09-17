# Changelog

All notable changes to Vorto are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Vorto follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- A finished model download no longer leaves an empty `.lock` file in the models folder.

## [1.0.1] - 2026-09-17

### Fixed

- "You have the latest version" no longer disappears when Vorto checks for updates in the background right after you asked.
- An old update error goes away once a later check in the background succeeds.
- Esc closes the dialog that asks before clearing History.

## [1.0.0] - 2026-09-17

The first public release.

### Added

- Dictation into any Windows app: hold a shortcut, speak and let go. The text is pasted or typed where you were typing.
- A recording pill at the top or bottom of the screen that shows the words while you speak and the icon and name of the app they go into.
- Live recognition of finished sentences at pauses, so long dictations are inserted about 0.2 to 0.7 seconds after release on the test PC.
- Any key or key combination as the shortcut, including a single key like Menu, in hold-to-talk or toggle mode. Esc cancels a dictation.
- Four local voice models: Parakeet v3 by NVIDIA (recommended, 25 European languages, runs on the processor) and Whisper Turbo, Small and Base (99 languages, on the graphics card through Vulkan when available).
- Model downloads from Hugging Face, pinned to a fixed commit, verified by size and SHA-256 and resumable.
- A separate voice engine process that Vorto restarts after a crash or hang, without taking the app down. Whisper on the graphics card runs in its own engine, and Vorto switches to the processor if the graphics driver fails.
- Pasting that puts your clipboard back afterwards and keeps dictations out of Windows clipboard history, or typing as an alternative.
- History of the last 100 dictations, which can be turned off.
- Onboarding, light and dark mode, a tray icon and an option to start with Windows.
- A per-user installer that needs no administrator rights and adds Microsoft Edge WebView2 if it's missing.
- Signed automatic updates from GitHub Releases, which can be turned off.
- A clear message on processors without AVX2, which aren't supported yet.

[Unreleased]: https://github.com/edgar-kessler/vorto/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/edgar-kessler/vorto/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/edgar-kessler/vorto/releases/tag/v1.0.0
