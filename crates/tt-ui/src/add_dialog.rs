//! The Add torrent dialog, qBittorrent's "Add new torrent": shown for every
//! `.torrent` file and magnet link the system, a second launch, Ctrl+O, the
//! link dialog or a drop opens, while Settings › Downloads › "Display torrent
//! content and some options" is on (the default, as in qBittorrent).
//!
//! The engine keeps the queue (`PendingAdd`) so the dialog survives the
//! window being closed; this module keeps only the form for the first item.

use crate::app::Workspace;
use crate::overlay::{Pick, button, dropdown, field, stop_label};
use crate::text_input::{InputEvent, TextInput};
use crate::theme::{FONT_NUM, Theme};
use crate::widgets::icon;
use gpui::{
    AnyElement, Context, Entity, FontWeight, IntoElement, ParentElement, PathPromptOptions,
    SharedString, Styled, Window, div, prelude::*, uniform_list,
};
use std::collections::{BTreeMap, HashSet};
use tt_ipc::{
    AddOptions, ContentLayout, PendingAdd, PendingState, PreviewFile, Request, Response,
    TorrentPreview, fmt,
};

/// The form for the torrent at the head of the queue.
pub struct AddForm {
    pub token: u64,
    pub opts: AddOptions,
    /// One per file, once the metadata is known.
    pub selected: Vec<bool>,
    pub collapsed: HashSet<String>,
    pub never_show: bool,
    pub free: Option<u64>,
    /// The file tree as drawn, rebuilt after every change.
    pub rows: Vec<TreeRow>,
    /// Lower-case text of the Files filter.
    pub filter: String,
    pub info_open: bool,
    pub later_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    All,
    Some,
    None,
}

/// One line of the Files tree.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRow {
    pub depth: usize,
    pub name: String,
    /// Folder key (its path) or the file's path.
    pub key: String,
    /// The file's index in the torrent; None for a folder.
    pub file: Option<usize>,
    /// The files under a folder (a file lists itself).
    pub members: Vec<usize>,
    pub size: u64,
    pub check: Check,
    pub open: bool,
}

#[derive(Default)]
struct Node {
    dirs: BTreeMap<String, Node>,
    files: Vec<(String, usize)>,
}

/// The tree qBittorrent draws: folders first, then files, each by name.
/// `root` is the torrent's own folder when the layout creates one.
pub fn tree_rows(
    files: &[PreviewFile],
    selected: &[bool],
    collapsed: &HashSet<String>,
    root: Option<&str>,
    filter: &str,
) -> Vec<TreeRow> {
    let filter = filter.trim().to_lowercase();
    // While filtering, every folder with a match shows open.
    let none = HashSet::new();
    let collapsed = if filter.is_empty() { collapsed } else { &none };
    let mut top = Node::default();
    for (i, f) in files.iter().enumerate() {
        if !filter.is_empty() && !f.path.to_lowercase().contains(&filter) {
            continue;
        }
        let mut parts: Vec<&str> = f.path.split('/').filter(|p| !p.is_empty()).collect();
        let name = parts.pop().unwrap_or("").to_owned();
        let mut n = &mut top;
        if let Some(r) = root {
            n = n.dirs.entry(r.to_owned()).or_default();
        }
        for p in parts {
            n = n.dirs.entry(p.to_owned()).or_default();
        }
        n.files.push((name, i));
    }
    let mut out = Vec::new();
    walk(&top, "", 0, files, selected, collapsed, &mut out);
    out
}

fn walk(
    n: &Node,
    prefix: &str,
    depth: usize,
    files: &[PreviewFile],
    selected: &[bool],
    collapsed: &HashSet<String>,
    out: &mut Vec<TreeRow>,
) -> Vec<usize> {
    let mut all = Vec::new();
    for (name, child) in &n.dirs {
        let key = format!("{prefix}{name}/");
        let at = out.len();
        let open = !collapsed.contains(&key);
        out.push(TreeRow {
            depth,
            name: name.clone(),
            key: key.clone(),
            file: None,
            members: Vec::new(),
            size: 0,
            check: Check::None,
            open,
        });
        let mut inner = Vec::new();
        let members = walk(
            child,
            &key,
            depth + 1,
            files,
            selected,
            collapsed,
            &mut inner,
        );
        if open {
            out.extend(inner);
        }
        let size = members.iter().map(|&i| files[i].size).sum();
        let on = members
            .iter()
            .filter(|&&i| selected.get(i).copied().unwrap_or(true))
            .count();
        let row = &mut out[at];
        row.size = size;
        row.check = check_of(on, members.len());
        all.extend_from_slice(&members);
        row.members = members;
    }
    let mut fs: Vec<&(String, usize)> = n.files.iter().collect();
    fs.sort_by(|a, b| tt_ipc::columns::natural_cmp(&a.0, &b.0));
    for (name, i) in fs {
        out.push(TreeRow {
            depth,
            name: name.clone(),
            key: files[*i].path.clone(),
            file: Some(*i),
            members: vec![*i],
            size: files[*i].size,
            check: if selected.get(*i).copied().unwrap_or(true) {
                Check::All
            } else {
                Check::None
            },
            open: false,
        });
        all.push(*i);
    }
    all
}

