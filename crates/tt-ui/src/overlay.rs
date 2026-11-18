//! Menus (torrent, menu bar, columns) and sheets (add magnet, remove, rename,
//! settings, shortcuts). Menus stay inside the window and scroll if needed;
//! submenus flip left at the edge (DESIGN.md › Overlays never clip).

use crate::app::Workspace;
use crate::theme::{FONT_HEAD, FONT_NUM, Theme};
use crate::widgets::{icon, kbd, pop_shadow, shortcut};
use gpui::{
    AnyElement, Context, FontWeight, Hsla, IntoElement, MouseButton, ParentElement, Pixels, Point,
    SharedString, Styled, Window, anchored, deferred, div, point, prelude::*, px,
};
use std::rc::Rc;
use tt_ipc::{Column, QueueMove, Request, TagMode, TorrentId, settings::ColumnPref};

#[derive(Clone, Debug, PartialEq)]
pub enum Menu {
    Torrent {
        at: Point<Pixels>,
    },
    Bar {
        index: usize,
        at: Point<Pixels>,
    },
    Columns {
        at: Point<Pixels>,
    },
    StatusBar {
        at: Point<Pixels>,
    },
    /// A dropdown's choices (above sheets, so it works inside the Add dialog).
    Pick {
        kind: Pick,
        at: Point<Pixels>,
    },
}

/// What a dropdown chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    AddCategory,
    AddLayout,
    AddStop,
    Layout,
    Stop,
    ShareMode,
    ShareAction,
}

/// qBittorrent's "Stop condition" values, read as a sentence after "Stop when".
pub fn stop_label(s: tt_ipc::StopCondition) -> &'static str {
    match s {
        tt_ipc::StopCondition::None => "Never",
        tt_ipc::StopCondition::MetadataReceived => "Metadata received",
        tt_ipc::StopCondition::FilesChecked => "Files checked",
    }
}

/// A dropdown: the current value and a chevron; a click opens its choices.
pub fn dropdown(
    t: &Theme,
    id: &'static str,
    value: impl Into<SharedString>,
    kind: Pick,
    cx: &mut Context<Workspace>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(t.px(30.))
        .px(t.px(10.))
        .flex()
        .items_center()
        .gap(t.px(8.))
        .rounded(t.px(tt_theme::radius::INPUT))
        .bg(t.c.well)
        .border_1()
        .border_color(t.c.rule)
        .cursor_pointer()
        .hover(|s| s.border_color(t.c.accent_line))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(t.c.ink)
                .child(value.into()),
        )
        .child(icon("chevron-down", 12., t.c.muted, t))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |ws, e: &gpui::MouseDownEvent, _, cx| {
                ws.menu = Some(Menu::Pick {
                    kind,
                    at: e.position,
                });
                ws.sub_open = None;
                cx.stop_propagation();
                cx.notify();
            }),
        )
}

fn pick_items(ws: &Workspace, kind: Pick) -> Vec<Item> {
    use tt_ipc::{ContentLayout, ShareLimitAction, ShareLimitsMode, StopCondition};
    let form = ws
        .add_form
        .as_ref()
        .map(|f| f.opts.clone())
        .unwrap_or_default();
    let s = ws.settings.clone().unwrap_or_default();
    let set = |f: fn(&mut tt_ipc::EngineSettings, usize), i: usize| {
        move |ws: &mut Workspace, _: &mut Window, cx: &mut Context<Workspace>| {
            if let Some(mut s) = ws.settings.clone() {
                f(&mut s, i);
                ws.set_engine_settings(s, cx);
            }
        }
    };
    match kind {
        Pick::AddCategory => {
            let cur = form.category.clone().unwrap_or_default();
            std::iter::once((String::new(), String::new()))
                .chain(
                    ws.model
                        .lists
                        .categories
                        .iter()
                        .map(|c| (c.name.clone(), c.save_path.clone())),
                )
                .map(|(name, path)| {
                    let label = if name.is_empty() {
                        "None".to_owned()
                    } else {
                        name.clone()
                    };
                    check(label, name == cur, None, move |ws, _, cx| {
                        ws.pick_add_category(name.clone(), path.clone(), cx)
                    })
                })
                .collect()
        }
        Pick::AddLayout => ContentLayout::ALL
            .iter()
            .map(|&l| {
                check(
                    l.label(),
                    form.content_layout.unwrap_or_default() == l,
                    None,
                    move |ws, _, cx| ws.edit_form(cx, |f| f.opts.content_layout = Some(l)),
                )
            })
            .collect(),
        Pick::AddStop => StopCondition::ALL
            .iter()
            .map(|&c| {
                check(
                    stop_label(c),
                    form.stop_condition.unwrap_or_default() == c,
                    None,
                    move |ws, _, cx| ws.edit_form(cx, |f| f.opts.stop_condition = Some(c)),
                )
            })
            .collect(),
        Pick::Layout => ContentLayout::ALL
            .iter()
            .enumerate()
            .map(|(i, &l)| {
                check(
                    l.label(),
                    s.content_layout == l,
                    None,
                    set(|s, i| s.content_layout = ContentLayout::ALL[i], i),
                )
            })
            .collect(),
        Pick::Stop => StopCondition::ALL
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                check(
                    stop_label(c),
                    s.stop_condition == c,
                    None,
                    set(|s, i| s.stop_condition = StopCondition::ALL[i], i),
                )
            })
            .collect(),
        Pick::ShareMode => ShareLimitsMode::ALL
            .iter()
            .enumerate()
            .map(|(i, &m)| {
                check(
                    m.label(),
                    s.share_limits_mode == m,
                    None,
                    set(|s, i| s.share_limits_mode = ShareLimitsMode::ALL[i], i),
                )
            })
            .collect(),
        Pick::ShareAction => ShareLimitAction::OFFERED
            .iter()
            .enumerate()
            .map(|(i, &a)| {
                check(
                    a.label(),
                    s.share_limit_action == a,
                    None,
                    set(
                        |s, i| s.share_limit_action = ShareLimitAction::OFFERED[i],
                        i,
                    ),
                )
            })
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dialog {
    Magnet,
    Remove {
        ids: Vec<TorrentId>,
        delete_files: bool,
    },
    Rename {
        id: TorrentId,
    },
    Settings,
    Shortcuts,
}

