use iced::{Subscription, window};
use idevice::usbmuxd::{UsbmuxdConnection, UsbmuxdListenEvent};
use std::sync::Arc;
use tray_icon::{TrayIconEvent, menu::MenuEvent};

use plume_utils::Device;

use crate::Message;

pub(crate) fn device_listener() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                use iced::futures::{SinkExt, StreamExt};
                let (tx, mut rx) = iced::futures::channel::mpsc::unbounded::<Message>();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move {
                        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                        {
                            if let Some(mac_udid) = plume_gestalt::get_udid() {
                                let _ = tx.unbounded_send(Message::DeviceConnected(Device {
                                    name: "This Mac".into(),
                                    udid: mac_udid,
                                    device_id: u32::MAX,
                                    usbmuxd_device: None,
                                    is_mac: true,
                                }));
                            }
                        }

                        let Ok(mut muxer) = UsbmuxdConnection::default().await else {
                            return;
                        };

                        if let Ok(devices) = muxer.get_devices().await {
                            for dev in devices {
                                let device = Device::new(dev).await;
                                let _ = tx.unbounded_send(Message::DeviceConnected(device));
                            }
                        }

                        let Ok(mut stream) = muxer.listen().await else {
                            return;
                        };

                        while let Some(event) = stream.next().await {
                            let msg = match event {
                                Ok(UsbmuxdListenEvent::Connected(dev)) => {
                                    Message::DeviceConnected(Device::new(dev).await)
                                }
                                Ok(UsbmuxdListenEvent::Disconnected(id)) => {
                                    Message::DeviceDisconnected(id)
                                }
                                Err(_) => continue,
                            };
                            let _ = tx.unbounded_send(msg);
                        }
                    });
                });

                while let Some(message) = rx.next().await {
                    let _ = output.send(message).await;
                }
            },
        )
    })
}

pub(crate) fn tray_subscription() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                use iced::futures::{SinkExt, StreamExt};
                let (tx, mut rx) = iced::futures::channel::mpsc::unbounded::<Message>();

                std::thread::spawn(move || {
                    let menu_channel = MenuEvent::receiver();
                    let tray_channel = TrayIconEvent::receiver();
                    loop {
                        if let Ok(event) = menu_channel.try_recv() {
                            let _ = tx.unbounded_send(Message::TrayMenuClicked(event.id));
                        }

                        if let Ok(event) = tray_channel.try_recv() {
                            match event {
                                TrayIconEvent::DoubleClick {
                                    button: tray_icon::MouseButton::Left,
                                    ..
                                } => {
                                    let _ = tx.unbounded_send(Message::TrayIconClicked);
                                }
                                _ => {}
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(32));
                    }
                });

                while let Some(message) = rx.next().await {
                    let _ = output.send(message).await;
                }
            },
        )
    })
}

pub(crate) fn file_hover_subscription() -> Subscription<Message> {
    let window_events = window::events().filter_map(|(_id, event)| match event {
        window::Event::CloseRequested => Some(Message::HideWindow),
        window::Event::FileHovered(_) => Some(Message::FilesHovered),
        window::Event::FilesHoveredLeft => Some(Message::FilesHoveredLeft),
        window::Event::FileDropped(path) => Some(Message::FilesDropped(vec![path])),
        _ => None,
    });

    window_events
}

// Helper struct to make Arc<Mutex<Receiver>> hashable
#[derive(Clone)]
struct ProgressReceiver(Arc<std::sync::Mutex<std::sync::mpsc::Receiver<(String, i32)>>>);

impl std::hash::Hash for ProgressReceiver {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the pointer address to uniquely identify this receiver
        Arc::as_ptr(&self.0).hash(state);
    }
}

pub(crate) fn installation_progress_listener(
    progress_rx: Option<Arc<std::sync::Mutex<std::sync::mpsc::Receiver<(String, i32)>>>>,
) -> Subscription<Message> {
    match progress_rx {
        Some(rx) => {
            let receiver = ProgressReceiver(rx);
            Subscription::run_with(receiver, |receiver| {
                let rx = receiver.0.clone();
                iced::stream::channel(
                    100,
                    move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                        use iced::futures::{SinkExt, StreamExt};

                        let (tx, mut rx_stream) =
                            iced::futures::channel::mpsc::unbounded::<Message>();

                        std::thread::spawn(move || {
                            loop {
                                let message = {
                                    if let Ok(guard) = rx.lock() {
                                        guard.try_recv().ok()
                                    } else {
                                        None
                                    }
                                };

                                if let Some((status, progress)) = message {
                                    let _ = tx.unbounded_send(Message::InstallationProgress(
                                        status, progress,
                                    ));
                                }

                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                        });

                        while let Some(message) = rx_stream.next().await {
                            let _ = output.send(message).await;
                        }
                    },
                )
            })
        }
        None => Subscription::none(),
    }
}

