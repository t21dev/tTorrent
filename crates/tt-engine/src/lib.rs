//! tTorrent's engine process: torrents (librqbit), state (SQLite), the local
//! API the window talks to, and the tray. It never loads the GPU (PLAN.md §2).

pub mod core;
pub mod persist;
pub mod server;
pub mod store;
pub mod window;

use std::path::PathBuf;

/// Where an engine instance keeps its files.
#[derive(Clone, Debug)]
pub struct Paths {
    pub data: PathBuf,
    pub db: PathBuf,
    /// librqbit's session file and fast-resume bitfields.
    pub session: PathBuf,
    pub backups: PathBuf,
    pub log: PathBuf,
}

impl Paths {
    pub fn new(data: PathBuf) -> Paths {
        Paths {
            db: data.join("ttorrent.db"),
            session: data.join("session"),
            backups: data.join("backups"),
            log: data.join("ttorrent.log"),
            data,
        }
    }

    pub fn instance(&self) -> String {
        tt_ipc::transport::instance_id(&self.data)
    }
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
