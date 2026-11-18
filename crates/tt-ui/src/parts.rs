//! Title bar, toolbar, sidebar, transfer list and status bar.

use crate::app::Workspace;
use crate::model::SideFilter;
use crate::overlay::Menu;
use crate::theme::{FONT_NUM, Theme};
use crate::widgets::{hairline_v, icon, kbd, moulded, percent_bar, section_label};
use gpui::{
    AnyElement, Context, FontWeight, Hsla, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Pixels, SharedString, Styled, Window, WindowControlArea, div, point, prelude::*, px,
    uniform_list,
};
use tt_ipc::columns::Align;
use tt_ipc::settings::{ToolbarItem, ToolbarLabel};
use tt_ipc::{Column, Request, StatusFilter, TorrentRow, TorrentState, fmt};

pub const MENUS: [&str; 6] = ["File", "Edit", "View", "Torrent", "Tools", "Help"];

pub fn title_bar(
    ws: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let mac = cfg!(target_os = "macos");
    let caption = |id: &'static str, name: &'static str, area: WindowControlArea, danger: bool| {
        div()
            .id(id)
            .w(t.px(46.))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .window_control_area(area)
            .hover(move |s| {
                if danger {
                    s.bg(t.c.bad).text_color(t.c.accent_ink)
                } else {
                    s.bg(t.c.hover)
                }
            })
            .text_color(t.c.ink_2)
            .child(icon(name, 14., t.c.ink_2, &t))
    };
    let maximized = window.is_maximized();
    div()
        .h(t.px(t.m.title_bar))
        .flex_none()
        .flex()
        .items_center()
        .bg(t.c.side)
        .border_b_1()
        .border_color(t.c.rule)
        .child(
            div()
                .flex()
                .items_center()
                .gap(t.px(2.))
                .pl(t.px(if mac { 80. } else { 12. }))
                .when(!mac, |d| {
                    d.child(
                        div()
                            .size(t.px(18.))
                            .mr(t.px(8.))
                            .rounded_full()
                            .bg(t.c.ink)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("sprout", 12., t.c.bg, &t)),
                    )
                    .children(MENUS.iter().enumerate().map(|(i, name)| {
                        let open = matches!(ws.menu, Some(Menu::Bar { index, .. }) if index == i);
                        div()
                            .id(SharedString::from(format!("menu-{name}")))
                            .px(t.px(8.))
                            .py(t.px(3.))
                            .rounded(t.px(6.))
                            .text_size(t.fpx(13.))
                            .text_color(t.c.ink_2)
                            .when(open, |d| d.bg(t.c.hover))
                            .hover(|s| s.bg(t.c.hover))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                    let _ = e;
                                    this.menu = Some(Menu::Bar {
                                        index: i,
                                        at: point(px(0.), px(0.)),
                                    });
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .child(*name)
                    }))
                }),
        )
        .child(
            div()
                .id("title-drag")
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .window_control_area(WindowControlArea::Drag)
                .text_size(t.fpx(12.5))
                .text_color(t.c.muted)
                .child("tTorrent"),
        )
        .when(!mac, |d| {
            d.child(caption("cap-min", "minus", WindowControlArea::Min, false))
                .child(caption(
                    "cap-max",
                    if maximized { "copy" } else { "square" },
                    WindowControlArea::Max,
                    false,
                ))
                .child(caption("cap-close", "x", WindowControlArea::Close, true))
        })
        .into_any_element()
}

