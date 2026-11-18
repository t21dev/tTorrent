//! The transfer list's 38 columns, in qBittorrent's order (transferlistmodel.h),
//! with their labels, alignment, default widths and sort order.

use crate::TorrentRow;
use crate::fmt;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    Queue,
    Name,
    Size,
    TotalSize,
    Progress,
    Status,
    Seeds,
    Peers,
    DownSpeed,
    UpSpeed,
    Eta,
    Ratio,
    Popularity,
    Category,
    Tags,
    #[default]
    AddedOn,
    CompletedOn,
    Tracker,
    DownLimit,
    UpLimit,
    Downloaded,
    Uploaded,
    SessionDownloaded,
    SessionUploaded,
    Remaining,
    TimeActive,
    SavePath,
    Completed,
    RatioLimit,
    LastSeenComplete,
    LastActivity,
    Availability,
    IncompleteSavePath,
    InfoHashV1,
    InfoHashV2,
    ReannounceIn,
    Private,
    CreatedOn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

impl Column {
    pub const ALL: [Column; 38] = [
        Column::Queue,
        Column::Name,
        Column::Size,
        Column::TotalSize,
        Column::Progress,
        Column::Status,
        Column::Seeds,
        Column::Peers,
        Column::DownSpeed,
        Column::UpSpeed,
        Column::Eta,
        Column::Ratio,
        Column::Popularity,
        Column::Category,
        Column::Tags,
        Column::AddedOn,
        Column::CompletedOn,
        Column::Tracker,
        Column::DownLimit,
        Column::UpLimit,
        Column::Downloaded,
        Column::Uploaded,
        Column::SessionDownloaded,
        Column::SessionUploaded,
        Column::Remaining,
        Column::TimeActive,
        Column::SavePath,
        Column::Completed,
        Column::RatioLimit,
        Column::LastSeenComplete,
        Column::LastActivity,
        Column::Availability,
        Column::IncompleteSavePath,
        Column::InfoHashV1,
        Column::InfoHashV2,
        Column::ReannounceIn,
        Column::Private,
        Column::CreatedOn,
    ];

