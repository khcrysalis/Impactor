use std::thread;

use futures::StreamExt;
use idevice::usbmuxd::{UsbmuxdConnection, UsbmuxdListenEvent};
use plume_core::{
    AnisetteConfiguration, CertificateIdentity, developer::DeveloperSession, store::AccountStore,
};
use plume_shared::get_data_path;
use plume_utils::{Device, Package, Signer, SignerInstallMode, SignerMode};
use tokio::{runtime::Builder, sync::mpsc};

use crate::app::AppMessage;

// -----------------------------------------------------------------------------
// storage
// -----------------------------------------------------------------------------

pub(crate) fn spawn_store_handler(sender: mpsc::UnboundedSender<AppMessage>) {
    thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_io().build().unwrap();

        rt.block_on(async move {
            let store = AccountStore::load(&Some(get_data_path().join("accounts.json")))
                .await
                .unwrap_or_default();

            let _ = sender.send(AppMessage::AccountStoreInitialized(store));
        });
    });
}

// -----------------------------------------------------------------------------
// usbmuxd
// -----------------------------------------------------------------------------
pub(crate) fn spawn_usbmuxd_listener(sender: mpsc::UnboundedSender<AppMessage>) {
    thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_io().build().unwrap();

        rt.block_on(async move {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if let Some(mac_udid) = plume_gestalt::get_udid() {
                    sender
                        .send(AppMessage::DeviceConnected(Device {
                            name: "This Mac".into(),
                            udid: mac_udid,
                            device_id: u32::MAX,
                            usbmuxd_device: None,
                            is_mac: true,
                        }))
                        .ok();
                }
            }

            let Ok(mut muxer) = UsbmuxdConnection::default().await else {
                return;
            };

            if let Ok(devices) = muxer.get_devices().await {
                for dev in devices {
                    let _ = sender.send(AppMessage::DeviceConnected(Device::new(dev).await));
                }
            }

            let Ok(mut stream) = muxer.listen().await else {
                return;
            };

            while let Some(event) = stream.next().await {
                let msg = match event {
                    Ok(UsbmuxdListenEvent::Connected(dev)) => {
                        AppMessage::DeviceConnected(Device::new(dev).await)
                    }
                    Ok(UsbmuxdListenEvent::Disconnected(id)) => AppMessage::DeviceDisconnected(id),
                    Err(e) => AppMessage::Error(e.to_string()),
                };
                let _ = sender.send(msg);
            }
        });
    });
}

// -----------------------------------------------------------------------------
// installer
// -----------------------------------------------------------------------------

pub(crate) fn spawn_package_handler(
    device: Option<Device>,
    selected_package: Package,
    gsa_account: Option<plume_core::store::GsaAccount>,
    signer_settings: plume_utils::SignerOptions,
    callback: impl Fn(String, i32) + Send + Clone + 'static,
) {
    tokio::task::spawn_blocking(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            if let Err(err) = spawn_package_handler_impl(
                device,
                &selected_package,
                gsa_account,
                signer_settings,
                callback.clone(),
            )
            .await
            {
                callback(format!("Error: {}", err), 1);
            }
        });
    });
}

async fn spawn_package_handler_impl(
    device: Option<Device>,
    selected_package: &Package,
    gsa_account: Option<plume_core::store::GsaAccount>,
    signer_settings: plume_utils::SignerOptions,
    callback: impl Fn(String, i32) + Send + Clone + 'static,
) -> Result<(), plume_utils::Error> {
    let package_file: std::path::PathBuf;

    callback("Preparing package...".to_string(), 10);

    match signer_settings.mode {
        SignerMode::Pem => {
            // pem (or "Apple ID") signing is only available for gsa accounts
            let Some(account) = gsa_account else {
                return Err(plume_utils::Error::Other(
                    "GSA account is required for PEM signing".to_string(),
                ));
            };

            callback("Ensuring account is valid...".to_string(), 20);

            let session = DeveloperSession::new(
                account.adsid().clone(),
                account.xcode_gs_token().clone(),
                AnisetteConfiguration::default().set_configuration_path(get_data_path()),
            )
            .await?;

            let team_id = &session.qh_list_teams().await?.teams[0].team_id;

            let identity =
                CertificateIdentity::new_with_session(&session, get_data_path(), None, team_id)
                    .await?;

            callback("Ensuring device is registered...".to_string(), 20);

            if let Some(dev) = &device {
                session
                    .qh_ensure_device(team_id, &dev.name, &dev.udid)
                    .await?;
            }

            callback("Extracting package...".to_string(), 50);

            let mut signer = Signer::new(Some(identity), signer_settings.clone());

            let Ok(bundle) = selected_package.get_package_bundle() else {
                return Err(plume_utils::Error::Other(
                    "Failed to get package bundle".to_string(),
                ));
            };

            callback("Signing package...".to_string(), 70);

            signer
                .modify_bundle(&bundle, &Some(team_id.clone()))
                .await?;
            signer.register_bundle(&bundle, &session, team_id).await?;
            signer.sign_bundle(&bundle).await?;

            package_file = bundle.bundle_dir().to_path_buf();
        }
        SignerMode::Adhoc => {
            let mut signer = Signer::new(None, signer_settings.clone());

            let Ok(bundle) = selected_package.get_package_bundle() else {
                return Err(plume_utils::Error::Other(
                    "Failed to get package bundle".to_string(),
                ));
            };

            callback("Signing package...".to_string(), 70);

            signer.modify_bundle(&bundle, &None).await?;
            signer.sign_bundle(&bundle).await?;

            package_file = bundle.bundle_dir().to_path_buf();
        }
        _ => {
            callback("Extracting package...".to_string(), 50);

            let Ok(bundle) = selected_package.get_package_bundle() else {
                return Err(plume_utils::Error::Other(
                    "Failed to get package bundle".to_string(),
                ));
            };

            package_file = bundle.bundle_dir().to_path_buf();
        }
    }

    match signer_settings.install_mode {
        SignerInstallMode::Install => {
            if let Some(dev) = &device {
                // On x86_64 macs, `is_mac` variable should never be true
                // since its only true if the device is added manually.
                if !dev.is_mac {
                    let callback_clone = callback.clone();
                    let progress_callback = {
                        move |progress: i32| {
                            let callback = callback_clone.clone();

                            async move {
                                callback("Installing...".to_string(), progress);
                            }
                        }
                    };

                    dev.install_app(&package_file, progress_callback).await?;
                } else {
                    plume_utils::install_app_mac(&package_file).await?;
                }
            } else {
                return Err(plume_utils::Error::Other(
                    "No device connected for installation".to_string(),
                ));
            }
        }
        SignerInstallMode::Export => {
            let archive_path = selected_package.get_archive_based_on_path(package_file)?;
            todo!("Export to {}", archive_path.display());
        }
    }

    callback("Finished.".to_string(), 100);

    Ok(())
}

// -----------------------------------------------------------------------------
// pair
// -----------------------------------------------------------------------------

// Spawn pair handler, we dont return anything here because frankly its not
// meaningful enough to the user.
pub fn spawn_pair_handler(device: Option<Device>) {
    tokio::spawn(async move {
        if let Some(device) = device {
            let _ = device.pair().await;
        }
    });
}
