//! The Mac app's controls (Components/Controls.swift): pill segment, chips, circle buttons, switches, pop-up
//! buttons, menus, popovers, sheets and grouped forms, drawn the same way on Windows and Linux.
use crate::{
    icons,
    theme::{self, W, ink},
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Id, Layout, Margin, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

const FULL: CornerRadius = CornerRadius::same(255);

/// Adds a widget at `rect` without moving the layout cursor (`Ui::put` would).
pub fn place(ui: &mut Ui, rect: Rect, widget: impl egui::Widget) -> Response {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::centered_and_justified(egui::Direction::TopDown)),
    );
    child.add(widget)
}

fn inset(rect: Rect, color: Color32) -> (Rect, Stroke) {
    (rect.shrink(0.5), Stroke::new(1.0_f32, color))
}
fn paint_capsule(ui: &Ui, rect: Rect, fill: Color32, outline: Option<Color32>) {
    ui.painter().rect_filled(rect, FULL, fill);
    if let Some(color) = outline {
        let (r, s) = inset(rect, color);
        ui.painter().rect_stroke(r, FULL, s, StrokeKind::Inside);
    }
}

// MARK: - Buttons

/// Video | Audio: a capsule with a white thumb behind the chosen option.
pub fn pill_segment<T: PartialEq + Clone>(
    ui: &mut Ui,
    id_salt: &str,
    value: &mut T,
    options: &[(T, &str)],
    height: f32,
) -> bool {
    let galleys: Vec<_> = options
        .iter()
        .map(|(v, title)| {
            let selected = v == value;
            theme::line(
                ui,
                title,
                13.0,
                if selected { W::Semibold } else { W::Medium },
                if selected {
                    theme::on_ink()
                } else {
                    theme::secondary()
                },
                None,
            )
        })
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 28.0).collect();
    let total = widths.iter().sum::<f32>() + 2.0 * (options.len() as f32 - 1.0) + 4.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
    paint_capsule(ui, rect, theme::surface(), Some(theme::stroke()));
    let mut x = rect.left() + 2.0;
    let mut slots = Vec::new();
    for width in &widths {
        slots.push(Rect::from_min_size(
            pos2(x, rect.top() + 2.0),
            vec2(*width, height - 4.0),
        ));
        x += width + 2.0;
    }
    let index = options.iter().position(|(v, _)| v == value).unwrap_or(0);
    let id = ui.id().with(id_salt);
    // Animate in slot units, so the thumb stays put when the window moves the control.
    let t = ui
        .ctx()
        .animate_value_with_time(id.with("slot"), index as f32, 0.2)
        .clamp(0.0, slots.len() as f32 - 1.0);
    let (a, b) = (t.floor() as usize, (t.ceil() as usize).min(slots.len() - 1));
    let f = t - a as f32;
    let left = slots[a].left() + (slots[b].left() - slots[a].left()) * f;
    let width = slots[a].width() + (slots[b].width() - slots[a].width()) * f;
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(left, slots[index].top()), vec2(width, height - 4.0)),
        FULL,
        theme::ink(1.0),
    );
    let mut changed = false;
    for (i, ((option, _), galley)) in options.iter().zip(galleys).enumerate() {
        let response = ui.interact(slots[i], id.with(i), Sense::click());
        ui.painter().galley(
            slots[i].center() - galley.size() / 2.0,
            galley,
            theme::ink(1.0),
        );
        if response.clicked() && option != value {
            *value = option.clone();
            changed = true;
        }
    }
    changed
}

/// A capsule button with an optional icon and chevron: the quality, folder and More buttons.
pub fn chip(ui: &mut Ui, text: &str, icon: Option<&str>, chevron: bool, open: bool) -> Response {
    let galley = theme::line(ui, text, 13.0, W::Medium, theme::text90(), Some(214.0));
    let mut width = 26.0 + galley.size().x;
    if icon.is_some() {
        width += 18.0;
    }
    if chevron {
        width += 15.0;
    }
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let fill = if open || response.is_pointer_button_down_on() {
        ink(0.1)
    } else if response.hovered() {
        ink(0.08)
    } else {
        theme::surface()
    };
    paint_capsule(ui, rect, fill, Some(theme::stroke()));
    let mut x = rect.left() + 13.0;
    if let Some(icon) = icon {
        icons::paint(
            ui,
            Rect::from_min_size(pos2(x, rect.top()), vec2(12.0, 32.0)),
            icon,
            12.0,
            W::Medium,
            theme::text90(),
        );
        x += 18.0;
    }
    let text_width = galley.size().x;
    ui.painter().galley(
        pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::ink(1.0),
    );
    if chevron {
        icons::paint(
            ui,
            Rect::from_min_size(pos2(x + text_width + 6.0, rect.top()), vec2(9.0, 32.0)),
            "chevronDown",
            9.0,
            W::Bold,
            theme::tertiary(),
        );
    }
    response
}

