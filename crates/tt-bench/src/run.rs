//! The bench commands. Everything runs the real `ttorrent` and `ttorrent-ui`
//! binaries next to this one and reads OS counters from outside.

use crate::fixture::{self, Spec};
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tt_ipc::transport::Client;
use tt_ipc::{AddOptions, AddSource, ClientKind, Request, Response, UiPrefs};

fn bin(name: &str) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let p = exe.with_file_name(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    });
    if !p.exists() {
        bail!("{} not found; build the workspace first", p.display());
    }
    Ok(p)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn mb(b: u64) -> f64 {
    b as f64 / (1024.0 * 1024.0)
}

fn connect(data: &Path) -> Result<Client> {
    let inst = tt_ipc::transport::instance_id(data);
    for _ in 0..200 {
        if let Ok((c, _ev)) = Client::connect(&inst, ClientKind::Bench) {
            return Ok(c);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    bail!("the engine did not open its socket")
}

fn start_engine(data: &Path, window: bool) -> Result<Child> {
    let mut c = Command::new(bin("ttorrent")?);
    c.arg("--data-dir").arg(data).arg("--offline");
    if !window {
        c.arg("--headless");
    }
    Ok(c.spawn()?)
}

/// A seeder on loopback: a plain librqbit session serving `dir`.
struct Seeder {
    rt: tokio::runtime::Runtime,
    session: Arc<librqbit::Session>,
    port: u16,
}

impl Seeder {
    fn start(dir: &Path, torrents: &[(Vec<u8>, String)], upload_bps: u32) -> Result<Seeder> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let dir = dir.to_path_buf();
        let torrents: Vec<Vec<u8>> = torrents.iter().map(|(b, _)| b.clone()).collect();
        let (session, port) = rt.block_on(async move {
            let opts = librqbit::SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                listen: Some(librqbit::ListenerOptions {
                    mode: librqbit::ListenerMode::TcpOnly,
                    listen_addr: (std::net::Ipv4Addr::LOCALHOST, 0).into(),
                    ..Default::default()
                }),
                ratelimits: librqbit::limits::LimitsConfig {
                    upload_bps: std::num::NonZeroU32::new(upload_bps),
                    download_bps: None,
                },
                ..Default::default()
            };
            let s = librqbit::Session::new_with_opts(dir.clone(), opts).await?;
            for t in torrents {
                let h = s
                    .add_torrent(
                        librqbit::AddTorrent::from_bytes(t),
                        Some(librqbit::AddTorrentOptions {
                            overwrite: true,
                            output_folder: Some(dir.to_string_lossy().into_owned()),
                            ..Default::default()
                        }),
                    )
                    .await?
                    .into_handle()
                    .ok_or_else(|| anyhow!("seeder: no handle"))?;
                h.wait_until_initialized().await?;
            }
            let port = s
                .listen_addr()
                .map(|a| a.port())
                .ok_or_else(|| anyhow!("seeder has no port"))?;
            anyhow::Ok((s, port))
        })?;
        Ok(Seeder { rt, session, port })
    }

    fn stop(self) {
        let s = self.session.clone();
        self.rt.block_on(async move { s.stop().await });
    }
}

#[cfg(windows)]
fn ui_window(engine_pid: u32) -> Option<xcap::Window> {
    xcap::Window::all()
        .ok()?
        .into_iter()
        .filter(|w| w.title().is_ok_and(|t| t == "tTorrent"))
        .filter(|w| w.pid().is_ok_and(|p| p != engine_pid))
        .filter(|w| !w.is_minimized().unwrap_or(true))
        .filter(|w| w.width().unwrap_or(0) >= 400)
        .max_by_key(|w| u64::from(w.width().unwrap_or(0)) * u64::from(w.height().unwrap_or(0)))
}

/// Capture the window into `out` (its own pixels, never the screen).
#[cfg(windows)]
fn capture(engine_pid: u32, out: &Path) -> Result<u32> {
    let w = ui_window(engine_pid).ok_or_else(|| anyhow!("no tTorrent window"))?;
    let pid = w.pid()?;
    let img = w.capture_image()?;
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d)?;
    }
    img.save(out)?;
    Ok(pid)
}

#[cfg(not(windows))]
fn capture(_engine_pid: u32, _out: &Path) -> Result<u32> {
    bail!("window capture is Windows-only for now")
}

