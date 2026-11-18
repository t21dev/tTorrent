"""Makes a small test torrent with a .zip inside, plus its data (ttorrent/VERIFY.md › Unpack).

    python -I ttorrent/scripts/test/make_sample_zip.py <output folder>

Writes "<output>/Elephants Dream (2006)/" (dummy data under an open-movie name)
and "<output>/elephants-sample.torrent". Add the torrent with
--save-path=<output> and it finishes at once, which triggers unpacking.
"""
import hashlib
import io
import os
import sys
import zipfile

if len(sys.argv) < 2:
    sys.exit(__doc__)
root = sys.argv[1]
folder = os.path.join(root, "Elephants Dream (2006)")
os.makedirs(folder, exist_ok=True)
buf = io.BytesIO()
with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("Elephants Dream/Read me.txt", "Elephants Dream, an open movie by the Blender Foundation.\n")
    z.writestr("Elephants Dream/Credits.txt", "Orange Open Movie Project\n" * 50)
files = [("Elephants Dream 720p.mkv", (hashlib.sha256(b"ed").digest() * 40000)[:1024 * 1024]), ("Extras.zip", buf.getvalue())]
for name, blob in files:
    with open(os.path.join(folder, name), "wb") as f:
        f.write(blob)
data = b"".join(b for _, b in files)
piece = 256 * 1024
pieces = b"".join(hashlib.sha1(data[i:i + piece]).digest() for i in range(0, len(data), piece))


def bencode(v):
    if isinstance(v, int):
        return b"i%de" % v
    if isinstance(v, str):
        v = v.encode()
    if isinstance(v, bytes):
        return b"%d:%s" % (len(v), v)
    if isinstance(v, list):
        return b"l" + b"".join(bencode(x) for x in v) + b"e"
    return b"d" + b"".join(bencode(k) + bencode(v[k]) for k in sorted(v)) + b"e"


info = {"name": "Elephants Dream (2006)", "piece length": piece, "pieces": pieces,
        "files": [{"length": len(b), "path": [n]} for n, b in files]}
with open(os.path.join(root, "elephants-sample.torrent"), "wb") as f:
    f.write(bencode({"info": info, "announce": "udp://127.0.0.1:1/announce"}))
print(os.path.join(root, "elephants-sample.torrent"))