/// The white Download button.
pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let galley = theme::line(ui, text, 13.0, W::Semibold, theme::on_ink(), None);
    let size = vec2(galley.size().x + 36.0, 32.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (mut rect, response) = ui.allocate_exact_size(size, sense);
    if enabled && response.is_pointer_button_down_on() {
        rect = rect.shrink2(rect.size() * 0.015);
    }
    ui.painter().rect_filled(
        rect,
        FULL,
        if enabled { theme::ink(1.0) } else { ink(0.25) },
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, theme::on_ink());
    response
}

/// A small capsule button for fixes like "Retry" and "Try Again".
pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let galley = theme::line(ui, text, 12.0, W::Semibold, ink(0.92), None);
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 24.0, 28.0), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        ink(0.2)
    } else if response.hovered() {
        ink(0.14)
    } else {
        ink(0.1)
    };
    paint_capsule(ui, rect, fill, Some(theme::stroke()));
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, theme::ink(1.0));
    response
}

/// Round icon buttons: the queue's pause, resume, cancel, retry and details buttons.
pub fn circle_button(
    ui: &mut Ui,
    icon: &str,
    size: f32,
    filled: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(size, size),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if !enabled {
        if filled {
            ink(0.06)
        } else {
            Color32::TRANSPARENT
        }
    } else if response.is_pointer_button_down_on() {
        ink(0.18)
    } else if response.hovered() {
        ink(0.12)
    } else if filled {
        ink(0.06)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().circle_filled(rect.center(), size / 2.0, fill);
    if filled {
        ui.painter().circle_stroke(
            rect.center(),
            size / 2.0 - 0.5,
            Stroke::new(1.0_f32, theme::stroke()),
        );
    }
    icons::paint(
        ui,
        rect,
        icon,
        (size * 0.42).round(),
        W::Semibold,
        if enabled { ink(0.9) } else { ink(0.3) },
    );
    response.on_hover_text(tooltip)
}

/// A Mac push button at the regular control size.
pub fn bezel_button(ui: &mut Ui, text: &str, default: bool, enabled: bool) -> Response {
    let color = if !enabled {
        theme::tertiary()
    } else if default {
        Color32::WHITE
    } else {
        theme::text90()
    };
    let galley = theme::line(ui, text, 13.0, W::Regular, color, None);
    let (rect, response) = ui.allocate_exact_size(
        vec2(galley.size().x + 20.0, 22.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let down = enabled && response.is_pointer_button_down_on();
    let fill = if default && enabled {
        if down {
            theme::ACCENT.gamma_multiply(0.8)
        } else {
            theme::ACCENT
        }
    } else if down {
        ink(0.18)
    } else {
        ink(0.1)
    };
    bezel(ui, rect, fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}
/// A push button that opens a menu: the title and a small chevron.
pub fn bezel_menu_button(ui: &mut Ui, text: &str) -> Response {
    let galley = theme::line(ui, text, 13.0, W::Regular, theme::text90(), None);
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 33.0, 22.0), Sense::click());
    bezel(
        ui,
        rect,
        if response.is_pointer_button_down_on() {
            ink(0.18)
        } else {
            ink(0.1)
        },
    );
    ui.painter().galley(
        pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::ink(1.0),
    );
    icons::paint(
        ui,
        Rect::from_min_size(pos2(rect.right() - 18.0, rect.top()), vec2(8.0, 22.0)),
        "chevronDown",
        8.0,
        W::Bold,
        theme::text90(),
    );
    response
}
fn bezel(ui: &Ui, rect: Rect, fill: Color32) {
    let radius = CornerRadius::same(7);
    if !theme::dark() {
        // Light mode: the Mac's white push button with a soft edge; grey while pressed.
        let blue = fill == theme::ACCENT || fill == theme::ACCENT.gamma_multiply(0.8);
        let fill = if blue {
            fill
        } else if fill.a() > 30 {
            Color32::from_rgb(0xe4, 0xe4, 0xe6)
        } else {
            Color32::WHITE
        };
        ui.painter().rect_filled(
            rect.translate(vec2(0.0, 0.5)).expand(0.5),
            radius,
            theme::black(0.14),
        );
        ui.painter().rect_filled(rect, radius, fill);
        return;
    }
    ui.painter()
        .rect_filled(rect.expand(0.5), radius, theme::black(0.3));
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().line_segment(
        [
            pos2(rect.left() + 4.0, rect.top() + 0.5),
            pos2(rect.right() - 4.0, rect.top() + 0.5),
        ],
        Stroke::new(0.5_f32, ink(0.08)),
    );
}

/// A plain text button, like "Select All".
pub fn plain_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let galley = theme::line(ui, text, 12.0, W::Medium, color, None);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    let color = if response.is_pointer_button_down_on() {
        color.gamma_multiply(0.6)
    } else {
        color
    };
    ui.painter().galley(rect.min, galley, color);
    response
}

/// A blue text link, like "Advanced Options…".
pub fn link_button(ui: &mut Ui, text: &str) -> Response {
    let galley = theme::line(ui, text, 13.0, W::Regular, theme::link(), None);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    if response.hovered() {
        ui.painter().line_segment(
            [
                pos2(rect.left(), rect.bottom() - 1.0),
                pos2(rect.right(), rect.bottom() - 1.0),
            ],
            Stroke::new(1.0_f32, theme::link()),
        );
    }
    ui.painter().galley(rect.min, galley, theme::link());
    response
}

// MARK: - Inputs

/// A Mac switch at the small control size.
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(32.0, 18.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.18);
    let off = theme::ink_rgb();
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    let track = Color32::from_rgba_unmultiplied(
        mix(off[0], 10),
        mix(off[1], 132),
        mix(off[2], 255),
        mix((0.14 * 255.0) as u8, 255),
    );
    ui.painter().rect_filled(rect, FULL, track);
    let x = rect.left() + 9.0 + 14.0 * t;
    let knob = pos2(x, rect.center().y);
    ui.painter()
        .circle_filled(knob + vec2(0.0, 1.0), 8.5, theme::black(0.25));
    ui.painter().circle_filled(
        knob,
        8.0,
        if t > 0.5 {
            Color32::WHITE
        } else {
            Color32::from_rgb(0xe9, 0xe9, 0xea)
        },
    );
    response
}