fn wait_exit(pid: u32, secs: u64) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(secs) {
        if tt_platform::process_memory(Some(pid)).is_none() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

struct Shot {
    file: &'static str,
    mode: &'static str,
    tint: &'static str,
    accent: &'static str,
    density: &'static str,
    show: &'static str,
    select: usize,
}

pub fn demo(args: &[String]) -> Result<()> {
    let mut shots_dir = PathBuf::from("docs/screenshots");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--shots" {
            shots_dir = it.next().map(PathBuf::from).context("--shots DIR")?;
        }
    }
    let root = PathBuf::from(".tt-dev/demo");
    let _ = std::fs::remove_dir_all(&root);
    let (seed_dir, have_dir, dl_dir, data) = (
        root.join("seed"),
        root.join("have"),
        root.join("downloads"),
        root.join("engine"),
    );
    let specs = fixture::demo_specs();
    println!(
        "demo: {} torrents, sparse files under {}",
        specs.len(),
        root.display()
    );
    let mut torrents = Vec::new();
    for s in &specs {
        fixture::sparse_file(&seed_dir.join(s.name), s.size)?;
        if matches!(s.role, "seed" | "done") {
            fixture::sparse_file(&have_dir.join(s.name), s.size)?;
        }
        torrents.push(fixture::zero_torrent(s.name, s.size));
    }
    let t0 = Instant::now();
    // About 9 MB/s shared by the downloads, so they show realistic progress.
    let seeder = Seeder::start(&seed_dir, &torrents, 9 * 1024 * 1024)?;
    println!(
        "seeder on 127.0.0.1:{} (checked in {:.1} s)",
        seeder.port,
        t0.elapsed().as_secs_f32()
    );

    let mut engine = start_engine(&data, false)?;
    let client = connect(&data)?;
    // Screenshots never show the owner's own folders (AGENTS.md).
    if let Ok(Response::Settings(mut s)) = client.call(Request::GetSettings) {
        std::fs::create_dir_all(&dl_dir)?;
        s.default_save_path = std::fs::canonicalize(&dl_dir)?
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_owned();
        let _ = client.call(Request::SetSettings { settings: s });
    }
    for c in ["Linux ISOs", "Films", "Software"] {
        let _ = client.call(Request::CreateCategory {
            name: c.into(),
            save_path: None,
        });
    }
    let peers = vec![format!("127.0.0.1:{}", seeder.port)];
    let stop_later: Vec<&Spec> = specs.iter().filter(|s| s.role == "stopped").collect();
    for (s, (bytes, _)) in specs.iter().zip(&torrents) {
        let save = if matches!(s.role, "seed" | "done") {
            &have_dir
        } else {
            &dl_dir
        };
        let r = client.call(Request::Add {
            source: AddSource::Bytes {
                data: bytes.clone(),
            },
            options: AddOptions {
                save_path: Some(
                    std::fs::canonicalize(save.parent().unwrap_or(save))?
                        .join(save.file_name().unwrap_or_default())
                        .to_string_lossy()
                        .into_owned(),
                ),
                category: Some(s.category.into()),
                tags: s.tags.iter().map(|t| (*t).to_owned()).collect(),
                paused: s.role == "done",
                peers: peers.clone(),
                added_on: Some(now() - (s.added_days * 86_400.0) as i64),
                ..Default::default()
            },
        });
        if let Err(e) = r {
            eprintln!("  add {}: {e}", s.name);
        }
        std::fs::create_dir_all(&dl_dir)?;
    }
    // Wait for the seeded copies to finish checking (at most 2 minutes), with
    // downloads running meanwhile; then stop the ones meant to look stopped.
    let t1 = Instant::now();
    let snap = loop {
        std::thread::sleep(Duration::from_secs(2));
        let snap = match client.call(Request::Snapshot) {
            Ok(Response::Snapshot(s)) => s,
            other => bail!("snapshot: {other:?}"),
        };
        let checking: Vec<String> = snap
            .rows
            .iter()
            .filter(|r| r.state == tt_ipc::TorrentState::Checking)
            .map(|r| format!("{} {:.0}%", r.name, r.progress * 100.0))
            .collect();
        println!("  {:>3} s: checking {:?}", t1.elapsed().as_secs(), checking);
        if checking.is_empty() || t1.elapsed() > Duration::from_secs(120) {
            break snap;
        }
    };
    let ids: Vec<u32> = snap
        .rows
        .iter()
        .filter(|r| stop_later.iter().any(|s| s.name == r.name))
        .map(|r| r.id)
        .collect();
    let _ = client.call(Request::Pause { ids });
    std::thread::sleep(Duration::from_secs(8));

    let shots = [
        Shot {
            file: "main-light.png",
            mode: "light",
            tint: "neutral",
            accent: "green",
            density: "cozy",
            show: "",
            select: 2,
        },
        Shot {
            file: "main-dark.png",
            mode: "dark",
            tint: "neutral",
            accent: "green",
            density: "cozy",
            show: "",
            select: 2,
        },
        Shot {
            file: "torrent-menu.png",
            mode: "light",
            tint: "neutral",
            accent: "green",
            density: "cozy",
            show: "menu",
            select: 2,
        },
        Shot {
            file: "settings-dark.png",
            mode: "dark",
            tint: "neutral",
            accent: "green",
            density: "cozy",
            show: "settings",
            select: 2,
        },
        Shot {
            file: "tinted-amber-compact.png",
            mode: "light",
            tint: "tinted",
            accent: "amber",
            density: "compact",
            show: "tab:Content",
            select: 0,
        },
        Shot {
            file: "speed-dark-blue.png",
            mode: "dark",
            tint: "tinted",
            accent: "blue",
            density: "cozy",
            show: "tab:Speed",
            select: 2,
        },
    ];
    let ui = bin("ttorrent-ui")?;
    let engine_pid = engine.id();
    let mut mem_rows = Vec::new();
    for s in &shots {
        let prefs = UiPrefs {
            mode: s.mode.into(),
            tint: s.tint.into(),
            accent: s.accent.into(),
            density: s.density.into(),
            window: Some([80.0, 60.0, 1280.0, 800.0]),
            ..UiPrefs::default()
        };
        client
            .call(Request::SetUiPrefs { prefs })
            .map_err(|e| anyhow!(e))?;
        let mut child = Command::new(&ui)
            .arg("--data-dir")
            .arg(&data)
            .env("TT_UI_SHOW", s.show)
            .env("TT_UI_SELECT", s.select.to_string())
            .spawn()?;
        std::thread::sleep(Duration::from_millis(3500));
        let out = shots_dir.join(s.file);
        match capture(engine_pid, &out) {
            Ok(_) => println!("  {}", out.display()),
            Err(e) => eprintln!("  {}: {e}", s.file),
        }
        let e = tt_platform::process_memory(Some(engine_pid)).unwrap_or_default();
        let w = tt_platform::process_memory(Some(child.id())).unwrap_or_default();
        mem_rows.push((s.file, e, w));
        let _ = client.call(Request::HideWindow);
        wait_exit(child.id(), 5);
        let _ = child.kill();
        let _ = child.wait();
    }
    // The Add torrent dialog: open a .torrent the way a double-click does.
    // The engine starts the window itself, with the dialog in front.
    {
        let prefs = UiPrefs {
            mode: "light".into(),
            window: Some([80.0, 60.0, 1280.0, 800.0]),
            ..UiPrefs::default()
        };
        client
            .call(Request::SetUiPrefs { prefs })
            .map_err(|e| anyhow!(e))?;
        let file = std::fs::canonicalize(&root)?.join("Sintel.torrent");
        std::fs::write(&file, fixture::sintel_torrent())?;
        client
            .call(Request::Open {
                sources: vec![AddSource::File {
                    path: file.to_string_lossy().into_owned(),
                }],
            })
            .map_err(|e| anyhow!(e))?;
        std::thread::sleep(Duration::from_millis(4000));
        let out = shots_dir.join("add-torrent.png");
        match capture(engine_pid, &out) {
            Ok(pid) => {
                println!("  {}", out.display());
                let w = tt_platform::process_memory(Some(pid)).unwrap_or_default();
                let e = tt_platform::process_memory(Some(engine_pid)).unwrap_or_default();
                mem_rows.push(("add-torrent.png", e, w));
            }
            Err(e) => eprintln!("  add-torrent.png: {e}"),
        }
        if let Ok(Response::PendingAdds { items }) = client.call(Request::PendingAdds) {
            println!(
                "  pending: {:?}",
                items.iter().map(|p| &p.label).collect::<Vec<_>>()
            );
            for p in items {
                let _ = client.call(Request::CancelAdd { token: p.token });
            }
        }
        let _ = client.call(Request::HideWindow);
    }
    std::thread::sleep(Duration::from_secs(5));
    let tray = tt_platform::process_memory(Some(engine_pid)).unwrap_or_default();
    println!("\nmemory (MB), 14 torrents, debug or release as built:");
    println!("| shot | engine TM | window TM | both TM | both private |");
    println!("|---|---|---|---|---|");
    for (f, e, w) in &mem_rows {
        println!(
            "| {f} | {:.1} | {:.1} | {:.1} | {:.1} |",
            mb(e.private_working_set),
            mb(w.private_working_set),
            mb(e.private_working_set + w.private_working_set),
            mb(e.private_bytes + w.private_bytes)
        );
    }
    println!(
        "| window closed (engine only) | {:.1} | – | {:.1} | {:.1} |",
        mb(tray.private_working_set),
        mb(tray.private_working_set),
        mb(tray.private_bytes)
    );
    let _ = client.call(Request::Quit);
    wait_exit(engine_pid, 10);
    let _ = engine.kill();
    seeder.stop();
    Ok(())
}

