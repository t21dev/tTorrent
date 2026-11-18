//! Settings (DESIGN.md › Settings): a sheet with a section list and a search
//! field on the left, the section's groups on the right. Search looks through
//! every section's labels and hints. Changes apply at once; there is no Save.

use crate::app::Workspace;
use crate::overlay::{
    ASSOC_HINT, Pick, active_label, active_step, assoc_state, button, dropdown, field, limit_step,
    minutes_label, minutes_step, ratio_step, seg, stepper, stop_label, toggle,
};
use crate::theme::{FONT_HEAD, FONT_NUM, Theme};
use crate::widgets::{icon, pop_shadow};
use gpui::{
    AnyElement, Context, FontWeight, Hsla, IntoElement, MouseButton, ParentElement,
    PathPromptOptions, SharedString, Styled, Window, div, prelude::*,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Behaviour,
    Downloads,
    Connection,
    Speed,
    Queue,
    BitTorrent,
    Files,
    Appearance,
    Agents,
}

impl Section {
    pub const ALL: [Section; 9] = [
        Section::Behaviour,
        Section::Downloads,
        Section::Connection,
        Section::Speed,
        Section::Queue,
        Section::BitTorrent,
        Section::Files,
        Section::Appearance,
        Section::Agents,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Behaviour => "Behaviour",
            Section::Downloads => "Downloads",
            Section::Connection => "Connection",
            Section::Speed => "Speed",
            Section::Queue => "Queue and seeding",
            Section::BitTorrent => "BitTorrent",
            Section::Files => "File association",
            Section::Appearance => "Appearance",
            Section::Agents => "AI agents",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::Behaviour => "sliders",
            Section::Downloads => "arrow-down",
            Section::Connection => "plug",
            Section::Speed => "gauge",
            Section::Queue => "list",
            Section::BitTorrent => "globe",
            Section::Files => "file",
            Section::Appearance => "sun",
            Section::Agents => "chip",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Section::Behaviour => "How tTorrent starts, closes and confirms.",
            Section::Downloads => "Where torrents are saved and what happens when you add one.",
            Section::Connection => "The port other peers reach you on.",
            Section::Speed => "Global limits, and the lower ones slow mode switches to.",
            Section::Queue => "How many torrents run at once, and when finished ones stop sharing.",
            Section::BitTorrent => "Ways to find peers, and how many to talk to.",
            Section::Files => "Which app opens .torrent files and magnet links.",
            Section::Appearance => "Mode, surfaces, accent, density and size.",
            Section::Agents => "Let AI agents on this computer see and manage torrents.",
        }
    }
}

struct Row {
    label: &'static str,
    hint: Option<&'static str>,
    control: AnyElement,
    /// Indented under the row before it (a sub-setting).
    sub: bool,
}

struct Group {
    title: &'static str,
    rows: Vec<Row>,
}

fn row(label: &'static str, hint: Option<&'static str>, control: AnyElement) -> Row {
    Row {
        label,
        hint,
        control,
        sub: false,
    }
}

fn sub(label: &'static str, hint: Option<&'static str>, control: AnyElement) -> Row {
    Row {
        label,
        hint,
        control,
        sub: true,
    }
}

fn mono(t: &Theme, text: impl Into<SharedString>) -> AnyElement {
    div()
        .font_family(FONT_NUM)
        .text_size(t.fpx(12.))
        .child(text.into())
        .into_any_element()
}

fn speed_limit(v: u64) -> String {
    if v == 0 {
        "Unlimited".into()
    } else {
        tt_ipc::fmt::speed(v)
    }
}

