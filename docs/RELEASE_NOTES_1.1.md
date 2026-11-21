# NothingX Desktop 1.1

Download the macOS DMG below to get CMF Buds Pro 2 support, automatic feature detection for unlisted Nothing and CMF earbuds, and the new fallback helper.

## Platform status

- macOS (Apple Silicon and Intel): ready now (`NothingX-Desktop-1.1.0-macos-arm64.dmg` and `NothingX-Desktop-1.1.0-macos-intel.dmg`).
- Windows: v1.1 is currently in development and will be uploaded once Windows testing is complete.

Built locally without GitHub Actions. Source archive and `SHA256SUMS-macos.txt` are attached.

## What changed in 1.1

1. Fixed CMF Buds Pro 2 replies and other payload-only CRC models. The parser now accepts whole-frame CRC, payload-only CRC, no-CRC unsolicited events, and error status codes (sending `0xF001` once if a device reports not activated).
2. Fixed ANC triple ordering, multi-pair in-ear replies, and call-control gesture filtering so CMF Buds Pro 2 battery, noise control, Ultra Bass, EQ, quick settings, and gestures all load and confirm properly.
3. Added automatic runtime feature probing for unlisted Nothing and CMF models, plus stricter model matching so newer models do not fall back to older profiles.
4. Added automatic capability pruning when a headset replies with `status 3` (not supported), removing dead toggles from Quick Settings.
5. Added a fallback card inside the popover for unrecognized or unresponsive Nothing and CMF devices, with buttons to open `ear (web)` (`https://earweb.bttl.xyz/`) in a Chromium browser or open a pre-filled GitHub issue with the detected model name. The card stays hidden when nothing is connected or when a known headset works normally.

## Install on macOS

The macOS builds are ad-hoc signed and unnotarized, so macOS may block first launch.

1. Open the DMG, drag `NothingX Desktop.app` to Applications, and quit any older copy first.
2. Open `NothingX Desktop.app` from Applications.
3. If macOS blocks it, go to System Settings > Privacy & Security, click Open Anyway, and confirm.
4. If Open Anyway does not appear, run this command in Terminal and launch again:

   ```sh
   xattr -dr com.apple.quarantine "/Applications/NothingX Desktop.app"
   ```

## Tested hardware

- CMF Buds 2 Plus on Apple Silicon macOS.
- CMF Buds Pro 2 on Apple Silicon macOS.

## Credits

Original UI design and Mac app by [Yash Raut (Nothing X Mac)](https://github.com/yashraut362/nothing-x-mac). Protocol research and web fallback by the [Radiance ear-web contributors](https://github.com/radiance-project/ear-web) ([earweb.bttl.xyz](https://earweb.bttl.xyz/)). Licensed under AGPL-3.0-only. Unofficial community project, unaffiliated with Nothing or CMF.
