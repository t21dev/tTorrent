//! Everything persistent in one SQLite database (WAL): torrents and their
//! history, categories, settings, window preferences, and the list snapshot
//! the window draws its first frame from (docs/startup.md).
//!
//! Writes are incremental: only torrents that changed, in one transaction.

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use tt_ipc::{Category, TorrentId, TorrentState};

/// One torrent's stored record.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TorrentRecord {
    pub id: TorrentId,
    pub info_hash: String,
    pub name: String,
    pub total_size: u64,
    pub size: u64,
    pub save_path: String,
    pub category: String,
    pub tags: Vec<String>,
    pub added_on: i64,
    pub completed_on: Option<i64>,
    /// The user wants it running (false after Stop). Independent of waves and quit.
    pub running: bool,
    pub uploaded: u64,
    pub downloaded: u64,
    pub time_active_secs: u64,
    pub last_activity: Option<i64>,
    pub progress: f32,
    pub state: TorrentState,
    pub ratio_limit: Option<f32>,
    pub down_limit: u64,
    pub up_limit: u64,
    pub queue_pos: Option<u32>,
    pub private: bool,
    pub created_on: Option<i64>,
    pub comment: String,
    pub created_by: String,
    pub tracker: String,
    /// Force start: runs outside the queue's limits until stopped.
    pub forced: bool,
    /// Seconds spent seeding (for "When total seeding time reaches").
    pub seeding_time_secs: u64,
}