type Act = Rc<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>;

#[derive(Clone)]
pub enum Item {
    Act {
        label: SharedString,
        icon: Option<&'static str>,
        keys: Option<&'static str>,
        check: Option<bool>,
        danger: bool,
        enabled: bool,
        on: Act,
    },
    Sub {
        label: SharedString,
        items: Vec<Item>,
    },
    Sep,
}

fn act(
    label: impl Into<SharedString>,
    icon: Option<&'static str>,
    keys: Option<&'static str>,
    on: impl Fn(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
) -> Item {
    Item::Act {
        label: label.into(),
        icon,
        keys,
        check: None,
        danger: false,
        enabled: true,
        on: Rc::new(on),
    }
}

fn check(
    label: impl Into<SharedString>,
    on_now: bool,
    keys: Option<&'static str>,
    on: impl Fn(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
) -> Item {
    Item::Act {
        label: label.into(),
        icon: None,
        keys,
        check: Some(on_now),
        danger: false,
        enabled: true,
        on: Rc::new(on),
    }
}

/// Shown but greyed: in qBittorrent's menu, not built yet.
fn later(label: &'static str, icon: Option<&'static str>, keys: Option<&'static str>) -> Item {
    Item::Act {
        label: label.into(),
        icon,
        keys,
        check: None,
        danger: false,
        enabled: false,
        on: Rc::new(|_, _, _| {}),
    }
}

/// The torrent menu, in qBittorrent's order (DESIGN.md › Torrent menu).
fn torrent_items(ws: &Workspace) -> Vec<Item> {
    let paused = ws.model.selection_paused();
    let focused = ws.model.focused().cloned();
    let mut cats: Vec<Item> = vec![
        later("New category…", Some("plus"), None),
        act("Reset", None, None, |ws, _, cx| {
            let ids = ws.selected();
            ws.call(
                Request::SetCategory {
                    ids,
                    category: None,
                },
                cx,
            );
        }),
        Item::Sep,
    ];
    for c in &ws.model.lists.categories {
        let name = c.name.clone();
        let on = focused.as_ref().is_some_and(|r| r.category == name);
        cats.push(check(name.clone(), on, None, move |ws, _, cx| {
            let ids = ws.selected();
            ws.call(
                Request::SetCategory {
                    ids,
                    category: Some(name.clone()),
                },
                cx,
            );
        }));
    }
    let mut tags: Vec<Item> = vec![
        later("Add tags…", Some("plus"), None),
        act("Remove all", None, None, |ws, _, cx| {
            let ids = ws.selected();
            ws.call(
                Request::SetTags {
                    ids,
                    tags: Vec::new(),
                    mode: TagMode::Set,
                },
                cx,
            );
        }),
        Item::Sep,
    ];
    for tg in &ws.model.lists.tags {
        let name = tg.clone();
        let on = focused.as_ref().is_some_and(|r| r.tags.contains(&name));
        tags.push(check(name.clone(), on, None, move |ws, _, cx| {
            let ids = ws.selected();
            ws.call(
                Request::SetTags {
                    ids,
                    tags: vec![name.clone()],
                    mode: if on { TagMode::Remove } else { TagMode::Add },
                },
                cx,
            );
        }));
    }
    let q = |label: &'static str, to: QueueMove| {
        act(label, None, None, move |ws, _, cx| {
            let ids = ws.selected();
            ws.call(Request::QueueMove { ids, to }, cx);
        })
    };
    let copy = |label: &'static str, f: fn(&tt_ipc::TorrentRow) -> String| {
        act(label, None, None, move |ws, _, cx| {
            if let Some(r) = ws.model.focused() {
                let text = f(r);
                ws.copy_text(text, label, cx);
            }
        })
    };
    vec![
        if paused {
            act("Start", Some("play"), Some("Ctrl+S"), |ws, _, cx| {
                ws.resume_sel(cx)
            })
        } else {
            act("Stop", Some("pause"), Some("Ctrl+P"), |ws, _, cx| {
                ws.pause_sel(cx)
            })
        },
        act("Force start", Some("zap"), None, |ws, _, cx| {
            let ids = ws.selected();
            ws.call(Request::ForceStart { ids }, cx);
        }),
        Item::Sep,
        Item::Act {
            label: "Remove".into(),
            icon: Some("trash"),
            keys: Some("Delete"),
            check: None,
            danger: true,
            enabled: true,
            on: Rc::new(|ws, _, cx| ws.ask_remove(false, cx)),
        },
        Item::Sep,
        later("Set location…", Some("folder"), None),
        act("Rename…", Some("pencil"), Some("F2"), |_, w, cx| {
            w.dispatch_action(Box::new(crate::app::Rename), cx)
        }),
        act("Manage content…", Some("list"), None, |ws, _, cx| {
            ws.set_tab(tt_ipc::DetailsTab::Content, cx)
        }),
        act("Edit trackers…", Some("server"), None, |ws, _, cx| {
            ws.set_tab(tt_ipc::DetailsTab::Trackers, cx)
        }),
        Item::Sub {
            label: "Category".into(),
            items: cats,
        },
        Item::Sub {
            label: "Tags".into(),
            items: tags,
        },
        later("Automatic torrent management", None, None),
        Item::Sep,
        Item::Sub {
            label: "Queue".into(),
            items: vec![
                q("Move to top", QueueMove::Top),
                q("Move up", QueueMove::Up),
                q("Move down", QueueMove::Down),
                q("Move to bottom", QueueMove::Bottom),
            ],
        },
        later("Torrent options…", Some("sliders"), None),
        later("Super seeding mode", None, None),
        Item::Sep,
        later("Preview file…", None, None),
        Item::Sep,
        act(
            "Force recheck",
            Some("refresh"),
            Some("Ctrl+R"),
            |ws, _, cx| {
                let ids = ws.selected();
                ws.call(Request::Recheck { ids }, cx);
            },
        ),
        act("Force reannounce", None, None, |ws, _, cx| {
            let ids = ws.selected();
            ws.call(Request::Reannounce { ids }, cx);
        }),
        Item::Sep,
        act(
            "Open destination folder",
            Some("folder"),
            Some("Ctrl+Enter"),
            |_, w, cx| w.dispatch_action(Box::new(crate::app::OpenFolder), cx),
        ),
        Item::Sub {
            label: "Copy".into(),
            items: vec![
                copy("Name", |r| r.name.clone()),
                copy("Info hash v1", |r| r.hash_v1.clone()),
                act("Magnet link", None, Some("Ctrl+Shift+C"), |ws, _, cx| {
                    ws.copy_magnet(cx)
                }),
                copy("Torrent ID", |r| r.hash_v1.clone()),
            ],
        },
        later("Export .torrent…", None, None),
    ]
}

