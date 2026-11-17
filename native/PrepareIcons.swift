// Keep the supplied application icon distinct from the adaptive menu-bar mark.
import AppKit
import Foundation

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let application = NSImage(contentsOf: root.appendingPathComponent("assets/nothingx-logo-original.webp"))!
let source = NSImage(contentsOf: root.appendingPathComponent("assets/nothingx-logo-cutout.png"))!
var sourceRect = NSRect(origin:.zero,size:source.size)
let image = source.cgImage(forProposedRect:&sourceRect,context:nil,hints:nil)!
// Trim the extraction canvas around the original foreground mark.
let cropped = image.cropping(to:CGRect(x:250,y:250,width:754,height:754))!
let original = NSImage(cgImage:cropped,size:NSSize(width:754,height:754))
let iconset = root.appendingPathComponent("build/NothingX.iconset")
try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
func bitmap(_ size: Int, draw: () -> Void) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: size*4, bitsPerPixel: 32)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    NSGraphicsContext.current?.imageInterpolation = .high
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}
for size in [16,32,128,256,512] {
    for scale in [1,2] {
        let pixels = size*scale
        let rep = bitmap(pixels) { application.draw(in: NSRect(x:0,y:0,width:pixels,height:pixels)) }
        let suffix = scale == 2 ? "@2x" : ""
        try rep.representation(using:.png,properties:[:])!.write(to:iconset.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
    }
}
let icon32 = bitmap(32) { application.draw(in:NSRect(x:0,y:0,width:32,height:32)) }
try Data(bytes:icon32.bitmapData!,count:32*32*4).write(to:root.appendingPathComponent("assets/app-icon.rgba"))
let template = bitmap(36) {
    NSGraphicsContext.current?.imageInterpolation = .high
    original.draw(in:NSRect(x:4,y:4,width:28,height:28))
}
try Data(bytes:template.bitmapData!,count:36*36*4).write(to:root.appendingPathComponent("assets/tray.rgba"))
func whiteFront(_ image: NSBitmapImageRep) -> Data {
    var data = Data(bytes:image.bitmapData!,count:image.pixelsWide*image.pixelsHigh*4)
    data.withUnsafeMutableBytes { raw in
        let pixels = raw.bindMemory(to: UInt8.self)
        for at in stride(from:0,to:pixels.count,by:4) {
            let alpha = Double(pixels[at+3])
            guard alpha > 0 else { continue }
            let rgb = [Double(pixels[at]),Double(pixels[at+1]),Double(pixels[at+2])].map { min(255,$0*255/alpha) }
            if rgb.max()! < 105 && rgb.max()! - rgb.min()! < 40 {
                pixels[at] = pixels[at+3]; pixels[at+1] = pixels[at+3]; pixels[at+2] = pixels[at+3]
            }
        }
    }
    return data
}
let darkPixels = whiteFront(template)
try darkPixels.write(to:root.appendingPathComponent("assets/tray-dark.rgba"))
let darkPreview = bitmap(36) {}
darkPixels.copyBytes(to:darkPreview.bitmapData!,count:36*36*4)
try darkPreview.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("assets/nothingx-menubar-dark.png"))
try template.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("assets/nothingx-menubar.png"))
let header = bitmap(512) { application.draw(in:NSRect(x:0,y:0,width:512,height:512)) }
try header.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("ui/src/assets/nothingx-logo.png"))
// The README uses the white/red menu-bar variant at full resolution. A charcoal
// tile makes the white artwork visible in both GitHub appearance settings.
let whiteMark = bitmap(512) { original.draw(in:NSRect(x:0,y:0,width:512,height:512)) }
whiteFront(whiteMark).copyBytes(to:whiteMark.bitmapData!,count:512*512*4)
try whiteMark.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("assets/nothingx-logo-white.png"))
let whiteImage = NSImage(cgImage:whiteMark.cgImage!,size:NSSize(width:512,height:512))
let readme = bitmap(512) {
    NSColor(calibratedWhite:0.1,alpha:1).setFill()
    NSBezierPath(roundedRect:NSRect(x:0,y:0,width:512,height:512),xRadius:112,yRadius:112).fill()
    whiteImage.draw(in:NSRect(x:76,y:76,width:360,height:360))
}
try readme.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("assets/readme-logo.png"))
try Data("[36,36]\n".utf8).write(to:root.appendingPathComponent("assets/tray-size.json"))
let sizes = [16,32,48,256]
let pngs = sizes.map { size in bitmap(size) { application.draw(in:NSRect(x:0,y:0,width:size,height:size)) }.representation(using:.png,properties:[:])! }
var ico = Data([0,0,1,0,4,0])
var offset = 6 + 16 * sizes.count
func little(_ value:Int)->Data { var value=UInt32(value).littleEndian; return Data(bytes:&value,count:4) }
for (size,png) in zip(sizes,pngs) {
    ico.append(contentsOf:[UInt8(size == 256 ? 0 : size),UInt8(size == 256 ? 0 : size),0,0,1,0,32,0])
    ico.append(little(png.count)); ico.append(little(offset)); offset += png.count
}
for png in pngs { ico.append(png) }
try ico.write(to:root.appendingPathComponent("assets/app-icon.ico"))
