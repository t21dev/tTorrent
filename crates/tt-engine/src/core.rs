//! The engine: one librqbit session, the store, and everything the window,
//! tray, Web API and MCP server ask of it.

use crate::store::{Store, TorrentRecord};
use crate::{Paths, now_unix};
use anyhow::{Context, Result, anyhow};
use librqbit::{
    Api, ManagedTorrent, Session, SessionOptions, SessionPersistenceConfig, TorrentStatsState,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tt_ipc::{
    AddOptions, AddSource, Category, EngineSettings, Event, FilePriority, FileRow, Lists, PeerRow,
    QueueMove, Request, Response, SessionStatus, Snapshot, SpeedPoint, TagMode, TorrentDetails,
    TorrentId, TorrentRow, TorrentState, TrackerRow, UiPrefs,
};

/// docs/startup.md: about 25 torrents per wave, a wave every 1.5 s.
pub const WAVE_SIZE: usize = 25;
pub const WAVE_EVERY: Duration = Duration::from_millis(1500);
/// Seconds of speed history kept for the Speed tab.
const SPEED_HISTORY: usize = 300;

mod adding;
pub use adding::{content_folder, safe_name};

#[derive(Default)]
struct Live {
    /// Running in librqbit during this engine session (a wave reached it, or it was added).
    started: bool,
    /// Held back by the download queue.
    queued: bool,
    prev_up: u64,
    prev_down: u64,
    session_up: u64,
    session_down: u64,
    slow_since: Option<Instant>,
    /// Stop condition "Files checked": pause once the check is over.
    stop_when_checked: bool,
}

struct Waves {
    order: VecDeque<TorrentId>,
    total: u32,
    done: u32,
}

#[derive(Default)]
struct Inner {
    recs: HashMap<TorrentId, TorrentRecord>,
    live: HashMap<TorrentId, Live>,
    dirty: HashSet<TorrentId>,
    last_rows: HashMap<TorrentId, TorrentRow>,
    status: SessionStatus,
    lists: Lists,
    settings: EngineSettings,
    prefs: UiPrefs,
    speed: VecDeque<SpeedPoint>,
    waves: Option<Waves>,
    resume_at: Option<Instant>,
    session_up: u64,
    session_down: u64,
    /// Torrents waiting in the Add torrent dialog.
    pending: Vec<adding::Pending>,
    next_token: u64,
}

pub struct Engine {
    pub rt: tokio::runtime::Handle,
    pub paths: Paths,
    session: OnceLock<Arc<Session>>,
    api: OnceLock<Api>,
    store: Mutex<Store>,
    inner: Mutex<Inner>,
    subscribers: Mutex<Vec<std::sync::mpsc::Sender<Event>>>,
    /// Set when the engine should exit (tray › Exit, Ctrl+Q).
    pub exit: tokio::sync::Notify,
    pub window: crate::window::WindowCtl,
    /// What the tray shows; read by the tray thread once a second.
    pub tray: tt_platform::tray::Shared,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Engine {
    /// Open the store and prepare the snapshot. Cheap: no torrent is touched yet,
    /// so the window can draw the list right away (docs/startup.md).
    pub fn open(paths: Paths, rt: tokio::runtime::Handle) -> Result<Arc<Engine>> {
        std::fs::create_dir_all(&paths.data)?;
        let store = Store::open(&paths.db)?;
        if !store.integrity_ok() {
            tracing::error!("database failed its integrity check; see backups/");
        }
        let settings: EngineSettings = store
            .get_kv("settings")?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let prefs: UiPrefs = store
            .get_kv("ui_prefs")?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let recs: HashMap<_, _> = store
            .load_torrents()?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        let lists = Lists {
            categories: store.categories()?,
            tags: collect_tags(recs.values()),
        };
        let inner = Inner {
            recs,
            lists,
            settings,
            prefs,
            ..Default::default()
        };
        Ok(Arc::new(Engine {
            rt,
            window: crate::window::WindowCtl::new(&paths),
            paths,
            session: OnceLock::new(),
            api: OnceLock::new(),
            store: Mutex::new(store),
            inner: Mutex::new(inner),
            subscribers: Mutex::new(Vec::new()),
            exit: tokio::sync::Notify::new(),
            tray: Default::default(),
        }))
    }

    pub fn settings(&self) -> EngineSettings {
        lock(&self.inner).settings.clone()
    }

    /// `--offline`: no DHT, LSD, UPnP or trackers (benchmarks, fixtures).
    pub fn set_offline(&self) {
        lock(&self.inner).settings.offline = true;
    }

    fn session(&self) -> Result<&Arc<Session>> {
        self.session
            .get()
            .ok_or_else(|| anyhow!("the engine is still starting"))
    }

    fn api(&self) -> Result<&Api> {
        self.api
            .get()
            .ok_or_else(|| anyhow!("the engine is still starting"))
    }

    /// Start librqbit, restore torrents (all paused), then start them in waves.
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        let mut settings = self.settings();
        if settings.listen_port == 0 {
            settings.listen_port = 20000 + (std::process::id() % 40000) as u16;
            lock(&self.inner).settings.listen_port = settings.listen_port;
            self.save_settings()?;
        }
        let save_path = if settings.default_save_path.is_empty() {
            tt_platform::downloads_dir()
        } else {
            settings.default_save_path.clone().into()
        };
        // Restored torrents come back paused; waves start them (docs/startup.md §2).
        crate::persist::mark_all_paused(&self.paths.session)?;
        let opts = session_options(&settings, &self.paths);
        let t0 = Instant::now();
        let session = Session::new_with_opts(save_path, opts)
            .await
            .context("starting the BitTorrent session")?;
        tracing::info!(ms = t0.elapsed().as_millis() as u64, "session restored");
        apply_limits(&session, &settings);
        let _ = self.api.set(Api::new(session.clone(), None));
        let _ = self.session.set(session.clone());
        self.reconcile(&session);
        self.begin_waves();
        let me = self.clone();
        self.rt.spawn(async move { me.tick_loop().await });
        Ok(())
    }

    /// Match librqbit's restored torrents with the store.
    fn reconcile(&self, session: &Arc<Session>) {
        let found = all_handles(session);
        let mut inner = lock(&self.inner);
        let mut seen = HashSet::new();
        for (id, h) in &found {
            seen.insert(*id);
            let hash = h.info_hash().as_string();
            match inner.recs.get_mut(id) {
                Some(r) if r.info_hash == hash => {}
                _ => {
                    // In the session but not the store (an add interrupted by a crash).
                    let rec = new_record(*id, h, now_unix(), &AddOptions::default());
                    inner.recs.insert(*id, rec);
                    inner.dirty.insert(*id);
                }
            }
        }
        let Inner { recs, dirty, .. } = &mut *inner;
        for (id, r) in recs.iter_mut() {
            if !seen.contains(id) && r.state != TorrentState::Error {
                r.state = TorrentState::Error;
                dirty.insert(*id);
            }
        }
        for id in seen {
            inner.live.entry(id).or_default();
        }
    }

    /// Wave order: downloading, then seeds with recent upload, then idle seeds.
    fn begin_waves(&self) {
        let mut inner = lock(&self.inner);
        let mut ids: Vec<&TorrentRecord> = inner
            .recs
            .values()
            .filter(|r| r.running && inner.live.contains_key(&r.id))
            .collect();
        ids.sort_by_key(|r| {
            (
                r.progress >= 1.0,
                std::cmp::Reverse(r.last_activity.unwrap_or(0)),
                r.id,
            )
        });
        let order: VecDeque<TorrentId> = ids.iter().map(|r| r.id).collect();
        let total = order.len() as u32;
        if inner.settings.start_gradually {
            inner.waves = Some(Waves {
                order,
                total,
                done: 0,
            });
        } else {
            drop(inner);
            for id in order {
                self.start_one(id);
            }
        }
    }

    fn handle_of(&self, id: TorrentId) -> Result<Arc<ManagedTorrent>> {
        self.session()?
            .get((id as usize).into())
            .ok_or_else(|| anyhow!("no torrent {id}"))
    }

    fn start_one(&self, id: TorrentId) {
        let Ok(h) = self.handle_of(id) else { return };
        if let Some(l) = lock(&self.inner).live.get_mut(&id) {
            l.started = true;
        }
        if h.is_paused()
            && let Ok(s) = self.session()
        {
            let s = s.clone();
            self.rt.spawn(async move {
                if let Err(e) = s.unpause(&h).await {
                    tracing::warn!("start {id}: {e:#}");
                }
            });
        }
    }

    fn pause_one(&self, id: TorrentId) {
        let Ok(h) = self.handle_of(id) else { return };
        if !h.is_paused()
            && let Ok(s) = self.session()
        {
            let s = s.clone();
            self.rt.spawn(async move {
                if let Err(e) = s.pause(&h).await {
                    tracing::warn!("pause {id}: {e:#}");
                }
            });
        }
    }

    async fn tick_loop(self: Arc<Self>) {
        // TT_TICK_MS overrides the period for perf experiments (docs/perf/README.md).
        let tick_ms = std::env::var("TT_TICK_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1000);
        let mut every = tokio::time::interval(Duration::from_millis(tick_ms));
        let mut wave = tokio::time::interval(WAVE_EVERY);
        let mut save = tokio::time::interval(Duration::from_secs(120));
        loop {
            tokio::select! {
                _ = every.tick() => {
                    // Seeding limits whose action is to remove the torrent.
                    for (id, delete_files) in self.tick() {
                        let name = lock(&self.inner).recs.get(&id).map(|r| r.name.clone());
                        match self.handle(Request::Remove { ids: vec![id], delete_files }).await {
                            Ok(_) => self.broadcast(Event::Notice {
                                text: format!(
                                    "Removed {} (seeding limit reached)",
                                    name.unwrap_or_default()
                                ),
                            }),
                            Err(e) => tracing::warn!("seeding limit, removing {id}: {e:#}"),
                        }
                    }
                }
                _ = wave.tick() => self.next_wave(),
                _ = save.tick() => { let _ = self.flush(); }
            }
        }
    }

    fn next_wave(&self) {
        let batch: Vec<TorrentId> = {
            let mut inner = lock(&self.inner);
            let Some(w) = inner.waves.as_mut() else {
                return;
            };
            let n = WAVE_SIZE.min(w.order.len());
            let b: Vec<_> = w.order.drain(..n).collect();
            w.done += b.len() as u32;
            if w.order.is_empty() {
                inner.waves = None;
            }
            b
        };
        for id in batch {
            self.start_one(id);
        }
    }

    /// Once a second: rows, status, queue, ratio limits; send what changed.
    /// Once a second. Returns torrents a seeding limit wants removed
    /// (id, delete files), which need the async remove.
    fn tick(&self) -> Vec<(TorrentId, bool)> {
        let Ok(session) = self.session() else {
            return Vec::new();
        };
        let handles = all_handles(session);
        let now = now_unix();
        let mut inner = lock(&self.inner);
        if inner.resume_at.is_some_and(|t| Instant::now() >= t) {
            inner.resume_at = None;
            for r in inner.recs.values_mut() {
                r.running = true;
            }
            drop(inner);
            for (id, _) in &handles {
                self.start_one(*id);
            }
            inner = lock(&self.inner);
        }
        let cfg = inner.settings.clone();
        let mut rows = Vec::with_capacity(handles.len());
        let mut to_pause = Vec::new();
        let mut to_remove = Vec::new();
        let (mut down, mut up, mut peers, mut active) = (0u64, 0u64, 0u32, 0u32);
        let (mut sess_up, mut sess_down) = (0u64, 0u64);
        for (id, h) in &handles {
            let st = h.stats();
            let Inner {
                recs, live, dirty, ..
            } = &mut *inner;
            let Some(rec) = recs.get_mut(id) else {
                continue;
            };
            let l = live.entry(*id).or_default();
            let (dbps, ubps, eta, seeds, peers_live, peers_seen) = match &st.live {
                Some(ls) => (
                    mbps_to_bps(ls.download_speed.mbps),
                    mbps_to_bps(ls.upload_speed.mbps),
                    None::<u64>,
                    ls.snapshot.peer_stats.live_seeders,
                    ls.snapshot.peer_stats.live,
                    ls.snapshot.peer_stats.seen,
                ),
                None => (0, 0, None, 0, 0, 0),
            };
            // Totals from librqbit's per-run counters (they restart on resume).
            if let Some(ls) = &st.live {
                let (u, d) = (ls.snapshot.uploaded_bytes, ls.snapshot.fetched_bytes);
                let du = if u >= l.prev_up { u - l.prev_up } else { u };
                let dd = if d >= l.prev_down { d - l.prev_down } else { d };
                l.prev_up = u;
                l.prev_down = d;
                if du + dd > 0 {
                    rec.uploaded += du;
                    rec.downloaded += dd;
                    l.session_up += du;
                    l.session_down += dd;
                    sess_up += du;
                    sess_down += dd;
                    rec.last_activity = Some(now);
                    dirty.insert(*id);
                }
                rec.time_active_secs += 1;
            } else {
                l.prev_up = 0;
                l.prev_down = 0;
            }
            let eta = eta.or_else(|| {
                (dbps > 0).then(|| st.total_bytes.saturating_sub(st.progress_bytes) / dbps)
            });
            let total = st.total_bytes.max(1);
            let progress = (st.progress_bytes as f64 / total as f64) as f32;
            if st.finished && rec.completed_on.is_none() {
                rec.completed_on = Some(now);
                rec.queue_pos = None;
                dirty.insert(*id);
            }
            let state = derive_state(rec, l, &st.state, st.finished, dbps, st.error.is_some());
            if l.stop_when_checked
                && !matches!(
                    state,
                    TorrentState::Checking | TorrentState::Starting | TorrentState::Metadata
                )
            {
                l.stop_when_checked = false;
                rec.running = false;
                to_pause.push(*id);
                dirty.insert(*id);
            }
            if (rec.progress - progress).abs() > 0.0005 || rec.state != state {
                rec.progress = progress;
                rec.state = state;
                dirty.insert(*id);
            }
            // "Do not count slow torrents": below both thresholds counts as slow.
            if dbps < u64::from(cfg.slow_down_kib) * 1024
                && ubps < u64::from(cfg.slow_up_kib) * 1024
            {
                l.slow_since.get_or_insert_with(Instant::now);
            } else {
                l.slow_since = None;
            }
            if st.finished && st.live.is_some() {
                rec.seeding_time_secs += 1;
            }
            let ratio = if rec.downloaded > 0 {
                rec.uploaded as f32 / rec.downloaded as f32
            } else if rec.uploaded > 0 {
                f32::INFINITY
            } else {
                0.0
            };
            // Seeding limits (qBittorrent's Options › BitTorrent › Seeding Limits).
            if st.finished && rec.running && state == TorrentState::Seeding {
                let inactive = now
                    - rec
                        .last_activity
                        .or(rec.completed_on)
                        .unwrap_or(rec.added_on);
                if share_limit_reached(
                    cfg.share_limits_mode,
                    rec.ratio_limit.or(cfg.ratio_limit),
                    cfg.seeding_time_limit_mins,
                    cfg.inactive_seeding_time_limit_mins,
                    ratio,
                    rec.seeding_time_secs,
                    inactive,
                ) {
                    rec.running = false;
                    dirty.insert(*id);
                    match cfg.share_limit_action {
                        tt_ipc::ShareLimitAction::Remove => to_remove.push((*id, false)),
                        tt_ipc::ShareLimitAction::RemoveWithFiles => to_remove.push((*id, true)),
                        tt_ipc::ShareLimitAction::Stop | tt_ipc::ShareLimitAction::SuperSeeding => {
                            to_pause.push(*id)
                        }
                    }
                }
            }
            down += dbps;
            up += ubps;
            peers += peers_live;
            if dbps > 0 || ubps > 0 {
                active += 1;
            }
            rows.push(make_row(
                rec,
                l,
                RowLive {
                    state,
                    progress,
                    dbps,
                    ubps,
                    eta,
                    seeds,
                    peers: peers_live.saturating_sub(seeds),
                    peers_seen,
                    total: st.total_bytes,
                    done: st.progress_bytes,
                    ratio,
                    error: st.error.clone(),
                },
            ));
        }
        // Rows only in the store (missing from the session).
        for (id, rec) in &inner.recs {
            if !handles.iter().any(|(h, _)| h == id) {
                let l = Live::default();
                rows.push(make_row(
                    rec,
                    &l,
                    RowLive {
                        state: TorrentState::Error,
                        progress: rec.progress,
                        total: rec.size,
                        done: (rec.size as f64 * f64::from(rec.progress)) as u64,
                        error: Some("Not in the session any more".into()),
                        ..RowLive::default()
                    },
                ));
            }
        }
        let queue_changes = plan_queue(&mut inner);
        inner.session_down += sess_down;
        inner.session_up += sess_up;
        // Status.
        let free = tt_platform::free_space(&self.default_save_path(&inner.settings));
        let mem = tt_platform::process_memory(None).unwrap_or_default();
        let status = SessionStatus {
            down_bps: down,
            up_bps: up,
            session_downloaded: inner.session_down,
            session_uploaded: inner.session_up,
            dht_nodes: session
                .get_dht()
                .map(|d| d.stats().routing_table_size as u32)
                .unwrap_or(0),
            free_space: free,
            engine_memory: mem.private_working_set,
            memory_cap: 0,
            port: session.announce_port(),
            port_open: None,
            slow_mode: inner.settings.slow_mode,
            connected_peers: peers,
            starting: inner.waves.as_ref().map(|w| (w.done, w.total)),
            torrents: rows.len() as u32,
            active,
        };
        inner.speed.push_back(SpeedPoint {
            down_bps: down,
            up_bps: up,
        });
        while inner.speed.len() > SPEED_HISTORY {
            inner.speed.pop_front();
        }
        // Deltas for subscribers.
        let mut upsert = Vec::new();
        let ids: HashSet<TorrentId> = rows.iter().map(|r| r.id).collect();
        for row in rows {
            if inner.last_rows.get(&row.id) != Some(&row) {
                inner.last_rows.insert(row.id, row.clone());
                upsert.push(row);
            }
        }
        let removed: Vec<TorrentId> = inner
            .last_rows
            .keys()
            .filter(|id| !ids.contains(id))
            .copied()
            .collect();
        for id in &removed {
            inner.last_rows.remove(id);
        }
        let status_changed = inner.status != status;
        inner.status = status.clone();
        let all_paused = !inner.recs.is_empty() && inner.recs.values().all(|r| !r.running);
        drop(inner);
        self.update_tray(&status, all_paused);
        for (id, start) in queue_changes {
            if start {
                self.start_one(id);
            } else {
                self.pause_one(id);
            }
        }
        for id in to_pause {
            self.pause_one(id);
        }
        if !upsert.is_empty() || !removed.is_empty() {
            self.broadcast(Event::Rows { upsert, removed });
        }
        if status_changed {
            self.broadcast(Event::Status(status));
        }
        to_remove
    }

    fn update_tray(&self, s: &SessionStatus, all_paused: bool) {
        use tt_ipc::fmt;
        let header = format!(
            "↓ {}   ↑ {}   {} active",
            fmt::speed(s.down_bps),
            fmt::speed(s.up_bps),
            s.active
        );
        let tooltip = format!(
            "tTorrent\n↓ {}  ↑ {}\n{} active of {}",
            fmt::speed(s.down_bps),
            fmt::speed(s.up_bps),
            s.active,
            s.torrents
        );
        let window_open = self.window.is_open();
        if let Ok(mut t) = self.tray.lock() {
            t.header = header;
            t.tooltip = tooltip;
            t.slow_mode = s.slow_mode;
            t.window_open = window_open;
            t.all_paused = all_paused;
        }
    }

    /// A tray menu choice, on the tray thread.
    pub fn tray_action(self: &Arc<Self>, a: tt_platform::tray::TrayAction) {
        use tt_platform::tray::TrayAction as A;
        let req = match a {
            A::ToggleWindow => {
                if self.window.is_open() {
                    self.broadcast(Event::CloseWindow);
                } else {
                    self.window.show(self);
                }
                return;
            }
            A::AddFile => return self.window.show_with(self, Some("add-file")),
            A::AddMagnet => return self.window.show_with(self, Some("add-magnet")),
            A::SpeedLimits => return self.window.show_with(self, Some("settings")),
            A::SlowMode => Request::SetSlowMode {
                on: !self.settings().slow_mode,
            },
            A::PauseResumeAll => {
                if lock(&self.inner).recs.values().all(|r| !r.running) {
                    Request::ResumeAll
                } else {
                    Request::PauseAll
                }
            }
            A::PauseHour => Request::PauseFor { secs: 3600 },
            A::Exit => Request::Quit,
        };
        let me = self.clone();
        self.rt.spawn(async move {
            if let Err(e) = me.handle(req).await {
                tracing::warn!("tray: {e:#}");
            }
        });
    }

    fn default_save_path(&self, s: &EngineSettings) -> std::path::PathBuf {
        if s.default_save_path.is_empty() {
            tt_platform::downloads_dir()
        } else {
            s.default_save_path.clone().into()
        }
    }

    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<Event> {
        let (tx, rx) = std::sync::mpsc::channel();
        lock(&self.subscribers).push(tx);
        rx
    }

    pub fn broadcast(&self, e: Event) {
        lock(&self.subscribers).retain(|s| s.send(e.clone()).is_ok());
    }

    /// Write changed torrents in one transaction.
    pub fn flush(&self) -> Result<()> {
        let recs: Vec<TorrentRecord> = {
            let mut inner = lock(&self.inner);
            let ids: Vec<_> = inner.dirty.drain().collect();
            ids.iter()
                .filter_map(|id| inner.recs.get(id).cloned())
                .collect()
        };
        lock(&self.store).save_torrents(&recs)
    }

    fn save_settings(&self) -> Result<()> {
        let s = serde_json::to_string(&lock(&self.inner).settings)?;
        lock(&self.store).set_kv("settings", &s)
    }

    /// The list for the window's first frame: live rows if the engine has
    /// them, otherwise the stored snapshot (docs/startup.md).
    pub fn snapshot(&self) -> Snapshot {
        let inner = lock(&self.inner);
        let from_snapshot = inner.last_rows.is_empty();
        let rows: Vec<TorrentRow> = if from_snapshot {
            let l = Live::default();
            inner
                .recs
                .values()
                .map(|r| {
                    let state = if r.running {
                        TorrentState::Starting
                    } else if r.progress >= 1.0 {
                        TorrentState::Completed
                    } else {
                        TorrentState::Stopped
                    };
                    make_row(
                        r,
                        &l,
                        RowLive {
                            state,
                            progress: r.progress,
                            total: r.size,
                            done: (r.size as f64 * f64::from(r.progress)) as u64,
                            ratio: if r.downloaded > 0 {
                                r.uploaded as f32 / r.downloaded as f32
                            } else {
                                0.0
                            },
                            ..RowLive::default()
                        },
                    )
                })
                .collect()
        } else {
            inner.last_rows.values().cloned().collect()
        };
        Snapshot {
            rows,
            status: inner.status.clone(),
            lists: inner.lists.clone(),
            prefs: inner.prefs.clone(),
            from_snapshot,
            pending: inner.pending.iter().map(|p| p.add.clone()).collect(),
        }
    }

    pub async fn handle(self: &Arc<Self>, req: Request) -> Result<Response> {
        match req {
            Request::Snapshot => Ok(Response::Snapshot(Box::new(self.snapshot()))),
            Request::Subscribe => Ok(Response::Ok),
            Request::Add { source, options } => self.add(source, options).await,
            Request::Open { sources } => self.open_sources(sources).await,
            Request::ConfirmAdd {
                token,
                options,
                never_show_again,
            } => self.confirm_add(token, options, never_show_again).await,
            Request::CancelAdd { token } => {
                self.cancel_add(token);
                Ok(Response::Ok)
            }
            Request::PendingAdds => Ok(Response::PendingAdds {
                items: self.pending_list(),
            }),
            Request::FreeSpace { path } => Ok(Response::FreeSpace {
                bytes: tt_platform::free_space(std::path::Path::new(&path)),
            }),
            Request::Associations => Ok(Response::Associations(self.associations())),
            Request::MakeDefault => self.make_default(),
            Request::Pause { ids } => {
                self.set_running(&ids, false);
                Ok(Response::Ok)
            }
            Request::Resume { ids } => {
                self.edit(&ids, |r| r.forced = false);
                self.set_running(&ids, true);
                Ok(Response::Ok)
            }
            // Force start: runs outside the queue's limits until stopped.
            Request::ForceStart { ids } => {
                self.edit(&ids, |r| r.forced = true);
                self.set_running(&ids, true);
                Ok(Response::Ok)
            }
            Request::PauseAll => {
                let ids = self.all_ids();
                self.set_running(&ids, false);
                Ok(Response::Ok)
            }
            Request::ResumeAll => {
                let ids = self.all_ids();
                lock(&self.inner).resume_at = None;
                self.set_running(&ids, true);
                Ok(Response::Ok)
            }
            Request::PauseFor { secs } => {
                let ids = self.all_ids();
                self.set_running(&ids, false);
                lock(&self.inner).resume_at = Some(Instant::now() + Duration::from_secs(secs));
                Ok(Response::Ok)
            }
            Request::Remove { ids, delete_files } => {
                let session = self.session()?.clone();
                for id in ids {
                    session
                        .delete((id as usize).into(), delete_files)
                        .await
                        .with_context(|| format!("removing {id}"))?;
                    lock(&self.inner).recs.remove(&id);
                    lock(&self.store).delete_torrent(id)?;
                }
                self.refresh_lists();
                Ok(Response::Ok)
            }
            Request::Recheck { .. } => Err(anyhow!(
                "Force recheck isn't available yet: librqbit has no recheck call (PLAN.md change list)"
            )),
            Request::Reannounce { .. } => Err(anyhow!(
                "Force reannounce isn't available yet: librqbit has no reannounce call"
            )),
            Request::SetCategory { ids, category } => {
                let cat = category.unwrap_or_default();
                if !cat.is_empty() {
                    self.ensure_category(&cat, None)?;
                }
                self.edit(&ids, |r| r.category = cat.clone());
                Ok(Response::Ok)
            }
            Request::CreateCategory { name, save_path } => {
                self.ensure_category(&name, save_path)?;
                Ok(Response::Ok)
            }
            Request::SetTags { ids, tags, mode } => {
                self.edit(&ids, |r| match mode {
                    TagMode::Set => r.tags = tags.clone(),
                    TagMode::Add => {
                        for t in &tags {
                            if !r.tags.contains(t) {
                                r.tags.push(t.clone());
                            }
                        }
                    }
                    TagMode::Remove => r.tags.retain(|t| !tags.contains(t)),
                });
                self.refresh_lists();
                Ok(Response::Ok)
            }
            Request::Rename { id, name } => {
                let name = name.trim().to_owned();
                if name.is_empty() {
                    return Err(anyhow!("A name can't be empty"));
                }
                self.edit(&[id], |r| r.name = name.clone());
                Ok(Response::Ok)
            }
            Request::SetLimits {
                ids: None,
                down_bps,
                up_bps,
            } => {
                {
                    let mut inner = lock(&self.inner);
                    if let Some(d) = down_bps {
                        inner.settings.down_limit = d;
                    }
                    if let Some(u) = up_bps {
                        inner.settings.up_limit = u;
                    }
                }
                self.save_settings()?;
                apply_limits(self.session()?, &self.settings());
                Ok(Response::Ok)
            }
            Request::SetLimits {
                ids: Some(ids),
                down_bps,
                up_bps,
            } => {
                self.edit(&ids, |r| {
                    if let Some(d) = down_bps {
                        r.down_limit = d;
                    }
                    if let Some(u) = up_bps {
                        r.up_limit = u;
                    }
                });
                Ok(Response::Ok)
            }
            Request::SetSlowMode { on } => {
                lock(&self.inner).settings.slow_mode = on;
                self.save_settings()?;
                apply_limits(self.session()?, &self.settings());
                Ok(Response::Ok)
            }
            Request::QueueMove { ids, to } => {
                self.queue_move(&ids, to);
                Ok(Response::Ok)
            }
            Request::Details { id } => Ok(Response::Details(Box::new(self.details(id)?))),
            Request::SpeedHistory => Ok(Response::SpeedHistory {
                points: lock(&self.inner).speed.iter().copied().collect(),
            }),
            Request::GetSettings => Ok(Response::Settings(Box::new(self.settings()))),
            Request::SetSettings { settings } => {
                {
                    let mut inner = lock(&self.inner);
                    let offline = inner.settings.offline;
                    inner.settings = *settings;
                    inner.settings.offline = offline;
                }
                self.save_settings()?;
                apply_limits(self.session()?, &self.settings());
                Ok(Response::Ok)
            }
            Request::GetUiPrefs => Ok(Response::UiPrefs(Box::new(lock(&self.inner).prefs.clone()))),
            Request::SetUiPrefs { prefs } => {
                let json = serde_json::to_string(&prefs)?;
                lock(&self.inner).prefs = prefs;
                lock(&self.store).set_kv("ui_prefs", &json)?;
                Ok(Response::Ok)
            }
            Request::ShowWindow => {
                self.window.show(self);
                Ok(Response::Ok)
            }
            Request::WindowClosing => Ok(Response::Ok),
            Request::HideWindow => {
                self.broadcast(Event::CloseWindow);
                Ok(Response::Ok)
            }
            Request::MagnetLink { id } => {
                let h = self.handle_of(id)?;
                let name = h.name().unwrap_or_default();
                let mut m = format!("magnet:?xt=urn:btih:{}", h.info_hash().as_string());
                if !name.is_empty() {
                    m.push_str("&dn=");
                    m.push_str(&url_escape(&name));
                }
                for t in &h.shared().trackers {
                    m.push_str("&tr=");
                    m.push_str(&url_escape(t.as_str()));
                }
                Ok(Response::Text { text: m })
            }
            Request::OpenFolder { id } => {
                let path = lock(&self.inner)
                    .recs
                    .get(&id)
                    .map(|r| r.save_path.clone())
                    .ok_or_else(|| anyhow!("no torrent {id}"))?;
                tt_platform::open_folder(std::path::Path::new(&path))?;
                Ok(Response::Ok)
            }
            Request::Quit => {
                self.exit.notify_waiters();
                self.exit.notify_one();
                Ok(Response::Ok)
            }
        }
    }

    fn all_ids(&self) -> Vec<TorrentId> {
        lock(&self.inner).recs.keys().copied().collect()
    }

    fn set_running(&self, ids: &[TorrentId], on: bool) {
        self.edit(ids, |r| r.running = on);
        {
            let mut inner = lock(&self.inner);
            if let Some(w) = inner.waves.as_mut() {
                // A torrent the user touched leaves the wave queue.
                w.order.retain(|i| !ids.contains(i));
            }
            for id in ids {
                if let Some(l) = inner.live.get_mut(id) {
                    l.queued = false;
                }
            }
        }
        for &id in ids {
            if on {
                self.start_one(id);
            } else {
                self.pause_one(id);
            }
        }
    }

    fn edit(&self, ids: &[TorrentId], mut f: impl FnMut(&mut TorrentRecord)) {
        let mut inner = lock(&self.inner);
        for id in ids {
            if let Some(r) = inner.recs.get_mut(id) {
                f(r);
                inner.dirty.insert(*id);
                // Force the row out on the next tick.
                inner.last_rows.remove(id);
            }
        }
    }

    fn ensure_category(&self, name: &str, save_path: Option<String>) -> Result<()> {
        let exists = lock(&self.inner)
            .lists
            .categories
            .iter()
            .any(|c| c.name == name);
        if !exists || save_path.is_some() {
            let color = lock(&self.inner).lists.categories.len() as u8 % 6;
            lock(&self.store).upsert_category(&Category {
                name: name.to_owned(),
                save_path: save_path.unwrap_or_default(),
                color,
            })?;
        }
        self.refresh_lists();
        Ok(())
    }

    fn refresh_lists(&self) {
        let cats = lock(&self.store).categories().unwrap_or_default();
        let lists = {
            let mut inner = lock(&self.inner);
            inner.lists = Lists {
                categories: cats,
                tags: collect_tags(inner.recs.values()),
            };
            inner.lists.clone()
        };
        self.broadcast(Event::Lists(lists));
    }

    fn queue_move(&self, ids: &[TorrentId], to: QueueMove) {
        let mut inner = lock(&self.inner);
        let mut q: Vec<(u32, TorrentId)> = inner
            .recs
            .values()
            .filter_map(|r| r.queue_pos.map(|p| (p, r.id)))
            .collect();
        q.sort();
        let mut order: Vec<TorrentId> = q.into_iter().map(|(_, id)| id).collect();
        for &id in ids {
            let Some(i) = order.iter().position(|&x| x == id) else {
                continue;
            };
            let id = order.remove(i);
            let j = match to {
                QueueMove::Top => 0,
                QueueMove::Up => i.saturating_sub(1),
                QueueMove::Down => (i + 1).min(order.len()),
                QueueMove::Bottom => order.len(),
            };
            order.insert(j, id);
        }
        for (i, id) in order.iter().enumerate() {
            if let Some(r) = inner.recs.get_mut(id) {
                r.queue_pos = Some(i as u32 + 1);
            }
            inner.dirty.insert(*id);
            inner.last_rows.remove(id);
        }
    }

    fn details(&self, id: TorrentId) -> Result<TorrentDetails> {
        let h = self.handle_of(id)?;
        let api = self.api()?;
        let st = h.stats();
        let (comment, created_by, piece_size, piece_count, files) = h
            .with_metadata(|m| {
                let piece_size = u64::from(m.info.lengths().default_piece_length());
                let lengths = m.info.lengths();
                let piece_count = lengths.total_pieces();
                let files: Vec<FileRow> = m
                    .file_infos
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let done = st.file_progress.get(i).copied().unwrap_or(0);
                        FileRow {
                            index: i as u32,
                            path: f.relative_filename.to_string_lossy().into_owned(),
                            size: f.len,
                            progress: if f.len == 0 {
                                1.0
                            } else {
                                (done as f64 / f.len as f64) as f32
                            },
                            priority: if h.only_files().is_none_or(|o| o.contains(&i)) {
                                FilePriority::Normal
                            } else {
                                FilePriority::Skip
                            },
                            missing: false,
                        }
                    })
                    .collect();
                (String::new(), String::new(), piece_size, piece_count, files)
            })
            .unwrap_or_default();
        let (pieces, pieces_have) = match api.api_dump_haves((id as usize).into()) {
            Ok((bf, _)) => (bf.as_raw_slice().to_vec(), bf.count_ones() as u32),
            Err(_) => (Vec::new(), 0),
        };
        let peers = api
            .api_peer_stats((id as usize).into(), Default::default())
            .ok()
            .and_then(|s| serde_json::to_value(&s).ok())
            .map(peer_rows)
            .unwrap_or_default();
        let trackers = h
            .shared()
            .trackers
            .iter()
            .map(|u| TrackerRow {
                url: mask_passkey(u.as_str()),
                status: if st.live.is_some() {
                    "Working".into()
                } else {
                    "Not contacted".into()
                },
                message: String::new(),
                peers: 0,
            })
            .collect();
        Ok(TorrentDetails {
            id,
            piece_count,
            piece_size,
            pieces_have,
            pieces,
            comment,
            created_by,
            files,
            trackers,
            peers,
            http_sources: Vec::new(),
        })
    }

    pub fn backup(&self, path: &std::path::Path) -> Result<()> {
        self.flush()?;
        lock(&self.store).backup_to(path)
    }

    /// Import a qBittorrent library: torrents, progress, categories, tags and
    /// history, without rechecking (docs/startup.md §1). Returns how many were added.
    pub async fn import_qbittorrent(
        self: &Arc<Self>,
        bt_backup: Option<std::path::PathBuf>,
    ) -> Result<usize> {
        let (bt, config) = match bt_backup {
            Some(d) => {
                let cfg = d.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                (d, cfg)
            }
            None => tt_import::default_qb_dirs().ok_or_else(|| anyhow!("no qBittorrent folder"))?,
        };
        let (list, errors) = tt_import::read_bt_backup(&bt)?;
        for e in &errors {
            tracing::warn!("import skipped {e}");
        }
        for c in tt_import::read_categories(&config) {
            self.ensure_category(&c.name, Some(c.save_path).filter(|s| !s.is_empty()))?;
        }
        let known: HashSet<String> = lock(&self.inner)
            .recs
            .values()
            .map(|r| r.info_hash.clone())
            .collect();
        let total = list.len();
        let mut added = 0;
        let mut to_start = Vec::new();
        for (i, t) in list.into_iter().enumerate() {
            if known.contains(&t.info_hash) {
                continue;
            }
            // qBittorrent's piece map becomes librqbit's fast-resume bitfield.
            if let Some(bits) = &t.pieces {
                let _ = std::fs::create_dir_all(&self.paths.session);
                let f = self.paths.session.join(format!("{}.bitv", t.info_hash));
                if let Err(e) = std::fs::write(&f, bits) {
                    tracing::warn!("writing {}: {e}", f.display());
                }
            }
            let content = match (&t.content_root, t.save_path.is_empty()) {
                (_, true) => None,
                (Some(root), false) => Some(
                    std::path::Path::new(&t.save_path)
                        .join(root)
                        .to_string_lossy()
                        .into_owned(),
                ),
                (None, false) => Some(t.save_path.clone()),
            };
            let options = AddOptions {
                save_path: content,
                // The folder above is already where qBittorrent put the files.
                content_layout: Some(tt_ipc::ContentLayout::NoSubfolder),
                category: Some(t.category.clone()).filter(|s| !s.is_empty()),
                tags: t.tags.clone(),
                paused: true,
                ..AddOptions::default()
            };
            match self
                .add(
                    AddSource::Bytes {
                        data: t.torrent_bytes.clone(),
                    },
                    options,
                )
                .await
            {
                Ok(Response::Added { id, .. }) => {
                    added += 1;
                    self.edit(&[id], |r| {
                        if let Some(n) = &t.name {
                            r.name = n.clone();
                        }
                        if t.added_on > 0 {
                            r.added_on = t.added_on;
                        }
                        r.completed_on = t.completed_on;
                        if !t.save_path.is_empty() {
                            r.save_path = t.save_path.clone();
                        }
                        r.uploaded = t.uploaded;
                        r.downloaded = t.downloaded;
                        r.time_active_secs = t.active_secs;
                        r.ratio_limit = t.ratio_limit;
                        r.running = !t.paused;
                    });
                    if !t.paused {
                        to_start.push(id);
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("import {}: {e:#}", t.info_hash),
            }
            if i % 25 == 0 {
                self.broadcast(Event::Notice {
                    text: format!(
                        "Importing {i} of {total} torrents from qBittorrent. No recheck needed."
                    ),
                });
            }
        }
        self.flush()?;
        {
            let mut inner = lock(&self.inner);
            let w = inner.waves.get_or_insert(Waves {
                order: VecDeque::new(),
                total: 0,
                done: 0,
            });
            w.total += to_start.len() as u32;
            w.order.extend(to_start);
        }
        self.broadcast(Event::Notice {
            text: format!("Imported {added} torrents from qBittorrent"),
        });
        Ok(added)
    }

    /// Quit cleanly: save everything, stop librqbit.
    pub async fn shutdown(&self) {
        self.broadcast(Event::Exiting);
        if let Err(e) = self.flush() {
            tracing::warn!("saving on exit: {e:#}");
        }
        self.window.close();
        if let Some(s) = self.session.get() {
            s.stop().await;
        }
    }
}

#[derive(Default)]
struct RowLive {
    state: TorrentState,
    progress: f32,
    dbps: u64,
    ubps: u64,
    eta: Option<u64>,
    seeds: u32,
    peers: u32,
    peers_seen: u32,
    total: u64,
    done: u64,
    ratio: f32,
    error: Option<String>,
}

fn make_row(r: &TorrentRecord, l: &Live, x: RowLive) -> TorrentRow {
    TorrentRow {
        id: r.id,
        queue: r.queue_pos,
        name: r.name.clone(),
        size: if x.total > 0 { x.total } else { r.size },
        total_size: r.total_size,
        progress: x.progress,
        state: x.state,
        seeds: x.seeds,
        seeds_total: x.seeds,
        peers: x.peers,
        peers_total: x.peers_seen,
        down_bps: x.dbps,
        up_bps: x.ubps,
        eta_secs: if x.progress >= 1.0 { None } else { x.eta },
        ratio: x.ratio,
        popularity: 0.0,
        category: r.category.clone(),
        tags: r.tags.clone(),
        added_on: r.added_on,
        completed_on: r.completed_on,
        tracker: r.tracker.clone(),
        down_limit: r.down_limit,
        up_limit: r.up_limit,
        downloaded: r.downloaded,
        uploaded: r.uploaded,
        session_downloaded: l.session_down,
        session_uploaded: l.session_up,
        remaining: x.total.saturating_sub(x.done),
        time_active_secs: r.time_active_secs,
        save_path: r.save_path.clone(),
        completed: x.done,
        ratio_limit: r.ratio_limit,
        last_seen_complete: None,
        last_activity: r.last_activity,
        availability: 0.0,
        incomplete_save_path: String::new(),
        hash_v1: r.info_hash.clone(),
        hash_v2: String::new(),
        reannounce_in_secs: None,
        private: r.private,
        created_on: r.created_on,
        error: x.error,
    }
}

fn derive_state(
    rec: &TorrentRecord,
    l: &Live,
    s: &TorrentStatsState,
    finished: bool,
    dbps: u64,
    has_error: bool,
) -> TorrentState {
    if has_error {
        return TorrentState::Error;
    }
    match s {
        TorrentStatsState::Initializing { .. } => TorrentState::Checking,
        TorrentStatsState::Error => TorrentState::Error,
        TorrentStatsState::Paused => {
            if l.queued {
                TorrentState::Queued
            } else if rec.running && !l.started {
                TorrentState::Starting
            } else if finished {
                TorrentState::Completed
            } else {
                TorrentState::Stopped
            }
        }
        TorrentStatsState::Live => {
            if finished {
                TorrentState::Seeding
            } else if dbps > 0 {
                TorrentState::Downloading
            } else {
                TorrentState::Stalled
            }
        }
    }
}

/// Seeding limits: the ones set are checked, combined by `mode`.
#[allow(clippy::too_many_arguments)]
fn share_limit_reached(
    mode: tt_ipc::ShareLimitsMode,
    ratio_limit: Option<f32>,
    time_limit_mins: Option<u32>,
    inactive_limit_mins: Option<u32>,
    ratio: f32,
    seeding_secs: u64,
    inactive_secs: i64,
) -> bool {
    let set: Vec<bool> = [
        ratio_limit.map(|l| ratio >= l),
        time_limit_mins.map(|m| seeding_secs >= u64::from(m) * 60),
        inactive_limit_mins.map(|m| inactive_secs >= i64::from(m) * 60),
    ]
    .into_iter()
    .flatten()
    .collect();
    match mode {
        _ if set.is_empty() => false,
        tt_ipc::ShareLimitsMode::MatchAny => set.iter().any(|b| *b),
        tt_ipc::ShareLimitsMode::MatchAll => set.iter().all(|b| *b),
    }
}

/// Torrent queueing with qBittorrent's (libtorrent's) rules: downloads in
/// queue order, then seeds, each within its own limit and all within the
/// total; -1 is unlimited. Force-started torrents sit outside the limits,
/// and with "Do not count slow torrents" on, slow ones run without counting.
/// Returns (id, start?) changes.
fn plan_queue(inner: &mut Inner) -> Vec<(TorrentId, bool)> {
    if inner.waves.is_some() {
        return Vec::new();
    }
    let Inner {
        recs,
        live,
        settings: s,
        ..
    } = inner;
    let mut changes = Vec::new();
    if !s.queueing {
        // Turning queueing off releases everything it held back.
        for (id, l) in live.iter_mut() {
            if l.queued {
                l.queued = false;
                if recs.get(id).is_some_and(|r| r.running) {
                    changes.push((*id, true));
                }
            }
        }
        changes.sort_unstable();
        return changes;
    }
    let limit = |v: i32| usize::try_from(v).unwrap_or(usize::MAX);
    let (max_dl, max_up, max_total) = (
        limit(s.max_active_downloads),
        limit(s.max_active_uploads),
        limit(s.max_active_torrents),
    );
    let slow_after = Duration::from_secs(u64::from(s.slow_timer_secs));
    // Only torrents that are transferring (or held in the queue) take part:
    // checking, starting, errored and stopped ones are not queued.
    let mut dl: Vec<(u32, TorrentId)> = Vec::new();
    let mut seeds: Vec<(i64, TorrentId)> = Vec::new();
    for r in recs.values() {
        let eligible = r.running
            && matches!(
                r.state,
                TorrentState::Downloading
                    | TorrentState::Stalled
                    | TorrentState::Metadata
                    | TorrentState::Queued
                    | TorrentState::Seeding
            );
        if !eligible {
            continue;
        }
        if r.forced {
            if let Some(l) = live.get_mut(&r.id)
                && l.queued
            {
                l.queued = false;
                changes.push((r.id, true));
            }
            continue;
        }
        if r.progress < 1.0 {
            dl.push((r.queue_pos.unwrap_or(u32::MAX), r.id));
        } else {
            // libtorrent ranks seeds by need; oldest first is our stand-in.
            seeds.push((r.added_on, r.id));
        }
    }
    dl.sort_unstable();
    seeds.sort_unstable();
    let order = dl
        .iter()
        .map(|x| (true, x.1))
        .chain(seeds.iter().map(|x| (false, x.1)));
    let (mut n_dl, mut n_up, mut n_total) = (0usize, 0usize, 0usize);
    for (downloading, id) in order {
        let Some(l) = live.get_mut(&id) else {
            continue;
        };
        let slow = s.ignore_slow_torrents
            && l.started
            && !l.queued
            && l.slow_since.is_some_and(|t| t.elapsed() >= slow_after);
        let room = n_total < max_total
            && if downloading {
                n_dl < max_dl
            } else {
                n_up < max_up
            };
        if slow || room {
            if !slow {
                n_total += 1;
                if downloading {
                    n_dl += 1;
                } else {
                    n_up += 1;
                }
            }
            if l.queued {
                l.queued = false;
                changes.push((id, true));
            }
        } else if !l.queued {
            l.queued = true;
            changes.push((id, false));
        }
    }
    changes
}

fn new_record(id: TorrentId, h: &ManagedTorrent, now: i64, o: &AddOptions) -> TorrentRecord {
    let st = h.stats();
    let (private, created_on, tracker) = h
        .with_metadata(|m| (m.info.info().private, None::<i64>, String::new()))
        .unwrap_or((false, None, String::new()));
    let tracker = if tracker.is_empty() {
        h.shared()
            .trackers
            .iter()
            .next()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default()
    } else {
        tracker
    };
    TorrentRecord {
        id,
        info_hash: h.info_hash().as_string(),
        name: h.name().unwrap_or_else(|| "Unnamed torrent".into()),
        total_size: st.total_bytes,
        size: st.total_bytes,
        save_path: display_path(h.output_folder()),
        category: o.category.clone().unwrap_or_default(),
        tags: o.tags.clone(),
        added_on: now,
        running: true,
        progress: if st.total_bytes > 0 {
            (st.progress_bytes as f64 / st.total_bytes as f64) as f32
        } else {
            0.0
        },
        private,
        created_on,
        tracker,
        ..Default::default()
    }
}

/// A path as people write it: without Windows' `\\?\` prefix.
pub fn display_path(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => s.strip_prefix(r"\\?\").unwrap_or(&s).to_owned(),
    }
}

fn all_handles(session: &Session) -> Vec<(TorrentId, Arc<ManagedTorrent>)> {
    session.with_torrents(|it| it.map(|(id, h)| (id as TorrentId, h.clone())).collect())
}

fn collect_tags<'a>(recs: impl Iterator<Item = &'a TorrentRecord>) -> Vec<String> {
    let mut t: Vec<String> = recs.flat_map(|r| r.tags.iter().cloned()).collect();
    t.sort_by_key(|s| s.to_lowercase());
    t.dedup();
    t
}

/// librqbit reports speeds in MiB/s.
fn mbps_to_bps(mbps: f64) -> u64 {
    (mbps * 1024.0 * 1024.0).max(0.0) as u64
}

fn nz(bps: u64) -> Option<NonZeroU32> {
    NonZeroU32::new(bps.min(u64::from(u32::MAX)) as u32)
}

fn apply_limits(session: &Session, s: &EngineSettings) {
    let (d, u) = if s.slow_mode {
        (s.alt_down_limit, s.alt_up_limit)
    } else {
        (s.down_limit, s.up_limit)
    };
    session.ratelimits.set_download_bps(nz(d));
    session.ratelimits.set_upload_bps(nz(u));
}

fn session_options(s: &EngineSettings, paths: &Paths) -> SessionOptions {
    use librqbit::{ListenerMode, ListenerOptions};
    SessionOptions {
        disable_trackers: s.offline,
        dht: if s.dht && !s.offline {
            Some(librqbit::DhtSessionConfig::default())
        } else {
            None
        },
        bind_device_name: s.bind_interface.clone(),
        fastresume: true,
        // docs/startup.md: no rechecks unless a file changed. Saved progress
        // is trusted when every file is at its expected size (like
        // libtorrent); otherwise librqbit samples, then rechecks.
        fastresume_check: librqbit::FastResumeCheck::FileSizes,
        persistence: Some(SessionPersistenceConfig::Json {
            folder: Some(paths.session.clone()),
        }),
        listen: Some(ListenerOptions {
            mode: ListenerMode::TcpAndUtp,
            listen_addr: (std::net::Ipv6Addr::UNSPECIFIED, s.listen_port).into(),
            enable_upnp_port_forwarding: s.upnp && !s.offline,
            ..Default::default()
        }),
        peer_limit: Some(s.max_connections_per_torrent as usize),
        disable_local_service_discovery: !s.lsd || s.offline,
        concurrent_init_limit: Some((s.max_active_checking as usize).max(1)),
        client_name_and_version: Some(format!("tTorrent {}", env!("CARGO_PKG_VERSION"))),
        ..Default::default()
    }
}

fn peer_rows(v: serde_json::Value) -> Vec<PeerRow> {
    let Some(map) = v.get("peers").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    let mut rows: Vec<PeerRow> = map
        .iter()
        .map(|(addr, p)| {
            let c = p.get("counters");
            let n = |k: &str| {
                c.and_then(|c| c.get(k))
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0)
            };
            PeerRow {
                addr: addr.clone(),
                client: p
                    .get("client_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                progress: 0.0,
                down_bps: 0,
                up_bps: 0,
                downloaded: n("fetched_bytes"),
                uploaded: n("uploaded_bytes"),
                state: p
                    .get("state")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_owned(),
            }
        })
        .filter(|p| p.state == "live")
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.downloaded));
    rows
}