fn bar_items(ws: &Workspace, index: usize) -> Vec<Item> {
    let p = &ws.prefs;
    match index {
        0 => vec![
            act(
                "Add torrent file…",
                Some("plus"),
                Some("Ctrl+O"),
                |ws, _, cx| ws.add_files(cx),
            ),
            act(
                "Add magnet link…",
                Some("magnet"),
                Some("Ctrl+Shift+O"),
                |_, w, cx| w.dispatch_action(Box::new(crate::app::AddMagnet), cx),
            ),
            later("Create torrent…", Some("file"), Some("Ctrl+N")),
            Item::Sep,
            act("Close window", None, Some("Ctrl+W"), |_, w, _| {
                w.remove_window()
            }),
            act("Exit", None, Some("Ctrl+Q"), |_, w, cx| {
                w.dispatch_action(Box::new(crate::app::Quit), cx)
            }),
        ],
        1 => vec![
            act("Select all", None, Some("Ctrl+A"), |ws, _, cx| {
                ws.model.select_all();
                cx.notify();
            }),
            act(
                "Filter torrents",
                Some("search"),
                Some("Ctrl+F"),
                |_, w, cx| w.dispatch_action(Box::new(crate::app::Filter), cx),
            ),
        ],
        2 => {
            let density = |label: &'static str, key: &'static str| {
                let on = ws.prefs.density == key;
                check(label, on, None, move |ws, _, cx| {
                    ws.prefs.density = key.into();
                    ws.save_prefs(cx);
                })
            };
            vec![
                check("Sidebar", p.sidebar, Some("Ctrl+B"), |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ToggleSidebar), cx)
                }),
                check("Details", p.details, Some("Ctrl+J"), |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ToggleDetails), cx)
                }),
                act("Columns…", Some("list"), None, |ws, w, cx| {
                    let at = w.mouse_position();
                    ws.open_menu_at(Menu::Columns { at }, cx);
                }),
                Item::Sub {
                    label: "Status bar".into(),
                    items: status_items(ws),
                },
                Item::Sep,
                check("Dark mode", p.mode == "dark", None, |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ToggleTheme), cx)
                }),
                Item::Sub {
                    label: "Density".into(),
                    items: vec![
                        density("Compact", "compact"),
                        density("Cozy", "cozy"),
                        density("Comfortable", "comfortable"),
                    ],
                },
                Item::Sep,
                act("Zoom in", None, Some("Ctrl+="), |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ZoomIn), cx)
                }),
                act("Zoom out", None, Some("Ctrl+-"), |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ZoomOut), cx)
                }),
                act("Actual size", None, Some("Ctrl+0"), |_, w, cx| {
                    w.dispatch_action(Box::new(crate::app::ZoomReset), cx)
                }),
            ]
        }
        3 => vec![
            act("Start all", Some("play"), None, |ws, _, cx| {
                ws.call(Request::ResumeAll, cx)
            }),
            act(
                "Stop all",
                Some("pause"),
                Some("Ctrl+Shift+P"),
                |ws, _, cx| ws.call(Request::PauseAll, cx),
            ),
            act("Stop for 1 hour", Some("clock"), None, |ws, _, cx| {
                ws.call(Request::PauseFor { secs: 3600 }, cx)
            }),
            Item::Sep,
            check(
                "Slow mode (alternative speed limits)",
                ws.model.status.slow_mode,
                Some("Ctrl+Alt+L"),
                |_, w, cx| w.dispatch_action(Box::new(crate::app::SlowMode), cx),
            ),
        ],
        4 => vec![
            act(
                "Settings…",
                Some("sliders"),
                Some("Ctrl+,"),
                |ws, _, cx| ws.open_settings(cx),
            ),
            later("RSS reader", Some("rss"), None),
            later("Statistics", Some("bar-chart"), None),
        ],
        _ => vec![
            act(
                "Keyboard shortcuts",
                Some("keyboard"),
                Some("?"),
                |ws, _, cx| {
                    ws.dialog = Some(Dialog::Shortcuts);
                    cx.notify();
                },
            ),
            act(
                format!("About tTorrent {}", env!("CARGO_PKG_VERSION")),
                Some("sprout"),
                None,
                |ws, _, cx| {
                    ws.notify(
                        format!(
                            "tTorrent {}, MIT licence. Built on librqbit and GPUI.",
                            env!("CARGO_PKG_VERSION")
                        ),
                        false,
                        cx,
                    )
                },
            ),
        ],
    }
}