    /// DESIGN.md › Layout: shown by default.
    pub const DEFAULT_VISIBLE: [Column; 11] = [
        Column::Name,
        Column::AddedOn,
        Column::Size,
        Column::Progress,
        Column::Status,
        Column::Downloaded,
        Column::Uploaded,
        Column::DownSpeed,
        Column::UpSpeed,
        Column::Eta,
        Column::Ratio,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Column::Queue => "#",
            Column::Name => "Name",
            Column::Size => "Size",
            Column::TotalSize => "Total size",
            Column::Progress => "Progress",
            Column::Status => "Status",
            Column::Seeds => "Seeds",
            Column::Peers => "Peers",
            Column::DownSpeed => "Down speed",
            Column::UpSpeed => "Up speed",
            Column::Eta => "ETA",
            Column::Ratio => "Ratio",
            Column::Popularity => "Popularity",
            Column::Category => "Category",
            Column::Tags => "Tags",
            Column::AddedOn => "Added on",
            Column::CompletedOn => "Completed on",
            Column::Tracker => "Tracker",
            Column::DownLimit => "Down limit",
            Column::UpLimit => "Up limit",
            Column::Downloaded => "Downloaded",
            Column::Uploaded => "Uploaded",
            Column::SessionDownloaded => "Session download",
            Column::SessionUploaded => "Session upload",
            Column::Remaining => "Remaining",
            Column::TimeActive => "Time active",
            Column::SavePath => "Save path",
            Column::Completed => "Completed",
            Column::RatioLimit => "Ratio limit",
            Column::LastSeenComplete => "Last seen complete",
            Column::LastActivity => "Last activity",
            Column::Availability => "Availability",
            Column::IncompleteSavePath => "Incomplete save path",
            Column::InfoHashV1 => "Info hash v1",
            Column::InfoHashV2 => "Info hash v2",
            Column::ReannounceIn => "Reannounce in",
            Column::Private => "Private",
            Column::CreatedOn => "Created on",
        }
    }

    /// Text columns sort ascending first, numbers descending first (DESIGN.md).
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Column::Name
                | Column::Status
                | Column::Category
                | Column::Tags
                | Column::Tracker
                | Column::SavePath
                | Column::IncompleteSavePath
                | Column::InfoHashV1
                | Column::InfoHashV2
                | Column::Private
        )
    }

    /// Columns allowed to ellipsize; every other cell must fit (TESTING.md layout matrix).
    pub fn ellipsizes(self) -> bool {
        matches!(
            self,
            Column::Name
                | Column::SavePath
                | Column::IncompleteSavePath
                | Column::Tracker
                | Column::InfoHashV1
                | Column::InfoHashV2
                | Column::Tags
                | Column::Category
        )
    }

    pub fn align(self) -> Align {
        if self.is_text() || self == Column::Progress {
            Align::Left
        } else {
            Align::Right
        }
    }

    /// Starting width in px at 100 % and Cozy; wide enough for header, arrow and value.
    pub fn default_width(self) -> f32 {
        match self {
            Column::Queue => 40.0,
            Column::Name => 300.0,
            Column::Progress => 150.0,
            Column::Status => 140.0,
            Column::AddedOn | Column::CompletedOn | Column::CreatedOn => 124.0,
            Column::LastSeenComplete | Column::LastActivity => 150.0,
            Column::SavePath | Column::IncompleteSavePath => 200.0,
            Column::InfoHashV1 | Column::InfoHashV2 => 300.0,
            Column::Tracker => 170.0,
            Column::Category | Column::Tags => 120.0,
            Column::SessionDownloaded | Column::SessionUploaded => 128.0,
            Column::Private | Column::Seeds | Column::Peers => 76.0,
            _ => 96.0,
        }
    }

    /// The cell's text (Progress shows its percentage beside the bar).
    pub fn text(self, r: &TorrentRow) -> String {
        match self {
            Column::Queue => r.queue.map(|q| q.to_string()).unwrap_or_else(|| "*".into()),
            Column::Name => r.name.clone(),
            Column::Size => fmt::size(r.size),
            Column::TotalSize => fmt::size(r.total_size),
            Column::Progress => fmt::percent(r.progress),
            Column::Status => r.state.label().to_string(),
            Column::Seeds => format!("{} ({})", r.seeds, r.seeds_total),
            Column::Peers => format!("{} ({})", r.peers, r.peers_total),
            Column::DownSpeed => fmt::speed_or_blank(r.down_bps),
            Column::UpSpeed => fmt::speed_or_blank(r.up_bps),
            Column::Eta => r.eta_secs.map(fmt::eta).unwrap_or_else(|| "∞".into()),
            Column::Ratio => fmt::ratio(r.ratio),
            Column::Popularity => format!("{:.2}", r.popularity),
            Column::Category => r.category.clone(),
            Column::Tags => r.tags.join(", "),
            Column::AddedOn => fmt::date(r.added_on),
            Column::CompletedOn => r.completed_on.map(fmt::date).unwrap_or_default(),
            Column::Tracker => r.tracker.clone(),
            Column::DownLimit => fmt::limit(r.down_limit),
            Column::UpLimit => fmt::limit(r.up_limit),
            Column::Downloaded => fmt::size(r.downloaded),
            Column::Uploaded => fmt::size(r.uploaded),
            Column::SessionDownloaded => fmt::size(r.session_downloaded),
            Column::SessionUploaded => fmt::size(r.session_uploaded),
            Column::Remaining => fmt::size(r.remaining),
            Column::TimeActive => fmt::eta(r.time_active_secs),
            Column::SavePath => r.save_path.clone(),
            Column::Completed => fmt::size(r.completed),
            Column::RatioLimit => r.ratio_limit.map(fmt::ratio).unwrap_or_default(),
            Column::LastSeenComplete => r.last_seen_complete.map(fmt::date).unwrap_or_default(),
            Column::LastActivity => r.last_activity.map(fmt::ago).unwrap_or_default(),
            Column::Availability => format!("{:.3}", r.availability),
            Column::IncompleteSavePath => r.incomplete_save_path.clone(),
            Column::InfoHashV1 => r.hash_v1.clone(),
            Column::InfoHashV2 => r.hash_v2.clone(),
            Column::ReannounceIn => r.reannounce_in_secs.map(fmt::eta).unwrap_or_default(),
            Column::Private => if r.private { "Yes" } else { "No" }.into(),
            Column::CreatedOn => r.created_on.map(fmt::date).unwrap_or_default(),
        }
    }

    /// Ascending comparison by this column (the caller flips it for descending).
    pub fn cmp(self, a: &TorrentRow, b: &TorrentRow) -> Ordering {
        fn f(x: f32, y: f32) -> Ordering {
            x.partial_cmp(&y).unwrap_or(Ordering::Equal)
        }
        let primary = match self {
            Column::Queue => a
                .queue
                .unwrap_or(u32::MAX)
                .cmp(&b.queue.unwrap_or(u32::MAX)),
            Column::Name => natural_cmp(&a.name, &b.name),
            Column::Size => a.size.cmp(&b.size),
            Column::TotalSize => a.total_size.cmp(&b.total_size),
            Column::Progress => f(a.progress, b.progress),
            Column::Status => a.state.label().cmp(b.state.label()),
            Column::Seeds => a.seeds.cmp(&b.seeds),
            Column::Peers => a.peers.cmp(&b.peers),
            Column::DownSpeed => a.down_bps.cmp(&b.down_bps),
            Column::UpSpeed => a.up_bps.cmp(&b.up_bps),
            Column::Eta => a
                .eta_secs
                .unwrap_or(u64::MAX)
                .cmp(&b.eta_secs.unwrap_or(u64::MAX)),
            Column::Ratio => f(a.ratio, b.ratio),
            Column::Popularity => f(a.popularity, b.popularity),
            Column::Category => natural_cmp(&a.category, &b.category),
            Column::Tags => natural_cmp(&a.tags.join(","), &b.tags.join(",")),
            Column::AddedOn => a.added_on.cmp(&b.added_on),
            Column::CompletedOn => a.completed_on.cmp(&b.completed_on),
            Column::Tracker => natural_cmp(&a.tracker, &b.tracker),
            Column::DownLimit => a.down_limit.cmp(&b.down_limit),
            Column::UpLimit => a.up_limit.cmp(&b.up_limit),
            Column::Downloaded => a.downloaded.cmp(&b.downloaded),
            Column::Uploaded => a.uploaded.cmp(&b.uploaded),
            Column::SessionDownloaded => a.session_downloaded.cmp(&b.session_downloaded),
            Column::SessionUploaded => a.session_uploaded.cmp(&b.session_uploaded),
            Column::Remaining => a.remaining.cmp(&b.remaining),
            Column::TimeActive => a.time_active_secs.cmp(&b.time_active_secs),
            Column::SavePath => natural_cmp(&a.save_path, &b.save_path),
            Column::Completed => a.completed.cmp(&b.completed),
            Column::RatioLimit => f(a.ratio_limit.unwrap_or(-1.0), b.ratio_limit.unwrap_or(-1.0)),
            Column::LastSeenComplete => a.last_seen_complete.cmp(&b.last_seen_complete),
            Column::LastActivity => a.last_activity.cmp(&b.last_activity),
            Column::Availability => f(a.availability, b.availability),
            Column::IncompleteSavePath => a.incomplete_save_path.cmp(&b.incomplete_save_path),
            Column::InfoHashV1 => a.hash_v1.cmp(&b.hash_v1),
            Column::InfoHashV2 => a.hash_v2.cmp(&b.hash_v2),
            Column::ReannounceIn => a.reannounce_in_secs.cmp(&b.reannounce_in_secs),
            Column::Private => a.private.cmp(&b.private),
            Column::CreatedOn => a.created_on.cmp(&b.created_on),
        };
        // A stable tie-break so rows never jump between refreshes.
        primary.then(a.id.cmp(&b.id))
    }
}

