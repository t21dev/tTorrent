"""Compares two screenshot runs (ttorrent/VERIFY.md › UI changes).

    python -I ttorrent/scripts/test/shotdiff.py <before dir> <after dir> [out dir]

For each PNG in both folders, prints the share of pixels that changed visibly
and whether the size changed; with an out dir, writes side-by-side images of
the screens that changed most, for a closer look.
"""
import os
import sys

from PIL import Image, ImageChops

before, after = sys.argv[1], sys.argv[2]
out = sys.argv[3] if len(sys.argv) > 3 else None
rows = []
for name in sorted(os.listdir(before)):
    if not name.endswith(".png") or not os.path.exists(os.path.join(after, name)):
        continue
    a = Image.open(os.path.join(before, name)).convert("RGB")
    b = Image.open(os.path.join(after, name)).convert("RGB")
    if a.size != b.size:
        rows.append((100.0, name, f"size {a.size} -> {b.size}"))
        continue
    diff = ImageChops.difference(a, b).convert("L").point(lambda v: 255 if v > 24 else 0)
    changed = sum(diff.histogram()[255:]) / (a.size[0] * a.size[1]) * 100
    rows.append((changed, name, ""))
for changed, name, note in sorted(rows, reverse=True):
    print(f"{changed:6.2f}%  {name}  {note}")
if out:
    os.makedirs(out, exist_ok=True)
    for changed, name, _ in sorted(rows, reverse=True)[:8]:
        a = Image.open(os.path.join(before, name)).convert("RGB")
        b = Image.open(os.path.join(after, name)).convert("RGB")
        w = Image.new("RGB", (a.width + b.width + 10, max(a.height, b.height)), (128, 128, 128))
        w.paste(a, (0, 0)); w.paste(b, (a.width + 10, 0))
        w.save(os.path.join(out, name))