fn tool_button(
    ws: &Workspace,
    item: ToolbarItem,
    label: ToolbarLabel,
    cx: &mut Context<Workspace>,
) -> Option<AnyElement> {
    let t = ws.theme;
    if label == ToolbarLabel::Hidden {
        return None;
    }
    let primary = item == ToolbarItem::AddTorrent;
    let name = match item {
        ToolbarItem::AddTorrent => "plus",
        ToolbarItem::AddMagnet => "magnet",
        ToolbarItem::Resume => "play",
        ToolbarItem::Pause => "pause",
        ToolbarItem::Remove => "trash",
        ToolbarItem::SlowMode => "gauge",
        ToolbarItem::QueueUp => "chevron-up",
        ToolbarItem::QueueDown => "chevron-down",
        ToolbarItem::CreateTorrent => "file",
        ToolbarItem::Rss => "rss",
        ToolbarItem::Search => "search",
        ToolbarItem::Settings => "sliders",
        ToolbarItem::Statistics => "bar-chart",
        ToolbarItem::Log => "list",
        ToolbarItem::Theme => "sun",
    };
    let slow_on = item == ToolbarItem::SlowMode && ws.model.status.slow_mode;
    let fg = if primary {
        t.c.accent_ink
    } else if slow_on {
        t.c.warn
    } else {
        t.c.ink_2
    };
    let with_label = label == ToolbarLabel::IconAndLabel;
    let el = div()
        .id(SharedString::from(format!("tb-{item:?}")))
        .h(t.px(t.m.button))
        .min_w(t.px(t.m.button))
        .px(t.px(if with_label { 12. } else { 0. }))
        .flex()
        .items_center()
        .justify_center()
        .gap(t.px(6.))
        .rounded(t.px(tt_theme::radius::BUTTON))
        .cursor_pointer()
        .when(primary, |d| {
            d.bg(t.c.accent)
                .text_color(t.c.accent_ink)
                .font_weight(FontWeight::SEMIBOLD)
                .hover(|s| s.bg(t.c.accent_hover))
        })
        .when(!primary, |d| {
            d.bg(t.c.raise)
                .border_1()
                .border_color(t.c.rule)
                .shadow(moulded(&t))
                .hover(|s| s.bg(t.c.hover))
                .when(slow_on, |d| d.border_color(t.c.warn))
        })
        .active(|s| s.mt(px(1.)))
        .on_click(cx.listener(move |this, _, window, cx| match item {
            ToolbarItem::AddTorrent => this.add_files(cx),
            ToolbarItem::AddMagnet => {
                window.dispatch_action(Box::new(crate::app::AddMagnet), cx);
            }
            ToolbarItem::Resume => this.resume_sel(cx),
            ToolbarItem::Pause => this.pause_sel(cx),
            ToolbarItem::Remove => this.ask_remove(false, cx),
            ToolbarItem::SlowMode => {
                window.dispatch_action(Box::new(crate::app::SlowMode), cx);
            }
            ToolbarItem::QueueUp => {
                let ids = this.selected();
                this.call(
                    Request::QueueMove {
                        ids,
                        to: tt_ipc::QueueMove::Up,
                    },
                    cx,
                );
            }
            ToolbarItem::QueueDown => {
                let ids = this.selected();
                this.call(
                    Request::QueueMove {
                        ids,
                        to: tt_ipc::QueueMove::Down,
                    },
                    cx,
                );
            }
            ToolbarItem::Settings => this.open_settings(cx),
            ToolbarItem::Theme => window.dispatch_action(Box::new(crate::app::ToggleTheme), cx),
            ToolbarItem::Search => window.dispatch_action(Box::new(crate::app::Filter), cx),
            _ => this.notify(
                format!("{} is coming in a later build", item.label()),
                false,
                cx,
            ),
        }))
        .child(icon(name, 15., fg, &t))
        .when(with_label, |d| d.child(item.label()));
    Some(el.into_any_element())
}

