#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod listeners;
mod login;

use eframe::{NativeOptions, egui};
use std::{cell::RefCell, rc::Rc};
use tokio::sync::mpsc;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

fn load_tray_icon() -> Icon {
    let bytes = include_bytes!("./tray.png");
    let image = image::load_from_memory(bytes)
        .expect("tray.png is invalid")
        .to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("tray icon data invalid")
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    env_logger::init();
    _ = rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    #[cfg(target_os = "linux")]
    {
        gtk::init().expect("GTK init failed");
    }

    let (tx, rx) = mpsc::unbounded_channel();
    listeners::spawn_usbmuxd_listener(tx.clone());
    listeners::spawn_store_handler(tx.clone());

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 400.0])
            .with_resizable(false),
        run_and_return: true,
        ..Default::default()
    };

    let tray = Rc::new(RefCell::new(None::<TrayIcon>));

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_| {
            tray.borrow_mut().replace(
                TrayIconBuilder::new()
                    .with_tooltip(APP_NAME)
                    .with_icon(load_tray_icon())
                    .build()
                    .unwrap(),
            );

            Ok(Box::new(app::ImpactorApp {
                receiver: Some(rx),
                tray_icon: tray.clone(),
                sender: Some(tx),
                ..Default::default()
            }))
        }),
    )
}