pub(crate) async fn run_installation(
    device: Option<Device>,
    package: plume_utils::Package,
    account: Option<plume_store::GsaAccount>,
    options: plume_utils::SignerOptions,
    callback: impl Fn(String, i32) + Send + Sync + 'static,
) -> Result<(), String> {
    use plume_core::{AnisetteConfiguration, CertificateIdentity, developer::DeveloperSession};
    use plume_utils::{Signer, SignerInstallMode, SignerMode};

    let package_file: std::path::PathBuf;
    let mut options = options;

    callback("Preparing package...".to_string(), 10);

    match options.mode {
        SignerMode::Pem => {
            let Some(account) = account else {
                return Err("GSA account is required for PEM signing".to_string());
            };

            callback("Ensuring account is valid...".to_string(), 20);

            let session = DeveloperSession::new(
                account.adsid().clone(),
                account.xcode_gs_token().clone(),
                AnisetteConfiguration::default()
                    .set_configuration_path(crate::defaults::get_data_path()),
            )
            .await
            .map_err(|e| e.to_string())?;

            let team_id = &session
                .qh_list_teams()
                .await
                .map_err(|e| e.to_string())?
                .teams[0]
                .team_id;

            let identity = CertificateIdentity::new_with_session(
                &session,
                crate::defaults::get_data_path(),
                None,
                team_id,
            )
            .await
            .map_err(|e| e.to_string())?;

            callback("Ensuring device is registered...".to_string(), 30);

            if let Some(dev) = &device {
                session
                    .qh_ensure_device(team_id, &dev.name, &dev.udid)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            callback("Extracting package...".to_string(), 50);

            let mut signer = Signer::new(Some(identity), options.clone());

            let bundle = package.get_package_bundle().map_err(|e| e.to_string())?;

            callback("Signing package...".to_string(), 70);

            signer
                .modify_bundle(&bundle, &Some(team_id.clone()))
                .await
                .map_err(|e| e.to_string())?;
            signer
                .register_bundle(&bundle, &session, team_id)
                .await
                .map_err(|e| e.to_string())?;
            signer
                .sign_bundle(&bundle)
                .await
                .map_err(|e| e.to_string())?;

            options = signer.options.clone();
            package_file = bundle.bundle_dir().to_path_buf();
        }
        SignerMode::Adhoc => {
            callback("Extracting package...".to_string(), 50);

            let mut signer = Signer::new(None, options.clone());

            let bundle = package.get_package_bundle().map_err(|e| e.to_string())?;

            callback("Signing package...".to_string(), 70);

            signer
                .modify_bundle(&bundle, &None)
                .await
                .map_err(|e| e.to_string())?;
            signer
                .sign_bundle(&bundle)
                .await
                .map_err(|e| e.to_string())?;

            options = signer.options.clone();
            package_file = bundle.bundle_dir().to_path_buf();
        }
        _ => {
            callback("Extracting package...".to_string(), 50);

            let bundle = package.get_package_bundle().map_err(|e| e.to_string())?;

            package_file = bundle.bundle_dir().to_path_buf();
        }
    }

    match options.install_mode {
        SignerInstallMode::Install => {
            if let Some(dev) = &device {
                if !dev.is_mac {
                    callback("Installing...".to_string(), 80);

                    dev.install_app(&package_file, |progress: i32| async move {
                        let _ = progress;
                    })
                    .await
                    .map_err(|e| e.to_string())?;

                    if options.app.supports_pairing_file() {
                        if let (Some(custom_identifier), Some(pairing_file_bundle_path)) = (
                            options.custom_identifier.as_ref(),
                            options.app.pairing_file_path(),
                        ) {
                            let _ = dev
                                .install_pairing_record(
                                    custom_identifier,
                                    &pairing_file_bundle_path,
                                )
                                .await;
                        }
                    }
                } else {
                    callback("Installing...".to_string(), 90);

                    plume_utils::install_app_mac(&package_file)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            } else {
                return Err("No device connected for installation".to_string());
            }
        }
        SignerInstallMode::Export => {
            callback("Exporting...".to_string(), 90);

            let archive_path = package
                .get_archive_based_on_path(package_file)
                .map_err(|e| e.to_string())?;

            let file = rfd::AsyncFileDialog::new()
                .set_title("Save Signed Package As")
                .set_file_name(
                    archive_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("signed_package.ipa"),
                )
                .save_file()
                .await;

            if let Some(save_path) = file {
                tokio::fs::copy(&archive_path, &save_path.path())
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    callback("Finished!".to_string(), 100);

    Ok(())
}

pub(crate) async fn export_certificate(account: plume_store::GsaAccount) -> Result<(), String> {
    use plume_core::{AnisetteConfiguration, CertificateIdentity, developer::DeveloperSession};

    let session = DeveloperSession::new(
        account.adsid().clone(),
        account.xcode_gs_token().clone(),
        AnisetteConfiguration::default().set_configuration_path(crate::defaults::get_data_path()),
    )
    .await
    .map_err(|e| e.to_string())?;

    let team_id = &session
        .qh_list_teams()
        .await
        .map_err(|e| e.to_string())?
        .teams[0]
        .team_id;

    let identity = CertificateIdentity::new_with_session(
        &session,
        crate::defaults::get_data_path(),
        None,
        team_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    let Some(p12_data) = identity.p12_data else {
        return Err("Missing p12 data".to_string());
    };

    let archive_path =
        crate::defaults::get_data_path().join(format!("{}_certificate.p12", team_id));
    tokio::fs::write(&archive_path, p12_data)
        .await
        .map_err(|e| e.to_string())?;

    let file = rfd::AsyncFileDialog::new()
        .set_title("Save Certificate As")
        .set_file_name(
            archive_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("certificate.p12"),
        )
        .save_file()
        .await;

    if let Some(save_path) = file {
        tokio::fs::copy(&archive_path, &save_path.path())
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
