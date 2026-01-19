use std::path::PathBuf;

use auto_launcher::AutoLaunchBuilder;

#[cfg(target_os = "linux")]
use auto_launcher::LinuxLaunchMode;
#[cfg(target_os = "macos")]
use auto_launcher::MacOSLaunchMode;
#[cfg(target_os = "windows")]
use auto_launcher::WindowsEnableMode;

pub(crate) const TRAY_ONLY_ARG: &str = "--tray";

pub(crate) fn start_in_tray_from_args() -> bool {
    std::env::args().any(|arg| arg == "--tray")
}

pub(crate) fn auto_start_enabled() -> bool {
    build_auto_launcher()
        .and_then(|launcher| launcher.is_enabled().map_err(|e| e.to_string()))
        .unwrap_or(false)
}

pub(crate) fn set_auto_start_enabled(enabled: bool) -> Result<(), String> {
    let launcher = build_auto_launcher()?;

    if enabled {
        launcher.enable().map_err(|e| e.to_string())
    } else {
        launcher.disable().map_err(|e| e.to_string())
    }
}

fn build_auto_launcher() -> Result<auto_launcher::AutoLaunch, String> {
    let app_path = resolve_app_path().map_err(|e| format!("Failed to resolve app path: {e}"))?;
    let app_path_string = app_path.to_string_lossy().to_string();

    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(crate::APP_NAME);
    builder.set_app_path(&app_path_string);
    builder.set_args(&[TRAY_ONLY_ARG]);

    #[cfg(target_os = "macos")]
    builder.set_macos_launch_mode(MacOSLaunchMode::LaunchAgent);

    #[cfg(target_os = "windows")]
    builder.set_windows_enable_mode(WindowsEnableMode::CurrentUser);

    #[cfg(target_os = "linux")]
    builder.set_linux_launch_mode(LinuxLaunchMode::XdgAutostart);

    builder.build().map_err(|e| e.to_string())
}

fn resolve_app_path() -> Result<PathBuf, std::io::Error> {
    std::env::current_exe()
}
