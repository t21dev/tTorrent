//! `tt-bench soak LIB_DIR [--hours H] [--rate-mb R] [--qb EXE]`: tTorrent and
//! qBittorrent side by side for hours, each seeding the same library to its
//! own local downloader at the same capped rate, with memory, CPU and I/O of
//! both logged every 5 minutes to `LIB_DIR/soak.csv`.
//!
//! Memory that grows with uptime needs traffic: torrents nobody downloads
//! don't touch their files. The downloaders discard what they receive (the
//! fixture's data is all zeros, so reading zeros back passes every hash
//! check), so the run doesn't write hundreds of gigabytes to the disk.
//!
//! Before: `tt-bench library 735 LIB_DIR --max-mb 64 --dense` (real disk
//! blocks, not sparse holes, so caching behaves as with real files), then
//! `tt-bench qb LIB_DIR` once to give qBittorrent the same torrents.
//! tTorrent runs from a copy of its binaries in `LIB_DIR/soak-bin`, so
//! rebuilding during the run can't touch it.

use anyhow::{Context, Result, anyhow, bail};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use librqbit::storage::{StorageFactory, StorageFactoryExt, TorrentStorage};
use tt_ipc::{ClientKind, Request, Response};

/// A downloader's storage: writes go nowhere, reads are zeros.
#[derive(Clone, Default)]
struct NullFactory;

struct Null;

impl StorageFactory for NullFactory {
    type Storage = Null;

    fn create(
        &self,
        _: &librqbit::ManagedTorrentShared,
        _: &librqbit::TorrentMetadata,
    ) -> Result<Null> {
        Ok(Null)
    }

    fn clone_box(&self) -> librqbit::storage::BoxStorageFactory {
        self.clone().boxed()
    }
}

impl TorrentStorage for Null {
    fn init(
        &mut self,
        _: &librqbit::ManagedTorrentShared,
        _: &librqbit::TorrentMetadata,
    ) -> Result<()> {
        Ok(())
    }
    fn pread_exact(&self, _: usize, _: u64, buf: &mut [u8]) -> Result<()> {
        buf.fill(0);
        Ok(())
    }
    fn pwrite_all(&self, _: usize, _: u64, _: &[u8]) -> Result<()> {
        Ok(())
    }
    fn remove_file(&self, _: usize, _: &Path) -> Result<()> {
        Ok(())
    }
    fn remove_directory_if_empty(&self, _: &Path) -> Result<()> {
        Ok(())
    }
    fn ensure_file_length(&self, _: usize, _: u64) -> Result<()> {
        Ok(())
    }
    fn take(&self) -> Result<Box<dyn TorrentStorage>> {
        Ok(Box::new(Null))
    }
}