/// A square check box with an optional label.
pub fn checkbox(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let galley = theme::line(ui, label, 13.0, W::Regular, theme::text90(), None);
    let width = 14.0
        + if label.is_empty() {
            0.0
        } else {
            6.0 + galley.size().x
        };
    let (rect, mut response) =
        ui.allocate_exact_size(vec2(width, 16.0_f32.max(galley.size().y)), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let boxed = Rect::from_min_size(pos2(rect.left(), rect.center().y - 7.0), vec2(14.0, 14.0));
    let radius = CornerRadius::same(4);
    if *on {
        ui.painter().rect_filled(boxed, radius, theme::ACCENT);
        icons::paint(ui, boxed, "checkmark", 10.0, W::Bold, Color32::WHITE);
    } else {
        ui.painter().rect_filled(boxed, radius, ink(0.1));
        ui.painter().rect_stroke(
            boxed,
            radius,
            Stroke::new(0.5_f32, ink(0.2)),
            StrokeKind::Inside,
        );
    }
    if !label.is_empty() {
        ui.painter().galley(
            pos2(boxed.right() + 6.0, rect.center().y - galley.size().y / 2.0),
            galley,
            theme::ink(1.0),
        );
    }
    response
}

/// A text field at the regular control size, with the Mac's blue focus ring.
pub fn text_input(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    text_input_at(ui, rect, value, hint, false)
}
pub fn text_input_at(
    ui: &mut Ui,
    rect: Rect,
    value: &mut String,
    hint: &str,
    multiline: bool,
) -> Response {
    let radius = CornerRadius::same(6);
    ui.painter().rect_filled(rect, radius, ink(0.07));
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(0.5_f32, ink(0.14)),
        StrokeKind::Inside,
    );
    let edit = if multiline {
        egui::TextEdit::multiline(value)
    } else {
        egui::TextEdit::singleline(value)
    };
    let response = place(
        ui,
        rect,
        edit.frame(false)
            .margin(Margin::symmetric(7, if multiline { 5 } else { 3 }))
            .font(theme::font(13.0, W::Regular))
            .text_color(theme::ink(1.0))
            .vertical_align(if multiline { Align::TOP } else { Align::Center })
            .desired_width(rect.width())
            .hint_text(theme::rich(hint, 13.0, W::Regular, theme::tertiary())),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(9),
            Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(10, 132, 255, 140)),
            StrokeKind::Inside,
        );
    }
    response
}

