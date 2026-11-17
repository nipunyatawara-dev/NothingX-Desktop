# NothingX Desktop 1.0

A lightweight menu-bar and tray companion for Nothing and CMF headphones,
rebuilt with Rust, the system webview and native Bluetooth transports.

## Downloads

- **Apple Silicon Mac:** `NothingX-Desktop-1.0.0-macos-arm64.dmg`
- **Intel Mac:** `NothingX-Desktop-1.0.0-macos-intel.dmg`
- **Windows x64:** `NothingX-Desktop-1.0.0-windows-x64-setup.exe`
- A portable Windows ZIP, corresponding source ZIP and SHA-256 checksums are also attached.

On macOS, open the DMG and drag the app to Applications. On Windows, setup
installs for the current user and offers Start menu and optional Desktop
shortcuts. If WebView2 is missing, setup installs Microsoft's Evergreen Runtime
using the bundled bootstrapper; that step needs internet access.

## What's included

- Battery and charging readings, supported noise controls, bass, EQ, quick settings, dual connection and gestures.
- The familiar compact panel with sharp icons, translucent Mac material and adaptive menu-bar artwork.
- Left-click to toggle, outside-click dismissal, short opening/closing fades and the panel power button to quit. Right-click does nothing.
- Reconnection to already-connected headphones after restarting the app, without resetting the Bluetooth audio connection.
- Release-audit fixes for rapid custom-EQ changes, headphone battery rendering, unsupported noise controls and Windows wording. Unused methods were removed and model profiles are cached.

## Compatibility and signing

**CMF Buds 2 Plus is the verified device.** The owner reports 2–3 hours of
successful use. Other model profiles are inherited from the upstream projects
and remain unverified on hardware. Available features depend on model and firmware.

Requires **macOS 13+** or **Windows 10/11 x64**. Apple Silicon runtime checks,
Rust tests, cross-platform linting and package checks are recorded in the source's
validation notes. Intel binaries are architecture checked; Intel execution and
native Windows installation, UI and Bluetooth testing remain pending.

Mac packages are **ad-hoc signed and unnotarized**. Windows packages are
**unsigned**. Operating-system security prompts may appear when opening them.

This release was built and uploaded manually. **No GitHub Actions were used;
Actions is disabled for this repository.**

## Credits

Thanks to **[Yash Raut / Nothing X Mac](https://github.com/yashraut362/nothing-x-mac)**
for the original application, panel and Bluetooth work, and to the
**[Radiance ear-web contributors](https://github.com/radiance-project/ear-web)**
for protocol and model knowledge. Retained notices and AGPL-3.0-only terms are
included with the source and packages.

This is an unofficial community project, unaffiliated with Nothing or CMF.
