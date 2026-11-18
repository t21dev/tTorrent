"""Checks tTorrent's MCP server end to end (ttorrent/VERIFY.md › MCP).

Start tTorrent in a scratch profile with MCP on and a known key, then:
    python -I ttorrent/scripts/test/mcp_check.py <key> [--manage]
--manage also stops and starts the first torrent (needs scope "manage").
Prints one line per check and exits 1 on the first failure.
"""
import json
import sys
import urllib.error
import urllib.request

URL = "http://127.0.0.1:7812/mcp"
MODERN = "2026-07-28"


def post(body, headers, expect=200):
    req = urllib.request.Request(URL, json.dumps(body).encode(), headers)
    try:
        with urllib.request.urlopen(req) as r:
            status, data = r.status, r.read()
    except urllib.error.HTTPError as e:
        status, data = e.code, e.read()
    if status != expect:
        sys.exit(f"FAIL: expected HTTP {expect}, got {status}: {data[:200]!r}")
    return json.loads(data.decode("utf8")) if data else None


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    key, manage = sys.argv[1], "--manage" in sys.argv
    base = {"Authorization": f"Bearer {key}", "Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    meta = {"io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientInfo": {"name": "mcp_check", "version": "1"},
            "io.modelcontextprotocol/clientCapabilities": {}}
    n = [0]

    def call(method, params=None, name=None, expect=200, headers=None):
        n[0] += 1
        p = dict(params or {}, _meta=meta)
        h = dict(base, **{"MCP-Protocol-Version": MODERN, "Mcp-Method": method})
        if name:
            h["Mcp-Name"] = name
        h.update(headers or {})
        return post({"jsonrpc": "2.0", "id": n[0], "method": method, "params": p}, h, expect)

    def tool(name, args=None):
        result = call("tools/call", {"name": name, "arguments": args or {}}, name)["result"]
        if result.get("isError"):
            sys.exit(f"FAIL: {name}: {result['content'][0]['text']}")
        return result["structuredContent"]

    legacy = post({"jsonrpc": "2.0", "id": 0, "method": "initialize",
                   "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "mcp_check", "version": "1"}}}, base)
    print("ok  legacy initialize ->", legacy["result"]["protocolVersion"])
    print("ok  server/discover ->", call("server/discover")["result"]["supportedVersions"])
    print("ok  tools/list ->", len(call("tools/list")["result"]["tools"]), "tools")
    print("ok  get_status ->", tool("get_status")["summary"])
    torrents = tool("list_torrents", {"limit": 3})["torrents"]
    print("ok  list_torrents ->", len(torrents))
    if torrents:
        tid = torrents[0]["id"]
        print("ok  diagnose_torrent ->", len(tool("diagnose_torrent", {"id": tid})["causes"]), "causes")
        for t in tool("get_torrent", {"id": tid})["trackers"]:
            if "?" in t["tracker"] or "/announce" in t["tracker"]:
                sys.exit("FAIL: tracker URL not masked: " + t["tracker"])
        print("ok  trackers masked")
        if manage:
            tool("stop_torrents", {"ids": [tid]})
            tool("start_torrents", {"ids": [tid]})
            print("ok  stop and start")
    call("tools/list", expect=400, headers={"MCP-Protocol-Version": "1900-01-01"})
    print("ok  header mismatch -> 400")
    post({}, dict(base, Authorization="Bearer wrong"), 401)
    print("ok  wrong key -> 401")
    post({}, dict(base, Origin="http://evil.example"), 403)
    print("ok  foreign Origin -> 403")
    print("all checks passed")


if __name__ == "__main__":
    main()
