//! The window's data, apart from drawing: rows by id, the filtered and sorted
//! view, selection, and sidebar counts. No GPUI here, so it is unit-tested.

use std::collections::{BTreeSet, HashMap};
use tt_ipc::{Column, Lists, SessionStatus, StatusFilter, TorrentId, TorrentRow};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum SideFilter {
    #[default]
    None,
    Category(String),
    /// Torrents without a category.
    Uncategorized,
    Tag(String),
    /// Torrents without tags.
    Untagged,
    Tracker(String),
    Trackerless,
}

#[derive(Default)]
pub struct Model {
    pub rows: HashMap<TorrentId, TorrentRow>,
    /// Ids in display order after filter and sort.
    pub view: Vec<TorrentId>,
    pub selected: BTreeSet<TorrentId>,
    /// The row keyboard focus and Shift-ranges start from.
    pub cursor: Option<TorrentId>,
    pub status_filter: StatusFilter,
    pub side_filter: SideFilter,
    pub search: String,
    pub sort: Column,
    pub sort_desc: bool,
    pub status: SessionStatus,
    pub lists: Lists,
    pub from_snapshot: bool,
}

impl Model {
    pub fn set_rows(&mut self, rows: Vec<TorrentRow>) {
        self.rows = rows.into_iter().map(|r| (r.id, r)).collect();
        self.selected.retain(|id| self.rows.contains_key(id));
        self.refresh();
    }

    /// Apply an engine delta. Returns true if the visible order may have changed.
    pub fn apply(&mut self, upsert: Vec<TorrentRow>, removed: &[TorrentId]) {
        for r in upsert {
            self.rows.insert(r.id, r);
        }
        for id in removed {
            self.rows.remove(id);
            self.selected.remove(id);
            if self.cursor == Some(*id) {
                self.cursor = None;
            }
        }
        self.refresh();
    }

    fn matches(&self, r: &TorrentRow) -> bool {
        if !self.status_filter.matches(r) {
            return false;
        }
        let side = match &self.side_filter {
            SideFilter::None => true,
            SideFilter::Category(c) => &r.category == c,
            SideFilter::Uncategorized => r.category.is_empty(),
            SideFilter::Tag(t) => r.tags.contains(t),
            SideFilter::Untagged => r.tags.is_empty(),
            SideFilter::Tracker(t) => &r.tracker == t,
            SideFilter::Trackerless => r.tracker.is_empty(),
        };
        if !side {
            return false;
        }
        if self.search.is_empty() {
            return true;
        }
        // Every word must appear (case-insensitive), like qBittorrent's filter box.
        let name = r.name.to_lowercase();
        self.search
            .to_lowercase()
            .split_whitespace()
            .all(|w| name.contains(w))
    }

    /// Recompute the view. Row order never jumps for rows whose sort key didn't change:
    /// the sort is total (ties broken by id).
    pub fn refresh(&mut self) {
        let mut v: Vec<&TorrentRow> = self.rows.values().filter(|r| self.matches(r)).collect();
        let (col, desc) = (self.sort, self.sort_desc);
        v.sort_by(|a, b| {
            let o = col.cmp(a, b);
            if desc { o.reverse() } else { o }
        });
        self.view = v.iter().map(|r| r.id).collect();
    }

    /// Click a header: same column flips; a new one starts descending for numbers
    /// and ascending for text (DESIGN.md).
    pub fn sort_by(&mut self, col: Column) {
        if self.sort == col {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort = col;
            self.sort_desc = !col.is_text();
        }
        self.refresh();
    }

    pub fn status_count(&self, f: StatusFilter) -> usize {
        self.rows.values().filter(|r| f.matches(r)).count()
    }

    pub fn category_count(&self, c: &str) -> usize {
        self.rows.values().filter(|r| r.category == c).count()
    }

    pub fn tag_count(&self, t: &str) -> usize {
        self.rows
            .values()
            .filter(|r| r.tags.iter().any(|x| x == t))
            .count()
    }