pub struct Store {
    conn: Connection,
}

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS torrents (
    id INTEGER PRIMARY KEY,
    info_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    total_size INTEGER NOT NULL DEFAULT 0,
    size INTEGER NOT NULL DEFAULT 0,
    save_path TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    added_on INTEGER NOT NULL,
    completed_on INTEGER,
    running INTEGER NOT NULL DEFAULT 1,
    uploaded INTEGER NOT NULL DEFAULT 0,
    downloaded INTEGER NOT NULL DEFAULT 0,
    time_active INTEGER NOT NULL DEFAULT 0,
    last_activity INTEGER,
    progress REAL NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'starting',
    ratio_limit REAL,
    down_limit INTEGER NOT NULL DEFAULT 0,
    up_limit INTEGER NOT NULL DEFAULT 0,
    queue_pos INTEGER,
    private INTEGER NOT NULL DEFAULT 0,
    created_on INTEGER,
    comment TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL DEFAULT '',
    tracker TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS categories (
    name TEXT PRIMARY KEY,
    save_path TEXT NOT NULL DEFAULT '',
    color INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Columns added after the first schema; added in place, data kept.
fn migrate(conn: &Connection) -> Result<()> {
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(torrents)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<_>>()?;
    for (name, ddl) in [
        ("forced", "INTEGER NOT NULL DEFAULT 0"),
        ("seeding_time", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if !cols.iter().any(|c| c == name) {
            conn.execute_batch(&format!("ALTER TABLE torrents ADD COLUMN {name} {ddl}"))?;
        }
    }
    Ok(())
}

fn state_str(s: TorrentState) -> String {
    serde_json::to_value(s)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "starting".into())
}

fn state_from(s: &str) -> TorrentState {
    serde_json::from_value(serde_json::Value::String(s.to_owned())).unwrap_or_default()
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        Ok(Store { conn })
    }

    /// SQLite's own check (docs/missing-files.md case 12).
    pub fn integrity_ok(&self) -> bool {
        self.conn
            .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .map(|s| s == "ok")
            .unwrap_or(false)
    }

    /// A consistent copy of the database (daily backups, 7 kept).
    pub fn backup_to(&self, path: &Path) -> Result<()> {
        let _ = std::fs::remove_file(path);
        self.conn
            .execute("VACUUM INTO ?1", params![path.to_string_lossy()])?;
        Ok(())
    }

    pub fn get_kv(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM kv WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_kv(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_torrents(&self) -> Result<Vec<TorrentRecord>> {
        let mut st = self.conn.prepare(
            "SELECT id, info_hash, name, total_size, size, save_path, category, tags, added_on,
                    completed_on, running, uploaded, downloaded, time_active, last_activity,
                    progress, state, ratio_limit, down_limit, up_limit, queue_pos, private,
                    created_on, comment, created_by, tracker, forced, seeding_time
             FROM torrents ORDER BY id",
        )?;
        let rows = st.query_map([], |r| {
            let tags: String = r.get(7)?;
            let state: String = r.get(16)?;
            Ok(TorrentRecord {
                id: r.get(0)?,
                info_hash: r.get(1)?,
                name: r.get(2)?,
                total_size: r.get::<_, i64>(3)? as u64,
                size: r.get::<_, i64>(4)? as u64,
                save_path: r.get(5)?,
                category: r.get(6)?,
                tags: serde_json::from_str(&tags).unwrap_or_default(),
                added_on: r.get(8)?,
                completed_on: r.get(9)?,
                running: r.get(10)?,
                uploaded: r.get::<_, i64>(11)? as u64,
                downloaded: r.get::<_, i64>(12)? as u64,
                time_active_secs: r.get::<_, i64>(13)? as u64,
                last_activity: r.get(14)?,
                progress: r.get::<_, f64>(15)? as f32,
                state: state_from(&state),
                ratio_limit: r.get::<_, Option<f64>>(17)?.map(|v| v as f32),
                down_limit: r.get::<_, i64>(18)? as u64,
                up_limit: r.get::<_, i64>(19)? as u64,
                queue_pos: r.get(20)?,
                private: r.get(21)?,
                created_on: r.get(22)?,
                comment: r.get(23)?,
                created_by: r.get(24)?,
                tracker: r.get(25)?,
                forced: r.get(26)?,
                seeding_time_secs: r.get::<_, i64>(27)? as u64,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Insert or update these records in one transaction.
    pub fn save_torrents(&mut self, recs: &[TorrentRecord]) -> Result<()> {
        if recs.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        {
            let mut st = tx.prepare_cached(
                "INSERT INTO torrents (id, info_hash, name, total_size, size, save_path, category,
                    tags, added_on, completed_on, running, uploaded, downloaded, time_active,
                    last_activity, progress, state, ratio_limit, down_limit, up_limit, queue_pos,
                    private, created_on, comment, created_by, tracker, forced, seeding_time)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
                 ON CONFLICT(id) DO UPDATE SET info_hash = excluded.info_hash,
                    name = excluded.name, total_size = excluded.total_size, size = excluded.size,
                    save_path = excluded.save_path, category = excluded.category,
                    tags = excluded.tags, added_on = excluded.added_on,
                    completed_on = excluded.completed_on, running = excluded.running,
                    uploaded = excluded.uploaded, downloaded = excluded.downloaded,
                    time_active = excluded.time_active, last_activity = excluded.last_activity,
                    progress = excluded.progress, state = excluded.state,
                    ratio_limit = excluded.ratio_limit, down_limit = excluded.down_limit,
                    up_limit = excluded.up_limit, queue_pos = excluded.queue_pos,
                    private = excluded.private, created_on = excluded.created_on,
                    comment = excluded.comment, created_by = excluded.created_by,
                    tracker = excluded.tracker, forced = excluded.forced,
                    seeding_time = excluded.seeding_time",
            )?;
            for t in recs {
                st.execute(params![
                    t.id,
                    t.info_hash,
                    t.name,
                    t.total_size as i64,
                    t.size as i64,
                    t.save_path,
                    t.category,
                    serde_json::to_string(&t.tags)?,
                    t.added_on,
                    t.completed_on,
                    t.running,
                    t.uploaded as i64,
                    t.downloaded as i64,
                    t.time_active_secs as i64,
                    t.last_activity,
                    f64::from(t.progress),
                    state_str(t.state),
                    t.ratio_limit.map(f64::from),
                    t.down_limit as i64,
                    t.up_limit as i64,
                    t.queue_pos,
                    t.private,
                    t.created_on,
                    t.comment,
                    t.created_by,
                    t.tracker,
                    t.forced,
                    t.seeding_time_secs as i64,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_torrent(&self, id: TorrentId) -> Result<()> {
        self.conn
            .execute("DELETE FROM torrents WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn categories(&self) -> Result<Vec<Category>> {
        let mut st = self.conn.prepare(
            "SELECT name, save_path, color FROM categories ORDER BY name COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Category {
                name: r.get(0)?,
                save_path: r.get(1)?,
                color: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_category(&self, c: &Category) -> Result<()> {
        self.conn.execute(
            "INSERT INTO categories (name, save_path, color) VALUES (?1, ?2, ?3)
             ON CONFLICT(name) DO UPDATE SET save_path = excluded.save_path",
            params![c.name, c.save_path, c.color],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: u32) -> TorrentRecord {
        TorrentRecord {
            id,
            info_hash: format!("{id:040x}"),
            name: format!("archlinux-2026.11.01-x86_64-{id}.iso"),
            total_size: 1_400_000_000,
            size: 1_400_000_000,
            tags: vec!["Keep".into()],
            added_on: 1_763_400_000,
            running: true,
            progress: 0.5,
            state: TorrentState::Downloading,
            ..Default::default()
        }
    }

    #[test]
    fn round_trip_and_update() {
        let mut s = Store::open_in_memory().unwrap();
        s.save_torrents(&[rec(1), rec(2)]).unwrap();
        let mut r = rec(1);
        r.progress = 1.0;
        r.state = TorrentState::Seeding;
        s.save_torrents(&[r.clone()]).unwrap();
        let all = s.load_torrents().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], r);
        s.delete_torrent(2).unwrap();
        assert_eq!(s.load_torrents().unwrap().len(), 1);
    }

    #[test]
    fn kv_and_categories() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.get_kv("x").unwrap(), None);
        s.set_kv("x", "1").unwrap();
        s.set_kv("x", "2").unwrap();
        assert_eq!(s.get_kv("x").unwrap().as_deref(), Some("2"));
        s.upsert_category(&Category {
            name: "Linux ISOs".into(),
            save_path: String::new(),
            color: 0,
        })
        .unwrap();
        assert_eq!(s.categories().unwrap().len(), 1);
        assert!(s.integrity_ok());
    }

    proptest::proptest! {
        #[test]
        fn records_round_trip(up in 0u64..(1 << 50), prog in 0.0f32..=1.0, name in "\\PC{1,40}") {
            let mut s = Store::open_in_memory().unwrap();
            let mut r = rec(5);
            r.uploaded = up;
            r.progress = prog;
            r.name = name;
            s.save_torrents(&[r.clone()]).unwrap();
            let back = s.load_torrents().unwrap().remove(0);
            proptest::prop_assert_eq!(back, r);
        }
    }
}
