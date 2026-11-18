use serde::{Deserialize, Serialize};

pub type TorrentId = u32;

/// What a row shows in the Status column (and which sidebar filter it falls under).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum TorrentState {
    /// Loaded from the snapshot, not started yet (startup waves).
    #[default]
    Starting,
    /// Fetching metadata for a magnet link.
    Metadata,
    Downloading,
    /// Downloading but nothing arriving.
    Stalled,
    Seeding,
    /// Finished and stopped.
    Completed,
    /// Not finished and stopped.
    Stopped,
    Queued,
    Checking,
    Moving,
    Error,
    /// docs/missing-files.md case 2 and 3.
    MissingFiles,
    /// docs/missing-files.md case 1.
    DriveUnavailable,
}

impl TorrentState {
    pub fn label(self) -> &'static str {
        match self {
            TorrentState::Starting => "Starting",
            TorrentState::Metadata => "Fetching metadata",
            TorrentState::Downloading => "Downloading",
            TorrentState::Stalled => "Stalled",
            TorrentState::Seeding => "Seeding",
            TorrentState::Completed => "Completed",
            TorrentState::Stopped => "Stopped",
            TorrentState::Queued => "Queued",
            TorrentState::Checking => "Checking",
            TorrentState::Moving => "Moving",
            TorrentState::Error => "Error",
            TorrentState::MissingFiles => "Missing files",
            TorrentState::DriveUnavailable => "Drive unavailable",
        }
    }

    pub fn is_errored(self) -> bool {
        matches!(
            self,
            TorrentState::Error | TorrentState::MissingFiles | TorrentState::DriveUnavailable
        )
    }

    pub fn is_paused(self) -> bool {
        matches!(self, TorrentState::Completed | TorrentState::Stopped)
    }
}

/// The sidebar's Status filters, in qBittorrent's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StatusFilter {
    #[default]
    All,
    Downloading,
    Seeding,
    Completed,
    Stopped,
    Active,
    Inactive,
    Stalled,
    Checking,
    Errored,
}

impl StatusFilter {
    pub const ALL: [StatusFilter; 10] = [
        StatusFilter::All,
        StatusFilter::Downloading,
        StatusFilter::Seeding,
        StatusFilter::Completed,
        StatusFilter::Stopped,
        StatusFilter::Active,
        StatusFilter::Inactive,
        StatusFilter::Stalled,
        StatusFilter::Checking,
        StatusFilter::Errored,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StatusFilter::All => "All",
            StatusFilter::Downloading => "Downloading",
            StatusFilter::Seeding => "Seeding",
            StatusFilter::Completed => "Completed",
            StatusFilter::Stopped => "Stopped",
            StatusFilter::Active => "Active",
            StatusFilter::Inactive => "Inactive",
            StatusFilter::Stalled => "Stalled",
            StatusFilter::Checking => "Checking",
            StatusFilter::Errored => "Errored",
        }
    }

    pub fn matches(self, r: &TorrentRow) -> bool {
        use TorrentState as S;
        let active = r.down_bps > 0 || r.up_bps > 0;
        match self {
            StatusFilter::All => true,
            StatusFilter::Downloading => {
                matches!(
                    r.state,
                    S::Downloading | S::Stalled | S::Metadata | S::Queued
                ) && r.progress < 1.0
            }
            StatusFilter::Seeding => r.state == S::Seeding,
            StatusFilter::Completed => r.progress >= 1.0,
            StatusFilter::Stopped => r.state.is_paused(),
            StatusFilter::Active => active,
            StatusFilter::Inactive => !active,
            StatusFilter::Stalled => r.state == S::Stalled,
            StatusFilter::Checking => r.state == S::Checking,
            StatusFilter::Errored => r.state.is_errored(),
        }
    }
}

