//! The MCP server (docs/mcp.md). `ttorrent --mcp` runs it on stdio: one
//! JSON-RPC message per line on stdin and stdout, logs on stderr. It is a
//! separate small process that talks to the engine over its local API, so
//! nothing of it loads into the engine.
//!
//! Protocol: answers both eras. 2026-07-28 clients send `server/discover`
//! and per-request `_meta`; 2025 clients send `initialize` first.

pub mod tools;

use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub const MODERN: &str = "2026-07-28";
pub const LEGACY: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// What a key or the `--scope` flag allows (docs/mcp.md › Scopes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Read,
    Manage,
    Full,
}

impl Scope {
    pub fn parse(s: &str) -> Option<Scope> {
        match s {
            "read" => Some(Scope::Read),
            "manage" => Some(Scope::Manage),
            "full" => Some(Scope::Full),
            _ => None,
        }
    }
}

/// Where tool calls go: the engine, or a fake in tests.
pub trait Engine {
    fn call(&self, req: tt_ipc::Request) -> Result<tt_ipc::Response, String>;
}

impl Engine for tt_ipc::transport::Client {
    fn call(&self, req: tt_ipc::Request) -> Result<tt_ipc::Response, String> {
        tt_ipc::transport::Client::call(self, req)
    }
}

pub struct Server<E: Engine> {
    pub engine: E,
    pub scope: Scope,
}

const INSTRUCTIONS: &str = "tTorrent is a desktop torrent client. Use get_status for an overview, \
list_torrents to find torrents (filter by status, category, tag or text), diagnose_torrent when \
one is stuck, plan_disk_cleanup before suggesting removals, and seeding_report before removing \
anything from private trackers. Removing never deletes files from here; the user does that in \
the app. Ask the user before changing more than a few torrents.";

fn error(id: &Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut e = json!({ "code": code, "message": message });
    if let Some(d) = data {
        e["data"] = d;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": e })
}

fn ok(id: &Value, mut result: Value) -> Value {
    result["resultType"] = json!("complete");
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

impl<E: Engine> Server<E> {
    /// Handle one message; `None` for notifications.
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id")?.clone();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let all: Vec<&str> = std::iter::once(MODERN).chain(LEGACY).collect();
        // Modern requests name their version in _meta; refuse ones we don't speak.
        if let Some(v) = params
            .pointer("/_meta/io.modelcontextprotocol~1protocolVersion")
            .and_then(Value::as_str)
            && !all.contains(&v)
        {
            return Some(error(
                &id,
                -32022,
                "Unsupported protocol version",
                Some(json!({ "supported": all, "requested": v })),
            ));
        }
        let info = json!({ "name": "ttorrent", "title": "tTorrent", "version": env!("CARGO_PKG_VERSION") });
        Some(match method {
            "initialize" => {
                let asked = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or(LEGACY[0]);
                let v = if LEGACY.contains(&asked) {
                    asked
                } else {
                    LEGACY[0]
                };
                ok(
                    &id,
                    json!({
                        "protocolVersion": v,
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": info,
                        "instructions": INSTRUCTIONS,
                    }),
                )
            }
            "server/discover" => ok(
                &id,
                json!({
                    "supportedVersions": all,
                    "capabilities": { "tools": {} },
                    "_meta": { "io.modelcontextprotocol/serverInfo": info },
                    "instructions": INSTRUCTIONS,
                }),
            ),
            "ping" => ok(&id, json!({})),
            "tools/list" => ok(&id, json!({ "tools": tools::definitions(self.scope) })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match tools::call(&self.engine, self.scope, name, &args) {
                    Err(tools::CallError::Unknown) => {
                        error(&id, -32602, &format!("Unknown tool: {name}"), None)
                    }
                    Err(tools::CallError::Failed(msg)) => ok(
                        &id,
                        json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
                    ),
                    Ok(v) => ok(
                        &id,
                        json!({
                            "content": [{ "type": "text", "text": v.to_string() }],
                            "structuredContent": v,
                            "isError": false,
                        }),
                    ),
                }
            }
            _ => error(&id, -32601, &format!("Method not found: {method}"), None),
        })
    }

    /// Serve stdin/stdout until stdin closes.
    pub fn serve_stdio(&self) -> anyhow::Result<()> {
        let stdin = std::io::stdin();
        let mut out = std::io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(&line) {
                Ok(Value::Array(batch)) => {
                    let r: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
                    (!r.is_empty()).then(|| Value::Array(r))
                }
                Ok(msg) => self.handle(&msg),
                Err(_) => Some(error(&Value::Null, -32700, "Parse error", None)),
            };
            if let Some(r) = reply {
                writeln!(out, "{r}")?;
                out.flush()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
