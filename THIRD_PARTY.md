# Source credits and notices

NothingX Desktop is an unofficial companion utility. Nothing and CMF are
trademarks of their respective owners. This project is not endorsed by them.

## Nothing X Mac — Yash Raut

Reference: https://github.com/yashraut362/nothing-x-mac
Audited revision: a6120eb13d012d033f88e334f113dc439f37070b.

The small macOS IOBluetooth bridge is adapted from native/NXBridge.swift.
The command codecs, model profiles and feature behavior were ported to Rust
using the source as a reference. The original popover HTML/CSS and renderer scripts are retained, with the
user-requested application branding and logo changes. No Electron or bundled
Chromium runtime is included. Source attribution is retained in this file.

Copyright (c) 2026 Yash Raut. Upstream package.json declares MIT; upstream does
not include a separate license file at the audited revision. Its declared
permission notice is reproduced here:

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Radiance ear-web contributors

https://github.com/radiance-project/ear-web

The original project attributes its protocol, model capabilities, EQ presets
and gesture data to ear-web. That project is licensed under AGPL version 3.
NothingX Desktop preserves this lineage and is distributed under AGPL-3.0-only.
The complete license is in LICENSE. Local packages include corresponding source.

Changes dated 17 November 2026: Rust implementation, system-webview host, Windows
RFCOMM support, strict CRC/length validation, confirmed state,
bounded parsing, graceful shutdown, conservative unknown-model behavior, and
Swift 6 bridge isolation and stream framing.

## Rust dependencies

Cargo.lock pins the exact dependency graph. Primary direct dependencies:
Tao, tray-icon/muda, fs2, serde/serde_json and windows-sys. Their source license
notices are included in DEPENDENCY_LICENSES.txt in packaged releases. System
frameworks are supplied by macOS and Windows.

## Microsoft WebView2

Windows packages include WebView2Loader.dll supplied by the webview2-com-sys
package. The Windows setup executable also bundles Microsoft's Evergreen
WebView2 bootstrapper, downloaded directly from Microsoft during packaging.
These Microsoft components retain their own terms; they are not relicensed
under this project's AGPL terms. The shared Runtime is installed only if missing
and is left in place when NothingX Desktop is uninstalled.

Distribution reference: https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
