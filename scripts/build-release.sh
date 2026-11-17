#!/bin/bash
# Local manual-release build. This script never publishes or invokes Actions.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v makensis >/dev/null
command -v x86_64-w64-mingw32-gcc >/dev/null
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-x86_64-w64-mingw32-gcc}"
bash scripts/build-macos.sh --universal
if [[ ! -x build/packaging-venv/bin/python ]]; then
  python3 -m venv build/packaging-venv
fi
build/packaging-venv/bin/python -m pip install -r packaging/requirements.txt
build/packaging-venv/bin/python scripts/package-macos.py
cargo build --release --locked --target x86_64-pc-windows-gnu
python3 scripts/package-windows.py target/x86_64-pc-windows-gnu/release/nothingx-desktop.exe --installer
python3 scripts/release-manifest.py