fn column_items(ws: &Workspace) -> Vec<Item> {
    Column::ALL
        .iter()
        .map(|&col| {
            let shown = ws.prefs.columns.iter().any(|c| c.column == col);
            Item::Act {
                label: col.label().into(),
                icon: None,
                keys: None,
                check: Some(shown),
                danger: false,
                enabled: col != Column::Name,
                on: Rc::new(move |ws, _, cx| {
                    if shown {
                        ws.prefs.columns.retain(|c| c.column != col);
                    } else {
                        // Insert at its qBittorrent position.
                        let pos = ws
                            .prefs
                            .columns
                            .iter()
                            .position(|c| c.column > col)
                            .unwrap_or(ws.prefs.columns.len());
                        ws.prefs.columns.insert(
                            pos,
                            ColumnPref {
                                column: col,
                                width: col.default_width(),
                            },
                        );
                    }
                    ws.save_prefs(cx);
                }),
            }
        })
        .collect()
}

/// Show or hide status bar items; they keep their place in the bar.
fn status_items(ws: &Workspace) -> Vec<Item> {
    use tt_ipc::settings::StatusItem;
    let mut items: Vec<Item> = StatusItem::ALL
        .iter()
        .map(|&item| {
            let shown = ws.prefs.status_items.contains(&item);
            Item::Act {
                label: item.label().into(),
                icon: None,
                keys: None,
                check: Some(shown),
                danger: false,
                enabled: true,
                on: Rc::new(move |ws, _, cx| {
                    if shown {
                        ws.prefs.status_items.retain(|i| *i != item);
                    } else {
                        ws.prefs.status_items.push(item);
                        ws.prefs
                            .status_items
                            .sort_by_key(|i| StatusItem::ALL.iter().position(|a| a == i));
                    }
                    ws.save_prefs(cx);
                }),
            }
        })
        .collect();
    items.push(Item::Sep);
    items.push(act("Reset to defaults", None, None, |ws, _, cx| {
        ws.prefs.status_items = StatusItem::DEFAULT.to_vec();
        ws.save_prefs(cx);
    }));
    items
}

fn menu_panel(t: &Theme, width: f32, max_h: Pixels) -> gpui::Stateful<gpui::Div> {
    div()
        .id("menu-panel")
        .w(t.px(width))
        .max_h(max_h)
        .overflow_y_scroll()
        .p(t.px(5.))
        .rounded(t.px(tt_theme::radius::MENU))
        .bg(t.c.raise)
        .border_1()
        .border_color(t.c.rule)
        .shadow(pop_shadow(t))
        .text_size(t.fpx(13.))
}

