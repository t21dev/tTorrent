//! qBittorrent importer (PLAN.md › Engine). Reads `BT_backup` (`<hash>.torrent`
//! plus `<hash>.fastresume`) and `categories.json`, keeping progress, so the
//! engine never rechecks imported torrents (docs/startup.md §1).

pub mod bencode;

use anyhow::{Context, Result};
use bencode::Value;
use std::path::{Path, PathBuf};

/// One torrent from qBittorrent, with what tTorrent keeps of it.
#[derive(Clone, Debug, PartialEq)]
pub struct QbTorrent {
    /// Lower-case hex v1 info hash (the file name in BT_backup).
    pub info_hash: String,
    pub torrent_bytes: Vec<u8>,
    pub name: Option<String>,
    pub save_path: String,
    /// The folder inside `save_path` that holds the files, as qBittorrent laid
    /// them out: the torrent's name for a multi-file torrent ("Original"),
    /// the renamed root if the user renamed it, None when the files sit
    /// directly in `save_path` ("Don't create subfolder", single files).
    pub content_root: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub added_on: i64,
    pub completed_on: Option<i64>,
    pub uploaded: u64,
    pub downloaded: u64,
    pub active_secs: u64,
    pub paused: bool,
    pub ratio_limit: Option<f32>,
    /// One bit per piece, most significant bit first (librqbit's layout).
    pub pieces: Option<Vec<u8>>,
    pub piece_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QbCategory {
    pub name: String,
    pub save_path: String,
}

/// Where qBittorrent keeps its data for this user.
pub fn default_qb_dirs() -> Option<(PathBuf, PathBuf)> {
    #[cfg(windows)]
    {
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
        let roaming = PathBuf::from(std::env::var_os("APPDATA")?);
        Some((
            local.join("qBittorrent/BT_backup"),
            roaming.join("qBittorrent"),
        ))
    }
    #[cfg(not(windows))]
    {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        Some((
            home.join(".local/share/qBittorrent/BT_backup"),
            home.join(".config/qBittorrent"),
        ))
    }
}

/// Read every torrent in a BT_backup folder. Unreadable entries are skipped
/// and returned as errors beside the good ones, so one bad file never stops an import.
pub fn read_bt_backup(dir: &Path) -> Result<(Vec<QbTorrent>, Vec<String>)> {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "fastresume"))
        .collect();
    entries.sort();
    for fr in entries {
        match read_one(&fr) {
            Ok(t) => out.push(t),
            Err(e) => errors.push(format!("{}: {e:#}", fr.display())),
        }
    }
    Ok((out, errors))
}

