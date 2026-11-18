//! The window is a separate process (`ttorrent-ui`), started when shown and
//! ended when closed to the tray, so the engine never holds a GPU device
//! (PLAN.md §2).

use crate::Paths;
use crate::core::Engine;
use std::process::{Child, Command};
use std::sync::Mutex;
use tt_ipc::Event;

pub struct WindowCtl {
    child: Mutex<Option<Child>>,
    exe: Option<std::path::PathBuf>,
    data_dir: std::path::PathBuf,
}

impl WindowCtl {
    pub fn new(paths: &Paths) -> Self {
        let exe = std::env::current_exe().ok().and_then(|e| {
            let name = if cfg!(windows) {
                "ttorrent-ui.exe"
            } else {
                "ttorrent-ui"
            };
            let p = e.with_file_name(name);
            p.exists().then_some(p)
        });
        Self {
            child: Mutex::new(None),
            exe,
            data_dir: paths.data.clone(),
        }
    }

    pub fn is_open(&self) -> bool {
        let mut c = self.child.lock().unwrap_or_else(|e| e.into_inner());
        match c.as_mut().map(|ch| ch.try_wait()) {
            Some(Ok(None)) => true,
            Some(_) => {
                *c = None;
                false
            }
            None => false,
        }
    }

    /// Can a window be shown at all (`ttorrent-ui` sits next to the engine)?
    pub fn available(&self) -> bool {
        self.exe.is_some()
    }

    /// Show the window: bring it forward if it runs, otherwise start it.
    pub fn show(&self, engine: &Engine) {
        self.show_with(engine, None);
    }

    /// Show the window and run a command in it (tray › Add torrent file…).
    pub fn show_with(&self, engine: &Engine, command: Option<&str>) {
        if self.is_open() {
            engine.broadcast(Event::Activate);
            if let Some(c) = command {
                engine.broadcast(Event::Command { name: c.into() });
            }
            return;
        }
        let Some(exe) = &self.exe else {
            tracing::warn!("ttorrent-ui is not next to ttorrent; can't open the window");
            return;
        };
        let mut cmd = Command::new(exe);
        cmd.arg("--data-dir").arg(&self.data_dir);
        if let Some(c) = command {
            cmd.env("TT_UI_SHOW", c);
        }
        match cmd.spawn() {
            Ok(ch) => *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(ch),
            Err(e) => tracing::warn!("starting the window: {e}"),
        }
    }

    /// Hide to the tray: the window process exits by itself on `Exiting` or
    /// when asked; this waits briefly and then makes sure.
    pub fn close(&self) {
        let mut c = self.child.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut ch) = c.take() {
            for _ in 0..20 {
                if matches!(ch.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let _ = ch.kill();
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.is_open().then(|| {
            self.child
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .map(|c| c.id())
        })?
    }
}
