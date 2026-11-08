//! Icons drawn to match the SF Symbols the Mac app uses (SF Symbols themselves may only be used on Apple
//! platforms). Each is on a 24-unit grid and rendered white, then tinted.
use crate::theme::W;
use eframe::egui::{self, Color32, Image, Ui, load::Bytes};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

enum Shape {
    /// Outlines drawn with the stroke; `<fill>` parts are solid.
    Stroke(&'static str),
    /// A solid shape with a mark knocked out of it, like SF Symbols' ".fill" circles.
    Knockout(&'static str, &'static str),
}

fn shape(name: &str) -> Option<Shape> {
    use Shape::*;
    Some(match name {
        "link" => Stroke(
            r#"<path d="M10.2 13.8a4 4 0 0 0 5.66 0l3.3-3.3a4 4 0 0 0-5.66-5.66l-1.2 1.2"/><path d="M13.8 10.2a4 4 0 0 0-5.66 0l-3.3 3.3a4 4 0 0 0 5.66 5.66l1.2-1.2"/>"#,
        ),
        "folder" => Stroke(
            r#"<path d="M3.5 7.2c0-1.2.9-2.1 2.1-2.1h3.6c.6 0 1.1.2 1.5.6l1.3 1.3h6.4c1.2 0 2.1.9 2.1 2.1v8.2c0 1.2-.9 2.1-2.1 2.1H5.6c-1.2 0-2.1-.9-2.1-2.1z"/><path d="M3.5 9.6h17"/>"#,
        ),
        "sliders" => Stroke(
            r#"<path d="M3.5 6.5h5M12.5 6.5h8M3.5 12h11M18.5 12h2M3.5 17.5h3M10.5 17.5h10"/><circle cx="10.5" cy="6.5" r="2"/><circle cx="16.5" cy="12" r="2"/><circle cx="8.5" cy="17.5" r="2"/>"#,
        ),
        "chevronDown" => Stroke(r#"<path d="M5 9l7 7 7-7"/>"#),
        "chevronRight" => Stroke(r#"<path d="M9 5l7 7-7 7"/>"#),
        "chevronUpDown" => Stroke(r#"<path d="M8 9.5l4-4 4 4M8 14.5l4 4 4-4"/>"#),
        "xmark" => Stroke(r#"<path d="M6 6l12 12M18 6L6 18"/>"#),
        "arrowDown" => Stroke(r#"<path d="M12 4v16M5.5 13.5L12 20l6.5-6.5"/>"#),
        "pauseFill" => Stroke(
            r#"<rect FILL x="6" y="4.5" width="4" height="15" rx="1.2"/><rect FILL x="14" y="4.5" width="4" height="15" rx="1.2"/>"#,
        ),
        "playFill" => Stroke(
            r#"<path FILL d="M7.5 4.9c0-1 1.1-1.6 1.9-1.1l10.2 6.7c.8.5.8 1.6 0 2.1L9.4 19.3c-.8.5-1.9-.1-1.9-1.1z"/>"#,
        ),
        "trash" => Stroke(
            r#"<path d="M4.5 6.5h15M9.5 6.5V4.8c0-.6.5-1.1 1.1-1.1h2.8c.6 0 1.1.5 1.1 1.1v1.7M6.5 6.5l.9 12.3c.1 1 .9 1.7 1.9 1.7h5.4c1 0 1.8-.7 1.9-1.7l.9-12.3M10 10.5v6M14 10.5v6"/>"#,
        ),
        "arrowClockwise" => {
            Stroke(r#"<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.6 4.2v4.2h-4.2"/>"#)
        }
        "magnifyingglass" => {
            Stroke(r#"<circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.3 15.3l5 5"/>"#)
        }
        "textAlignLeft" => Stroke(r#"<path d="M4 6h16M4 10h10M4 14h16M4 18h10"/>"#),
        "checkmark" => Stroke(r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#),
        "circle" => Stroke(r#"<circle cx="12" cy="12" r="8.5"/>"#),
        "checkmarkCircleFill" => Knockout(
            r#"<circle cx="12" cy="12" r="10"/>"#,
            r#"<path d="M7.5 12.3l3 3 6-6.3"/>"#,
        ),
        "exclamationCircleFill" => Knockout(
            r#"<circle cx="12" cy="12" r="10"/>"#,
            r#"<path d="M12 7v6"/><circle cx="12" cy="16.6" r=".2"/>"#,
        ),
        "pauseCircleFill" => Knockout(
            r#"<circle cx="12" cy="12" r="10"/>"#,
            r#"<path d="M9.8 8.5v7M14.2 8.5v7"/>"#,
        ),
        "xmarkCircleFill" => Knockout(
            r#"<circle cx="12" cy="12" r="10"/>"#,
            r#"<path d="M8.8 8.8l6.4 6.4M15.2 8.8l-6.4 6.4"/>"#,
        ),
        "exclamationTriangleFill" => Knockout(
            r#"<path d="M10.3 3.9c.8-1.3 2.6-1.3 3.4 0l7.8 13.6c.8 1.3-.2 3-1.7 3H4.2c-1.5 0-2.5-1.7-1.7-3z"/>"#,
            r#"<path d="M12 8.6v5"/><circle cx="12" cy="16.9" r=".2"/>"#,
        ),
        "sparkles" => Stroke(
            r#"<path FILL d="M10 3.5l1.6 4.4c.3.8.9 1.4 1.7 1.7l4.4 1.6-4.4 1.6c-.8.3-1.4.9-1.7 1.7L10 18.9l-1.6-4.4c-.3-.8-.9-1.4-1.7-1.7L2.3 11.2l4.4-1.6c.8-.3 1.4-.9 1.7-1.7z"/><path FILL d="M18.5 2.5l.7 1.8c.1.3.4.6.7.7l1.8.7-1.8.7c-.3.1-.6.4-.7.7l-.7 1.8-.7-1.8c-.1-.3-.4-.6-.7-.7L15.3 5.7l1.8-.7c.3-.1.6-.4.7-.7z"/>"#,
        ),
        "listBullet" => Stroke(
            r#"<path d="M9 6.5h11M9 12h11M9 17.5h11"/><circle FILL cx="4.6" cy="6.5" r="1.4"/><circle FILL cx="4.6" cy="12" r="1.4"/><circle FILL cx="4.6" cy="17.5" r="1.4"/>"#,
        ),
        "gearshape" => Stroke(
            r#"<path d="M10.4 3.3h3.2l.5 2.3c.6.2 1.1.5 1.6.9l2.2-.8 1.6 2.8-1.8 1.5c.1.6.1 1.2 0 1.8l1.8 1.5-1.6 2.8-2.2-.8c-.5.4-1 .7-1.6.9l-.5 2.3h-3.2l-.5-2.3c-.6-.2-1.1-.5-1.6-.9l-2.2.8-1.6-2.8 1.8-1.5c-.1-.6-.1-1.2 0-1.8L4.5 8.5l1.6-2.8 2.2.8c.5-.4 1-.7 1.6-.9z" transform="translate(0 1.5)"/><circle cx="12" cy="12" r="2.8"/>"#,
        ),
        "infoCircle" => Stroke(
            r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><circle FILL cx="12" cy="7.6" r="1.25"/>"#,
        ),
        "docOnDoc" => Stroke(
            r#"<rect x="8" y="8" width="11.5" height="13" rx="2.2"/><path d="M5 15.5V5.7c0-1.2 1-2.2 2.2-2.2h7.3"/>"#,
        ),
        "arrowDownCircle" => {
            Stroke(r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v9M8.3 13l3.7 3.7 3.7-3.7"/>"#)
        }
        "power" => Stroke(r#"<path d="M12 3.5v8"/><path d="M7.4 6.3a7.5 7.5 0 1 0 9.2 0"/>"#),
        "clock" => Stroke(r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5.2l3.4 2"/>"#),
        "lineHorizontal3" => Stroke(r#"<path d="M4.5 7h15M4.5 12h15M4.5 17h15"/>"#),
        "plus" => Stroke(r#"<path d="M12 5.5v13M5.5 12h13"/>"#),
        "minus" => Stroke(r#"<path d="M5.5 12h13"/>"#),
        _ => return None,
    })
}

fn stroke_width(weight: W) -> f32 {
    match weight {
        W::Regular => 1.9,
        W::Medium => 2.15,
        W::Semibold => 2.4,
        W::Bold => 2.8,
    }
}

fn svg(name: &str, weight: W) -> Option<Vec<u8>> {
    let body = match shape(name)? {
        Shape::Stroke(body) => body.replace("FILL", r##"fill="#fff" stroke="none""##),
        Shape::Knockout(fill, cut) => format!(
            r##"<defs><mask id="k" maskUnits="userSpaceOnUse" x="0" y="0" width="24" height="24"><rect width="24" height="24" fill="#fff" stroke="none"/><g stroke="#000" fill="#000">{cut}</g></mask></defs><g mask="url(#k)" fill="#fff" stroke="none">{fill}</g>"##
        ),
    };
    Some(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##,
        stroke_width(weight)
    ).into_bytes())
}

static CACHE: LazyLock<Mutex<HashMap<String, Arc<[u8]>>>> = LazyLock::new(Default::default);

/// The icon as an image, `size` points square, tinted `color`.
pub fn image(name: &str, size: f32, weight: W, color: Color32) -> Image<'static> {
    let key = format!(
        "{name}-{}",
        match weight {
            W::Regular => "r",
            W::Medium => "m",
            W::Semibold => "s",
            W::Bold => "b",
        }
    );
    let bytes = CACHE
        .lock()
        .unwrap()
        .entry(key.clone())
        .or_insert_with(|| Arc::from(svg(name, weight).unwrap_or_default()))
        .clone();
    Image::from_bytes(format!("bytes://icon/{key}.svg"), Bytes::Shared(bytes))
        .fit_to_exact_size(egui::vec2(size, size))
        .tint(color)
}

/// Paints the icon centred in `rect`.
pub fn paint(ui: &Ui, rect: egui::Rect, name: &str, size: f32, weight: W, color: Color32) {
    let target = egui::Rect::from_center_size(rect.center(), egui::vec2(size, size));
    image(name, size, weight, color).paint_at(ui, target);
}

/// Adds the icon inline, like a label.
pub fn show(ui: &mut Ui, name: &str, size: f32, weight: W, color: Color32) -> egui::Response {
    ui.add(image(name, size, weight, color))
}