    /// Trackers by host with counts, plus the trackerless count.
    pub fn trackers(&self) -> (Vec<(String, usize)>, usize) {
        let mut m: HashMap<&str, usize> = HashMap::new();
        let mut none = 0;
        for r in self.rows.values() {
            if r.tracker.is_empty() {
                none += 1;
            } else {
                *m.entry(r.tracker.as_str()).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = m.into_iter().map(|(k, n)| (k.to_owned(), n)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        (v, none)
    }

    pub fn row_at(&self, ix: usize) -> Option<&TorrentRow> {
        self.view.get(ix).and_then(|id| self.rows.get(id))
    }

    pub fn index_of(&self, id: TorrentId) -> Option<usize> {
        self.view.iter().position(|&x| x == id)
    }

    /// Click (with Ctrl or Shift as given) on the row at `ix`.
    pub fn click(&mut self, ix: usize, ctrl: bool, shift: bool) {
        let Some(&id) = self.view.get(ix) else { return };
        if shift {
            let from = self.cursor.and_then(|c| self.index_of(c)).unwrap_or(ix);
            let (a, b) = if from <= ix { (from, ix) } else { (ix, from) };
            if !ctrl {
                self.selected.clear();
            }
            self.selected.extend(self.view[a..=b].iter().copied());
        } else if ctrl {
            if !self.selected.remove(&id) {
                self.selected.insert(id);
            }
            self.cursor = Some(id);
        } else {
            self.selected.clear();
            self.selected.insert(id);
            self.cursor = Some(id);
        }
    }

    /// Right-click: a row outside the selection becomes the selection.
    pub fn context_click(&mut self, ix: usize) {
        if let Some(&id) = self.view.get(ix)
            && !self.selected.contains(&id)
        {
            self.click(ix, false, false);
        }
    }

    /// Arrow keys, Home, End: move the cursor (and selection) by `delta` rows.
    pub fn move_cursor(&mut self, delta: isize, extend: bool) {
        if self.view.is_empty() {
            return;
        }
        let cur = self.cursor.and_then(|c| self.index_of(c));
        let next = match cur {
            None => 0,
            Some(i) => (i as isize + delta).clamp(0, self.view.len() as isize - 1) as usize,
        };
        if extend {
            let id = self.view[next];
            self.selected.insert(id);
            self.cursor = Some(id);
        } else {
            self.click(next, false, false);
        }
    }

    pub fn select_all(&mut self) {
        self.selected = self.view.iter().copied().collect();
    }

    pub fn selected_ids(&self) -> Vec<TorrentId> {
        self.selected.iter().copied().collect()
    }

    /// The torrent the details panel shows: the cursor if selected, else the first selected.
    pub fn focused(&self) -> Option<&TorrentRow> {
        self.cursor
            .filter(|c| self.selected.contains(c))
            .or_else(|| self.selected.iter().next().copied())
            .and_then(|id| self.rows.get(&id))
    }

    /// True when every selected torrent is stopped (the menu says Start, else Stop).
    pub fn selection_paused(&self) -> bool {
        !self.selected.is_empty()
            && self
                .selected
                .iter()
                .filter_map(|id| self.rows.get(id))
                .all(|r| r.state.is_paused())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tt_ipc::TorrentState;

    fn row(id: u32, name: &str, state: TorrentState, progress: f32, added: i64) -> TorrentRow {
        TorrentRow {
            id,
            name: name.into(),
            state,
            progress,
            added_on: added,
            ..Default::default()
        }
    }

    fn model() -> Model {
        let mut m = Model {
            sort: Column::AddedOn,
            sort_desc: true,
            ..Default::default()
        };
        let mut a = row(
            1,
            "ubuntu-24.04.3-desktop-amd64.iso",
            TorrentState::Downloading,
            0.6,
            30,
        );
        a.category = "Linux ISOs".into();
        a.down_bps = 100;
        let mut b = row(
            2,
            "Big Buck Bunny (2008) 4K",
            TorrentState::Seeding,
            1.0,
            20,
        );
        b.tags = vec!["Open movies".into()];
        b.tracker = "bt.example.org".into();
        let c = row(
            3,
            "debian-13.2.0-amd64-netinst.iso",
            TorrentState::Stopped,
            0.3,
            10,
        );
        m.set_rows(vec![a, b, c]);
        m
    }

    #[test]
    fn default_sort_is_newest_first() {
        assert_eq!(model().view, vec![1, 2, 3]);
    }

    #[test]
    fn filters_combine() {
        let mut m = model();
        m.status_filter = StatusFilter::Downloading;
        m.refresh();
        assert_eq!(m.view, vec![1]);
        m.status_filter = StatusFilter::All;
        m.side_filter = SideFilter::Tag("Open movies".into());
        m.refresh();
        assert_eq!(m.view, vec![2]);
        m.side_filter = SideFilter::None;
        m.search = "AMD64 iso".into();
        m.refresh();
        assert_eq!(m.view, vec![1, 3]);
        assert_eq!(m.status_count(StatusFilter::Stopped), 1);
        assert_eq!(m.status_count(StatusFilter::Active), 1);
        assert_eq!(m.trackers(), (vec![("bt.example.org".into(), 1)], 2));
    }

    #[test]
    fn sort_toggles_and_text_starts_ascending() {
        let mut m = model();
        m.sort_by(Column::Name);
        assert!(!m.sort_desc);
        assert_eq!(m.view, vec![2, 3, 1]);
        m.sort_by(Column::Name);
        assert_eq!(m.view, vec![1, 3, 2]);
        m.sort_by(Column::Progress);
        assert!(m.sort_desc);
        assert_eq!(m.view[0], 2);
    }

    #[test]
    fn selection_with_modifiers_and_keys() {
        let mut m = model();
        m.click(0, false, false);
        m.click(2, false, true);
        assert_eq!(m.selected_ids(), vec![1, 2, 3]);
        m.click(1, true, false);
        assert_eq!(m.selected_ids(), vec![1, 3]);
        m.click(0, false, false);
        m.move_cursor(1, false);
        assert_eq!(m.selected_ids(), vec![2]);
        m.move_cursor(10, false);
        assert_eq!(m.focused().map(|r| r.id), Some(3));
        assert!(m.selection_paused());
        m.context_click(0);
        assert_eq!(m.selected_ids(), vec![1]);
    }

    #[test]
    fn deltas_keep_selection_and_drop_removed() {
        let mut m = model();
        m.click(0, false, false);
        let mut changed = m.rows[&1].clone();
        changed.progress = 0.7;
        m.apply(vec![changed], &[3]);
        assert_eq!(m.view, vec![1, 2]);
        assert_eq!(m.selected_ids(), vec![1]);
        m.apply(Vec::new(), &[1]);
        assert!(m.selected.is_empty());
        assert_eq!(m.cursor, None);
    }
}
