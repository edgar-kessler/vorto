# Changelog

All notable changes to Vorto are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and Vorto follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.1] - 2026-11-04

### Changed

- Dictate shows more: your shortcut, voice model, language and microphone, today's numbers and your last dictations.
- The navigation marks the current page with a thin line.
- The V stays still. While you speak, five small bars in the pill show your voice.
- Apps without an icon get a quiet grey letter instead of a colored tile.

### Fixed

- History showed the real icon only for apps used since Vorto started. Now it shows the icon of every app it knows.
- The website's favicon no longer stays old in browsers that kept it.

## [1.1.0] - 2026-11-02

### Added

- Vorto learns from you. Fix a misheard word right after dictating, or with the pencil in History, and it's remembered. See and remove what it learned on the Dictionary page.
- Parakeet now uses your dictionary: it checks your terms against what you said, so a term goes in only where the audio supports it.
- New voice models: Parakeet Ultra, with fewer errors than v3 at the same speed and now the recommended one, Parakeet Deutsch, primeLine's German fine-tune, and Parakeet English.
- Voice models says which model fits the language you mostly speak.
- A `vorto` command for the terminal: settings, shortcut, voice models, dictionary and more, also while Vorto runs.
- Headless mode: `vorto start` runs only the recording pill, with no window or tray.
- Install from PowerShell: `irm https://edgar-kessler.github.io/vorto/install.ps1 | iex`.

### Changed

- New look, dark and clear: neutral black and white, hairlines instead of cards, navigation along the top, and a new logo.
- A new setup: pick the language you speak and get the right voice model, then try it right away on a sheet of paper.
- A new website: one screen, and a dictation that plays by itself.

### Fixed

- The last word no longer gets cut off when you let go of the shortcut early.

## [1.0.2] - 2026-09-22

### Added

- AI editing: a language model rewrites each dictation before it's inserted, with six presets (Email, Chat message, AI prompt, Notes and lists, Formal writing, Clean up) that you turn on for the apps or websites you choose and can add your own wishes to. Apps come from the ones you've dictated into, with their icons.
- Layout by voice: with AI editing, saying "new line", "three lines, the first says …" or "as bullet points" lays the text out that way. Line breaks are pasted as Windows line breaks and typed as Shift+Enter, so chat apps don't send early.
- Providers for AI editing: Ollama and LM Studio on your PC, found on their own, OpenAI, Anthropic, Google Gemini, Groq, Mistral, OpenRouter, DeepSeek, xAI, Together AI and any OpenAI-compatible address, with their logos. Models come from the provider, with names, context sizes and prices from OpenRouter's catalog, and any model ID can be typed. API keys are kept in Windows Credential Manager. Online providers need your permission before they receive text.
- Esc skips AI editing, and a dictation is inserted as spoken when the model is slow or fails. Local models are loaded while you speak.
- A dictionary for names and terms, which Whisper listens for and AI editing keeps, with suggestions from History.
- Replacements that swap words in every dictation, including line breaks.
- More shortcuts: paste the last dictation again, undo the last insertion, copy the last dictation and turn AI editing on or off.
- The tray menu pastes or copies the last dictation, pastes one of the last five again, turns AI editing on or off and opens History, the Dictionary and Settings.
- History keeps the words as spoken when AI editing changed them, with **Copy as spoken**, and groups dictations by app or by day.
- Stats: time saved compared with typing at your speed, words, dictations, the last 30 days and speaking time by app. Only numbers are kept, never text.
- A step for AI editing in the setup, which finds Ollama or LM Studio on its own or connects an online provider, and a livelier setup with soft light and a glow around the mark.
- **Appearance** in Settings: light, dark or following Windows.
- **Reset everything** in Settings deletes settings, dictionary, AI editing, API keys, History and Stats and starts the setup again.
- A short glow over the words Vorto just inserted, found through UI Automation, or at the cursor in apps that only report that. It can be turned off in Settings.
- Sounds: the click of a mechanical key switch when dictation starts and ends, and a quiet tick when the text is in place. They can be turned off in Settings.

### Changed

- A new logo and app icon, with slimmer bars, also in the app's own mark.
- Redesigned voice model cards, with speed and accuracy at a glance, in the app and the setup.
- The Email preset leaves out the sign-off, since email apps add your signature.

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

[Unreleased]: https://github.com/edgar-kessler/vorto/compare/v1.1.1...HEAD
[1.1.1]: https://github.com/edgar-kessler/vorto/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/edgar-kessler/vorto/compare/v1.0.2...v1.1.0
[1.0.2]: https://github.com/edgar-kessler/vorto/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/edgar-kessler/vorto/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/edgar-kessler/vorto/releases/tag/v1.0.0
