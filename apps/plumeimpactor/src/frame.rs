use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    ptr,
    thread,
};

use grand_slam::{
    AnisetteConfiguration, CertificateIdentity, auth::Account, developer::DeveloperSession
};

use idevice::{
    usbmuxd::{UsbmuxdConnection, UsbmuxdListenEvent},
};

use utils::{Device, Package, PlistInfoTrait, Signer};

use wxdragon::prelude::*;
use futures::StreamExt;
use tokio::{runtime::Builder, sync::mpsc};

use crate::{
    get_data_path,
    handlers::{PlumeFrameMessage, PlumeFrameMessageHandler},
    keychain::AccountCredentials,
    pages::{
        LoginDialog, DefaultPage, InstallPage, SettingsDialog, WINDOW_SIZE,
        create_default_page, create_install_page, create_login_dialog, create_settings_dialog,
    },
};

#[cfg(target_os = "windows")]
const INSTALLER_IMAGE_BYTES: &[u8] = include_bytes!("../../../package/windows/icon.rgba");
#[cfg(target_os = "windows")]
const INSTALLER_IMAGE_SIZE: u32 = 128;

pub const APP_NAME: &str = concat!(env!("CARGO_PKG_NAME"), " – Version ", env!("CARGO_PKG_VERSION"));

pub struct PlumeFrame {
    pub frame: Frame,
    pub default_page: DefaultPage,
    pub install_page: InstallPage,
    pub usbmuxd_picker: Choice,

    pub add_ipa_button: Button,
    pub apple_id_button: Button,

    pub login_dialog: LoginDialog,
    pub settings_dialog: SettingsDialog,
}

impl PlumeFrame {
    pub fn new() -> Self {
        let frame = Frame::builder()
            .with_title(APP_NAME)
            .with_size(Size::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
            .with_style(FrameStyle::CloseBox | FrameStyle::MinimizeBox | FrameStyle::Caption | FrameStyle::SystemMenu)
            .build();

        #[cfg(target_os = "windows")]
        {
            let bitmap = Bitmap::from_rgba(
                INSTALLER_IMAGE_BYTES,
                INSTALLER_IMAGE_SIZE,
                INSTALLER_IMAGE_SIZE,
            ).unwrap();

            frame.set_icon(&bitmap);
        }

        let sizer = BoxSizer::builder(Orientation::Vertical).build();

        let top_row = BoxSizer::builder(Orientation::Horizontal).build();

        let add_ipa_button = Button::builder(&frame).with_label("Import").build();
        let device_picker = Choice::builder(&frame).build();
        let apple_id_button = Button::builder(&frame).with_label("Settings").build();

        top_row.add(&add_ipa_button, 0, SizerFlag::All, 0);
        top_row.add_spacer(13);
        top_row.add(&device_picker, 1, SizerFlag::Expand | SizerFlag::All, 0);
        top_row.add_spacer(13);
        top_row.add(&apple_id_button, 0, SizerFlag::All, 0);

        let default_page = create_default_page(&frame);
        let install_page = create_install_page(&frame);
        sizer.add_sizer(&top_row, 0, SizerFlag::Expand | SizerFlag::All, 13);
        sizer.add(
            &default_page.panel,
            1,
            SizerFlag::Expand | SizerFlag::All,
            0,
        );
        sizer.add(
            &install_page.panel,
            1,
            SizerFlag::Expand | SizerFlag::All,
            0,
        );
        frame.set_sizer(sizer, true);
        install_page.panel.hide();

        let mut s = Self {
            frame: frame.clone(),
            default_page,
            install_page,
            usbmuxd_picker: device_picker,
            add_ipa_button,
            apple_id_button,
            login_dialog: create_login_dialog(&frame),
            settings_dialog: create_settings_dialog(&frame),
        };

        s.setup_event_handlers();

        s
    }

    pub fn show(&mut self) {
        self.frame.show(true);
        self.frame.centre();
        self.frame.set_extra_style(ExtraWindowStyle::ProcessIdle);
    }
}

// MARK: - Event Handlers

impl PlumeFrame {
    fn setup_event_handlers(&mut self) {
        let (sender, receiver) = mpsc::unbounded_channel::<PlumeFrameMessage>();
        let message_handler = self.setup_idle_handler(receiver);
        Self::spawn_background_threads(sender.clone());
        self.bind_widget_handlers(sender, message_handler);
    }