pub fn toolbar(
    ws: &mut Workspace,
    _window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let items = ws.prefs.toolbar.clone();
    let dark = ws.prefs.mode == "dark";
    div()
        .h(t.px(t.m.toolbar))
        .flex_none()
        .flex()
        .items_center()
        .gap(t.px(6.))
        .px(t.px(10.))
        .bg(t.c.panel)
        .border_b_1()
        .border_color(t.c.rule)
        .children(
            items
                .into_iter()
                .filter_map(|(item, label)| tool_button(ws, item, label, cx)),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("search")
                .w(t.px(340.))
                .h(t.px(t.m.button + 2.))
                .flex()
                .items_center()
                .gap(t.px(8.))
                .px(t.px(10.))
                .rounded(t.px(tt_theme::radius::INPUT))
                .bg(t.c.well)
                .border_1()
                .border_color(t.c.rule)
                .child(icon("search", 14., t.c.muted, &t))
                .child(ws.search.clone())
                .child(kbd(crate::widgets::shortcut("Ctrl+K"), &t)),
        )
        .child(
            div()
                .id("theme-toggle")
                .size(t.px(t.m.button + 2.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.px(tt_theme::radius::BUTTON))
                .bg(t.c.raise)
                .border_1()
                .border_color(t.c.rule)
                .hover(|s| s.bg(t.c.hover))
                .on_click(cx.listener(|_, _, window, cx| {
                    window.dispatch_action(Box::new(crate::app::ToggleTheme), cx)
                }))
                .child(icon(if dark { "moon" } else { "sun" }, 15., t.c.ink_2, &t)),
        )
        .into_any_element()
}

fn side_item(
    t: &Theme,
    id: SharedString,
    leading: AnyElement,
    label: SharedString,
    count: usize,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .mx(t.px(6.))
        .h(t.px(t.m.sidebar_item))
        .px(t.px(8.))
        .flex()
        .items_center()
        .gap(t.px(9.))
        .rounded(t.px(tt_theme::radius::ITEM))
        .cursor_pointer()
        .when(selected, |d| {
            d.bg(t.c.accent_soft).text_color(t.c.accent_fg)
        })
        .when(!selected, |d| {
            d.text_color(t.c.ink_2).hover(|s| s.bg(t.c.hover))
        })
        .child(leading)
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(
            div()
                .font_family(FONT_NUM)
                .text_size(t.fpx(11.5))
                .text_color(if selected { t.c.accent_fg } else { t.c.muted })
                .child(count.to_string()),
        )
}

fn status_icon(f: StatusFilter) -> &'static str {
    match f {
        StatusFilter::All => "grid",
        StatusFilter::Downloading => "arrow-down",
        StatusFilter::Seeding => "arrow-up",
        StatusFilter::Completed => "check",
        StatusFilter::Stopped => "pause",
        StatusFilter::Active => "zap",
        StatusFilter::Inactive => "clock",
        StatusFilter::Stalled => "alert",
        StatusFilter::Checking => "refresh",
        StatusFilter::Errored => "alert",
    }
}

