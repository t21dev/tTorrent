//! The tray icon and its menu: qBittorrent's items and order, plus a live
//! header and "Pause for 1 hour" (DESIGN.md › Tray and menu-bar icon).
//! It lives in the engine process, so it costs no GPU memory.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    ToggleWindow,
    AddFile,
    AddMagnet,
    SlowMode,
    SpeedLimits,
    PauseResumeAll,
    PauseHour,
    Exit,
}

/// What the tray shows; the engine updates it, the tray reads it once a second.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayState {
    pub header: String,
    pub tooltip: String,
    pub slow_mode: bool,
    pub window_open: bool,
    pub all_paused: bool,
}

pub type Shared = Arc<Mutex<TrayState>>;

/// Run the tray on this thread until `exit` is set. Calls `on_action` on this thread.
pub fn run(state: Shared, exit: Arc<AtomicBool>, on_action: impl Fn(TrayAction) + 'static) {
    #[cfg(windows)]
    if let Err(e) = win::run(state, exit.clone(), on_action) {
        log::warn!("tray: {e:#}");
    } else {
        return;
    }
    #[cfg(not(windows))]
    let _ = (state, on_action);
    while !exit.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(windows)]
mod win {
    use super::{Shared, TrayAction};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tray_icon::menu::accelerator::{Accelerator, Code, Modifiers};
    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, KillTimer, MSG, SetTimer, TranslateMessage, WM_TIMER,
    };

    const ICON: &[u8] = include_bytes!("../../../assets/brand/tray-32.rgba");

    pub fn run(
        state: Shared,
        exit: Arc<AtomicBool>,
        on_action: impl Fn(TrayAction) + 'static,
    ) -> anyhow::Result<()> {
        let ctrl = Modifiers::CONTROL;
        let header = MenuItem::new("tTorrent", false, None);
        let show = MenuItem::new("Show tTorrent", true, None);
        let add_file = MenuItem::new(
            "Add torrent file…",
            true,
            Some(Accelerator::new(ctrl, Code::KeyO)),
        );
        let add_magnet = MenuItem::new(
            "Add magnet link…",
            true,
            Some(Accelerator::new(
                Modifiers::CONTROL | Modifiers::SHIFT,
                Code::KeyO,
            )),
        );
        let slow = CheckMenuItem::new("Slow mode (alternative speed limits)", true, false, None);
        let limits = MenuItem::new("Set global speed limits…", true, None);
        let pause_all = MenuItem::new(
            "Pause all",
            true,
            Some(Accelerator::new(
                Modifiers::CONTROL | Modifiers::SHIFT,
                Code::KeyP,
            )),
        );
        let pause_hour = MenuItem::new("Pause for 1 hour", true, None);
        let quit = MenuItem::new("Exit", true, Some(Accelerator::new(ctrl, Code::KeyQ)));
        let menu = Menu::new();
        menu.append_items(&[
            &header,
            &PredefinedMenuItem::separator(),
            &show,
            &PredefinedMenuItem::separator(),
            &add_file,
            &add_magnet,
            &PredefinedMenuItem::separator(),
            &slow,
            &limits,
            &PredefinedMenuItem::separator(),
            &pause_all,
            &pause_hour,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;
        let icon = Icon::from_rgba(ICON.to_vec(), 32, 32)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("tTorrent")
            .with_icon(icon)
            .build()?;

        let ids: Vec<(MenuId, TrayAction)> = vec![
            (show.id().clone(), TrayAction::ToggleWindow),
            (add_file.id().clone(), TrayAction::AddFile),
            (add_magnet.id().clone(), TrayAction::AddMagnet),
            (slow.id().clone(), TrayAction::SlowMode),
            (limits.id().clone(), TrayAction::SpeedLimits),
            (pause_all.id().clone(), TrayAction::PauseResumeAll),
            (pause_hour.id().clone(), TrayAction::PauseHour),
            (quit.id().clone(), TrayAction::Exit),
        ];
        // Menu and click events arrive on this thread's message loop; queue them
        // and act after dispatch so `on_action` never runs inside a menu call.
        let queue: Rc<RefCell<Vec<TrayAction>>> = Rc::default();
        let (tx, rx) = std::sync::mpsc::channel::<TrayAction>();
        let tx2 = tx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Some((_, a)) = ids.iter().find(|(id, _)| *id == e.id) {
                let _ = tx.send(*a);
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                let _ = tx2.send(TrayAction::ToggleWindow);
            }
        }));

        // SAFETY: a thread timer (no window) on this thread, killed below; the
        // MSG struct is owned here and only passed to the API by pointer.
        let timer = unsafe { SetTimer(None, 0, 1000, None) };
        let mut last = super::TrayState::default();
        let mut msg = MSG::default();
        loop {
            // SAFETY: standard message loop on the thread that owns the tray window.
            let got = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            if !got.as_bool() || exit.load(Ordering::Relaxed) {
                break;
            }
            // SAFETY: msg was filled by GetMessageW above.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            while let Ok(a) = rx.try_recv() {
                queue.borrow_mut().push(a);
            }
            for a in queue.borrow_mut().drain(..) {
                on_action(a);
            }
            if msg.message == WM_TIMER {
                let now = state.lock().map(|s| s.clone()).unwrap_or_default();
                if now != last {
                    header.set_text(&now.header);
                    let _ = tray.set_tooltip(Some(&now.tooltip));
                    slow.set_checked(now.slow_mode);
                    show.set_text(if now.window_open {
                        "Hide tTorrent"
                    } else {
                        "Show tTorrent"
                    });
                    pause_all.set_text(if now.all_paused {
                        "Resume all"
                    } else {
                        "Pause all"
                    });
                    last = now;
                }
            }
        }
        // SAFETY: the timer id came from SetTimer on this thread.
        unsafe {
            let _ = KillTimer(None, timer);
        }
        drop(tray);
        Ok(())
    }
}
