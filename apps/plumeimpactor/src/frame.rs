use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::{env, ptr, thread};

use grand_slam::AnisetteConfiguration;
use grand_slam::auth::Account;
use wxdragon::prelude::*;

use futures::StreamExt;
use idevice::usbmuxd::{UsbmuxdConnection, UsbmuxdListenEvent};
use tokio::runtime::Builder;
use tokio::sync::mpsc;

use crate::APP_NAME;
use crate::handlers::{PlumeFrameMessage, PlumeFrameMessageHandler};
use crate::keychain::AccountCredentials;
use crate::pages::login::LoginDialog;
use crate::pages::{
    DefaultPage, InstallPage, create_default_page, create_install_page, create_login_dialog,
};
use types::{Device, Package};

pub struct PlumeFrame {
    pub frame: Frame,
    pub default_page: DefaultPage,
    pub install_page: InstallPage,
    pub usbmuxd_picker: Choice,

    pub apple_id_button: Button,
    pub login_dialog: LoginDialog,
}

impl PlumeFrame {
    pub fn new() -> Self {
        let frame = Frame::builder()
            .with_title(APP_NAME)
            .with_size(Size::new(530, 410))
            .with_style(FrameStyle::CloseBox | FrameStyle::MinimizeBox)
            .build();

        let sizer = BoxSizer::builder(Orientation::Vertical).build();

        let top_panel = Panel::builder(&frame).build();
        let top_row = BoxSizer::builder(Orientation::Horizontal).build();

        let device_picker = Choice::builder(&top_panel).build();
        let apple_id_button = Button::builder(&top_panel).with_label("Account").build();

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
            apple_id_button,
            login_dialog: create_login_dialog(&frame),
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
                        sender
                            .send(PlumeFrameMessage::Error(format!(
                                "Failed to connect to usbmuxd: {}",
                                e
                            )))
                            .ok();
                        return;
                    }
                };

                match muxer.get_devices().await {
                    Ok(devices) => {
                        for dev in devices {
                            sender
                                .send(PlumeFrameMessage::DeviceConnected(Device::new(dev).await))
                                .ok();
                        }
                    }
                    Err(e) => {
                        sender
                            .send(PlumeFrameMessage::Error(format!(
                                "Failed to get initial device list: {}",
                                e
                            )))
                            .ok();
                    }
                }

                let mut stream = match muxer.listen().await {
                    Ok(stream) => stream,
                    Err(e) => {
                        sender
                            .send(PlumeFrameMessage::Error(format!(
                                "Failed to listen for events: {}",
                                e
                            )))
                            .ok();
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

    /// Spawns the automatic account login thread.
    fn spawn_auto_login_thread(sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        thread::spawn(move || {
            let creds = AccountCredentials;

            let (email, password) = match (creds.get_email(), creds.get_password()) {
                (Ok(email), Ok(password)) => (email, password),
                _ => {
                    return;
                }
            };

            match run_login_flow(sender.clone(), email, password) {
                Ok(account) => {
                    sender.send(PlumeFrameMessage::AccountLogin(account)).ok();
                }
                Err(e) => {
                    sender
                        .send(PlumeFrameMessage::Error(format!("Login error: {}", e)))
                        .ok();
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
        self.usbmuxd_picker
            .on_selection_changed(move |_event_data| {
                let mut handler = handler_for_choice.borrow_mut();

                if let Some(index) = picker_clone.get_selection() {
                    if let Some(selected_item) = handler.usbmuxd_device_list.get(index as usize) {
                        handler.usbmuxd_selected_device_id =
                            Some(selected_item.usbmuxd_device.device_id.to_string());
                    }
                } else {
                    handler.usbmuxd_selected_device_id = None;
                }
            });

        // --- Apple ID / Login Dialog ---
		
        let login_dialog_rc = Rc::new(self.login_dialog.clone());
        self.apple_id_button.on_click({
            let login_dialog = login_dialog_rc.clone();
            move |_| {
                login_dialog.show_modal();
            }
        });

        self.login_dialog.set_cancel_handler({
            let login_dialog = login_dialog_rc.clone();
            move || {
                login_dialog.clear_fields();
                login_dialog.hide();
            }
        });

        // --- Login Dialog "Next" Button ---
		
        self.bind_login_dialog_next_handler(sender.clone(), login_dialog_rc);

        // --- File Drop/Open Handlers ---
		
        self.bind_file_handlers(sender.clone());

        // --- Install Page Handlers ---
		
        let sender_for_cancel = sender.clone();
        self.install_page.set_cancel_handler(move || {
            sender_for_cancel
                .send(PlumeFrameMessage::PackageDeselected)
                .ok();
        });

        let sender_for_install = sender.clone();
        self.install_page.set_install_handler(move || {
            sender_for_install
                .send(PlumeFrameMessage::PackageInstallationStarted)
                .ok();
        });
    }

    fn bind_login_dialog_next_handler(
        &self,
        sender: mpsc::UnboundedSender<PlumeFrameMessage>,
        login_dialog: Rc<LoginDialog>,
    ) {
        login_dialog.clone().set_next_handler(move || {
            let email = login_dialog.get_email();
            let password = login_dialog.get_password();
            let creds = AccountCredentials;

            match creds.credentials_exist(email.clone(), password.clone()) {
                Ok(true) => {
                    creds.delete_password().ok();
                    sender
                        .send(PlumeFrameMessage::Error(
                            "Account already exists.".to_string(),
                        ))
                        .ok();
                }
                Ok(false) => {
                    let sender_for_login_thread = sender.clone();

                    thread::spawn(move || {
                        match run_login_flow(
                            sender_for_login_thread.clone(),
                            email.clone(),
                            password.clone(),
                        ) {
                            Ok(account) => {
                                if let Err(e) =
                                    creds.set_credentials(email.clone(), password.clone())
                                {
                                    sender_for_login_thread
                                        .send(PlumeFrameMessage::Error(format!(
                                            "Keychain error: {}",
                                            e
                                        )))
                                        .ok();
                                }

                                sender_for_login_thread
                                    .send(PlumeFrameMessage::AccountLogin(account))
                                    .ok();
                            }
                            Err(e) => {
                                sender_for_login_thread
                                    .send(PlumeFrameMessage::Error(format!("Login error: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
                Err(e) => {
                    sender
                        .send(PlumeFrameMessage::Error(format!("Keychain error: {}", e)))
                        .ok();
                }
            }

            login_dialog.clear_fields();
            login_dialog.hide();
        });
    }

    fn bind_file_handlers(&self, sender: mpsc::UnboundedSender<PlumeFrameMessage>) {
        let handler_for_import = self.frame.clone();

        self.default_page.set_file_handlers(
            {
                let sender = sender.clone();
                move |file_path| Self::process_package_file(sender.clone(), PathBuf::from(file_path))
            },
            move || {
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
            },
        );
    }

    /// This de-duplicates logic from the file handlers.
    fn process_package_file(
        sender: mpsc::UnboundedSender<PlumeFrameMessage>,
        file_path: PathBuf,
    ) {
        match Package::new(file_path) {
            Ok(package) => {
                sender
                    .send(PlumeFrameMessage::PackageSelected(package))
                    .ok();
            }
            Err(e) => {
                sender
                    .send(PlumeFrameMessage::Error(format!(
                        "Failed to open package: {}",
                        e
                    )))
                    .ok();
            }
        }
    }

    pub fn create_single_field_dialog(&self, title: &str, label: &str) -> Result<String, String> {
        let dialog = Dialog::builder(&self.frame, title)
            .with_style(DialogStyle::DefaultDialogStyle)
            .build();
    
        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        sizer.add_spacer(16);
    
        let field_label = StaticText::builder(&dialog).with_label(label).build();
        let text_field = TextCtrl::builder(&dialog).build();
        sizer.add(&field_label, 0, SizerFlag::All, 12);
        sizer.add(&text_field, 0, SizerFlag::Expand | SizerFlag::All, 8);
    
        dialog.set_sizer(sizer, true);
    
        dialog.show_modal();
        let value = text_field.get_value().to_string();
        dialog.destroy();
        println!("2FA code entered: {}", value);
        Ok(value)
    }
}

fn run_login_flow(
    sender: mpsc::UnboundedSender<PlumeFrameMessage>,
    email: String,
    password: String,
) -> Result<Account, String> {
    let anisette_config =
        AnisetteConfiguration::default().set_configuration_path(env::temp_dir());

    let rt = match Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => return Err(format!("Failed to create Tokio runtime: {}", e)),
    };

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
