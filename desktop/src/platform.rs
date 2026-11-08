use crossbeam_channel::Receiver;
use eframe::egui;
use std::path::Path;
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

pub struct Tray {
    pub icon: TrayIcon,
    pub events: Receiver<String>,
}
impl Tray {
    pub fn new(ctx: &egui::Context) -> anyhow::Result<Self> {
        let menu = Menu::new();
        for (id, title) in [
            ("show", "Open Purrloader"),
            ("pause", "Pause all downloads"),
            ("resume", "Resume all downloads"),
            ("history", "History"),
            ("settings", "Settings"),
            ("quit", "Quit Purrloader"),
        ] {
            menu.append(&MenuItem::with_id(id, title, true, None))?;
        }
        let image = image::load_from_memory(include_bytes!("../assets/tray.png"))?.into_rgba8();
        let (width, height) = image.dimensions();
        let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)?;
        let (tx, events) = crossbeam_channel::unbounded();
        let wake = ctx.clone();
        let sender = tx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let _ = sender.send(event.id.0);
            wake.request_repaint();
        }));
        let wake = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick { .. }
            ) {
                let _ = tx.send("show".into());
                wake.request_repaint();
            }
        }));
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("Purrloader")
            .build()?;
        Ok(Self { icon, events })
    }
}
pub fn reveal(file: Option<&Path>, folder: &Path) {
    let path = file.filter(|p| p.exists());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Some(path) = path {
            let _ = std::process::Command::new("explorer.exe")
                .arg(format!("/select,{}", path.display()))
                .creation_flags(0x08000000)
                .spawn();
            return;
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = path {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
        return;
    }
    let _ = open::that_detached(path.and_then(Path::parent).unwrap_or(folder));
}

/// Windows 11 (22H2 and later) draws the blurred, see-through backdrop behind Purrloader's windows, like the
/// Mac app's. Windows 10 and Linux have no dependable equivalent, so their windows keep the solid background.
#[cfg(windows)]
pub mod backdrop {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
    use std::num::NonZeroIsize;
    use windows_sys::{
        Wdk::System::SystemServices::RtlGetVersion,
        Win32::{
            Foundation::{HWND, LPARAM},
            System::{SystemInformation::OSVERSIONINFOW, Threading::GetCurrentProcessId},
            UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId},
        },
        core::BOOL,
    };

    /// Windows 11 22H2: the first version with the system backdrop for ordinary windows. Earlier versions
    /// only offer a blur that stutters when the window is moved or resized.
    pub fn supported() -> bool {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` is a valid, sized OSVERSIONINFOW.
        unsafe { RtlGetVersion(&mut info) == 0 && info.dwBuildNumber >= 22621 }
    }

    pub fn apply(window: impl HasWindowHandle) -> bool {
        window_vibrancy::apply_acrylic(window, None).is_ok()
    }

    /// Applies the backdrop to this process's window with the given title (for Settings and History, whose
    /// handles egui doesn't hand out).
    pub fn apply_to_title(title: &str) -> bool {
        unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the `Vec` passed to EnumWindows below, alive for the whole call.
            unsafe { (*(data as *mut Vec<HWND>)).push(hwnd) };
            1
        }
        let mut windows: Vec<HWND> = Vec::new();
        // SAFETY: the callback only pushes into `windows`, which outlives the call.
        unsafe { EnumWindows(Some(collect), &mut windows as *mut Vec<HWND> as LPARAM) };
        // SAFETY: plain Win32 queries on window handles that EnumWindows just returned.
        let process = unsafe { GetCurrentProcessId() };
        for hwnd in windows {
            let mut owner = 0;
            unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
            if owner != process {
                continue;
            }
            let mut buffer = [0u16; 128];
            let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
            if length <= 0 || String::from_utf16_lossy(&buffer[..length as usize]) != title {
                continue;
            }
            let Some(raw) = NonZeroIsize::new(hwnd as isize) else {
                continue;
            };
            // SAFETY: the handle belongs to a live window of this process.
            let handle = unsafe {
                WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(raw)))
            };
            return apply(handle);
        }
        false
    }
}
