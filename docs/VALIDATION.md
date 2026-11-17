# Validation — version 1.0, 18 November 2026

## Scope

Test host: Apple Silicon, macOS 27.2.1. The macOS application contains both
arm64 and x86_64 executables and a universal Swift helper, targets macOS 13+,
and is ad-hoc signed. Intel execution and older macOS versions were not tested.
The Windows x64 GNU executable is cross-built and packaged with WebView2Loader.dll.
MSVC compile/lint checks pass; native Windows UI/Bluetooth execution is pending.

## Live hardware

CMF Buds 2 Plus, firmware 1.0.1.54, connected for audio. The Rust app read real
left/right battery, noise mode, bass and other device state. Battery was first
90% left / 95% right and later 90% / 90%; the case was not reporting a level.
Unknown case readings stay absent instead of becoming zero. Live verification
used status queries and did not change ANC/EQ or play a Find sound.

The owner reports 2–3 hours of successful use with CMF Buds 2 Plus. This is
user-reported runtime coverage, distinct from the automated checks below.

Three version 1.0 quit/relaunch cycles received real battery readings in
1.04, 0.53 and 0.63 seconds without disconnecting/reconnecting Bluetooth audio.
Exact results are in relaunch-results.json. Earlier cold/control-channel checks
on 17 November took 7.61, 7.52 and 7.53 seconds. Startup latency depends on the
current Bluetooth session; these are not comparable performance benchmarks.

The macOS bridge uses asynchronous RFCOMM opening, handles a completion callback
that may arrive during open, keeps its run loop alive to acknowledge close,
and retries the control channel without resetting audio. Opening an already
running application refreshes status and discovery through Tao's Reopen event.

## UI verification

Native accessibility and screenshots verified the readable 300-point popover,
real battery values, new logo and branding, Quick Settings gear navigation,
Equalizer presets, custom EQ controls, back navigation and Quit. The original
panel's layout is retained. Authorized changes replace its branding/author link,
add the supplied logo, sharpen system symbols and lighten the material tint.

Symbols are exported from native SF Symbol representations at approximately
200–340 pixels rather than upscaled from tiny default raster images.
The menu-bar mark keeps red/gray details and switches its front bar between
black and white according to the menu-bar appearance.
The current icon pass restores the supplied white-backed rounded-square icon
in the header and native application bundles. A screenshot verified the header
crop at its original size. The rebuilt app reconnected to CMF Buds 2 Plus and
read 80% left / 85% right without reconnecting Bluetooth audio.

macOS now draws the menu-bar variant using AppKit's current drawing appearance
with image caching disabled, so selection happens on native redraw rather than
waiting for a hover. Windows reads SystemUsesLightTheme for taskbar contrast;
native Windows appearance switching still needs runtime testing. The README
uses a high-resolution white/red mark on a charcoal tile, separate from the
white-backed application icon.

Opening fades the whole native window over 150 ms and closing over 125 ms,
using cubic-bezier(.23,1,.32,1). There is no scale that could blur its contents.
Reduced Motion uses the gentler 125 ms fade. Timers stop when the transition
ends. Focus loss and native macOS outside-click monitors hide the popover; no
keep-open testing override is shipped. The tray icon only responds to left
click: right-click has no context menu. Quit is the panel’s power button.

## Footprint

Universal macOS 1.0 bundle: approximately 6.3 MiB
including helper, application artwork and dependency notices. Rebuilding may
slightly change sizes.

A pre-1.0 connected, visible idle snapshot measured about **77 MiB summed physical
footprint** across all five relevant processes: application 26.4 MiB, WebKit
GPU 12.6 MiB, WebContent 27.7 MiB, Networking about 6.2 MiB and Bluetooth helper
about 3.6 MiB. Each sampled 0.0% CPU at that instant. Raw vmmap readings are in
memory-measurement.json. This is one snapshot, not a long-term benchmark.

Summed `ps` RSS at that snapshot was about 217 MiB; RSS counts shared resident
pages differently and should not be confused with physical footprint. The
original app's user-reported 200 MB RAM was not benchmarked under the same
conditions, so no measured RAM reduction percentage is claimed. The system
webview preserves the required UI and has a real process cost.

## Automated checks and remaining limits

19 Rust tests pass, including independent upstream wire vectors, CRC reference
values, fragmentation/corruption, battery decoding, confirmed writes/timeouts,
disconnect cleanup, EQ encoding, ring auto-stop and dual-device protection.
Formatting and Clippy with warnings denied pass for macOS and Windows MSVC;
Swift 6 and release linking pass. The packaged app passes strict codesign
verification. Additional regression cases cover rapid custom-EQ input and ANC
on models that have no adjustable intensity. Node-based UI checks cover stereo
headphone battery reports, ordinary earbuds, unknown readings and noise controls.

Separate arm64 and x86_64 DMGs are built locally. Each image is checksummed,
mounted read-only, and checked for the correct single architecture in both
executables, version 1.0 metadata, an Applications link and strict app signature
verification after packaging. Extension-hiding Finder metadata was removed
because it invalidated strict signature checks on the packaged app.

The Windows x64 NSIS installer is built locally, extracted with 7-Zip and its
app/loader payloads inspected for x64 architecture and version metadata. The
app imports only Windows system libraries and the included WebView2 loader.
Setup includes Microsoft's Evergreen bootstrapper and checks for an existing
Runtime before running it. Installing/uninstalling on Windows is not tested here.

An OSV snapshot queried all 275 registry packages in Cargo.lock. It returned
advisories for Linux-only glib and proc-macro-error dependencies; neither is in
the selected macOS or Windows dependency graphs. No matches were returned for
the shipping graphs at the time of the query. See dependency-audit-1.0.json.
This is a database snapshot, not a guarantee that dependencies have no defects.

The repository contains no workflow files. GitHub Actions is disabled, and the
repository had zero Actions runs when preparing this manual release.

Actual setting writes, all model/firmware combinations, Find audio behavior,
sleep/wake and native Windows Bluetooth remain hardware validation work.
Simulation exercises UI paths without any Bluetooth writes.

A real Finder-window click verified `popoverVisible=false` and
`popoverOpen=false` while the same live earbud session stayed connected.
Native mouse monitors complement focus-loss handling for desktop/menu-bar
clicks. The tray has no attached context menu and its handler ignores right
clicks; the panel power button retains application Quit.


## Implementation guidance

The animation skill shaped the brief native opacity transition: a frequently
used popover gets subtle spatial feedback, no bounce or scale, interruption
starts from current opacity, and no timers continue at rest.

Modern Web Guidance `security` shaped the bridge: “Validate and sanitize the
properties of incoming message payloads”. IPC is restricted to the embedded
app origin, commands are validated in Rust, assets have explicit MIME types,
and the existing CSP is preserved. No remote scripts are loaded.

## Spatial-audio follow-up fix

The Rust decoder required two bytes for a spatial-audio status report, while the
upstream decoder accepts a single mode byte and defaults the optional
head-tracking byte to zero. One-byte reports were discarded, leaving the
settings tile unknown. The decoder now accepts both layouts, preserving an
empty report as unknown. Regression coverage checks Off, Fixed, Head Tracked
and pending-change confirmation from one-byte reports.

20 Rust tests and macOS/Windows Clippy checks passed for this fix. The universal
Mac application was rebuilt and the installed app updated. A read-only live
check with CMF Buds 2 Plus returned spatial mode `off`, with no pending change,
where the previous build displayed an unknown state. Earbud settings were not
changed during the live check; on/off confirmation paths use protocol fixtures.
