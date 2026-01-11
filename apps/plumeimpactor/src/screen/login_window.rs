use iced::futures::SinkExt;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Fill, Task, window};
use plume_core::{AnisetteConfiguration, auth::Account};
use plume_store::{AccountStore, GsaAccount};
use std::sync::mpsc as std_mpsc;

#[derive(Debug, Clone)]
pub enum Message {
    EmailChanged(String),
    PasswordChanged(String),
    LoginSubmit,
    LoginCancel,
    LoginSuccess(GsaAccount),
    LoginFailed(String),
    TwoFactorCodeChanged(String),
    TwoFactorSubmit,
    TwoFactorCancel,
    RequestTwoFactor,
}

pub struct LoginWindow {
    pub window_id: Option<window::Id>,
    email: String,
    password: String,
    two_factor_code: String,
    two_factor_error: Option<String>,
    is_logging_in: bool,
    show_two_factor: bool,
    two_factor_tx: Option<std_mpsc::Sender<Result<String, String>>>,
}

impl LoginWindow {
    pub fn new() -> (Self, Task<Message>) {
        let (id, task) = window::open(window::Settings {
            size: iced::Size::new(400.0, 300.0),
            position: window::Position::Centered,
            resizable: false,
            decorations: true,
            ..Default::default()
        });

        (
            Self {
                window_id: Some(id),
                email: String::new(),
                password: String::new(),
                two_factor_code: String::new(),
                two_factor_error: None,
                is_logging_in: false,
                show_two_factor: false,
                two_factor_tx: None,
            },
            task.discard(),
        )
    }

