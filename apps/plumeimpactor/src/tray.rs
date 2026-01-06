#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicIsize, Ordering},
};
use std::{cell::RefCell, rc::Rc, sync::mpsc as std_mpsc};

use tray_icon::{
    Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{MenuEvent, MenuId},
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow},
};

#[cfg(target_os = "windows")]
pub fn setup_tray(
    tray: &Rc<RefCell<Option<TrayIcon>>>,
    ctx: &egui::Context,
    win32_hwnd: Arc<AtomicIsize>,
) -> std_mpsc::Receiver<MenuEvent> {
    let icon = load_tray_icon();

    let tray_icon = TrayIconBuilder::new()
        .with_menu_on_left_click(false)
        .with_icon(icon)
        .with_tooltip(crate::APP_NAME)
        .build()
        .unwrap();

    let (menu_tx, menu_rx) = std_mpsc::channel();

    MenuEvent::set_event_handler(Some({
        let ctx = ctx.clone();
        let win32_hwnd = win32_hwnd.clone();
        let menu_tx = menu_tx.clone();
        move |event: MenuEvent| {
            if event.id.as_ref() == "quit" {
                std::process::exit(0);
            }
            if event.id.as_ref() == "open" {
                restore_window_from_tray(win32_hwnd.load(Ordering::Acquire));
            }

            let _ = menu_tx.send(event);
            ctx.request_repaint();
        }
    }));

    TrayIconEvent::set_event_handler(Some({
        let ctx = ctx.clone();
        move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                restore_window_from_tray(win32_hwnd.load(Ordering::Acquire));
                let _ = menu_tx.send(MenuEvent {
                    id: MenuId::new("open"),
                });
                ctx.request_repaint();
            }
        }
    }));

    tray.borrow_mut().replace(tray_icon);

    menu_rx
}

#[cfg(target_os = "windows")]
fn restore_window_from_tray(hwnd: isize) {
    if hwnd == 0 {
        return;
    }

    unsafe {
        _ = ShowWindow(hwnd as HWND, SW_SHOW);
        _ = ShowWindow(hwnd as HWND, SW_RESTORE);
        _ = SetForegroundWindow(hwnd as HWND);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn setup_tray(
    tray: &Rc<RefCell<Option<TrayIcon>>>,
    ctx: &egui::Context,
) -> std_mpsc::Receiver<MenuEvent> {
    let icon = load_tray_icon();

    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(crate::APP_NAME)
        .build()
        .unwrap();

    let (menu_tx, menu_rx) = std_mpsc::channel();

    MenuEvent::set_event_handler(Some({
        let ctx = ctx.clone();
        move |event: MenuEvent| {
            let _ = menu_tx.send(event);
            ctx.request_repaint();
        }
    }));

    tray.borrow_mut().replace(tray_icon);

    menu_rx
}

// -----------------------------------------------------------------------------
// Images
// -----------------------------------------------------------------------------

fn load_tray_icon() -> Icon {
    let bytes = include_bytes!("./tray.png");
    let image = image::load_from_memory(bytes)
        .expect("tray.png is invalid")
        .to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("tray icon data invalid")
}