fn groups(ws: &Workspace, section: Section, cx: &mut Context<Workspace>) -> Vec<Group> {
    let t = ws.theme;
    let p = ws.prefs.clone();
    let Some(s) = ws.settings.clone() else {
        return Vec::new();
    };
    match section {
        Section::Behaviour => vec![Group {
            title: "General",
            rows: vec![
                row(
                    "Start torrents gradually",
                    Some("Start in waves after launch so the disk doesn't spike"),
                    toggle(&t, "t-grad", s.start_gradually, cx, |s| {
                        s.start_gradually = !s.start_gradually
                    }),
                ),
                row(
                    "Close to the tray",
                    Some("Closing the window keeps torrents running"),
                    toggle(&t, "t-tray", s.close_to_tray, cx, |s| {
                        s.close_to_tray = !s.close_to_tray
                    }),
                ),
                row(
                    "Ask before removing torrents",
                    None,
                    toggle(&t, "t-confirm", s.confirm_remove, cx, |s| {
                        s.confirm_remove = !s.confirm_remove
                    }),
                ),
            ],
        }],
        Section::Downloads => vec![
            Group {
                title: "Saving",
                rows: vec![row(
                    "Default save folder",
                    Some("New torrents go here unless their category has a folder"),
                    div()
                        .flex()
                        .items_center()
                        .gap(t.px(8.))
                        .child(
                            div()
                                .max_w(t.px(260.))
                                .truncate()
                                .text_color(t.c.ink_2)
                                .child(if s.default_save_path.is_empty() {
                                    "Downloads".to_owned()
                                } else {
                                    s.default_save_path.clone()
                                }),
                        )
                        .child(
                            button(&t, "s-browse", "Browse…", 0)
                                .on_click(cx.listener(|ws, _, _, cx| ws.browse_default_save(cx))),
                        )
                        .into_any_element(),
                )],
            },
            Group {
                title: "When adding a torrent",
                rows: vec![
                    row(
                        "Show the Add torrent dialog",
                        Some("Choose files, folder and options for each torrent you open"),
                        toggle(&t, "t-adddlg", s.show_add_dialog, cx, |s| {
                            s.show_add_dialog = !s.show_add_dialog
                        }),
                    ),
                    row(
                        "Bring the dialog to the front",
                        None,
                        toggle(&t, "t-addfront", s.add_dialog_front, cx, |s| {
                            s.add_dialog_front = !s.add_dialog_front
                        }),
                    ),
                    row(
                        "Don't start downloads automatically",
                        None,
                        toggle(&t, "t-paused", s.start_paused, cx, |s| {
                            s.start_paused = !s.start_paused
                        }),
                    ),
                    row(
                        "Add to the top of the queue",
                        None,
                        toggle(&t, "t-top", s.add_to_top, cx, |s| {
                            s.add_to_top = !s.add_to_top
                        }),
                    ),
                    row(
                        "Content layout",
                        Some("Original keeps a folder for multi-file torrents"),
                        dropdown(&t, "s-layout", s.content_layout.label(), Pick::Layout, cx)
                            .w(t.px(200.))
                            .into_any_element(),
                    ),
                    row(
                        "Stop torrent when",
                        None,
                        dropdown(&t, "s-stop", stop_label(s.stop_condition), Pick::Stop, cx)
                            .w(t.px(200.))
                            .into_any_element(),
                    ),
                ],
            },
        ],
        Section::Connection => vec![Group {
            title: "Listening",
            rows: vec![
                row(
                    "Listening port",
                    Some("Takes effect after a restart"),
                    mono(&t, s.listen_port.to_string()),
                ),
                row(
                    "UPnP / NAT-PMP port forwarding",
                    Some("Ask the router to open the port"),
                    toggle(&t, "t-upnp", s.upnp, cx, |s| s.upnp = !s.upnp),
                ),
            ],
        }],
        Section::Speed => vec![
            Group {
                title: "Global limits",
                rows: vec![
                    row(
                        "Download limit",
                        None,
                        stepper(&t, "dl", speed_limit(s.down_limit), cx, |s, d| {
                            s.down_limit = limit_step(s.down_limit, d)
                        }),
                    ),
                    row(
                        "Upload limit",
                        None,
                        stepper(&t, "ul", speed_limit(s.up_limit), cx, |s, d| {
                            s.up_limit = limit_step(s.up_limit, d)
                        }),
                    ),
                ],
            },
            Group {
                title: "Slow mode",
                rows: vec![
                    row(
                        "Download limit in slow mode",
                        Some("Ctrl+Alt+L or the toolbar turns slow mode on"),
                        stepper(&t, "adl", speed_limit(s.alt_down_limit), cx, |s, d| {
                            s.alt_down_limit = limit_step(s.alt_down_limit, d)
                        }),
                    ),
                    row(
                        "Upload limit in slow mode",
                        None,
                        stepper(&t, "aul", speed_limit(s.alt_up_limit), cx, |s, d| {
                            s.alt_up_limit = limit_step(s.alt_up_limit, d)
                        }),
                    ),
                ],
            },
        ],
        Section::Queue => {
            let mut queue = vec![row(
                "Torrent queueing",
                Some("Limit how many torrents run at once; the rest wait their turn"),
                toggle(&t, "t-q", s.queueing, cx, |s| s.queueing = !s.queueing),
            )];
            if s.queueing {
                queue.extend([
                    sub(
                        "Maximum active downloads",
                        None,
                        stepper(
                            &t,
                            "mad",
                            active_label(s.max_active_downloads),
                            cx,
                            |s, d| s.max_active_downloads = active_step(s.max_active_downloads, d),
                        ),
                    ),
                    sub(
                        "Maximum active uploads",
                        None,
                        stepper(&t, "mau", active_label(s.max_active_uploads), cx, |s, d| {
                            s.max_active_uploads = active_step(s.max_active_uploads, d)
                        }),
                    ),
                    sub(
                        "Maximum active torrents",
                        None,
                        stepper(
                            &t,
                            "mat",
                            active_label(s.max_active_torrents),
                            cx,
                            |s, d| s.max_active_torrents = active_step(s.max_active_torrents, d),
                        ),
                    ),
                    sub(
                        "Don't count slow torrents in these limits",
                        Some("Slower than both rates below for the inactivity time"),
                        toggle(&t, "t-slow", s.ignore_slow_torrents, cx, |s| {
                            s.ignore_slow_torrents = !s.ignore_slow_torrents
                        }),
                    ),
                ]);
                if s.ignore_slow_torrents {
                    queue.extend([
                        sub(
                            "Download rate below",
                            None,
                            stepper(
                                &t,
                                "sdr",
                                format!("{} KiB/s", s.slow_down_kib),
                                cx,
                                |s, d| {
                                    s.slow_down_kib =
                                        (i64::from(s.slow_down_kib) + d).clamp(1, 2_000_000) as u32
                                },
                            ),
                        ),
                        sub(
                            "Upload rate below",
                            None,
                            stepper(&t, "sur", format!("{} KiB/s", s.slow_up_kib), cx, |s, d| {
                                s.slow_up_kib =
                                    (i64::from(s.slow_up_kib) + d).clamp(1, 2_000_000) as u32
                            }),
                        ),
                        sub(
                            "For at least",
                            Some("qBittorrent's torrent inactivity timer"),
                            stepper(&t, "sit", format!("{} s", s.slow_timer_secs), cx, |s, d| {
                                s.slow_timer_secs =
                                    (i64::from(s.slow_timer_secs) + d * 10).clamp(1, 999_999) as u32
                            }),
                        ),
                    ]);
                }
            }
            vec![
                Group {
                    title: "Torrent queueing",
                    rows: queue,
                },
                Group {
                    title: "Seeding limits",
                    rows: vec![
                        row(
                            "When ratio reaches",
                            None,
                            stepper(
                                &t,
                                "srt",
                                s.ratio_limit
                                    .map(|r| format!("{r:.2}"))
                                    .unwrap_or_else(|| "Off".into()),
                                cx,
                                |s, d| s.ratio_limit = ratio_step(s.ratio_limit, d),
                            ),
                        ),
                        row(
                            "When total seeding time reaches",
                            None,
                            stepper(
                                &t,
                                "sst",
                                minutes_label(s.seeding_time_limit_mins),
                                cx,
                                |s, d| {
                                    s.seeding_time_limit_mins =
                                        minutes_step(s.seeding_time_limit_mins, d)
                                },
                            ),
                        ),
                        row(
                            "When inactive seeding time reaches",
                            None,
                            stepper(
                                &t,
                                "sist",
                                minutes_label(s.inactive_seeding_time_limit_mins),
                                cx,
                                |s, d| {
                                    s.inactive_seeding_time_limit_mins =
                                        minutes_step(s.inactive_seeding_time_limit_mins, d)
                                },
                            ),
                        ),
                        row(
                            "Apply when",
                            None,
                            dropdown(
                                &t,
                                "s-slm",
                                match s.share_limits_mode {
                                    tt_ipc::ShareLimitsMode::MatchAny => "Any limit is reached",
                                    tt_ipc::ShareLimitsMode::MatchAll => "All limits are reached",
                                },
                                Pick::ShareMode,
                                cx,
                            )
                            .w(t.px(220.))
                            .into_any_element(),
                        ),
                        row(
                            "Then",
                            Some("Remove torrent and its files deletes the downloaded data"),
                            dropdown(
                                &t,
                                "s-sla",
                                s.share_limit_action.label(),
                                Pick::ShareAction,
                                cx,
                            )
                            .w(t.px(220.))
                            .into_any_element(),
                        ),
                    ],
                },
            ]
        }
        Section::BitTorrent => vec![Group {
            title: "Peers",
            rows: vec![
                row(
                    "DHT",
                    Some("Find peers without trackers"),
                    toggle(&t, "t-dht", s.dht, cx, |s| s.dht = !s.dht),
                ),
                row(
                    "Local peer discovery",
                    Some("Find peers on your own network"),
                    toggle(&t, "t-lsd", s.lsd, cx, |s| s.lsd = !s.lsd),
                ),
                row(
                    "Connections per torrent",
                    None,
                    stepper(
                        &t,
                        "cpt",
                        s.max_connections_per_torrent.to_string(),
                        cx,
                        |s, d| {
                            s.max_connections_per_torrent =
                                (s.max_connections_per_torrent as i64 + d * 10).clamp(10, 2000)
                                    as u32
                        },
                    ),
                ),
            ],
        }],
        Section::Files => {
            let mut rows = vec![
                row(
                    "Opens .torrent files",
                    None,
                    assoc_state(&t, ws.assoc.map(|a| a.torrent_files)),
                ),
                row(
                    "Opens magnet links",
                    None,
                    assoc_state(&t, ws.assoc.map(|a| a.magnet_links)),
                ),
                row(
                    "Make tTorrent the default",
                    Some(ASSOC_HINT),
                    button(
                        &t,
                        "assoc-make",
                        "Make default…",
                        if ws.assoc.is_some_and(|a| a.torrent_files && a.magnet_links) {
                            0
                        } else {
                            1
                        },
                    )
                    .on_click(cx.listener(|ws, _, _, cx| ws.make_default(cx)))
                    .into_any_element(),
                ),
            ];
            if let Some(note) = ws.assoc_note.clone() {
                rows.push(row(
                    "",
                    None,
                    div().text_color(t.c.ink_2).child(note).into_any_element(),
                ));
            }
            vec![Group {
                title: "Default app",
                rows,
            }]
        }
        Section::Appearance => {
            let accents: Vec<AnyElement> = ["green", "blue", "violet", "amber", "rose", "ink"]
                .iter()
                .map(|&a| {
                    let col = tt_theme::Palette::new(t.mode, t.tint, crate::theme::parse_accent(a))
                        .accent;
                    let col: Hsla = gpui::Rgba {
                        r: col.r,
                        g: col.g,
                        b: col.b,
                        a: 1.0,
                    }
                    .into();
                    let sel = p.accent == a;
                    div()
                        .id(SharedString::from(format!("acc-{a}")))
                        .size(t.px(22.))
                        .rounded_full()
                        .bg(col)
                        .cursor_pointer()
                        .when(sel, |d| d.border_2().border_color(t.c.ink))
                        .on_click(cx.listener(move |ws, _, _, cx| {
                            ws.prefs.accent = a.into();
                            ws.save_prefs(cx);
                        }))
                        .into_any_element()
                })
                .collect();
            vec![Group {
                title: "Look",
                rows: vec![
                    row(
                        "Mode",
                        None,
                        seg(
                            &t,
                            "mode",
                            &[("Light", "light"), ("Dark", "dark")],
                            &p.mode,
                            cx,
                            |ws, k| ws.prefs.mode = k.into(),
                        )
                        .into_any_element(),
                    ),
                    row(
                        "Surfaces",
                        Some("Neutral greys, or the warm Dose paper"),
                        seg(
                            &t,
                            "tint",
                            &[("Neutral", "neutral"), ("Tinted", "tinted")],
                            &p.tint,
                            cx,
                            |ws, k| ws.prefs.tint = k.into(),
                        )
                        .into_any_element(),
                    ),
                    row(
                        "Accent",
                        None,
                        div()
                            .flex()
                            .gap(t.px(8.))
                            .children(accents)
                            .into_any_element(),
                    ),
                    row(
                        "Density",
                        None,
                        seg(
                            &t,
                            "density",
                            &[
                                ("Compact", "compact"),
                                ("Cozy", "cozy"),
                                ("Comfortable", "comfortable"),
                            ],
                            &p.density,
                            cx,
                            |ws, k| ws.prefs.density = k.into(),
                        )
                        .into_any_element(),
                    ),
                    row(
                        "Interface size",
                        Some("Ctrl+= and Ctrl+- also change it"),
                        mono(&t, format!("{:.0}%", p.scale * 100.0)),
                    ),
                ],
            }]
        }
        Section::Agents => vec![Group {
            title: "MCP server",
            rows: vec![row(
                "Allow AI agents to connect",
                Some("Off by default. Agents can never delete files"),
                toggle(&t, "t-mcp", s.mcp_enabled, cx, |s| {
                    s.mcp_enabled = !s.mcp_enabled
                }),
            )],
        }],
    }
}

