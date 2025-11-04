use wxdragon::prelude::*;

#[derive(Clone)]
pub struct LoginDialog {
    pub dialog: Dialog,
    pub email_field: TextCtrl,
    pub password_field: TextCtrl,
    pub cancel_button: Button,
    pub next_button: Button,
}

pub fn create_login_dialog(parent: &Window) -> LoginDialog {
    let dialog = Dialog::builder(parent, "Sign in with your Apple ID")
        .with_style(DialogStyle::DefaultDialogStyle)
        .build();

    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    sizer.add_spacer(12);

    let description = StaticText::builder(&dialog)
		.with_label("In order to use the main features for PlumeImpactor, you will\nneed to sign into your Apple ID. But don't worry! Your\ncredentials are sent to Apple and not anywhere else!")
		.build();
	sizer.add(&description, 0, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right, 12);

	let email_row = BoxSizer::builder(Orientation::Horizontal).build();
	let email_label = StaticText::builder(&dialog)
		.with_label("       Email:")
		.build();
	let email_field = TextCtrl::builder(&dialog).build();
	email_row.add(&email_label, 0, SizerFlag::AlignCenterVertical | SizerFlag::All, 8);
	email_row.add(&email_field, 1, SizerFlag::Expand | SizerFlag::All, 12);
	sizer.add_sizer(&email_row, 0, SizerFlag::Expand | SizerFlag::All, 0);

	let password_row = BoxSizer::builder(Orientation::Horizontal).build();
	let password_label = StaticText::builder(&dialog).with_label("Password:").build();
	let password_field = TextCtrl::builder(&dialog)
		.with_style(TextCtrlStyle::Password)
		.build();
	password_row.add(&password_label, 0, SizerFlag::AlignCenterVertical | SizerFlag::All, 8);
	password_row.add(&password_field, 1, SizerFlag::Expand | SizerFlag::All, 12);
	sizer.add_sizer(&password_row, 0, SizerFlag::Expand | SizerFlag::All, 0);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let cancel_button = Button::builder(&dialog).with_label("Cancel").build();
    let next_button = Button::builder(&dialog).with_label("Next").build();
    button_sizer.add(&cancel_button, 1, SizerFlag::Expand | SizerFlag::All, 0);
    button_sizer.add_spacer(12);
    button_sizer.add(&next_button, 1, SizerFlag::Expand | SizerFlag::All, 0);

    sizer.add_sizer(&button_sizer, 0, SizerFlag::AlignRight | SizerFlag::All, 12);

    dialog.set_sizer(sizer, true);

    LoginDialog {
        dialog,
        email_field,
        password_field,
        cancel_button,
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

    pub fn set_cancel_handler(&self, on_cancel: impl Fn() + 'static) {
        self.cancel_button.on_click(move |_evt| {
            on_cancel();
        });
    }

    pub fn set_next_handler(&self, on_next: impl Fn() + 'static) {
        self.next_button.on_click(move |_evt| {
            on_next();
        });
    }
}
