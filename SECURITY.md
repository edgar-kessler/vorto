# Security policy

Vorto hears what people say and types into their apps, so security reports matter. Thank you for taking the time to send one.

## Supported versions

Security fixes go into the latest release. Vorto updates itself, so that's the version to test against.

| Version | Supported |
|---|---|
| Latest release | Yes |
| Older releases | No |

## Reporting a vulnerability

Please don't open a public issue for a vulnerability. Report it privately through GitHub instead:

1. Open the [Security tab](https://github.com/edgar-kessler/vorto/security) of the repository.
2. Select **Report a vulnerability**.
3. Describe the problem, the Vorto and Windows versions, the steps to reproduce it, and what an attacker could do with it.

Only you and the maintainer can see the report. The conversation and the fix happen in that private advisory. Once a fixed release is out, the advisory can be published, with credit to you if you like.

## What's in scope

These are the parts of Vorto where a weakness would matter most:

- **Voice model downloads.** Files come from Hugging Face, pinned to a fixed commit. Each file must match the size and SHA-256 built into the app before the model is used. A way to get Vorto to load a model file that doesn't match is in scope.
- **Updates.** Update files are signed with minisign through Tauri's updater, and Vorto refuses files that don't match the public key built into the app. A way around that check is in scope.
- **Text insertion and the clipboard.** Vorto pastes through the clipboard (keeping transcripts out of Windows clipboard history and restoring the previous content) or types Unicode input. Leaks of dictated text to other apps or places, or insertion into a window other than the intended one, are in scope.
- **The web interface and the app.** The UI runs in WebView2 with a Content Security Policy and a limited set of Tauri permissions per window (`app/capabilities/`). Ways for content to run script in the UI or reach app commands it shouldn't are in scope.
- **The voice engines.** `vorto-engine.exe` and `vorto-engine-gpu.exe` run as separate processes and take JSON commands from the app. Crafted model files or audio that lead to code execution are in scope.
- **Privacy promises.** Audio or text leaving the PC, or transcripts written to logs, count as security issues.

## What's not in scope

- SmartScreen warnings on the installer. Vorto isn't code-signed yet; this is known.
- Windows not letting Vorto type into apps that run as administrator. That's how Windows protects those apps.
- Attacks that need malware already running as your user, or physical access to an unlocked PC. Such access can read the data folder in `%LOCALAPPDATA%` anyway.
- Vulnerabilities in third-party components that Vorto doesn't make reachable. Please report those upstream.
