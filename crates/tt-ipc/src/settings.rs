//! Settings. Every limit keeps qBittorrent's default and stays a setting
//! (AGENTS.md: no feature cuts for memory).

use crate::Column;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Encryption {
    /// qBittorrent's default: allow encryption.
    #[default]
    Allow,
    Require,
    Disable,
}

/// How several seeding limits combine (qBittorrent's "Any of the above" / "All the above").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShareLimitsMode {
    #[default]
    MatchAny,
    MatchAll,
}

impl ShareLimitsMode {
    pub const ALL: [ShareLimitsMode; 2] = [ShareLimitsMode::MatchAny, ShareLimitsMode::MatchAll];
    pub fn label(self) -> &'static str {
        match self {
            ShareLimitsMode::MatchAny => "Any of the above",
            ShareLimitsMode::MatchAll => "All the above",
        }
    }
}

/// What happens when a seeding limit is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShareLimitAction {
    #[default]
    Stop,
    Remove,
    RemoveWithFiles,
    /// librqbit has no super seeding yet; treated as Stop.
    SuperSeeding,
}

impl ShareLimitAction {
    /// The choices the window offers (super seeding waits for librqbit).
    pub const OFFERED: [ShareLimitAction; 3] = [
        ShareLimitAction::Stop,
        ShareLimitAction::Remove,
        ShareLimitAction::RemoveWithFiles,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ShareLimitAction::Stop => "Stop torrent",
            ShareLimitAction::Remove => "Remove torrent",
            ShareLimitAction::RemoveWithFiles => "Remove torrent and its files",
            ShareLimitAction::SuperSeeding => "Enable super seeding",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineSettings {
    // Downloads
    pub default_save_path: String,
    pub incomplete_path: Option<String>,
    pub start_paused: bool,
    // Downloads › When adding a torrent (qBittorrent's names and defaults)
    /// "Display torrent content and some options": the Add torrent dialog.
    pub show_add_dialog: bool,
    /// "Bring torrent dialog to the front".
    pub add_dialog_front: bool,
    pub content_layout: crate::ContentLayout,
    /// "Add to top of queue".
    pub add_to_top: bool,
    pub stop_condition: crate::StopCondition,
    // Connection
    /// 0 picks a random port on first start and keeps it.
    pub listen_port: u16,
    pub upnp: bool,
    pub max_connections: u32,
    pub max_connections_per_torrent: u32,
    pub max_uploads: u32,
    pub max_uploads_per_torrent: u32,
    pub bind_interface: Option<String>,
    // Speed, bytes per second, 0 = unlimited
    pub down_limit: u64,
    pub up_limit: u64,
    pub alt_down_limit: u64,
    pub alt_up_limit: u64,
    pub slow_mode: bool,
    // BitTorrent
    pub dht: bool,
    pub pex: bool,
    pub lsd: bool,
    pub encryption: Encryption,
    /// "Torrent Queueing": off by default, as in qBittorrent.
    pub queueing: bool,
    /// Maximum active downloads, uploads and torrents; -1 is ∞ (qBittorrent's spin boxes).
    pub max_active_downloads: i32,
    pub max_active_uploads: i32,
    pub max_active_torrents: i32,
    /// "Do not count slow torrents in these limits".
    pub ignore_slow_torrents: bool,
    /// "Download rate threshold" and "Upload rate threshold", KiB/s.
    pub slow_down_kib: u32,
    pub slow_up_kib: u32,
    /// "Torrent inactivity timer", seconds.
    pub slow_timer_secs: u32,
    // Seeding limits; each is off while None.
    /// "When ratio reaches".
    pub ratio_limit: Option<f32>,
    /// "When total seeding time reaches", minutes.
    pub seeding_time_limit_mins: Option<u32>,
    /// "When inactive seeding time reaches", minutes.
    pub inactive_seeding_time_limit_mins: Option<u32>,
    /// "Any of the above" or "All the above".
    pub share_limits_mode: ShareLimitsMode,
    /// "then …".
    pub share_limit_action: ShareLimitAction,
    // Advanced
    pub max_active_checking: u32,
    pub file_pool_size: u32,
    /// MiB, qBittorrent's "Physical memory (RAM) usage limit".
    pub memory_limit_mb: u32,
    /// Peers remembered per torrent (libtorrent's max_peerlist_size).
    pub max_peerlist_size: u32,
    /// No DHT, LSD, UPnP or trackers: for benchmarks and fixtures on loopback.
    /// Set by `--offline`, never saved.
    #[serde(skip)]
    pub offline: bool,
    // Behaviour
    /// docs/startup.md: start torrents in waves.
    pub start_gradually: bool,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub confirm_remove: bool,
    // AI agents (docs/mcp.md), off by default
    pub mcp_enabled: bool,
    pub mcp_port: u16,
    // Web API, off by default as in qBittorrent
    pub webui_enabled: bool,
    pub webui_port: u16,
    pub webui_username: String,
    /// PBKDF2 hash, never the password.
    pub webui_password_hash: String,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            default_save_path: String::new(),
            incomplete_path: None,
            start_paused: false,
            show_add_dialog: true,
            add_dialog_front: true,
            content_layout: crate::ContentLayout::Original,
            add_to_top: false,
            stop_condition: crate::StopCondition::None,
            listen_port: 0,
            upnp: true,
            max_connections: 500,
            max_connections_per_torrent: 100,
            max_uploads: 20,
            max_uploads_per_torrent: 4,
            bind_interface: None,
            down_limit: 0,
            up_limit: 0,
            alt_down_limit: 10 * 1024,
            alt_up_limit: 10 * 1024,
            slow_mode: false,
            dht: true,
            pex: true,
            lsd: true,
            encryption: Encryption::Allow,
            queueing: false,
            max_active_downloads: 3,
            max_active_uploads: 3,
            max_active_torrents: 5,
            ignore_slow_torrents: false,
            slow_down_kib: 2,
            slow_up_kib: 2,
            slow_timer_secs: 60,
            ratio_limit: None,
            seeding_time_limit_mins: None,
            inactive_seeding_time_limit_mins: None,
            share_limits_mode: ShareLimitsMode::MatchAny,
            share_limit_action: ShareLimitAction::Stop,
            max_active_checking: 1,
            file_pool_size: 100,
            memory_limit_mb: 1024,
            max_peerlist_size: 3000,
            offline: false,
            start_gradually: true,
            close_to_tray: true,
            start_minimized: false,
            confirm_remove: true,
            mcp_enabled: false,
            mcp_port: 7812,
            webui_enabled: false,
            webui_port: 8080,
            webui_username: "admin".into(),
            webui_password_hash: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolbarLabel {
    Hidden,
    Icon,
    IconAndLabel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolbarItem {
    AddTorrent,
    AddMagnet,
    Resume,
    Pause,
    Remove,
    SlowMode,
    QueueUp,
    QueueDown,
    CreateTorrent,
    Rss,
    Search,
    Settings,
    Statistics,
    Log,
    Theme,
}

impl ToolbarItem {
    pub const ALL: [ToolbarItem; 15] = [
        ToolbarItem::AddTorrent,
        ToolbarItem::AddMagnet,
        ToolbarItem::Resume,
        ToolbarItem::Pause,
        ToolbarItem::Remove,
        ToolbarItem::SlowMode,
        ToolbarItem::QueueUp,
        ToolbarItem::QueueDown,
        ToolbarItem::CreateTorrent,
        ToolbarItem::Rss,
        ToolbarItem::Search,
        ToolbarItem::Settings,
        ToolbarItem::Statistics,
        ToolbarItem::Log,
        ToolbarItem::Theme,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ToolbarItem::AddTorrent => "Add torrent",
            ToolbarItem::AddMagnet => "Add magnet link",
            ToolbarItem::Resume => "Start",
            ToolbarItem::Pause => "Stop",
            ToolbarItem::Remove => "Remove",
            ToolbarItem::SlowMode => "Slow mode",
            ToolbarItem::QueueUp => "Move up",
            ToolbarItem::QueueDown => "Move down",
            ToolbarItem::CreateTorrent => "Create torrent",
            ToolbarItem::Rss => "RSS",
            ToolbarItem::Search => "Filter",
            ToolbarItem::Settings => "Settings",
            ToolbarItem::Statistics => "Statistics",
            ToolbarItem::Log => "Log",
            ToolbarItem::Theme => "Light or dark",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColumnPref {
    pub column: Column,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailsTab {
    General,
    Trackers,
    Peers,
    HttpSources,
    Content,
    Speed,
}

impl DetailsTab {
    pub const ALL: [DetailsTab; 6] = [
        DetailsTab::General,
        DetailsTab::Trackers,
        DetailsTab::Peers,
        DetailsTab::HttpSources,
        DetailsTab::Content,
        DetailsTab::Speed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DetailsTab::General => "General",
            DetailsTab::Trackers => "Trackers",
            DetailsTab::Peers => "Peers",
            DetailsTab::HttpSources => "HTTP sources",
            DetailsTab::Content => "Content",
            DetailsTab::Speed => "Speed",
        }
    }
}

/// Window preferences, stored by the engine so there is one database.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPrefs {
    pub mode: String,
    pub tint: String,
    pub accent: String,
    pub density: String,
    pub scale: f32,
    /// Shown columns in display order with widths.
    pub columns: Vec<ColumnPref>,
    pub sort: Column,
    pub sort_desc: bool,
    pub sidebar: bool,
    pub details: bool,
    pub details_tab: DetailsTab,
    pub details_height: f32,
    pub toolbar: Vec<(ToolbarItem, ToolbarLabel)>,
    pub window: Option<[f32; 4]>,
    pub maximized: bool,
    pub reduced_motion: bool,
    /// What the status bar shows (right-click it, or View › Status bar).
    pub status_items: Vec<StatusItem>,
}

/// One item of the status bar. Slow mode and "Starting torrents" always show
/// while they apply: they are state, not a choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusItem {
    FreeSpace,
    Dht,
    Download,
    Upload,
    /// Session totals beside the speeds.
    SessionTotals,
    Ratio,
    Peers,
    Memory,
    Port,
}

impl StatusItem {
    /// Menu and bar order.
    pub const ALL: [StatusItem; 9] = [
        StatusItem::FreeSpace,
        StatusItem::Dht,
        StatusItem::Download,
        StatusItem::Upload,
        StatusItem::SessionTotals,
        StatusItem::Ratio,
        StatusItem::Peers,
        StatusItem::Memory,
        StatusItem::Port,
    ];
    /// DESIGN.md › Layout: everything but session ratio and connected peers.
    pub const DEFAULT: [StatusItem; 7] = [
        StatusItem::FreeSpace,
        StatusItem::Dht,
        StatusItem::Download,
        StatusItem::Upload,
        StatusItem::SessionTotals,
        StatusItem::Memory,
        StatusItem::Port,
    ];
    pub fn label(self) -> &'static str {
        match self {
            StatusItem::FreeSpace => "Free space",
            StatusItem::Dht => "DHT nodes",
            StatusItem::Download => "Download speed",
            StatusItem::Upload => "Upload speed",
            StatusItem::SessionTotals => "Session totals",
            StatusItem::Ratio => "Session ratio",
            StatusItem::Peers => "Connected peers",
            StatusItem::Memory => "Memory",
            StatusItem::Port => "Port",
        }
    }
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            mode: "light".into(),
            tint: "neutral".into(),
            accent: "green".into(),
            density: "cozy".into(),
            scale: 1.0,
            columns: Column::DEFAULT_VISIBLE
                .iter()
                .map(|&column| ColumnPref {
                    column,
                    width: column.default_width(),
                })
                .collect(),
            sort: Column::AddedOn,
            sort_desc: true,
            sidebar: true,
            details: true,
            details_tab: DetailsTab::General,
            details_height: 260.0,
            toolbar: vec![
                (ToolbarItem::AddTorrent, ToolbarLabel::IconAndLabel),
                (ToolbarItem::AddMagnet, ToolbarLabel::Icon),
                (ToolbarItem::Resume, ToolbarLabel::Icon),
                (ToolbarItem::Pause, ToolbarLabel::Icon),
                (ToolbarItem::Remove, ToolbarLabel::Icon),
                (ToolbarItem::SlowMode, ToolbarLabel::Icon),
            ],
            window: None,
            maximized: false,
            status_items: StatusItem::DEFAULT.to_vec(),
            reduced_motion: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qbittorrent_defaults_kept() {
        let s = EngineSettings::default();
        assert_eq!(s.max_connections, 500);
        assert_eq!(s.max_connections_per_torrent, 100);
        assert_eq!(s.max_uploads, 20);
        assert_eq!(s.max_uploads_per_torrent, 4);
        assert_eq!(s.max_active_downloads, 3);
        assert_eq!(s.max_active_uploads, 3);
        assert_eq!(s.max_active_torrents, 5);
        assert_eq!(s.max_active_checking, 1);
        assert_eq!(s.file_pool_size, 100);
        assert_eq!(s.max_peerlist_size, 3000);
        assert!(s.dht && s.pex && s.lsd);
        assert!(!s.mcp_enabled && !s.webui_enabled);
    }

    #[test]
    fn prefs_round_trip_and_tolerate_missing_fields() {
        let p = UiPrefs::default();
        let json = serde_json::to_string(&p).unwrap_or_default();
        let back: UiPrefs = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(p, back);
        let partial: UiPrefs = serde_json::from_str(r#"{"mode":"dark"}"#).unwrap_or_default();
        assert_eq!(partial.mode, "dark");
        assert_eq!(partial.columns.len(), 11);
    }
}