/// Sizes for a synthetic library: deterministic, median around 700 MB, a long tail.
fn sizes(n: usize) -> Vec<u64> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let u = (x % 10_000) as f64 / 10_000.0;
            // Log-uniform between 20 MB and 40 GB.
            let lo = (20.0f64 * 1024.0 * 1024.0).ln();
            let hi = (40.0f64 * 1024.0 * 1024.0 * 1024.0).ln();
            (lo + u * (hi - lo)).exp() as u64
        })
        .collect()
}

/// N torrents with librqbit resume data (session.json, .torrent, full .bitv),
/// so an engine starts them without hashing (S1, M2, T1 fixtures).
pub fn library(args: &[String]) -> Result<()> {
    let n: usize = args.first().context("library N DIR")?.parse()?;
    let dir = PathBuf::from(args.get(1).context("library N DIR")?);
    // --max-mb caps sizes, to separate per-torrent from per-piece memory.
    let max = args
        .iter()
        .position(|a| a == "--max-mb")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|mb| mb * 1024 * 1024);
    // --dense writes real zero blocks instead of sparse holes (soak runs).
    let dense = args.iter().any(|a| a == "--dense");
    let _ = std::fs::remove_dir_all(&dir);
    let content = dir.join("content");
    let session = dir.join("engine").join("session");
    std::fs::create_dir_all(&session)?;
    let content_abs = {
        std::fs::create_dir_all(&content)?;
        std::fs::canonicalize(&content)?
    };
    let mut map = serde_json::Map::new();
    let mut total = 0u64;
    for (i, size) in sizes(n).into_iter().enumerate() {
        let size = max.map_or(size, |m| size.min(m));
        let name = format!("open-content-{i:04}.iso");
        if dense {
            fixture::dense_file(&content.join(&name), size)?;
        } else {
            fixture::sparse_file(&content.join(&name), size)?;
        }
        let (bytes, hash) = fixture::zero_torrent(&name, size);
        std::fs::write(session.join(format!("{hash}.torrent")), &bytes)?;
        let pieces = size.div_ceil(fixture::piece_length(size));
        let mut bits = vec![0xFFu8; pieces.div_ceil(8) as usize];
        let rem = (pieces % 8) as u32;
        if rem != 0
            && let Some(last) = bits.last_mut()
        {
            *last = 0xFFu8 << (8 - rem);
        }
        std::fs::write(session.join(format!("{hash}.bitv")), &bits)?;
        map.insert(
            i.to_string(),
            serde_json::json!({
                "info_hash": hash,
                "trackers": [],
                "output_folder": content_abs,
                "only_files": null,
                "is_paused": false,
            }),
        );
        total += size;
    }
    std::fs::write(
        session.join("session.json"),
        serde_json::to_vec(&serde_json::json!({ "torrents": map }))?,
    )?;
    let pieces: u64 = sizes(n)
        .into_iter()
        .map(|s| max.map_or(s, |m| s.min(m)))
        .map(|s| s.div_ceil(fixture::piece_length(s)))
        .sum();
    println!(
        "{n} torrents, {pieces} pieces, {:.0} GB of {} content, resume data in {}",
        total as f64 / (1u64 << 30) as f64,
        if dense { "dense" } else { "sparse" },
        session.display()
    );
    Ok(())
}