fn check_of(on: usize, total: usize) -> Check {
    if total > 0 && on == total {
        Check::All
    } else if on == 0 {
        Check::None
    } else {
        Check::Some
    }
}

/// The torrent's own folder as the dialog shows it, matching the engine's
/// `content_folder`.
fn root_folder(p: &TorrentPreview, layout: ContentLayout) -> Option<String> {
    match layout {
        ContentLayout::Original if p.files.len() > 1 => Some(p.name.clone()),
        ContentLayout::Subfolder => Some(if p.files.len() > 1 {
            p.name.clone()
        } else {
            std::path::Path::new(&p.name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.name.clone())
        }),
        _ => None,
    }
}

fn preview(p: &PendingAdd) -> Option<&TorrentPreview> {
    match &p.state {
        PendingState::Ready { preview } => Some(preview),
        _ => None,
    }
}

impl AddForm {
    fn new(p: &PendingAdd) -> Self {
        Self {
            token: p.token,
            opts: p.defaults.clone(),
            selected: Vec::new(),
            collapsed: HashSet::new(),
            never_show: false,
            free: None,
            rows: Vec::new(),
            filter: String::new(),
            info_open: false,
            later_open: false,
        }
    }

    pub fn rebuild(&mut self, p: Option<&TorrentPreview>) {
        let Some(p) = p else {
            self.rows.clear();
            return;
        };
        if self.selected.len() != p.files.len() {
            self.selected = vec![true; p.files.len()];
        }
        let layout = self.opts.content_layout.unwrap_or_default();
        self.rows = tree_rows(
            &p.files,
            &self.selected,
            &self.collapsed,
            root_folder(p, layout).as_deref(),
            &self.filter,
        );
    }

    fn selected_size(&self, p: &TorrentPreview) -> u64 {
        p.files
            .iter()
            .zip(&self.selected)
            .filter(|(_, on)| **on)
            .map(|(f, _)| f.size)
            .sum()
    }
}

/// The text fields of the dialog.
pub struct AddInputs {
    pub save: Entity<TextInput>,
    pub tags: Entity<TextInput>,
    pub filter: Entity<TextInput>,
}

impl AddInputs {
    pub fn new(theme: Theme, cx: &mut Context<Workspace>) -> Self {
        let save = cx.new(|cx| TextInput::new("Save folder", theme, cx));
        let tags = cx.new(|cx| TextInput::new("Tags, separated by commas", theme, cx));
        cx.subscribe(
            &save,
            |ws: &mut Workspace, input, e: &InputEvent, cx| match e {
                InputEvent::Changed => {
                    let path = input.read(cx).text.clone();
                    ws.refresh_free(path, cx);
                }
                InputEvent::Submit => ws.confirm_add(cx),
                InputEvent::Cancel => ws.cancel_add(cx),
            },
        )
        .detach();
        cx.subscribe(&tags, |ws: &mut Workspace, _, e: &InputEvent, cx| match e {
            InputEvent::Submit => ws.confirm_add(cx),
            InputEvent::Cancel => ws.cancel_add(cx),
            InputEvent::Changed => {}
        })
        .detach();
        let filter = cx.new(|cx| TextInput::new("Filter files", theme, cx));
        cx.subscribe(
            &filter,
            |ws: &mut Workspace, input, e: &InputEvent, cx| match e {
                InputEvent::Changed => {
                    let text = input.read(cx).text.clone();
                    ws.edit_form(cx, |f| f.filter = text);
                }
                InputEvent::Submit => {}
                // Esc in the filter clears it first.
                InputEvent::Cancel => input.update(cx, |i, cx| i.set("", cx)),
            },
        )
        .detach();
        Self { save, tags, filter }
    }