pub fn sidebar(
    ws: &mut Workspace,
    _window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let m = &ws.model;
    let shown = [
        StatusFilter::All,
        StatusFilter::Downloading,
        StatusFilter::Seeding,
        StatusFilter::Completed,
        StatusFilter::Stopped,
        StatusFilter::Active,
        StatusFilter::Inactive,
        StatusFilter::Stalled,
        StatusFilter::Checking,
        StatusFilter::Errored,
    ];
    let mut col = div()
        .id("sidebar")
        .w(t.px(t.m.sidebar))
        .flex_none()
        .h_full()
        .overflow_y_scroll()
        .bg(t.c.side)
        .border_r_1()
        .border_color(t.c.rule)
        .pb(t.px(10.))
        .child(section_label("Status", &t));
    for f in shown {
        let sel = m.status_filter == f;
        let leading = icon(
            status_icon(f),
            14.,
            if sel { t.c.accent_fg } else { t.c.muted },
            &t,
        )
        .into_any_element();
        col = col.child(
            side_item(
                &t,
                format!("st-{f:?}").into(),
                leading,
                f.label().into(),
                m.status_count(f),
                sel,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.model.status_filter = f;
                this.model.refresh();
                cx.notify();
            })),
        );
    }
    col = col.child(section_label("Categories", &t));
    for (i, c) in m.lists.categories.iter().enumerate() {
        let sel = m.side_filter == SideFilter::Category(c.name.clone());
        let color = t.c.categories[c.color as usize % 6];
        let name = c.name.clone();
        col = col.child(
            side_item(
                &t,
                format!("cat-{i}").into(),
                div()
                    .size(t.px(10.))
                    .rounded(t.px(3.))
                    .bg(color)
                    .mx(t.px(2.))
                    .into_any_element(),
                c.name.clone().into(),
                m.category_count(&c.name),
                sel,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                let f = SideFilter::Category(name.clone());
                let f = if this.model.side_filter == f {
                    SideFilter::None
                } else {
                    f
                };
                this.set_side_filter(f, cx);
            })),
        );
    }
    {
        let sel = m.side_filter == SideFilter::Uncategorized;
        col = col.child(
            side_item(
                &t,
                "cat-none".into(),
                icon("folder", 14., t.c.muted, &t).into_any_element(),
                "Uncategorized".into(),
                m.category_count(""),
                sel,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                let f = if this.model.side_filter == SideFilter::Uncategorized {
                    SideFilter::None
                } else {
                    SideFilter::Uncategorized
                };
                this.set_side_filter(f, cx);
            })),
        );
    }
    if !m.lists.tags.is_empty() {
        col = col.child(section_label("Tags", &t));
        for (i, tag) in m.lists.tags.iter().enumerate() {
            let sel = m.side_filter == SideFilter::Tag(tag.clone());
            let name = tag.clone();
            col = col.child(
                side_item(
                    &t,
                    format!("tag-{i}").into(),
                    icon("tag", 14., t.c.muted, &t).into_any_element(),
                    tag.clone().into(),
                    m.tag_count(tag),
                    sel,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    let f = SideFilter::Tag(name.clone());
                    let f = if this.model.side_filter == f {
                        SideFilter::None
                    } else {
                        f
                    };
                    this.set_side_filter(f, cx);
                })),
            );
        }
    }
    if !m.lists.tags.is_empty() {
        let sel = m.side_filter == SideFilter::Untagged;
        let untagged = m.rows.values().filter(|r| r.tags.is_empty()).count();
        col = col.child(
            side_item(
                &t,
                "tag-none".into(),
                icon("tag", 14., t.c.muted, &t).into_any_element(),
                "Untagged".into(),
                untagged,
                sel,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                let f = if this.model.side_filter == SideFilter::Untagged {
                    SideFilter::None
                } else {
                    SideFilter::Untagged
                };
                this.set_side_filter(f, cx);
            })),
        );
    }
    let (trackers, trackerless) = m.trackers();
    col = col.child(section_label("Trackers", &t));
    for (i, (host, n)) in trackers.into_iter().enumerate() {
        let sel = m.side_filter == SideFilter::Tracker(host.clone());
        let h2 = host.clone();
        col = col.child(
            side_item(
                &t,
                format!("trk-{i}").into(),
                icon("server", 14., t.c.muted, &t).into_any_element(),
                host.into(),
                n,
                sel,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                let f = SideFilter::Tracker(h2.clone());
                let f = if this.model.side_filter == f {
                    SideFilter::None
                } else {
                    f
                };
                this.set_side_filter(f, cx);
            })),
        );
    }
    let sel = m.side_filter == SideFilter::Trackerless;
    col = col.child(
        side_item(
            &t,
            "trk-none".into(),
            icon("server", 14., t.c.muted, &t).into_any_element(),
            "Trackerless".into(),
            trackerless,
            sel,
        )
        .on_click(cx.listener(|this, _, _, cx| {
            let f = if this.model.side_filter == SideFilter::Trackerless {
                SideFilter::None
            } else {
                SideFilter::Trackerless
            };
            this.set_side_filter(f, cx);
        })),
    );
    col.into_any_element()
}