/// − and + in one bezel.
pub fn stepper(ui: &mut Ui, value: &mut u32, min: u32, max: u32, step: u32) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(44.0, 22.0), Sense::hover());
    bezel(ui, rect, ink(0.1));
    let mut changed = false;
    for (i, icon) in ["minus", "plus"].into_iter().enumerate() {
        let half = Rect::from_min_size(
            pos2(rect.left() + 22.0 * i as f32, rect.top()),
            vec2(22.0, 22.0),
        );
        let enabled = if i == 0 { *value > min } else { *value < max };
        let response = ui.interact(half, ui.id().with(("stepper", i)), Sense::click());
        if response.is_pointer_button_down_on() && enabled {
            ui.painter()
                .rect_filled(half, CornerRadius::same(7), ink(0.12));
        }
        icons::paint(
            ui,
            half,
            icon,
            10.0,
            W::Bold,
            if enabled {
                theme::text90()
            } else {
                theme::tertiary()
            },
        );
        if response.clicked() && enabled {
            *value = if i == 0 {
                value.saturating_sub(step).max(min)
            } else {
                (*value + step).min(max)
            };
            changed = true;
        }
    }
    ui.painter().line_segment(
        [
            pos2(rect.center().x, rect.top() + 4.0),
            pos2(rect.center().x, rect.bottom() - 4.0),
        ],
        Stroke::new(1.0_f32, ink(0.08)),
    );
    changed
}

/// The Mac's small segmented control (Settings).
pub fn segmented<T: PartialEq + Clone>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, t)| theme::line(ui, t, 13.0, W::Regular, theme::text90(), None))
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 24.0).collect();
    let (rect, _) =
        ui.allocate_exact_size(vec2(widths.iter().sum::<f32>() + 2.0, 22.0), Sense::hover());
    ui.painter().rect_filled(
        rect.expand(0.5),
        CornerRadius::same(7),
        theme::black(if theme::dark() { 0.3 } else { 0.06 }),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::same(7), ink(0.08));
    let mut x = rect.left() + 1.0;
    let mut changed = false;
    for (i, ((option, _), galley)) in options.iter().zip(galleys).enumerate() {
        let slot = Rect::from_min_size(pos2(x, rect.top() + 1.0), vec2(widths[i], 20.0));
        let response = ui.interact(slot, ui.id().with(("segmented", i)), Sense::click());
        if option == value {
            ui.painter().rect_filled(
                slot.translate(vec2(0.0, 1.0)),
                CornerRadius::same(6),
                theme::black(0.2),
            );
            let selected = if theme::dark() {
                ink(0.22)
            } else {
                Color32::WHITE
            };
            ui.painter()
                .rect_filled(slot, CornerRadius::same(6), selected);
        }
        ui.painter()
            .galley(slot.center() - galley.size() / 2.0, galley, theme::ink(1.0));
        if response.clicked() && option != value {
            *value = option.clone();
            changed = true;
        }
        x += widths[i];
    }
    changed
}

// MARK: - Menus

pub fn menu_frame() -> Frame {
    Frame::new()
        .fill(theme::menu_bg())
        .corner_radius(11)
        .inner_margin(Margin::same(5))
        .stroke(Stroke::new(0.5_f32, ink(0.14)))
        .shadow(egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: theme::black(0.45),
        })
}