/// One transfer-list row: every one of qBittorrent's 38 columns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TorrentRow {
    pub id: TorrentId,
    /// Queue position, 1-based, when queued or downloading with queueing on.
    pub queue: Option<u32>,
    pub name: String,
    /// Size of the selected files.
    pub size: u64,
    pub total_size: u64,
    /// 0..1.
    pub progress: f32,
    pub state: TorrentState,
    pub seeds: u32,
    pub seeds_total: u32,
    pub peers: u32,
    pub peers_total: u32,
    pub down_bps: u64,
    pub up_bps: u64,
    pub eta_secs: Option<u64>,
    pub ratio: f32,
    pub popularity: f32,
    pub category: String,
    pub tags: Vec<String>,
    /// Unix seconds.
    pub added_on: i64,
    pub completed_on: Option<i64>,
    /// The tracker host currently working, or empty.
    pub tracker: String,
    pub down_limit: u64,
    pub up_limit: u64,
    pub downloaded: u64,
    pub uploaded: u64,
    pub session_downloaded: u64,
    pub session_uploaded: u64,
    pub remaining: u64,
    pub time_active_secs: u64,
    pub save_path: String,
    pub completed: u64,
    pub ratio_limit: Option<f32>,
    pub last_seen_complete: Option<i64>,
    pub last_activity: Option<i64>,
    pub availability: f32,
    pub incomplete_save_path: String,
    pub hash_v1: String,
    pub hash_v2: String,
    pub reannounce_in_secs: Option<u64>,
    pub private: bool,
    pub created_on: Option<i64>,
    /// An explanation for error states (missing path, tracker message).
    pub error: Option<String>,
}

/// Everything the status bar and tray header show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionStatus {
    pub down_bps: u64,
    pub up_bps: u64,
    pub session_downloaded: u64,
    pub session_uploaded: u64,
    pub dht_nodes: u32,
    pub free_space: Option<u64>,
    /// Memory the engine process uses (private bytes), and the cap it holds its buffers to.
    pub engine_memory: u64,
    pub memory_cap: u64,
    pub port: Option<u16>,
    pub port_open: Option<bool>,
    pub slow_mode: bool,
    pub connected_peers: u32,
    /// "Starting torrents 240 of 735" while waves run.
    pub starting: Option<(u32, u32)>,
    pub torrents: u32,
    pub active: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Category {
    pub name: String,
    pub save_path: String,
    /// Index into the theme's category colours.
    pub color: u8,
}

/// Sidebar contents besides Status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Lists {
    pub categories: Vec<Category>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileRow {
    pub index: u32,
    pub path: String,
    pub size: u64,
    pub progress: f32,
    pub priority: FilePriority,
    /// docs/missing-files.md: present on disk?
    pub missing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilePriority {
    Skip,
    Normal,
    High,
    Maximum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackerRow {
    pub url: String,
    pub status: String,
    pub message: String,
    pub peers: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerRow {
    pub addr: String,
    pub client: String,
    pub progress: f32,
    pub down_bps: u64,
    pub up_bps: u64,
    pub downloaded: u64,
    pub uploaded: u64,
    pub state: String,
}

/// Details tabs for one torrent, fetched only for the selected one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TorrentDetails {
    pub id: TorrentId,
    pub piece_count: u32,
    pub piece_size: u64,
    pub pieces_have: u32,
    /// One bit per piece, most significant bit first.
    pub pieces: Vec<u8>,
    pub comment: String,
    pub created_by: String,
    pub files: Vec<FileRow>,
    pub trackers: Vec<TrackerRow>,
    pub peers: Vec<PeerRow>,
    pub http_sources: Vec<String>,
}

/// One point a second for the Speed tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpeedPoint {
    pub down_bps: u64,
    pub up_bps: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddSource {
    Magnet {
        uri: String,
    },
    Url {
        url: String,
    },
    File {
        path: String,
    },
    /// .torrent bytes, base64 in JSON.
    Bytes {
        data: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AddOptions {
    pub save_path: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub paused: bool,
    pub sequential: bool,
    /// Selected file indices; None means all.
    pub only_files: Option<Vec<u32>>,
    /// Peers to connect to right away, `ip:port` (qBittorrent's "Add peers…").
    #[serde(default)]
    pub peers: Vec<String>,
    /// Keep an existing "added on" time (importers, the Web API, fixtures).
    #[serde(default)]
    pub added_on: Option<i64>,
    /// None: the setting (Settings › Downloads).
    #[serde(default)]
    pub content_layout: Option<ContentLayout>,
    /// None: the setting.
    #[serde(default)]
    pub stop_condition: Option<StopCondition>,
    /// None: the setting.
    #[serde(default)]
    pub add_to_top: Option<bool>,
    /// qBittorrent's "Skip hash check": trust the files on disk.
    #[serde(default)]
    pub skip_check: bool,
}

/// Where a torrent's files go inside the save folder (qBittorrent's
/// "Torrent content layout").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContentLayout {
    /// A folder named after the torrent when it has several files.
    #[default]
    Original,
    /// Always a folder, even for one file.
    Subfolder,
    /// Never a folder.
    NoSubfolder,
}

impl ContentLayout {
    pub const ALL: [ContentLayout; 3] = [
        ContentLayout::Original,
        ContentLayout::Subfolder,
        ContentLayout::NoSubfolder,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ContentLayout::Original => "Original",
            ContentLayout::Subfolder => "Create subfolder",
            ContentLayout::NoSubfolder => "Don't create subfolder",
        }
    }
}

/// When a new torrent stops by itself (qBittorrent's "Stop condition").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StopCondition {
    #[default]
    None,
    MetadataReceived,
    FilesChecked,
}

impl StopCondition {
    pub const ALL: [StopCondition; 3] = [
        StopCondition::None,
        StopCondition::MetadataReceived,
        StopCondition::FilesChecked,
    ];
    pub fn label(self) -> &'static str {
        match self {
            StopCondition::None => "None",
            StopCondition::MetadataReceived => "Metadata received",
            StopCondition::FilesChecked => "Files checked",
        }
    }
}

