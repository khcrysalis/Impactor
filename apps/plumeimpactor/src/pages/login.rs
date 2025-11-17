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

    pub fn show_modal(&self) {
        self.dialog.show_modal();
    }

    pub fn hide(&self) {
        self.dialog.end_modal(0);
    }

    pub fn set_next_handler(&self, on_next: impl Fn() + 'static) {
        self.next_button.on_click(move |_evt| {
            on_next();
        });
    }
}

// MARK: - AccountDialog

#[derive(Clone)]
pub struct AccountDialog {
    pub dialog: Dialog,
    pub logout_button: Button,
    pub label: StaticText,
}

pub fn create_account_dialog(parent: &Window) -> AccountDialog {
    let dialog = Dialog::builder(parent, "Account")
        .with_style(DialogStyle::SystemMenu | DialogStyle::Caption)
        .with_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
        .build();

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add_spacer(13);

    let label = StaticText::builder(&dialog).with_label("").build();
    sizer.add(&label, 0, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right, 13);

    let buttons = BoxSizer::builder(Orientation::Horizontal).build();
    
    let close_button = Button::builder(&dialog).with_label("Close").build();
    buttons.add(&close_button, 0, SizerFlag::All, 8);
    let logout_button = Button::builder(&dialog).with_label("Log out").build();
    
    buttons.add(&logout_button, 0, SizerFlag::All, 8);
    
    sizer.add_sizer(&buttons, 0, SizerFlag::AlignRight | SizerFlag::All, 8);

    dialog.set_sizer(sizer, true);
    
    close_button.on_click({
        let dialog = dialog.clone();
        move |_| dialog.end_modal(ID_OK as i32)
    });

    AccountDialog {
        dialog,
        logout_button,
        label,
    }
}

impl AccountDialog {
    pub fn show_modal(&self) {
        self.dialog.show_modal();
    }

    pub fn set_logout_handler(&self, on_logout: impl Fn() + 'static) {
        let dialog = self.dialog.clone();
        self.logout_button.on_click(move |_| {
            on_logout();
            dialog.end_modal(ID_OK as i32);
        });
    }
    
    pub fn set_account_name(&self, account_name: (String, String)) {
        self.label.set_label(&format!("Logged in as {} {}", account_name.0, account_name.1));
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
