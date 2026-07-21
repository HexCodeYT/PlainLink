import AppKit
import Foundation

struct IconVariant {
    let points: Int
    let scale: Int

    var pixels: Int {
        points * scale
    }

    var filename: String {
        scale == 1 ? "icon_\(points)x\(points).png" : "icon_\(points)x\(points)@2x.png"
    }
}

let variants = [
    IconVariant(points: 16, scale: 1),
    IconVariant(points: 16, scale: 2),
    IconVariant(points: 32, scale: 1),
    IconVariant(points: 32, scale: 2),
    IconVariant(points: 128, scale: 1),
    IconVariant(points: 128, scale: 2),
    IconVariant(points: 256, scale: 1),
    IconVariant(points: 256, scale: 2),
    IconVariant(points: 512, scale: 1),
    IconVariant(points: 512, scale: 2)
]

guard CommandLine.arguments.count == 2 else {
    fputs("usage: render-app-icon.swift <iconset-dir>\n", stderr)
    exit(1)
}

let iconsetURL = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: iconsetURL, withIntermediateDirectories: true)

for variant in variants {
    let image = renderIcon(size: CGFloat(variant.pixels))
    let outputURL = iconsetURL.appendingPathComponent(variant.filename)
    try writePNG(image: image, to: outputURL)
}

func renderIcon(size: CGFloat) -> NSImage {
    let image = NSImage(size: NSSize(width: size, height: size))
    image.lockFocus()
    defer { image.unlockFocus() }

    let rect = NSRect(x: 0, y: 0, width: size, height: size)
    NSGraphicsContext.current?.imageInterpolation = .high
    NSColor.clear.setFill()
    rect.fill()

    let inset = size * 0.055
    let iconRect = rect.insetBy(dx: inset, dy: inset)
    let cornerRadius = size * 0.225
    let background = NSBezierPath(roundedRect: iconRect, xRadius: cornerRadius, yRadius: cornerRadius)

    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.24)
    shadow.shadowBlurRadius = size * 0.035
    shadow.shadowOffset = NSSize(width: 0, height: -size * 0.018)
    shadow.set()

    let gradient = NSGradient(colors: [
        NSColor(calibratedRed: 0.06, green: 0.18, blue: 0.24, alpha: 1),
        NSColor(calibratedRed: 0.05, green: 0.45, blue: 0.43, alpha: 1),
        NSColor(calibratedRed: 0.70, green: 0.96, blue: 0.77, alpha: 1)
    ])!
    gradient.draw(in: background, angle: 42)

    NSGraphicsContext.saveGraphicsState()
    background.addClip()
    drawTopGlow(in: iconRect, size: size)
    drawDiagonalCleanBand(in: iconRect, size: size)
    NSGraphicsContext.restoreGraphicsState()

    drawLink(in: iconRect, size: size)
    drawCheck(in: iconRect, size: size)

    return image
}

func drawTopGlow(in rect: NSRect, size: CGFloat) {
    let glow = NSBezierPath(ovalIn: NSRect(
        x: rect.minX - size * 0.10,
        y: rect.midY + size * 0.05,
        width: size * 0.92,
        height: size * 0.58
    ))
    NSColor.white.withAlphaComponent(0.16).setFill()
    glow.fill()
}

func drawDiagonalCleanBand(in rect: NSRect, size: CGFloat) {
    let band = NSBezierPath()
    band.move(to: NSPoint(x: rect.minX + size * 0.08, y: rect.minY + size * 0.20))
    band.line(to: NSPoint(x: rect.maxX - size * 0.10, y: rect.maxY - size * 0.19))
    band.line(to: NSPoint(x: rect.maxX - size * 0.02, y: rect.maxY - size * 0.10))
    band.line(to: NSPoint(x: rect.minX + size * 0.17, y: rect.minY + size * 0.29))
    band.close()
    NSColor.white.withAlphaComponent(0.13).setFill()
    band.fill()
}

func drawLink(in rect: NSRect, size: CGFloat) {
    let strokeWidth = max(size * 0.072, 1.6)
    let linkSize = NSSize(width: size * 0.38, height: size * 0.20)

    drawLinkSegment(
        center: NSPoint(x: rect.midX - size * 0.105, y: rect.midY + size * 0.045),
        size: linkSize,
        angle: -28,
        lineWidth: strokeWidth,
        color: NSColor.white.withAlphaComponent(0.94)
    )

    drawLinkSegment(
        center: NSPoint(x: rect.midX + size * 0.105, y: rect.midY - size * 0.045),
        size: linkSize,
        angle: -28,
        lineWidth: strokeWidth,
        color: NSColor(calibratedRed: 0.91, green: 1.00, blue: 0.78, alpha: 0.98)
    )
}

func drawLinkSegment(center: NSPoint, size: NSSize, angle: CGFloat, lineWidth: CGFloat, color: NSColor) {
    let origin = NSPoint(x: center.x - size.width / 2, y: center.y - size.height / 2)
    let path = NSBezierPath(roundedRect: NSRect(origin: origin, size: size), xRadius: size.height / 2, yRadius: size.height / 2)
    var transform = AffineTransform()
    transform.translate(x: center.x, y: center.y)
    transform.rotate(byDegrees: angle)
    transform.translate(x: -center.x, y: -center.y)
    path.transform(using: transform)

    color.setStroke()
    path.lineWidth = lineWidth
    path.stroke()
}

func drawCheck(in rect: NSRect, size: CGFloat) {
    let check = NSBezierPath()
    check.move(to: NSPoint(x: rect.minX + size * 0.60, y: rect.minY + size * 0.30))
    check.line(to: NSPoint(x: rect.minX + size * 0.70, y: rect.minY + size * 0.20))
    check.line(to: NSPoint(x: rect.minX + size * 0.84, y: rect.minY + size * 0.40))
    check.lineCapStyle = .round
    check.lineJoinStyle = .round
    check.lineWidth = max(size * 0.045, 1.2)
    NSColor.white.withAlphaComponent(0.92).setStroke()
    check.stroke()
}

func writePNG(image: NSImage, to url: URL) throws {
    guard let tiffData = image.tiffRepresentation else {
        throw NSError(domain: "PlainLinkIcon", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "could not create TIFF representation for \(url.lastPathComponent)"
        ])
    }

    guard let bitmap = NSBitmapImageRep(data: tiffData) else {
        throw NSError(domain: "PlainLinkIcon", code: 2, userInfo: [
            NSLocalizedDescriptionKey: "could not create bitmap representation for \(url.lastPathComponent)"
        ])
    }

    guard let pngData = bitmap.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "PlainLinkIcon", code: 3, userInfo: [
            NSLocalizedDescriptionKey: "could not encode \(url.lastPathComponent)"
        ])
    }

    try pngData.write(to: url)
}