/// One row of a menu: a checkmark column, the title and an optional shortcut. Hovered rows turn blue.
pub fn menu_item(
    ui: &mut Ui,
    label: &str,
    checked: bool,
    shortcut: Option<&str>,
    enabled: bool,
) -> bool {
    let color = if enabled {
        theme::text90()
    } else {
        theme::tertiary()
    };
    let galley = theme::line(ui, label, 13.0, W::Regular, color, None);
    let shortcut_galley =
        shortcut.map(|s| theme::line(ui, s, 13.0, W::Regular, theme::tertiary(), None));
    let desired = 24.0
        + galley.size().x
        + shortcut_galley
            .as_ref()
            .map(|g| g.size().x + 24.0)
            .unwrap_or(0.0)
        + 10.0;
    let (rect, response) = ui.allocate_at_least(
        vec2(desired.max(150.0), 24.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let active = enabled && response.hovered();
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), theme::ACCENT);
    }
    let text = if active { Color32::WHITE } else { color };
    if checked {
        icons::paint(
            ui,
            Rect::from_min_size(rect.min + vec2(6.0, 0.0), vec2(18.0, 24.0)),
            "checkmark",
            11.0,
            W::Semibold,
            text,
        );
    }
    ui.painter().galley(
        pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text,
    );
    if let Some(g) = shortcut_galley {
        let c = if active { ink(0.75) } else { theme::tertiary() };
        ui.painter().galley(
            pos2(
                rect.right() - 10.0 - g.size().x,
                rect.center().y - g.size().y / 2.0,
            ),
            g,
            c,
        );
    }
    let clicked = response.clicked();
    if clicked {
        ui.close();
    }
    clicked
}
pub fn menu_separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_at_least(vec2(1.0, 11.0), Sense::hover());
    ui.painter().line_segment(
        [
            pos2(rect.left() + 10.0, rect.center().y),
            pos2(rect.right() - 10.0, rect.center().y),
        ],
        Stroke::new(1.0_f32, ink(0.1)),
    );
}

/// Opens a menu under `anchor` when it's clicked. The menu closes when an item is chosen.
pub fn menu(anchor: &Response, align: egui::RectAlign, add: impl FnOnce(&mut Ui)) {
    egui::Popup::menu(anchor)
        .frame(menu_frame())
        .align(align)
        .gap(4.0)
        .layout(Layout::top_down_justified(Align::LEFT))
        .style(|style: &mut egui::Style| style.spacing.item_spacing = Vec2::ZERO)
        .show(add);
}

/// A Mac pop-up button: the current choice and up-down chevrons; clicking opens the choices.
pub fn popup_button<T: PartialEq + Clone>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    min_width: f32,
) -> bool {
    let current = options
        .iter()
        .find(|(v, _)| v == value)
        .map(|(_, t)| *t)
        .unwrap_or("");
    let galley = theme::line(ui, current, 13.0, W::Regular, theme::text90(), Some(260.0));
    let width = (galley.size().x + 31.0).max(min_width);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 22.0), Sense::click());
    bezel(
        ui,
        rect,
        if response.is_pointer_button_down_on() {
            ink(0.16)
        } else {
            ink(0.1)
        },
    );
    ui.painter().galley(
        pos2(rect.left() + 9.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme::ink(1.0),
    );
    icons::paint(
        ui,
        Rect::from_min_size(pos2(rect.right() - 18.0, rect.top()), vec2(11.0, 22.0)),
        "chevronUpDown",
        11.0,
        W::Semibold,
        theme::secondary(),
    );
    let mut changed = false;
    menu(&response, egui::RectAlign::BOTTOM_START, |ui| {
        ui.set_min_width(rect.width());
        for (option, title) in options {
            if *title == "—" {
                menu_separator(ui);
            } else if menu_item(ui, title, option == value, None, true) && option != value {
                *value = option.clone();
                changed = true;
            }
        }
    });
    changed
}

// MARK: - Popovers and sheets

pub fn popover_frame() -> Frame {
    Frame::new()
        .fill(theme::popover_bg())
        .corner_radius(14)
        .stroke(Stroke::new(0.5_f32, ink(0.14)))
        .shadow(egui::Shadow {
            offset: [0, 16],
            blur: 40,
            spread: 0,
            color: theme::black(0.5),
        })
}

