# Copyright (C) 2026  t21 dev. GPL v2 or later.
# Writes a small single-file .torrent (and its data file) for tests, with a
# tracker of your choice. --private sets the private flag, like a private
# tracker's torrents. Pair it with tracker_listener.py to see what tTorrent
# sends to trackers.
# Usage: python make_test_torrent.py <out dir> <announce url> [--private]

import hashlib
import sys
from pathlib import Path


def bencode(value) -> bytes:
    if isinstance(value, int):
        return b"i%de" % value
    if isinstance(value, str):
        value = value.encode()
    if isinstance(value, bytes):
        return b"%d:%s" % (len(value), value)
    if isinstance(value, list):
        return b"l" + b"".join(bencode(v) for v in value) + b"e"
    if isinstance(value, dict):
        items = sorted((k.encode() if isinstance(k, str) else k, v) for k, v in value.items())
        return b"d" + b"".join(bencode(k) + bencode(v) for k, v in items) + b"e"
    raise TypeError(type(value))


def main() -> None:
    out = Path(sys.argv[1])
    announce = sys.argv[2]
    private = "--private" in sys.argv[3:]
    out.mkdir(parents=True, exist_ok=True)
    name = "private-sample.bin" if private else "public-sample.bin"
    data = bytes(range(256)) * 1024  # 256 KiB
    (out / name).write_bytes(data)
    piece = 64 * 1024
    pieces = b"".join(hashlib.sha1(data[i:i + piece]).digest() for i in range(0, len(data), piece))
    info = {"name": name, "length": len(data), "piece length": piece, "pieces": pieces}
    if private:
        info["private"] = 1
    torrent = out / (name + ".torrent")
    torrent.write_bytes(bencode({"announce": announce, "info": info}))
    print(torrent)


if __name__ == "__main__":
    main()
