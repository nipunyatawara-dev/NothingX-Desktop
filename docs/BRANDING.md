# NothingX Desktop artwork

User-supplied source: `assets/nothingx-logo-original.webp`. This replaces the reference
repository's logo throughout the application. The reference author's footer and
menu link are removed; source attribution remains in THIRD_PARTY.md.

The selected transparent color cutout is `assets/nothingx-logo-cutout.png`.
It was produced with the built-in image tool, using this prompt:

```text
Use case: background-extraction. Edit target: the supplied Nothing X logo.
Remove ONLY the white rounded-square background to actual transparency.
Preserve the foreground artwork EXACTLY: the front charcoal-black diagonal
rounded capsule, the rear light-gray diagonal rounded capsule, and the red
circular accent. Keep their original colors, sizes, proportions, angles,
overlaps, positions and crisp smooth edges unchanged. Do not redraw, restyle,
monochromize, recolor, add shading, invent texture, change the geometry, or add
text. The output must be the original colored foreground mark cut out cleanly
on a genuinely transparent square canvas. Keep the black/gray/red colors intact.
```

`native/PrepareIcons.swift` keeps the original white-backed artwork for the
panel header and application icons. The header uses a 512-pixel representation
inside its existing 28-point rounded-square badge. The transparent cutout is
trimmed only for the menu bar. With dark menu-bar icons, the original
black/gray/red mark is used. With light menu-bar icons, the front black bar becomes
white, preserving the gray rear bar and red accent so the shape stays distinct.
No flat monochrome X is used. AppKit draws the appropriate variant from the
menu bar's current drawing appearance, without polling or requiring hover.
Windows reads its system/taskbar theme rather than its application theme.

`assets/nothingx-menubar.png` and `assets/nothingx-menubar-dark.png` are the
small native icon previews. `assets/NothingX.icns` and `assets/app-icon.ico`
are packaged into the macOS application and Windows executable.
`assets/nothingx-logo-white.png` is the high-resolution white/red variant.
`assets/readme-logo.png` places that variant on a charcoal tile so it stays
visible in both GitHub themes; the README does not use the original white-backed
application icon as its hero logo.

Other controls retain the reference app's SF Symbol names. ExportSymbols.swift
renders native symbol representations directly at high resolution (about
200–340 pixels), avoiding a tiny intermediate TIFF raster. CSS still displays
them at the reference sizes; high-resolution masks preserve Retina sharpness.

The panel retains native menu-material blur, with a 60% light / 65% dark CSS
tint to allow more background through while retaining readable text. The whole
window uses a brief opacity-only opening/closing fade so its content stays sharp.
