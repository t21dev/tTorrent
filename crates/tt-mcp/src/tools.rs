//! The tools (docs/mcp.md › Tools). Results are JSON with a plain-language
//! `summary` an agent can quote. Sizes are bytes plus text; times ISO 8601.

use crate::{Engine, Scope};
use serde_json::{Value, json};
use tt_ipc::{
    AddOptions, AddSource, Column, QueueMove, Request, Response, SessionStatus, StatusFilter,
    TagMode, TorrentId, TorrentRow, TorrentState, fmt,
};

/// Changes to more torrents than this need the user's yes in the app (docs/mcp.md › Safety).
pub const BULK_LIMIT: usize = 20;

pub enum CallError {
    Unknown,
    Failed(String),
}

impl From<String> for CallError {
    fn from(s: String) -> Self {
        CallError::Failed(s)
    }
}

struct Tool {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    scope: Scope,
    destructive: bool,
    schema: fn() -> Value,
}

fn no_args() -> Value {
    json!({ "type": "object", "additionalProperties": false })
}
fn id_arg() -> Value {
    json!({ "type": "object", "properties": { "id": { "type": "integer", "description": "Torrent id from list_torrents" } }, "required": ["id"], "additionalProperties": false })
}
fn ids_arg() -> Value {
    json!({ "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "integer" }, "description": "Torrent ids from list_torrents" } }, "required": ["ids"], "additionalProperties": false })
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "get_status",
        title: "Status",
        description: "Speeds, counts (active, seeding, errored, total), free disk space, port, slow mode and memory use.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "list_torrents",
        title: "List torrents",
        description: "Torrents with optional filter and sort. status: all, downloading, seeding, completed, stopped, active, inactive, stalled, checking, errored. sort: any column such as added_on, name, size, progress, ratio, uploaded. Pages with limit and cursor.",
        scope: Scope::Read,
        destructive: false,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string" },
                    "category": { "type": "string" },
                    "tag": { "type": "string" },
                    "text": { "type": "string", "description": "Words that must all appear in the name" },
                    "sort": { "type": "string" },
                    "descending": { "type": "boolean" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500 },
                    "cursor": { "type": "string" }
                },
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "get_torrent",
        title: "Torrent details",
        description: "Everything in the details tabs: general figures, trackers (passkeys masked), peers, files.",
        scope: Scope::Read,
        destructive: false,
        schema: id_arg,
    },
    Tool {
        name: "diagnose_torrent",
        title: "Diagnose a torrent",
        description: "Why a torrent is stuck or slow: likely causes in order, each with a fix. Changes nothing.",
        scope: Scope::Read,
        destructive: false,
        schema: id_arg,
    },
    Tool {
        name: "seeding_report",
        title: "Seeding report",
        description: "Per tracker: torrents, uploaded, average ratio, and which torrents are below ratio 1 and still need seeding, private trackers first.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "plan_disk_cleanup",
        title: "Plan disk cleanup",
        description: "Given a target in GB, finished torrents to remove and why (largest and longest seeded first, private torrents below ratio 1 kept). Changes nothing.",
        scope: Scope::Read,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "target_gb": { "type": "number", "minimum": 0 } }, "required": ["target_gb"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "list_categories",
        title: "Categories",
        description: "Categories with counts and save paths.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "list_tags",
        title: "Tags",
        description: "Tags with counts.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "get_settings",
        title: "Settings",
        description: "Settings an agent may read: limits, queueing, seeding limits, connection. Never passwords.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "get_skill",
        title: "How to use tTorrent",
        description: "A short guide for agents on using these tools well.",
        scope: Scope::Read,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "add_torrent",
        title: "Add a torrent",
        description: "Add a magnet link, a URL to a .torrent file, or a local .torrent path. Optional category, tags, save path, paused.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "magnet:…, https://…/x.torrent or a file path" },
                    "category": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "save_path": { "type": "string" },
                    "paused": { "type": "boolean" }
                },
                "required": ["source"],
                "additionalProperties": false
            })
        },
    },
    Tool {
        name: "pause",
        title: "Stop torrents",
        description: "Stop the given torrents.",
        scope: Scope::Manage,
        destructive: false,
        schema: ids_arg,
    },
    Tool {
        name: "resume",
        title: "Start torrents",
        description: "Start the given torrents.",
        scope: Scope::Manage,
        destructive: false,
        schema: ids_arg,
    },
    Tool {
        name: "force_start",
        title: "Force start",
        description: "Start the given torrents now, ignoring the queue.",
        scope: Scope::Manage,
        destructive: false,
        schema: ids_arg,
    },
    Tool {
        name: "pause_all",
        title: "Stop everything",
        description: "Stop all torrents, optionally resuming after until_minutes.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "until_minutes": { "type": "integer", "minimum": 1 } }, "additionalProperties": false
            })
        },
    },
    Tool {
        name: "resume_all",
        title: "Start everything",
        description: "Start all torrents.",
        scope: Scope::Manage,
        destructive: false,
        schema: no_args,
    },
    Tool {
        name: "set_category",
        title: "Set category",
        description: "Set (or clear with an empty string) the category of the given torrents. Creates the category if new.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "integer" } }, "category": { "type": "string" } }, "required": ["ids", "category"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "set_tags",
        title: "Set tags",
        description: "Add, remove or set tags on the given torrents. mode: add, remove or set.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "integer" } }, "tags": { "type": "array", "items": { "type": "string" } }, "mode": { "type": "string", "enum": ["add", "remove", "set"] } }, "required": ["ids", "tags", "mode"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "set_speed_mode",
        title: "Slow mode",
        description: "Turn slow mode (alternative speed limits) on or off.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "slow": { "type": "boolean" } }, "required": ["slow"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "set_limits",
        title: "Speed limits",
        description: "Set global speed limits in KiB/s (0 = unlimited). Omit a value to keep it.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "down_kib": { "type": "integer", "minimum": 0 }, "up_kib": { "type": "integer", "minimum": 0 } }, "additionalProperties": false
            })
        },
    },
    Tool {
        name: "rename",
        title: "Rename",
        description: "Rename a torrent in the list (files on disk keep their names).",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "id": { "type": "integer" }, "name": { "type": "string" } }, "required": ["id", "name"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "queue_move",
        title: "Move in queue",
        description: "Move torrents in the download queue: top, up, down or bottom.",
        scope: Scope::Manage,
        destructive: false,
        schema: || {
            json!({
                "type": "object", "properties": { "ids": { "type": "array", "items": { "type": "integer" } }, "to": { "type": "string", "enum": ["top", "up", "down", "bottom"] } }, "required": ["ids", "to"], "additionalProperties": false
            })
        },
    },
    Tool {
        name: "remove_torrents",
        title: "Remove torrents",
        description: "Remove torrents from the list. Files on disk are kept: deleting files is done by the user in the app.",
        scope: Scope::Manage,
        destructive: true,
        schema: ids_arg,
    },
];

