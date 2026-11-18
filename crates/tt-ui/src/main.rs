//! `ttorrent-ui`: the window. The engine starts it when the window is shown;
//! it exits when the window closes (to the tray), freeing the GPU device.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod add_dialog;
mod app;
mod assets;
mod details;
mod model;
mod overlay;
mod parts;
mod settings;
mod text_input;
mod theme;
mod widgets;

use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px,
    size,
};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tt_ipc::transport::Client;
use tt_ipc::{ClientKind, Request, Response};

fn main() {
    let t0 = std::time::Instant::now();
    let mut data_dir: Option<PathBuf> = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        if a == "--data-dir" {
            data_dir = args.next().map(PathBuf::from);
        }
    }
    let data = data_dir
        .or_else(tt_platform::data_dir)
        .unwrap_or_else(|| PathBuf::from("ttorrent-data"));
    let instance = tt_ipc::transport::instance_id(&data);

    // The engine started us, so it is listening; allow it a moment if not yet.
    let mut conn = None;
    for _ in 0..50 {
        match Client::connect(&instance, ClientKind::Window) {
            Ok(c) => {
                conn = Some(c);
                break;
            }
            Err(_) => std::thread::sleep(Duration::from_millis(60)),
        }
    }
    let Some((client, events)) = conn else {
        eprintln!("tTorrent: the engine is not running");
        std::process::exit(1);
    };
    let client = Arc::new(client);
    let snapshot = match client.call(Request::Snapshot) {
        Ok(Response::Snapshot(s)) => *s,
        other => {
            eprintln!("tTorrent: no snapshot from the engine: {other:?}");
            std::process::exit(1);
        }
    };
    let _ = client.call(Request::Subscribe);
    let snapshot_ms = t0.elapsed().as_millis();

    // Engine events to an async channel the window's task awaits (no polling).
    let (tx, rx) = smol::channel::unbounded();
    std::thread::Builder::new()
        .name("tt-ui-events".into())
        .spawn(move || {
            while let Ok(e) = events.recv() {
                let exiting = matches!(e, tt_ipc::Event::Exiting);
                if tx.send_blocking(e).is_err() || exiting {
                    break;
                }
            }
        })
        .ok();

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            let _ = cx
                .text_system()
                .add_fonts(assets::FONTS.iter().map(|b| Cow::Borrowed(*b)).collect());
            app::bind_keys(cx);
            let prefs = snapshot.prefs.clone();
            let bounds = match prefs.window {
                Some([x, y, w, h]) if w >= 640.0 && h >= 400.0 => {
                    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
                }
                _ => Bounds::centered(None, size(px(1280.0), px(800.0)), cx),
            };
            let options = WindowOptions {
                window_bounds: Some(if prefs.maximized {
                    WindowBounds::Maximized(bounds)
                } else {
                    WindowBounds::Windowed(bounds)
                }),
                titlebar: Some(TitlebarOptions {
                    title: Some("tTorrent".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(14.0), px(14.0))),
                }),
                window_min_size: Some(size(px(720.0), px(480.0))),
                app_id: Some("tTorrent".into()),
                ..Default::default()
            };
            let client2 = client.clone();
            let opened = cx.open_window(options, move |window, cx| {
                cx.new(|cx| app::Workspace::new(client2, snapshot, rx, window, cx))
            });
            if opened.is_err() {
                eprintln!("tTorrent: could not open the window");
                cx.quit();
            }
            log::info!("snapshot in {snapshot_ms} ms");
            cx.on_window_closed(|cx| cx.quit()).detach();
            cx.activate(true);
        });
}
