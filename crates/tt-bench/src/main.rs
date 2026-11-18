//! `tt-bench`: black-box measurement of the real binaries (docs/perf/README.md).
//!
//!   tt-bench demo [--shots DIR]    synthetic open-content library on a local
//!                                  swarm, README screenshots, memory table
//!   tt-bench library N DIR         N synthetic torrents with resume data, for
//!                                  start-up and memory runs (S1, M2, T1)
//!   tt-bench startup DIR           time from launch to the list on screen,
//!                                  engine and window memory, startup disk
//!                                  reads and idle CPU
//!   tt-bench open-shot DATA ITEM OUT
//!                                  open a .torrent or magnet the way the
//!                                  system does (a second `ttorrent ITEM`),
//!                                  capture the Add torrent dialog
//!   tt-bench soak DIR [--hours H] [--rate-mb R] [--qb EXE]
//!                                  tTorrent and qBittorrent side by side for
//!                                  hours under the same upload load
//!   tt-bench qb DIR [QBITTORRENT_EXE]
//!                                  qBittorrent on the same library, isolated
//!                                  profile, offline: the like-for-like baseline

mod fixture;
mod qb;
mod run;
mod soak;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(String::as_str) {
        Some("demo") => run::demo(&args[1..]),
        Some("library") => run::library(&args[1..]),
        Some("startup") => run::startup(&args[1..]),
        Some("engine-mem") => run::engine_mem(&args[1..]),
        Some("shot") => run::shot(&args[1..]),
        Some("open-shot") => run::open_shot(&args[1..]),
        Some("qb") => qb::qb(&args[1..]),
        Some("soak") => soak::soak(&args[1..]),
        Some("winshot") => run::winshot(&args[1..]),
        Some("enable-mcp") => run::enable_mcp(&args[1..]),
        _ => {
            eprintln!("usage: tt-bench demo [--shots DIR] | library N DIR | startup DIR");
            std::process::exit(2);
        }
    };
    if let Err(e) = r {
        eprintln!("tt-bench: {e:#}");
        std::process::exit(1);
    }
}