pub fn state_color(t: &Theme, s: TorrentState) -> Hsla {
    match s {
        TorrentState::Downloading | TorrentState::Metadata => t.c.accent_fg,
        TorrentState::Seeding | TorrentState::Completed => t.c.ok,
        TorrentState::Checking | TorrentState::Stalled | TorrentState::Moving => t.c.warn,
        TorrentState::DriveUnavailable => t.c.warn,
        TorrentState::Error | TorrentState::MissingFiles => t.c.bad,
        TorrentState::Stopped | TorrentState::Queued | TorrentState::Starting => t.c.muted,
    }
}

pub fn state_icon(s: TorrentState) -> &'static str {
    match s {
        TorrentState::Downloading | TorrentState::Metadata => "arrow-down",
        TorrentState::Seeding => "arrow-up",
        TorrentState::Completed => "check",
        TorrentState::Checking | TorrentState::Moving => "refresh",
        TorrentState::Stalled => "alert",
        TorrentState::Stopped => "pause",
        TorrentState::Queued | TorrentState::Starting => "clock",
        TorrentState::Error | TorrentState::MissingFiles | TorrentState::DriveUnavailable => {
            "alert"
        }
    }
}

fn progress_fill(t: &Theme, r: &TorrentRow) -> Hsla {
    match r.state {
        TorrentState::Checking | TorrentState::Moving => t.c.warn,
        TorrentState::Error | TorrentState::MissingFiles => t.c.bad,
        TorrentState::Stopped | TorrentState::Queued | TorrentState::Starting => t.c.muted,
        _ if r.progress >= 1.0 => t.c.ok,
        _ => t.c.accent,
    }
}

fn cell(t: &Theme, col: Column, r: &TorrentRow, name_w: Pixels, name: &SharedString) -> AnyElement {
    let w = col.default_width();
    let base = div()
        .w(t.px(w))
        .flex_none()
        .px(t.px(10.))
        .overflow_hidden()
        .whitespace_nowrap();
    match col {
        Column::Name => div()
            .w(name_w)
            .flex_none()
            .px(t.px(10.))
            .flex()
            .items_center()
            .gap(t.px(8.))
            .overflow_hidden()
            .child(icon("file", 14., t.c.muted, t))
            .child(
                div()
                    .w(name_w - t.px(44.))
                    .flex_none()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(t.c.ink)
                    .text_size(t.fpx(t.m.ui_font + 1.))
                    .child(name.clone()),
            )
            .into_any_element(),
        Column::Progress => base
            .flex()
            .items_center()
            .gap(t.px(8.))
            .child(percent_bar(r.progress, progress_fill(t, r), t))
            .child(
                div()
                    .w(t.px(44.))
                    .flex_none()
                    .text_right()
                    .font_family(FONT_NUM)
                    .text_size(t.fpx(t.m.num_font))
                    .text_color(t.c.ink_2)
                    .child(fmt::percent(r.progress)),
            )
            .into_any_element(),
        Column::Status => {
            let c = state_color(t, r.state);
            base.flex()
                .items_center()
                .gap(t.px(6.))
                .text_color(c)
                .child(icon(state_icon(r.state), 13., c, t))
                .child(div().truncate().child(r.state.label()))
                .into_any_element()
        }
        Column::Category => base
            .truncate()
            .text_color(t.c.ink_2)
            .child(r.category.clone())
            .into_any_element(),
        _ => {
            let right = col.align() == Align::Right;
            base.when(right, |d| d.text_right())
                .when(!col.is_text(), |d| {
                    d.font_family(FONT_NUM).text_size(t.fpx(t.m.num_font))
                })
                .when(col.ellipsizes(), |d| d.truncate())
                .text_color(t.c.ink_2)
                .child(col.text(r))
                .into_any_element()
        }
    }
}

