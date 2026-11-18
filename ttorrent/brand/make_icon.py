"""Draws tTorrent's app icon (sprout tile, chosen 2026-11-18) into
ttorrent-256.png, ttorrent.ico (16-256 px) and icon.svg, plus the .torrent
file icon (ttorrent-file.ico). Geometry is on a
256 grid, matching icon.svg; each size is drawn 8x larger and scaled down."""
from pathlib import Path
from PIL import Image, ImageDraw

HERE = Path(__file__).parent
GREEN = (16, 168, 116, 255)
WHITE = (255, 255, 255, 255)

SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
<rect width="256" height="256" rx="58" fill="#10a874"/>
<g fill="none" stroke="#fff" stroke-width="26" stroke-linecap="round" stroke-linejoin="round">
<path d="M128 112v94"/><path d="M84 164l44 44 44-44"/></g>
<path d="M128 116c0-36 22-60 70-60 0 40-22 64-70 60Z M128 116c0-26-18-46-54-46 0 30 18 50 54 46Z" fill="#fff"/>
</svg>
"""


def cubic(p0, p1, p2, p3, steps=48):
    pts = []
    for i in range(steps + 1):
        t = i / steps
        a, b, c, d = (1 - t) ** 3, 3 * (1 - t) ** 2 * t, 3 * (1 - t) * t ** 2, t ** 3
        pts.append((a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1]))
    return pts


def draw(size):
    k = 8
    s = size * k / 256.0
    img = Image.new("RGBA", (size * k, size * k), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, size * k - 1, size * k - 1], radius=58 * s, fill=GREEN)
    w = 26 * s

    def line(a, b):
        d.line([(a[0] * s, a[1] * s), (b[0] * s, b[1] * s)], fill=WHITE, width=round(w))
        for p in (a, b):
            d.ellipse([p[0] * s - w / 2, p[1] * s - w / 2, p[0] * s + w / 2, p[1] * s + w / 2], fill=WHITE)

    line((128, 112), (128, 206))
    line((84, 164), (128, 208))
    line((128, 208), (172, 164))
    leaves = [
        cubic((128, 116), (128, 80), (150, 56), (198, 56)) + cubic((198, 56), (198, 96), (176, 120), (128, 116)),
        cubic((128, 116), (128, 90), (110, 70), (74, 70)) + cubic((74, 70), (74, 100), (92, 120), (128, 116)),
    ]
    for leaf in leaves:
        d.polygon([(x * s, y * s) for x, y in leaf], fill=WHITE)
    return img.resize((size, size), Image.LANCZOS)


def draw_file(size):
    """A .torrent file: a white page with a folded corner and the sprout tile."""
    k = 8
    S = size * k
    s = S / 256.0
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    page, edge, fold = (250, 250, 248, 255), (176, 180, 186, 255), (222, 225, 228, 255)
    x0, y0, x1, y1, c = 40 * s, 12 * s, 216 * s, 244 * s, 56 * s
    outline = [(x0, y0), (x1 - c, y0), (x1, y0 + c), (x1, y1), (x0, y1)]
    d.polygon(outline, fill=edge)
    m = 6 * s
    d.polygon([(x0 + m, y0 + m), (x1 - c - m * 0.4, y0 + m), (x1 - m, y0 + c + m * 0.4), (x1 - m, y1 - m), (x0 + m, y1 - m)], fill=page)
    d.polygon([(x1 - c, y0), (x1 - c, y0 + c), (x1, y0 + c)], fill=fold)
    tile = draw(256).resize((round(136 * s), round(136 * s)), Image.LANCZOS)
    img.alpha_composite(tile, (round(60 * s), round(84 * s)))
    return img.resize((size, size), Image.LANCZOS)


def main():
    (HERE / "icon.svg").write_text(SVG, encoding="utf8")
    draw(256).save(HERE / "ttorrent-256.png")
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    images = [draw(n) for n in sizes]
    images[-1].save(HERE / "ttorrent.ico", sizes=[(n, n) for n in sizes], append_images=images[:-1])
    files = [draw_file(n) for n in sizes]
    files[-1].save(HERE / "ttorrent-file.ico", sizes=[(n, n) for n in sizes], append_images=files[:-1])
    draw_file(256).save(HERE / "ttorrent-file-256.png")
    print("ok")


if __name__ == "__main__":
    main()