/// One file in the Add torrent dialog.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreviewFile {
    /// Path inside the torrent, `/`-separated, without the torrent's own folder.
    pub path: String,
    pub size: u64,
}

/// What the Add torrent dialog shows once the metadata is known.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TorrentPreview {
    pub name: String,
    pub total_size: u64,
    pub files: Vec<PreviewFile>,
    pub info_hash_v1: String,
    pub info_hash_v2: Option<String>,
    pub comment: Option<String>,
    pub created_on: Option<i64>,
    pub created_by: Option<String>,
    pub private: bool,
    pub piece_size: u64,
    pub pieces: u32,
    pub trackers: Vec<String>,
    /// Already in the list under this id.
    pub existing: Option<TorrentId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PendingState {
    /// A magnet link or URL: fetching the metadata.
    Resolving,
    Ready {
        preview: Box<TorrentPreview>,
    },
    Failed {
        error: String,
    },
}

/// A torrent waiting in the Add torrent dialog.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingAdd {
    pub token: u64,
    /// What was opened: the file name, or the magnet's display name.
    pub label: String,
    pub is_magnet: bool,
    pub state: PendingState,
    /// The dialog's starting values, from the settings.
    pub defaults: AddOptions,
}

/// Which of the two tTorrent opens today (Settings › Behaviour).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Associations {
    pub torrent_files: bool,
    pub magnet_links: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagMode {
    Add,
    Remove,
    Set,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMove {
    Top,
    Up,
    Down,
    Bottom,
}

/// Who is asking (for logs and the MCP activity record).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    Window,
    SecondLaunch,
    Cli,
    WebApi,
    Mcp { agent: String },
    Bench,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Everything the window needs for its first frame.
    Snapshot,
    /// Start receiving events on this connection.
    Subscribe,
    Add {
        source: AddSource,
        options: AddOptions,
    },
    /// Files and links opened from the system, a second launch, Ctrl+O or
    /// the link dialog: shown in the Add torrent dialog when that setting is
    /// on, added straight away otherwise.
    Open {
        sources: Vec<AddSource>,
    },
    /// The Add torrent dialog's OK.
    ConfirmAdd {
        token: u64,
        options: AddOptions,
        /// "Never show again": turns the dialog off.
        never_show_again: bool,
    },
    /// The Add torrent dialog's Cancel.
    CancelAdd {
        token: u64,
    },
    /// The dialog queue (the window asks once after subscribing, so an item
    /// queued between its snapshot and its subscription isn't missed).
    PendingAdds,
    /// Free space where a path lives (the dialog's size line).
    FreeSpace {
        path: String,
    },
    /// Does tTorrent open .torrent files and magnet links?
    Associations,
    /// Make tTorrent the default, the way the system allows.
    MakeDefault,
    Pause {
        ids: Vec<TorrentId>,
    },
    Resume {
        ids: Vec<TorrentId>,
    },
    ForceStart {
        ids: Vec<TorrentId>,
    },
    PauseAll,
    ResumeAll,
    /// Pause everything and resume after `secs` (tray › Pause for 1 hour).
    PauseFor {
        secs: u64,
    },
    Remove {
        ids: Vec<TorrentId>,
        delete_files: bool,
    },
    Recheck {
        ids: Vec<TorrentId>,
    },
    Reannounce {
        ids: Vec<TorrentId>,
    },
    SetCategory {
        ids: Vec<TorrentId>,
        category: Option<String>,
    },
    CreateCategory {
        name: String,
        save_path: Option<String>,
    },
    SetTags {
        ids: Vec<TorrentId>,
        tags: Vec<String>,
        mode: TagMode,
    },
    Rename {
        id: TorrentId,
        name: String,
    },
    SetLimits {
        /// None: the global limits.
        ids: Option<Vec<TorrentId>>,
        down_bps: Option<u64>,
        up_bps: Option<u64>,
    },
    SetSlowMode {
        on: bool,
    },
    QueueMove {
        ids: Vec<TorrentId>,
        to: QueueMove,
    },
    Details {
        id: TorrentId,
    },
    SpeedHistory,
    GetSettings,
    SetSettings {
        settings: EngineSettingsBox,
    },
    GetUiPrefs,
    SetUiPrefs {
        prefs: crate::UiPrefs,
    },
    /// A second launch or the tray asks for the window.
    ShowWindow,
    /// The window process tells the engine it is going away (closed to the tray).
    WindowClosing,
    /// Close the window to the tray (tray menu, benchmarks).
    HideWindow,
    /// Copy text for the Torrent › Copy submenu (magnet link, hashes).
    MagnetLink {
        id: TorrentId,
    },
    OpenFolder {
        id: TorrentId,
    },
    /// Exit everything (tray › Exit, Ctrl+Q).
    Quit,
}

