//! `ttorrent`: the engine process. A second launch hands its torrents and
//! magnet links to the running one and exits. Files and links the system
//! opens (double-click, a browser's magnet link) arrive as arguments on
//! Windows and Linux; see docs/associations.md for macOS.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;
use tt_engine::Paths;
use tt_engine::core::Engine;
use tt_ipc::{AddSource, ClientKind, Request};

struct Args {
    data_dir: Option<PathBuf>,
    minimized: bool,
    headless: bool,
    offline: bool,
    import_qb: Option<Option<PathBuf>>,
    items: Vec<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        data_dir: None,
        minimized: false,
        headless: false,
        offline: false,
        import_qb: None,
        items: Vec::new(),
    };
    let mut it = std::env::args_os().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.to_string_lossy().as_ref() {
            "--version" => {
                println!("tTorrent {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--data-dir" => a.data_dir = it.next().map(PathBuf::from),
            "--minimized" => a.minimized = true,
            "--headless" => a.headless = true,
            "--offline" => a.offline = true,
            "--mcp" => {}
            "--scope" => {
                it.next();
            }
            "--import-qbittorrent" => {
                let dir = it
                    .peek()
                    .filter(|p| !p.to_string_lossy().starts_with("--"))
                    .map(PathBuf::from);
                if dir.is_some() {
                    it.next();
                }
                a.import_qb = Some(dir);
            }
            "--register" => {
                if let Ok(exe) = std::env::current_exe() {
                    match tt_platform::assoc::make_default(&exe) {
                        Ok(r) => println!("{r:?}"),
                        Err(e) => eprintln!("tTorrent: {e:#}"),
                    }
                    println!("{:?}", tt_platform::assoc::status(&exe));
                }
                std::process::exit(0);
            }
            s if s.starts_with("--") => eprintln!("unknown option {s}"),
            _ => a.items.push(arg.to_string_lossy().into_owned()),
        }
    }
    a
}

fn source_of(item: &str) -> AddSource {
    if item.starts_with("magnet:") {
        AddSource::Magnet { uri: item.into() }
    } else if item.starts_with("http://") || item.starts_with("https://") {
        AddSource::Url { url: item.into() }
    } else {
        let p = std::path::Path::new(item);
        let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        AddSource::File {
            path: abs.to_string_lossy().into_owned(),
        }
    }
}

fn main() {
    let args = parse_args();
    let data = args
        .data_dir
        .clone()
        .or_else(tt_platform::data_dir)
        .unwrap_or_else(|| PathBuf::from("ttorrent-data"));
    let paths = Paths::new(data);
    let _ = std::fs::create_dir_all(&paths.data);
    let instance = paths.instance();

    // `ttorrent --mcp`: a stdio MCP server for an AI agent (docs/mcp.md).
    if let Some(scope) = mcp_scope() {
        std::process::exit(run_mcp(&paths, &instance, scope));
    }

    // One engine per data folder: a second launch forwards its items.
    let listener = match tt_engine::server::listen(&instance) {
        Ok(l) => l,
        Err(_) => {
            forward(&instance, &args);
            return;
        }
    };

    init_logging(&paths);
    tracing::info!(data = %paths.data.display(), "tTorrent {} starting", env!("CARGO_PKG_VERSION"));

    // Two workers keep threads (and memory) low; TT_WORKERS and TT_BLOCKING
    // override them for perf experiments (docs/perf/README.md).
    let env_n = |k: &str, d: usize| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(env_n("TT_WORKERS", 2))
        .max_blocking_threads(env_n("TT_BLOCKING", 8))
        .thread_name("tt-engine")
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("tTorrent: could not start: {e}");
            std::process::exit(1);
        }
    };
    let engine = match Engine::open(paths.clone(), rt.handle().clone()) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("opening the database: {e:#}");
            eprintln!("tTorrent: {e:#}");
            std::process::exit(1);
        }
    };
    if args.offline {
        engine.set_offline();
    }
    // The window can connect and draw the snapshot before torrents start.
    tt_engine::server::serve(engine.clone(), listener);
    if !args.headless && !args.minimized {
        engine.window.show(&engine);
    }

    // The tray runs on this (main) thread; torrents run on the runtime's threads.
    let exit = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker = {
        let engine = engine.clone();
        let exit = exit.clone();
        let items = args.items.clone();
        let import_qb = args.import_qb.clone();
        std::thread::Builder::new()
            .name("tt-engine-main".into())
            .spawn(move || {
                rt.block_on(run(engine, items, import_qb));
                rt.shutdown_timeout(std::time::Duration::from_secs(3));
                exit.store(true, std::sync::atomic::Ordering::Relaxed);
            })
    };
    let Ok(worker) = worker else {
        eprintln!("tTorrent: could not start its worker thread");
        std::process::exit(1);
    };
    if args.headless {
        let _ = worker.join();
    } else {
        let e = engine.clone();
        tt_platform::tray::run(engine.tray.clone(), exit, move |a| e.tray_action(a));
        let _ = worker.join();
    }
    tracing::info!("tTorrent stopped");
}

