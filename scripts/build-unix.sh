#!/bin/sh
# Release build of Vorto on macOS and Linux: the packages end up in packages/.
#   macOS   Vorto-macOS.dmg
#   Linux   Vorto-Linux.deb, Vorto-Linux.rpm and Vorto-Linux.AppImage
# Needs Rust, Node.js with `npm ci --prefix ui` done and CMake. On Linux also the libraries that
# .github/workflows/build.yml installs.
set -eu
cd "$(dirname "$0")/.."

triple=$(rustc -vV | sed -n 's/^host: //p')
case "$(uname -s)" in
  Darwin) bundles=dmg ;;
  *) bundles=deb,rpm,appimage ;;
esac

npm --prefix ui run build
cargo build --release --locked -p vorto-engine
cargo build --release --locked -p vorto --bin vorto-cli

rm -rf app/binaries packages
mkdir -p app/binaries packages
# Tauri's externalBin names carry the target triple.
cp target/release/vorto-engine "app/binaries/vorto-engine-$triple"
cp target/release/vorto-cli "app/binaries/vorto-$triple"
node scripts/notices.mjs app/binaries/THIRD-PARTY-NOTICES.txt

ui/node_modules/.bin/tauri build --ci --config app/tauri.unix.conf.json --bundles "$bundles"

bundle=target/release/bundle
case "$(uname -s)" in
  Darwin)
    cp "$bundle"/dmg/*.dmg packages/Vorto-macOS.dmg
    ;;
  *)
    cp "$bundle"/deb/*.deb packages/Vorto-Linux.deb
    cp "$bundle"/rpm/*.rpm packages/Vorto-Linux.rpm
    cp "$bundle"/appimage/*.AppImage packages/Vorto-Linux.AppImage
    ;;
esac
ls -la packages
