use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, fs, ptr, thread};

use grand_slam::{AnisetteConfiguration, BundleType, MachO, MobileProvision};
use grand_slam::auth::Account;
use grand_slam::developer::DeveloperSession;
use grand_slam::utils::PlistInfoTrait;
use idevice::IdeviceService;
use idevice::lockdown::LockdownClient;
use wxdragon::prelude::*;

use futures::StreamExt;
use idevice::usbmuxd::{UsbmuxdAddr, UsbmuxdConnection, UsbmuxdListenEvent};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

use crate::APP_NAME;
use crate::handlers::{PlumeFrameMessage, PlumeFrameMessageHandler};
use crate::keychain::AccountCredentials;
use crate::pages::login::{AccountDialog, LoginDialog};
use crate::pages::{DefaultPage, InstallPage, create_account_dialog, create_default_page, create_install_page, create_login_dialog};
use crate::utils::{Device, Package};

pub struct PlumeFrame {
    pub frame: Frame,
    pub default_page: DefaultPage,
    pub install_page: InstallPage,
    pub usbmuxd_picker: Choice,

    pub add_ipa_button: Button,
    pub apple_id_button: Button,
    pub login_dialog: LoginDialog,
    pub account_dialog: AccountDialog,
}

#[cfg(target_os = "linux")]
const WINDOW_SIZE: (i32, i32) = (700, 660);
#[cfg(not(target_os = "linux"))]
const WINDOW_SIZE: (i32, i32) = (530, 410);

