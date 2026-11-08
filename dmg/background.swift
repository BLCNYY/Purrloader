// Draws the disk image window background. Usage: swift dmg/background.swift <out.png> <scale>
// Coordinates are in window points (640 x 400), origin top left. Icons sit at (170, 205) and (470, 205).
import AppKit

let size = CGSize(width: 640, height: 400)
let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "background.png"
let scale = args.count > 2 ? CGFloat(Double(args[2]) ?? 1) : 1

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: Int(size.width * scale), pixelsHigh: Int(size.height * scale),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
rep.size = size
let ns = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = ns
let ctx = ns.cgContext
// The context is already scaled to points (rep.size); flip to a top-left origin so the cat's SVG coordinates carry over.
ctx.translateBy(x: 0, y: size.height)
ctx.scaleBy(x: 1, y: -1)
NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx, flipped: true)

func gray(_ v: CGFloat, _ a: CGFloat = 1) -> CGColor { CGColor(gray: v, alpha: a) }

// Background: near black, a touch lighter behind the icons.
ctx.setFillColor(gray(0.035))
ctx.fill(CGRect(origin: .zero, size: size))
let glow = CGGradient(
    colorsSpace: CGColorSpaceCreateDeviceGray(),
    colors: [gray(1, 0.045), gray(1, 0)] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(
    glow, startCenter: CGPoint(x: 320, y: 205), startRadius: 0,
    endCenter: CGPoint(x: 320, y: 205), endRadius: 300, options: [])

// The menu bar icon's outline cat (Sources/Purrloader/MenuBarIcon.swift), same geometry, drawn in white.
// Its coordinates are the icon's 1024-unit canvas with y pointing up.
let catCanvas = CGRect(x: 230, y: 234, width: 564, height: 580)

func drawCat(origin: CGPoint, height: CGFloat, lineWidth: CGFloat) {
    ctx.saveGState()
    let scale = height / catCanvas.height
    ctx.translateBy(x: origin.x, y: origin.y + height)
    ctx.scaleBy(x: scale, y: -scale)
    ctx.translateBy(x: -catCanvas.minX, y: -catCanvas.minY)
    ctx.setStrokeColor(gray(1)); ctx.setFillColor(gray(1))
    ctx.setLineCap(.round); ctx.setLineJoin(.round)
    ctx.setLineWidth(lineWidth / scale)

    // Head and ears merged into one shape, with the same rounding as the app icon.
    let face = CGPath(
        roundedRect: CGRect(x: 282, y: 286, width: 460, height: 360), cornerWidth: 180, cornerHeight: 170,
        transform: nil)
    let ears = [
        [CGPoint(x: 300, y: 520), CGPoint(x: 336, y: 762), CGPoint(x: 486, y: 628)],
        [CGPoint(x: 724, y: 520), CGPoint(x: 688, y: 762), CGPoint(x: 538, y: 628)],
    ].map { points -> CGPath in
        let ear = CGMutablePath(); ear.addLines(between: points); ear.closeSubpath(); return ear
    }
    var silhouette: CGPath = face
    for part in [face] + ears {
        silhouette = silhouette.union(part)
            .union(part.copy(strokingWithWidth: 44, lineCap: .round, lineJoin: .round, miterLimit: 10))
    }
    ctx.addPath(silhouette); ctx.strokePath()

    for x in [424.0, 600.0] { ctx.fillEllipse(in: CGRect(x: x - 30, y: 512, width: 60, height: 60)) }
    ctx.move(to: CGPoint(x: 512, y: 520)); ctx.addLine(to: CGPoint(x: 512, y: 382))
    ctx.move(to: CGPoint(x: 456, y: 438)); ctx.addLine(to: CGPoint(x: 512, y: 382));
    ctx.addLine(to: CGPoint(x: 568, y: 438))
    ctx.strokePath()
    ctx.restoreGState()
}

// Header: cat mark and wordmark, centered together.
let title = NSAttributedString(
    string: "Purrloader",
    attributes: [
        .font: NSFont.systemFont(ofSize: 25, weight: .semibold),
        .foregroundColor: NSColor.white,
        .kern: 0.2,
    ])
let catHeight: CGFloat = 30, catWidth = catHeight * catCanvas.width / catCanvas.height, gap: CGFloat = 11
let titleSize = title.size()
let headerWidth = catWidth + gap + titleSize.width
let headerX = (size.width - headerWidth) / 2, headerY: CGFloat = 62
drawCat(origin: CGPoint(x: headerX, y: headerY - catHeight / 2 - 1), height: catHeight, lineWidth: 2.3)
title.draw(at: CGPoint(x: headerX + catWidth + gap, y: headerY - titleSize.height / 2 - 0.5))

// Caption under the icons.
let caption = NSAttributedString(
    string: "Drag Purrloader into Applications",
    attributes: [
        .font: NSFont.systemFont(ofSize: 12, weight: .regular),
        .foregroundColor: NSColor(white: 1, alpha: 0.42),
        .kern: 0.3,
    ])
let captionSize = caption.size()
caption.draw(at: CGPoint(x: (size.width - captionSize.width) / 2, y: 338))

// A hand-drawn arrow: a brush stroke that swells in the middle and wobbles a little, then two uneven barbs.
func bezier(_ p0: CGPoint, _ c1: CGPoint, _ c2: CGPoint, _ p3: CGPoint, _ t: CGFloat) -> (CGPoint, CGVector) {
    let u = 1 - t
    let p = CGPoint(
        x: u * u * u * p0.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t * t * t * p3.x,
        y: u * u * u * p0.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t * t * t * p3.y)
    let d = CGVector(
        dx: 3 * u * u * (c1.x - p0.x) + 6 * u * t * (c2.x - c1.x) + 3 * t * t * (p3.x - c2.x),
        dy: 3 * u * u * (c1.y - p0.y) + 6 * u * t * (c2.y - c1.y) + 3 * t * t * (p3.y - c2.y))
    let len = max(hypot(d.dx, d.dy), 0.0001)
    return (p, CGVector(dx: d.dx / len, dy: d.dy / len))
}

// Fills a tapered stroke along a cubic curve; width(t) is the full width at t.
func brush(_ p0: CGPoint, _ c1: CGPoint, _ c2: CGPoint, _ p3: CGPoint, wobble: CGFloat, width: (CGFloat) -> CGFloat) {
    let n = 160
    var left: [CGPoint] = [], right: [CGPoint] = []
    for i in 0...n {
        let t = CGFloat(i) / CGFloat(n)
        var (p, d) = bezier(p0, c1, c2, p3, t)
        let normal = CGVector(dx: -d.dy, dy: d.dx)
        let w = wobble * (sin(t * 13 + 0.7) * 0.6 + sin(t * 29 + 2.1) * 0.4) * sin(.pi * t)
        p = CGPoint(x: p.x + normal.dx * w, y: p.y + normal.dy * w)
        let half = width(t) / 2
        left.append(CGPoint(x: p.x + normal.dx * half, y: p.y + normal.dy * half))
        right.append(CGPoint(x: p.x - normal.dx * half, y: p.y - normal.dy * half))
    }
    let path = CGMutablePath()
    path.addLines(between: left + right.reversed())
    path.closeSubpath()
    ctx.addPath(path); ctx.fillPath()
    // Round the ends so the taper doesn't look clipped.
    for (p, i) in [(left[0], 0), (left[n], n)] {
        let r = width(CGFloat(i) / CGFloat(n)) / 2
        let c = CGPoint(x: (p.x + right[i].x) / 2, y: (p.y + right[i].y) / 2)
        ctx.fillEllipse(in: CGRect(x: c.x - r, y: c.y - r, width: r * 2, height: r * 2))
    }
}

// One transparency layer, so where the barbs overlap the shaft it doesn't get brighter.
ctx.setAlpha(0.92)
ctx.beginTransparencyLayer(auxiliaryInfo: nil)
ctx.setFillColor(gray(1))
let start = CGPoint(x: 254, y: 214), tip = CGPoint(x: 388, y: 202)
brush(start, CGPoint(x: 284, y: 168), CGPoint(x: 352, y: 160), tip, wobble: 1.1) { t in
    0.9 + 2.5 * pow(sin(.pi * min(t * 1.1, 1)), 0.7) + 0.9 * t
}
let (_, dir) = bezier(start, CGPoint(x: 284, y: 168), CGPoint(x: 352, y: 160), tip, 1)
func barb(angle: CGFloat, length: CGFloat, bow: CGFloat) {
    let back = atan2(-dir.dy, -dir.dx) + angle
    let end = CGPoint(x: tip.x + cos(back) * length, y: tip.y + sin(back) * length)
    let mid = CGPoint(x: (tip.x + end.x) / 2 - sin(back) * bow, y: (tip.y + end.y) / 2 + cos(back) * bow)
    brush(CGPoint(x: tip.x + dir.dx * 0.6, y: tip.y + dir.dy * 0.6), mid, mid, end, wobble: 0.3) { t in 4.2 - 3.0 * t }
}
barb(angle: 0.58, length: 23, bow: 1.6)
barb(angle: -0.52, length: 20, bow: -1.3)
ctx.endTransparencyLayer()

NSGraphicsContext.current = nil
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