    pub fn set_theme(&self, t: Theme, cx: &mut Context<Workspace>) {
        self.save.update(cx, |i, _| i.theme = t);
        self.tags.update(cx, |i, _| i.theme = t);
        self.filter.update(cx, |i, _| i.theme = t);
    }
}

impl Workspace {
    /// The engine's queue changed (or the window just started).
    pub fn set_pending(&mut self, items: Vec<PendingAdd>, cx: &mut Context<Self>) {
        self.pending = items;
        self.sync_add_form(cx);
        cx.notify();
    }

    fn sync_add_form(&mut self, cx: &mut Context<Self>) {
        let Some(head) = self.pending.first().cloned() else {
            self.add_form = None;
            return;
        };
        if self.add_form.as_ref().map(|f| f.token) != Some(head.token) {
            let form = AddForm::new(&head);
            let save = form.opts.save_path.clone().unwrap_or_default();
            self.add_inputs.save.update(cx, |i, cx| i.set(&save, cx));
            self.add_inputs.tags.update(cx, |i, cx| i.set("", cx));
            self.add_inputs.filter.update(cx, |i, cx| i.set("", cx));
            self.add_form = Some(form);
            self.file_scroll = gpui::UniformListScrollHandle::new();
            self.menu = None;
        }
        if let Some(f) = self.add_form.as_mut() {
            f.rebuild(preview(&head));
        }
    }

    pub fn refresh_free(&mut self, path: String, cx: &mut Context<Self>) {
        let token = self.add_form.as_ref().map(|f| f.token);
        self.call_then(Request::FreeSpace { path }, cx, move |ws, r, cx| {
            if let (Response::FreeSpace { bytes }, Some(f)) = (r, ws.add_form.as_mut())
                && Some(f.token) == token
            {
                f.free = bytes;
                cx.notify();
            }
        });
    }

    /// A category from the dropdown; one with its own folder moves the save
    /// path there (qBittorrent's automatic mode).
    pub(crate) fn pick_add_category(&mut self, name: String, path: String, cx: &mut Context<Self>) {
        if !path.is_empty() {
            self.add_inputs.save.update(cx, |i, cx| i.set(&path, cx));
        }
        self.edit_form(cx, |f| {
            f.opts.category = Some(name).filter(|n| !n.is_empty())
        });
    }

    pub(crate) fn edit_form(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut AddForm)) {
        let head = self.pending.first().cloned();
        if let Some(form) = self.add_form.as_mut() {
            f(form);
            form.rebuild(head.as_ref().and_then(preview));
        }
        cx.notify();
    }

    fn take_head(&mut self, cx: &mut Context<Self>) -> Option<u64> {
        let token = self.add_form.as_ref()?.token;
        self.pending.retain(|p| p.token != token);
        self.add_form = None;
        self.sync_add_form(cx);
        cx.notify();
        Some(token)
    }

    pub fn confirm_add(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.add_form.as_ref() else {
            return;
        };
        if !form.selected.is_empty() && !form.selected.iter().any(|s| *s) {
            self.notify("Choose at least one file", true, cx);
            return;
        }
        let mut options = form.opts.clone();
        let save = self.add_inputs.save.read(cx).text.trim().to_owned();
        options.save_path = Some(save).filter(|s| !s.is_empty());
        options.tags = self
            .add_inputs
            .tags
            .read(cx)
            .text
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect();
        options.only_files = (form.selected.iter().any(|s| !*s)).then(|| {
            form.selected
                .iter()
                .enumerate()
                .filter(|(_, on)| **on)
                .map(|(i, _)| i as u32)
                .collect()
        });
        let never_show_again = form.never_show;
        let Some(token) = self.take_head(cx) else {
            return;
        };
        self.call(
            Request::ConfirmAdd {
                token,
                options,
                never_show_again,
            },
            cx,
        );
    }

    pub fn cancel_add(&mut self, cx: &mut Context<Self>) {
        if let Some(token) = self.take_head(cx) {
            self.call(Request::CancelAdd { token }, cx);
        }
    }

    fn browse_save(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Save at".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                let _ = this.update(cx, |ws, cx| {
                    let p = p.to_string_lossy().into_owned();
                    ws.add_inputs.save.update(cx, |i, cx| i.set(&p, cx));
                });
            }
        })
        .detach();
    }

    fn toggle_row(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.edit_form(cx, |f| {
            let Some(row) = f.rows.get(ix) else { return };
            let on = row.check != Check::All;
            for &i in &row.members {
                if let Some(s) = f.selected.get_mut(i) {
                    *s = on;
                }
            }
        });
    }

    fn toggle_open(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.edit_form(cx, |f| {
            let Some(row) = f.rows.get(ix) else { return };
            let key = row.key.clone();
            if !f.collapsed.remove(&key) {
                f.collapsed.insert(key);
            }
        });
    }
}