/// docs/mcp.md: announce URLs are shown with the passkey masked.
pub fn mask_passkey(url: &str) -> String {
    let Ok(mut u) = url::Url::parse(url) else {
        return url.to_owned();
    };
    let segs: Vec<String> = u
        .path_segments()
        .map(|s| {
            s.map(|seg| {
                if seg.len() >= 16 && seg.chars().all(|c| c.is_ascii_alphanumeric()) {
                    "•••".to_owned()
                } else {
                    seg.to_owned()
                }
            })
            .collect()
        })
        .unwrap_or_default();
    u.set_path(&segs.join("/"));
    let pairs: Vec<(String, String)> = u
        .query_pairs()
        .map(|(k, v)| {
            let secret = matches!(
                k.as_ref(),
                "passkey" | "pk" | "key" | "authkey" | "torrent_pass"
            );
            (
                k.into_owned(),
                if secret {
                    "•••".into()
                } else {
                    v.into_owned()
                },
            )
        })
        .collect();
    if pairs.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut().clear().extend_pairs(pairs);
    }
    u.to_string()
}

fn url_escape(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_prefix_dropped() {
        use std::path::Path;
        assert_eq!(display_path(Path::new(r"\\?\D:\Films")), r"D:\Films");
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\nas\share")),
            r"\\nas\share"
        );
        assert_eq!(display_path(Path::new("/home/a")), "/home/a");
    }

    #[test]
    fn passkeys_masked() {
        let m = mask_passkey("https://tracker.example.org/abcdef0123456789abcd/announce");
        assert!(!m.contains("abcdef0123456789abcd"), "{m}");
        let m = mask_passkey("https://t.example.org/announce.php?passkey=secret123&x=1");
        assert!(!m.contains("secret123") && m.contains("x=1"), "{m}");
        assert_eq!(
            mask_passkey("udp://tracker.opentrackr.org:1337/announce"),
            "udp://tracker.opentrackr.org:1337/announce"
        );
    }

    fn inner_with(n: u32, max: i32) -> Inner {
        let mut i = Inner::default();
        i.settings.queueing = true;
        i.settings.max_active_downloads = max;
        for id in 0..n {
            i.recs.insert(
                id,
                TorrentRecord {
                    id,
                    running: true,
                    queue_pos: Some(id + 1),
                    state: TorrentState::Downloading,
                    ..Default::default()
                },
            );
            i.live.insert(
                id,
                Live {
                    started: true,
                    ..Default::default()
                },
            );
        }
        i
    }

    #[test]
    fn queue_holds_back_extra_downloads() {
        let mut i = inner_with(5, 3);
        let ch = plan_queue(&mut i);
        assert_eq!(ch, vec![(3, false), (4, false)]);
        // Second pass is stable.
        assert!(plan_queue(&mut i).is_empty());
        // One finishes: the next in line starts.
        if let Some(r) = i.recs.get_mut(&0) {
            r.progress = 1.0;
        }
        assert_eq!(plan_queue(&mut i), vec![(3, true)]);
    }

    /// Found by tt-bench demo: a torrent still checking must never be queued
    /// (that paused its hash check half way).
    #[test]
    fn checking_torrents_are_not_queued() {
        let mut i = inner_with(5, 3);
        for id in 3..5 {
            if let Some(r) = i.recs.get_mut(&id) {
                r.state = TorrentState::Checking;
            }
        }
        assert!(plan_queue(&mut i).is_empty());
    }

    #[test]
    fn queue_off_changes_nothing() {
        let mut i = inner_with(5, 1);
        i.settings.queueing = false;
        assert!(plan_queue(&mut i).is_empty());
    }

    fn seeding(i: &mut Inner, ids: std::ops::Range<u32>) {
        for id in ids {
            if let Some(r) = i.recs.get_mut(&id) {
                r.progress = 1.0;
                r.state = TorrentState::Seeding;
                r.added_on = i64::from(id);
            }
        }
    }

    #[test]
    fn upload_and_total_limits() {
        // qBittorrent's defaults: 3 downloads, 3 uploads, 5 in total.
        let mut i = inner_with(8, 3);
        seeding(&mut i, 4..8);
        let ch = plan_queue(&mut i);
        // Downloads 0..3 run, 3 waits; seeds 4 and 5 fill the total of 5.
        assert_eq!(ch, vec![(3, false), (6, false), (7, false)]);
        i.settings.max_active_torrents = -1;
        // Unlimited total: the third seed may run, the fourth waits on uploads.
        assert_eq!(plan_queue(&mut i), vec![(6, true)]);
    }

    #[test]
    fn slow_torrents_count_unless_asked() {
        let mut i = inner_with(4, 1);
        if let Some(l) = i.live.get_mut(&0) {
            l.slow_since = Some(Instant::now() - Duration::from_secs(120));
        }
        assert_eq!(plan_queue(&mut i), vec![(1, false), (2, false), (3, false)]);
        i.settings.ignore_slow_torrents = true;
        // Torrent 0 is slow, so it no longer holds the only slot.
        assert_eq!(plan_queue(&mut i), vec![(1, true)]);
    }

    #[test]
    fn force_start_bypasses_the_queue() {
        let mut i = inner_with(3, 1);
        assert_eq!(plan_queue(&mut i), vec![(1, false), (2, false)]);
        if let Some(r) = i.recs.get_mut(&2) {
            r.forced = true;
        }
        assert_eq!(plan_queue(&mut i), vec![(2, true)]);
    }

    #[test]
    fn queueing_off_releases_held_torrents() {
        let mut i = inner_with(3, 1);
        plan_queue(&mut i);
        i.settings.queueing = false;
        assert_eq!(plan_queue(&mut i), vec![(1, true), (2, true)]);
    }

    #[test]
    fn seeding_limits_any_or_all() {
        use tt_ipc::ShareLimitsMode::{MatchAll, MatchAny};
        // Nothing set: never.
        assert!(!share_limit_reached(
            MatchAny,
            None,
            None,
            None,
            9.0,
            1 << 30,
            1 << 30
        ));
        // Ratio 1.0 reached, 24 h seeding not yet.
        assert!(share_limit_reached(
            MatchAny,
            Some(1.0),
            Some(1440),
            None,
            1.2,
            60,
            0
        ));
        assert!(!share_limit_reached(
            MatchAll,
            Some(1.0),
            Some(1440),
            None,
            1.2,
            60,
            0
        ));
        assert!(share_limit_reached(
            MatchAll,
            Some(1.0),
            Some(1440),
            None,
            1.2,
            86_400,
            0
        ));
        // Inactive for 2 h with a 60 min limit.
        assert!(share_limit_reached(
            MatchAny,
            None,
            None,
            Some(60),
            0.0,
            0,
            7_200
        ));
    }

    #[test]
    fn state_mapping() {
        let rec = TorrentRecord {
            running: true,
            ..Default::default()
        };
        let l = Live::default();
        assert_eq!(
            derive_state(&rec, &l, &TorrentStatsState::Paused, false, 0, false),
            TorrentState::Starting
        );
        let l = Live {
            started: true,
            ..Default::default()
        };
        let stopped = TorrentRecord::default();
        assert_eq!(
            derive_state(&stopped, &l, &TorrentStatsState::Paused, true, 0, false),
            TorrentState::Completed
        );
        assert_eq!(
            derive_state(&rec, &l, &TorrentStatsState::Live, false, 0, false),
            TorrentState::Stalled
        );
        assert_eq!(
            derive_state(&rec, &l, &TorrentStatsState::Live, true, 0, false),
            TorrentState::Seeding
        );
    }
}
