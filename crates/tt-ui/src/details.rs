//! Details tabs: General (piece map and key figures), Trackers, Peers,
//! HTTP sources, Content, Speed.

use crate::app::Workspace;
use crate::theme::{FONT_NUM, Theme};
use crate::widgets::{icon, percent_bar};
use gpui::{
    AnyElement, Bounds, Context, FontWeight, IntoElement, ParentElement, PathBuilder, Pixels,
    SharedString, Styled, Window, canvas, div, fill, point, prelude::*, px, size,
};
use tt_ipc::{DetailsTab, TorrentDetails, TorrentRow, fmt};

pub fn details(
    ws: &mut Workspace,
    _window: &mut Window,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let t = ws.theme;
    let tab = ws.prefs.details_tab;
    let row = ws.model.focused().cloned();
    let tabs = div()
        .h(t.px(t.m.tab_bar))
        .flex_none()
        .flex()
        .items_end()
        .px(t.px(10.))
        .gap(t.px(4.))
        .border_b_1()
        .border_color(t.c.rule)
        .children(DetailsTab::ALL.iter().map(|&tb| {
            let active = tb == tab;
            div()
                .id(SharedString::from(format!("tab-{tb:?}")))
                .px(t.px(10.))
                .pb(t.px(9.))
                .relative()
                .cursor_pointer()
                .text_color(if active { t.c.ink } else { t.c.muted })
                .when(active, |d| d.font_weight(FontWeight::MEDIUM))
                .hover(|s| s.text_color(t.c.ink))
                .on_click(cx.listener(move |this, _, _, cx| this.set_tab(tb, cx)))
                .child(tb.label())
                .when(active, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left(t.px(8.))
                            .right(t.px(8.))
                            .bottom(px(-1.))
                            .h(px(2.))
                            .bg(t.c.accent_fg),
                    )
                })
        }))
        .child(div().flex_1())
        .when_some(row.as_ref(), |d, r| {
            d.child(
                div()
                    .pb(t.px(9.))
                    .max_w(t.px(320.))
                    .truncate()
                    .text_size(t.fpx(12.))
                    .text_color(t.c.muted)
                    .child(r.name.clone()),
            )
        });
    let body: AnyElement = match (&row, tab) {
        (None, _) => div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(t.c.muted)
            .child("Select a torrent to see its details")
            .into_any_element(),
        (Some(r), DetailsTab::General) => general(&t, r, ws.details.as_ref()),
        (Some(_), DetailsTab::Trackers) => trackers(&t, ws.details.as_ref()),
        (Some(_), DetailsTab::Peers) => peers(&t, ws.details.as_ref()),
        (Some(_), DetailsTab::HttpSources) => http(&t, ws.details.as_ref()),
        (Some(_), DetailsTab::Content) => content(&t, ws.details.as_ref()),
        (Some(_), DetailsTab::Speed) => speed(&t, &ws.speed),
    };
    div()
        .h(t.px(ws.prefs.details_height))
        .flex_none()
        .flex()
        .flex_col()
        .bg(t.c.panel)
        .border_t_1()
        .border_color(t.c.rule)
        .child(tabs)
        .child(
            div()
                .id("details-body")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(body),
        )
        .into_any_element()
}

/// Map `count` pieces onto `cells` squares: 0 none, 1 partial, 2 all.
pub fn piece_cells(pieces: &[u8], count: u32, cells: usize) -> Vec<u8> {
    let count = count as usize;
    let have = |i: usize| {
        pieces
            .get(i / 8)
            .is_some_and(|b| b & (0x80 >> (i % 8)) != 0)
    };
    (0..cells)
        .map(|c| {
            if count == 0 {
                return 0;
            }
            let a = c * count / cells;
            let b = ((c + 1) * count / cells).max(a + 1).min(count);
            let n = (a..b).filter(|&i| have(i)).count();
            if n == 0 {
                0
            } else if n == b - a {
                2
            } else {
                1
            }
        })
        .collect()
}

fn figure(t: &Theme, label: &'static str, value: String) -> gpui::Div {
    div()
        .flex()
        .gap(t.px(14.))
        .h(t.px(24.))
        .items_center()
        .child(
            div()
                .w(t.px(86.))
                .flex_none()
                .text_size(t.fpx(12.5))
                .text_color(t.c.muted)
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_family(FONT_NUM)
                .text_size(t.fpx(12.5))
                .text_color(t.c.ink)
                .child(value),
        )
}

