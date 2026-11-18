//! A single-line text field: typing, Backspace, Ctrl+Backspace, Ctrl+V,
//! Ctrl+A (clear on next key), Esc and Enter handled by the owner.

use crate::theme::{FONT_UI, Theme};
use gpui::{
    App, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, ParentElement, Render,
    SharedString, Styled, Window, div, prelude::*,
};

pub enum InputEvent {
    Changed,
    Submit,
    Cancel,
}

pub struct TextInput {
    pub text: String,
    pub placeholder: SharedString,
    focus: FocusHandle,
    all_selected: bool,
    pub theme: Theme,
}

impl gpui::EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    pub fn new(placeholder: &str, theme: Theme, cx: &mut Context<Self>) -> Self {
        Self {
            text: String::new(),
            placeholder: SharedString::from(placeholder.to_owned()),
            focus: cx.focus_handle(),
            all_selected: false,
            theme,
        }
    }

    pub fn set(&mut self, text: &str, cx: &mut Context<Self>) {
        self.text = text.to_owned();
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn key(&mut self, e: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let k = &e.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        match k.key.as_str() {
            "enter" => {
                cx.emit(InputEvent::Submit);
                cx.stop_propagation();
                return;
            }
            "escape" => {
                cx.emit(InputEvent::Cancel);
                cx.stop_propagation();
                return;
            }
            "backspace" => {
                if self.all_selected || ctrl {
                    if ctrl && !self.all_selected {
                        let t = self.text.trim_end();
                        let cut = t.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
                        self.text.truncate(cut);
                    } else {
                        self.text.clear();
                    }
                } else {
                    self.text.pop();
                }
                self.all_selected = false;
            }
            "a" if ctrl => {
                self.all_selected = true;
                cx.notify();
                cx.stop_propagation();
                return;
            }
            "v" if ctrl => {
                if let Some(s) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    if self.all_selected {
                        self.text.clear();
                    }
                    self.text.push_str(s.trim_end_matches(['\r', '\n']));
                    self.all_selected = false;
                }
            }
            _ => {
                if ctrl || k.modifiers.alt {
                    return;
                }
                let Some(ch) = k.key_char.as_deref() else {
                    return;
                };
                if ch.chars().any(char::is_control) {
                    return;
                }
                if self.all_selected {
                    self.text.clear();
                    self.all_selected = false;
                }
                self.text.push_str(ch);
            }
        }
        cx.stop_propagation();
        cx.emit(InputEvent::Changed);
        cx.notify();
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        let focused = self.focus.is_focused(window);
        let empty = self.text.is_empty();
        div()
            .id("text-input")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .flex()
            .items_center()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .font_family(FONT_UI)
            .text_size(t.fpx(t.m.ui_font))
            .child(if empty {
                div()
                    .text_color(t.c.muted)
                    .child(self.placeholder.clone())
                    .into_any_element()
            } else {
                div()
                    .text_color(t.c.ink)
                    .when(self.all_selected, |d| d.bg(t.c.accent_soft))
                    .child(SharedString::from(self.text.clone()))
                    .into_any_element()
            })
            .when(focused, |d| {
                d.child(div().w(gpui::px(1.5)).h(t.px(15.)).bg(t.c.accent_fg))
            })
    }
}