fn render_items(
    ws: &Workspace,
    items: &[Item],
    depth: usize,
    max_h: Pixels,
    cx: &mut Context<Workspace>,
) -> Vec<AnyElement> {
    let t = ws.theme;
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            Item::Sep => out.push(
                div()
                    .h(px(1.))
                    .my(t.px(4.))
                    .mx(t.px(6.))
                    .bg(t.c.row_rule)
                    .into_any_element(),
            ),
            Item::Act {
                label,
                icon: ic,
                keys,
                check,
                danger,
                enabled,
                on,
            } => {
                let on = on.clone();
                let enabled = *enabled;
                let fg: Hsla = if !enabled {
                    t.c.muted.opacity(0.7)
                } else if *danger {
                    t.c.bad
                } else {
                    t.c.ink
                };
                out.push(
                    div()
                        .id(SharedString::from(format!("mi-{depth}-{i}")))
                        .h(t.px(26.))
                        .px(t.px(8.))
                        .flex()
                        .items_center()
                        .gap(t.px(9.))
                        .rounded(t.px(tt_theme::radius::ITEM))
                        .text_color(fg)
                        .when(enabled, |d| d.hover(|s| s.bg(t.c.hover)).cursor_pointer())
                        .on_hover(cx.listener(move |ws, hovered: &bool, _, cx| {
                            if *hovered && ws.sub_open.is_some_and(|(d, _)| d >= depth) {
                                ws.sub_open = None;
                                cx.notify();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |ws, _, w, cx| {
                                cx.stop_propagation();
                                if !enabled {
                                    return;
                                }
                                ws.menu = None;
                                ws.sub_open = None;
                                on(ws, w, cx);
                                cx.notify();
                            }),
                        )
                        .child(div().w(t.px(16.)).flex_none().flex().justify_center().map(
                            |d| match (check, ic) {
                                (Some(true), _) => d.child(icon("check", 14., t.c.accent_fg, &t)),
                                (Some(false), _) => d,
                                (None, Some(name)) => d.child(icon(
                                    name,
                                    14.,
                                    if *danger { t.c.bad } else { t.c.muted },
                                    &t,
                                )),
                                (None, None) => d,
                            },
                        ))
                        .child(div().flex_1().whitespace_nowrap().child(label.clone()))
                        .when_some(*keys, |d, k| {
                            d.child(
                                div()
                                    .pl(t.px(16.))
                                    .font_family(FONT_NUM)
                                    .text_size(t.fpx(11.5))
                                    .text_color(t.c.muted)
                                    .child(shortcut(k)),
                            )
                        })
                        .into_any_element(),
                );
            }
            Item::Sub { label, items } => {
                let open = ws.sub_open.is_some_and(|(d, ix)| d == depth && ix == i);
                let n = items.len();
                out.push(
                    div()
                        .id(SharedString::from(format!("ms-{depth}-{i}")))
                        .h(t.px(26.))
                        .px(t.px(8.))
                        .flex()
                        .items_center()
                        .gap(t.px(9.))
                        .rounded(t.px(tt_theme::radius::ITEM))
                        .cursor_pointer()
                        .when(open, |d| d.bg(t.c.hover))
                        .hover(|s| s.bg(t.c.hover))
                        .on_hover(cx.listener(move |ws, hovered: &bool, _, cx| {
                            if *hovered {
                                ws.sub_open = Some((depth, i));
                                cx.notify();
                            }
                        }))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |ws, _, _, cx| {
                                ws.sub_open = Some((depth, i));
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                        .child(div().w(t.px(16.)).flex_none())
                        .child(div().flex_1().whitespace_nowrap().child(label.clone()))
                        .child(icon("chevron-right", 13., t.c.muted, &t))
                        .when(open && n > 0, |d| {
                            let sub = render_items(ws, items, depth + 1, max_h, cx);
                            d.child(
                                deferred(
                                    anchored()
                                        .snap_to_window_with_margin(px(8.))
                                        .anchor(gpui::Corner::TopLeft)
                                        .offset(point(t.px(214.), t.px(-6.)))
                                        .child(menu_panel(&t, 230., max_h).children(sub)),
                                )
                                .with_priority(2),
                            )
                        })
                        .into_any_element(),
                );
            }
        }
    }
    out
}

pub fn render_menu(
    ws: &mut Workspace,
    menu: Menu,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let above_sheets = matches!(menu, Menu::Pick { .. });
    let (items, at, width) = match menu {
        Menu::Pick { kind, at } => (pick_items(ws, kind), at, 240.),
        Menu::Torrent { at } => (torrent_items(ws), at, 292.),
        Menu::Columns { at } => (column_items(ws), at, 220.),
        Menu::StatusBar { at } => (status_items(ws), at, 220.),
        Menu::Bar { index, .. } => {
            // Under its title: logo, then each title is about 50 px.
            let x = 38. + 52. * index as f32;
            (
                bar_items(ws, index),
                point(t.px(x), t.px(t.m.title_bar - 2.)),
                260.,
            )
        }
    };
    let max_h = window.viewport_size().height - px(16.);
    let rows = render_items(ws, &items, 0, max_h, cx);
    deferred(
        div()
            .id("menu-scrim")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|ws, _, _, cx| {
                    ws.menu = None;
                    ws.sub_open = None;
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|ws, _, _, cx| {
                    ws.menu = None;
                    ws.sub_open = None;
                    cx.notify();
                }),
            )
            .child(
                anchored()
                    .position(at)
                    .snap_to_window_with_margin(px(8.))
                    .child(
                        menu_panel(&t, width, max_h)
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .children(rows),
                    ),
            ),
    )
    .with_priority(if above_sheets { 4 } else { 1 })
    .into_any_element()
}

pub fn toast(t: &Theme, text: SharedString, error: bool) -> AnyElement {
    deferred(
        div()
            .absolute()
            .bottom(t.px(t.m.status_bar + 14.))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                div()
                    .max_w(t.px(560.))
                    .flex()
                    .items_center()
                    .gap(t.px(10.))
                    .px(t.px(14.))
                    .py(t.px(9.))
                    .rounded(t.px(tt_theme::radius::TOAST))
                    .bg(t.c.raise)
                    .border_1()
                    .border_color(if error { t.c.bad } else { t.c.rule })
                    .shadow(pop_shadow(t))
                    .child(icon(
                        if error { "alert" } else { "check" },
                        15.,
                        if error { t.c.bad } else { t.c.ok },
                        t,
                    ))
                    .child(text),
            ),
    )
    .with_priority(3)
    .into_any_element()
}

