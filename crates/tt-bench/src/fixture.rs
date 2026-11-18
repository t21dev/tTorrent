//! Synthetic torrents (TESTING.md › Fixtures): sparse zero-filled files of
//! realistic sizes, so a library costs almost nothing on disk, with real
//! .torrent files (every full piece of zeros has the same SHA-1).
//! Names are open content: Linux ISOs, Blender open movies, Internet Archive.

use anyhow::{Context, Result};
use std::path::Path;

pub struct Spec {
    pub name: &'static str,
    pub size: u64,
    pub category: &'static str,
    pub tags: &'static [&'static str],
    /// What the demo engine should show: "seed", "download", "stopped", "checking", "done".
    pub role: &'static str,
    /// Days ago it was added (for the Added on column).
    pub added_days: f32,
}

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

/// The demo library behind the README screenshots.
pub fn demo_specs() -> Vec<Spec> {
    let s = |name, size, category, tags, role, added_days| Spec {
        name,
        size,
        category,
        tags,
        role,
        added_days,
    };
    vec![
        s(
            "kali-linux-2026.3-installer-amd64.iso",
            4 * GB + 210 * MB,
            "Linux ISOs",
            &[],
            "download",
            0.02,
        ),
        s(
            "Sintel (2010) 1080p.mkv",
            GB + 230 * MB,
            "Films",
            &["Open movies"],
            "download",
            0.05,
        ),
        s(
            "ubuntu-24.04.3-desktop-amd64.iso",
            5 * GB + 920 * MB,
            "Linux ISOs",
            &["Keep"],
            "download",
            0.1,
        ),
        s(
            "Fedora-Workstation-Live-x86_64-43.iso",
            2 * GB + 310 * MB,
            "Linux ISOs",
            &[],
            "download",
            0.3,
        ),
        s(
            "debian-13.2.0-amd64-DVD-1.iso",
            3 * GB + 720 * MB,
            "Linux ISOs",
            &["Keep"],
            "seed",
            1.4,
        ),
        s(
            "Big Buck Bunny (2008) 4K.mp4",
            870 * MB,
            "Films",
            &["Open movies"],
            "seed",
            2.2,
        ),
        s(
            "archlinux-2026.11.01-x86_64.iso",
            GB + 400 * MB,
            "Linux ISOs",
            &[],
            "seed",
            5.5,
        ),
        s(
            "Tears of Steel (2012) 4K.mov",
            6 * GB + 330 * MB,
            "Films",
            &["Open movies"],
            "stopped",
            6.1,
        ),
        s(
            "LibreOffice_25.8_Win_x86-64.msi",
            352 * MB,
            "Software",
            &[],
            "done",
            7.3,
        ),
        s(
            "blender-4.5-linux-x64.tar.xz",
            380 * MB,
            "Software",
            &[],
            "seed",
            8.0,
        ),
        s(
            "Cosmos Laundromat (2015) 2K.mkv",
            GB + 120 * MB,
            "Films",
            &["Open movies"],
            "seed",
            9.4,
        ),
        s(
            "linuxmint-22.2-cinnamon-64bit.iso",
            2 * GB + 980 * MB,
            "Linux ISOs",
            &[],
            "seed",
            12.0,
        ),
        s(
            "Night of the Living Dead (1968).mp4",
            1180 * MB,
            "Films",
            &["Public domain"],
            "done",
            14.5,
        ),
        s(
            "openSUSE-Tumbleweed-DVD-x86_64-Current.iso",
            4 * GB + 600 * MB,
            "Linux ISOs",
            &[],
            "stopped",
            18.0,
        ),
    ]
}

/// Piece length for a size, like common clients choose.
pub fn piece_length(size: u64) -> u64 {
    match size {
        s if s <= 512 * MB => 512 * 1024,
        s if s <= 2 * GB => MB,
        s if s <= 8 * GB => 2 * MB,
        _ => 4 * MB,
    }
}

