use wxdragon::prelude::*;

use crate::frame::PlumeFrame;
use super::DIALOG_SIZE;

#[derive(Clone)]
pub struct LoginDialog {
    pub dialog: Dialog,
    pub email_field: TextCtrl,
    pub password_field: TextCtrl,
    pub next_button: Button,
}

pub fn create_login_dialog(parent: &Window) -> LoginDialog {
    let dialog = Dialog::builder(parent, "Sign in with your Apple ID")
        .with_style(DialogStyle::SystemMenu | DialogStyle::Caption)
        .with_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
        .build();

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add_spacer(13);

    let email_row = BoxSizer::builder(Orientation::Horizontal).build();
    let email_label = StaticText::builder(&dialog)
        .with_label("       Email:")
        .build();
    let email_field = TextCtrl::builder(&dialog).build();
    email_row.add(&email_label, 0, SizerFlag::AlignCenterVertical | SizerFlag::All, 4);
    email_row.add(&email_field, 1, SizerFlag::Expand | SizerFlag::Right, 8);
    sizer.add_sizer(&email_row, 0, SizerFlag::Expand | SizerFlag::All, 4);

    let password_row = BoxSizer::builder(Orientation::Horizontal).build();
    let password_label = StaticText::builder(&dialog).with_label("Password:").build();
    let password_field = TextCtrl::builder(&dialog)
        .with_style(TextCtrlStyle::Password)
        .build();
    password_row.add(&password_label, 0, SizerFlag::AlignCenterVertical | SizerFlag::All, 4);
    password_row.add(&password_field, 1, SizerFlag::Expand | SizerFlag::Right, 8);
    sizer.add_sizer(&password_row, 0, SizerFlag::Expand | SizerFlag::All, 4);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let cancel_button = Button::builder(&dialog).with_label("Cancel").build();
    let next_button = Button::builder(&dialog).with_label("Next").build();
    button_sizer.add(&cancel_button, 1, SizerFlag::Expand | SizerFlag::All, 0);
    button_sizer.add_spacer(13);
    button_sizer.add(&next_button, 1, SizerFlag::Expand | SizerFlag::All, 0);

    sizer.add_sizer(&button_sizer, 0, SizerFlag::AlignRight | SizerFlag::All, 13);

    dialog.set_sizer(sizer, true);

    cancel_button.on_click({
        let dialog = dialog.clone();
        move |_| dialog.hide()
    });

    LoginDialog {
        dialog,
        email_field,
        password_field,
        next_button,
    }
}

impl LoginDialog {
    pub fn get_email(&self) -> String {
        self.email_field.get_value().to_string()
    }

    pub fn get_password(&self) -> String {
        self.password_field.get_value().to_string()
    }

    pub fn clear_fields(&self) {
        self.email_field.set_value("");
        self.password_field.set_value("");
    }

    pub fn set_next_handler(&self, on_next: impl Fn() + 'static) {
        self.next_button.on_click(move |_evt| {
            on_next();
        });
    }
}

// MARK: - AccountDialog

#[derive(Clone)]
pub struct SettingsDialog {
    pub dialog: Dialog,
    pub account_list: CheckListBox,
    pub add_button: Button,
    pub remove_button: Button,
}

pub fn create_settings_dialog(parent: &Window) -> SettingsDialog {
    let dialog = Dialog::builder(parent, "Settings")
        .with_size(DIALOG_SIZE.0 + 50, DIALOG_SIZE.1 + 150)
        .build();

    let main_sizer = BoxSizer::builder(Orientation::Vertical).build();
    
    main_sizer.add_spacer(16);

    let accounts_label = StaticText::builder(&dialog)
        .with_label("Apple ID Accounts")
        .build();
    main_sizer.add(&accounts_label, 0, SizerFlag::Left | SizerFlag::Left | SizerFlag::Right, 16);
    
    main_sizer.add_spacer(8);

    let account_list = CheckListBox::builder(&dialog).build();
    main_sizer.add(&account_list, 1, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right, 16);

    main_sizer.add_spacer(12);

    let button_row = BoxSizer::builder(Orientation::Horizontal).build();
    let add_button = Button::builder(&dialog).with_label("Add Account").build();
    let remove_button = Button::builder(&dialog).with_label("Remove Account").build();
    
    button_row.add(&add_button, 0, SizerFlag::All, 0);
    button_row.add_spacer(8);
    button_row.add(&remove_button, 0, SizerFlag::All, 0);
    button_row.add_stretch_spacer(1);

    main_sizer.add_sizer(&button_row, 0, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right, 16);
    
    main_sizer.add_spacer(16);

    dialog.set_sizer(main_sizer, true);

    SettingsDialog {
        dialog,
        account_list,
        add_button,
        remove_button,
    }
}

