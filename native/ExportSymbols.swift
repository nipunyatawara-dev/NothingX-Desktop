// Preserve the original app's system SF Symbol masks and accent color.
import AppKit
import Foundation
let mapping = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1])))
var symbols: [String: String] = [:]
for (key, name) in mapping {
    guard let symbol = NSImage(systemSymbolName: name, accessibilityDescription: nil),
          let image = symbol.withSymbolConfiguration(.init(pointSize: 64, weight: .regular)) else { continue }
    // Draw the native symbol representation directly at 4× resolution. Going
    // through tiffRepresentation first rasterizes the default tiny symbol.
    let scale = 4.0
    let width = max(1, Int(ceil(image.size.width * scale)))
    let height = max(1, Int(ceil(image.size.height * scale)))
    guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: width * 4, bitsPerPixel: 32) else { continue }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    NSGraphicsContext.current?.imageInterpolation = .high
    image.draw(in: NSRect(x: 0, y: 0, width: width, height: height))
    NSGraphicsContext.restoreGraphicsState()
    guard let png = bitmap.representation(using: .png, properties: [:]) else { continue }
    symbols[key] = "data:image/png;base64," + png.base64EncodedString()
}
let color = NSColor.controlAccentColor.usingColorSpace(.sRGB) ?? .systemBlue
let accent = String(format: "#%02x%02x%02x", Int(round(color.redComponent * 255)), Int(round(color.greenComponent * 255)), Int(round(color.blueComponent * 255)))
let data = try JSONSerialization.data(withJSONObject: ["symbols": symbols, "accent": accent], options: [.sortedKeys])
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