pub fn button(
    t: &Theme,
    id: &'static str,
    label: &'static str,
    kind: u8,
) -> gpui::Stateful<gpui::Div> {
    // kind: 0 plain, 1 primary, 2 danger
    div()
        .id(id)
        .h(t.px(30.))
        .px(t.px(14.))
        .flex()
        .items_center()
        .rounded(t.px(tt_theme::radius::BUTTON))
        .cursor_pointer()
        .map(|d| match kind {
            1 => d
                .bg(t.c.accent)
                .text_color(t.c.accent_ink)
                .font_weight(FontWeight::SEMIBOLD)
                .hover(|s| s.bg(t.c.accent_hover)),
            2 => d
                .bg(t.c.bad)
                .text_color(t.c.accent_ink)
                .font_weight(FontWeight::SEMIBOLD),
            _ => d
                .bg(t.c.raise)
                .border_1()
                .border_color(t.c.rule)
                .hover(|s| s.bg(t.c.hover)),
        })
        .child(label)
}

pub fn sheet(t: &Theme, width: f32, title: impl Into<SharedString>) -> gpui::Stateful<gpui::Div> {
    div()
        .id("sheet")
        .w(t.px(width))
        .max_h(gpui::relative(0.88))
        .overflow_y_scroll()
        .rounded(t.px(tt_theme::radius::MENU))
        .bg(t.c.raise)
        .border_1()
        .border_color(t.c.rule)
        .shadow(pop_shadow(t))
        .p(t.px(20.))
        .flex()
        .flex_col()
        .gap(t.px(14.))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .font_family(FONT_HEAD)
                .font_weight(FontWeight::BOLD)
                .text_size(t.fpx(16.))
                .child(title.into()),
        )
}

pub fn field(t: &Theme, input: gpui::Entity<crate::text_input::TextInput>) -> gpui::Div {
    div()
        .h(t.px(34.))
        .px(t.px(10.))
        .flex()
        .items_center()
        .rounded(t.px(tt_theme::radius::INPUT))
        .bg(t.c.well)
        .border_1()
        .border_color(t.c.accent_line)
        .child(input)
}

pub fn render_dialog(
    ws: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let Some(dialog) = ws.dialog.clone() else {
        return div().into_any_element();
    };
    let body: AnyElement = match dialog {
        Dialog::Magnet => sheet(&t, 520., "Add magnet link")
            .child(div().text_color(t.c.muted).child(
                "Paste a magnet link or a link to a .torrent file. One per line adds several.",
            ))
            .child(field(&t, ws.magnet.clone()))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(t.px(8.))
                    .child(button(&t, "cancel", "Cancel", 0).on_click(cx.listener(
                        |ws, _, _, cx| {
                            ws.dialog = None;
                            cx.notify();
                        },
                    )))
                    .child(
                        button(&t, "ok", "Add", 1).on_click(cx.listener(|ws, _, _, cx| {
                            let text = ws.magnet.read(cx).text.clone();
                            if !text.trim().is_empty() {
                                ws.add_link(&text, cx);
                                ws.magnet.update(cx, |i, cx| i.set("", cx));
                            }
                            ws.dialog = None;
                            cx.notify();
                        })),
                    ),
            )
            .into_any_element(),
        Dialog::Rename { id } => sheet(&t, 520., "Rename torrent")
            .child(field(&t, ws.magnet.clone()))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(t.px(8.))
                    .child(button(&t, "cancel", "Cancel", 0).on_click(cx.listener(
                        |ws, _, _, cx| {
                            ws.dialog = None;
                            cx.notify();
                        },
                    )))
                    .child(button(&t, "ok", "Rename", 1).on_click(cx.listener(
                        move |ws, _, _, cx| {
                            let name = ws.magnet.read(cx).text.clone();
                            ws.call(Request::Rename { id, name }, cx);
                            ws.magnet.update(cx, |i, cx| i.set("", cx));
                            ws.dialog = None;
                            cx.notify();
                        },
                    ))),
            )
            .into_any_element(),
        Dialog::Remove { ids, delete_files } => {
            let names: Vec<String> = ids
                .iter()
                .filter_map(|id| ws.model.rows.get(id).map(|r| r.name.clone()))
                .take(3)
                .collect();
            let more = ids.len().saturating_sub(names.len());
            let title = if ids.len() == 1 {
                "Remove this torrent?".to_string()
            } else {
                format!("Remove {} torrents?", ids.len())
            };
            let ids2 = ids.clone();
            sheet(&t, 480., title)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(t.px(2.))
                        .children(
                            names
                                .into_iter()
                                .map(|n| div().truncate().text_color(t.c.ink_2).child(n)),
                        )
                        .when(more > 0, |d| {
                            d.child(
                                div()
                                    .text_color(t.c.muted)
                                    .child(format!("and {more} more")),
                            )
                        }),
                )
                .child(
                    div()
                        .id("del-files")
                        .flex()
                        .items_center()
                        .gap(t.px(8.))
                        .cursor_pointer()
                        .on_click(cx.listener(move |ws, _, _, cx| {
                            if let Some(Dialog::Remove { delete_files, .. }) = ws.dialog.as_mut() {
                                *delete_files = !*delete_files;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .size(t.px(16.))
                                .rounded(t.px(4.))
                                .border_1()
                                .border_color(if delete_files { t.c.bad } else { t.c.rule })
                                .when(delete_files, |d| d.bg(t.c.bad))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(delete_files, |d| {
                                    d.child(icon("check", 12., t.c.accent_ink, &t))
                                }),
                        )
                        .child("Also delete the files on disk"),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(t.px(8.))
                        .child(button(&t, "cancel", "Cancel", 0).on_click(cx.listener(
                            |ws, _, _, cx| {
                                ws.dialog = None;
                                cx.notify();
                            },
                        )))
                        .child(button(&t, "ok", "Remove", 2).on_click(cx.listener(
                            move |ws, _, _, cx| {
                                ws.call(
                                    Request::Remove {
                                        ids: ids2.clone(),
                                        delete_files,
                                    },
                                    cx,
                                );
                                ws.dialog = None;
                                cx.notify();
                            },
                        ))),
                )
                .into_any_element()
        }
        Dialog::Shortcuts => shortcuts_sheet(&t).into_any_element(),
        Dialog::Settings => crate::settings::settings_sheet(ws, window, cx),
    };
    deferred(
        div()
            .id("scrim")
            .absolute()
            .inset_0()
            .bg(t.c.shadow.opacity(0.9))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|ws, _, w, cx| {
                    ws.dialog = None;
                    w.focus(&ws.focus);
                    cx.notify();
                }),
            )
            .child(body),
    )
    .with_priority(2)
    .into_any_element()
}

