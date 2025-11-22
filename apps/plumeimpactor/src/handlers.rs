use wxdragon::prelude::*;
use tokio::sync::{
    mpsc, 
    mpsc::error::TryRecvError
};
use std::sync::mpsc as std_mpsc;
use grand_slam::auth::Account;
use utils::{
    SignerOptions, 
    Package, 
    PlistInfoTrait,
    Device
};
use crate::frame::PlumeFrame;
use crate::keychain::AccountCredentials;

#[derive(Debug)]
pub enum PlumeFrameMessage {
    DeviceConnected(Device),
    DeviceDisconnected(u32),
    PackageSelected(Package),
    PackageDeselected,
    AccountLogin(Account),
    AccountDeleted,
    AwaitingTwoFactorCode(std_mpsc::Sender<Result<String, String>>),
    RequestTeamSelection(Vec<String>, std_mpsc::Sender<Result<i32, String>>),
    InstallProgress(i32, Option<String>),
    Error(String),
}

pub struct PlumeFrameMessageHandler {
    pub receiver: mpsc::UnboundedReceiver<PlumeFrameMessage>,
    pub plume_frame: PlumeFrame,
    pub installation_progress_dialog: Option<ProgressDialog>,
    // --- device ---
    pub usbmuxd_device_list: Vec<Device>,
    pub usbmuxd_selected_device_id: Option<String>,
    // --- ipa ---
    pub package_selected: Option<Package>,
    // --- account ---
    pub account_credentials: Option<Account>,
    // --- signer settings ---
    pub signer_settings: SignerOptions,
}

impl PlumeFrameMessageHandler {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<PlumeFrameMessage>,
        plume_frame: PlumeFrame,
    ) -> Self {
        let signer_settings = SignerOptions::default();
        Self {
            receiver,
            plume_frame,
            usbmuxd_device_list: Vec::new(),
            usbmuxd_selected_device_id: None,
            package_selected: None,
            account_credentials: None,
            installation_progress_dialog: None,
            signer_settings,
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
                
                self.plume_frame.install_page.install_button.enable(true);
            }
            PlumeFrameMessage::DeviceDisconnected(device_id) => {
                if let Some(index) = self
                    .usbmuxd_device_list
                    .iter()
                    .position(|d| d.usbmuxd_device.device_id == device_id)
                {
                    self.usbmuxd_device_list.remove(index);
                    self.usbmuxd_picker_rebuild_contents();
                    self.usbmuxd_picker_reconcile_selection();
                }
                
                if self.usbmuxd_device_list.is_empty() {
                    self.plume_frame.install_page.install_button.enable(false);
                }
            }
            PlumeFrameMessage::PackageSelected(package) => {
                if self.package_selected.is_some() {
                    return;
                }

                package.load_into_signer_options(&mut self.signer_settings);

                self.package_selected = Some(package);
                self.plume_frame.install_page.set_settings(&self.signer_settings, Some(self.package_selected.as_ref().unwrap()));
                self.plume_frame.default_page.panel.hide();
                self.plume_frame.install_page.panel.show(true);
                self.plume_frame.frame.layout();
                
                self.plume_frame.add_ipa_button.enable(false);
            }
            PlumeFrameMessage::PackageDeselected => {
                // TODO: should it be this way?
                if let Some(package) = self.package_selected.as_ref() {
                    package.clone().remove_package_stage();
                }

                self.package_selected = None;
                self.plume_frame.install_page.panel.hide();
                self.plume_frame.default_page.panel.show(true);
                self.plume_frame.frame.layout();
                self.signer_settings = SignerOptions::default();
                self.plume_frame.install_page.set_settings(&self.signer_settings, None);
                self.plume_frame.add_ipa_button.enable(true);
            }
            PlumeFrameMessage::AccountLogin(account) => {
                let (first, last) = account.get_name();
                let dialog = MessageDialog::builder(
                    &self.plume_frame.frame, 
                    &format!("Logged in as {} {}", first, last), 
                    "Signed In"
                )
                .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
                .build();
                dialog.show_modal();
                self.account_credentials = Some(account);
                
                self.plume_frame.login_dialog.dialog.hide();
                self.plume_frame.settings_dialog.set_account_name(Some((first, last)));
            }
            PlumeFrameMessage::AccountDeleted => {
                if self.account_credentials.is_none() {
                    return;
                }
                
                let creds = AccountCredentials;
                if let Err(e) = creds.delete_password() {
                    self.handle_message(PlumeFrameMessage::Error(format!("Failed to delete account credentials: {}", e)));
                    return;
                }
                
                self.account_credentials = None;
                self.plume_frame.settings_dialog.set_account_name(None);
            }
            PlumeFrameMessage::AwaitingTwoFactorCode(tx) => {
                let result = self.plume_frame.create_single_field_dialog(
                    "Two-Factor Authentication",
                    "Enter the verification code sent to your device:",
                );

                if let Err(e) = tx.send(result) {
                    self.handle_message(PlumeFrameMessage::Error(format!("Failed to send two-factor code response: {}", e)));
                }
            }
            PlumeFrameMessage::RequestTeamSelection(teams, tx) => {
                let result = self.plume_frame.create_text_selection_dialog(
                    "Select a Team",
                    "Please select a team from the list:",
                    teams,
                );

                if let Err(e) = tx.send(result) {
                    self.handle_message(PlumeFrameMessage::Error(format!("Failed to send team selection response: {}", e)));
                }
            }
            PlumeFrameMessage::InstallProgress(progress, message_opt) => {
                println!("Progress: {} - {:?}", progress, message_opt);
                let Some(selected_package) = &self.package_selected else {
                    return;
                };
                
                if self.installation_progress_dialog.is_none() {
                    let progress_dialog = ProgressDialog::builder(
                        &self.plume_frame.frame,
                        &format!("Installing {}", selected_package.get_name().unwrap_or("Unknown".to_string())),
                        "Waiting...",
                        100
                    )
                    .show_elapsed_time()
                    .show_estimated_time()
                    .show_remaining_time()
                    .can_abort()
                    .smooth()
                    .build();
                
                    self.installation_progress_dialog = Some(progress_dialog);
                }

                if let Some(dialog) = &mut self.installation_progress_dialog {
                    dialog.update(progress, message_opt.as_deref());

                    if progress >= 90 {
                        self.installation_progress_dialog = None;
                    }
                }
            }
            PlumeFrameMessage::Error(error_msg) => {
                let dialog = MessageDialog::builder(&self.plume_frame.frame, &error_msg, "Error")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                    .build();
                dialog.show_modal();
            }
        }
    }
}

// USBMUXD HANDLERS

impl PlumeFrameMessageHandler {
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
