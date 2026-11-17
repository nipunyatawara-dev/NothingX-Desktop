# NothingX Desktop 1.0

First release of NothingX Desktop, a menu-bar and system-tray app for Nothing and CMF headphones rebuilt in Rust using the OS webview and native Bluetooth.

## Downloads

- Apple Silicon Mac: `NothingX-Desktop-1.0.0-macos-arm64.dmg`
- Intel Mac: `NothingX-Desktop-1.0.0-macos-intel.dmg`
- Windows x64 installer: `NothingX-Desktop-1.0.0-windows-x64-setup.exe`
- Portable Windows ZIP, source archive, and `SHA256SUMS.txt` are also attached.

On macOS, open the DMG and drag `NothingX Desktop.app` to Applications. On Windows, the installer sets up the app for your user account and installs Microsoft Edge WebView2 if it is missing (this requires internet access).

## Opening the unsigned builds

Neither build is notarized or code-signed with a paid developer certificate, so your OS will block it on first launch.

On macOS ("app can't be opened"):
1. Open **System Settings > Privacy & Security** and click **Open Anyway**.
2. Or clear the quarantine flag in Terminal:
   ```sh
   xattr -cr "/Applications/NothingX Desktop.app"
   ```

On Windows (SmartScreen / security warning):
1. Click **More info** (or **View more**).
2. Click **Run anyway**.

## What is in 1.0

- Battery and charging status for left earbud, right earbud, and case.
- Noise control modes (ANC, transparency, capsule-shaped ANC intensity) and Ultra Bass.
- EQ presets and custom bass, mid, and treble sliders.
- Quick settings for in-ear detection, low latency, spatial audio, dual connection, Find sounds, and earbud gesture controls.
- Auto-reconnection to already paired headphones without dropping Bluetooth audio.
- Compact translucent popover on macOS and tray panel on Windows. Left-click the tray icon to open, click outside to close, and use the power button to quit.

## Support

CMF Buds 2 Plus is tested on Apple Silicon (macOS 13+), with 2 to 3 hours of daily use. Other Nothing and CMF profiles are carried over from upstream and are unverified on hardware.

Intel Mac and Windows 10/11 x64 builds are compiled and packaged, but native runtime and Bluetooth testing on those platforms is still pending. Built locally without GitHub Actions.

## Credits

Original UI design and Mac app by [Yash Raut (Nothing X Mac)](https://github.com/yashraut362/nothing-x-mac). Protocol and device profiles from the [Radiance ear-web contributors](https://github.com/radiance-project/ear-web). Licensed under AGPL-3.0-only. Unofficial community project, not affiliated with Nothing or CMF.
