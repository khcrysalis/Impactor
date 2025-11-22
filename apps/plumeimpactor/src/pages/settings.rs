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
        move |_| dialog.end_modal(ID_CANCEL as i32)
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
    pub logout_button: Button,
    pub account_label: StaticText,
}

pub fn create_settings_dialog(parent: &Window) -> SettingsDialog {
    let dialog = Dialog::builder(parent, "Settings")
        .with_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
        .build();

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add_spacer(13);

    let account_row = BoxSizer::builder(Orientation::Horizontal).build();
    let account_label = StaticText::builder(&dialog).with_label("Not logged in").build();
    let logout_button = Button::builder(&dialog).with_label("Login").build();
    account_row.add(&account_label, 4, SizerFlag::Expand, 0);
    account_row.add_stretch_spacer(1);
    account_row.add(&logout_button, 1, SizerFlag::Expand, 0);

    sizer.add_sizer(&account_row, 0, SizerFlag::Right | SizerFlag::Left, 13);

    sizer.add(&StaticLine::builder(&dialog).build(), 0, SizerFlag::Expand | SizerFlag::All, 13);

    dialog.set_sizer(sizer, true);

    SettingsDialog {
        dialog,
        logout_button,
        account_label,
    }
}

impl SettingsDialog {
    pub fn set_logout_handler(&self, on_logout: impl Fn() + 'static) {
        self.logout_button.on_click(move |_| {
            on_logout();
        });
    }

    pub fn set_account_name(&self, account_name: Option<(String, String)>) {
        match account_name {
            Some((first, last)) => {
                self.account_label.set_label(&format!("Logged in as {} {}", first, last));
                self.logout_button.set_label("Logout");
            }
            None => {
                self.account_label.set_label("Not logged in");
                self.logout_button.set_label("Sign In");
            }
        }
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
