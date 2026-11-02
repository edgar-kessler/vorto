#!/bin/sh
# Installs the package this runner just built and asks its vorto command for a status: that checks
# the files are there and the command finds what it needs beside it. Needs no display or microphone.
set -eux

export VORTO_DATA_DIR="$(mktemp -d)"
case "$(uname -s)" in
  Darwin)
    mount=$(mktemp -d)
    hdiutil info
    hdiutil imageinfo packages/Vorto-macOS.dmg | head -20
    hdiutil attach -nobrowse -mountpoint "$mount" packages/Vorto-macOS.dmg
    bin="$mount/Vorto.app/Contents/MacOS"
    ls -la "$mount" "$mount/Vorto.app/Contents" "$bin"
    test -x "$bin/vorto-app"
    test -x "$bin/vorto-engine"
    "$bin/vorto" status
    "$bin/vorto" version
    # The way install.sh links it.
    ln -sf "$bin/vorto" "$VORTO_DATA_DIR/vorto"
    "$VORTO_DATA_DIR/vorto" models
    hdiutil detach -quiet "$mount"
    ;;
  *)
    sudo apt-get install -y "$PWD/packages/Vorto-Linux.deb"
    test -x /usr/bin/vorto-app
    test -x /usr/bin/vorto-engine
    vorto status
    vorto version
    vorto models
    ;;
esac