fn checkbox_mark(t: &Theme, check: Check, enabled: bool) -> gpui::Div {
    let on = check != Check::None;
    let fill = if enabled { t.c.accent } else { t.c.muted };
    div()
        .size(t.px(16.))
        .flex_none()
        .rounded(t.px(4.))
        .border_1()
        .border_color(if on { fill } else { t.c.rule })
        .when(on, |d| d.bg(fill))
        .when(!on, |d| d.bg(t.c.raise))
        .flex()
        .items_center()
        .justify_center()
        .map(|d| match check {
            Check::All => d.child(icon("check", 12., t.c.accent_ink, t)),
            Check::Some => d.child(icon("minus", 12., t.c.accent_ink, t)),
            Check::None => d,
        })
}

/// A labelled checkbox; `hint` is shown under the label.
fn checkbox(
    t: &Theme,
    id: &'static str,
    label: &'static str,
    on: bool,
    hint: Option<&'static str>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_start()
        .gap(t.px(10.))
        .cursor_pointer()
        .child(div().pt(t.px(1.)).child(checkbox_mark(
            t,
            if on { Check::All } else { Check::None },
            true,
        )))
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_color(t.c.ink).child(label))
                .when_some(hint, |d, h| {
                    d.child(div().text_size(t.fpx(12.)).text_color(t.c.muted).child(h))
                }),
        )
}

/// A label column and a control, the left side's rows.
fn labelled(t: &Theme, label: &'static str, control: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(t.px(12.))
        .child(
            div()
                .w(t.px(84.))
                .flex_none()
                .text_color(t.c.ink_2)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(control))
}

fn heading(t: &Theme, text: &'static str) -> gpui::Div {
    div()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(t.c.ink)
        .child(text)
}

/// `1791` → `1,791`.
fn count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn info_pair(
    t: &Theme,
    label: &'static str,
    value: impl Into<SharedString>,
    mono: bool,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(t.px(2.))
        .child(div().text_color(t.c.muted).child(label))
        .child(
            div()
                .text_color(t.c.ink)
                .when(mono, |d| d.font_family(FONT_NUM).text_size(t.fpx(12.5)))
                .child(value.into()),
        )
}

fn disclosure(
    t: &Theme,
    id: &'static str,
    label: impl Into<SharedString>,
    open: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(t.px(6.))
        .cursor_pointer()
        .text_color(t.c.ink_2)
        .hover(|s| s.text_color(t.c.ink))
        .child(icon(
            if open {
                "chevron-down"
            } else {
                "chevron-right"
            },
            13.,
            t.c.muted,
            t,
        ))
        .child(label.into())
}

