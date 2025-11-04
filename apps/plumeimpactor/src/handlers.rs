use wxdragon::prelude::*;

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use std::sync::mpsc as std_mpsc;

use grand_slam::{auth::Account, developer::DeveloperSession};
use types::{Device, Package, PlistInfoTrait};

use crate::frame::PlumeFrame;

#[derive(Debug)]
pub enum PlumeFrameMessage {
    DeviceConnected(Device),
    DeviceDisconnected(u32),
    PackageSelected(Package),
    PackageDeselected,
    PackageInstallationStarted,
    AccountLogin(Account),
    AccountDeleted,
    AwaitingTwoFactorCode(std_mpsc::Sender<Result<String, String>>),
    Error(String),
}

pub struct PlumeFrameMessageHandler {
    pub receiver: mpsc::UnboundedReceiver<PlumeFrameMessage>,
    pub plume_frame: PlumeFrame,
    // --- device ---
    pub usbmuxd_device_list: Vec<Device>,
    pub usbmuxd_selected_device_id: Option<String>,
    // --- ipa ---
    pub package_selected: Option<Package>,
    // --- account ---
    pub account_credentials: Option<Account>,
}

impl PlumeFrameMessageHandler {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<PlumeFrameMessage>,
        plume_frame: PlumeFrame,
    ) -> Self {
        Self {
            receiver,
            plume_frame,
            usbmuxd_device_list: Vec::new(),
            usbmuxd_selected_device_id: None,
            package_selected: None,
            account_credentials: None,
        }
    }

    pub fn process_messages(&mut self) -> bool {
        let mut processed_count = 0;
        let mut has_more = false;

        for _ in 0..10 {
            match self.receiver.try_recv() {
                Ok(message) => {
                    processed_count += 1;
                    self.handle_message(message);
                }
                Err(TryRecvError::Empty) => return false,
                Err(TryRecvError::Disconnected) => return false,
            }
        }

        if processed_count == 10 {
            has_more = true;
        }

        has_more
    }

    fn handle_message(&mut self, message: PlumeFrameMessage) {
        match message {
            PlumeFrameMessage::DeviceConnected(device) => {
                println!("Device connected: {}", device);
                if !self
                    .usbmuxd_device_list
                    .iter()
                    .any(|d| d.usbmuxd_device.device_id == device.usbmuxd_device.device_id)
                {
                    self.usbmuxd_device_list.push(device.clone());
                    self.usbmuxd_picker_rebuild_contents();

                    if self.usbmuxd_device_list.len() == 1 {
                        self.usbmuxd_picker_select_item(&device.usbmuxd_device.device_id);
                    } else {
                        self.usbmuxd_picker_reconcile_selection();
                    }
                }
            }
            PlumeFrameMessage::DeviceDisconnected(device_id) => {
                println!("Device disconnected: {}", device_id);
                if let Some(index) = self
                    .usbmuxd_device_list
                    .iter()
                    .position(|d| d.usbmuxd_device.device_id == device_id)
                {
                    self.usbmuxd_device_list.remove(index);
                    self.usbmuxd_picker_rebuild_contents();
                    self.usbmuxd_picker_reconcile_selection();
                }
            }
            PlumeFrameMessage::PackageSelected(package) => {
                if self.package_selected.is_some() {
                    return;
                }

                let package_name = package.get_name().unwrap_or_else(|| "Unknown".to_string());
                let package_id = package
                    .get_bundle_identifier()
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("Package selected: {}", package_name);
                self.package_selected = Some(package);
                self.plume_frame
                    .install_page
                    .set_top_text(format!("{} - {}", package_name, package_id).as_str());
                self.plume_frame.default_page.panel.hide();
                self.plume_frame.install_page.panel.show(true);
                self.plume_frame.frame.layout();
            }
            PlumeFrameMessage::PackageDeselected => {
                println!("Package deselected");
                self.package_selected = None;
                self.plume_frame.install_page.panel.hide();
                self.plume_frame.default_page.panel.show(true);
                self.plume_frame.frame.layout();
            }
			PlumeFrameMessage::PackageInstallationStarted => {
                let package = match &self.package_selected {
                    Some(pkg) => pkg.clone(),
                    None => {
                        self.handle_message(PlumeFrameMessage::Error(
							"No package selected for installation.".to_string(),
						));
                        return;
                    }
                };

                let account = match &self.account_credentials {
                    Some(acc) => acc.clone(),
                    None => {
                        self.handle_message(PlumeFrameMessage::Error(
							"Installation failed: No account logged in.".to_string(),
						));
                        return;
                    }
                };
				//
            }
            PlumeFrameMessage::AccountLogin(account) => {
                self.account_credentials = Some(account);
                println!("Account logged in");
            }
            PlumeFrameMessage::AccountDeleted => {
                self.account_credentials = None;
                println!("Account deleted");
            }
            PlumeFrameMessage::AwaitingTwoFactorCode(tx) => {
                let result = self.plume_frame.create_single_field_dialog(
                    "Two-Factor Authentication",
                    "Enter the verification code sent to your device:",
                );

                if let Err(e) = tx.send(result) {
                    println!("Failed to send 2FA code back to background thread: {:?}", e);
                }
            }
            PlumeFrameMessage::Error(error_msg) => {
                println!("Error: {}", error_msg);
                let dialog = MessageDialog::builder(&self.plume_frame.frame, &error_msg, "Error")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                    .build();
                dialog.show_modal();
            }
        }
    }

    // --- Device Picker Helpers ---

    fn usbmuxd_picker_rebuild_contents(&self) {
        self.plume_frame.usbmuxd_picker.clear();
        for item_string in &self.usbmuxd_device_list {
            self.plume_frame
                .usbmuxd_picker
                .append(&item_string.to_string());
        }
    }

    fn usbmuxd_picker_select_item(&mut self, device_id: &u32) {
        if let Some(index) = self
            .usbmuxd_device_list
            .iter()
            .position(|d| d.usbmuxd_device.device_id == *device_id)
        {
            self.plume_frame.usbmuxd_picker.set_selection(index as u32);
            self.usbmuxd_selected_device_id = Some(device_id.to_string());
        } else {
            self.usbmuxd_selected_device_id = None;
        }
    }

    fn usbmuxd_picker_reconcile_selection(&mut self) {
        if let Some(selected_item) = self.usbmuxd_selected_device_id.clone() {
            if let Some(new_index) = self
                .usbmuxd_device_list
                .iter()
                .position(|d| d.usbmuxd_device.device_id.to_string() == selected_item)
            {
                self.plume_frame
                    .usbmuxd_picker
                    .set_selection(new_index as u32);
            } else {
                self.usbmuxd_picker_default_selection();
            }
        } else {
            self.usbmuxd_picker_default_selection();
        }
    }

    fn usbmuxd_picker_default_selection(&mut self) {
        if !self.usbmuxd_device_list.is_empty() {
            self.plume_frame.usbmuxd_picker.set_selection(0);
        } else {
            self.usbmuxd_selected_device_id = None;
        }
    }
}