fn read_one(fastresume: &Path) -> Result<QbTorrent> {
    let hash = fastresume
        .file_stem()
        .and_then(|s| s.to_str())
        .context("file name")?
        .to_ascii_lowercase();
    let resume_bytes = std::fs::read(fastresume)?;
    let resume = bencode::parse(&resume_bytes).context("fastresume is not bencode")?;
    let torrent_path = fastresume.with_extension("torrent");
    let torrent_bytes = match std::fs::read(&torrent_path) {
        Ok(b) => b,
        // qBittorrent 4.4+ may keep the info dict inside the fastresume instead.
        Err(_) => {
            let info = resume
                .get("info")
                .and_then(Value::raw)
                .context("no .torrent and no info in the fastresume")?;
            let mut t = b"d4:info".to_vec();
            t.extend_from_slice(info);
            t.push(b'e');
            t
        }
    };
    let torrent = bencode::parse(&torrent_bytes).context(".torrent is not bencode")?;
    let piece_count = torrent
        .get("info")
        .and_then(|i| i.get("pieces"))
        .and_then(Value::bytes)
        .map(|p| p.len() / 20)
        .unwrap_or(0);
    let content_root = content_root(&torrent, &resume);
    let int = |k: &str| resume.get(k).and_then(Value::int);
    let text = |k: &str| resume.get(k).and_then(Value::str).map(str::to_owned);
    let pieces = resume.get("pieces").and_then(Value::bytes).map(pack_pieces);
    let tags = resume
        .get("qBt-tags")
        .and_then(Value::list)
        .map(|l| l.iter().filter_map(Value::str).map(str::to_owned).collect())
        .unwrap_or_default();
    let paused = int("paused").unwrap_or(0) != 0
        || text("qBt-stopped").is_some_and(|s| s == "1")
        || int("qBt-stopped").unwrap_or(0) != 0;
    let completed = int("completed_time").filter(|&t| t > 0);
    let ratio_limit = int("qBt-ratioLimit")
        .filter(|&v| v >= 0)
        .map(|v| v as f32 / 1000.0);
    Ok(QbTorrent {
        info_hash: hash,
        name: text("qBt-name").filter(|n| !n.is_empty()),
        save_path: text("qBt-savePath")
            .filter(|s| !s.is_empty())
            .or_else(|| text("save_path"))
            .unwrap_or_default(),
        content_root,
        category: text("qBt-category").unwrap_or_default(),
        tags,
        added_on: int("added_time").unwrap_or(0),
        completed_on: completed,
        uploaded: int("total_uploaded").unwrap_or(0).max(0) as u64,
        downloaded: int("total_downloaded").unwrap_or(0).max(0) as u64,
        active_secs: int("active_time").unwrap_or(0).max(0) as u64,
        paused,
        ratio_limit,
        pieces,
        piece_count,
        torrent_bytes,
    })
}

/// Where the files are inside the save path. libtorrent keeps renamed paths
/// (qBittorrent's content layout and renames) in the resume's `mapped_files`.
fn content_root(torrent: &Value, resume: &Value) -> Option<String> {
    let info = torrent.get("info")?;
    let name = info.get("name").and_then(Value::str).map(str::to_owned);
    let multi = info.get("files").and_then(Value::list).is_some();
    let mapped: Vec<String> = resume
        .get("mapped_files")
        .and_then(Value::list)
        .map(|l| l.iter().filter_map(Value::str).map(str::to_owned).collect())
        .unwrap_or_default();
    let mapped: Vec<&String> = mapped.iter().filter(|m| !m.is_empty()).collect();
    if mapped.is_empty() {
        return if multi { name } else { None };
    }
    // Every mapped path under one first folder: that folder is the root.
    let first = |m: &str| {
        let m = m.replace('\\', "/");
        m.split_once('/').map(|(a, _)| a.to_owned())
    };
    let roots: Vec<Option<String>> = mapped.iter().map(|m| first(m)).collect();
    match roots.first() {
        Some(Some(r)) if roots.iter().all(|x| x.as_deref() == Some(r.as_str())) => Some(r.clone()),
        _ => None,
    }
}

/// libtorrent's resume "pieces" is one byte per piece (bit 0 = have); librqbit
/// wants one bit per piece, most significant first.
pub fn pack_pieces(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bytes.len().div_ceil(8)];
    for (i, b) in bytes.iter().enumerate() {
        if b & 1 == 1 {
            out[i / 8] |= 0x80 >> (i % 8);
        }
    }
    out
}

