//! Adding torrents: metadata first (a magnet is resolved without starting
//! anything), then the content layout and the options picked in the Add
//! torrent dialog (qBittorrent's "Display torrent content and some options").
//!
//! Files and links opened from the system arrive as `Request::Open`. With the
//! dialog on, each becomes a `PendingAdd` the window shows; the dialog's OK
//! sends `ConfirmAdd` with the options. With it off, they are added straight
//! away with the settings' defaults.

use super::*;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tt_ipc::{
    Associations, ContentLayout, PendingAdd, PendingState, PreviewFile, StopCondition,
    TorrentPreview,
};

/// A torrent waiting in the dialog, with what the engine keeps beside it.
pub(super) struct Pending {
    pub add: PendingAdd,
    pub source: AddSource,
    pub resolved: Option<Resolved>,
    pub task: Option<tokio::task::AbortHandle>,
}

/// A torrent whose metadata is known.
#[derive(Clone)]
pub(super) struct Resolved {
    pub preview: TorrentPreview,
    pub bytes: Vec<u8>,
    /// Peers met while resolving a magnet, handed to the real add.
    pub peers: Vec<SocketAddr>,
}

impl Engine {
    /// The session, waiting for it if a file was opened during start-up.
    pub(super) async fn session_ready(&self) -> Arc<Session> {
        loop {
            if let Some(s) = self.session.get() {
                return s.clone();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn existing(&self, info_hash: &str) -> Option<(TorrentId, String)> {
        lock(&self.inner)
            .recs
            .values()
            .find(|r| r.info_hash.eq_ignore_ascii_case(info_hash))
            .map(|r| (r.id, r.name.clone()))
    }

    /// Metadata for a source: read the file, or let librqbit download the
    /// URL or resolve the magnet with `list_only`, so nothing starts yet.
    pub(super) async fn resolve(&self, source: &AddSource) -> Result<Resolved> {
        let (bytes, peers) = match source {
            AddSource::Bytes { data } => (data.clone(), Vec::new()),
            AddSource::File { path } => (
                std::fs::read(path).with_context(|| format!("reading {path}"))?,
                Vec::new(),
            ),
            AddSource::Magnet { uri: link } | AddSource::Url { url: link } => {
                let session = self.session_ready().await;
                let opts = AddTorrentOptions {
                    list_only: true,
                    ..Default::default()
                };
                match session
                    .add_torrent(AddTorrent::from_url(link.clone()), Some(opts))
                    .await?
                {
                    AddTorrentResponse::ListOnly(r) => (r.torrent_bytes.to_vec(), r.seen_peers),
                    AddTorrentResponse::AlreadyManaged(_, h) => {
                        return Err(anyhow!(
                            "{} is already in the list",
                            h.name().unwrap_or_default()
                        ));
                    }
                    AddTorrentResponse::Added(..) => return Err(anyhow!("unexpected add")),
                }
            }
        };
        let mut preview = preview_of(&bytes)?;
        preview.existing = self.existing(&preview.info_hash_v1).map(|(id, _)| id);
        Ok(Resolved {
            preview,
            bytes,
            peers,
        })
    }

    /// Add straight away (MCP, the bench, the importer, the dialog turned off).
    pub(super) async fn add(
        self: &Arc<Self>,
        source: AddSource,
        options: AddOptions,
    ) -> Result<Response> {
        if let AddSource::Magnet { uri } = &source
            && let Some((id, name)) = magnet_hash(uri).and_then(|h| self.existing(&h))
        {
            return Err(already(&name, id));
        }
        let r = self.resolve(&source).await?;
        if let Some(id) = r.preview.existing {
            return Err(already(&r.preview.name, id));
        }
        self.add_resolved(r, options, false).await
    }

    /// `explicit`: the dialog chose "Start torrent" itself, so the
    /// "Do not start the download automatically" setting doesn't apply again.
    pub(super) async fn add_resolved(
        self: &Arc<Self>,
        r: Resolved,
        options: AddOptions,
        explicit: bool,
    ) -> Result<Response> {
        let session = self.session_ready().await;
        let settings = self.settings();
        let cat_path = options.category.as_ref().and_then(|c| {
            lock(&self.inner)
                .lists
                .categories
                .iter()
                .find(|x| &x.name == c && !x.save_path.is_empty())
                .map(|x| x.save_path.clone())
        });
        let save = options
            .save_path
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or(cat_path)
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_save_path(&settings));
        let layout = options.content_layout.unwrap_or(settings.content_layout);
        let folder = content_folder(&save, layout, &r.preview.name, r.preview.files.len());
        let stop = options.stop_condition.unwrap_or(settings.stop_condition);
        let paused = options.paused
            || (!explicit && settings.start_paused)
            || stop == StopCondition::MetadataReceived;
        if options.skip_check {
            // "Skip hash check": librqbit's fast-resume with every piece present.
            let _ = std::fs::create_dir_all(&self.paths.session);
            let f = self
                .paths
                .session
                .join(format!("{}.bitv", r.preview.info_hash_v1));
            if let Err(e) = std::fs::write(&f, all_pieces(r.preview.pieces)) {
                tracing::warn!("writing {}: {e}", f.display());
            }
        }
        let mut peers: Vec<SocketAddr> = options
            .peers
            .iter()
            .filter_map(|p| p.parse().ok())
            .collect();
        peers.extend(r.peers.iter().copied());
        let opts = AddTorrentOptions {
            paused,
            output_folder: Some(folder.to_string_lossy().into_owned()),
            only_files: options
                .only_files
                .as_ref()
                .map(|v| v.iter().map(|&i| i as usize).collect()),
            overwrite: true,
            peer_limit: Some(settings.max_connections_per_torrent as usize),
            initial_peers: Some(peers).filter(|v| !v.is_empty()),
            ..Default::default()
        };
        let resp = session
            .add_torrent(AddTorrent::from_bytes(r.bytes), Some(opts))
            .await?;
        let (id, handle) = match resp {
            AddTorrentResponse::Added(id, h) => (id as TorrentId, h),
            AddTorrentResponse::AlreadyManaged(id, h) => {
                return Err(already(&h.name().unwrap_or_default(), id as TorrentId));
            }
            AddTorrentResponse::ListOnly(_) => return Err(anyhow!("unexpected list-only reply")),
        };
        if let Some(c) = &options.category
            && !c.is_empty()
        {
            self.ensure_category(c, None)?;
        }
        let mut rec = new_record(
            id,
            &handle,
            options.added_on.unwrap_or_else(now_unix),
            &options,
        );
        rec.running = !paused;
        // The Save path column shows the save folder, as qBittorrent does,
        // not the torrent's own folder inside it.
        rec.save_path = display_path(&save);
        let name = rec.name.clone();
        let unfinished = rec.progress < 1.0;
        {
            let mut inner = lock(&self.inner);
            if unfinished {
                let max = inner
                    .recs
                    .values()
                    .filter_map(|r| r.queue_pos)
                    .max()
                    .unwrap_or(0);
                rec.queue_pos = Some(max + 1);
            }
            inner.recs.insert(id, rec.clone());
            inner.live.insert(
                id,
                Live {
                    started: !paused,
                    stop_when_checked: !paused && stop == StopCondition::FilesChecked,
                    ..Live::default()
                },
            );
        }
        lock(&self.store).save_torrents(&[rec])?;
        if unfinished && options.add_to_top.unwrap_or(settings.add_to_top) {
            self.queue_move(&[id], QueueMove::Top);
        }
        self.refresh_lists();
        self.broadcast(Event::Notice {
            text: format!("Added {name}"),
        });
        Ok(Response::Added { id, name })
    }

    /// Files and links from the system, a second launch, Ctrl+O or the link
    /// dialog. Shown in the Add torrent dialog when that setting is on.
    pub(super) async fn open_sources(
        self: &Arc<Self>,
        sources: Vec<AddSource>,
    ) -> Result<Response> {
        let s = self.settings();
        if !(s.show_add_dialog && self.window.available()) {
            let mut errors = Vec::new();
            let mut last = Response::Ok;
            for src in sources {
                match self.add(src, AddOptions::default()).await {
                    Ok(r) => last = r,
                    Err(e) => errors.push(format!("{e:#}")),
                }
            }
            for e in &errors {
                self.broadcast(Event::Notice { text: e.clone() });
            }
            return if errors.is_empty() {
                Ok(last)
            } else {
                Err(anyhow!(errors.join("; ")))
            };
        }
        let mut queued = 0;
        for src in sources {
            queued += usize::from(self.queue_pending(src));
        }
        if queued == 0 {
            return Ok(Response::Ok);
        }
        self.broadcast_pending();
        // "Bring torrent dialog to the front"; a closed window always opens.
        if s.add_dialog_front || !self.window.is_open() {
            self.window.show(self);
        }
        Ok(Response::Ok)
    }

    /// The dialog's starting values, from Settings › Downloads.
    fn add_defaults(&self) -> AddOptions {
        let s = self.settings();
        AddOptions {
            save_path: Some(display_path(&self.default_save_path(&s))),
            paused: s.start_paused,
            content_layout: Some(s.content_layout),
            stop_condition: Some(s.stop_condition),
            add_to_top: Some(s.add_to_top),
            ..AddOptions::default()
        }
    }

    /// Queue one source for the dialog; false when it was already in the list.
    fn queue_pending(self: &Arc<Self>, source: AddSource) -> bool {
        let (label, is_magnet) = describe(&source);
        if let AddSource::Magnet { uri } = &source
            && let Some((id, name)) = magnet_hash(uri).and_then(|h| self.existing(&h))
        {
            self.broadcast(Event::Notice {
                text: already(&name, id).to_string(),
            });
            return false;
        }
        let defaults = self.add_defaults();
        let token = {
            let mut inner = lock(&self.inner);
            inner.next_token += 1;
            let token = inner.next_token;
            inner.pending.push(Pending {
                add: PendingAdd {
                    token,
                    label,
                    is_magnet,
                    state: PendingState::Resolving,
                    defaults,
                },
                source: source.clone(),
                resolved: None,
                task: None,
            });
            token
        };
        let me = self.clone();
        let task = self.rt.spawn(async move {
            let r = me.resolve(&source).await;
            me.finish_pending(token, r);
        });
        if let Some(p) = lock(&self.inner)
            .pending
            .iter_mut()
            .find(|p| p.add.token == token)
        {
            p.task = Some(task.abort_handle());
        }
        true
    }

    fn finish_pending(&self, token: TokenId, r: Result<Resolved>) {
        let mut notice = None;
        {
            let mut inner = lock(&self.inner);
            let Some(ix) = inner.pending.iter().position(|p| p.add.token == token) else {
                return; // cancelled meanwhile
            };
            match r {
                Ok(r) if r.preview.existing.is_some() => {
                    // qBittorrent: "Torrent is already present"; no dialog.
                    let id = r.preview.existing.unwrap_or_default();
                    notice = Some(already(&r.preview.name, id).to_string());
                    inner.pending.remove(ix);
                }
                Ok(r) => {
                    let p = &mut inner.pending[ix];
                    if p.add.is_magnet {
                        p.add.label = r.preview.name.clone();
                    }
                    p.add.state = PendingState::Ready {
                        preview: Box::new(r.preview.clone()),
                    };
                    p.resolved = Some(r);
                    p.task = None;
                }
                Err(e) => {
                    let p = &mut inner.pending[ix];
                    p.add.state = PendingState::Failed {
                        error: format!("{e:#}"),
                    };
                    p.task = None;
                }
            }
        }
        if let Some(text) = notice {
            self.broadcast(Event::Notice { text });
        }
        self.broadcast_pending();
    }

    pub(super) fn pending_list(&self) -> Vec<PendingAdd> {
        lock(&self.inner)
            .pending
            .iter()
            .map(|p| p.add.clone())
            .collect()
    }

    fn broadcast_pending(&self) {
        let items = self.pending_list();
        self.broadcast(Event::PendingAdds { items });
    }

    /// The dialog's OK. While a magnet is still resolving it is added as a
    /// magnet (qBittorrent allows that too) and the options still apply.
    pub(super) async fn confirm_add(
        self: &Arc<Self>,
        token: TokenId,
        options: AddOptions,
        never_show_again: bool,
    ) -> Result<Response> {
        let p = {
            let mut inner = lock(&self.inner);
            let ix = inner
                .pending
                .iter()
                .position(|p| p.add.token == token)
                .ok_or_else(|| anyhow!("this torrent is no longer waiting to be added"))?;
            inner.pending.remove(ix)
        };
        if let Some(t) = &p.task {
            t.abort();
        }
        if never_show_again {
            lock(&self.inner).settings.show_add_dialog = false;
            self.save_settings()?;
        }
        self.broadcast_pending();
        let r = match p.resolved {
            Some(r) => self.add_resolved(r, options, true).await,
            None => match self.resolve(&p.source).await {
                Ok(r) if r.preview.existing.is_some() => Err(already(
                    &r.preview.name,
                    r.preview.existing.unwrap_or_default(),
                )),
                Ok(r) => self.add_resolved(r, options, true).await,
                Err(e) => Err(e),
            },
        };
        if let Err(e) = &r {
            self.broadcast(Event::Notice {
                text: format!("{e:#}"),
            });
        }
        r
    }

    pub(super) fn cancel_add(&self, token: TokenId) {
        let removed = {
            let mut inner = lock(&self.inner);
            inner
                .pending
                .iter()
                .position(|p| p.add.token == token)
                .map(|ix| inner.pending.remove(ix))
        };
        if let Some(p) = removed {
            if let Some(t) = p.task {
                t.abort();
            }
            self.broadcast_pending();
        }
    }

    pub(super) fn associations(&self) -> Associations {
        let Ok(exe) = std::env::current_exe() else {
            return Associations::default();
        };
        let s = tt_platform::assoc::status(&exe);
        Associations {
            torrent_files: s.torrent_files,
            magnet_links: s.magnet_links,
        }
    }

    pub(super) fn make_default(&self) -> Result<Response> {
        use tt_platform::assoc::MakeDefault;
        let exe = std::env::current_exe()?;
        let note = match tt_platform::assoc::make_default(&exe)? {
            MakeDefault::Done => None,
            MakeDefault::SettingsOpened => Some(
                "Windows opened Default apps. Choose tTorrent for .torrent and MAGNET there."
                    .to_owned(),
            ),
            MakeDefault::SystemAsked => Some("macOS may ask you to confirm the change.".to_owned()),
            MakeDefault::Unsupported(why) => Some(why),
        };
        Ok(Response::MadeDefault {
            status: self.associations(),
            note,
        })
    }
}

type TokenId = u64;

fn already(name: &str, id: TorrentId) -> anyhow::Error {
    anyhow!("{name} is already in the list (#{id}); its trackers were not merged")
}

/// The v1 info hash of a magnet link, lower-case hex.
pub(super) fn magnet_hash(uri: &str) -> Option<String> {
    librqbit::Magnet::parse(uri)
        .ok()?
        .as_id20()
        .map(|h| h.as_string())
}

/// What the dialog calls a source before its metadata is known.
fn describe(source: &AddSource) -> (String, bool) {
    match source {
        AddSource::Magnet { uri } => {
            let m = librqbit::Magnet::parse(uri).ok();
            let name = m
                .as_ref()
                .and_then(|m| m.name.clone())
                .filter(|n| !n.is_empty())
                .or_else(|| magnet_hash(uri))
                .unwrap_or_else(|| "Magnet link".into());
            (name, true)
        }
        AddSource::Url { url } => (
            url.rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(url)
                .to_owned(),
            false,
        ),
        AddSource::File { path } => (
            Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone()),
            false,
        ),
        AddSource::Bytes { .. } => ("Torrent".into(), false),
    }
}

/// Everything the dialog shows, read from the .torrent bytes.
pub(super) fn preview_of(bytes: &[u8]) -> Result<TorrentPreview> {
    let t = librqbit::torrent_from_bytes(bytes).map_err(|e| anyhow!("not a torrent file: {e}"))?;
    let text = |b: &Option<librqbit::ByteBuf<'_>>| {
        b.as_ref()
            .map(|b| String::from_utf8_lossy(b.as_ref()).trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let comment = text(&t.comment);
    let created_by = text(&t.created_by);
    let trackers: Vec<String> = t
        .iter_announce()
        .map(|b| String::from_utf8_lossy(b.as_ref()).into_owned())
        .collect();
    let info_hash_v1 = t.info_hash.as_string();
    let created_on = t.creation_date.map(|d| d as i64);
    let v = t
        .info
        .data
        .validate()
        .map_err(|e| anyhow!("the torrent's metadata is invalid: {e}"))?;
    let files: Vec<PreviewFile> = v
        .iter_file_details()
        .map(|fd| PreviewFile {
            path: fd.filename.to_vec().join("/"),
            size: fd.len,
        })
        .collect();
    Ok(TorrentPreview {
        name: v
            .name()
            .map(|n| n.into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| info_hash_v1.clone()),
        total_size: files.iter().map(|f| f.size).sum(),
        files,
        info_hash_v2: None,
        comment,
        created_on,
        created_by,
        private: v.info().private,
        piece_size: u64::from(v.lengths().default_piece_length()),
        pieces: v.lengths().total_pieces(),
        trackers,
        existing: None,
        info_hash_v1,
    })
}

/// qBittorrent's "Torrent content layout": where the files go inside `save`.
pub fn content_folder(save: &Path, layout: ContentLayout, name: &str, files: usize) -> PathBuf {
    match layout {
        ContentLayout::Original if files > 1 => save.join(safe_name(name)),
        ContentLayout::Original | ContentLayout::NoSubfolder => save.to_path_buf(),
        ContentLayout::Subfolder => {
            // A single file gets a folder named after it, without the extension.
            let base = if files > 1 {
                name
            } else {
                Path::new(name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(name)
            };
            save.join(safe_name(base))
        }
    }
}

/// A torrent name as one folder name on every system: no separators, none of
/// Windows' reserved characters, no trailing dots or spaces.
pub fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let s = s.trim().trim_end_matches(['.', ' ']).to_owned();
    if s.is_empty() || s == ".." {
        "_".into()
    } else {
        s
    }
}

/// A fast-resume bitfield with every piece present (MSB first, spare bits clear).
pub fn all_pieces(pieces: u32) -> Vec<u8> {
    let n = pieces as usize;
    let mut v = vec![0xFFu8; n.div_ceil(8)];
    if !n.is_multiple_of(8)
        && let Some(last) = v.last_mut()
    {
        *last = 0xFFu8 << (8 - n % 8);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_qbittorrent() {
        let save = Path::new("D:/Downloads");
        let o = ContentLayout::Original;
        assert_eq!(content_folder(save, o, "Sintel", 3), save.join("Sintel"));
        assert_eq!(content_folder(save, o, "sintel.mkv", 1), save);
        let s = ContentLayout::Subfolder;
        assert_eq!(
            content_folder(save, s, "sintel.mkv", 1),
            save.join("sintel")
        );
        assert_eq!(content_folder(save, s, "Sintel", 2), save.join("Sintel"));
        let n = ContentLayout::NoSubfolder;
        assert_eq!(content_folder(save, n, "Sintel", 3), save);
    }

    #[test]
    fn names_are_safe_folders() {
        assert_eq!(safe_name("a/b\\c:d"), "a_b_c_d");
        assert_eq!(safe_name("Ubuntu 24.04 LTS. "), "Ubuntu 24.04 LTS");
        assert_eq!(safe_name(".."), "_");
        assert_eq!(safe_name(""), "_");
    }

    #[test]
    fn all_pieces_bitfield() {
        assert_eq!(all_pieces(8), vec![0xFF]);
        assert_eq!(all_pieces(10), vec![0xFF, 0b1100_0000]);
        assert_eq!(all_pieces(0), Vec::<u8>::new());
    }

    #[test]
    fn magnet_hash_and_label() {
        let m = "magnet:?xt=urn:btih:dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c&dn=Big+Buck+Bunny";
        assert_eq!(
            magnet_hash(m).as_deref(),
            Some("dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c")
        );
        let (label, is_magnet) = describe(&AddSource::Magnet { uri: m.into() });
        assert!(is_magnet);
        assert_eq!(label, "Big Buck Bunny");
        assert_eq!(magnet_hash("not a magnet"), None);
    }

    #[test]
    fn preview_reads_a_multi_file_torrent() {
        fn b(s: &str) -> String {
            format!("{}:{}", s.len(), s)
        }
        let info = format!(
            "d5:filesld6:lengthi100e4:pathl{}eed6:lengthi50e4:pathl{}{}eee4:name{}12:piece lengthi16384e6:pieces20:{}e",
            b("Sintel.mkv"),
            b("Subs"),
            b("en.srt"),
            b("Sintel"),
            "x".repeat(20)
        );
        let t = format!(
            "d8:announce{}7:comment{}13:creation datei1700000000e4:info{info}e",
            b("udp://tracker.example.org:1337"),
            b("Blender open movie")
        );
        let p = preview_of(t.as_bytes()).unwrap();
        assert_eq!(p.name, "Sintel");
        assert_eq!(p.files.len(), 2);
        assert_eq!(p.files[1].path, "Subs/en.srt");
        assert_eq!(p.total_size, 150);
        assert_eq!(p.comment.as_deref(), Some("Blender open movie"));
        assert_eq!(p.created_on, Some(1_700_000_000));
        assert_eq!(
            p.trackers,
            vec!["udp://tracker.example.org:1337".to_string()]
        );
        assert_eq!(p.pieces, 1);
        assert_eq!(p.info_hash_v1.len(), 40);
    }
}