/// Case-insensitive comparison where runs of digits compare by value ("file 9" < "file 10").
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut n1 = String::new();
                while let Some(c) = x.peek().copied().filter(char::is_ascii_digit) {
                    n1.push(c);
                    x.next();
                }
                let mut n2 = String::new();
                while let Some(c) = y.peek().copied().filter(char::is_ascii_digit) {
                    n2.push(c);
                    y.next();
                }
                let t1 = n1.trim_start_matches('0');
                let t2 = n2.trim_start_matches('0');
                let o = t1.len().cmp(&t2.len()).then_with(|| t1.cmp(t2));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirty_eight_columns_in_order() {
        assert_eq!(Column::ALL.len(), 38);
        let mut sorted = Column::ALL;
        sorted.sort();
        assert_eq!(sorted, Column::ALL);
    }

    #[test]
    fn natural_order() {
        assert_eq!(natural_cmp("file 9", "file 10"), Ordering::Less);
        assert_eq!(natural_cmp("Alpha", "alpha"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "ab"), Ordering::Less);
    }

    proptest::proptest! {
        /// Sorting is a total order: antisymmetric for any pair of rows.
        #[test]
        fn sort_is_antisymmetric(a in 0u64..1000, b in 0u64..1000, ia in 0u32..5, ib in 0u32..5) {
            let r1 = TorrentRow { id: ia, size: a, name: format!("n{a}"), ..Default::default() };
            let r2 = TorrentRow { id: ib, size: b, name: format!("n{b}"), ..Default::default() };
            for c in Column::ALL {
                proptest::prop_assert_eq!(c.cmp(&r1, &r2), c.cmp(&r2, &r1).reverse());
            }
        }
    }
}
