//! The workspace: state, actions, keyboard map, and the root layout.

use crate::model::{Model, SideFilter};
use crate::overlay::{Dialog, Menu};
use crate::text_input::{InputEvent, TextInput};
use crate::theme::{FONT_UI, Theme};
use gpui::{
    App, AppContext, ClipboardItem, Context, Entity, FocusHandle, Focusable, IntoElement,
    KeyBinding, ParentElement, PathPromptOptions, Pixels, Point, Render, SharedString, Styled,
    Task, UniformListScrollHandle, Window, actions, div, prelude::*,
};
use std::sync::Arc;
use std::time::Duration;
use tt_ipc::transport::Client;
use tt_ipc::{
    AddSource, DetailsTab, Event, Request, Response, Snapshot, SpeedPoint, TorrentDetails,
    TorrentId, UiPrefs,
};

actions!(
    tt,
    [
        AddFile,
        AddMagnet,
        Resume,
        Pause,
        TogglePause,
        PauseAll,
        Remove,
        RemoveWithFiles,
        Rename,
        OpenFolder,
        CopyMagnet,
        Recheck,
        SelectAll,
        Filter,
        Palette,
        ToggleSidebar,
        ToggleDetails,
        Tab1,
        Tab2,
        Tab3,
        Tab4,
        Tab5,
        Tab6,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        SlowMode,
        OpenSettings,
        CloseWindow,
        Quit,
        Shortcuts,
        Up,
        Down,
        UpExtend,
        DownExtend,
        Home,
        End,
        Escape,
        ShowMenu,
        ToggleTheme,
    ]
);

const LIST: &str = "TorrentList";

/// DESIGN.md › Keyboard. `secondary` is ⌘ on macOS and Ctrl elsewhere.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-o", AddFile, None),
        KeyBinding::new("secondary-shift-o", AddMagnet, None),
        KeyBinding::new("secondary-s", Resume, None),
        KeyBinding::new("secondary-p", Pause, None),
        KeyBinding::new("space", TogglePause, Some(LIST)),
        KeyBinding::new("secondary-shift-p", PauseAll, None),
        KeyBinding::new("delete", Remove, Some(LIST)),
        KeyBinding::new("shift-delete", RemoveWithFiles, Some(LIST)),
        KeyBinding::new("f2", Rename, Some(LIST)),
        KeyBinding::new("secondary-enter", OpenFolder, None),
        KeyBinding::new("secondary-shift-c", CopyMagnet, None),
        KeyBinding::new("secondary-r", Recheck, None),
        KeyBinding::new("secondary-a", SelectAll, Some(LIST)),
        KeyBinding::new("secondary-f", Filter, None),
        KeyBinding::new("secondary-k", Palette, None),
        KeyBinding::new("secondary-b", ToggleSidebar, None),
        KeyBinding::new("secondary-j", ToggleDetails, None),
        KeyBinding::new("secondary-1", Tab1, None),
        KeyBinding::new("secondary-2", Tab2, None),
        KeyBinding::new("secondary-3", Tab3, None),
        KeyBinding::new("secondary-4", Tab4, None),
        KeyBinding::new("secondary-5", Tab5, None),
        KeyBinding::new("secondary-6", Tab6, None),
        KeyBinding::new("secondary-=", ZoomIn, None),
        KeyBinding::new("secondary-+", ZoomIn, None),
        KeyBinding::new("secondary--", ZoomOut, None),
        KeyBinding::new("secondary-0", ZoomReset, None),
        KeyBinding::new("secondary-alt-l", SlowMode, None),
        KeyBinding::new("secondary-,", OpenSettings, None),
        KeyBinding::new("secondary-w", CloseWindow, None),
        KeyBinding::new("secondary-q", Quit, None),
        KeyBinding::new("?", Shortcuts, Some(LIST)),
        KeyBinding::new("shift-/", Shortcuts, Some(LIST)),
        KeyBinding::new("up", Up, Some(LIST)),
        KeyBinding::new("down", Down, Some(LIST)),
        KeyBinding::new("shift-up", UpExtend, Some(LIST)),
        KeyBinding::new("shift-down", DownExtend, Some(LIST)),
        KeyBinding::new("home", Home, Some(LIST)),
        KeyBinding::new("end", End, Some(LIST)),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("shift-f10", ShowMenu, Some(LIST)),
        KeyBinding::new("menu", ShowMenu, Some(LIST)),
    ]);
}