/// A popover under `anchor` while `open`. Clicks in menus opened from inside it don't close it.
pub fn popover(anchor: &Response, open: &mut bool, width: f32, add: impl FnOnce(&mut Ui)) {
    if !*open {
        return;
    }
    let ctx = anchor.ctx.clone();
    let id = anchor.id.with("popover");
    let shown = egui::Popup::from_response(anchor)
        .id(id)
        .open(true)
        .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
        .frame(popover_frame())
        .align(egui::RectAlign::BOTTOM_END)
        .gap(6.0)
        .width(width)
        .show(|ui| {
            ui.set_width(width);
            add(ui)
        });
    let Some(shown) = shown else { return };
    let (click, pos, escape) = ctx.input(|i| {
        (
            i.pointer.any_click(),
            i.pointer.interact_pos(),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if escape {
        *open = false;
    }
    if click && let Some(pos) = pos {
        let layer = ctx.layer_id_at(pos);
        let in_child = layer.is_some_and(|l| l.order == egui::Order::Foreground && l.id != id);
        if !shown.response.rect.contains(pos) && !anchor.rect.contains(pos) && !in_child {
            *open = false;
        }
    }
}

pub fn sheet_frame() -> Frame {
    Frame::new()
        .fill(theme::matte())
        .corner_radius(18)
        .stroke(Stroke::new(1.0_f32, theme::stroke()))
        .shadow(egui::Shadow {
            offset: [0, 24],
            blur: 60,
            spread: 0,
            color: theme::black(0.55),
        })
}

/// A sheet over the window, like SwiftUI's `.sheet`. Returns `true` when Escape or the backdrop asks to close.
pub fn sheet(ctx: &egui::Context, id: &str, add: impl FnOnce(&mut Ui)) -> bool {
    egui::Modal::new(Id::new(("sheet", id)))
        .frame(sheet_frame())
        .backdrop_color(theme::black(0.3))
        .show(ctx, add)
        .should_close()
}

// MARK: - Progress and status

/// A 4-point progress bar; without a fraction it sweeps.
pub fn slim_progress(ui: &mut Ui, fraction: Option<f32>, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 4.0), Sense::hover());
    let radius = CornerRadius::same(2);
    ui.painter().rect_filled(rect, radius, ink(0.08));
    match fraction {
        Some(f) => {
            let w = (rect.width() * f.clamp(0.0, 1.0)).max(4.0);
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, vec2(w, 4.0)),
                radius,
                theme::ink(1.0),
            );
        }
        None => {
            let t = (ui.input(|i| i.time) / 1.4).fract() as f32;
            let w = rect.width() * 0.3;
            let left = rect.left() - w + (rect.width() + w) * t;
            let bar = Rect::from_min_max(
                pos2(left.max(rect.left()), rect.top()),
                pos2((left + w).min(rect.right()), rect.bottom()),
            );
            if bar.width() > 0.0 {
                ui.painter().rect_filled(bar, radius, ink(0.8));
            }
            ui.ctx().request_repaint();
        }
    }
}

/// The Mac's twelve-spoke activity indicator.
pub fn spinner(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    spinner_at(ui, rect);
}
pub fn spinner_at(ui: &Ui, rect: Rect) {
    let step = ((ui.input(|i| i.time) * 12.0) as usize) % 12;
    let scale = rect.width() / 24.0;
    for i in 0..12 {
        let angle = (i as f32) * std::f32::consts::TAU / 12.0;
        let (sin, cos) = angle.sin_cos();
        let at = |r: f32| rect.center() + vec2(sin * r, -cos * r) * scale;
        let age = (i + 12 - step) % 12;
        let alpha = 0.25 + (1.0 - age as f32 / 12.0) * 0.75;
        ui.painter().line_segment(
            [at(5.0), at(9.0)],
            Stroke::new(2.0 * scale, theme::secondary().gamma_multiply(alpha)),
        );
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(83));
}

