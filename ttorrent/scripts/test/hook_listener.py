"""Receives tTorrent's webhooks and appends each JSON body to a file (ttorrent/VERIFY.md › Webhooks).

    python -I ttorrent/scripts/test/hook_listener.py <log file> [port]

Default port 8799; set the webhook URL to http://127.0.0.1:<port>/hook. Pick a
port nothing else uses, or another server answers instead.
"""
import http.server
import sys

if len(sys.argv) < 2:
    sys.exit(__doc__)
LOG = sys.argv[1]
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 8799


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        with open(LOG, "ab") as f:
            f.write(body + b"\n")
        self.send_response(204)
        self.end_headers()

    def log_message(self, *args):
        pass


http.server.HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
