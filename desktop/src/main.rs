#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod icons;
mod platform;
mod settings_ui;
mod theme;
mod widgets;

use eframe::egui;
use purrloader::store::Paths;

fn main() {
    if std::env::args().any(|s| s == "--version") {
        println!("Purrloader {}", purrloader::VERSION);
        return;
    }
    if let Err(error) = run() {
        eprintln!("Purrloader couldn't start: {error:#}");
        rfd::MessageDialog::new()
            .set_title("Purrloader couldn't start")
            .set_description(format!("{error:#}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}
fn run() -> anyhow::Result<()> {
    let smoke = std::env::args().any(|s| s == "--smoke-test");
    // Smoke tests never touch a user's queue or start network requests.
    let smoke_data = if smoke {
        Some(tempfile::tempdir()?)
    } else {
        None
    };
    let paths = if let Some(temp) = &smoke_data {
        Paths::new(
            temp.path().join("data"),
            temp.path().join("downloads"),
            temp.path().into(),
        )?
    } else {
        Paths::discover()?
    };
    let lock = match paths.lock() {
        Ok(lock) => lock,
        Err(error) => {
            if !smoke {
                let _ = std::fs::write(paths.root.join("activate"), b"show");
            }
            // A second launch activates the already-running window.
            if error
                .root_cause()
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::WouldBlock)
            {
                return Ok(());
            }
            return Err(error);
        }
    };
    #[cfg(windows)]
    let blur = platform::backdrop::supported();
    #[cfg(not(windows))]
    let blur = false;
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Purrloader")
            .with_app_id("purrloader")
            .with_inner_size([780.0, 660.0])
            .with_min_inner_size([720.0, 580.0])
            .with_transparent(blur)
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "Purrloader",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, paths, lock, smoke, blur)?))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
