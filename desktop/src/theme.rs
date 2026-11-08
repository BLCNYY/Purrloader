//! Purrloader's look, point for point from the Mac app: colours, Inter in place of SF Pro, and text helpers.
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Galley, RichText,
    Shadow, Stroke, TextFormat, Ui,
    text::{LayoutJob, TextWrapping},
};
use std::sync::Arc;

pub const ACCENT: Color32 = Color32::from_rgb(0x0a, 0x84, 0xff);
pub const GREEN: Color32 = Color32::from_rgb(0x30, 0xd1, 0x58);
pub const RED: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);
pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x9f, 0x0a);

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
static DARK: AtomicBool = AtomicBool::new(true);
static BLURRED: AtomicBool = AtomicBool::new(false);

/// Light or dark, as Settings > General > Appearance and the system say. Every colour below follows it.
pub fn dark() -> bool {
    DARK.load(Relaxed)
}
/// Called once the system draws a blurred backdrop behind the windows (Windows 11).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn set_blurred(on: bool) {
    BLURRED.store(on, Relaxed);
}
pub fn blurred() -> bool {
    BLURRED.load(Relaxed)
}
fn pick(dark: Color32, light: Color32) -> Color32 {
    if self::dark() { dark } else { light }
}

/// What the Mac's matte (black, or white in light mode) at 80% over the blurred desktop comes out as.
pub fn bg() -> Color32 {
    pick(
        Color32::from_rgb(0x2d, 0x2d, 0x2f),
        Color32::from_rgb(0xec, 0xec, 0xee),
    )
}
/// The matte itself: sheets and the details log sit on it.
pub fn matte() -> Color32 {
    pick(
        Color32::from_rgb(0x09, 0x09, 0x0a),
        Color32::from_rgb(0xf9, 0xf9, 0xfa),
    )
}
/// The window background: the matte at 80% over the blurred desktop where the system blurs it, as on the
/// Mac; elsewhere the solid colour that comes out as.
pub fn window_fill() -> Color32 {
    if blurred() {
        let m = matte();
        Color32::from_rgba_unmultiplied(m.r(), m.g(), m.b(), 204)
    } else {
        bg()
    }
}
/// The sticky header colour of tables (History, the format list).
pub fn table_head() -> Color32 {
    pick(
        Color32::from_rgb(0x2a, 0x2a, 0x2c),
        Color32::from_rgb(0xe4, 0xe4, 0xe6),
    )
}
pub fn link() -> Color32 {
    pick(
        Color32::from_rgb(0x41, 0x9c, 0xff),
        Color32::from_rgb(0x00, 0x68, 0xda),
    )
}

/// Text and the marks drawn with it: white in dark mode, near-black in light mode. Surfaces, strokes and
/// dimmed text are this at lower opacity, so every control reads the same in both.
pub fn ink_rgb() -> [u8; 3] {
    if dark() {
        [255, 255, 255]
    } else {
        [26, 26, 28]
    }
}
pub fn ink(alpha: f32) -> Color32 {
    let [r, g, b] = ink_rgb();
    Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0).round() as u8)
}
/// What sits on an `ink` fill, like the Download button's title.
pub fn on_ink() -> Color32 {
    pick(Color32::BLACK, Color32::WHITE)
}
pub fn black(alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(0, 0, 0, (alpha * 255.0).round() as u8)
}
pub fn text90() -> Color32 {
    ink(0.9)
}
pub fn secondary() -> Color32 {
    ink(0.55)
}
pub fn tertiary() -> Color32 {
    ink(0.4)
}
pub fn surface() -> Color32 {
    ink(0.06)
}
pub fn stroke() -> Color32 {
    ink(if dark() { 0.08 } else { 0.1 })
}
pub fn warning() -> Color32 {
    pick(
        Color32::from_rgba_unmultiplied(255, 159, 10, 230),
        Color32::from_rgb(0xc9, 0x6a, 0x00),
    )
}
pub fn menu_bg() -> Color32 {
    pick(
        Color32::from_rgba_unmultiplied(42, 42, 44, 240),
        Color32::from_rgba_unmultiplied(246, 246, 247, 245),
    )
}
pub fn popover_bg() -> Color32 {
    pick(
        Color32::from_rgba_unmultiplied(38, 38, 40, 248),
        Color32::from_rgba_unmultiplied(250, 250, 251, 250),
    )
}

/// SwiftUI's font weights; Inter has a static face for each one Purrloader uses.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum W {
    Regular,
    Medium,
    Semibold,
    Bold,
}
pub fn family(weight: W) -> FontFamily {
    match weight {
        W::Regular => FontFamily::Proportional,
        W::Medium => FontFamily::Name("Inter-Medium".into()),
        W::Semibold => FontFamily::Name("Inter-SemiBold".into()),
        W::Bold => FontFamily::Name("Inter-Bold".into()),
    }
}
pub fn font(size: f32, weight: W) -> FontId {
    FontId::new(size, family(weight))
}
/// Inter runs a little wider than SF Pro; this keeps line lengths and chip widths like the Mac's.
fn tracking(size: f32) -> f32 {
    -0.011 * size
}