/// Launch to list on screen, then memory with the window open and closed.
pub fn startup(args: &[String]) -> Result<()> {
    let dir = PathBuf::from(args.first().context("startup DIR")?);
    let data = dir.join("engine");
    let t0 = Instant::now();
    let mut engine = start_engine(&data, true)?;
    let engine_pid = engine.id();
    // The list is on screen when the window's middle has row text in it.
    let mut shown = None;
    #[cfg(windows)]
    while t0.elapsed() < Duration::from_secs(20) {
        if let Some(w) = ui_window(engine_pid)
            && let Ok(img) = w.capture_image()
        {
            let (wd, ht) = img.dimensions();
            let mut dark = 0;
            for y in (ht / 4..ht / 2).step_by(2) {
                for x in (wd / 3..wd * 2 / 3).step_by(2) {
                    let p = img.get_pixel(x, y);
                    // Text, including "Starting" rows drawn at 62 % opacity
                    // (about 340); the light background is about 750.
                    if (p[0] as u32 + p[1] as u32 + p[2] as u32) < 500 {
                        dark += 1;
                    }
                }
            }
            if dark > 200 {
                shown = Some(t0.elapsed());
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ui_pid_early = {
        #[cfg(windows)]
        {
            ui_window(engine_pid).and_then(|w| w.pid().ok())
        }
        #[cfg(not(windows))]
        {
            None::<u32>
        }
    };
    let ui_cpu0 = ui_pid_early.and_then(tt_platform::process_io);
    std::thread::sleep(Duration::from_secs(10));
    let ui_cpu1 = ui_pid_early.and_then(tt_platform::process_io);
    let ui_pid = {
        #[cfg(windows)]
        {
            ui_window(engine_pid).and_then(|w| w.pid().ok())
        }
        #[cfg(not(windows))]
        {
            None::<u32>
        }
    };
    // Memory once every torrent has started (the steady state): sampling
    // while startup waves are still running flatters it.
    let client = connect(&data)?;
    let t_wait = Instant::now();
    while t_wait.elapsed() < Duration::from_secs(180) {
        match client.call(Request::Snapshot) {
            Ok(Response::Snapshot(s)) if s.status.starting.is_none() && !s.from_snapshot => break,
            _ => std::thread::sleep(Duration::from_millis(500)),
        }
    }
    std::thread::sleep(Duration::from_secs(5));
    println!(
        "every torrent started {:.0} s after launch; memory sampled 5 s later",
        t0.elapsed().as_secs_f64() - 5.0
    );
    let e = tt_platform::process_memory(Some(engine_pid)).unwrap_or_default();
    let w = ui_pid
        .and_then(|p| tt_platform::process_memory(Some(p)))
        .unwrap_or_default();
    let _ = client.call(Request::HideWindow);
    std::thread::sleep(Duration::from_secs(10));
    let tray = tt_platform::process_memory(Some(engine_pid)).unwrap_or_default();
    let count = match client.call(Request::Snapshot) {
        Ok(Response::Snapshot(s)) => s.rows.len(),
        _ => 0,
    };
    // Disk: everything the engine read until every startup wave is done.
    let waves = Duration::from_millis(1500) * (count as u32).div_ceil(25) + Duration::from_secs(15);
    while t0.elapsed() < waves {
        std::thread::sleep(Duration::from_millis(500));
    }
    let io_start = tt_platform::process_io(engine_pid).unwrap_or_default();
    let at = t0.elapsed();
    // CPU: the engine idling in the tray (nothing transferring: --offline).
    let idle_secs = 30;
    std::thread::sleep(Duration::from_secs(idle_secs));
    let io_idle = tt_platform::process_io(engine_pid).unwrap_or_default();
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    let pct = |ms: u64, secs: f64| ms as f64 / (secs * 1000.0) * 100.0;
    let idle_ms = io_idle.cpu_ms.saturating_sub(io_start.cpu_ms);
    println!("torrents: {count}");
    println!(
        "engine file reads, launch to {:.0} s (all waves done): {:.1} MB in {} reads (includes OS cache hits); writes {:.1} MB",
        at.as_secs_f64(),
        mb(io_start.read_bytes),
        io_start.read_ops,
        mb(io_start.write_bytes)
    );
    println!(
        "engine CPU, launch to {:.0} s: {:.2} s of CPU time",
        at.as_secs_f64(),
        io_start.cpu_ms as f64 / 1000.0
    );
    println!(
        "engine CPU idle in the tray, {idle_secs} s: {:.2} % of one core, {:.3} % of all {cores} (Task Manager's scale)",
        pct(idle_ms, idle_secs as f64),
        pct(idle_ms, idle_secs as f64) / cores
    );
    if let (Some(a), Some(b)) = (ui_cpu0, ui_cpu1) {
        println!(
            "window CPU, first 10 s after the list showed: {:.2} % of one core",
            pct(b.cpu_ms.saturating_sub(a.cpu_ms), 10.0)
        );
    }
    match shown {
        Some(d) => println!("launch to list on screen: {} ms", d.as_millis()),
        None => println!("launch to list on screen: not detected in 20 s"),
    }
    println!("| state | Task Manager | working set | private bytes |");
    println!("|---|---|---|---|");
    println!(
        "| window open, engine + window | {:.1} MB | {:.1} MB | {:.1} MB |",
        mb(e.private_working_set + w.private_working_set),
        mb(e.working_set + w.working_set),
        mb(e.private_bytes + w.private_bytes)
    );
    println!(
        "| window open, engine | {:.1} MB | {:.1} MB | {:.1} MB |",
        mb(e.private_working_set),
        mb(e.working_set),
        mb(e.private_bytes)
    );
    println!(
        "| window open, window | {:.1} MB | {:.1} MB | {:.1} MB |",
        mb(w.private_working_set),
        mb(w.working_set),
        mb(w.private_bytes)
    );
    println!(
        "| window closed (tray only) | {:.1} MB | {:.1} MB | {:.1} MB |",
        mb(tray.private_working_set),
        mb(tray.working_set),
        mb(tray.private_bytes)
    );
    let _ = client.call(Request::Quit);
    wait_exit(engine_pid, 15);
    let _ = engine.kill();
    Ok(())
}

/// Turn on Settings › AI agents for a test engine (users do this in Settings).
pub fn enable_mcp(args: &[String]) -> Result<()> {
    let data = PathBuf::from(args.first().context("enable-mcp DATA_DIR")?);
    let c = connect(&data)?;
    let mut s = match c.call(Request::GetSettings) {
        Ok(Response::Settings(s)) => *s,
        other => bail!("settings: {other:?}"),
    };
    s.mcp_enabled = true;
    c.call(Request::SetSettings {
        settings: Box::new(s),
    })
    .map_err(|e| anyhow!(e))?;
    println!("AI agents allowed for {}", data.display());
    Ok(())
}

/// Engine alone (no window): memory with torrents live, then all stopped.
/// Locates where engine memory goes (docs/perf/README.md § 6 step 2).
pub fn engine_mem(args: &[String]) -> Result<()> {
    let dir = PathBuf::from(args.first().context("engine-mem DIR")?);
    let data = dir.join("engine");
    let mut engine = start_engine(&data, false)?;
    let pid = engine.id();
    let client = connect(&data)?;
    let row = |label: &str| {
        let m = tt_platform::process_memory(Some(pid)).unwrap_or_default();
        println!(
            "| {label} | {:.1} MB | {:.1} MB | {:.1} MB |",
            mb(m.private_working_set),
            mb(m.working_set),
            mb(m.private_bytes)
        );
    };
    println!("| engine only | Task Manager | working set | private bytes |");
    println!("|---|---|---|---|");
    std::thread::sleep(Duration::from_secs(3));
    row("3 s after start (waves running)");
    std::thread::sleep(Duration::from_secs(60));
    row("63 s, all live");
    let _ = client.call(Request::PauseAll);
    std::thread::sleep(Duration::from_secs(20));
    row("all stopped, 20 s later");
    let _ = client.call(Request::ResumeAll);
    let _ = client.call(Request::Quit);
    wait_exit(pid, 15);
    let _ = engine.kill();
    Ok(())
}

/// One screenshot of an existing data folder: `shot DATA_DIR OUT.png`.
/// TT_UI_SHOW and TT_UI_SELECT pass through to the window.
pub fn shot(args: &[String]) -> Result<()> {
    let data = PathBuf::from(args.first().context("shot DATA_DIR OUT")?);
    let out = PathBuf::from(args.get(1).context("shot DATA_DIR OUT")?);
    let mut engine = start_engine(&data, false)?;
    let pid = engine.id();
    let client = connect(&data)?;
    let mut ui = Command::new(bin("ttorrent-ui")?)
        .arg("--data-dir")
        .arg(&data)
        .spawn()?;
    std::thread::sleep(Duration::from_millis(4000));
    let r = capture(pid, &out);
    let _ = client.call(Request::Quit);
    wait_exit(pid, 10);
    let _ = ui.kill();
    let _ = engine.kill();
    r.map(|_| println!("{}", out.display()))
}

/// Open ITEM the way a double-click or a browser's magnet link does: a second
/// `ttorrent ITEM` hands it to the running engine, which shows the Add
/// torrent dialog. Captures the window, prints the dialog queue, cancels it.
pub fn open_shot(args: &[String]) -> Result<()> {
    let usage = "open-shot DATA_DIR ITEM OUT";
    let data = PathBuf::from(args.first().context(usage)?);
    let item = args.get(1).context(usage)?;
    let out = PathBuf::from(args.get(2).context(usage)?);
    let mut engine = start_engine(&data, false)?;
    let pid = engine.id();
    let client = connect(&data)?;
    // Screenshots never show the owner's own folders (AGENTS.md): save into
    // the fixture's downloads folder.
    if let Ok(Response::Settings(mut s)) = client.call(Request::GetSettings) {
        let dl = data.parent().unwrap_or(&data).join("downloads");
        std::fs::create_dir_all(&dl)?;
        s.default_save_path = std::fs::canonicalize(&dl)?
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_owned();
        let _ = client.call(Request::SetSettings { settings: s });
    }
    let t0 = Instant::now();
    let status = Command::new(bin("ttorrent")?)
        .arg("--data-dir")
        .arg(&data)
        .arg(item)
        .status()?;
    println!(
        "second launch exited {status} after {} ms",
        t0.elapsed().as_millis()
    );
    std::thread::sleep(Duration::from_millis(4000));
    let r = capture(pid, &out);
    // `--confirm DIR`: press Add with DIR as the save folder, then list what
    // was created there (checks the content layout end to end).
    let confirm = args
        .iter()
        .position(|a| a == "--confirm")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    if let Ok(Response::PendingAdds { items }) = client.call(Request::PendingAdds) {
        for p in &items {
            println!(
                "pending: {} ({:?})",
                p.label,
                std::mem::discriminant(&p.state)
            );
            match &confirm {
                Some(dir) => {
                    std::fs::create_dir_all(dir)?;
                    let mut options = p.defaults.clone();
                    options.save_path =
                        Some(std::fs::canonicalize(dir)?.to_string_lossy().into_owned());
                    options.paused = true;
                    let r = client.call(Request::ConfirmAdd {
                        token: p.token,
                        options,
                        never_show_again: false,
                    });
                    println!("confirm: {r:?}");
                }
                None => {
                    let _ = client.call(Request::CancelAdd { token: p.token });
                }
            }
        }
    }
    if let Some(dir) = &confirm {
        std::thread::sleep(Duration::from_secs(3));
        for e in walk(dir) {
            println!("  {}", e.display());
        }
        if let Ok(Response::Snapshot(s)) = client.call(Request::Snapshot)
            && let Some(r) = s.rows.iter().find(|r| r.name == "Sintel")
        {
            println!(
                "row: {} | save path {} | {:?}",
                r.name, r.save_path, r.state
            );
            let _ = client.call(Request::Remove {
                ids: vec![r.id],
                delete_files: true,
            });
        }
    }
    let _ = client.call(Request::Quit);
    wait_exit(pid, 10);
    let _ = engine.kill();
    r.map(|_| println!("{}", out.display()))
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        out.push(p.clone());
        if p.is_dir() {
            out.extend(walk(&p));
        }
    }
    out.sort();
    out
}

/// `winshot PID OUT`: capture the largest window of a process (any app), from
/// its own pixels.
pub fn winshot(args: &[String]) -> Result<()> {
    let pid: u32 = args.first().context("winshot PID OUT")?.parse()?;
    let out = PathBuf::from(args.get(1).context("winshot PID OUT")?);
    #[cfg(windows)]
    {
        let w = xcap::Window::all()?
            .into_iter()
            .filter(|w| w.pid().is_ok_and(|p| p == pid))
            .filter(|w| !w.is_minimized().unwrap_or(true))
            .max_by_key(|w| u64::from(w.width().unwrap_or(0)) * u64::from(w.height().unwrap_or(0)))
            .ok_or_else(|| anyhow!("no window for pid {pid}"))?;
        w.capture_image()?.save(&out)?;
        println!("{}", out.display());
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, out);
        bail!("window capture is Windows-only for now")
    }
}
