#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::refresh::spawn_refresh_daemon;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use single_instance::SingleInstance;

mod appearance;
mod defaults;
mod refresh;
mod screen;
mod subscriptions;
mod startup;
mod tray;

pub const APP_NAME: &str = "Impactor";
pub const APP_NAME_VERSIONED: &str = concat!("Impactor", " - Version ", env!("CARGO_PKG_VERSION"));

fn main() -> iced::Result {
    env_logger::init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let _single_instance = match SingleInstance::new(APP_NAME) {
        Ok(instance) => {
            if !instance.is_single() {
                log::info!("Another instance is already running; exiting.");
                return Ok(());
            }
            Some(instance)
        }
        Err(err) => {
            log::warn!("Failed to acquire single-instance lock: {err}");
            None
        }
    };

    #[cfg(target_os = "linux")]
    {
        gtk::init().expect("GTK init failed");
    }

    #[cfg(target_os = "macos")]
    {
        notify_rust::get_bundle_identifier_or_default("Impactor");
        let _ = notify_rust::set_application("dev.khcrysalis.PlumeImpactor");
    }

    let (_daemon_handle, connected_devices) = spawn_refresh_daemon();
    screen::set_refresh_daemon_devices(connected_devices);

    iced::daemon(
        screen::Impactor::new,
        screen::Impactor::update,
        screen::Impactor::view,
    )
    .subscription(screen::Impactor::subscription)
    .title(APP_NAME_VERSIONED)
    .theme(appearance::PlumeTheme::default().to_iced_theme())
    .settings(defaults::default_settings())
    .run()
}
