//! `tt-bench qb LIB_DIR [QBITTORRENT_EXE]`: qBittorrent on the same synthetic
//! library as `tt-bench startup`, so the two can be compared like for like.
//!
//! qBittorrent runs from its own `--profile` folder inside LIB_DIR (its own
//! settings, resume data, ports: an installed copy and its data are not
//! touched), offline (no DHT, LSD, PeX, UPnP; the torrents have no
//! trackers), with its Web API on 127.0.0.1:18080 for the bench only.
//!
//! 1. Populate: add LIB_DIR's 735 .torrent files with `skip_checking`, the
//!    same "trust the resume data" tTorrent's fixture has, then quit.
//! 2. Window launch: launch to window, launch to every torrent loaded,
//!    memory, file I/O and CPU at 60 s, then 30 s of idle CPU.
//! 3. Tray launch: the same, started hidden in the tray.

use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const API: &str = "http://127.0.0.1:18080/api/v2";

fn mb(b: u64) -> f64 {
    b as f64 / 1_048_576.0
}

fn plain(p: &Path) -> String {
    p.to_string_lossy().trim_start_matches(r"\\?\").to_owned()
}

fn get(path: &str) -> Option<String> {
    let o = Command::new("curl")
        .args(["-s", "-m", "5", &format!("{API}/{path}")])
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn post(path: &str, form: &[String]) -> Result<String> {
    let mut c = Command::new("curl");
    c.args(["-s", "-m", "300"]);
    for f in form {
        c.arg("-F").arg(f);
    }
    let o = c.arg(format!("{API}/{path}")).output()?;
    if !o.status.success() {
        bail!("curl {path}: {}", String::from_utf8_lossy(&o.stderr));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

fn ini(content: &str, hidden: bool) -> String {
    format!(
        "[LegalNotice]\nAccepted=true\n\n\
         [BitTorrent]\n\
         Session\\DHTEnabled=false\nSession\\LSDEnabled=false\nSession\\PeXEnabled=false\n\
         Session\\Port=18881\nSession\\QueueingSystemEnabled=false\n\
         Session\\DefaultSavePath={}\nSession\\Preallocation=false\n\n\
         [Network]\nPortForwardingEnabled=false\n\n\
         [AddNewTorrentDialog]\nEnabled=false\n\n\
         [GUI]\nStartUpWindowState={}\n\n\
         [Preferences]\n\
         General\\SystrayEnabled=true\nGeneral\\CloseToTray=true\nGeneral\\ExitConfirm=false\n\
         Connection\\ResolvePeerCountries=false\nAdvanced\\updateCheck=false\n\n\
         [WebUI]\n\
         Enabled=true\nPort=18080\nAddress=127.0.0.1\nLocalHostAuth=false\n\
         CSRFProtection=false\nHostHeaderValidation=false\nUsername=bench\n\
         Password_PBKDF2=\"{}\"\n",
        content.replace('\\', "/"),
        if hidden { "Hidden" } else { "Normal" },
        // qBittorrent 5 keeps its Web API off without credentials, even for
        // localhost. A fixed, bench-only password (PBKDF2-SHA512, 100,000
        // rounds, as qBittorrent stores it); the API listens on 127.0.0.1
        // only and localhost needs no login.
        "@ByteArray(AAECAwQFBgcICQoLDA0ODw==:eF4A52j8tlUiNbf4TWqwHDRRj/JyN/+D3byfn+OfcA8uSRWDRmQGs8HmjF9Kn6JlvEgXZ67cysjxw2EbuXAMYQ==)"
    )
}

fn launch(exe: &Path, profile: &Path) -> Result<Child> {
    Ok(Command::new(exe)
        .arg(format!("--profile={}", plain(profile)))
        .arg("--confirm-legal-notice")
        .spawn()?)
}

fn torrent_count() -> Option<usize> {
    let body = get("torrents/info")?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    v.as_array().map(Vec::len)
}

fn states() -> Vec<String> {
    get("torrents/info")
        .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|t| t["state"].as_str().map(str::to_owned))
        .collect()
}

fn quit(child: &mut Child) {
    let _ = post("app/shutdown", &[]);
    for _ in 0..240 {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
fn qb_window_visible(pid: u32) -> bool {
    xcap::Window::all().is_ok_and(|ws| {
        ws.into_iter().any(|w| {
            w.pid().is_ok_and(|p| p == pid)
                && w.title().is_ok_and(|t| t.starts_with("qBittorrent"))
                && !w.is_minimized().unwrap_or(true)
                && w.width().unwrap_or(0) >= 400
        })
    })
}

#[cfg(not(windows))]
fn qb_window_visible(_pid: u32) -> bool {
    false
}

struct Run {
    window_ms: Option<u128>,
    loaded_ms: Option<u128>,
    mem: tt_platform::ProcessMemory,
    io: tt_platform::ProcessIo,
    idle_cpu_ms: u64,
}

fn measure(exe: &Path, profile: &Path, n: usize, hidden: bool) -> Result<Run> {
    let t0 = Instant::now();
    let mut child = launch(exe, profile)?;
    let pid = child.id();
    let (mut window_ms, mut loaded_ms) = (None, None);
    while t0.elapsed() < Duration::from_secs(60) {
        if !hidden && window_ms.is_none() && qb_window_visible(pid) {
            window_ms = Some(t0.elapsed().as_millis());
        }
        if loaded_ms.is_none() && torrent_count() == Some(n) {
            loaded_ms = Some(t0.elapsed().as_millis());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let mem = tt_platform::process_memory(Some(pid)).unwrap_or_default();
    let io = tt_platform::process_io(pid).unwrap_or_default();
    std::thread::sleep(Duration::from_secs(30));
    let io2 = tt_platform::process_io(pid).unwrap_or_default();
    quit(&mut child);
    Ok(Run {
        window_ms,
        loaded_ms,
        mem,
        io,
        idle_cpu_ms: io2.cpu_ms.saturating_sub(io.cpu_ms),
    })
}

pub fn qb(args: &[String]) -> Result<()> {
    let dir = std::fs::canonicalize(args.first().context("qb LIB_DIR [QBITTORRENT_EXE]")?)?;
    let exe = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or(r"C:\Program Files\qBittorrent\qbittorrent.exe"),
    );
    if !exe.exists() {
        bail!("{} not found", exe.display());
    }
    let content = plain(&dir.join("content"));
    let mut torrents: Vec<PathBuf> = std::fs::read_dir(dir.join("engine").join("session"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "torrent"))
        .collect();
    torrents.sort();
    let n = torrents.len();
    if n == 0 {
        bail!(
            "no .torrent files; run `tt-bench library N {}` first",
            dir.display()
        );
    }
    let profile = dir.join("qb-profile");
    let _ = std::fs::remove_dir_all(&profile);
    let config = profile.join("qBittorrent").join("config");
    std::fs::create_dir_all(&config)?;
    std::fs::write(config.join("qBittorrent.ini"), ini(&content, false))?;
    if get("app/version").is_some() {
        bail!("something already answers on 127.0.0.1:18080");
    }

    // 1. Populate.
    println!(
        "qBittorrent {}: adding {n} torrents, no hash check",
        exe.display()
    );
    let mut child = launch(&exe, &profile)?;
    let t = Instant::now();
    let version = loop {
        if let Some(v) = get("app/version") {
            break v;
        }
        if t.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            bail!("qBittorrent's Web API did not answer in 60 s");
        }
        std::thread::sleep(Duration::from_millis(250));
    };
    let lt = get("app/buildInfo").unwrap_or_default();
    println!("  version {version}, build {lt}");
    for chunk in torrents.chunks(50) {
        let mut form = vec![
            format!("savepath={content}"),
            "skip_checking=true".to_owned(),
            "stopped=false".to_owned(),
            "paused=false".to_owned(),
        ];
        form.extend(chunk.iter().map(|p| format!("torrents=@{}", plain(p))));
        post("torrents/add", &form)?;
    }
    let t = Instant::now();
    loop {
        let st = states();
        let checking = st
            .iter()
            .filter(|s| s.contains("checking") || *s == "metaDL")
            .count();
        if st.len() == n && checking == 0 {
            break;
        }
        if t.elapsed() > Duration::from_secs(2700) {
            quit(&mut child);
            return Err(anyhow!(
                "only {} of {n} torrents ready after 45 min",
                st.len()
            ));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    let mut seen: Vec<String> = states();
    seen.sort();
    seen.dedup();
    println!("  added; states: {seen:?}");
    std::thread::sleep(Duration::from_secs(5));
    quit(&mut child);

    // 2. and 3.
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
    let w = measure(&exe, &profile, n, false)?;
    std::fs::write(config.join("qBittorrent.ini"), {
        // Keep what qBittorrent itself wrote; only flip the start-up state.
        let s = std::fs::read_to_string(config.join("qBittorrent.ini")).unwrap_or_default();
        if s.contains("StartUpWindowState=Normal") {
            s.replace("StartUpWindowState=Normal", "StartUpWindowState=Hidden")
        } else {
            format!("{s}\n[GUI]\nStartUpWindowState=Hidden\n")
        }
    })?;
    let h = measure(&exe, &profile, n, true)?;

    let ms = |v: Option<u128>| v.map_or("not seen".into(), |v| format!("{v} ms"));
    println!("\nqBittorrent {version}, {n} torrents (same synthetic library as tt-bench startup)");
    println!("| | window open | tray only |");
    println!("|---|---|---|");
    println!("| launch to window | {} | – |", ms(w.window_ms));
    println!(
        "| launch to all {n} torrents loaded (Web API) | {} | {} |",
        ms(w.loaded_ms),
        ms(h.loaded_ms)
    );
    println!(
        "| Task Manager at 60 s | {:.1} MB | {:.1} MB |",
        mb(w.mem.private_working_set),
        mb(h.mem.private_working_set)
    );
    println!(
        "| working set / private bytes at 60 s | {:.1} / {:.1} MB | {:.1} / {:.1} MB |",
        mb(w.mem.working_set),
        mb(w.mem.private_bytes),
        mb(h.mem.working_set),
        mb(h.mem.private_bytes)
    );
    println!(
        "| file reads, launch to 60 s | {:.1} MB in {} reads | {:.1} MB in {} reads |",
        mb(w.io.read_bytes),
        w.io.read_ops,
        mb(h.io.read_bytes),
        h.io.read_ops
    );
    println!(
        "| file writes, launch to 60 s | {:.1} MB | {:.1} MB |",
        mb(w.io.write_bytes),
        mb(h.io.write_bytes)
    );
    println!(
        "| CPU time, launch to 60 s | {:.2} s | {:.2} s |",
        w.io.cpu_ms as f64 / 1000.0,
        h.io.cpu_ms as f64 / 1000.0
    );
    let pct = |ms: u64| ms as f64 / 30_000.0 * 100.0;
    println!(
        "| idle CPU, next 30 s (one core / all {cores}) | {:.2} % / {:.3} % | {:.2} % / {:.3} % |",
        pct(w.idle_cpu_ms),
        pct(w.idle_cpu_ms) / cores,
        pct(h.idle_cpu_ms),
        pct(h.idle_cpu_ms) / cores
    );
    Ok(())
}
