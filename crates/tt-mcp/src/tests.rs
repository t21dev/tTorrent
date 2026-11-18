use super::*;
use std::cell::RefCell;
use tt_ipc::{EngineSettings, Request, Response, Snapshot, TorrentRow, TorrentState, UiPrefs};

struct Fake {
    enabled: bool,
    rows: Vec<TorrentRow>,
    seen: RefCell<Vec<Request>>,
}

impl Engine for Fake {
    fn call(&self, req: Request) -> Result<Response, String> {
        self.seen.borrow_mut().push(req.clone());
        Ok(match req {
            Request::GetSettings => Response::Settings(Box::new(EngineSettings {
                mcp_enabled: self.enabled,
                ..Default::default()
            })),
            Request::Snapshot => Response::Snapshot(Box::new(Snapshot {
                rows: self.rows.clone(),
                status: Default::default(),
                lists: Default::default(),
                prefs: UiPrefs::default(),
                from_snapshot: false,
                pending: Vec::new(),
            })),
            _ => Response::Ok,
        })
    }
}

fn row(id: u32, name: &str, size_gb: u64, ratio: f32, private: bool) -> TorrentRow {
    TorrentRow {
        id,
        name: name.into(),
        size: size_gb << 30,
        progress: 1.0,
        state: TorrentState::Seeding,
        ratio,
        private,
        ..Default::default()
    }
}

fn server(enabled: bool, scope: Scope) -> Server<Fake> {
    Server {
        engine: Fake {
            enabled,
            rows: vec![
                row(1, "debian-13.2.0-amd64-DVD-1.iso", 4, 2.0, false),
                row(2, "Big Buck Bunny (2008) 4K.mp4", 1, 0.2, true),
                row(3, "archlinux-2026.11.01-x86_64.iso", 2, 0.5, false),
            ],
            seen: RefCell::default(),
        },
        scope,
    }
}

fn req(method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
}

#[test]
fn legacy_initialize_echoes_supported_version() {
    let s = server(true, Scope::Read);
    let r = s
        .handle(&req(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {} }),
        ))
        .unwrap();
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    assert!(r["result"]["capabilities"]["tools"].is_object());
    let r = s
        .handle(&req(
            "initialize",
            json!({ "protocolVersion": "2099-01-01" }),
        ))
        .unwrap();
    assert_eq!(r["result"]["protocolVersion"], LEGACY[0]);
    assert!(
        s.handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .is_none()
    );
}

#[test]
fn modern_discover_and_version_check() {
    let s = server(true, Scope::Read);
    let meta = json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": MODERN, "io.modelcontextprotocol/clientCapabilities": {} } });
    let r = s.handle(&req("server/discover", meta)).unwrap();
    assert_eq!(r["result"]["resultType"], "complete");
    assert!(
        r["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!(MODERN))
    );
    let bad = json!({ "_meta": { "io.modelcontextprotocol/protocolVersion": "1900-01-01" } });
    let r = s.handle(&req("tools/list", bad)).unwrap();
    assert_eq!(r["error"]["code"], -32022);
}

#[test]
fn scopes_limit_the_tool_list_and_calls() {
    let read = server(true, Scope::Read);
    let r = read.handle(&req("tools/list", json!({}))).unwrap();
    let names: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"diagnose_torrent") && !names.contains(&"pause"));
    let r = read
        .handle(&req(
            "tools/call",
            json!({ "name": "pause", "arguments": { "ids": [1] } }),
        ))
        .unwrap();
    assert_eq!(r["result"]["isError"], true);
    let manage = server(true, Scope::Manage);
    let r = manage
        .handle(&req(
            "tools/call",
            json!({ "name": "pause", "arguments": { "ids": [1] } }),
        ))
        .unwrap();
    assert_eq!(r["result"]["isError"], false);
    assert!(
        manage
            .engine
            .seen
            .borrow()
            .contains(&Request::Pause { ids: vec![1] })
    );
}

#[test]
fn off_by_default_refuses_every_tool() {
    let s = server(false, Scope::Full);
    let r = s
        .handle(&req(
            "tools/call",
            json!({ "name": "get_status", "arguments": {} }),
        ))
        .unwrap();
    assert_eq!(r["result"]["isError"], true);
    assert!(
        r["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("turned off")
    );
}

#[test]
fn removing_never_deletes_files_and_bulk_is_refused() {
    let s = server(true, Scope::Full);
    s.handle(&req(
        "tools/call",
        json!({ "name": "remove_torrents", "arguments": { "ids": [1] } }),
    ));
    assert!(s.engine.seen.borrow().contains(&Request::Remove {
        ids: vec![1],
        delete_files: false
    }));
    let many: Vec<u32> = (0..25).collect();
    let r = s
        .handle(&req(
            "tools/call",
            json!({ "name": "pause", "arguments": { "ids": many } }),
        ))
        .unwrap();
    assert_eq!(r["result"]["isError"], true);
}

#[test]
fn cleanup_plan_keeps_private_below_ratio_one() {
    let s = server(true, Scope::Read);
    let r = s
        .handle(&req(
            "tools/call",
            json!({ "name": "plan_disk_cleanup", "arguments": { "target_gb": 10 } }),
        ))
        .unwrap();
    let picked: Vec<u64> = r["result"]["structuredContent"]["torrents"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["id"].as_u64())
        .collect();
    assert_eq!(picked, vec![1, 3]);
    assert_eq!(r["result"]["structuredContent"]["reaches_target"], false);
}

#[test]
fn unknown_tool_is_a_protocol_error() {
    let s = server(true, Scope::Read);
    let r = s
        .handle(&req("tools/call", json!({ "name": "nope" })))
        .unwrap();
    assert_eq!(r["error"]["code"], -32602);
}

#[test]
fn iso_dates() {
    assert_eq!(tools_iso(0), "1970-01-01T00:00:00Z");
    assert_eq!(tools_iso(1_763_400_000), "2025-11-17T17:20:00Z");
}

fn tools_iso(t: i64) -> String {
    // Through list_torrents' JSON so the private helper is covered.
    let s = Server {
        engine: Fake {
            enabled: true,
            rows: vec![TorrentRow {
                id: 9,
                added_on: t,
                ..Default::default()
            }],
            seen: RefCell::default(),
        },
        scope: Scope::Read,
    };
    let r = s
        .handle(&req(
            "tools/call",
            json!({ "name": "list_torrents", "arguments": {} }),
        ))
        .unwrap();
    r["result"]["structuredContent"]["torrents"][0]["added_on"]
        .as_str()
        .unwrap()
        .to_owned()
}