/// A small coloured capsule like "LIVE".
pub fn tag(ui: &mut Ui, text: &str, color: Color32) {
    let galley = theme::line(ui, text, 10.0, W::Semibold, color, None);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(14.0, 6.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, FULL, color.gamma_multiply(0.18));
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// A 16:9 thumbnail with a play mark until the image loads.
pub fn thumbnail(ui: &mut Ui, url: Option<&str>, width: f32, opacity: f32) -> Response {
    let size = vec2(width, width * 9.0 / 16.0);
    let radius = (width * 0.08).max(6.0).round() as u8;
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), ink(0.05));
    icons::paint(
        ui,
        rect,
        "playFill",
        (width * 0.16).max(8.0),
        W::Medium,
        theme::tertiary(),
    );
    if let Some(url) = url {
        egui::Image::new(url)
            .fit_to_exact_size(size)
            .maintain_aspect_ratio(false)
            .corner_radius(radius)
            .tint(Color32::WHITE.gamma_multiply(opacity))
            .show_loading_spinner(false)
            .paint_at(ui, rect);
    }
    response
}

/// A frame with the Mac's faint fill and hairline: info card, queue rows.
pub fn card() -> Frame {
    Frame::new()
        .fill(ink(0.04))
        .stroke(Stroke::new(1.0_f32, theme::stroke()))
        .corner_radius(16)
}

// MARK: - Grouped forms (Settings)

pub struct Form {
    rows: usize,
}

/// A grouped section: an optional header, the rows in a rounded box, an optional footer.
pub fn section(
    ui: &mut Ui,
    header: Option<&str>,
    footer: Option<&str>,
    add: impl FnOnce(&mut Ui, &mut Form),
) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        if let Some(header) = header {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(theme::rich(header, 13.0, W::Semibold, ink(0.85)));
            });
        }
        Frame::new()
            .fill(ink(0.04))
            .stroke(Stroke::new(1.0_f32, ink(0.06)))
            .corner_radius(10)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut form = Form { rows: 0 };
                add(ui, &mut form);
            });
        if let Some(footer) = footer {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 10.0);
                    paragraph(ui, footer, 11.0, theme::secondary());
                });
            });
        }
    });
}
fn paragraph(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    theme::paragraph(ui, text, size, W::Regular, color, 0);
}

/// A form row: the label on the left, the control on the right.
pub fn row(
    ui: &mut Ui,
    form: &mut Form,
    label: &str,
    sub: Option<&str>,
    value: impl FnOnce(&mut Ui),
) {
    let first = form.rows == 0;
    form.rows += 1;
    let width = ui.available_width();
    let label = theme::line(
        ui,
        label,
        13.0,
        W::Regular,
        theme::text90(),
        Some(width * 0.6),
    );
    let sub =
        sub.map(|s| theme::wrapped(ui, s, 11.0, W::Regular, theme::secondary(), width * 0.6, 0));
    let text_height = label.size().y + sub.as_ref().map(|g| g.size().y + 2.0).unwrap_or(0.0);
    let (rect, _) =
        ui.allocate_exact_size(vec2(width, (text_height + 16.0).max(40.0)), Sense::hover());
    let mut y = rect.center().y - text_height / 2.0;
    let label_height = label.size().y;
    ui.painter()
        .galley(pos2(rect.left() + 12.0, y), label, theme::ink(1.0));
    if let Some(sub) = sub {
        y += label_height + 2.0;
        ui.painter()
            .galley(pos2(rect.left() + 12.0, y), sub, theme::ink(1.0));
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(12.0, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    value(&mut child);
    if !first {
        separator(ui, rect);
    }
}
/// A row whose content fills the width (text fields, tables, previews).
pub fn stack_row(ui: &mut Ui, form: &mut Form, add: impl FnOnce(&mut Ui)) {
    let first = form.rows == 0;
    form.rows += 1;
    let response = Frame::new()
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            add(ui);
        });
    if !first {
        separator(ui, response.response.rect);
    }
}
fn separator(ui: &Ui, rect: Rect) {
    ui.painter().line_segment(
        [
            pos2(rect.left() + 12.0, rect.top()),
            pos2(rect.right() - 12.0, rect.top()),
        ],
        Stroke::new(1.0_f32, ink(0.06)),
    );
}
pub fn switch_row(ui: &mut Ui, form: &mut Form, label: &str, on: &mut bool) -> bool {
    let mut changed = false;
    row(ui, form, label, None, |ui| {
        changed = switch(ui, on).changed()
    });
    changed
}
pub fn picker_row<T: PartialEq + Clone>(
    ui: &mut Ui,
    form: &mut Form,
    label: &str,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    row(ui, form, label, None, |ui| {
        changed = popup_button(ui, value, options, 0.0);
    });
    changed
}