    pub fn window_id(&self) -> Option<window::Id> {
        self.window_id
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EmailChanged(email) => {
                self.email = email;
                Task::none()
            }
            Message::PasswordChanged(password) => {
                self.password = password;
                Task::none()
            }
            Message::LoginSubmit => {
                if self.email.trim().is_empty() || self.password.is_empty() {
                    return Task::done(Message::LoginFailed(
                        "Email and password required".to_string(),
                    ));
                }

                self.is_logging_in = true;
                let email = self.email.trim().to_string();
                let password = self.password.clone();
                self.password.clear();

                let (tx, rx) = std_mpsc::channel::<Result<String, String>>();
                self.two_factor_tx = Some(tx);

                // Run the login stream
                Task::run(Self::perform_login(email, password, rx), |msg| msg)
            }
            Message::RequestTwoFactor => {
                self.show_two_factor = true;
                self.is_logging_in = false;
                Task::none()
            }
            Message::LoginCancel => {
                if let Some(id) = self.window_id {
                    self.two_factor_tx = None;
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            Message::LoginSuccess(account) => {
                let path = crate::defaults::get_data_path().join("accounts.json");
                // Synchronous save for simplicity
                if let Ok(mut store) = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { AccountStore::load(&Some(path.clone())).await })
                {
                    let _ = store.accounts_add_sync(account);
                }

                if let Some(id) = self.window_id {
                    self.two_factor_tx = None;
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            Message::LoginFailed(error) => {
                self.is_logging_in = false;
                self.show_two_factor = false;
                self.two_factor_code.clear();
                self.two_factor_tx = None;

                std::thread::spawn(move || {
                    rfd::MessageDialog::new()
                        .set_title("Login Failed")
                        .set_description(&error)
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();
                });
                Task::none()
            }
            Message::TwoFactorCodeChanged(code) => {
                self.two_factor_code = code;
                Task::none()
            }
            Message::TwoFactorSubmit => {
                let code = self.two_factor_code.trim().to_string();
                if code.is_empty() {
                    self.two_factor_error = Some("Code required".to_string());
                    return Task::none();
                }

                if let Some(tx) = &self.two_factor_tx {
                    let _ = tx.send(Ok(code));
                }
                Task::none()
            }
            Message::TwoFactorCancel => {
                if let Some(tx) = self.two_factor_tx.take() {
                    let _ = tx.send(Err("Cancelled".to_string()));
                }
                if let Some(id) = self.window_id {
                    window::close(id)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.show_two_factor {
            self.view_two_factor()
        } else {
            self.view_login()
        }
    }

    fn view_login(&self) -> Element<'_, Message> {
        let email_input = text_input("Email", &self.email)
            .on_input(Message::EmailChanged)
            .padding(8)
            .width(Fill);

        let password_input = text_input("Password", &self.password)
            .on_input(Message::PasswordChanged)
            .secure(true)
            .padding(8)
            .width(Fill);

        let mut content = column![
            text("Your Apple ID is used to sign and install apps. Credentials sent only to Apple.")
                .size(14),
            text("Email:").size(14),
            email_input,
            text("Password:").size(14),
            password_input,
        ]
        .spacing(10)
        .align_x(Alignment::Start);

        let buttons = row![
            container(text("")).width(Fill),
            button("Cancel").on_press(Message::LoginCancel).padding(8),
            button(if self.is_logging_in {
                "Connecting..."
            } else {
                "Next"
            })
            .on_press_maybe(if self.is_logging_in {
                None
            } else {
                Some(Message::LoginSubmit)
            })
            .padding(8)
            .style(button::primary),
        ]
        .spacing(10);

        content = content.push(container(text("")).width(Fill));
        content = content.push(buttons);

        container(content).padding(10).into()
    }

    fn view_two_factor(&self) -> Element<'_, Message> {
        let code_input = text_input("Verification Code", &self.two_factor_code)
            .on_input(Message::TwoFactorCodeChanged)
            .padding(8)
            .width(Fill);

        let mut content = column![
            text("Two-Factor Authentication").size(20),
            text("Enter the verification code sent to your device:").size(14),
            code_input,
        ]
        .spacing(10)
        .padding(20)
        .align_x(Alignment::Start);

        if let Some(error) = &self.two_factor_error {
            content = content.push(text(error).style(|_theme| text::Style {
                color: Some(iced::Color::from_rgb(1.0, 0.3, 0.3)),
            }));
        }

        let buttons = row![
            button("Cancel")
                .on_press(Message::TwoFactorCancel)
                .padding(8),
            button("Verify")
                .on_press(Message::TwoFactorSubmit)
                .padding(8)
                .style(button::primary),
        ]
        .spacing(10);

        content = content.push(buttons);
        container(content).padding(20).into()
    }

    fn perform_login(
        email: String,
        password: String,
        two_factor_rx: std_mpsc::Receiver<Result<String, String>>,
    ) -> impl iced::futures::Stream<Item = Message> {
        iced::stream::channel(
            10,
            move |mut output: futures::channel::mpsc::Sender<Message>| async move {
                let (bridge_tx, mut bridge_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
                let email_clone = email.clone();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    let anisette_config = AnisetteConfiguration::default()
                        .set_configuration_path(crate::defaults::get_data_path());

                    let account_result = rt.block_on(Account::login(
                        || Ok((email_clone.clone(), password.clone())),
                        || {
                            let _ = bridge_tx.send(Message::RequestTwoFactor);

                            match two_factor_rx.recv() {
                                Ok(result) => result,
                                Err(_) => Err("Two-factor authentication cancelled".to_string()),
                            }
                        },
                        anisette_config,
                    ));

                    let final_msg = match account_result {
                        Ok(account) => {
                            match rt.block_on(plume_store::account_from_session(
                                email_clone.clone(),
                                account,
                            )) {
                                Ok(gsa) => Message::LoginSuccess(gsa),
                                Err(e) => Message::LoginFailed(e.to_string()),
                            }
                        }
                        Err(e) => Message::LoginFailed(e.to_string()),
                    };
                    let _ = bridge_tx.send(final_msg);
                });

                while let Some(msg) = bridge_rx.recv().await {
                    let _ = output.send(msg).await;
                }
            },
        )
    }
}