pub fn definitions(scope: Scope) -> Vec<Value> {
    TOOLS
        .iter()
        .filter(|t| t.scope <= scope)
        .map(|t| {
            json!({
                "name": t.name,
                "title": t.title,
                "description": t.description,
                "inputSchema": (t.schema)(),
                "annotations": {
                    "readOnlyHint": t.scope == Scope::Read,
                    "destructiveHint": t.destructive,
                    "openWorldHint": false
                }
            })
        })
        .collect()
}

fn rows(e: &impl Engine) -> Result<(Vec<TorrentRow>, SessionStatus, tt_ipc::Lists), String> {
    match e.call(Request::Snapshot)? {
        Response::Snapshot(s) => Ok((s.rows, s.status, s.lists)),
        _ => Err("unexpected reply from tTorrent".into()),
    }
}

fn iso(unix: i64) -> String {
    // Civil date from days (Howard Hinnant's algorithm), UTC.
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn torrent_json(r: &TorrentRow) -> Value {
    json!({
        "id": r.id,
        "name": r.name,
        "state": r.state.label(),
        "progress_percent": (r.progress * 1000.0).floor() / 10.0,
        "size": r.size,
        "size_text": fmt::size(r.size),
        "down_speed_text": fmt::speed(r.down_bps),
        "up_speed_text": fmt::speed(r.up_bps),
        "eta": r.eta_secs.map(fmt::eta),
        "ratio": (r.ratio * 100.0).round() / 100.0,
        "uploaded": r.uploaded,
        "uploaded_text": fmt::size(r.uploaded),
        "seeds": r.seeds,
        "peers": r.peers,
        "category": r.category,
        "tags": r.tags,
        "tracker": r.tracker,
        "private": r.private,
        "added_on": iso(r.added_on),
        "completed_on": r.completed_on.map(iso),
        "save_path": r.save_path,
        "error": r.error,
    })
}

fn status_filter(s: &str) -> Option<StatusFilter> {
    StatusFilter::ALL
        .into_iter()
        .find(|f| f.label().eq_ignore_ascii_case(s))
}

fn column(s: &str) -> Option<Column> {
    let want = s.replace(['_', ' '], "").to_lowercase();
    Column::ALL
        .into_iter()
        .find(|c| c.label().replace(' ', "").to_lowercase() == want)
}

fn ids_of(args: &Value) -> Result<Vec<TorrentId>, CallError> {
    let ids: Vec<TorrentId> = args
        .get("ids")
        .and_then(Value::as_array)
        .ok_or_else(|| CallError::Failed("ids is required".into()))?
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as TorrentId))
        .collect();
    if ids.len() > BULK_LIMIT {
        return Err(CallError::Failed(format!(
            "That changes {} torrents. Changes to more than {BULK_LIMIT} need the user's confirmation in tTorrent, which this version can't ask for yet; do it in smaller groups after asking the user.",
            ids.len()
        )));
    }
    Ok(ids)
}