    fn setup_idle_handler(
        &self,
        receiver: mpsc::UnboundedReceiver<PlumeFrameMessage>,
    ) -> Rc<RefCell<PlumeFrameMessageHandler>> {
        let message_handler = Rc::new(RefCell::new(PlumeFrameMessageHandler::new(
            receiver,
            unsafe { ptr::read(self) },
        )));

        let handler_for_idle = message_handler.clone();
        self.frame.on_idle(move |event_data| {
            if let WindowEventData::Idle(event) = event_data {
                event.request_more(handler_for_idle.borrow_mut().process_messages());
            }
        });

        message_handler
    }

    fn spawn_background_threads(sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        Self::spawn_usbmuxd_listener(sender.clone());
        Self::spawn_auto_login_thread(sender);
    }

    fn spawn_usbmuxd_listener(sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        thread::spawn(move || {
            let rt = Builder::new_current_thread().enable_io().build().unwrap();
            rt.block_on(async move {
                let mut muxer = match UsbmuxdConnection::default().await {
                    Ok(muxer) => muxer,
                    Err(e) => {
                        sender.send(PlumeFrameMessage::Error(format!("Failed to connect to usbmuxd: {}", e))).ok();
                        return;
                    }
                };

                match muxer.get_devices().await {
                    Ok(devices) => {
                        for dev in devices {
                            sender.send(PlumeFrameMessage::DeviceConnected(Device::new(dev).await)).ok();
                        }
                    }
                    Err(e) => {
                        sender.send(PlumeFrameMessage::Error(format!("Failed to get initial device list: {}", e))).ok();
                    }
                }

                let mut stream = match muxer.listen().await {
                    Ok(stream) => stream,
                    Err(e) => {
                        sender.send(PlumeFrameMessage::Error(format!("Failed to listen for events: {}", e))).ok();
                        return;
                    }
                };

                while let Some(event) = stream.next().await {
                    let msg = match event {
                        Ok(dev_event) => match dev_event {
                            UsbmuxdListenEvent::Connected(dev) => {
                                PlumeFrameMessage::DeviceConnected(Device::new(dev).await)
                            }
                            UsbmuxdListenEvent::Disconnected(device_id) => {
                                PlumeFrameMessage::DeviceDisconnected(device_id)
                            }
                        },
                        Err(e) => {
                            PlumeFrameMessage::Error(format!("Failed to listen for events: {}", e))
                        }
                    };

                    if sender.send(msg).is_err() {
                        break;
                    }
                }
            });
        });
    }

