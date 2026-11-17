#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p build dist
swift native/PrepareIcons.swift
swift native/ExportSymbols.swift assets/symbol-map.json assets/appearance.json
iconutil -c icns build/NothingX.iconset -o assets/NothingX.icns
export MACOSX_DEPLOYMENT_TARGET=13.0
if [[ "${1:-}" == "--universal" ]]; then
  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked --target "$target"
  done
  lipo -create target/aarch64-apple-darwin/release/nothingx-desktop target/x86_64-apple-darwin/release/nothingx-desktop -output build/nothingx-desktop
  for arch in arm64 x86_64; do
    swiftc -O -swift-version 6 -target "${arch}-apple-macos13.0" -framework IOBluetooth -framework CoreBluetooth \
      -Xlinker -sectcreate -Xlinker __TEXT -Xlinker __info_plist -Xlinker native/Info.plist \
      native/BluetoothBridge.swift -o "build/nothingx-bluetooth-${arch}"
  done
  lipo -create build/nothingx-bluetooth-arm64 build/nothingx-bluetooth-x86_64 -output build/nothingx-bluetooth
else
  cargo build --release --locked
  cp target/release/nothingx-desktop build/nothingx-desktop
  swiftc -O -swift-version 6 -framework IOBluetooth -framework CoreBluetooth \
    -Xlinker -sectcreate -Xlinker __TEXT -Xlinker __info_plist -Xlinker native/Info.plist \
    native/BluetoothBridge.swift -o build/nothingx-bluetooth
fi
python3 scripts/package-source.py
stage="$(mktemp -d build/package.XXXXXX)"
app="$stage/NothingX Desktop.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp build/nothingx-desktop "$app/Contents/MacOS/"
cp build/nothingx-bluetooth README.md LICENSE THIRD_PARTY.md dist/DEPENDENCY_LICENSES.txt "$app/Contents/Resources/"
cp assets/NothingX.icns "$app/Contents/Resources/"
cp native/AppInfo.plist "$app/Contents/Info.plist"
codesign --force --sign - "$app/Contents/Resources/nothingx-bluetooth"
codesign --force --sign - "$app"
if [[ -d "dist/NothingX Desktop.app" ]]; then
  mv "dist/NothingX Desktop.app" "build/NothingX Desktop.previous.$(date +%s).app"
fi
mv "$app" "dist/NothingX Desktop.app"
app="dist/NothingX Desktop.app"
ditto -c -k --sequesterRsrc --keepParent "$app" dist/NothingX-Desktop-macos.zip
shasum -a 256 dist/NothingX-Desktop-macos.zip dist/NothingX-Desktop-source.zip > dist/SHA256SUMS.txt
du -sh "$app"
