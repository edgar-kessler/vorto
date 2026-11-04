#!/bin/sh
# Installs Vorto and the `vorto` command on macOS and Linux:
#   curl -fsSL https://edgar-kessler.github.io/vorto/install.sh | sh
set -eu

base="https://github.com/edgar-kessler/vorto/releases/latest/download"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fetch() {
  echo "Downloading $1..."
  curl -fL --progress-bar -o "$tmp/$1" "$base/$1"
}

link_command() {
  mkdir -p "$HOME/.local/bin"
  ln -sf "$1" "$HOME/.local/bin/vorto"
  case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) echo "Add $HOME/.local/bin to your PATH to use the vorto command." ;;
  esac
}

case "$(uname -s)" in
  Darwin)
    if [ "$(uname -m)" != arm64 ]; then
      echo "Vorto for macOS needs a Mac with Apple silicon." >&2
      exit 1
    fi
    fetch Vorto-macOS.dmg
    hdiutil attach -quiet -nobrowse -mountpoint "$tmp/mount" "$tmp/Vorto-macOS.dmg"
    mkdir -p "$HOME/Applications"
    rm -rf "$HOME/Applications/Vorto.app"
    cp -R "$tmp/mount/Vorto.app" "$HOME/Applications/"
    hdiutil detach -quiet "$tmp/mount"
    link_command "$HOME/Applications/Vorto.app/Contents/MacOS/vorto"
    echo ""
    echo "The first time, macOS asks for the microphone and for Accessibility."
    ;;
  Linux)
    if [ "$(uname -m)" != x86_64 ]; then
      echo "Vorto for Linux needs a 64-bit Intel or AMD processor." >&2
      exit 1
    fi
    if command -v apt-get >/dev/null 2>&1; then
      fetch Vorto-Linux.deb
      sudo apt-get install -y "$tmp/Vorto-Linux.deb"
    elif command -v dnf >/dev/null 2>&1; then
      fetch Vorto-Linux.rpm
      sudo dnf install -y "$tmp/Vorto-Linux.rpm"
    else
      fetch Vorto-Linux.AppImage
      mkdir -p "$HOME/.local/bin"
      install -m 755 "$tmp/Vorto-Linux.AppImage" "$HOME/.local/bin/Vorto.AppImage"
      echo ""
      echo "Vorto is installed. Start it with: $HOME/.local/bin/Vorto.AppImage"
      echo "The vorto command comes with the .deb and .rpm packages."
      exit 0
    fi
    ;;
  *)
    echo "This installer is for macOS and Linux. For Windows, see the website." >&2
    exit 1
    ;;
esac

echo ""
echo "Vorto is installed. Open a new terminal, then:"
echo "  vorto models download parakeet-en   a voice model (see: vorto models)"
echo "  vorto start                         only the recording pill"
echo "  vorto start --window                with its window"
