# Changelog

All notable changes to NothingX Desktop live here.

## v1.1 (macOS DMG available; Windows in development)

**Platform status**
- macOS: Apple Silicon and Intel DMGs are built and ready.
- Windows: v1.1 is currently in development and will be added after Windows testing finishes.

**What changed**
1. Fixed CMF Buds Pro 2 and payload-only CRC models. The packet parser accepts whole-frame CRC, payload-only CRC, no-CRC unsolicited events, and status codes (triggering one-shot activation on status `2`), and decoders handle reversed ANC triples, multi-pair in-ear responses, and call-control (`button 9`) gesture filtering.
2. Added automatic runtime feature probing for unlisted Nothing and CMF models. When an unlisted model connects, the app runs read-only probes and builds controls from the features the earbuds confirm. Newer generations like CMF Buds 3 or Nothing Ear (4) are no longer misclassified as older profiles.
3. Added automatic pruning for unsupported features. If a known model returns `status 3` (not supported) on a setting, the app hides that tile instead of showing a dead control.
4. Added a fallback prompt for unrecognized or unresponsive Bluetooth devices. It stays hidden when no Nothing or CMF device is connected and when a recognized headset works normally. If a Nothing or CMF device is connected in Bluetooth but not recognized or not responding, the panel offers buttons to open `ear (web)` (`https://earweb.bttl.xyz/`) or file a pre-filled GitHub issue with the device name.
5. Added built-in update checks and in-app upgrades. The app checks GitHub Releases automatically on launch and when you click `NothingX Desktop` at the bottom of the panel. When a matching update is ready, the footer turns blue (`There is an update`); clicking it downloads the release, verifies its SHA-256 checksum and signature, installs, and relaunches.

**Hardware verification**
- CMF Buds 2 Plus: verified on Apple Silicon macOS.
- CMF Buds Pro 2: verified on Apple Silicon macOS (battery, noise control, Ultra Bass, EQ, quick settings, and gestures).

## v1.0

First Rust and system-webview release replacing the Electron runtime, built locally without GitHub Actions.

1. Added battery and charging display for left earbud, right earbud, and case.
2. Added noise control modes, ANC intensity selector, Ultra Bass, EQ presets, and custom 3-band EQ sliders.
3. Added quick settings for in-ear detection, low latency, spatial audio, dual connection, Find sounds, and earbud gestures.
4. Added native macOS Swift IOBluetooth transport and native Windows WinSock RFCOMM transport.
5. Verified on hardware with CMF Buds 2 Plus on Apple Silicon macOS.