impl SettingsDialog {
    pub fn set_add_handler(&self, on_add: impl Fn() + 'static) {
        self.add_button.on_click(move |_| {
            on_add();
        });
    }

    pub fn set_remove_handler(&self, on_remove: impl Fn() + 'static) {
        self.remove_button.on_click(move |_| {
            on_remove();
        });
    }
    
    pub fn set_checklistbox_handler(&self, on_select: impl Fn(usize) + 'static) {
        let checklistbox = self.account_list.clone();
        self.account_list.on_selected(move |event_data| {
            if let Some(selected_index) = event_data.get_selection() {
                let selected_index = selected_index as usize;
                
                let count = checklistbox.get_count() as usize;
                for i in 0..count {
                    checklistbox.check(i as u32, false);
                }
                
                checklistbox.check(selected_index as u32, true);
                on_select(selected_index);
            }
        });
    }
    
    pub fn refresh_account_list(&self, accounts: Vec<(String, String, bool)>) {
        self.account_list.clear();
        
        let has_accounts = !accounts.is_empty();
        
        for (i, (email, first_name, is_selected)) in accounts.into_iter().enumerate() {
            let label = format!("{} ({})", first_name, email);
            self.account_list.append(&label);
            
            if is_selected {
                self.account_list.check(i as u32, true);
            }
        }
        
        self.remove_button.enable(has_accounts);
    }
    
    pub fn get_checked_index(&self) -> Option<usize> {
        let count = self.account_list.get_count() as usize;
        for i in 0..count {
            if self.account_list.is_checked(i as u32) {
                return Some(i);
            }
        }
        None
    }
}

// MARK: - Single Field Dialog
impl PlumeFrame {
    pub fn create_single_field_dialog(&self, title: &str, label: &str) -> Result<String, String> {
        let dialog = Dialog::builder(&self.frame, title)
            .with_style(DialogStyle::SystemMenu | DialogStyle::Caption)
            .with_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
            .build();

        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        sizer.add_spacer(16);

        sizer.add(
            &StaticText::builder(&dialog).with_label(label).build(),
            0,
            SizerFlag::All,
            12,
        );
        let text_field = TextCtrl::builder(&dialog).build();
        sizer.add(&text_field, 0, SizerFlag::Expand | SizerFlag::All, 8);

        let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();

        let cancel_button = Button::builder(&dialog).with_label("Cancel").build();
        let ok_button = Button::builder(&dialog).with_label("OK").build();

        button_sizer.add(&cancel_button, 0, SizerFlag::All, 8);
        button_sizer.add_spacer(8);
        button_sizer.add(&ok_button, 0, SizerFlag::All, 8);

        sizer.add_sizer(&button_sizer, 0, SizerFlag::AlignRight | SizerFlag::All, 8);

        dialog.set_sizer(sizer, true);

        cancel_button.on_click({
            let dialog = dialog.clone();
            move |_| dialog.end_modal(ID_CANCEL as i32)
        });
        ok_button.on_click({
            let dialog = dialog.clone();
            move |_| dialog.end_modal(ID_OK as i32)
        });

        text_field.set_focus();

        let rc = dialog.show_modal();
        let result = if rc == ID_OK as i32 {
            Ok(text_field.get_value().to_string())
        } else {
            Err("2FA cancelled".to_string())
        };
        dialog.destroy();
        result
    }
}

// MARK: - Text Selection Dialog
impl PlumeFrame {
    pub fn create_text_selection_dialog(
        &self, title: &str,
        label: &str,
        choices: Vec<String>,
    ) -> Result<i32, String> {
        let choice_refs: Vec<&str> = choices.iter().map(|s| s.as_str()).collect();
        let dialog = SingleChoiceDialog::builder(&self.frame, label, title, &choice_refs).build();
        let rc = dialog.show_modal();
        if rc == ID_OK as i32 {
            Ok(dialog.get_selection())
        } else {
            Err("Dialog cancelled".to_string())
        }
    }
}
