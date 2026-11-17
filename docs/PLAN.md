# NothingX Desktop rebuild

## Decision — 17 November 2026

Use Rust for protocol, device state, scheduling and tray/window lifecycle.
Render the original popover using Wry and the operating system webview:
WKWebView on macOS and WebView2 on Windows. Tao owns the event loop;
tray-icon/muda provide the tray and native pop-up selectors. macOS uses a small
Swift 6 IOBluetooth helper; Windows uses RFCOMM sockets inside Rust.

The visual requirement is to preserve the original panel’s layout and behavior.
Authorized exceptions are NothingX Desktop branding, removing the author footer,
the user-supplied white-backed application icon and transparent black/gray/red
menu-bar mark (adaptive front-bar contrast), readable material
backgrounds with a lighter tint, and sharp Retina symbol rendering. No text-menu replacement is used.

The reference is yashraut362/nothing-x-mac at
`a6120eb13d012d033f88e334f113dc439f37070b`. It is plain JavaScript/Electron 45 with a Swift
IOBluetooth helper, not React. The installed app occupies 477 MiB according to
`du -sh`. It was not running during the audit, so its reported 200 MB RAM has
not been independently measured. REA inspected its installed app.asar, SHA-256
`809534707ab5518d625e9604926dc2cf6a202d6d1073c0baba02cf0637ac030a`, and
found an Electron window/preload/IPC application. Static analysis is partial;
source code is the authority for the implementation plan.

### Why this stack

- Rust shares device logic and transport scheduling across both systems.
- Wry preserves the existing small HTML/CSS panel without bundling Chromium.
  This retains a webview process cost, which is necessary for this UI choice.
- Tauri is viable but its higher-level application framework is unnecessary for
  this small tray app. Wry uses the same system-webview approach directly.
- Slint or native-only menus would require a different renderer and extensive
  UI recreation. Go does not simplify platform RFCOMM; Swift plus C# would
  duplicate shared device logic.

These are architecture judgments, not benchmark claims. Release size and idle
RSS must be measured on the completed build, including its helper process.

## Bluetooth constraints

The control service UUID is AEAC4A03-DFF5-498F-843A-34487CF133EB. It is Bluetooth
Classic SPP/RFCOMM, not BLE/GATT; btleplug is therefore the wrong transport.
Pair and connect audio using the OS first. A phone companion app or another
desktop controller can occupy the control channel. Do not forcibly reset the
audio connection to recover: reconnect the control channel with backoff.

Use the upstream model capability data as compatibility hints. Upstream reports
hardware testing only with CMF Buds 2 Plus. A model appearing in its table does
not establish that it has been tested. Unknown or renamed devices get battery
and firmware reads only rather than speculative feature writes.

## Implementation sequence

1. Build and test the bounded stream parser, command codecs and per-model data.
   Validate reference packet vectors, arbitrary fragmentation, CRC errors,
   malformed payloads and model name precedence.
2. Implement a single owner of device state, paced initial reads, confirmation
   readbacks, setting timeouts, automatic reconnect, and 60-second battery polls.
   Clear every pending operation and stale value on disconnect.
3. Add macOS IOBluetooth and Windows RFCOMM transports, graceful shutdown and
   single-instance protection. Keep transport details out of the protocol core.
4. Preserve the original panel: battery, noise control, bass, EQ presets/custom bands,
   supported quick settings, spatial modes, dual connection, gestures and find.
   Include an explicit 30-second ring limit and left-click-only tray interaction.
5. Package macOS and Windows, run compile/lint/tests, exercise simulation and
   real connection states, measure release footprint, and record unverified
   hardware/platform behavior. Supply reproducible build and validation steps.

## Feature map

| Feature | Implementation target |
| --- | --- |
| Battery and charging | Left, right, case, or stereo; unknown stays unknown |
| Noise control | Off, transparency and supported ANC strengths |
| Bass | Model-specific 5-level / 3-step encodings |
| EQ | Nothing EQ / CMF listening presets; 3 bands from -6 to +6 dB |
| Quick settings | In-ear detection, low latency, existing personal sound profile |
| Spatial audio | Only modes declared by the model mask |
| Dual connection | Enable, remembered devices, connection limit and self protection |
| Gestures | CMF tap profiles; four valid noise cycles |
| Find | Left/right or whole device, explicit stop, auto-stop at 30 seconds |
| Lifecycle | Tray only, reconnect, graceful exit, one instance |

## Acceptance and remaining validation

Aim for a macOS app under 15 MiB. Measure main, helper, WebContent, GPU and
network processes; report RSS separately from physical footprint. The earlier
50 MiB combined RSS budget belonged to the rejected native-menu design and
does not describe the system-webview implementation.
Windows cross-compilation cannot replace a native Windows Bluetooth test.
Real command confirmation, audio continuity, sleep/wake and different firmware
revisions require the corresponding hardware. Simulation must be conspicuously
labelled and must never send Bluetooth writes.

## Sources and attribution

- https://github.com/yashraut362/nothing-x-mac
- https://github.com/radiance-project/ear-web (protocol/model lineage; AGPL-3.0)
- https://docs.rs/tray-icon/0.25.1/tray_icon/
- https://docs.rs/muda/0.20.0/muda/
- https://learn.microsoft.com/en-us/windows/win32/bluetooth/bluetooth-and-socket
- https://v2.tauri.app/concept/architecture/

Upstream package.json declares MIT but its tree contains no standalone LICENSE.
Retain its authorship notice. Because protocol and model work also acknowledges
AGPL ear-web, this rebuild conservatively uses AGPL-3.0-only and ships source
and notices alongside release artifacts.