const SHORTCUTS: [(&str, &str); 22] = [
    ("Add torrent file", "Ctrl+O"),
    ("Add magnet link", "Ctrl+Shift+O"),
    ("Start / Stop", "Ctrl+S / Ctrl+P"),
    ("Start or stop selected", "Space"),
    ("Stop all / start all", "Ctrl+Shift+P"),
    ("Remove / with files", "Delete / Shift+Delete"),
    ("Rename", "F2"),
    ("Open containing folder", "Ctrl+Enter"),
    ("Copy magnet link", "Ctrl+Shift+C"),
    ("Force recheck", "Ctrl+R"),
    ("Select all", "Ctrl+A"),
    ("Filter torrents", "Ctrl+F"),
    ("Command palette", "Ctrl+K"),
    ("Sidebar / Details", "Ctrl+B / Ctrl+J"),
    ("Details tab 1 to 6", "Ctrl+1 … 6"),
    ("Zoom in / out / actual size", "Ctrl+= / Ctrl+- / Ctrl+0"),
    ("Slow mode", "Ctrl+Alt+L"),
    ("Settings", "Ctrl+,"),
    ("Close window (keeps running)", "Ctrl+W"),
    ("Quit", "Ctrl+Q"),
    ("Shortcuts", "?"),
    ("Torrent menu", "Shift+F10"),
];

fn shortcuts_sheet(t: &Theme) -> gpui::Stateful<gpui::Div> {
    sheet(t, 520., "Keyboard shortcuts").children(SHORTCUTS.iter().map(|(what, keys)| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(t.px(28.))
            .border_b_1()
            .border_color(t.c.row_rule)
            .child(div().text_color(t.c.ink_2).child(*what))
            .child(kbd(shortcut(keys), t))
    }))
}

pub(crate) fn seg<'a>(
    t: &Theme,
    id: &'static str,
    options: &'a [(&'static str, &'static str)],
    current: &str,
    cx: &mut Context<Workspace>,
    on: fn(&mut Workspace, &'static str),
) -> gpui::Div {
    div()
        .flex()
        .p(t.px(2.))
        .gap(t.px(2.))
        .rounded(t.px(tt_theme::radius::BUTTON))
        .bg(t.c.well)
        .border_1()
        .border_color(t.c.rule)
        .children(options.iter().map(|(label, key)| {
            let sel = *key == current;
            let key: &'static str = key;
            div()
                .id(SharedString::from(format!("{id}-{key}")))
                .px(t.px(10.))
                .h(t.px(26.))
                .flex()
                .items_center()
                .rounded(t.px(8.))
                .cursor_pointer()
                .when(sel, |d| {
                    d.bg(t.c.raise)
                        .text_color(t.c.ink)
                        .shadow(crate::widgets::moulded(t))
                })
                .when(!sel, |d| {
                    d.text_color(t.c.muted).hover(|s| s.text_color(t.c.ink))
                })
                .on_click(cx.listener(move |ws, _, _, cx| {
                    on(ws, key);
                    ws.save_prefs(cx);
                }))
                .child(*label)
        }))
}

pub(crate) fn toggle(
    t: &Theme,
    id: &'static str,
    on: bool,
    cx: &mut Context<Workspace>,
    f: fn(&mut tt_ipc::EngineSettings),
) -> AnyElement {
    div()
        .id(id)
        .w(t.px(34.))
        .h(t.px(20.))
        .flex_none()
        .rounded_full()
        .p(t.px(2.))
        .flex()
        .when(on, |d| d.bg(t.c.accent).justify_end())
        .when(!on, |d| d.bg(t.c.track))
        .cursor_pointer()
        .on_click(cx.listener(move |ws, _, _, cx| {
            if let Some(mut s) = ws.settings.clone() {
                f(&mut s);
                ws.set_engine_settings(s, cx);
            }
        }))
        .child(div().size(t.px(16.)).rounded_full().bg(t.c.raise))
        .into_any_element()
}

