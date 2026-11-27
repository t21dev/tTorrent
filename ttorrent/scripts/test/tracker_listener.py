# Copyright (C) 2026  t21 dev. GPL v2 or later.
# A stand-in HTTP tracker on 127.0.0.1 that logs every request's path, user
# agent and peer_id, one line each, to stdout and to <log>. It answers with an
# empty peer list. Use it with make_test_torrent.py.
# Usage: python tracker_listener.py <port> <log>

import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs, urlsplit


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        parts = urlsplit(self.path)
        query = parse_qs(parts.query, encoding="latin-1")
        peer_id = query.get("peer_id", [""])[0][:8]
        line = f"{parts.path} ua={self.headers.get('User-Agent', '')!r} peer_id={peer_id!r}"
        print(line, flush=True)
        with open(LOG, "a", encoding="utf8") as f:
            f.write(line + "\n")
        body = b"d8:intervali1800e5:peers0:e" if parts.path.endswith("announce") else b"d5:filesdee"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    LOG = sys.argv[2]
    HTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