/// Boxed so `Request` stays small.
pub type EngineSettingsBox = Box<crate::EngineSettings>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub rows: Vec<TorrentRow>,
    pub status: SessionStatus,
    pub lists: Lists,
    pub prefs: crate::UiPrefs,
    /// True when the rows come from the stored snapshot and torrents are still starting.
    pub from_snapshot: bool,
    /// Torrents waiting in the Add torrent dialog.
    #[serde(default)]
    pub pending: Vec<PendingAdd>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Ok,
    Snapshot(Box<Snapshot>),
    Added {
        id: TorrentId,
        name: String,
    },
    Details(Box<TorrentDetails>),
    SpeedHistory {
        points: Vec<SpeedPoint>,
    },
    Settings(EngineSettingsBox),
    UiPrefs(Box<crate::UiPrefs>),
    Text {
        text: String,
    },
    FreeSpace {
        bytes: Option<u64>,
    },
    PendingAdds {
        items: Vec<PendingAdd>,
    },
    Associations(Associations),
    /// `MakeDefault`'s outcome; `note` says what happens next, if anything.
    MadeDefault {
        status: Associations,
        note: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// Rows that changed since the last event, and rows that left.
    Rows {
        upsert: Vec<TorrentRow>,
        removed: Vec<TorrentId>,
    },
    Status(SessionStatus),
    Lists(Lists),
    /// A short message for the status bar ("Added ubuntu-24.04.iso").
    Notice {
        text: String,
    },
    /// Bring the window to the front.
    Activate,
    /// Close the window; the engine keeps running.
    CloseWindow,
    /// Run a window command from the tray: "add-file", "add-magnet", "settings".
    Command {
        name: String,
    },
    /// The Add torrent dialog's queue changed (new, resolved, gone).
    PendingAdds {
        items: Vec<PendingAdd>,
    },
    /// The engine is exiting; the window closes too.
    Exiting,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello { version: u32, client: ClientKind },
    Request { id: u64, request: Request },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ServerMsg {
    Hello {
        version: u32,
        engine_pid: u32,
    },
    Reply {
        id: u64,
        result: Result<Response, String>,
    },
    Event {
        event: Event,
    },
}