fn bstr(out: &mut Vec<u8>, s: &[u8]) {
    out.extend_from_slice(s.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(s);
}

/// A single-file .torrent for `size` zero bytes. Returns (torrent bytes, info hash hex).
pub fn zero_torrent(name: &str, size: u64) -> (Vec<u8>, String) {
    let plen = piece_length(size);
    let full = size / plen;
    let tail = size % plen;
    let zero_full = sha1_smol::Sha1::from(vec![0u8; plen as usize])
        .digest()
        .bytes();
    let mut pieces = Vec::with_capacity(((full + 1) * 20) as usize);
    for _ in 0..full {
        pieces.extend_from_slice(&zero_full);
    }
    if tail > 0 {
        pieces.extend_from_slice(
            &sha1_smol::Sha1::from(vec![0u8; tail as usize])
                .digest()
                .bytes(),
        );
    }
    // Keys in sorted order: length, name, piece length, pieces.
    let mut info = b"d6:lengthi".to_vec();
    info.extend_from_slice(size.to_string().as_bytes());
    info.extend_from_slice(b"e4:name");
    bstr(&mut info, name.as_bytes());
    info.extend_from_slice(b"12:piece lengthi");
    info.extend_from_slice(plen.to_string().as_bytes());
    info.extend_from_slice(b"e6:pieces");
    bstr(&mut info, &pieces);
    info.push(b'e');
    let hash = sha1_smol::Sha1::from(&info).digest().to_string();
    let mut t = b"d10:created by11:tTorrent fx4:info".to_vec();
    t.extend_from_slice(&info);
    t.push(b'e');
    (t, hash)
}

/// A multi-file torrent for the Add torrent dialog's screenshot: Blender's
/// open movie Sintel with subtitles and extras. Never downloaded (the hashes
/// are placeholders), only previewed.
pub fn sintel_torrent() -> Vec<u8> {
    let files: [(&[&str], u64); 7] = [
        (&["Sintel.2010.1080p.mkv"], 1_224_736_768),
        (&["Sintel.2010.720p.mkv"], 652_214_272),
        (&["Subtitles", "sintel_en.srt"], 1_652),
        (&["Subtitles", "sintel_de.srt"], 1_778),
        (&["Subtitles", "sintel_es.srt"], 1_705),
        (&["Extras", "poster.jpg"], 412_331),
        (&["README.txt"], 2_048),
    ];
    let total: u64 = files.iter().map(|f| f.1).sum();
    let plen = piece_length(total);
    let pieces = total.div_ceil(plen) as usize;
    let mut info = b"d5:filesl".to_vec();
    for (path, len) in files {
        info.extend_from_slice(b"d6:lengthi");
        info.extend_from_slice(len.to_string().as_bytes());
        info.extend_from_slice(b"e4:pathl");
        for p in path {
            bstr(&mut info, p.as_bytes());
        }
        info.extend_from_slice(b"ee");
    }
    info.extend_from_slice(b"e4:name");
    bstr(&mut info, b"Sintel");
    info.extend_from_slice(b"12:piece lengthi");
    info.extend_from_slice(plen.to_string().as_bytes());
    info.extend_from_slice(b"e6:pieces");
    bstr(&mut info, &vec![0u8; pieces * 20]);
    info.push(b'e');
    let mut t = b"d8:announce".to_vec();
    bstr(&mut t, b"udp://tracker.example.org:1337/announce");
    t.extend_from_slice(b"7:comment");
    bstr(
        &mut t,
        b"Sintel, the Durian Open Movie Project. Blender Foundation, CC BY 3.0.",
    );
    t.extend_from_slice(b"10:created by11:tTorrent fx13:creation datei1285891200e4:info");
    t.extend_from_slice(&info);
    t.push(b'e');
    t
}

/// A file of `size` zero bytes with real disk blocks (not sparse), for runs
/// where caching must behave as with real files.
pub fn dense_file(path: &Path, size: u64) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::io::BufWriter::with_capacity(
        4 << 20,
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?,
    );
    let block = vec![0u8; 4 << 20];
    let mut left = size;
    while left > 0 {
        let n = left.min(block.len() as u64) as usize;
        f.write_all(&block[..n])?;
        left -= n as u64;
    }
    f.flush()?;
    Ok(())
}

/// A sparse file of `size` zero bytes (no disk space used on NTFS, ext4, APFS).
pub fn sparse_file(path: &Path, size: u64) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let f = std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    drop(f);
    #[cfg(windows)]
    {
        let ok = std::process::Command::new("fsutil")
            .args(["sparse", "setflag"])
            .arg(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            anyhow::bail!("could not mark {} sparse", path.display());
        }
    }
    let f = std::fs::OpenOptions::new().write(true).open(path)?;
    f.set_len(size)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torrent_is_valid_bencode_with_right_piece_count() {
        let (t, hash) = zero_torrent("x.iso", 5 * MB + 3);
        assert_eq!(hash.len(), 40);
        let s = String::from_utf8_lossy(&t);
        assert!(s.starts_with("d10:created by"));
        // 5 MiB + 3 bytes at 512 KiB pieces: 11 pieces, 220 bytes of hashes.
        assert!(s.contains("6:pieces220:"));
    }
}