/// GPUI 0.2.2 measures text once before its width is known and keeps that
/// size, so `text_ellipsis` never applies in the list; shorten it here with
/// GPUI's own line wrapper instead.
pub fn ellipsize(cx: &gpui::App, text: &str, width: Pixels, size: Pixels) -> SharedString {
    let font = gpui::font(crate::theme::FONT_UI);
    let mut runs = vec![gpui::TextRun {
        len: text.len(),
        font: font.clone(),
        color: Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let mut w = cx.text_system().line_wrapper(font, size);
    w.truncate_line(SharedString::from(text.to_owned()), width, "…", &mut runs)
}

/// The Name column fills what the other columns leave (at least its default
/// width), with a definite width so long names end in an ellipsis.
fn name_width(ws: &Workspace, window: &Window) -> Pixels {
    let t = ws.theme;
    let others: f32 = ws
        .prefs
        .columns
        .iter()
        .filter(|c| c.column != Column::Name)
        .map(|c| f32::from(t.px(c.column.default_width())))
        .sum();
    let side = if ws.prefs.sidebar {
        f32::from(t.px(t.m.sidebar)) + 1.0
    } else {
        0.0
    };
    let free = f32::from(window.viewport_size().width) - side - others;
    px(free.max(f32::from(t.px(Column::Name.default_width()))))
}

pub fn transfer_list(
    ws: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let cols: Vec<Column> = ws.prefs.columns.iter().map(|c| c.column).collect();
    let name_w = name_width(ws, window);
    let (sort, desc) = (ws.model.sort, ws.model.sort_desc);
    let header = div()
        .h(t.px(t.m.header))
        .flex_none()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(t.c.rule)
        .bg(t.c.bg)
        .text_size(t.fpx(12.5))
        .text_color(t.c.muted)
        .children(cols.iter().map(|&col| {
            let active = col == sort;
            let right = col.align() == Align::Right;
            let w = col.default_width();
            div()
                .id(SharedString::from(format!("h-{col:?}")))
                .when(col == Column::Name, |d| d.w(name_w).flex_none())
                .when(col != Column::Name, |d| d.w(t.px(w)).flex_none())
                .h_full()
                .px(t.px(10.))
                .flex()
                .items_center()
                .gap(t.px(3.))
                .when(right, |d| d.justify_end())
                .cursor_pointer()
                .hover(|s| s.bg(t.c.hover))
                .when(active, |d| {
                    d.text_color(t.c.ink).font_weight(FontWeight::MEDIUM)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model.sort_by(col);
                    this.save_prefs(cx);
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, e: &MouseDownEvent, _, cx| {
                        this.open_menu_at(Menu::Columns { at: e.position }, cx);
                        cx.stop_propagation();
                    }),
                )
                .child(div().truncate().child(col.label()))
                .when(active, |d| {
                    d.child(icon(
                        if desc { "chevron-down" } else { "chevron-up" },
                        12.,
                        t.c.ink_2,
                        &t,
                    ))
                })
        }));
    let count = ws.model.view.len();
    let body = if count == 0 {
        empty_state(ws).into_any_element()
    } else {
        let row_h = t.px(t.m.row);
        uniform_list(
            "rows",
            count,
            cx.processor(
                move |this: &mut Workspace, range: std::ops::Range<usize>, _window, cx| {
                    let t = this.theme;
                    let cols: Vec<Column> = this.prefs.columns.iter().map(|c| c.column).collect();
                    range
                        .filter_map(|ix| {
                            let r = this.model.row_at(ix)?.clone();
                            let selected = this.model.selected.contains(&r.id);
                            let starting = r.state == TorrentState::Starting;
                            let name =
                                ellipsize(cx, &r.name, name_w - t.px(48.), t.fpx(t.m.ui_font + 1.));
                            Some(
                                div()
                                    .id(ix)
                                    .h(row_h)
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .relative()
                                    .border_b_1()
                                    .border_color(t.c.row_rule)
                                    .when(selected, |d| d.bg(t.c.accent_soft))
                                    .when(!selected, |d| d.hover(|s| s.bg(t.c.hover)))
                                    .when(starting, |d| d.opacity(0.62))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                            let ctrl = e.modifiers.control || e.modifiers.platform;
                                            this.menu = None;
                                            window.focus(&this.focus);
                                            if e.click_count == 2 {
                                                let paused = this.model.selection_paused();
                                                if paused {
                                                    this.resume_sel(cx)
                                                } else {
                                                    this.pause_sel(cx)
                                                }
                                                return;
                                            }
                                            this.select_row(ix, ctrl, e.modifiers.shift, cx);
                                        }),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                            window.focus(&this.focus);
                                            this.model.context_click(ix);
                                            this.context_menu_at(e.position, cx);
                                            cx.stop_propagation();
                                        }),
                                    )
                                    .when(selected, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .left_0()
                                                .top_0()
                                                .bottom_0()
                                                .w(t.px(3.))
                                                .bg(t.c.accent),
                                        )
                                    })
                                    .children(cols.iter().map(|&c| cell(&t, c, &r, name_w, &name))),
                            )
                        })
                        .collect::<Vec<_>>()
                },
            ),
        )
        .track_scroll(ws.scroll.clone())
        .flex_1()
        .into_any_element()
    };
    div()
        .id("list-hscroll")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_x_scroll()
        .bg(t.c.bg)
        .child(div().flex().flex_col().h_full().child(header).child(body))
        .into_any_element()
}