fn id_of(args: &Value) -> Result<TorrentId, CallError> {
    args.get("id")
        .and_then(Value::as_u64)
        .map(|n| n as TorrentId)
        .ok_or_else(|| CallError::Failed("id is required".into()))
}

fn done(e: &impl Engine, req: Request, summary: String) -> Result<Value, CallError> {
    e.call(req)?;
    Ok(json!({ "summary": summary }))
}

pub fn call(e: &impl Engine, scope: Scope, name: &str, args: &Value) -> Result<Value, CallError> {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == name)
        .ok_or(CallError::Unknown)?;
    if tool.scope > scope {
        return Err(CallError::Failed(format!(
            "{name} needs the {:?} scope; this connection has {scope:?}. Change it in Settings › AI agents.",
            tool.scope
        )));
    }
    // Off by default: nothing works until the user allows agents.
    if let Response::Settings(s) = e.call(Request::GetSettings)?
        && !s.mcp_enabled
    {
        return Err(CallError::Failed(
            "AI agents are turned off in tTorrent. Ask the user to turn on Settings › AI agents › Allow AI agents to connect.".into(),
        ));
    }
    match name {
        "get_status" => {
            let (rows, s, _) = rows(e)?;
            let count = |f: StatusFilter| rows.iter().filter(|r| f.matches(r)).count();
            let errored = count(StatusFilter::Errored);
            Ok(json!({
                "summary": format!(
                    "{} torrents: {} downloading, {} seeding, {} errored. Down {}, up {}.{}",
                    rows.len(), count(StatusFilter::Downloading), count(StatusFilter::Seeding), errored,
                    fmt::speed(s.down_bps), fmt::speed(s.up_bps),
                    if s.slow_mode { " Slow mode is on." } else { "" }
                ),
                "torrents": rows.len(),
                "downloading": count(StatusFilter::Downloading),
                "seeding": count(StatusFilter::Seeding),
                "active": count(StatusFilter::Active),
                "errored": errored,
                "down_speed_text": fmt::speed(s.down_bps),
                "up_speed_text": fmt::speed(s.up_bps),
                "free_space": s.free_space,
                "free_space_text": s.free_space.map(fmt::size),
                "port": s.port,
                "slow_mode": s.slow_mode,
                "engine_memory_text": fmt::size(s.engine_memory),
            }))
        }
        "list_torrents" => {
            let (mut rows, _, _) = rows(e)?;
            if let Some(st) = args.get("status").and_then(Value::as_str) {
                let f = status_filter(st)
                    .ok_or_else(|| CallError::Failed(format!("Unknown status {st}")))?;
                rows.retain(|r| f.matches(r));
            }
            if let Some(c) = args.get("category").and_then(Value::as_str) {
                rows.retain(|r| r.category.eq_ignore_ascii_case(c));
            }
            if let Some(t) = args.get("tag").and_then(Value::as_str) {
                rows.retain(|r| r.tags.iter().any(|x| x.eq_ignore_ascii_case(t)));
            }
            if let Some(text) = args.get("text").and_then(Value::as_str) {
                let words: Vec<String> = text
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                rows.retain(|r| {
                    let n = r.name.to_lowercase();
                    words.iter().all(|w| n.contains(w))
                });
            }
            let col = match args.get("sort").and_then(Value::as_str) {
                Some(s) => {
                    column(s).ok_or_else(|| CallError::Failed(format!("Unknown sort {s}")))?
                }
                None => Column::AddedOn,
            };
            let desc = args
                .get("descending")
                .and_then(Value::as_bool)
                .unwrap_or(!col.is_text());
            rows.sort_by(|a, b| {
                let o = col.cmp(a, b);
                if desc { o.reverse() } else { o }
            });
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(50)
                .clamp(1, 500) as usize;
            let start: usize = args
                .get("cursor")
                .and_then(Value::as_str)
                .and_then(|c| c.parse().ok())
                .unwrap_or(0);
            let total = rows.len();
            let page: Vec<Value> = rows
                .iter()
                .skip(start)
                .take(limit)
                .map(torrent_json)
                .collect();
            let next = (start + limit < total).then(|| (start + limit).to_string());
            Ok(json!({
                "summary": format!("{total} torrents match; showing {}.", page.len()),
                "total": total,
                "torrents": page,
                "next_cursor": next,
            }))
        }
        "get_torrent" => {
            let id = id_of(args)?;
            let (rows, _, _) = rows(e)?;
            let r = rows
                .iter()
                .find(|r| r.id == id)
                .ok_or_else(|| CallError::Failed(format!("No torrent with id {id}")))?;
            let mut v = torrent_json(r);
            if let Ok(Response::Details(d)) = e.call(Request::Details { id }) {
                v["pieces_have"] = json!(d.pieces_have);
                v["piece_count"] = json!(d.piece_count);
                v["piece_size"] = json!(d.piece_size);
                v["trackers"] = json!(
                    d.trackers
                        .iter()
                        .map(|t| json!({ "url": t.url, "status": t.status, "message": t.message }))
                        .collect::<Vec<_>>()
                );
                v["peers_connected"] = json!(d.peers.len());
                v["files"] = json!(d.files.iter().take(200).map(|f| json!({ "path": f.path, "size": f.size, "progress_percent": (f.progress * 1000.0).floor() / 10.0 })).collect::<Vec<_>>());
            }
            v["summary"] = json!(format!(
                "{} is {} at {}.",
                r.name,
                r.state.label().to_lowercase(),
                fmt::percent(r.progress)
            ));
            Ok(v)
        }
        "diagnose_torrent" => {
            let id = id_of(args)?;
            let (rows, status, _) = rows(e)?;
            let r = rows
                .iter()
                .find(|r| r.id == id)
                .ok_or_else(|| CallError::Failed(format!("No torrent with id {id}")))?;
            let causes = diagnose(r, &status);
            let summary = match causes.first() {
                Some(c) => format!(
                    "{}: most likely {}",
                    r.name,
                    c["cause"].as_str().unwrap_or("")
                ),
                None => format!("{} looks healthy.", r.name),
            };
            Ok(json!({ "summary": summary, "id": id, "state": r.state.label(), "causes": causes }))
        }
        "seeding_report" => {
            let (rows, _, _) = rows(e)?;
            let mut by: std::collections::BTreeMap<String, Vec<&TorrentRow>> = Default::default();
            for r in &rows {
                let k = if r.tracker.is_empty() {
                    "(no tracker)".to_owned()
                } else {
                    r.tracker.clone()
                };
                by.entry(k).or_default().push(r);
            }
            let mut trackers: Vec<Value> = by
                .iter()
                .map(|(t, rs)| {
                    let private = rs.iter().any(|r| r.private);
                    let up: u64 = rs.iter().map(|r| r.uploaded).sum();
                    let avg = rs.iter().map(|r| r.ratio.min(100.0)).sum::<f32>() / rs.len() as f32;
                    let needs: Vec<Value> = rs
                        .iter()
                        .filter(|r| r.progress >= 1.0 && r.ratio < 1.0)
                        .map(|r| json!({ "id": r.id, "name": r.name, "ratio": (r.ratio * 100.0).round() / 100.0 }))
                        .collect();
                    json!({ "tracker": t, "private": private, "torrents": rs.len(), "uploaded_text": fmt::size(up), "average_ratio": (avg * 100.0).round() / 100.0, "below_ratio_1": needs })
                })
                .collect();
            trackers.sort_by_key(|v| !v["private"].as_bool().unwrap_or(false));
            let short: usize = trackers
                .iter()
                .map(|t| t["below_ratio_1"].as_array().map_or(0, Vec::len))
                .sum();
            Ok(json!({
                "summary": format!("{} trackers; {short} finished torrents are below ratio 1.", trackers.len()),
                "trackers": trackers,
            }))
        }
        "plan_disk_cleanup" => {
            let target = args.get("target_gb").and_then(Value::as_f64).unwrap_or(0.0);
            let goal = (target * 1024.0 * 1024.0 * 1024.0) as u64;
            let (rows, _, _) = rows(e)?;
            let mut cands: Vec<&TorrentRow> = rows
                .iter()
                .filter(|r| r.progress >= 1.0 && !(r.private && r.ratio < 1.0))
                .filter(|r| !r.tags.iter().any(|t| t.eq_ignore_ascii_case("keep")))
                .collect();
            // Ratio met first, then largest, then oldest.
            cands.sort_by(|a, b| {
                (b.ratio >= 1.0)
                    .cmp(&(a.ratio >= 1.0))
                    .then(b.size.cmp(&a.size))
                    .then(a.added_on.cmp(&b.added_on))
            });
            let mut picked = Vec::new();
            let mut sum = 0u64;
            for r in cands {
                if sum >= goal {
                    break;
                }
                sum += r.size;
                let why = if r.ratio >= 1.0 {
                    format!("finished, ratio {:.2}", r.ratio)
                } else {
                    "finished, public tracker".into()
                };
                picked.push(json!({ "id": r.id, "name": r.name, "size_text": fmt::size(r.size), "why": why }));
            }
            Ok(json!({
                "summary": format!("{} torrents free {} of the {:.0} GB asked. Nothing was changed; ask the user, then they remove them in the app.", picked.len(), fmt::size(sum), target),
                "freed": sum,
                "freed_text": fmt::size(sum),
                "reaches_target": sum >= goal,
                "torrents": picked,
                "kept_rules": ["private torrents below ratio 1", "torrents tagged Keep", "unfinished torrents"],
            }))
        }
        "list_categories" => {
            let (rows, _, lists) = rows(e)?;
            let cats: Vec<Value> = lists.categories.iter().map(|c| json!({ "name": c.name, "save_path": c.save_path, "torrents": rows.iter().filter(|r| r.category == c.name).count() })).collect();
            Ok(json!({ "summary": format!("{} categories.", cats.len()), "categories": cats }))
        }
        "list_tags" => {
            let (rows, _, lists) = rows(e)?;
            let tags: Vec<Value> = lists.tags.iter().map(|t| json!({ "name": t, "torrents": rows.iter().filter(|r| r.tags.contains(t)).count() })).collect();
            Ok(json!({ "summary": format!("{} tags.", tags.len()), "tags": tags }))
        }
        "get_settings" => match e.call(Request::GetSettings)? {
            Response::Settings(s) => Ok(json!({
                "summary": "Current settings (no passwords).",
                "down_limit_kib": s.down_limit / 1024, "up_limit_kib": s.up_limit / 1024,
                "slow_mode": s.slow_mode, "slow_down_kib": s.alt_down_limit / 1024, "slow_up_kib": s.alt_up_limit / 1024,
                "queueing": s.queueing, "max_active_downloads": s.max_active_downloads,
                "max_active_uploads": s.max_active_uploads, "max_active_torrents": s.max_active_torrents,
                "ratio_limit": s.ratio_limit, "seeding_time_limit_mins": s.seeding_time_limit_mins,
                "max_connections": s.max_connections, "max_connections_per_torrent": s.max_connections_per_torrent,
                "listen_port": s.listen_port, "dht": s.dht, "pex": s.pex, "lsd": s.lsd, "upnp": s.upnp,
                "default_save_path": s.default_save_path,
            })),
            _ => Err(CallError::Failed("unexpected reply".into())),
        },
        "get_skill" => Ok(json!({ "summary": crate::INSTRUCTIONS })),
        "add_torrent" => {
            let src = args
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_owned();
            if src.is_empty() {
                return Err(CallError::Failed("source is required".into()));
            }
            let source = if src.starts_with("magnet:") {
                AddSource::Magnet { uri: src }
            } else if src.starts_with("http://") || src.starts_with("https://") {
                AddSource::Url { url: src }
            } else {
                AddSource::File { path: src }
            };
            let options = AddOptions {
                category: args
                    .get("category")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                tags: args
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
                save_path: args
                    .get("save_path")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                paused: args.get("paused").and_then(Value::as_bool).unwrap_or(false),
                ..Default::default()
            };
            match e.call(Request::Add { source, options })? {
                Response::Added { id, name } => Ok(
                    json!({ "summary": format!("Added {name} (id {id})."), "id": id, "name": name }),
                ),
                _ => Ok(json!({ "summary": "Added." })),
            }
        }
        "pause" => {
            let ids = ids_of(args)?;
            let n = ids.len();
            done(e, Request::Pause { ids }, format!("Stopped {n} torrents."))
        }
        "resume" => {
            let ids = ids_of(args)?;
            let n = ids.len();
            done(e, Request::Resume { ids }, format!("Started {n} torrents."))
        }
        "force_start" => {
            let ids = ids_of(args)?;
            let n = ids.len();
            done(
                e,
                Request::ForceStart { ids },
                format!("Force-started {n} torrents."),
            )
        }
        "pause_all" => match args.get("until_minutes").and_then(Value::as_u64) {
            Some(m) => done(
                e,
                Request::PauseFor { secs: m * 60 },
                format!("Stopped everything; it starts again in {m} minutes."),
            ),
            None => done(e, Request::PauseAll, "Stopped everything.".into()),
        },
        "resume_all" => done(e, Request::ResumeAll, "Started everything.".into()),
        "set_category" => {
            let ids = ids_of(args)?;
            let c = args
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let summary = if c.is_empty() {
                format!("Cleared the category of {} torrents.", ids.len())
            } else {
                format!("Put {} torrents in {c}.", ids.len())
            };
            done(
                e,
                Request::SetCategory {
                    ids,
                    category: Some(c).filter(|c| !c.is_empty()),
                },
                summary,
            )
        }
        "set_tags" => {
            let ids = ids_of(args)?;
            let tags: Vec<String> = args
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let mode = match args.get("mode").and_then(Value::as_str) {
                Some("add") => TagMode::Add,
                Some("remove") => TagMode::Remove,
                Some("set") => TagMode::Set,
                _ => return Err(CallError::Failed("mode must be add, remove or set".into())),
            };
            let summary = format!("Updated tags on {} torrents.", ids.len());
            done(e, Request::SetTags { ids, tags, mode }, summary)
        }
        "set_speed_mode" => {
            let on = args.get("slow").and_then(Value::as_bool).unwrap_or(false);
            done(
                e,
                Request::SetSlowMode { on },
                format!("Slow mode {}.", if on { "on" } else { "off" }),
            )
        }
        "set_limits" => {
            let d = args
                .get("down_kib")
                .and_then(Value::as_u64)
                .map(|k| k * 1024);
            let u = args.get("up_kib").and_then(Value::as_u64).map(|k| k * 1024);
            done(
                e,
                Request::SetLimits {
                    ids: None,
                    down_bps: d,
                    up_bps: u,
                },
                format!(
                    "Limits: down {}, up {}.",
                    d.map(fmt::limit).unwrap_or_else(|| "unchanged".into()),
                    u.map(fmt::limit).unwrap_or_else(|| "unchanged".into())
                ),
            )
        }
        "rename" => {
            let id = id_of(args)?;
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            done(
                e,
                Request::Rename {
                    id,
                    name: name.clone(),
                },
                format!("Renamed {id} to {name}."),
            )
        }
        "queue_move" => {
            let ids = ids_of(args)?;
            let to = match args.get("to").and_then(Value::as_str) {
                Some("top") => QueueMove::Top,
                Some("up") => QueueMove::Up,
                Some("down") => QueueMove::Down,
                Some("bottom") => QueueMove::Bottom,
                _ => {
                    return Err(CallError::Failed(
                        "to must be top, up, down or bottom".into(),
                    ));
                }
            };
            done(
                e,
                Request::QueueMove { ids, to },
                "Moved in the queue.".into(),
            )
        }
        "remove_torrents" => {
            let ids = ids_of(args)?;
            let n = ids.len();
            done(
                e,
                Request::Remove {
                    ids,
                    delete_files: false,
                },
                format!("Removed {n} torrents from the list; their files are still on disk."),
            )
        }
        _ => Err(CallError::Unknown),
    }
}