impl Workspace {
    fn browse_default_save(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Default save folder".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(p) = paths.into_iter().next()
            {
                let _ = this.update(cx, |ws, cx| {
                    if let Some(mut s) = ws.settings.clone() {
                        s.default_save_path = p.to_string_lossy().into_owned();
                        ws.set_engine_settings(s, cx);
                    }
                });
            }
        })
        .detach();
    }
}

fn group_view(t: &Theme, g: Group) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(t.px(8.))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.c.ink)
                .child(g.title),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(t.px(tt_theme::radius::MENU))
                .border_1()
                .border_color(t.c.rule)
                .bg(t.c.panel)
                .children(g.rows.into_iter().enumerate().map(|(i, r)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(t.px(16.))
                        .min_h(t.px(46.))
                        .py(t.px(8.))
                        .pr(t.px(16.))
                        .pl(t.px(if r.sub { 36. } else { 16. }))
                        .when(i > 0, |d| d.border_t_1().border_color(t.c.row_rule))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(t.px(2.))
                                .when(!r.label.is_empty(), |d| {
                                    d.child(div().text_color(t.c.ink).child(r.label))
                                })
                                .when_some(r.hint, |d, h| {
                                    d.child(
                                        div().text_size(t.fpx(12.)).text_color(t.c.muted).child(h),
                                    )
                                }),
                        )
                        .child(div().flex_none().child(r.control))
                }))
                .when(false, |d| d),
        )
}