fn empty_state(ws: &Workspace) -> impl IntoElement {
    let t = ws.theme;
    let filtered = !ws.model.rows.is_empty();
    let (title, hint) = if filtered {
        (
            "No torrents match",
            "Clear the filter or pick another status".to_string(),
        )
    } else {
        (
            "No torrents yet",
            format!(
                "Drop a .torrent file anywhere, or press {}",
                crate::widgets::shortcut("Ctrl+O")
            ),
        )
    };
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(t.px(6.))
        .child(
            div()
                .font_family(crate::theme::FONT_HEAD)
                .font_weight(FontWeight::BOLD)
                .text_size(t.fpx(16.))
                .child(title),
        )
        .child(div().text_color(t.c.muted).child(hint))
}

fn stat(t: &Theme, name: &'static str, value: String, label: Option<&'static str>) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(t.px(6.))
        .px(t.px(10.))
        .whitespace_nowrap()
        .child(icon(name, 13., t.c.muted, t))
        .when_some(label, |d, l| d.child(div().text_color(t.c.muted).child(l)))
        .child(
            div()
                .font_family(FONT_NUM)
                .text_size(t.fpx(11.5))
                .text_color(t.c.ink_2)
                .child(value),
        )
}

pub fn status_bar(
    ws: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    use tt_ipc::settings::StatusItem as I;
    let t = ws.theme;
    let s = &ws.model.status;
    let has = |i: I| ws.prefs.status_items.contains(&i);
    let width = f32::from(window.viewport_size().width) / t.scale;
    // DESIGN.md › Overlays never clip: drop totals, then labels, then optional items.
    let totals = has(I::SessionTotals) && width >= 1100.;
    let labels = width >= 900.;
    let narrow_ok = width >= 800.;
    let mem = s.engine_memory + ws.window_memory;
    let speed = |bps: u64, total: u64| {
        if totals {
            format!("{} ({})", fmt::speed(bps), fmt::size(total))
        } else {
            fmt::speed(bps)
        }
    };
    let mut left: Vec<AnyElement> = Vec::new();
    if has(I::FreeSpace) {
        left.push(
            stat(
                &t,
                "drive",
                s.free_space.map(fmt::size).unwrap_or_else(|| "—".into()),
                labels.then_some("Free space"),
            )
            .into_any_element(),
        );
    }
    if has(I::Dht) && narrow_ok {
        left.push(
            stat(
                &t,
                "globe",
                format!("{} nodes", s.dht_nodes),
                labels.then_some("DHT"),
            )
            .into_any_element(),
        );
    }
    if has(I::Download) {
        left.push(
            stat(
                &t,
                "arrow-down",
                speed(s.down_bps, s.session_downloaded),
                None,
            )
            .into_any_element(),
        );
    }
    if has(I::Upload) {
        left.push(
            stat(&t, "arrow-up", speed(s.up_bps, s.session_uploaded), None).into_any_element(),
        );
    }
    if let Some((done, total)) = s.starting {
        let p = if total == 0 {
            1.0
        } else {
            done as f32 / total as f32
        };
        left.push(
            div()
                .flex()
                .items_center()
                .gap(t.px(8.))
                .px(t.px(10.))
                .whitespace_nowrap()
                .text_color(t.c.muted)
                .child(format!("Starting torrents {done} of {total}"))
                .child(
                    div()
                        .w(t.px(64.))
                        .flex()
                        .child(percent_bar(p, t.c.accent, &t)),
                )
                .into_any_element(),
        );
    } else if ws.model.from_snapshot {
        left.push(
            div()
                .px(t.px(10.))
                .text_color(t.c.muted)
                .child("Starting…")
                .into_any_element(),
        );
    }
    let mut right: Vec<AnyElement> = Vec::new();
    if s.slow_mode {
        right.push(
            div()
                .flex()
                .items_center()
                .gap(t.px(6.))
                .px(t.px(10.))
                .text_color(t.c.warn)
                .child(icon("gauge", 13., t.c.warn, &t))
                .child("Slow mode")
                .into_any_element(),
        );
    }
    if has(I::Ratio) && narrow_ok {
        let ratio = if s.session_downloaded > 0 {
            fmt::ratio(s.session_uploaded as f32 / s.session_downloaded as f32)
        } else {
            "—".into()
        };
        right.push(stat(&t, "refresh", ratio, labels.then_some("Ratio")).into_any_element());
    }
    if has(I::Peers) && narrow_ok {
        right.push(
            stat(
                &t,
                "server",
                s.connected_peers.to_string(),
                labels.then_some("Peers"),
            )
            .into_any_element(),
        );
    }
    if has(I::Memory) {
        right.push(stat(&t, "chip", fmt::size(mem), labels.then_some("Memory")).into_any_element());
    }
    if has(I::Port) {
        let (port_label, port_color, port_icon) = match (s.port, s.port_open) {
            (Some(p), Some(true)) => (format!("Port {p} open"), t.c.ok, "check"),
            (Some(p), Some(false)) => (format!("Port {p} closed"), t.c.warn, "alert"),
            (Some(p), None) => (format!("Port {p}"), t.c.muted, "plug"),
            (None, _) => ("No port".into(), t.c.muted, "plug"),
        };
        right.push(
            div()
                .flex()
                .items_center()
                .gap(t.px(6.))
                .px(t.px(10.))
                .whitespace_nowrap()
                .text_color(port_color)
                .child(icon(port_icon, 13., port_color, &t))
                .child(port_label)
                .into_any_element(),
        );
    }
    // Items separated by hairlines.
    let join = |items: Vec<AnyElement>| {
        let mut out = Vec::new();
        for (i, el) in items.into_iter().enumerate() {
            if i > 0 {
                out.push(hairline_v(&t).into_any_element());
            }
            out.push(el);
        }
        out
    };
    div()
        .id("status-bar")
        .h(t.px(t.m.status_bar))
        .flex_none()
        .flex()
        .items_center()
        .bg(t.c.panel)
        .border_t_1()
        .border_color(t.c.rule)
        .text_size(t.fpx(12.))
        .overflow_hidden()
        // Right-click chooses what it shows.
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|this, e: &MouseDownEvent, _, cx| {
                this.open_menu_at(Menu::StatusBar { at: e.position }, cx);
                cx.stop_propagation();
            }),
        )
        .children(join(left))
        .child(div().flex_1())
        .children(join(right))
        .into_any_element()
}