/// Likely causes for a stuck or slow torrent, most likely first.
pub fn diagnose(r: &TorrentRow, s: &SessionStatus) -> Vec<Value> {
    let mut c = Vec::new();
    let mut add = |cause: &str, fix: &str| c.push(json!({ "cause": cause, "fix": fix }));
    match r.state {
        TorrentState::Error => add(
            &format!(
                "an error: {}",
                r.error.clone().unwrap_or_else(|| "unknown".into())
            ),
            "Check the save path and the drive, then start it again.",
        ),
        TorrentState::MissingFiles => add(
            "its files are missing",
            "Locate the files, or download them again.",
        ),
        TorrentState::DriveUnavailable => add(
            "its drive is unavailable",
            "Connect the drive; it resumes by itself.",
        ),
        TorrentState::Stopped | TorrentState::Completed => add("it is stopped", "Start it."),
        TorrentState::Queued => add(
            &format!(
                "it is waiting in the download queue (position {})",
                r.queue.unwrap_or(0)
            ),
            "Raise the maximum active downloads, move it up, or force start it.",
        ),
        TorrentState::Checking => add("it is checking its files", "Wait for the check to finish."),
        TorrentState::Starting => add("tTorrent is still starting torrents", "Wait a few seconds."),
        _ => {}
    }
    if r.progress < 1.0 {
        if r.seeds == 0 && r.peers == 0 {
            add(
                "no peers are connected, so nobody is sending data",
                "Wait, add trackers, or check that DHT is on. Old or rare torrents may have no seeds at all.",
            );
        } else if r.seeds == 0 {
            add(
                "only partial peers are connected and none has the whole file",
                "Wait for a seed to appear; progress may stop at the availability limit.",
            );
        } else if r.down_bps == 0 && r.state == TorrentState::Stalled {
            add(
                "peers are connected but not sending",
                "Peers may be busy; wait, or check speed limits and slow mode.",
            );
        }
        if let Some(free) = s.free_space
            && free < r.remaining
        {
            add(
                &format!(
                    "the disk has {} free but {} is still needed",
                    fmt::size(free),
                    fmt::size(r.remaining)
                ),
                "Free space or move the torrent to another drive.",
            );
        }
    }
    if s.slow_mode {
        add("slow mode is on", "Turn slow mode off for full speed.");
    }
    if s.port_open == Some(false) {
        add(
            "the listening port is closed",
            "Forward the port in the router or turn on UPnP.",
        );
    }
    c
}
