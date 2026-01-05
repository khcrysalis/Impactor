#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod listeners;
mod login;
mod tray;

use std::{
    cell::RefCell,
    env, fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use eframe::NativeOptions;
use eframe::egui;
use tokio::sync::mpsc;
use tray_icon::TrayIcon;

pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() -> eframe::Result<()> {
    env_logger::init();
    _ = rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    #[cfg(target_os = "linux")]
    {
        // SAFETY: wayland is so fucking broken idc
        unsafe {
            env::set_var("WINIT_UNIX_BACKEND", "x11");
            env::remove_var("WAYLAND_DISPLAY");
            env::remove_var("WAYLAND_SOCKET");
        }

        gtk::init().expect("GTK init failed");
    }

    let (tx, rx) = mpsc::unbounded_channel();
    listeners::spawn_usbmuxd_listener(tx.clone());
    listeners::spawn_store_handler(tx.clone());

    let mut options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 400.0])
            .with_resizable(false),
        run_and_return: true,
        ..Default::default()
    };

    // on macOS, just remove the icon entirely, with no icon data..
    // we use the bundle icon instead.
    #[cfg(target_os = "macos")]
    {
        options.viewport.icon = Some(std::sync::Arc::new(egui::IconData::default()));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let icon_bytes: &[u8] = include_bytes!(
            "../../../package/linux/icons/hicolor/64x64/apps/dev.khcrysalis.PlumeImpactor.png"
        );
        let d = eframe::icon_data::from_png_bytes(icon_bytes).expect("The icon data must be valid");
        options.viewport.icon = Some(std::sync::Arc::new(d));
    }

    let tray = Rc::new(RefCell::new(None::<TrayIcon>));
    #[cfg(target_os = "windows")]
    let win32_hwnd = Arc::new(AtomicIsize::new(0));

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|ctx| {
            #[cfg(target_os = "windows")]
            let tray_menu_events = tray::setup_tray(&tray, &ctx.egui_ctx, win32_hwnd.clone());
            #[cfg(not(target_os = "windows"))]
            let tray_menu_events = tray::setup_tray(&tray, &ctx.egui_ctx);

            Ok(Box::new(app::ImpactorApp {
                receiver: Some(rx),
                tray_menu_events: Some(tray_menu_events),
                tray_icon: tray.clone(),
                sender: Some(tx),
                #[cfg(target_os = "windows")]
                win32_hwnd: Some(win32_hwnd.clone()),
                ..Default::default()
            }))
        }),
    )
}

pub fn get_data_path() -> PathBuf {
    let base = if cfg!(windows) {
        env::var("APPDATA").unwrap()
    } else {
        env::var("HOME").unwrap() + "/.config"
    };

    let dir = Path::new(&base).join("PlumeImpactor");

    fs::create_dir_all(&dir).ok();

    dir
}
