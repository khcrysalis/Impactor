use iced::widget::{button, column, container, row, rule, scrollable, text};
use iced::{Center, Color, Element, Task};

use crate::appearance;
use plume_utils::{Device, SignerAppReal};

#[derive(Debug, Clone)]
pub enum Message {
    RefreshApps,
    AppsLoaded(Result<Vec<SignerAppReal>, String>),
    InstallPairingFile(SignerAppReal),
    Trust,
    PairResult(Result<(), String>),
    InstallPairingResult(Result<(), String>),
}

#[derive(Debug, Clone)]
pub struct UtilitiesScreen {
    device: Option<Device>,
    installed_apps: Vec<SignerAppReal>,
    error_message: Option<String>,
    loading: bool,
    trust_loading: bool,
}

impl UtilitiesScreen {
    pub fn new(device: Option<Device>) -> Self {
        let mut screen = Self {
            device,
            installed_apps: Vec::new(),
            error_message: None,
            loading: false,
            trust_loading: false,
        };

        if screen.device.as_ref().map(|d| d.is_mac).unwrap_or(false) {
            screen.error_message = Some("macOS devices are not supported".to_string());
        }

        screen
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshApps => {
                self.loading = true;
                self.error_message = None;
                if let Some(device) = &self.device {
                    if device.is_mac {
                        return Task::none();
                    }

                    let device = device.clone();
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async move {
                            device
                                .installed_apps()
                                .await
                                .map_err(|e| format!("Failed to load apps: {}", e))
                        });
                        let _ = tx.send(result);
                    });

                    Task::perform(
                        async move {
                            std::thread::spawn(move || {
                                rx.recv()
                                    .unwrap_or_else(|_| Err("Failed to receive result".to_string()))
                            })
                            .join()
                            .unwrap()
                        },
                        Message::AppsLoaded,
                    )
                } else {
                    Task::done(Message::AppsLoaded(Err("No device connected".to_string())))
                }
            }
            Message::AppsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(apps) => {
                        self.installed_apps = apps;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                        self.installed_apps.clear();
                    }
                }
                Task::none()
            }
            Message::InstallPairingFile(app) => {
                if let Some(device) = &self.device {
                    let device = device.clone();
                    let bundle_id = app.bundle_id.clone().unwrap_or_default();
                    let pairing_path = app.app.pairing_file_path().unwrap_or_default();
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async move {
                            device
                                .install_pairing_record(&bundle_id, &pairing_path)
                                .await
                                .map_err(|e| format!("Failed to install pairing record: {}", e))
                        });
                        let _ = tx.send(result);
                    });

                    Task::perform(
                        async move {
                            std::thread::spawn(move || {
                                rx.recv()
                                    .unwrap_or_else(|_| Err("Failed to receive result".to_string()))
                            })
                            .join()
                            .unwrap()
                        },
                        Message::InstallPairingResult,
                    )
                } else {
                    Task::none()
                }
            }
            Message::Trust => {
                self.trust_loading = true;
                self.error_message = None;
                if let Some(device) = &self.device {
                    let device = device.clone();
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async move {
                            device
                                .pair()
                                .await
                                .map_err(|e| format!("Failed to pair: {}", e))
                        });
                        let _ = tx.send(result);
                    });

                    Task::perform(
                        async move {
                            std::thread::spawn(move || {
                                rx.recv()
                                    .unwrap_or_else(|_| Err("Failed to receive result".to_string()))
                            })
                            .join()
                            .unwrap()
                        },
                        Message::PairResult,
                    )
                } else {
                    Task::none()
                }
            }
            Message::PairResult(result) => {
                self.trust_loading = false;
                match result {
                    Ok(_) => {
                        self.error_message = Some("Device paired successfully!".to_string());
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
                Task::none()
            }
            Message::InstallPairingResult(result) => {
                match result {
                    Ok(_) => {
                        self.error_message =
                            Some("Pairing file installed successfully!".to_string());
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![].spacing(appearance::THEME_PADDING);

        if let Some(ref device) = self.device {
            content = content.push(
                column![
                    text(format!("Name: {}", device.name)),
                    text(format!("UDID: {}", device.udid)),
                ]
                .spacing(4),
            );
        } else {
            content =
                content.push(text("No device connected").color(Color::from_rgb(0.7, 0.7, 0.7)));
        }

        if let Some(ref error) = self.error_message {
            content = content.push(text(error).size(14).color(Color::from_rgb(0.9, 0.2, 0.2)));
        }

        if self.device.is_some() && !self.device.as_ref().unwrap().is_mac {
            let refresh_button_text = if self.loading {
                "Loading..."
            } else {
                "Refresh Installed Apps"
            };

            let trust_button_text = if self.trust_loading {
                "Pairing..."
            } else {
                "Trust Device"
            };

            content = content.push(
                row![
                    button(text(trust_button_text).align_x(Center))
                        .on_press_maybe(if self.trust_loading {
                            None
                        } else {
                            Some(Message::Trust)
                        })
                        .style(appearance::s_button)
                        .width(iced::Length::Fill),
                    button(text(refresh_button_text).align_x(Center))
                        .on_press_maybe(if self.loading {
                            None
                        } else {
                            Some(Message::RefreshApps)
                        })
                        .style(appearance::s_button)
                        .width(iced::Length::Fill),
                ]
                .spacing(appearance::THEME_PADDING),
            );
        }

        if !self.installed_apps.is_empty() {
            content = content
                .push(container(rule::horizontal(1)).padding([appearance::THEME_PADDING, 0.0]));

            let mut apps_list = column![].spacing(4);

            for app in &self.installed_apps {
                apps_list = apps_list.push(
                    row![
                        text(format!(
                            "{} ({})",
                            app.app.to_string(),
                            app.bundle_id.clone().unwrap_or("???".to_string())
                        ))
                        .size(14)
                        .width(iced::Length::Fill),
                        button(text("Install Pairing").align_x(Center))
                            .on_press(Message::InstallPairingFile(app.clone()))
                            .style(appearance::s_button)
                    ]
                    .spacing(appearance::THEME_PADDING)
                    .align_y(Center),
                );
            }

            content = content.push(apps_list);
        }

        container(scrollable(content)).into()
    }
}