/// Downloads random torrents from one client, forever, at `rate` bytes/s.
fn leech(
    rt: &tokio::runtime::Runtime,
    name: &'static str,
    port: u16,
    torrents: Arc<Vec<Vec<u8>>>,
    rate: u32,
    done: Arc<AtomicU64>,
    stop: Arc<std::sync::atomic::AtomicBool>,
) {
    rt.spawn(async move {
        let opts = librqbit::SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: None,
            ratelimits: librqbit::limits::LimitsConfig {
                download_bps: std::num::NonZeroU32::new(rate),
                upload_bps: None,
            },
            default_storage_factory: Some(NullFactory.boxed()),
            ..Default::default()
        };
        let session = match librqbit::Session::new_with_opts(
            std::env::temp_dir().join("tt-soak-unused"),
            opts,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{name} downloader: {e:#}");
                return;
            }
        };
        let peer: std::net::SocketAddr = (std::net::Ipv4Addr::LOCALHOST, port).into();
        let mut i: usize = name.len() * 7919;
        while !stop.load(Ordering::Relaxed) {
            i = (i * 1_103_515_245 + 12_345) % torrents.len().max(1);
            let added = session
                .add_torrent(
                    librqbit::AddTorrent::from_bytes(torrents[i].clone()),
                    Some(librqbit::AddTorrentOptions {
                        overwrite: true,
                        initial_peers: Some(vec![peer]),
                        output_folder: Some(
                            std::env::temp_dir()
                                .join("tt-soak-unused")
                                .to_string_lossy()
                                .into(),
                        ),
                        ..Default::default()
                    }),
                )
                .await;
            let Ok(h) = added.and_then(|r| r.into_handle().ok_or_else(|| anyhow!("no handle")))
            else {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            };
            let t = Instant::now();
            while !stop.load(Ordering::Relaxed) && t.elapsed() < Duration::from_secs(900) {
                let st = h.stats();
                if st.finished {
                    done.fetch_add(st.total_bytes, Ordering::Relaxed);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            let _ = session.delete(h.id().into(), false).await;
        }
    });
}

fn mb(b: u64) -> f64 {
    b as f64 / 1_048_576.0
}

fn copy_bins(to: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(to)?;
    let exe = std::env::current_exe()?;
    let dir = exe.parent().context("bin dir")?;
    for name in ["ttorrent.exe", "ttorrent-ui.exe"] {
        std::fs::copy(dir.join(name), to.join(name)).with_context(|| format!("copying {name}"))?;
    }
    Ok(to.join("ttorrent.exe"))
}

fn plain(p: &Path) -> String {
    p.to_string_lossy().trim_start_matches(r"\\?\").to_owned()
}

pub fn soak(args: &[String]) -> Result<()> {
    let usage = "soak LIB_DIR [--hours H] [--rate-mb R] [--qb EXE]";
    let dir = std::fs::canonicalize(args.first().context(usage)?)?;
    let flag = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let hours: f64 = flag("--hours").map_or(Ok(24.0), |v| v.parse())?;
    let rate = flag("--rate-mb").map_or(Ok(4.0), |v| v.parse::<f64>())?;
    let rate = (rate * 1_048_576.0) as u32;
    let qb_exe = PathBuf::from(
        flag("--qb").unwrap_or_else(|| r"C:\Program Files\qBittorrent\qbittorrent.exe".into()),
    );
    let profile = dir.join("qb-profile");
    let ini = profile
        .join("qBittorrent")
        .join("config")
        .join("qBittorrent.ini");
    if !ini.exists() {
        bail!(
            "no qBittorrent profile; run `tt-bench qb {}` first",
            dir.display()
        );
    }
    // qBittorrent starts hidden in the tray, like tTorrent's --minimized.
    let s = std::fs::read_to_string(&ini)?;
    std::fs::write(
        &ini,
        s.replace("StartUpWindowState=Normal", "StartUpWindowState=Hidden"),
    )?;

    let torrents: Vec<Vec<u8>> = {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir.join("engine").join("session"))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "torrent"))
            .collect();
        v.sort();
        v.iter()
            .map(std::fs::read)
            .collect::<std::io::Result<_>>()?
    };
    let torrents = Arc::new(torrents);

    // tTorrent, tray only, from copied binaries.
    let tt_exe = copy_bins(&dir.join("soak-bin"))?;
    let data = dir.join("engine");
    let mut tt: Child = Command::new(&tt_exe)
        .arg("--data-dir")
        .arg(&data)
        .args(["--minimized", "--offline"])
        .spawn()?;
    let mut qb: Child = Command::new(&qb_exe)
        .arg(format!("--profile={}", plain(&profile)))
        .arg("--confirm-legal-notice")
        .spawn()?;
    let instance = tt_ipc::transport::instance_id(&data);
    let client = {
        let mut got = None;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(200));
            if let Ok((c, _)) = tt_ipc::transport::Client::connect(&instance, ClientKind::Bench) {
                got = Some(c);
                break;
            }
        }
        got.context("tTorrent didn't answer")?
    };
    let tt_port = {
        let mut port = None;
        for _ in 0..120 {
            if let Ok(Response::Snapshot(s)) = client.call(Request::Snapshot)
                && let Some(p) = s.status.port
                && s.status.starting.is_none()
                && !s.from_snapshot
            {
                port = Some(p);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        port.context("tTorrent didn't start its torrents")?
    };
    let qb_port = 18881;
    println!(
        "soak: {:.1} h, {:.1} MB/s per client, tTorrent pid {} port {tt_port}, qBittorrent pid {} port {qb_port}",
        hours,
        rate as f64 / 1_048_576.0,
        tt.id(),
        qb.id()
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let tt_done = Arc::new(AtomicU64::new(0));
    let qb_done = Arc::new(AtomicU64::new(0));
    leech(
        &rt,
        "ttorrent",
        tt_port,
        torrents.clone(),
        rate,
        tt_done.clone(),
        stop.clone(),
    );
    leech(
        &rt,
        "qbittorrent",
        qb_port,
        torrents.clone(),
        rate,
        qb_done.clone(),
        stop.clone(),
    );

    let csv_path = dir.join("soak.csv");
    let mut csv = std::fs::File::create(&csv_path)?;
    writeln!(
        csv,
        "minute,client,task_manager_mb,working_set_mb,private_bytes_mb,cpu_s,read_mb,write_mb,served_mb"
    )?;
    let t0 = Instant::now();
    let end = Duration::from_secs_f64(hours * 3600.0);
    let mut rows: Vec<(u64, &str, f64)> = Vec::new();
    loop {
        let minute = t0.elapsed().as_secs() / 60;
        for (name, pid, served) in [
            ("ttorrent", tt.id(), &tt_done),
            ("qbittorrent", qb.id(), &qb_done),
        ] {
            let m = tt_platform::process_memory(Some(pid)).unwrap_or_default();
            let io = tt_platform::process_io(pid).unwrap_or_default();
            writeln!(
                csv,
                "{minute},{name},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.0}",
                mb(m.private_working_set),
                mb(m.working_set),
                mb(m.private_bytes),
                io.cpu_ms as f64 / 1000.0,
                mb(io.read_bytes),
                mb(io.write_bytes),
                mb(served.load(Ordering::Relaxed)),
            )?;
            rows.push((minute, name, mb(m.private_working_set)));
        }
        csv.flush()?;
        if t0.elapsed() >= end
            || matches!(tt.try_wait(), Ok(Some(_)))
            || matches!(qb.try_wait(), Ok(Some(_)))
        {
            break;
        }
        std::thread::sleep(
            Duration::from_secs(300).min(end.saturating_sub(t0.elapsed()) + Duration::from_secs(1)),
        );
    }
    stop.store(true, Ordering::Relaxed);
    let _ = client.call(Request::Quit);
    let _ = Command::new("curl")
        .args([
            "-s",
            "-m",
            "5",
            "-X",
            "POST",
            "http://127.0.0.1:18080/api/v2/app/shutdown",
        ])
        .output();
    std::thread::sleep(Duration::from_secs(10));
    let _ = tt.kill();
    let _ = qb.kill();
    rt.shutdown_timeout(Duration::from_secs(5));

    println!("\n{}", csv_path.display());
    println!("| minute | tTorrent (Task Manager) | qBittorrent (Task Manager) |");
    println!("|---|---|---|");
    let mut minutes: Vec<u64> = rows.iter().map(|r| r.0).collect();
    minutes.dedup();
    for m in minutes {
        let get = |n: &str| {
            rows.iter()
                .find(|r| r.0 == m && r.1 == n)
                .map_or("–".to_string(), |r| format!("{:.1} MB", r.2))
        };
        println!("| {m} | {} | {} |", get("ttorrent"), get("qbittorrent"));
    }
    println!(
        "served: tTorrent {:.0} MB, qBittorrent {:.0} MB",
        mb(tt_done.load(Ordering::Relaxed)),
        mb(qb_done.load(Ordering::Relaxed))
    );
    Ok(())
}