pub fn settings_sheet(
    ws: &mut Workspace,
    _window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    if ws.settings.is_none() {
        return div().into_any_element();
    }
    let query = ws.settings_search.read(cx).text.trim().to_lowercase();
    let current = ws.settings_section;

    // Search looks through every section; otherwise the chosen one.
    let (title, blurb, groups): (SharedString, SharedString, Vec<Group>) = if query.is_empty() {
        (
            current.label().into(),
            format!("{} Changes apply right away.", current.blurb()).into(),
            groups(ws, current, cx),
        )
    } else {
        let mut found = Vec::new();
        let mut count = 0;
        for sec in Section::ALL {
            for g in groups(ws, sec, cx) {
                let rows: Vec<Row> = g
                    .rows
                    .into_iter()
                    .filter(|r| {
                        r.label.to_lowercase().contains(&query)
                            || r.hint.is_some_and(|h| h.to_lowercase().contains(&query))
                            || g.title.to_lowercase().contains(&query)
                            || sec.label().to_lowercase().contains(&query)
                    })
                    .collect();
                if !rows.is_empty() {
                    count += rows.len();
                    found.push(Group {
                        title: sec.label(),
                        rows,
                    });
                }
            }
        }
        (
            "Search".into(),
            match count {
                0 => "Nothing matches. Try another word, like port or ratio.".into(),
                1 => "1 setting matches.".into(),
                n => format!("{n} settings match.").into(),
            },
            found,
        )
    };

    let nav = div()
        .w(t.px(228.))
        .flex_none()
        .flex()
        .flex_col()
        .gap(t.px(2.))
        .px(t.px(12.))
        .py(t.px(16.))
        .bg(t.c.side)
        .border_r_1()
        .border_color(t.c.rule)
        .child(
            div()
                .px(t.px(10.))
                .pb(t.px(12.))
                .font_family(FONT_HEAD)
                .font_weight(FontWeight::BOLD)
                .text_size(t.fpx(17.))
                .child("Settings"),
        )
        .child(
            div()
                .pb(t.px(10.))
                .child(field(&t, ws.settings_search.clone()).h(t.px(32.))),
        )
        .children(Section::ALL.iter().map(|&sec| {
            let on = query.is_empty() && sec == current;
            div()
                .id(SharedString::from(format!("sec-{sec:?}")))
                .flex()
                .items_center()
                .gap(t.px(10.))
                .h(t.px(32.))
                .px(t.px(10.))
                .rounded(t.px(8.))
                .cursor_pointer()
                .when(on, |d| {
                    d.bg(t.c.accent_soft)
                        .text_color(t.c.ink)
                        .font_weight(FontWeight::MEDIUM)
                })
                .when(!on, |d| d.text_color(t.c.ink_2).hover(|s| s.bg(t.c.hover)))
                .on_click(cx.listener(move |ws, _, _, cx| {
                    ws.settings_section = sec;
                    ws.settings_search.update(cx, |i, cx| i.set("", cx));
                    if sec == Section::Files {
                        ws.refresh_associations(cx);
                    }
                    cx.notify();
                }))
                .child(icon(
                    sec.icon(),
                    15.,
                    if on { t.c.accent_fg } else { t.c.muted },
                    &t,
                ))
                .child(sec.label())
        }));

    let header = div()
        .flex()
        .items_start()
        .gap(t.px(12.))
        .pl(t.px(28.))
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
                .gap(t.px(4.))
                .child(
                    div()
                        .font_family(FONT_HEAD)
                        .font_weight(FontWeight::BOLD)
                        .text_size(t.fpx(16.))
                        .child(title),
                )
                .child(div().text_color(t.c.muted).child(blurb)),
        )
        .child(
            div()
                .id("settings-close")
                .size(t.px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(t.px(8.))
                .cursor_pointer()
                .hover(|s| s.bg(t.c.hover))
                .on_click(cx.listener(|ws, _, w, cx| {
                    ws.dialog = None;
                    w.focus(&ws.focus);
                    cx.notify();
                }))
                .child(icon("x", 16., t.c.muted, &t)),
        );

    let body = div()
        .id("settings-body")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .px(t.px(28.))
        .pt(t.px(20.))
        .pb(t.px(28.))
        .gap(t.px(22.))
        .children(groups.into_iter().map(|g| group_view(&t, g)));

    div()
        .id("settings-sheet")
        .w(t.px(980.))
        .max_w(gpui::relative(0.96))
        .h(t.px(680.))
        .max_h(gpui::relative(0.92))
        .flex()
        .rounded(t.px(tt_theme::radius::MENU))
        .bg(t.c.raise)
        .border_1()
        .border_color(t.c.rule)
        .shadow(pop_shadow(&t))
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(nav)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(header)
                .child(body),
        )
        .into_any_element()
}