/// qBittorrent's `categories.json`: `{"Films": {"save_path": "D:/Films"}}`.
pub fn read_categories(config_dir: &Path) -> Vec<QbCategory> {
    let Ok(text) = std::fs::read_to_string(config_dir.join("categories.json")) else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&text)
    else {
        return Vec::new();
    };
    let mut v: Vec<QbCategory> = map
        .into_iter()
        .map(|(name, val)| QbCategory {
            save_path: val
                .get("save_path")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_owned(),
            name,
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bstr(s: &str) -> String {
        format!("{}:{}", s.len(), s)
    }

    /// A synthetic BT_backup entry for an open-content torrent.
    fn write_fixture(dir: &Path) -> String {
        let hash = "a".repeat(40);
        let info = format!(
            "d6:lengthi1048576e4:name{}12:piece lengthi262144e6:pieces80:{}e",
            bstr("big_buck_bunny_1080p.mp4"),
            "x".repeat(80)
        );
        let torrent = format!("d4:info{info}e");
        std::fs::write(dir.join(format!("{hash}.torrent")), torrent).unwrap();
        let resume = format!(
            "d10:added_timei1763400000e14:completed_timei1763403600e6:pausedi0e6:pieces4:{}12:qBt-category{}8:qBt-tagsl{}e12:qBt-savePath{}14:total_uploadedi5000e16:total_downloadedi1048576ee",
            "\u{1}\u{1}\u{0}\u{1}",
            bstr("Films"),
            bstr("Open movies"),
            bstr("D:/Films"),
        );
        std::fs::write(dir.join(format!("{hash}.fastresume")), resume).unwrap();
        hash
    }

    #[test]
    fn reads_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let hash = write_fixture(dir.path());
        std::fs::write(dir.path().join("broken.fastresume"), b"not bencode").unwrap();
        let (ts, errs) = read_bt_backup(dir.path()).unwrap();
        assert_eq!(errs.len(), 1);
        let t = &ts[0];
        assert_eq!(t.info_hash, hash);
        assert_eq!(t.category, "Films");
        assert_eq!(t.tags, vec!["Open movies".to_string()]);
        assert_eq!(t.save_path, "D:/Films");
        assert_eq!(t.uploaded, 5000);
        assert_eq!(t.completed_on, Some(1_763_403_600));
        assert_eq!(t.piece_count, 4);
        assert_eq!(t.pieces.as_deref(), Some(&[0b1101_0000u8][..]));
        assert!(!t.paused);
        // A single file sits directly in the save path.
        assert_eq!(t.content_root, None);
    }

    #[test]
    fn multi_file_content_root() {
        let multi = |name: &str| {
            format!(
                "d4:infod5:filesld6:lengthi1e4:pathl{}eed6:lengthi1e4:pathl{}eee4:name{}ee",
                bstr("a.txt"),
                bstr("b.txt"),
                bstr(name)
            )
        };
        let resume = |mapped: &[&str]| {
            let list: String = mapped.iter().map(|m| bstr(m)).collect();
            format!("d12:mapped_filesl{list}ee")
        };
        let tb = multi("Sintel");
        let t = bencode::parse(tb.as_bytes()).unwrap();
        let none = bencode::parse(b"de").unwrap();
        // Original layout: the torrent's name.
        assert_eq!(content_root(&t, &none).as_deref(), Some("Sintel"));
        // Renamed root folder.
        let r = resume(&["Sintel 2010/a.txt", r"Sintel 2010\b.txt"]);
        assert_eq!(
            content_root(&t, &bencode::parse(r.as_bytes()).unwrap()).as_deref(),
            Some("Sintel 2010")
        );
        // Don't create subfolder: no shared root.
        let r = resume(&["a.txt", "b.txt"]);
        assert_eq!(
            content_root(&t, &bencode::parse(r.as_bytes()).unwrap()),
            None
        );
    }

    #[test]
    fn categories_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("categories.json"),
            r#"{"Linux ISOs":{"save_path":"D:/ISOs"},"Films":{"save_path":""}}"#,
        )
        .unwrap();
        let c = read_categories(dir.path());
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].save_path, "D:/ISOs");
    }

    proptest::proptest! {
        #[test]
        fn packing_keeps_every_bit(bits in proptest::collection::vec(0u8..2, 0..200)) {
            let packed = pack_pieces(&bits);
            for (i, b) in bits.iter().enumerate() {
                let have = packed[i / 8] & (0x80 >> (i % 8)) != 0;
                proptest::prop_assert_eq!(have, *b == 1);
            }
        }
    }
}
