# Changelog

All notable changes to Vorto are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Vorto follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.2] - 2026-09-22

### Added

- AI editing: a language model polishes each dictation before it's inserted, in a style picked by app or window title. Styles for email, AI prompts, chats and a general clean-up are included, and their instructions can be changed or extended.
- Providers for AI editing: Ollama and LM Studio on your PC, OpenAI, Anthropic, Google Gemini, Groq, Mistral, OpenRouter, DeepSeek, xAI, Together AI and any OpenAI-compatible address. API keys are kept in Windows Credential Manager. Online providers need your permission before they receive text.
- Esc skips AI editing, and a dictation is inserted as spoken when the model is slow or fails. Local models are loaded while you speak.
- A dictionary for names and terms, which Whisper listens for and AI editing keeps, with suggestions from History.
- Replacements that swap words in every dictation, including line breaks.
- More shortcuts: paste the last dictation again, undo the last insertion, copy the last dictation and turn AI editing on or off.
- The tray menu pastes or copies the last dictation, pastes one of the last five again, turns AI editing on or off and opens History, the Dictionary and Settings.
- History keeps the words as spoken when AI editing changed them, with **Copy as spoken**.
- A short glow over the words Vorto just inserted, found through UI Automation, or at the cursor in apps that only report that. It can be turned off in Settings.
- Sounds: a soft key click when dictation starts and ends, and a chime when the text is in place. They can be turned off in Settings.

### Changed

- A new logo and app icon.

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

[Unreleased]: https://github.com/edgar-kessler/vorto/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/edgar-kessler/vorto/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/edgar-kessler/vorto/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/edgar-kessler/vorto/releases/tag/v1.0.0