pub(crate) fn stepper(
    t: &Theme,
    id: &'static str,
    value: String,
    cx: &mut Context<Workspace>,
    f: fn(&mut tt_ipc::EngineSettings, i64),
) -> AnyElement {
    let b = |sid: &'static str, label: &'static str, d: i64, cx: &mut Context<Workspace>| {
        div()
            .id(sid)
            .size(t.px(24.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(t.px(6.))
            .bg(t.c.raise)
            .border_1()
            .border_color(t.c.rule)
            .cursor_pointer()
            .hover(|s| s.bg(t.c.hover))
            .on_click(cx.listener(move |ws, _, _, cx| {
                if let Some(mut s) = ws.settings.clone() {
                    f(&mut s, d);
                    ws.set_engine_settings(s, cx);
                }
            }))
            .child(label)
    };
    let _ = id;
    div()
        .flex()
        .items_center()
        .gap(t.px(6.))
        .child(b(concat_id(id, "-"), "−", -1, cx))
        .child(
            div()
                .min_w(t.px(84.))
                .text_center()
                .font_family(FONT_NUM)
                .text_size(t.fpx(12.5))
                .child(value),
        )
        .child(b(concat_id(id, "+"), "+", 1, cx))
        .into_any_element()
}

pub(crate) fn assoc_state(t: &Theme, ours: Option<bool>) -> AnyElement {
    let (text, col) = match ours {
        Some(true) => ("tTorrent", t.c.ok),
        Some(false) => ("Another app", t.c.muted),
        None => ("Checking…", t.c.muted),
    };
    div().text_color(col).child(text).into_any_element()
}

/// What "Make tTorrent the default" does on this system.
pub(crate) const ASSOC_HINT: &str = if cfg!(windows) {
    "Windows lets only you change defaults: Settings › Default apps opens on tTorrent"
} else if cfg!(target_os = "macos") {
    "macOS may ask you to confirm"
} else {
    "Sets tTorrent for this user (xdg-mime)"
};

fn concat_id(id: &'static str, suffix: &'static str) -> &'static str {
    // Small fixed set of ids; leaking a few bytes once per id is fine.
    Box::leak(format!("{id}{suffix}").into_boxed_str())
}

/// qBittorrent's ∞ is -1.
pub(crate) fn active_label(v: i32) -> String {
    if v < 0 { "∞".into() } else { v.to_string() }
}

/// Down from 0 goes to ∞; up from ∞ goes to 1.
pub(crate) fn active_step(v: i32, d: i64) -> i32 {
    match (v, d) {
        (v, d) if v < 0 && d > 0 => 1,
        (v, _) if v < 0 => -1,
        (0, d) if d < 0 => -1,
        (v, d) => (i64::from(v) + d).clamp(0, 99_999) as i32,
    }
}

pub(crate) fn ratio_step(v: Option<f32>, d: i64) -> Option<f32> {
    match v {
        None if d > 0 => Some(1.0),
        None => None,
        Some(r) => {
            let n = r + d as f32 * 0.25;
            (n >= 0.05).then_some((n * 100.0).round() / 100.0)
        }
    }
}

const MINUTE_STEPS: [u32; 10] = [30, 60, 120, 360, 720, 1440, 2880, 4320, 10080, 43200];

pub(crate) fn minutes_label(v: Option<u32>) -> String {
    match v {
        None => "Off".into(),
        Some(m) if m % 1440 == 0 => format!("{} d", m / 1440),
        Some(m) if m % 60 == 0 => format!("{} h", m / 60),
        Some(m) => format!("{m} min"),
    }
}

pub(crate) fn minutes_step(v: Option<u32>, d: i64) -> Option<u32> {
    let i = match v {
        None => -1,
        Some(m) => MINUTE_STEPS
            .iter()
            .position(|&s| s >= m)
            .unwrap_or(MINUTE_STEPS.len() - 1) as i64,
    };
    let n = (i + d).clamp(-1, MINUTE_STEPS.len() as i64 - 1);
    usize::try_from(n).ok().map(|n| MINUTE_STEPS[n])
}

pub(crate) fn limit_step(v: u64, d: i64) -> u64 {
    // Steps through common limits in KiB/s: off, 64, 128 … 64 MiB/s.
    const STEPS: [u64; 12] = [
        0, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
    ];
    let kib = v / 1024;
    let i = STEPS
        .iter()
        .position(|&s| s >= kib)
        .unwrap_or(STEPS.len() - 1) as i64;
    STEPS[(i + d).clamp(0, STEPS.len() as i64 - 1) as usize] * 1024
}

#[cfg(test)]
mod tests {
    use super::limit_step;

    #[test]
    fn active_limits_reach_infinity() {
        use super::{active_label, active_step};
        assert_eq!(active_step(0, -1), -1);
        assert_eq!(active_label(-1), "∞");
        assert_eq!(active_step(-1, 1), 1);
        assert_eq!(active_step(3, 1), 4);
    }

    #[test]
    fn seeding_presets() {
        use super::{minutes_label, minutes_step, ratio_step};
        assert_eq!(ratio_step(None, 1), Some(1.0));
        assert_eq!(ratio_step(Some(0.25), -1), None);
        assert_eq!(minutes_step(None, 1), Some(30));
        assert_eq!(minutes_step(Some(30), -1), None);
        assert_eq!(minutes_label(Some(1440)), "1 d");
        assert_eq!(minutes_label(Some(120)), "2 h");
    }

    #[test]
    fn limits_step_through_presets() {
        assert_eq!(limit_step(0, 1), 64 * 1024);
        assert_eq!(limit_step(64 * 1024, -1), 0);
        assert_eq!(limit_step(0, -1), 0);
        assert_eq!(limit_step(65536 * 1024, 1), 65536 * 1024);
    }
}
