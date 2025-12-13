use std::path::PathBuf;

use anyhow::Ok;
use clap::Args;
use dialoguer::Select;
use idevice::{IdeviceService, installation_proxy::InstallationProxyClient, usbmuxd::{UsbmuxdAddr, UsbmuxdConnection}};
use plume_utils::{Device, get_device_for_id};

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct DeviceArgs {
    /// Device UDID to target (optional, will prompt if not provided)
    #[arg(short = 'u', long = "udid", value_name = "UDID", conflicts_with = "mac")]
    pub udid: Option<String>,
    /// Install app at specified path to device (.ipa, .app)
    #[arg(short = 'i', long = "install", value_name = "PATH")]
    pub install: Option<PathBuf>,
    /// Install pairing record from specified path to device
    #[arg(short = 'p', long = "pairing", value_name = "MAC", conflicts_with = "mac", requires = "pairing_path")]
    pub pairing: bool,
    /// Path to pairing record to install (i.e. /Documents/pairingFile.plist)
    #[arg(long = "pairing-path", value_name = "PATH", requires = "pairing")]
    pub pairing_path: Option<PathBuf>,
    /// App identifier for the app to use for pairing record installation (optional, will prompt if not provided)
    #[arg(long = "pairing-app-identifier", value_name = "IDENTIFIER")]
    pub pairing_app_identifier: Option<String>,
    /// Install to connected Mac (arm64 only)
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[arg(short = 'm', long = "mac", value_name = "MAC", conflicts_with = "udid")]
    pub mac: bool,
}

pub async fn execute(args: DeviceArgs) -> anyhow::Result<()> {
    let device = if let Some(udid) = args.udid {
        get_device_for_id(&udid).await?
    } else {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if args.mac {
            Device {
                name: "My Mac".to_string(),
                udid: String::new(),
                device_id: 0,
                usbmuxd_device: None,
            }
        } else {
            devices().await?
        }
        
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        devices().await?
    };

    if let Some(app_path) = args.install {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if args.mac {
            log::info!("Installing app at {:?} to connected Mac", app_path);
            device.install_app_mac(&app_path).await?;
            return Ok(())
        }
        
        log::info!("Installing app at {:?} to device {}", app_path, device.name);
        device.install_app(&app_path, |progress| async move {
            log::info!("{}", progress);
        }).await?;
    }

    if args.pairing {
        if let Some(pairing_path) = args.pairing_path {
            log::info!("Installing pairing record from {:?} to device {}", pairing_path, device.name);
            let app_identifier = if let Some(identifier) = args.pairing_app_identifier {
                identifier
            } else {
                apps(&device).await?
            };
            device.install_pairing_record(&app_identifier, pairing_path.to_str().unwrap()).await?;
        }
    }

    Ok(())
}

async fn devices() -> Result<Device, anyhow::Error> {
    let mut muxer = UsbmuxdConnection::default().await
        .map_err(|e| anyhow::anyhow!("Failed to get muxer: {}", e))?;

    let devices = muxer.get_devices().await
        .map_err(|e| anyhow::anyhow!("Failed to get devices: {}", e))?;

    let device_futures: Vec<_> = devices.into_iter()
        .map(|d| {
            Device::new(d)
        })
        .collect();

    let devices = futures::future::join_all(device_futures).await;

    let device_names: Vec<String> = devices.iter()
        .map(|d| d.to_string())
        .collect();

    let selection = Select::new()
        .items(&device_names)
        .default(0)
        .with_prompt("Select a connected device")
        .interact()?;

    Ok(devices[selection].clone())
}

async fn apps(device: &Device) -> Result<String, anyhow::Error> {
    const INSTALLATION_LABEL: &str = "App Installation";
    let p = device.usbmuxd_device.clone().unwrap().to_provider(
        UsbmuxdAddr::from_env_var().unwrap_or_default(),
        INSTALLATION_LABEL,
    );

    let mut lpc = InstallationProxyClient::connect(&p).await
        .map_err(|e| anyhow::anyhow!("Failed to create installation proxy client: {}", e))?;

    let ia = lpc.get_apps(Some("User"), None).await
        .map_err(|e| anyhow::anyhow!("Failed to get installed apps: {}", e))?;

    let app_names: Vec<String> = ia.keys().cloned().collect();

    let selection = Select::new()
        .items(&app_names)
        .default(0)
        .with_prompt("Select an installed app")
        .interact()?;

    Ok(app_names[selection].clone())
}
