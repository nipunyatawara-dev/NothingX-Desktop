# Release audit — 1.0

Reviewed the Rust protocol/parser, model matching, device state and scheduler,
both RFCOMM transports, Swift helper lifecycle, tray/popover host, JavaScript
controls, retained capability data, resource serving and local build scripts.

## Confirmed fixes

- **Rapid custom EQ:** previously only the first input was accepted while a setting was pending. The scheduler now coalesces unsent EQ writes and waits for confirmation of the final target, including a return to the original value. A wire-level regression covers that sequence.
- **Model capabilities:** ANC support is now separate from adjustable intensity. CMF Buds and Buds 2a retain fixed ANC; Ear (open) no longer advertises ANC/transparency controls. Unknown devices stay read-only. These data corrections do not establish hardware verification for those models.
- **Battery and UI:** stereo/headphone battery reports now appear as a single headphone column. Unknown readings stay absent. Unsupported noise/find controls are hidden; the Windows dual-device label says “This computer.”
- **Runtime details:** model JSON is cached instead of reparsed on each state update. Two unused methods were removed. PNG resources have the correct MIME type under the existing nosniff policy. Windows shutdown gives queued commands, including Stop Sound, a bounded chance to flush before closing.
- **Release packaging:** version 1.0 metadata and a CLI version command were added. Separate signed Mac architectures, mounted-image checks, a Windows per-user installer with WebView2 handling, clean source archives and versioned artifact hashes replace the development-only packaging path. All build steps run locally.

## Capability references

ANC without intensity controls must not be mistaken for a model without ANC.
Nothing confirms ANC for [CMF Buds 2a](https://support.nothing.tech/hc/en-us/articles/34007671847569-Does-CMF-Buds-2a-support-ANC)
and [CMF Buds](https://support.cmf.tech/hc/en-us/articles/22781778205713-I-m-not-happy-with-the-noise-cancellation-effect-What-can-I-do).
Nothing's [Ear (open) sound guidance](https://support.nothing.tech/hc/en-us/sections/7455087454993-Sound-issue?page=2)
states that it does not support ANC. Ear (stick)'s firmware-dependent Noise
Reduction feature is not implemented as a separate control in this release.

## Validation and limits

See [VALIDATION.md](VALIDATION.md) for the checks and exact scope. CMF Buds 2 Plus
is the verified hardware target; other model/firmware combinations remain
unverified. Intel execution and native Windows installer/UI/Bluetooth checks
remain pending. No claim of universal compatibility or defect-free code is made.

The Windows installer follows Microsoft's [WebView2 distribution guidance](https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution)
and uses a shared Evergreen Runtime. Local packages are ad-hoc signed on macOS,
unnotarized, and unsigned on Windows. GitHub Actions is disabled.
