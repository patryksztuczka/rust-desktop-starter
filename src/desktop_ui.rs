//! Shared visual tokens and controls. Every pane builds from these so the app
//! keeps one look; add a helper here instead of styling ad hoc in a pane.

use gpui::{
    AnyElement, Div, ElementId, FontWeight, IntoElement, SharedString, Stateful, div, prelude::*,
    px, rgb,
};
use gpui_symbols::{RenderingMode, SymbolWeight};

pub const SIDEBAR: u32 = 0x111111;
pub const CANVAS: u32 = 0x161616;
pub const SURFACE: u32 = 0x1d1d1d;
pub const SURFACE_HOVER: u32 = 0x252525;
pub const SURFACE_SELECTED: u32 = 0x2e2e2e;
pub const LINE: u32 = 0x2a2a2a;
pub const ACCENT: u32 = 0x3b5cf6;
pub const ACCENT_HOVER: u32 = 0x5070ff;
pub const POSITIVE: u32 = 0x69d89f;
pub const NEGATIVE: u32 = 0xf17471;
pub const TEXT: u32 = 0xf2f2f2;
pub const TEXT_SOFT: u32 = 0xd0d0d0;
pub const MUTED: u32 = 0x969696;

pub const SIDEBAR_WIDTH: f32 = 200.0;
/// Space reserved for the transparent titlebar's traffic-light buttons.
pub const TITLEBAR_HEIGHT: f32 = 40.0;
pub const CONTROL_HEIGHT: f32 = 30.0;
pub const PANEL_RADIUS: f32 = 10.0;
pub const PANE_CONTENT_WIDTH: f32 = 720.0;

/// An SF Symbol, e.g. `"gearshape"`. Browse names in Apple's SF Symbols app.
pub fn icon(name: &'static str, size: f32, color: u32) -> AnyElement {
    gpui_symbols::Icon::new(name)
        .size(px(size))
        .color(rgb(color))
        .weight(SymbolWeight::Medium)
        .rendering_mode(RenderingMode::Monochrome)
        .into_any_element()
}

pub fn window_frame() -> Div {
    div()
        .size_full()
        .flex()
        .bg(rgb(CANVAS))
        .text_color(rgb(TEXT))
        .text_size(px(13.0))
}

pub fn sidebar() -> Div {
    div()
        .w(px(SIDEBAR_WIDTH))
        .h_full()
        .flex_none()
        .flex()
        .flex_col()
        .gap_0p5()
        .px_2()
        .pt(px(TITLEBAR_HEIGHT + 8.0))
        .bg(rgb(SIDEBAR))
        .border_r_1()
        .border_color(rgb(LINE))
}

pub fn navigation_item(
    id: impl Into<ElementId>,
    symbol: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(32.0))
        .px_3()
        .flex()
        .items_center()
        .gap_2p5()
        .rounded(px(6.0))
        .cursor_pointer()
        .font_weight(if selected {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        })
        .text_color(rgb(if selected { TEXT } else { MUTED }))
        .when(selected, |item| item.bg(rgb(SURFACE_SELECTED)))
        .when(!selected, |item| {
            item.hover(|item| item.bg(rgb(SURFACE)).text_color(rgb(TEXT_SOFT)))
        })
        .child(
            div()
                .size(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(symbol, 14.0, if selected { TEXT } else { MUTED })),
        )
        .child(label.into())
}

/// The scaffold every pane renders: a scrollable column with a title header.
/// Add the pane's `pane_body` as its next child.
pub fn pane(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .pt(px(TITLEBAR_HEIGHT))
        .pb_8()
        .px_8()
        .child(
            div()
                .max_w(px(PANE_CONTENT_WIDTH))
                .flex()
                .flex_col()
                .gap_1()
                .pb_5()
                .child(
                    div()
                        .text_size(px(22.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title.into()),
                )
                .child(div().text_color(rgb(MUTED)).child(subtitle.into())),
        )
}

/// Pane bodies go inside this so every pane shares one content width.
pub fn pane_body() -> Div {
    div()
        .max_w(px(PANE_CONTENT_WIDTH))
        .flex()
        .flex_col()
        .gap_3()
}

pub fn panel() -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(PANEL_RADIUS))
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(LINE))
}

/// A labelled row inside a `panel`, with a control aligned to the right.
pub fn setting_row(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(div().font_weight(FontWeight::MEDIUM).child(title.into()))
                .child(div().text_color(rgb(MUTED)).child(detail.into())),
        )
        .child(div().flex_none().child(control))
}

pub fn divider() -> Div {
    div().h(px(1.0)).bg(rgb(LINE))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    style: ButtonStyle,
    enabled: bool,
) -> Stateful<Div> {
    let (background, hover) = match style {
        ButtonStyle::Primary => (ACCENT, ACCENT_HOVER),
        ButtonStyle::Secondary => (SURFACE_HOVER, SURFACE_SELECTED),
    };
    div()
        .id(id)
        .h(px(CONTROL_HEIGHT))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .font_weight(FontWeight::MEDIUM)
        .bg(rgb(background))
        .when(!enabled, |button| button.opacity(0.45))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|button| button.bg(rgb(hover)))
        })
        .child(label.into())
}

pub fn toggle(id: impl Into<ElementId>, on: bool) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(36.0))
        .h(px(20.0))
        .p(px(2.0))
        .flex()
        .when(on, |track| track.justify_end())
        .rounded_full()
        .cursor_pointer()
        .bg(rgb(if on { ACCENT } else { SURFACE_SELECTED }))
        .child(div().size(px(16.0)).rounded_full().bg(rgb(TEXT)))
}

/// Small coloured status text, e.g. under a control that can fail.
pub fn status_text(message: impl Into<SharedString>, positive: bool) -> Div {
    div()
        .text_color(rgb(if positive { POSITIVE } else { NEGATIVE }))
        .child(message.into())
}
