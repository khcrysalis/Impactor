#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::refresh::spawn_refresh_daemon;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use single_instance::SingleInstance;

mod appearance;
mod defaults;
mod refresh;
mod screen;
mod startup;
mod subscriptions;
mod tray;

pub const APP_NAME: &str = "Impactor";
pub const APP_NAME_VERSIONED: &str = concat!("Impactor", " - Version ", env!("CARGO_PKG_VERSION"));

fn main() -> iced::Result {
    env_logger::init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

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
        notify_rust::set_application("dev.khcrysalis.PlumeImpactor").ok();
    }

    let (_daemon_handle, connected_devices) = spawn_refresh_daemon();
    screen::set_refresh_daemon_devices(connected_devices);

    // we set this to none so we can avoid prioritizing the gpu
    unsafe {
        std::env::set_var("WGPU_POWER_PREF", "none");
    }

    // We're going to try and try running the iced_daemon with different
    // environment variables so it can run properly
    // RE: https://github.com/claration/Impactor/issues/103, https://github.com/claration/Impactor/issues/90
    run_daemon().or_else(|_| try_with_env("ICED_BACKEND", "tiny-skia"))
}

fn try_with_env(var: &str, value: &str) -> iced::Result {
    unsafe {
        std::env::set_var(var, value);
    }
    run_daemon()
}

fn run_daemon() -> iced::Result {
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