    fn spawn_auto_login_thread(sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        thread::spawn(move || {
            let creds = AccountCredentials;

            let (email, password) = match (creds.get_email(), creds.get_password()) {
                (Ok(email), Ok(password)) => (email, password),
                _ => { return; }
            };

            match run_login_flow(sender.clone(), &email, &password) {
                Ok(account) => {
                    sender.send(PlumeFrameMessage::AccountLogin(account)).ok();
                }
                Err(e) => {
                    sender.send(PlumeFrameMessage::AccountDeleted).ok();
                    sender.send(PlumeFrameMessage::Error(format!("Login error: {}", e))).ok();
                }
            }
        });
    }
}

// MARK: - Button Handlers

impl PlumeFrame {
    fn bind_widget_handlers(
        &mut self,
        sender: mpsc::UnboundedSender<PlumeFrameMessage>,
        message_handler: Rc<RefCell<PlumeFrameMessageHandler>>,
    ) {
        // MARK: Device Picker

        self.usbmuxd_picker.on_selection_changed( {
            let message_handler = message_handler.clone();
            let picker_clone = self.usbmuxd_picker.clone();
            move |_| {
                let mut handler = message_handler.borrow_mut();
                handler.usbmuxd_selected_device_id = picker_clone
                    .get_selection()
                    .and_then(|i| handler.usbmuxd_device_list.get(i as usize))
                    .map(|item| item.usbmuxd_device.device_id.to_string());
            }
        });

        // MARK: Apple ID / Login Dialog

        self.apple_id_button.on_click({
            let account_dialog = Rc::new(self.settings_dialog.clone());
            move |_| {
                account_dialog.dialog.show(true);
            }
        });
        
        self.settings_dialog.set_logout_handler({
            let message_handler = message_handler.clone();
            let sender = sender.clone();
            let login_dialog = self.login_dialog.clone();
            move || {
                if message_handler.borrow().account_credentials.is_some() {
                    sender.send(PlumeFrameMessage::AccountDeleted).ok(); 
                } else {
                    login_dialog.dialog.show(true);
                }
            }
        });

        // MARK: File Drop/Open Handlers

        fn process_package_file(sender: mpsc::UnboundedSender<PlumeFrameMessage>, file_path: PathBuf) {
            match Package::new(file_path) {
                Ok(package) => {
                    sender.send(PlumeFrameMessage::PackageSelected(package)).ok();
                }
                Err(e) => {
                    sender.send(PlumeFrameMessage::Error(format!("Failed to open package: {}", e))).ok();
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        self.default_page.set_file_handlers({
            let sender = sender.clone();
            move |file_path| process_package_file(sender.clone(), PathBuf::from(file_path))
        });

        self.add_ipa_button.on_click({
            let sender = sender.clone();
            let handler_for_import = self.frame.clone();
            move |_| {
                let dialog = FileDialog::builder(&handler_for_import)
                    .with_message("Open IPA File")
                    .with_style(FileDialogStyle::default() | FileDialogStyle::Open)
                    .with_wildcard("IPA files (*.ipa;*.tipa)|*.ipa;*.tipa")
                    .build();

                if dialog.show_modal() != ID_OK {
                    return;
                }

                if let Some(file_path) = dialog.get_path() {
                    process_package_file(sender.clone(), PathBuf::from(file_path));
                }
            }
        });

        // MARK: Install Page Handlers

        self.install_page.set_cancel_handler({
            let sender = sender.clone();
            move || {
                sender.send(PlumeFrameMessage::PackageDeselected).ok();
            }
        });
        
        self.install_page.set_install_handler({
            let message_handler = message_handler.clone();
            let sender = sender.clone();
            move || {
                let binding = message_handler.borrow();

                let Some(selected_device) = binding.usbmuxd_selected_device_id.as_deref() else {
                    sender.send(PlumeFrameMessage::Error("No device selected for installation.".to_string())).ok();
                    return;
                };
                
                let Some(selected_package) = binding.package_selected.as_ref() else {
                    sender.send(PlumeFrameMessage::Error("No package selected for installation.".to_string())).ok();
                    return;
                };

                let Some(selected_account) = binding.account_credentials.as_ref() else {
                    sender.send(PlumeFrameMessage::Error("No Apple ID account available for installation.".to_string())).ok();
                    return;
                };
                
                let mut signer_settings = binding.signer_settings.clone();
                binding.plume_frame.install_page.update_fields(&mut signer_settings);

                let package = selected_package.clone();
                let account = selected_account.clone();
                let device_id = selected_device.to_string();
                let sender_clone = sender.clone();

                thread::spawn(move || {
                    let rt = Builder::new_current_thread().enable_all().build().unwrap();

                    let install_result = rt.block_on(async {
                        let session = DeveloperSession::with(account.clone());

                        sender_clone.send(PlumeFrameMessage::InstallProgress(10, Some("Ensuring current device is registered...".to_string()))).ok();

                        let mut usbmuxd = UsbmuxdConnection::default()
                            .await
                            .map_err(|e| format!("usbmuxd connect error: {e}"))?;
                        let usbmuxd_device = usbmuxd.get_devices()
                            .await
                            .map_err(|e| format!("usbmuxd device list error: {e}"))?
                            .into_iter()
                            .find(|d| d.device_id.to_string() == device_id)
                            .ok_or_else(|| format!("Device ID {device_id} not found"))?;

                        let device = Device::new(usbmuxd_device.clone()).await;
                        
                        let teams = session.qh_list_teams()
                            .await
                            .map_err(|e| format!("Failed to list teams: {}", e))?.teams;
                        
                        if teams.is_empty() {
                            return Err("No teams available for the Apple ID account.".to_string());
                        }
                        
                        let team_id = if teams.len() == 1 {
                            &teams[0].team_id
                        } else {
                            let team_names: Vec<String> = teams.iter()
                                .map(|t| format!("{} ({})", t.name, t.team_id))
                                .collect();
                            
                            let (tx, rx) = std::sync::mpsc::channel();
                            sender_clone.send(PlumeFrameMessage::RequestTeamSelection(team_names, tx)).ok();
                            
                            let selected_index = rx.recv()
                                .map_err(|_| "Team selection cancelled".to_string())?
                                .map_err(|e| format!("Team selection error: {}", e))?;
                            
                            &teams[selected_index as usize].team_id
                        };

                        let cert_identity = CertificateIdentity::new_with_session(
                            &session,
                            get_data_path(),
                            None,
                            team_id,
                        ).await.map_err(|e| e.to_string())?;

                        let mut signer = Signer::new(
                            Some(cert_identity),
                            signer_settings.clone(),
                        );

                        session.qh_ensure_device(
                            team_id,
                            &device.name,
                            &device.uuid,
                        )
                        .await
                        .map_err(|e| format!("Failed to ensure device is registered: {}", e))?;
                                    
                        sender_clone.send(PlumeFrameMessage::InstallProgress(20, Some("Extracting package...".to_string()))).ok();
                        
                        let bundle = package.get_package_bundle()
                            .map_err(|e| format!("Failed to get package bundle: {}", e))?;

                        signer.modify_bundle(&bundle, &Some(team_id.clone()))
                            .await
                            .map_err(|e| format!("Failed to modify bundle: {}", e))?;

                        sender_clone.send(PlumeFrameMessage::InstallProgress(30, Some(format!("Registering {}...", bundle.get_name().unwrap_or_default())))).ok();

                        signer.register_bundle(&bundle, &session, &team_id)
                            .await
                            .map_err(|e| format!("Failed to register bundle: {}", e))?;

                        sender_clone.send(PlumeFrameMessage::InstallProgress(50, Some(format!("Signing {}...", bundle.get_name().unwrap_or_default())))).ok();

                        signer.sign_bundle(&bundle).await
                            .map_err(|e| format!("Failed to sign bundle: {}", e))?;

                        let progress_callback = {
                            let sender = sender_clone.clone();
                            move |progress: i32| {
                                let sender = sender.clone();
                                async move {
                                    sender.send(PlumeFrameMessage::InstallProgress(progress, None)).ok();
                                }
                            }
                        };

                        device.install_app(&bundle.bundle_dir(), progress_callback).await
                            .map_err(|e| format!("Failed to install app: {}", e))?;

                        if signer_settings.app.supports_pairing_file() {
                            if let (Some(custom_identifier), Some(pairing_file_bundle_path)) = (
                                signer.options.custom_identifier.as_ref(),
                                signer_settings.app.pairing_file_path(),
                            ) {
                                sender_clone.send(PlumeFrameMessage::InstallProgress(90, Some("Installing pairing record...".to_string()))).ok();
                                device.install_pairing_record(custom_identifier, &pairing_file_bundle_path)
                                    .await
                                    .map_err(|e| format!("Failed to install pairing record: {}", e))?;
                            }
                        }

                        Ok::<_, String>(())
                    });

                    if let Err(e) = install_result {
                        sender_clone.send(PlumeFrameMessage::InstallProgress(100, None)).ok();
                        sender_clone.send(PlumeFrameMessage::Error(format!("{}", e))).ok();
                        return;
                    }
                });
            }
        });
        
        // MARK: Login Dialog "Next" Button

        self.login_dialog.set_next_handler({
            let frame = self.frame.clone();
            let login_dialog = self.login_dialog.clone();
            move || {
                let email = login_dialog.get_email();
                let password = login_dialog.get_password();

                if email.trim().is_empty() || password.is_empty() {
                    let dialog = MessageDialog::builder(
                        &frame,
                        "Please enter both email and password.",
                        "Missing Information",
                    )
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                    .build();
                    dialog.show_modal();
                    return;
                }

                login_dialog.clear_fields();

                thread::spawn({
                    let email = email.clone();
                    let password = password.clone();
                    let sender = sender.clone();
                    move || {
                        match run_login_flow(sender.clone(), &email, &password) {
                            Ok(account) => {
                                sender.send(PlumeFrameMessage::AccountLogin(account)).ok();

                                if let Err(e) = AccountCredentials.set_credentials(email, password) {
                                    sender.send(PlumeFrameMessage::Error(format!("Failed to save credentials: {}", e))).ok();
                                    return;
                                }
                            },
                            Err(e) => {
                                sender.send(PlumeFrameMessage::Error(format!("Login failed: {}", e))).ok();
                            },
                        }
                    }
                });
            }
        });

    }
}

// MARK: - Login flow

pub fn run_login_flow(
    sender: mpsc::UnboundedSender<PlumeFrameMessage>,
    email: &String,
    password: &String,
) -> Result<Account, String> {
    let anisette_config = AnisetteConfiguration::default()
        .set_configuration_path(get_data_path());

    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    
    let (code_tx, code_rx) = std::sync::mpsc::channel::<Result<String, String>>();

    let account_result = rt.block_on(Account::login(
        || Ok((email.clone(), password.clone())),
        || {
            if sender
                .send(PlumeFrameMessage::AwaitingTwoFactorCode(code_tx.clone()))
                .is_err()
            {
                return Err("Failed to send 2FA request to main thread.".to_string());
            }
            match code_rx.recv() {
                Ok(result) => result,
                Err(_) => Err("2FA process cancelled or main thread error.".to_string()),
            }
        },
        anisette_config,
    ));

    account_result.map_err(|e| e.to_string())
}