pub struct Workspace {
    pub client: Arc<Client>,
    pub model: Model,
    pub prefs: UiPrefs,
    pub theme: Theme,
    pub focus: FocusHandle,
    pub scroll: UniformListScrollHandle,
    pub search: Entity<TextInput>,
    pub magnet: Entity<TextInput>,
    pub details: Option<TorrentDetails>,
    pub speed: Vec<SpeedPoint>,
    pub menu: Option<Menu>,
    /// The open submenu: (depth, item index).
    pub sub_open: Option<(usize, usize)>,
    pub dialog: Option<Dialog>,
    pub notice: Option<(SharedString, bool)>,
    pub settings: Option<tt_ipc::EngineSettings>,
    pub window_memory: u64,
    /// The Add torrent dialog's queue (the engine owns it) and its form.
    pub pending: Vec<tt_ipc::PendingAdd>,
    pub add_form: Option<crate::add_dialog::AddForm>,
    pub add_inputs: crate::add_dialog::AddInputs,
    pub file_scroll: UniformListScrollHandle,
    /// Settings › Behaviour › File association.
    pub assoc: Option<tt_ipc::Associations>,
    pub assoc_note: Option<String>,
    pub settings_section: crate::settings::Section,
    pub settings_search: Entity<TextInput>,
    notice_task: Option<Task<()>>,
    _tasks: Vec<Task<()>>,
}

