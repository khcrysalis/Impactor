use std::{sync::mpsc as std_mpsc, thread};

use eframe::egui;
use plume_core::{AnisetteConfiguration, auth::Account, store::account_from_session};
use plume_shared::get_data_path;
use tokio::{runtime::Builder, sync::mpsc};

use crate::app::AppMessage;

// -----------------------------------------------------------------------------
// State
// -----------------------------------------------------------------------------

#[derive(Default)]
pub(crate) struct LoginUi {
    pub(crate) open: bool,
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) error: Option<String>,
    pub(crate) is_logging_in: bool,
    pub(crate) show_two_factor: bool,
    pub(crate) two_factor_code: String,
    pub(crate) two_factor_error: Option<String>,
    pub(crate) two_factor_tx: Option<std_mpsc::Sender<Result<String, String>>>,
}

impl LoginUi {
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.error = None;
        self.two_factor_error = None;
    }

    pub(crate) fn request_two_factor(&mut self, tx: std_mpsc::Sender<Result<String, String>>) {
        self.show_two_factor = true;
        self.two_factor_code.clear();
        self.two_factor_error = None;
        self.two_factor_tx = Some(tx);
    }

    pub(crate) fn fail(&mut self, error: String) {
        self.is_logging_in = false;
        self.error = Some(error);
        self.show_two_factor = false;
        self.two_factor_tx = None;
        self.two_factor_code.clear();
    }

    pub(crate) fn success(&mut self) {
        self.is_logging_in = false;
        self.open = false;
        self.error = None;
        self.password.clear();
        self.show_two_factor = false;
        self.two_factor_tx = None;
        self.two_factor_code.clear();
    }
}

// -----------------------------------------------------------------------------
// UI
// -----------------------------------------------------------------------------

pub(crate) fn ui_login(
    ctx: &egui::Context,
    state: &mut LoginUi,
    sender: Option<&mpsc::UnboundedSender<AppMessage>>,
) {
    if state.open {
        let mut open = state.open;
        let mut requested_close = false;
        egui::Window::new("Sign in with your Apple ID")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                let fields_enabled = !state.is_logging_in;

                ui.add_enabled_ui(fields_enabled, |ui| {
                    ui.label("Email");
                    ui.text_edit_singleline(&mut state.email);

                    ui.label("Password");
                    ui.add(egui::TextEdit::singleline(&mut state.password).password(true));
                });

                if let Some(error) = state.error.as_ref() {
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::RED, error);
                }

                if state.is_logging_in {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label("Logging in...");
                    });
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let cancel = ui.add_enabled(!state.is_logging_in, egui::Button::new("Cancel"));
                    if cancel.clicked() {
                        requested_close = true;
                    }

                    let next = ui.add_enabled(!state.is_logging_in, egui::Button::new("Next"));
                    if next.clicked() {
                        let email = state.email.trim().to_string();
                        let password = state.password.clone();

                        if email.is_empty() || password.is_empty() {
                            state.error = Some("Please enter both email and password.".to_string());
                            return;
                        }

                        let Some(sender) = sender else {
                            state.error =
                                Some("Login unavailable: missing app sender.".to_string());
                            return;
                        };

                        state.is_logging_in = true;
                        state.error = None;
                        state.email.clear();
                        state.password.clear();

                        spawn_login_handler(sender.clone(), email, password);
                    }
                });
            });

        if requested_close {
            open = false;
        }

        if !state.is_logging_in {
            state.open = open;
        }
    }

    ui_two_factor(ctx, state);
}

fn ui_two_factor(ctx: &egui::Context, state: &mut LoginUi) {
    if !state.show_two_factor {
        return;
    }

    let mut open = state.show_two_factor;
    egui::Window::new("Two-Factor Authentication")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 90.0))
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label("Enter the verification code sent to your device:");
            ui.add(egui::TextEdit::singleline(&mut state.two_factor_code).desired_width(180.0));

            if let Some(error) = state.two_factor_error.as_ref() {
                ui.add_space(6.0);
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    finish_two_factor(state, Err("Two-factor authentication cancelled.".into()));
                    return;
                }

                let verify = ui.add_enabled(
                    !state.two_factor_code.trim().is_empty(),
                    egui::Button::new("Verify"),
                );
                if verify.clicked() {
                    let code = state.two_factor_code.trim().to_string();
                    finish_two_factor(state, Ok(code));
                }
            });
        });

    if !open && state.show_two_factor {
        finish_two_factor(state, Err("Two-factor authentication cancelled.".into()));
    }
}

fn finish_two_factor(state: &mut LoginUi, result: Result<String, String>) {
    if let Some(tx) = state.two_factor_tx.take() {
        if tx.send(result).is_err() {
            state.two_factor_error = Some("Failed to submit verification code.".to_string());
        } else {
            state.show_two_factor = false;
            state.two_factor_code.clear();
            state.two_factor_error = None;
        }
    } else {
        state.two_factor_error = Some("No 2FA request pending.".to_string());
    }
}

// -----------------------------------------------------------------------------
// Worker
// -----------------------------------------------------------------------------

fn spawn_login_handler(sender: mpsc::UnboundedSender<AppMessage>, email: String, password: String) {
    thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        let anisette_config =
            AnisetteConfiguration::default().set_configuration_path(get_data_path());

        let (code_tx, code_rx) = std_mpsc::channel::<Result<String, String>>();
        let sender_for_2fa = sender.clone();

        let account_result = rt.block_on(Account::login(
            || Ok((email.clone(), password.clone())),
            || {
                let _ = sender_for_2fa.send(AppMessage::LoginNeedsTwoFactor(code_tx.clone()));
                match code_rx.recv() {
                    Ok(result) => result,
                    Err(_) => Err("Two-factor authentication cancelled.".to_string()),
                }
            },
            anisette_config,
        ));

        match account_result {
            Ok(account) => match rt.block_on(account_from_session(email.clone(), account)) {
                Ok(gsa_account) => {
                    let _ = sender.send(AppMessage::AccountAdded(gsa_account));
                }
                Err(e) => {
                    let _ = sender.send(AppMessage::LoginFailed(format!(
                        "Failed to create GSA account from session: {}",
                        e
                    )));
                }
            },
            Err(e) => {
                let _ = sender.send(AppMessage::LoginFailed(format!("Login failed: {}", e)));
            }
        }
    });
}