fn general(t: &Theme, r: &TorrentRow, d: Option<&TorrentDetails>) -> AnyElement {
    let cols = tt_theme::PIECE_COLUMNS;
    let rows = 6;
    let (have, count) = d.map(|d| (d.pieces_have, d.piece_count)).unwrap_or((0, 0));
    let cells = match d {
        Some(d) if d.piece_count > 0 => piece_cells(&d.pieces, d.piece_count, cols * rows),
        // No details yet (still starting): draw from the stored progress.
        _ => {
            let full = (r.progress * (cols * rows) as f32) as usize;
            (0..cols * rows)
                .map(|i| if i < full { 2 } else { 0 })
                .collect()
        }
    };
    let (c_full, c_part, c_none) = (t.c.accent, t.c.accent_line, t.c.track);
    let gap = t.scale * 2.0;
    let map = canvas(
        move |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window, _| {
            let w = f32::from(bounds.size.width);
            let cell = ((w - gap * (cols as f32 - 1.0)) / cols as f32).clamp(4.0, 14.0);
            for (i, v) in cells.iter().enumerate() {
                let (x, y) = ((i % cols) as f32, (i / cols) as f32);
                let o = point(
                    bounds.origin.x + px(x * (cell + gap)),
                    bounds.origin.y + px(y * (cell + gap)),
                );
                let color = match v {
                    2 => c_full,
                    1 => c_part,
                    _ => c_none,
                };
                window.paint_quad(
                    fill(Bounds::new(o, size(px(cell), px(cell))), color)
                        .corner_radii(px(tt_theme::radius::PIECE)),
                );
            }
        },
    )
    .w_full()
    .h(t.px(6.0 * 14.0));
    let ratio = fmt::ratio(r.ratio);
    let left = div()
        .w(t.px(520.))
        .flex_none()
        .flex()
        .flex_col()
        .gap(t.px(8.))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(t.fpx(12.5))
                .text_color(t.c.muted)
                .child("Pieces")
                .child(div().font_family(FONT_NUM).child(if count > 0 {
                    format!("{have} of {count}")
                } else {
                    fmt::percent(r.progress)
                })),
        )
        .child(map);
    let piece_size = d
        .map(|d| fmt::size(d.piece_size))
        .unwrap_or_else(|| "—".into());
    let callout = r.error.clone().filter(|_| r.state.is_errored());
    div()
        .p(t.px(14.))
        .flex()
        .flex_col()
        .gap(t.px(12.))
        .when_some(callout, |d, msg| {
            d.child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.px(10.))
                    .px(t.px(12.))
                    .py(t.px(8.))
                    .rounded(t.px(10.))
                    .border_1()
                    .border_color(t.c.bad)
                    .text_color(t.c.ink)
                    .child(icon("alert", 16., t.c.bad, t))
                    .child(msg),
            )
        })
        .child(
            div().flex().gap(t.px(28.)).child(left).child(
                div()
                    .flex()
                    .gap(t.px(24.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(figure(t, "Downloaded", fmt::size(r.downloaded)))
                            .child(figure(t, "Down speed", fmt::speed(r.down_bps)))
                            .child(figure(
                                t,
                                "Seeds",
                                format!("{} of {}", r.seeds, r.seeds_total),
                            ))
                            .child(figure(t, "Ratio", ratio))
                            .child(figure(t, "Piece size", piece_size))
                            .child(figure(t, "Added on", fmt::date(r.added_on))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .child(figure(t, "Uploaded", fmt::size(r.uploaded)))
                            .child(figure(t, "Up speed", fmt::speed(r.up_bps)))
                            .child(figure(
                                t,
                                "Peers",
                                format!("{} of {}", r.peers, r.peers_total),
                            ))
                            .child(figure(
                                t,
                                "ETA",
                                r.eta_secs.map(fmt::eta).unwrap_or_else(|| "∞".into()),
                            ))
                            .child(figure(t, "Save path", r.save_path.clone()))
                            .child(figure(
                                t,
                                "Category",
                                if r.category.is_empty() {
                                    "—".into()
                                } else {
                                    r.category.clone()
                                },
                            )),
                    ),
            ),
        )
        .into_any_element()
}

fn table(t: &Theme, headers: &[(&'static str, f32, bool)], rows: Vec<Vec<String>>) -> AnyElement {
    let head = div()
        .flex()
        .h(t.px(28.))
        .items_center()
        .border_b_1()
        .border_color(t.c.rule)
        .text_size(t.fpx(12.))
        .text_color(t.c.muted)
        .children(headers.iter().map(|(h, w, right)| {
            div()
                .when(*w == 0.0, |d| d.flex_1().min_w_0())
                .when(*w > 0.0, |d| d.w(t.px(*w)).flex_none())
                .px(t.px(12.))
                .when(*right, |d| d.text_right())
                .child(*h)
        }));
    let mut body = div().flex().flex_col();
    for r in rows {
        body = body.child(
            div()
                .flex()
                .h(t.px(26.))
                .items_center()
                .border_b_1()
                .border_color(t.c.row_rule)
                .children(r.into_iter().zip(headers.iter()).map(|(v, (_, w, right))| {
                    div()
                        .when(*w == 0.0, |d| d.flex_1().min_w_0())
                        .when(*w > 0.0, |d| d.w(t.px(*w)).flex_none())
                        .px(t.px(12.))
                        .truncate()
                        .when(*right, |d| {
                            d.text_right().font_family(FONT_NUM).text_size(t.fpx(12.))
                        })
                        .text_color(t.c.ink_2)
                        .child(v)
                })),
        );
    }
    div()
        .flex()
        .flex_col()
        .child(head)
        .child(body)
        .into_any_element()
}

fn note(t: &Theme, text: &'static str) -> AnyElement {
    div()
        .p(t.px(16.))
        .text_color(t.c.muted)
        .child(text)
        .into_any_element()
}

fn trackers(t: &Theme, d: Option<&TorrentDetails>) -> AnyElement {
    let Some(d) = d else {
        return note(t, "Loading…");
    };
    if d.trackers.is_empty() {
        return note(
            t,
            "No trackers. This torrent finds peers through DHT, PEX and LSD.",
        );
    }
    table(
        t,
        &[
            ("URL", 0.0, false),
            ("Status", 140.0, false),
            ("Message", 220.0, false),
        ],
        d.trackers
            .iter()
            .map(|x| vec![x.url.clone(), x.status.clone(), x.message.clone()])
            .collect(),
    )
}

fn peers(t: &Theme, d: Option<&TorrentDetails>) -> AnyElement {
    let Some(d) = d else {
        return note(t, "Loading…");
    };
    if d.peers.is_empty() {
        return note(t, "No peers connected right now.");
    }
    table(
        t,
        &[
            ("Address", 0.0, false),
            ("Client", 200.0, false),
            ("Downloaded", 110.0, true),
            ("Uploaded", 110.0, true),
        ],
        d.peers
            .iter()
            .map(|p| {
                vec![
                    p.addr.clone(),
                    p.client.clone(),
                    fmt::size(p.downloaded),
                    fmt::size(p.uploaded),
                ]
            })
            .collect(),
    )
}

fn http(t: &Theme, d: Option<&TorrentDetails>) -> AnyElement {
    match d {
        Some(d) if !d.http_sources.is_empty() => table(
            t,
            &[("URL", 0.0, false)],
            d.http_sources.iter().map(|u| vec![u.clone()]).collect(),
        ),
        _ => note(t, "No HTTP sources (web seeds) in this torrent."),
    }
}

fn content(t: &Theme, d: Option<&TorrentDetails>) -> AnyElement {
    let Some(d) = d else {
        return note(t, "Loading…");
    };
    let mut col = div().flex().flex_col();
    col = col.child(
        div()
            .flex()
            .h(t.px(28.))
            .items_center()
            .border_b_1()
            .border_color(t.c.rule)
            .text_size(t.fpx(12.))
            .text_color(t.c.muted)
            .child(div().flex_1().px(t.px(12.)).child("Name"))
            .child(div().w(t.px(96.)).px(t.px(12.)).text_right().child("Size"))
            .child(div().w(t.px(180.)).px(t.px(12.)).child("Progress"))
            .child(div().w(t.px(90.)).px(t.px(12.)).child("Priority")),
    );
    for f in &d.files {
        col = col.child(
            div()
                .flex()
                .h(t.px(26.))
                .items_center()
                .border_b_1()
                .border_color(t.c.row_rule)
                .text_color(t.c.ink_2)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(t.px(12.))
                        .flex()
                        .gap(t.px(8.))
                        .items_center()
                        .child(icon("file", 13., t.c.muted, t))
                        .child(div().truncate().child(f.path.clone())),
                )
                .child(
                    div()
                        .w(t.px(96.))
                        .px(t.px(12.))
                        .text_right()
                        .font_family(FONT_NUM)
                        .text_size(t.fpx(12.))
                        .child(fmt::size(f.size)),
                )
                .child(
                    div()
                        .w(t.px(180.))
                        .px(t.px(12.))
                        .flex()
                        .items_center()
                        .gap(t.px(8.))
                        .child(percent_bar(
                            f.progress,
                            if f.progress >= 1.0 {
                                t.c.ok
                            } else {
                                t.c.accent
                            },
                            t,
                        ))
                        .child(
                            div()
                                .w(t.px(44.))
                                .text_right()
                                .font_family(FONT_NUM)
                                .text_size(t.fpx(12.))
                                .child(fmt::percent(f.progress)),
                        ),
                )
                .child(div().w(t.px(90.)).px(t.px(12.)).child(match f.priority {
                    tt_ipc::FilePriority::Skip => "Don't download",
                    tt_ipc::FilePriority::Normal => "Normal",
                    tt_ipc::FilePriority::High => "High",
                    tt_ipc::FilePriority::Maximum => "Maximum",
                })),
        );
    }
    col.into_any_element()
}

fn speed(t: &Theme, points: &[tt_ipc::SpeedPoint]) -> AnyElement {
    if points.len() < 2 {
        return note(t, "Collecting speed samples…");
    }
    let max = points
        .iter()
        .map(|p| p.down_bps.max(p.up_bps))
        .max()
        .unwrap_or(1)
        .max(1024) as f32;
    let pts: Vec<(u64, u64)> = points.iter().map(|p| (p.down_bps, p.up_bps)).collect();
    let (cd, cu) = (t.c.accent_fg, t.c.ok);
    let last = points.last().copied().unwrap_or_default();
    let line_w = px(1.6 * t.scale);
    div()
        .p(t.px(14.))
        .flex()
        .flex_col()
        .gap(t.px(8.))
        .child(
            div()
                .flex()
                .gap(t.px(18.))
                .text_size(t.fpx(12.5))
                .child(
                    div()
                        .flex()
                        .gap(t.px(6.))
                        .items_center()
                        .child(div().size(t.px(8.)).rounded_full().bg(cd))
                        .child(format!("Download {}", fmt::speed(last.down_bps))),
                )
                .child(
                    div()
                        .flex()
                        .gap(t.px(6.))
                        .items_center()
                        .child(div().size(t.px(8.)).rounded_full().bg(cu))
                        .child(format!("Upload {}", fmt::speed(last.up_bps))),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_color(t.c.muted)
                        .font_family(FONT_NUM)
                        .child(format!("peak {}", fmt::speed(max as u64))),
                ),
        )
        .child(
            canvas(
                move |_, _, _| (),
                move |b: Bounds<Pixels>, _, window, _| {
                    let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                    let n = pts.len();
                    for (which, color) in [(0usize, cd), (1usize, cu)] {
                        let mut path = PathBuilder::stroke(line_w);
                        for (i, p) in pts.iter().enumerate() {
                            let v = if which == 0 { p.0 } else { p.1 } as f32;
                            let x = b.origin.x + px(w * i as f32 / (n - 1) as f32);
                            let y = b.origin.y + px(h - h * (v / max).min(1.0));
                            if i == 0 {
                                path.move_to(point(x, y));
                            } else {
                                path.line_to(point(x, y));
                            }
                        }
                        if let Ok(p) = path.build() {
                            window.paint_path(p, color);
                        }
                    }
                },
            )
            .w_full()
            .h(t.px(150.)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::piece_cells;

    #[test]
    fn piece_cells_aggregate() {
        // 8 pieces, first 4 present, onto 4 cells.
        assert_eq!(piece_cells(&[0b1111_0000], 8, 4), vec![2, 2, 0, 0]);
        // 8 pieces, alternating, onto 4 cells: every cell partial.
        assert_eq!(piece_cells(&[0b1010_1010], 8, 4), vec![1, 1, 1, 1]);
        // Fewer pieces than cells: each cell maps to one piece.
        assert_eq!(piece_cells(&[0b1000_0000], 2, 4), vec![2, 2, 0, 0]);
        assert_eq!(piece_cells(&[], 0, 3), vec![0, 0, 0]);
    }
}
