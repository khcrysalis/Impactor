#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod listeners;
mod login;

use eframe::{NativeOptions, egui};
use std::{cell::RefCell, rc::Rc};
use tokio::sync::mpsc;
use tray_icon::{TrayIcon, TrayIconBuilder};

pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() -> eframe::Result<()> {
    env_logger::init();

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
                    .build()
                    .unwrap(),
            );

            Ok(Box::new(app::ImpactorApp {
                receiver: Some(rx),
                tray_icon: tray.clone(),
                ..Default::default()
            }))
        }),
    )
}