impl PlumeFrame {
    pub fn new() -> Self {
        let frame = Frame::builder()
            .with_title(APP_NAME)
            .with_size(Size::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
            .with_style(FrameStyle::CloseBox | FrameStyle::MinimizeBox | FrameStyle::Caption | FrameStyle::SystemMenu)
            .build();

        let sizer = BoxSizer::builder(Orientation::Vertical).build();

        let top_panel = Panel::builder(&frame).build();
        let top_row = BoxSizer::builder(Orientation::Horizontal).build();

        let add_ipa_button = Button::builder(&top_panel).with_label("+").build();
        let device_picker = Choice::builder(&top_panel).build();
        let apple_id_button = Button::builder(&top_panel).with_label("Account").build();

        top_row.add(&add_ipa_button, 0, SizerFlag::All, 0);
        top_row.add_spacer(12);
        top_row.add(&device_picker, 1, SizerFlag::Expand | SizerFlag::All, 0);
        top_row.add_spacer(12);
        top_row.add(&apple_id_button, 0, SizerFlag::All, 0);

        top_panel.set_sizer(top_row, true);

        let default_page = create_default_page(&frame);
        let install_page = create_install_page(&frame);
        sizer.add(&top_panel, 0, SizerFlag::Expand | SizerFlag::All, 12);
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
            account_dialog: create_account_dialog(&frame),
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

            match run_login_flow(sender.clone(), email, password) {
                Ok(account) => {
                    sender.send(PlumeFrameMessage::AccountLogin(account)).ok();
                }
                Err(e) => {
                    sender.send(PlumeFrameMessage::Error(format!("Login error: {}", e))).ok();
                    sender.send(PlumeFrameMessage::AccountDeleted).ok();
                }
            }
        });
    }

    fn bind_widget_handlers(
        &mut self,
        sender: mpsc::UnboundedSender<PlumeFrameMessage>,
        message_handler: Rc<RefCell<PlumeFrameMessageHandler>>,
    ) {
        // --- Device Picker ---

        let handler_for_choice = message_handler.clone();
        let picker_clone = self.usbmuxd_picker.clone();
        self.usbmuxd_picker.on_selection_changed(move |_| {
            let mut handler = handler_for_choice.borrow_mut();
            handler.usbmuxd_selected_device_id = picker_clone
                .get_selection()
                .and_then(|i| handler.usbmuxd_device_list.get(i as usize))
                .map(|item| item.usbmuxd_device.device_id.to_string());
        });

        // --- Apple ID / Login Dialog ---

        let login_dialog_rc = Rc::new(self.login_dialog.clone());
        let account_dialog_rc = Rc::new(self.account_dialog.clone());
        let handler_for_account = message_handler.clone();
        self.apple_id_button.on_click({
            let login_dialog = login_dialog_rc.clone();
            let account_dialog = account_dialog_rc.clone();
            move |_| {
                if let Some(creds) = handler_for_account.borrow().account_credentials.as_ref() {
                    let (first, last) = creds.get_name();
                    account_dialog.set_account_name((first, last));
                    account_dialog.show_modal();
                } else {
                    login_dialog.show_modal();
                }
            }
        });
        
        self.account_dialog.set_logout_handler({
            let sender = sender.clone();
            move || {
                sender.send(PlumeFrameMessage::AccountDeleted).ok();
            }
        });

        // --- Login Dialog "Next" Button ---

        self.bind_login_dialog_next_handler(sender.clone(), login_dialog_rc);

        // --- File Drop/Open Handlers ---

        self.bind_file_handlers(sender.clone());

        // --- Install Page Handlers ---

        self.install_page.set_cancel_handler({
            let sender = sender.clone();
            move || {
                sender.send(PlumeFrameMessage::PackageDeselected).ok();
            }
        });

        
        
        
        
        
        
        
        
        
        
        let message_handler_for_install = message_handler.clone();
        self.install_page.set_install_handler({
            let frame = self.frame.clone();
            let sender = sender.clone();
            move || {
            let binding = message_handler_for_install.borrow();

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

            let package = selected_package.clone();
            let account = selected_account.clone();
            let device_id = selected_device.to_string();
            let sender_clone = sender.clone();

            thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();

                let install_result = rt.block_on(async {
                    let session = DeveloperSession::with(account.clone());
                    
                    sender_clone.send(PlumeFrameMessage::InstallProgress(10, Some("Ensuring current device is registered...".to_string()))).ok();

                    let mut usbmuxd = UsbmuxdConnection::default().await
                        .map_err(|e| format!("usbmuxd connect error: {e}"))?;
                    let usbmuxd_device = usbmuxd.get_devices().await
                        .map_err(|e| format!("usbmuxd device list error: {e}"))?
                        .into_iter()
                        .find(|d| d.device_id.to_string() == device_id)
                        .ok_or_else(|| format!("Device ID {device_id} not found"))?;

                    let device = Device::new(usbmuxd_device).await;
                    
                    // TODO: Handle multiple teams properly
                    let teams = session.qh_list_teams().await.map_err(|e| format!("Failed to list teams: {}", e))?;
                    
                    session.qh_ensure_device(
                        &teams.teams.get(0)
                            .ok_or("No teams available for the Apple ID account.")?
                            .team_id,
                        &device.name,
                        &device.uuid,
                    ).await.map_err(|e| format!("Failed to ensure device is registered: {}", e))?;
                    
                    let bundle = package.get_package_bundle()
                        .map_err(|e| format!("Failed to get package bundle: {}", e))?;
                    let bundles = bundle.collect_bundles_sorted()
                        .map_err(|e| format!("Failed to collect bundles: {}", e))?;
                    
                    let team_id = &teams.teams.get(0)
                        .ok_or("No teams available for the Apple ID account.")?
                        .team_id;
                    
                    let bundle_identifier = bundle.get_bundle_identifier()
                        .ok_or("Failed to get bundle identifier from package.")?;

                    let new_id = new_identifier(&bundle_identifier, team_id);

                    fn new_identifier(original: &str, team_id: &str) -> String {
                        format!("{}.{}", original, team_id)
                    }
                    
                    if let Some(old_identifier) = bundle.get_bundle_identifier() {
                        for embedded_bundle in &bundles {
                            embedded_bundle.set_matching_identifier(
                                &old_identifier,
                                &new_id,
                            ).map_err(|e| format!("Failed to set matching identifier: {}", e))?;
                        }
                    }

                    let mut provisionings: Vec<MobileProvision> = Vec::new();

                    for bundle in &bundles {
                        if 
                            bundle._type != BundleType::AppExtension &&
                            bundle._type != BundleType::App 
                           {
                            continue;
                        }

                        sender_clone.send(PlumeFrameMessage::InstallProgress(
                            20,
                            Some(format!("Registering {}...", bundle.get_name().unwrap_or_default()))
                        )).ok();

                        let bundle_executable_name = bundle.get_executable()
                            .ok_or("Failed to get executable from bundle.")?;
                        
                        let bundle_executable_path = bundle.dir().join(&bundle_executable_name);
                        
                        let macho = MachO::new(&bundle_executable_path)
                            .map_err(|e| format!("Failed to read Mach-O binary: {}", e))?;
                        
                        let macho_entitlements = macho.entitlements()
                            .map_err(|e| format!("Failed to get entitlements from Mach-O binary: {}", e))?;
                        
                        let id = bundle.get_bundle_identifier()
                            .ok_or("Failed to get bundle identifier from bundle.")?;
                                                
                        let app_groups: Vec<String> = macho_entitlements
                            .as_ref()
                            .and_then(|dict| dict.get("com.apple.security.application-groups"))
                            .and_then(|val| val.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_string().map(|s| format!("{}.{}", s, team_id)))
                                    .collect()
                            })
                            .unwrap_or_else(Vec::new);

                        session.qh_ensure_app_id(team_id, &bundle.get_name().unwrap_or_default(), &id)
                            .await
                            .map_err(|e| format!("Failed to ensure app ID: {}", e))?;
                        
                        let capabilities = session.v1_list_capabilities(team_id).await
                            .map_err(|e| format!("Failed to list capabilities: {}", e))?;
                                                
                        println!("Mach-O Entitlements: {:?}", &macho_entitlements);
                        
                        let mut capabilities_to_enable = Vec::new();
                        
                        if let Some(entitlements) = &macho_entitlements {
                            for (ent_key, _) in entitlements {
                                for cap in &capabilities.data {
                                    if let Some(ent_list) = &cap.attributes.entitlements {
                                        if ent_list.iter().any(|e| e.profile_key == *ent_key) {
                                            capabilities_to_enable.push(cap.id.clone());
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("Enabling capabilities: {:?}", &capabilities_to_enable);
                        
                        let app_id_id = session.qh_get_app_id(team_id, &id).await
                            .map_err(|e| e.to_string())?
                            .ok_or("Failed to get ensured app ID.")?;

                        if !capabilities_to_enable.is_empty() {
                            session.v1_update_app_id(team_id, &id, capabilities_to_enable)
                                .await
                                .map_err(|e| format!("Failed to enable capabilities: {}", e))?;
                        }
                        
                        for group in &app_groups {
                            let group_id = session.qh_ensure_app_group(team_id, group, group)
                                .await
                                .map_err(|e| format!("Failed to ensure app group: {}", e))?;

                            println!("{:#?}", group_id);

                            session.qh_assign_app_group(team_id, &app_id_id.app_id_id, &group_id.application_group)
                                .await
                                .map_err(|e| format!("Failed to add app group to app ID: {}", e))?;
                        }

                        let profiles = session.qh_get_profile(team_id, &app_id_id.app_id_id).await
                            .map_err(|e| format!("Failed to list profiles: {}", e))?;

                        let profile_data = profiles.provisioning_profile.encoded_profile;
                        
                        let mobile_provision = MobileProvision::load_from_bytes(profile_data.as_ref())
                            .map_err(|e| format!("Failed to load mobile provision: {}", e))?;
                        
                        provisionings.push(mobile_provision);
                    }
                    
                    sender_clone.send(PlumeFrameMessage::InstallProgress(30, Some("Downloading Certificates...".to_string()))).ok();
                    
                    Ok::<_, String>(())
                });

                if let Err(e) = install_result {
                    sender_clone.send(PlumeFrameMessage::InstallProgress(99, Some(format!("{}", e)))).ok();
                    return;
                }
            });
            }
        });
        
        
        
        
        
        
        
        
        
        
        
    }

    
    fn bind_login_dialog_next_handler(
        &self,
        sender: mpsc::UnboundedSender<PlumeFrameMessage>,
        login_dialog: Rc<LoginDialog>,
    ) {
        let frame_for_errors = self.frame.clone();
        login_dialog.clone().set_next_handler(move || {
            let email = login_dialog.get_email();
            let password = login_dialog.get_password();

            if email.trim().is_empty() || password.is_empty() {
                let dialog = MessageDialog::builder(
                    &frame_for_errors,
                    "Please enter both email and password.",
                    "Missing Information",
                )
                .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                .build();
                dialog.show_modal();
                return;
            }

            let creds = AccountCredentials;
            if let Err(e) = creds.set_credentials(email.clone(), password.clone()) {
                sender.send(PlumeFrameMessage::Error(format!("Failed to save credentials: {}", e))).ok();
                return;
            }

            login_dialog.clear_fields();
            login_dialog.hide();

            let sender_for_login_thread = sender.clone();
            thread::spawn(move || {
                match run_login_flow(sender_for_login_thread.clone(), email, password) {
                    Ok(account) => sender_for_login_thread.send(PlumeFrameMessage::AccountLogin(account)).ok(),
                    Err(e) => sender_for_login_thread.send(PlumeFrameMessage::Error(format!("Login failed: {}", e))).ok(),
                }
            });
        });
    }

    fn bind_file_handlers(&self, sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        #[cfg(not(target_os = "linux"))]
        self.default_page.set_file_handlers({
            let sender = sender.clone();
            move |file_path| Self::process_package_file(sender.clone(), PathBuf::from(file_path))
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
                    Self::process_package_file(sender.clone(), PathBuf::from(file_path));
                }
            }
        });
    }

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
}

pub fn run_login_flow(
    sender: mpsc::UnboundedSender<PlumeFrameMessage>,
    email: String,
    password: String,
) -> Result<Account, String> {
    let anisette_config = AnisetteConfiguration::default()
        .set_configuration_path(PathBuf::from(env::temp_dir()));

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