pub fn add_sheet(
    ws: &mut Workspace,
    _window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let (Some(head), Some(form)) = (ws.pending.first().cloned(), ws.add_form.as_ref()) else {
        return div().into_any_element();
    };
    let waiting = ws.pending.len();
    let p = preview(&head).cloned();
    let opts = form.opts.clone();
    let layout = opts.content_layout.unwrap_or_default();
    let stop = opts.stop_condition.unwrap_or_default();
    let start = !opts.paused;
    let never_show = form.never_show;
    let info_open = form.info_open;
    let later_open = form.later_open;
    let rows_len = form.rows.len();
    let selected_n = form.selected.iter().filter(|s| **s).count();
    let all_check = check_of(selected_n, form.selected.len());
    let none_chosen = !form.selected.is_empty() && selected_n == 0;
    let need = p.as_ref().map(|p| form.selected_size(p));
    let free = form.free;
    let category = opts.category.clone().unwrap_or_default();

    // ── Header: what this is, and a way out ──
    let name: SharedString = p
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| head.label.clone())
        .into();
    let summary: AnyElement = match (&head.state, &p) {
        (_, Some(p)) => div()
            .flex()
            .gap(t.px(28.))
            .pt(t.px(4.))
            .child(info_pair(&t, "Size", fmt::size(p.total_size), true))
            .child(info_pair(&t, "Files", count(p.files.len() as u64), true))
            .child(info_pair(
                &t,
                "Pieces",
                format!(
                    "{} of {}",
                    count(u64::from(p.pieces)),
                    fmt::size(p.piece_size)
                ),
                true,
            ))
            .when_some(p.created_on, |d, c| {
                d.child(info_pair(&t, "Created", fmt::date_long(c), false))
            })
            .when(p.private, |d| {
                d.child(info_pair(&t, "Tracker", "Private", false))
            })
            .into_any_element(),
        (PendingState::Failed { error }, None) => div()
            .text_color(t.c.bad)
            .child(format!("Couldn't read this torrent: {error}"))
            .into_any_element(),
        _ => div()
            .text_color(t.c.muted)
            .child("Retrieving metadata from peers. You can add it now and choose files later.")
            .into_any_element(),
    };
    let header = div()
        .flex()
        .items_start()
        .gap(t.px(16.))
        .pl(t.px(24.))
        .pr(t.px(16.))
        .pt(t.px(18.))
        .pb(t.px(14.))
        .border_b_1()
        .border_color(t.c.rule)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(t.px(6.))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(t.px(10.))
                        .child(
                            div()
                                .font_family(crate::theme::FONT_HEAD)
                                .font_weight(FontWeight::BOLD)
                                .text_size(t.fpx(17.))
                                .child("Add torrent"),
                        )
                        .when(waiting > 1, |d| {
                            d.child(
                                div()
                                    .text_color(t.c.muted)
                                    .child(format!("1 of {waiting} waiting")),
                            )
                        }),
                )
                .child(
                    div()
                        .text_size(t.fpx(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(name),
                )
                .child(summary),
        )
        .child(
            div()
                .id("add-close")
                .size(t.px(32.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.px(8.))
                .cursor_pointer()
                .hover(|s| s.bg(t.c.hover))
                .on_click(cx.listener(|ws, _, _, cx| ws.cancel_add(cx)))
                .child(icon("x", 16., t.c.muted, &t)),
        );

    // ── Left: where and how ──
    let space_line: AnyElement = match (free, need) {
        (Some(f), Some(n)) if n > f => div()
            .text_color(t.c.bad)
            .child(format!(
                "Not enough space: {} needed, {} free",
                fmt::size(n),
                fmt::size(f)
            ))
            .into_any_element(),
        (Some(f), Some(n)) => div()
            .text_color(t.c.muted)
            .child(format!("{} free, {} needed", fmt::size(f), fmt::size(n)))
            .into_any_element(),
        (Some(f), None) => div()
            .text_color(t.c.muted)
            .child(format!("{} free", fmt::size(f)))
            .into_any_element(),
        _ => div().into_any_element(),
    };
    let left = div()
        .id("add-left")
        .w(t.px(340.))
        .flex_none()
        .flex()
        .flex_col()
        .gap(t.px(20.))
        .overflow_y_scroll()
        .px(t.px(24.))
        .py(t.px(18.))
        .border_r_1()
        .border_color(t.c.rule)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(t.px(8.))
                .child(heading(&t, "Save at"))
                .child(
                    div()
                        .flex()
                        .gap(t.px(8.))
                        .child(field(&t, ws.add_inputs.save.clone()).flex_1().min_w_0())
                        .child(
                            button(&t, "add-browse", "Browse…", 0)
                                .h(t.px(34.))
                                .on_click(cx.listener(|ws, _, _, cx| ws.browse_save(cx))),
                        ),
                )
                .child(space_line),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(t.px(10.))
                .child(labelled(
                    &t,
                    "Category",
                    dropdown(
                        &t,
                        "add-cat",
                        if category.is_empty() {
                            "None".to_owned()
                        } else {
                            category
                        },
                        Pick::AddCategory,
                        cx,
                    ),
                ))
                .child(labelled(
                    &t,
                    "Tags",
                    field(&t, ws.add_inputs.tags.clone()).h(t.px(30.)),
                ))
                .child(labelled(
                    &t,
                    "Layout",
                    dropdown(&t, "add-layout", layout.label(), Pick::AddLayout, cx),
                ))
                .child(labelled(
                    &t,
                    "Stop when",
                    dropdown(&t, "add-stop", stop_label(stop), Pick::AddStop, cx)
                        .when(!start, |d| d.opacity(0.5)),
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(t.px(10.))
                .child(heading(&t, "When added"))
                .child(
                    checkbox(&t, "add-start", "Start torrent", start, None).on_click(cx.listener(
                        |ws, _, _, cx| ws.edit_form(cx, |f| f.opts.paused = !f.opts.paused),
                    )),
                )
                .child(
                    checkbox(
                        &t,
                        "add-top",
                        "Add to top of queue",
                        opts.add_to_top.unwrap_or(false),
                        None,
                    )
                    .on_click(cx.listener(|ws, _, _, cx| {
                        ws.edit_form(cx, |f| {
                            f.opts.add_to_top = Some(!f.opts.add_to_top.unwrap_or(false))
                        })
                    })),
                )
                .child(
                    checkbox(
                        &t,
                        "add-skip",
                        "Skip hash check",
                        opts.skip_check,
                        Some("Trust the files already on disk"),
                    )
                    .on_click(cx.listener(|ws, _, _, cx| {
                        ws.edit_form(cx, |f| f.opts.skip_check = !f.opts.skip_check)
                    })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(t.px(6.))
                .child(
                    disclosure(&t, "add-later", "2 options coming later", later_open).on_click(
                        cx.listener(|ws, _, _, cx| {
                            ws.edit_form(cx, |f| f.later_open = !f.later_open)
                        }),
                    ),
                )
                .when(later_open, |d| {
                    d.child(
                        div()
                            .pl(t.px(19.))
                            .flex()
                            .flex_col()
                            .gap(t.px(4.))
                            .text_color(t.c.muted)
                            .child("Download in sequential order")
                            .child("Download first and last pieces first"),
                    )
                }),
        );

    // ── Right: the files ──
    let files_body: AnyElement = match &head.state {
        PendingState::Resolving => div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(t.px(6.))
            .child(icon("magnet", 20., t.c.muted, &t))
            .child(div().text_color(t.c.ink_2).child("Retrieving metadata…"))
            .child(
                div()
                    .text_color(t.c.muted)
                    .max_w(t.px(320.))
                    .text_center()
                    .child("The file list appears here as soon as a peer sends it."),
            )
            .into_any_element(),
        PendingState::Failed { error } => div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(t.px(6.))
            .p(t.px(16.))
            .child(icon("alert", 20., t.c.bad, &t))
            .child(
                div()
                    .text_color(t.c.bad)
                    .child("Couldn't read this torrent"),
            )
            .child(
                div()
                    .text_color(t.c.muted)
                    .text_center()
                    .child(error.clone()),
            )
            .into_any_element(),
        PendingState::Ready { .. } if rows_len == 0 => div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(t.c.muted)
            .child("No file matches the filter")
            .into_any_element(),
        PendingState::Ready { .. } => uniform_list(
            "add-files",
            rows_len,
            cx.processor(
                move |ws: &mut Workspace, range: std::ops::Range<usize>, _w, cx| {
                    let t = ws.theme;
                    let rows = ws
                        .add_form
                        .as_ref()
                        .map(|f| f.rows.clone())
                        .unwrap_or_default();
                    range
                        .filter_map(|ix| {
                            let r = rows.get(ix)?.clone();
                            let folder = r.file.is_none();
                            Some(
                                div()
                                    .id(ix)
                                    .w_full()
                                    .h(t.px(t.m.row))
                                    .flex()
                                    .items_center()
                                    .gap(t.px(8.))
                                    .pl(t.px(8. + r.depth as f32 * 20.))
                                    .pr(t.px(14.))
                                    .border_b_1()
                                    .border_color(t.c.row_rule)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(t.c.hover))
                                    .on_click(
                                        cx.listener(move |ws, _, _, cx| ws.toggle_row(ix, cx)),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("add-fold-{ix}")))
                                            .size(t.px(16.))
                                            .flex_none()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(folder, |d| {
                                                d.child(icon(
                                                    if r.open {
                                                        "chevron-down"
                                                    } else {
                                                        "chevron-right"
                                                    },
                                                    12.,
                                                    t.c.muted,
                                                    &t,
                                                ))
                                                .on_click(cx.listener(move |ws, _, _, cx| {
                                                    cx.stop_propagation();
                                                    ws.toggle_open(ix, cx)
                                                }))
                                            }),
                                    )
                                    .child(checkbox_mark(&t, r.check, true))
                                    .child(icon(
                                        if folder { "folder" } else { "file" },
                                        14.,
                                        t.c.muted,
                                        &t,
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .text_color(if r.check == Check::None {
                                                t.c.muted
                                            } else {
                                                t.c.ink
                                            })
                                            .child(r.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .w(t.px(84.))
                                            .flex_none()
                                            .flex()
                                            .justify_end()
                                            .font_family(FONT_NUM)
                                            .text_size(t.fpx(12.))
                                            .text_color(t.c.muted)
                                            .child(fmt::size(r.size)),
                                    )
                                    .into_any_element(),
                            )
                        })
                        .collect()
                },
            ),
        )
        .track_scroll(ws.file_scroll.clone())
        .flex_1()
        .size_full()
        .into_any_element(),
    };
    let ready = matches!(head.state, PendingState::Ready { .. });
    let files_line = match &p {
        Some(p) => format!(
            "{} of {} selected, {}",
            selected_n,
            p.files.len(),
            fmt::size(need.unwrap_or(0))
        ),
        None => String::new(),
    };
    let info: AnyElement = match &p {
        Some(p) if info_open => div()
            .flex()
            .flex_col()
            .gap(t.px(4.))
            .pl(t.px(19.))
            .child(info_row(&t, "Info hash v1", p.info_hash_v1.clone(), true))
            .when_some(p.info_hash_v2.clone(), |d, h| {
                d.child(info_row(&t, "Info hash v2", h, true))
            })
            .when_some(p.comment.clone(), |d, c| {
                d.child(info_row(&t, "Comment", c, false))
            })
            .when_some(p.created_by.clone(), |d, c| {
                d.child(info_row(&t, "Created by", c, false))
            })
            .child(info_row(
                &t,
                "Trackers",
                if p.trackers.is_empty() {
                    "None (DHT and peer exchange only)".to_owned()
                } else {
                    p.trackers.join(", ")
                },
                false,
            ))
            .into_any_element(),
        _ => div().into_any_element(),
    };
    let right = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(t.px(10.))
        .px(t.px(24.))
        .pt(t.px(18.))
        .pb(t.px(16.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(t.px(12.))
                .child(heading(&t, "Files"))
                .child(div().text_color(t.c.muted).child(files_line))
                .child(div().flex_1())
                .when(ready, |d| {
                    d.child(
                        div()
                            .w(t.px(180.))
                            .child(field(&t, ws.add_inputs.filter.clone()).h(t.px(30.))),
                    )
                }),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .rounded(t.px(tt_theme::radius::INPUT))
                .border_1()
                .border_color(t.c.rule)
                .bg(t.c.bg)
                .overflow_hidden()
                .child(
                    div()
                        .id("add-files-head")
                        .h(t.px(30.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(t.px(8.))
                        .pl(t.px(32.))
                        .pr(t.px(14.))
                        .bg(t.c.panel)
                        .border_b_1()
                        .border_color(t.c.rule)
                        .text_color(t.c.muted)
                        .when(ready, |d| {
                            d.child(
                                div()
                                    .id("add-all")
                                    .cursor_pointer()
                                    .child(checkbox_mark(&t, all_check, true))
                                    .on_click(cx.listener(move |ws, _, _, cx| {
                                        let on = all_check != Check::All;
                                        ws.edit_form(cx, |f| {
                                            f.selected.iter_mut().for_each(|s| *s = on)
                                        })
                                    })),
                            )
                        })
                        .child(div().flex_1().pl(t.px(22.)).child("Name"))
                        .child(div().w(t.px(84.)).flex().justify_end().child("Size")),
                )
                .child(div().flex_1().min_h_0().flex().flex_col().child(files_body)),
        )
        .when(p.is_some(), |d| {
            d.child(
                disclosure(&t, "add-info", "Torrent information", info_open).on_click(
                    cx.listener(|ws, _, _, cx| ws.edit_form(cx, |f| f.info_open = !f.info_open)),
                ),
            )
            .child(info)
        });

    // ── Footer ──
    let footer = div()
        .flex()
        .items_center()
        .gap(t.px(8.))
        .pl(t.px(24.))
        .pr(t.px(20.))
        .py(t.px(12.))
        .border_t_1()
        .border_color(t.c.rule)
        .bg(t.c.panel)
        .child(
            checkbox(&t, "add-never", "Don't show this again", never_show, None).on_click(
                cx.listener(|ws, _, _, cx| ws.edit_form(cx, |f| f.never_show = !f.never_show)),
            ),
        )
        .child(div().flex_1())
        .child(
            button(&t, "add-cancel", "Cancel", 0)
                .on_click(cx.listener(|ws, _, _, cx| ws.cancel_add(cx))),
        )
        .child(
            button(&t, "add-ok", "Add torrent", if none_chosen { 0 } else { 1 })
                .when(none_chosen, |d| d.opacity(0.5))
                .on_click(cx.listener(|ws, _, _, cx| ws.confirm_add(cx))),
        );

    gpui::deferred(
        div()
            .id("add-scrim")
            .absolute()
            .inset_0()
            .bg(t.c.shadow.opacity(0.9))
            .flex()
            .items_center()
            .justify_center()
            .child(
                // Header and footer stay put; the options column and the
                // file list scroll on their own (DESIGN.md › Overlays never clip).
                div()
                    .id("add-sheet")
                    .w(t.px(960.))
                    .max_w(gpui::relative(0.96))
                    .h(t.px(680.))
                    .max_h(gpui::relative(0.92))
                    .flex()
                    .flex_col()
                    .rounded(t.px(tt_theme::radius::MENU))
                    .bg(t.c.raise)
                    .border_1()
                    .border_color(t.c.rule)
                    .shadow(crate::widgets::pop_shadow(&t))
                    .overflow_hidden()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(header)
                    .child(div().flex_1().min_h_0().flex().child(left).child(right))
                    .child(footer),
            ),
    )
    .with_priority(3)
    .into_any_element()
}

fn info_row(
    t: &Theme,
    label: &'static str,
    value: impl Into<SharedString>,
    mono: bool,
) -> gpui::Div {
    div()
        .flex()
        .gap(t.px(12.))
        .child(
            div()
                .w(t.px(96.))
                .flex_none()
                .text_color(t.c.muted)
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_color(t.c.ink_2)
                .when(mono, |d| d.font_family(FONT_NUM).text_size(t.fpx(12.)))
                .child(value.into()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[(&str, u64)]) -> Vec<PreviewFile> {
        paths
            .iter()
            .map(|(p, s)| PreviewFile {
                path: (*p).into(),
                size: *s,
            })
            .collect()
    }

    #[test]
    fn tree_groups_folders_first_and_sums_sizes() {
        let f = files(&[
            ("b.mkv", 10),
            ("Subs/en.srt", 1),
            ("Subs/de.srt", 2),
            ("a.nfo", 3),
        ]);
        let sel = vec![true, true, false, true];
        let rows = tree_rows(&f, &sel, &HashSet::new(), Some("Sintel"), "");
        let names: Vec<_> = rows.iter().map(|r| (r.depth, r.name.as_str())).collect();
        assert_eq!(
            names,
            vec![
                (0, "Sintel"),
                (1, "Subs"),
                (2, "de.srt"),
                (2, "en.srt"),
                (1, "a.nfo"),
                (1, "b.mkv")
            ]
        );
        assert_eq!(rows[0].size, 16);
        assert_eq!(rows[0].check, Check::Some);
        assert_eq!(rows[1].check, Check::Some);
        assert_eq!(rows[2].check, Check::None);
        let mut members = rows[1].members.clone();
        members.sort();
        assert_eq!(members, vec![1, 2]);
    }

    #[test]
    fn collapsed_folders_hide_their_files() {
        let f = files(&[("Subs/en.srt", 1), ("film.mkv", 10)]);
        let collapsed: HashSet<String> = ["Subs/".to_string()].into();
        let rows = tree_rows(&f, &[true, true], &collapsed, None, "");
        assert_eq!(rows.len(), 2);
        assert!(!rows[0].open);
        assert_eq!(rows[0].check, Check::All);
    }

    #[test]
    fn filter_keeps_matches_and_their_folders() {
        let f = files(&[("Subs/en.srt", 1), ("Subs/de.srt", 2), ("film.mkv", 10)]);
        let collapsed: HashSet<String> = ["Subs/".to_string()].into();
        let rows = tree_rows(&f, &[true, true, true], &collapsed, None, "EN");
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        // The folder shows open while filtering, even though it was collapsed.
        assert_eq!(names, vec!["Subs", "en.srt"]);
        assert_eq!(rows[0].members, vec![0]);
    }

    #[test]
    fn counts_get_separators() {
        assert_eq!(count(7), "7");
        assert_eq!(count(1791), "1,791");
        assert_eq!(count(1_257_434), "1,257,434");
    }

    #[test]
    fn root_follows_the_layout() {
        let p = TorrentPreview {
            name: "sintel.mkv".into(),
            total_size: 1,
            files: files(&[("sintel.mkv", 1)]),
            info_hash_v1: String::new(),
            info_hash_v2: None,
            comment: None,
            created_on: None,
            created_by: None,
            private: false,
            piece_size: 1,
            pieces: 1,
            trackers: Vec::new(),
            existing: None,
        };
        assert_eq!(root_folder(&p, ContentLayout::Original), None);
        assert_eq!(
            root_folder(&p, ContentLayout::Subfolder).as_deref(),
            Some("sintel")
        );
        assert_eq!(root_folder(&p, ContentLayout::NoSubfolder), None);
    }
}