async fn run(engine: Arc<Engine>, items: Vec<String>, import_qb: Option<Option<PathBuf>>) {
    {
        if let Err(e) = engine.start().await {
            tracing::error!("starting the session: {e:#}");
            eprintln!("tTorrent: {e:#}");
            return;
        }
        backup_daily(&engine);
        if let Some(dir) = &import_qb {
            match engine.import_qbittorrent(dir.clone()).await {
                Ok(n) => tracing::info!("imported {n} torrents from qBittorrent"),
                Err(e) => tracing::error!("import: {e:#}"),
            }
        }
        if !items.is_empty() {
            let sources = items.iter().map(|i| source_of(i)).collect();
            if let Err(e) = engine.handle(Request::Open { sources }).await {
                tracing::warn!("adding: {e:#}");
            }
        }
        engine.exit.notified().await;
        engine.shutdown().await;
    }
}

fn mcp_scope() -> Option<tt_mcp::Scope> {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--mcp") {
        return None;
    }
    let scope = args
        .iter()
        .position(|a| a == "--scope")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| tt_mcp::Scope::parse(s))
        .unwrap_or(tt_mcp::Scope::Read);
    Some(scope)
}

/// Connect to the engine (starting it in the tray if needed) and serve MCP on stdio.
fn run_mcp(paths: &Paths, instance: &str, scope: tt_mcp::Scope) -> i32 {
    let connect = || {
        tt_ipc::transport::Client::connect(
            instance,
            ClientKind::Mcp {
                agent: "stdio".into(),
            },
        )
    };
    let client = match connect() {
        Ok((c, _)) => c,
        Err(_) => {
            let started = std::env::current_exe().ok().and_then(|exe| {
                std::process::Command::new(exe)
                    .arg("--data-dir")
                    .arg(&paths.data)
                    .arg("--minimized")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .spawn()
                    .ok()
            });
            if started.is_none() {
                eprintln!("tTorrent: could not start the engine");
                return 1;
            }
            let mut got = None;
            for _ in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok((c, _)) = connect() {
                    got = Some(c);
                    break;
                }
            }
            match got {
                Some(c) => c,
                None => {
                    eprintln!("tTorrent: the engine did not start");
                    return 1;
                }
            }
        }
    };
    let server = tt_mcp::Server {
        engine: client,
        scope,
    };
    match server.serve_stdio() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("tTorrent MCP: {e:#}");
            1
        }
    }
}

fn forward(instance: &str, args: &Args) {
    let Ok((client, _events)) =
        tt_ipc::transport::Client::connect(instance, ClientKind::SecondLaunch)
    else {
        eprintln!("tTorrent is already running but did not answer");
        return;
    };
    if !args.items.is_empty() {
        // The running engine shows the Add torrent dialog (or adds them,
        // when that dialog is off) and brings its window forward.
        let sources = args.items.iter().map(|i| source_of(i)).collect();
        if let Err(e) = client.call(Request::Open { sources }) {
            eprintln!("tTorrent: {e}");
        }
    } else if !args.minimized && !args.headless {
        let _ = client.call(Request::ShowWindow);
    }
}

fn init_logging(paths: &Paths) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("TT_LOG").unwrap_or_else(|_| {
        EnvFilter::new("info,librqbit=warn,librqbit_dht=warn,librqbit_upnp=warn")
    });
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log);
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false);
    match file {
        Ok(f) => builder.with_writer(Arc::new(f)).init(),
        Err(_) => builder.init(),
    }
}

/// docs/missing-files.md case 12: a daily copy of the database, last 7 kept.
fn backup_daily(engine: &Arc<Engine>) {
    let dir = &engine.paths.backups;
    let _ = std::fs::create_dir_all(dir);
    let day = tt_engine::now_unix() / 86_400;
    let path = dir.join(format!("ttorrent-{day}.db"));
    if path.exists() {
        return;
    }
    if let Err(e) = engine.backup(&path) {
        tracing::warn!("backup: {e:#}");
        return;
    }
    let mut old: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "db"))
        .collect();
    old.sort();
    while old.len() > 7 {
        let _ = std::fs::remove_file(old.remove(0));
    }
}
