#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wxdragon::appearance::AppAppearance;

mod frame;
mod pages;
mod handlers;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    _ = rustls::crypto::ring::default_provider().install_default().unwrap();

    let _ = wxdragon::main(|_| {
        if let Some(app) = wxdragon::app::get_app() {
            app.set_appearance(wxdragon::appearance::Appearance::System);
        }

        frame::PlumeFrame::new().show();
    });
}

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("Idevice error: {0}")]
    Idevice(#[from] idevice::IdeviceError),
    #[error("Core error: {0}")]
    Core(#[from] plume_core::Error),
    #[error("Utils error: {0}")]
    Utils(#[from] plume_utils::Error),
}