impl Workspace {
    pub fn new(
        client: Arc<Client>,
        snap: Snapshot,
        events: smol::channel::Receiver<Event>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let prefs = snap.prefs.clone();
        let theme = Theme::from_prefs(&prefs);
        let mut model = Model {
            sort: prefs.sort,
            sort_desc: prefs.sort_desc,
            status: snap.status.clone(),
            lists: snap.lists.clone(),
            from_snapshot: snap.from_snapshot,
            ..Default::default()
        };
        model.set_rows(snap.rows);
        // Start with the newest torrent selected so Details has something to show.
        if !model.view.is_empty() {
            model.click(0, false, false);
        }
        let search = cx.new(|cx| TextInput::new("Filter torrents", theme, cx));
        cx.subscribe(
            &search,
            |this: &mut Self, input, e: &InputEvent, cx| match e {
                InputEvent::Changed => {
                    this.model.search = input.read(cx).text.clone();
                    this.model.refresh();
                    cx.notify();
                }
                InputEvent::Submit => {}
                InputEvent::Cancel => {
                    input.update(cx, |i, cx| i.set("", cx));
                }
            },
        )
        .detach();
        let magnet = cx
            .new(|cx| TextInput::new("magnet:?xt=urn:btih:… or https://…/file.torrent", theme, cx));
        cx.subscribe(
            &magnet,
            |this: &mut Self, input, e: &InputEvent, cx| match e {
                InputEvent::Submit => {
                    let text = input.read(cx).text.trim().to_owned();
                    if !text.is_empty() {
                        this.add_link(&text, cx);
                        input.update(cx, |i, cx| i.set("", cx));
                        this.dialog = None;
                    }
                    cx.notify();
                }
                InputEvent::Cancel => {
                    this.dialog = None;
                    cx.notify();
                }
                InputEvent::Changed => {}
            },
        )
        .detach();

        let add_inputs = crate::add_dialog::AddInputs::new(theme, cx);
        let settings_search = cx.new(|cx| TextInput::new("Search settings", theme, cx));
        cx.subscribe(
            &settings_search,
            |_: &mut Self, input, e: &InputEvent, cx| match e {
                InputEvent::Changed | InputEvent::Submit => cx.notify(),
                InputEvent::Cancel => input.update(cx, |i, cx| i.set("", cx)),
            },
        )
        .detach();
        let pending = snap.pending.clone();
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut tasks = Vec::new();
        // Engine events.
        tasks.push(cx.spawn(async move |this, cx| {
            while let Ok(e) = events.recv().await {
                let exiting = matches!(e, Event::Exiting | Event::CloseWindow);
                if this.update(cx, |this, cx| this.on_event(e, cx)).is_err() || exiting {
                    break;
                }
            }
            let _ = cx.update(|cx| cx.quit());
        }));
        // Details for the selected torrent, and the window's own memory, every second.
        tasks.push(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update(cx, |this, cx| this.refresh_details(cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        let mut ws = Self {
            client,
            model,
            prefs,
            theme,
            focus,
            scroll: UniformListScrollHandle::new(),
            search,
            magnet,
            details: None,
            speed: Vec::new(),
            menu: None,
            sub_open: None,
            dialog: None,
            notice: None,
            settings: None,
            window_memory: 0,
            pending: Vec::new(),
            add_form: None,
            add_inputs,
            file_scroll: UniformListScrollHandle::new(),
            assoc: None,
            assoc_note: None,
            settings_section: crate::settings::Section::default(),
            settings_search,
            notice_task: None,
            _tasks: tasks,
        };
        ws.refresh_details(cx);
        // Files opened before the window connected wait in the engine.
        ws.set_pending(pending, cx);
        ws.call_then(Request::PendingAdds, cx, |ws, r, cx| {
            if let Response::PendingAdds { items } = r {
                ws.set_pending(items, cx);
            }
        });
        // The tray (add a file, settings) and tt-bench (screenshots) open
        // something at start.
        match std::env::var("TT_UI_SHOW").as_deref() {
            Ok(c @ ("add-file" | "add-magnet")) => ws.run_command(c, cx),
            Ok("settings") => ws.open_settings(cx),
            Ok("shortcuts") => ws.dialog = Some(Dialog::Shortcuts),
            Ok("menu") => {
                ws.menu = Some(Menu::Torrent {
                    at: gpui::point(gpui::px(520.), gpui::px(250.)),
                })
            }
            Ok("status-menu") => {
                let h = window.viewport_size().height;
                ws.menu = Some(Menu::StatusBar {
                    at: gpui::point(gpui::px(900.), h - gpui::px(30.)),
                })
            }
            Ok("bar") => {
                ws.menu = Some(Menu::Bar {
                    index: 3,
                    at: gpui::point(gpui::px(0.), gpui::px(0.)),
                })
            }
            Ok(tab) if tab.starts_with("tab:") => {
                if let Some(t) = DetailsTab::ALL
                    .iter()
                    .find(|t| format!("tab:{t:?}").eq_ignore_ascii_case(tab))
                {
                    ws.prefs.details_tab = *t;
                }
            }
            _ => {}
        }
        if let Ok(ix) = std::env::var("TT_UI_SELECT")
            && let Ok(ix) = ix.parse::<usize>()
        {
            ws.model.click(ix, false, false);
        }
        window.on_window_should_close(cx, |_, cx| {
            // Closing the window keeps the engine running in the tray.
            let _ = cx;
            true
        });
        ws
    }

    fn on_event(&mut self, e: Event, cx: &mut Context<Self>) {
        match e {
            Event::Rows { upsert, removed } => {
                self.model.from_snapshot = false;
                self.model.apply(upsert, &removed);
            }
            Event::Status(s) => self.model.status = s,
            Event::Lists(l) => self.model.lists = l,
            Event::Notice { text } => self.notify(text, false, cx),
            Event::Activate => cx.activate(true),
            Event::CloseWindow | Event::Exiting => cx.quit(),
            Event::Command { name } => self.run_command(&name, cx),
            Event::PendingAdds { items } => self.set_pending(items, cx),
        }
        cx.notify();
    }

    /// Ask the engine in the background; errors become a notice.
    pub fn call(&mut self, req: Request, cx: &mut Context<Self>) {
        self.call_then(req, cx, |_, _, _| {});
    }

    pub fn call_then(
        &mut self,
        req: Request,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, Response, &mut Context<Self>) + 'static,
    ) {
        let client = self.client.clone();
        let task = cx.background_spawn(async move { client.call(req) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            let _ = this.update(cx, |this, cx| match r {
                Ok(resp) => then(this, resp, cx),
                Err(e) => this.notify(e, true, cx),
            });
        })
        .detach();
    }

    pub fn notify(&mut self, text: impl Into<SharedString>, error: bool, cx: &mut Context<Self>) {
        self.notice = Some((text.into(), error));
        self.notice_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = this.update(cx, |this, cx| {
                this.notice = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn refresh_details(&mut self, cx: &mut Context<Self>) {
        self.window_memory = tt_platform::process_memory(None)
            .map(|m| m.private_working_set)
            .unwrap_or(0);
        if !self.prefs.details {
            return;
        }
        if self.prefs.details_tab == DetailsTab::Speed {
            self.call_then(Request::SpeedHistory, cx, |this, r, cx| {
                if let Response::SpeedHistory { points } = r {
                    this.speed = points;
                    cx.notify();
                }
            });
        }
        let Some(id) = self.model.focused().map(|r| r.id) else {
            if self.details.take().is_some() {
                cx.notify();
            }
            return;
        };
        if self.model.from_snapshot {
            return;
        }
        let client = self.client.clone();
        let task = cx.background_spawn(async move { client.call(Request::Details { id }) });
        cx.spawn(async move |this, cx| {
            let r = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(Response::Details(d)) = r {
                    this.details = Some(*d);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn save_prefs(&mut self, cx: &mut Context<Self>) {
        self.prefs.sort = self.model.sort;
        self.prefs.sort_desc = self.model.sort_desc;
        self.theme = Theme::from_prefs(&self.prefs);
        let t = self.theme;
        self.search.update(cx, |i, _| i.theme = t);
        self.magnet.update(cx, |i, _| i.theme = t);
        self.add_inputs.set_theme(t, cx);
        self.settings_search.update(cx, |i, _| i.theme = t);
        self.call(
            Request::SetUiPrefs {
                prefs: self.prefs.clone(),
            },
            cx,
        );
        cx.notify();
    }

    pub fn select_row(&mut self, ix: usize, ctrl: bool, shift: bool, cx: &mut Context<Self>) {
        self.model.click(ix, ctrl, shift);
        self.details = None;
        self.refresh_details(cx);
        cx.notify();
    }

    /// The link dialog: one link per line. Each opens the Add torrent dialog
    /// (or is added straight away when that dialog is off).
    pub fn add_link(&mut self, text: &str, cx: &mut Context<Self>) {
        let sources: Vec<AddSource> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|line| {
                if line.starts_with("magnet:") {
                    AddSource::Magnet { uri: line.into() }
                } else if line.starts_with("http") {
                    AddSource::Url { url: line.into() }
                } else if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) {
                    // A bare info hash, as qBittorrent's "Download from URLs" accepts.
                    AddSource::Magnet {
                        uri: format!("magnet:?xt=urn:btih:{line}"),
                    }
                } else {
                    AddSource::File { path: line.into() }
                }
            })
            .collect();
        self.open_sources(sources, cx);
    }

    pub fn open_sources(&mut self, sources: Vec<AddSource>, cx: &mut Context<Self>) {
        if !sources.is_empty() {
            self.call(Request::Open { sources }, cx);
        }
    }

    pub fn add_files(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Add torrent files".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                let _ = this.update(cx, |this, cx| {
                    let sources = paths
                        .into_iter()
                        .map(|p| AddSource::File {
                            path: p.to_string_lossy().into_owned(),
                        })
                        .collect();
                    this.open_sources(sources, cx);
                });
            }
        })
        .detach();
    }

    /// A command from the tray.
    pub fn run_command(&mut self, name: &str, cx: &mut Context<Self>) {
        match name {
            "add-file" => self.add_files(cx),
            "add-magnet" => {
                self.dialog = Some(Dialog::Magnet);
                cx.notify();
            }
            "settings" => self.open_settings(cx),
            _ => {}
        }
    }

    pub fn selected(&self) -> Vec<TorrentId> {
        self.model.selected_ids()
    }

    pub fn pause_sel(&mut self, cx: &mut Context<Self>) {
        let ids = self.selected();
        if !ids.is_empty() {
            self.call(Request::Pause { ids }, cx);
        }
    }

    pub fn resume_sel(&mut self, cx: &mut Context<Self>) {
        let ids = self.selected();
        if !ids.is_empty() {
            self.call(Request::Resume { ids }, cx);
        }
    }

    pub fn ask_remove(&mut self, with_files: bool, cx: &mut Context<Self>) {
        let ids = self.selected();
        if ids.is_empty() {
            return;
        }
        self.menu = None;
        self.dialog = Some(Dialog::Remove {
            ids,
            delete_files: with_files,
        });
        cx.notify();
    }

    pub fn copy_magnet(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.model.focused().map(|r| r.id) else {
            return;
        };
        self.call_then(Request::MagnetLink { id }, cx, |this, r, cx| {
            if let Response::Text { text } = r {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                this.notify("Magnet link copied", false, cx);
            }
        });
    }

    pub fn copy_text(&mut self, text: String, what: &str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.notify(format!("{what} copied"), false, cx);
    }

    pub fn set_tab(&mut self, tab: DetailsTab, cx: &mut Context<Self>) {
        self.prefs.details_tab = tab;
        self.prefs.details = true;
        self.save_prefs(cx);
        self.refresh_details(cx);
    }

    pub fn open_menu_at(&mut self, menu: Menu, cx: &mut Context<Self>) {
        self.menu = Some(menu);
        cx.notify();
    }

    pub fn set_side_filter(&mut self, f: SideFilter, cx: &mut Context<Self>) {
        self.model.side_filter = f;
        self.model.refresh();
        cx.notify();
    }

    fn scroll_to_cursor(&mut self) {
        if let Some(i) = self.model.cursor.and_then(|c| self.model.index_of(c)) {
            self.scroll.scroll_to_item(i, gpui::ScrollStrategy::Top);
        }
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        self.assoc_note = None;
        self.refresh_associations(cx);
        self.call_then(Request::GetSettings, cx, |this, r, cx| {
            if let Response::Settings(s) = r {
                this.settings = Some(*s);
                this.dialog = Some(Dialog::Settings);
                cx.notify();
            }
        });
    }

    pub fn refresh_associations(&mut self, cx: &mut Context<Self>) {
        self.call_then(Request::Associations, cx, |this, r, cx| {
            if let Response::Associations(a) = r {
                this.assoc = Some(a);
                cx.notify();
            }
        });
    }

    /// Settings › File association › Make default.
    pub fn make_default(&mut self, cx: &mut Context<Self>) {
        self.call_then(Request::MakeDefault, cx, |this, r, cx| {
            if let Response::MadeDefault { status, note } = r {
                this.assoc = Some(status);
                this.assoc_note = note.or_else(|| {
                    (status.torrent_files && status.magnet_links)
                        .then(|| "tTorrent now opens .torrent files and magnet links.".into())
                });
                cx.notify();
            }
        });
    }

    pub fn set_engine_settings(&mut self, s: tt_ipc::EngineSettings, cx: &mut Context<Self>) {
        self.settings = Some(s.clone());
        self.call(
            Request::SetSettings {
                settings: Box::new(s),
            },
            cx,
        );
        cx.notify();
    }

    pub fn context_menu_at(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.menu = Some(Menu::Torrent { at: pos });
        cx.notify();
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        let sheet_open = self.dialog.is_some();
        div()
            .id("workspace")
            .key_context(LIST)
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(t.c.bg)
            .text_color(t.c.ink)
            .font_family(FONT_UI)
            .text_size(t.fpx(t.m.ui_font))
            .on_action(cx.listener(|this, _: &AddFile, _, cx| this.add_files(cx)))
            .on_action(cx.listener(|this, _: &AddMagnet, window, cx| {
                this.dialog = Some(Dialog::Magnet);
                let f = this.magnet.focus_handle(cx);
                window.focus(&f);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Resume, _, cx| this.resume_sel(cx)))
            .on_action(cx.listener(|this, _: &Pause, _, cx| this.pause_sel(cx)))
            .on_action(cx.listener(|this, _: &TogglePause, _, cx| {
                if this.model.selection_paused() {
                    this.resume_sel(cx)
                } else {
                    this.pause_sel(cx)
                }
            }))
            .on_action(cx.listener(|this, _: &PauseAll, _, cx| {
                let all_paused = this.model.rows.values().all(|r| r.state.is_paused());
                this.call(
                    if all_paused {
                        Request::ResumeAll
                    } else {
                        Request::PauseAll
                    },
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &Remove, _, cx| this.ask_remove(false, cx)))
            .on_action(cx.listener(|this, _: &RemoveWithFiles, _, cx| this.ask_remove(true, cx)))
            .on_action(cx.listener(|this, _: &Rename, window, cx| {
                if let Some(r) = this.model.focused() {
                    let (id, name) = (r.id, r.name.clone());
                    this.magnet.update(cx, |i, cx| i.set(&name, cx));
                    this.dialog = Some(Dialog::Rename { id });
                    let f = this.magnet.focus_handle(cx);
                    window.focus(&f);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &OpenFolder, _, cx| {
                if let Some(id) = this.model.focused().map(|r| r.id) {
                    this.call(Request::OpenFolder { id }, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CopyMagnet, _, cx| this.copy_magnet(cx)))
            .on_action(cx.listener(|this, _: &Recheck, _, cx| {
                let ids = this.selected();
                this.call(Request::Recheck { ids }, cx);
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| {
                this.model.select_all();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Filter, window, cx| {
                let f = this.search.focus_handle(cx);
                window.focus(&f);
            }))
            .on_action(cx.listener(|this, _: &Palette, window, cx| {
                let f = this.search.focus_handle(cx);
                window.focus(&f);
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.prefs.sidebar = !this.prefs.sidebar;
                this.save_prefs(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleDetails, _, cx| {
                this.prefs.details = !this.prefs.details;
                this.save_prefs(cx);
                this.refresh_details(cx);
            }))
            .on_action(cx.listener(|this, _: &Tab1, _, cx| this.set_tab(DetailsTab::ALL[0], cx)))
            .on_action(cx.listener(|this, _: &Tab2, _, cx| this.set_tab(DetailsTab::ALL[1], cx)))
            .on_action(cx.listener(|this, _: &Tab3, _, cx| this.set_tab(DetailsTab::ALL[2], cx)))
            .on_action(cx.listener(|this, _: &Tab4, _, cx| this.set_tab(DetailsTab::ALL[3], cx)))
            .on_action(cx.listener(|this, _: &Tab5, _, cx| this.set_tab(DetailsTab::ALL[4], cx)))
            .on_action(cx.listener(|this, _: &Tab6, _, cx| this.set_tab(DetailsTab::ALL[5], cx)))
            .on_action(cx.listener(|this, _: &ZoomIn, _, cx| {
                this.prefs.scale = tt_theme::step_scale(this.prefs.scale, 1);
                this.save_prefs(cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomOut, _, cx| {
                this.prefs.scale = tt_theme::step_scale(this.prefs.scale, -1);
                this.save_prefs(cx);
            }))
            .on_action(cx.listener(|this, _: &ZoomReset, _, cx| {
                this.prefs.scale = 1.0;
                this.save_prefs(cx);
            }))
            .on_action(cx.listener(|this, _: &SlowMode, _, cx| {
                let on = !this.model.status.slow_mode;
                this.call(Request::SetSlowMode { on }, cx);
                this.notify(if on { "Slow mode on" } else { "Slow mode off" }, false, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| this.open_settings(cx)))
            .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                let client = this.client.clone();
                cx.background_spawn(async move { client.call(Request::Quit) })
                    .detach();
                cx.quit();
            }))
            .on_action(cx.listener(|this, _: &Shortcuts, _, cx| {
                this.dialog = Some(Dialog::Shortcuts);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Up, _, cx| {
                this.model.move_cursor(-1, false);
                this.scroll_to_cursor();
                this.refresh_details(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Down, _, cx| {
                this.model.move_cursor(1, false);
                this.scroll_to_cursor();
                this.refresh_details(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &UpExtend, _, cx| {
                this.model.move_cursor(-1, true);
                this.scroll_to_cursor();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DownExtend, _, cx| {
                this.model.move_cursor(1, true);
                this.scroll_to_cursor();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Home, _, cx| {
                this.model.move_cursor(isize::MIN / 2, false);
                this.scroll_to_cursor();
                this.refresh_details(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &End, _, cx| {
                this.model.move_cursor(isize::MAX / 2, false);
                this.scroll_to_cursor();
                this.refresh_details(cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Escape, window, cx| {
                if this.menu.is_none() && this.add_form.is_some() {
                    this.cancel_add(cx);
                } else if this.menu.take().is_none() && this.dialog.take().is_none() {
                    this.search.update(cx, |i, cx| i.set("", cx));
                }
                window.focus(&this.focus);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ShowMenu, window, cx| {
                let pos = window.mouse_position();
                this.context_menu_at(pos, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleTheme, _, cx| {
                this.prefs.mode = if this.prefs.mode == "dark" {
                    "light"
                } else {
                    "dark"
                }
                .into();
                this.save_prefs(cx);
            }))
            .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| {
                let sources = paths
                    .paths()
                    .iter()
                    .map(|p| AddSource::File {
                        path: p.to_string_lossy().into_owned(),
                    })
                    .collect();
                this.open_sources(sources, cx);
            }))
            .child(crate::parts::title_bar(self, window, cx))
            .child(crate::parts::toolbar(self, window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.prefs.sidebar, |d| {
                        d.child(crate::parts::sidebar(self, window, cx))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(crate::parts::transfer_list(self, window, cx))
                            .when(self.prefs.details, |d| {
                                d.child(crate::details::details(self, window, cx))
                            }),
                    ),
            )
            .child(crate::parts::status_bar(self, window, cx))
            .when_some(self.menu.clone(), |d, m| {
                d.child(crate::overlay::render_menu(self, m, window, cx))
            })
            .when(sheet_open, |d| {
                d.child(crate::overlay::render_dialog(self, window, cx))
            })
            .when(self.add_form.is_some(), |d| {
                d.child(crate::add_dialog::add_sheet(self, window, cx))
            })
            .when_some(self.notice.clone(), |d, (text, err)| {
                d.child(crate::overlay::toast(&t, text, err))
            })
    }
}