pub fn rich(text: impl Into<String>, size: f32, weight: W, color: Color32) -> RichText {
    RichText::new(text)
        .font(font(size, weight))
        .color(color)
        .extra_letter_spacing(tracking(size))
}

fn job(text: &str, size: f32, weight: W, color: Color32) -> LayoutJob {
    LayoutJob::single_section(
        text.to_owned(),
        TextFormat {
            font_id: font(size, weight),
            color,
            extra_letter_spacing: tracking(size),
            ..Default::default()
        },
    )
}
/// One line of text, cut with an ellipsis at `max_width` if given.
pub fn line(
    ui: &Ui,
    text: &str,
    size: f32,
    weight: W,
    color: Color32,
    max_width: Option<f32>,
) -> Arc<Galley> {
    let mut job = job(text, size, weight, color);
    if let Some(width) = max_width {
        job.wrap = TextWrapping::truncate_at_width(width.max(1.0));
    }
    ui.painter().layout_job(job)
}
/// Wrapped text, at most `rows` lines (0 = no limit).
pub fn wrapped(
    ui: &Ui,
    text: &str,
    size: f32,
    weight: W,
    color: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = job(text, size, weight, color);
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: if rows == 0 { usize::MAX } else { rows },
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}
/// A wrapped label that takes the width it is given.
pub fn paragraph(ui: &mut Ui, text: &str, size: f32, weight: W, color: Color32, rows: usize) {
    let galley = wrapped(ui, text, size, weight, color, ui.available_width(), rows);
    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    ui.painter().galley(rect.min, galley, color);
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (
            "Inter-Regular",
            &include_bytes!("../assets/fonts/Inter-Regular.ttf")[..],
        ),
        (
            "Inter-Medium",
            &include_bytes!("../assets/fonts/Inter-Medium.ttf")[..],
        ),
        (
            "Inter-SemiBold",
            &include_bytes!("../assets/fonts/Inter-SemiBold.ttf")[..],
        ),
        (
            "Inter-Bold",
            &include_bytes!("../assets/fonts/Inter-Bold.ttf")[..],
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    // egui's own fonts stay as fallbacks for scripts and symbols Inter doesn't cover.
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "Inter-Regular".into());
    let fallback = proportional.clone();
    for name in ["Inter-Medium", "Inter-SemiBold", "Inter-Bold"] {
        let mut list = vec![name.to_owned()];
        list.extend(fallback.iter().skip(1).cloned());
        fonts.families.insert(FontFamily::Name(name.into()), list);
    }
    ctx.set_fonts(fonts);
    apply(ctx, true);
}

/// Switches every colour and egui's own visuals to light or dark.
pub fn apply(ctx: &egui::Context, dark: bool) {
    DARK.store(dark, Relaxed);
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    ctx.all_styles_mut(|style| {
        use egui::TextStyle::*;
        style.text_styles = [
            (Small, FontId::proportional(11.0)),
            (Body, FontId::proportional(13.0)),
            (Button, FontId::proportional(13.0)),
            (Heading, font(15.0, W::Semibold)),
            (Monospace, FontId::monospace(11.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 22.0;
        style.spacing.menu_margin = egui::Margin::same(5);
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 7.0,
            floating_width: 3.0,
            handle_min_length: 24.0,
            bar_inner_margin: 3.0,
            bar_outer_margin: 0.0,
            ..egui::style::ScrollStyle::floating()
        };
        style.interaction.selectable_labels = false;
        let v = &mut style.visuals;
        *v = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.panel_fill = bg();
        v.window_fill = matte();
        v.extreme_bg_color = ink(0.07);
        v.faint_bg_color = ink(0.03);
        v.override_text_color = None;
        v.hyperlink_color = link();
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(10, 132, 255, 140);
        v.selection.stroke = Stroke::new(1.0_f32, ink(1.0));
        v.text_cursor.stroke = Stroke::new(1.5_f32, ink(1.0));
        v.window_corner_radius = CornerRadius::same(18);
        v.menu_corner_radius = CornerRadius::same(11);
        v.window_stroke = Stroke::new(1.0_f32, stroke());
        v.window_shadow = Shadow {
            offset: [0, 24],
            blur: 60,
            spread: 0,
            color: black(if dark { 0.55 } else { 0.25 }),
        };
        v.popup_shadow = Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: black(if dark { 0.45 } else { 0.18 }),
        };
        for (w, fill) in [
            (&mut v.widgets.noninteractive, Color32::TRANSPARENT),
            (&mut v.widgets.inactive, ink(0.1)),
            (&mut v.widgets.hovered, ink(0.14)),
            (&mut v.widgets.active, ink(0.18)),
            (&mut v.widgets.open, ink(0.1)),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::NONE;
            w.corner_radius = CornerRadius::same(6);
            w.expansion = 0.0;
        }
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text90());
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, stroke());
        v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text90());
        v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, ink(1.0));
        v.widgets.active.fg_stroke = Stroke::new(1.0_f32, ink(1.0));
        // Scroll handles: the Mac's thin grey bars.
        v.widgets.inactive.bg_fill = ink(0.28);
        v.widgets.hovered.bg_fill = ink(0.42);
        v.widgets.active.bg_fill = ink(0.5);
    });
}
