#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod defaults;
mod screen;
mod signer_options;
mod subscriptions;
mod tray;

use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Element, Fill, Subscription, Task};
use iced::{Theme, window};
use plume_store::AccountStore;
use plume_utils::{Device, Package, SignerOptions};

use crate::tray::ImpactorTray;

pub const APP_NAME: &str = "Impactor";
pub const APP_NAME_VERSIONED: &str = concat!("Impactor", " - Version ", env!("CARGO_PKG_VERSION"));

fn main() -> iced::Result {
    env_logger::init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    iced::daemon(Impactor::new, Impactor::update, Impactor::view)
        .subscription(Impactor::subscription)
        .title(APP_NAME_VERSIONED)
        .theme(Theme::GruvboxDark)
        .settings(defaults::default_settings())
        .run()
}

#[derive(Debug, Clone)]
pub enum Message {
    NavigateToScreen(ImpactorScreen),
    NextScreen,
    PreviousScreen,
    ComboBoxSelected(String),
    DeviceConnected(Device),
    DeviceDisconnected(u32),
    TrayMenuClicked(tray_icon::menu::MenuId),
    TrayIconClicked,
    ShowWindow,
    HideWindow,
    Quit,
    FilesHovered,
    FilesHoveredLeft,
    FilesDropped(Vec<std::path::PathBuf>),
    OpenFileDialog,
    FileDialogClosed(Option<std::path::PathBuf>),
    ShowLogin,
    LoginWindowMessage(window::Id, screen::login_window::Message),
    SelectAccount(usize),
    RemoveAccount(usize),
    ExportP12,
    // Signer options
    UpdateCustomName(String),
    UpdateCustomIdentifier(String),
    UpdateCustomVersion(String),
    ToggleMinimumOsVersion(bool),
    ToggleFileSharing(bool),
    ToggleIpadFullscreen(bool),
    ToggleGameMode(bool),
    ToggleProMotion(bool),
    ToggleSingleProfile(bool),
    ToggleLiquidGlass(bool),
    UpdateSignerMode(plume_utils::SignerMode),
    UpdateInstallMode(plume_utils::SignerInstallMode),
    AddTweak,
    AddTweakSelected(Option<std::path::PathBuf>),
    AddBundle,
    AddBundleSelected(Option<std::path::PathBuf>),
    RemoveTweak(usize),
    // Installation
    StartInstallation,
    InstallationProgress(String, i32),
    InstallationError(String),
    InstallationFinished,
}

struct Impactor {
    current_screen: ImpactorScreen,
    previous_screen: Option<ImpactorScreen>,
    devices: Vec<Device>,
    selected_device: Option<Device>,
    tray: Option<ImpactorTray>,
    main_window: Option<window::Id>,
    is_file_hovering: bool,
    installer: ImpactorInstaller,
    account_store: Option<AccountStore>,
    login_windows: std::collections::HashMap<window::Id, screen::login_window::LoginWindow>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImpactorScreen {
    Main,
    Settings,
    Installer,
    Progress,
}

#[derive(Debug, Clone, Default)]
pub struct ImpactorInstaller {
    pub selected_package_file: Option<Package>,
    pub package_options: SignerOptions,
    pub is_installing: bool,
    pub progress: i32,
    pub status: String,
    pub progress_rx:
        Option<std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<(String, i32)>>>>,
}

impl Impactor {
    fn new() -> (Self, Task<Message>) {
        let tray = ImpactorTray::new();
        let (id, open_task) = window::open(defaults::default_window_settings());

        (
            Self {
                current_screen: ImpactorScreen::Main,
                previous_screen: None,
                devices: Vec::new(),
                selected_device: None,
                tray: Some(tray),
                main_window: Some(id),
                is_file_hovering: false,
                installer: ImpactorInstaller::default(),
                account_store: Some(Self::init_account_store_sync()),
                login_windows: std::collections::HashMap::new(),
            },
            open_task.discard(),
        )
    }

    fn init_account_store_sync() -> AccountStore {
        let path = defaults::get_data_path().join("accounts.json");
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { AccountStore::load(&Some(path)).await.unwrap_or_default() })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ComboBoxSelected(value) => {
                self.selected_device = self
                    .devices
                    .iter()
                    .find(|d| d.to_string() == value)
                    .cloned();
                Task::none()
            }
            Message::DeviceConnected(device) => {
                if !self.devices.iter().any(|d| d.device_id == device.device_id) {
                    self.devices.push(device.clone());

                    if self.selected_device.is_none() && device.device_id != u32::MAX {
                        self.selected_device = Some(device.clone());
                    }
                }

                Task::none()
            }
            Message::DeviceDisconnected(id) => {
                self.devices.retain(|d| d.device_id != id);

                if self.selected_device.as_ref().map(|d| d.device_id) == Some(id) {
                    self.selected_device = self.devices.first().cloned();
                }

                Task::none()
            }
            Message::NavigateToScreen(screen) => {
                if screen == ImpactorScreen::Main {
                    if let Some(package) = self.installer.selected_package_file.take() {
                        package.remove_package_stage();
                    }
                    self.installer = ImpactorInstaller::default();
                }

                if screen == ImpactorScreen::Settings {
                    if self.current_screen != ImpactorScreen::Progress {
                        self.previous_screen = Some(self.current_screen.clone());
                    }
                }

                self.current_screen = screen;
                Task::none()
            }
            Message::NextScreen => {
                self.current_screen = match self.current_screen {
                    ImpactorScreen::Main => ImpactorScreen::Installer,
                    ImpactorScreen::Installer => ImpactorScreen::Progress,
                    ImpactorScreen::Settings => ImpactorScreen::Settings,
                    ImpactorScreen::Progress => ImpactorScreen::Progress,
                };

                Task::none()
            }
            Message::PreviousScreen => {
                self.current_screen = match self.current_screen {
                    ImpactorScreen::Main => ImpactorScreen::Main,
                    ImpactorScreen::Installer => {
                        if let Some(package) = self.installer.selected_package_file.take() {
                            package.remove_package_stage();
                        }
                        self.installer = ImpactorInstaller::default();
                        ImpactorScreen::Main
                    }
                    ImpactorScreen::Progress => {
                        if let Some(package) = self.installer.selected_package_file.take() {
                            package.remove_package_stage();
                        }
                        self.installer = ImpactorInstaller::default();
                        ImpactorScreen::Main
                    }
                    ImpactorScreen::Settings => {
                        self.previous_screen.take().unwrap_or(ImpactorScreen::Main)
                    }
                };

                Task::none()
            }
            Message::TrayIconClicked => Task::done(Message::ShowWindow),
            Message::TrayMenuClicked(id) => {
                if let Some(tray) = &self.tray {
                    if tray.is_quit_clicked(&id) {
                        Task::done(Message::Quit)
                    } else if tray.is_show_clicked(&id) {
                        Task::done(Message::ShowWindow)
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            Message::ShowWindow => {
                if let Some(id) = self.main_window {
                    window::gain_focus(id)
                } else {
                    let (id, open_task) = window::open(defaults::default_window_settings());
                    self.main_window = Some(id);
                    open_task.discard()
                }
            }
            Message::HideWindow => {
                if let Some(id) = self.main_window {
                    self.main_window = None;
                    window::close(id)
                } else {
                    Task::none()
                }
            }
            Message::Quit => {
                self.tray.take();
                std::process::exit(0);
            }
            Message::FilesHovered => {
                self.is_file_hovering = true;
                Task::none()
            }
            Message::FilesHoveredLeft => {
                self.is_file_hovering = false;
                Task::none()
            }
            Message::FilesDropped(paths) => {
                self.is_file_hovering = false;

                for path in paths {
                    if let Some(ext) = path.extension() {
                        if ext == "ipa" || ext == "tipa" {
                            if let Ok(package) = Package::new(path) {
                                package
                                    .load_into_signer_options(&mut self.installer.package_options);
                                self.installer.selected_package_file = Some(package);
                                return Task::done(Message::NextScreen);
                            }
                        }
                    }
                }

                Task::none()
            }
            Message::OpenFileDialog => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("iOS App Package", &["ipa", "tipa"])
                        .set_title("Select IPA/TIPA file")
                        .pick_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                },
                Message::FileDialogClosed,
            ),
            Message::FileDialogClosed(path) => {
                if let Some(path) = path {
                    if let Ok(package) = Package::new(path) {
                        self.installer.selected_package_file = Some(package);
                        return Task::done(Message::NextScreen);
                    }
                }
                Task::none()
            }
            Message::ShowLogin => {
                let (login_window, task) = screen::login_window::LoginWindow::new();
                let id = login_window.window_id().unwrap();
                self.login_windows.insert(id, login_window);
                task.map(move |msg| Message::LoginWindowMessage(id, msg))
            }
            Message::LoginWindowMessage(id, msg) => {
                if let Some(login_window) = self.login_windows.get_mut(&id) {
                    let task = login_window.update(msg.clone());

                    if matches!(
                        msg,
                        screen::login_window::Message::LoginSuccess(_)
                            | screen::login_window::Message::LoginCancel
                            | screen::login_window::Message::TwoFactorCancel
                    ) {
                        self.login_windows.remove(&id);
                        self.account_store = Some(Self::init_account_store_sync());
                    }

                    task.map(move |msg| Message::LoginWindowMessage(id, msg))
                } else {
                    Task::none()
                }
            }
            Message::SelectAccount(index) => {
                if let Some(store) = &mut self.account_store {
                    let mut emails: Vec<_> = store.accounts().keys().cloned().collect();
                    emails.sort();
                    if let Some(email) = emails.get(index) {
                        let _ = store.account_select_sync(email);
                    }
                }
                Task::none()
            }
            Message::RemoveAccount(index) => {
                if let Some(store) = &mut self.account_store {
                    let mut emails: Vec<_> = store.accounts().keys().cloned().collect();
                    emails.sort();
                    if let Some(email) = emails.get(index) {
                        let _ = store.accounts_remove_sync(email);
                    }
                }
                Task::none()
            }
            Message::ExportP12 => {
                if let Some(account) = self
                    .account_store
                    .as_ref()
                    .and_then(|s| s.selected_account().cloned())
                {
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();

                        rt.block_on(async move {
                            match subscriptions::export_certificate(account).await {
                                Ok(_) => {}
                                Err(e) => {
                                    rfd::MessageDialog::new()
                                        .set_title("Export Failed")
                                        .set_description(&e)
                                        .set_buttons(rfd::MessageButtons::Ok)
                                        .show();
                                }
                            }
                        });
                    });
                }
                Task::none()
            }
            Message::UpdateCustomName(_)
            | Message::UpdateCustomIdentifier(_)
            | Message::UpdateCustomVersion(_)
            | Message::ToggleMinimumOsVersion(_)
            | Message::ToggleFileSharing(_)
            | Message::ToggleIpadFullscreen(_)
            | Message::ToggleGameMode(_)
            | Message::ToggleProMotion(_)
            | Message::ToggleSingleProfile(_)
            | Message::ToggleLiquidGlass(_)
            | Message::UpdateSignerMode(_)
            | Message::UpdateInstallMode(_)
            | Message::AddTweak
            | Message::AddTweakSelected(_)
            | Message::AddBundle
            | Message::AddBundleSelected(_)
            | Message::RemoveTweak(_) => {
                signer_options::handle_message(&mut self.installer, &message)
            }
            // Installation handlers
            Message::StartInstallation => {
                if self.installer.package_options.mode == plume_utils::SignerMode::Pem {
                    if self
                        .account_store
                        .as_ref()
                        .and_then(|s| s.selected_account())
                        .is_none()
                    {
                        std::thread::spawn(|| {
                            rfd::MessageDialog::new()
                                .set_title("Account Required")
                                .set_description("Please add and select an Apple ID account in Settings to use PEM signing mode.")
                                .set_buttons(rfd::MessageButtons::Ok)
                                .show();
                        });
                        return Task::none();
                    }
                }

                self.installer.is_installing = true;
                self.installer.progress = 0;
                self.installer.status = "Starting...".to_string();
                self.current_screen = ImpactorScreen::Progress;

                let Some(package) = self.installer.selected_package_file.clone() else {
                    return Task::none();
                };

                let device = self.selected_device.clone();
                let options = self.installer.package_options.clone();
                let account = self
                    .account_store
                    .as_ref()
                    .and_then(|s| s.selected_account().cloned());

                let (tx, rx) = std::sync::mpsc::channel();
                self.installer.progress_rx = Some(std::sync::Arc::new(std::sync::Mutex::new(rx)));

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let tx_error = tx.clone();
                    rt.block_on(async move {
                        match subscriptions::run_installation(
                            device,
                            package,
                            account,
                            options,
                            move |status, progress| {
                                let _ = tx.send((status, progress));
                            },
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                let _ = tx_error.send((format!("Error: {}", e), -1));
                            }
                        }
                    });
                });

                Task::none()
            }
            Message::InstallationProgress(status, progress) => {
                self.installer.status = status.clone();
                self.installer.progress = progress;

                if progress == -1 {
                    self.installer.progress_rx = None;

                    let error_msg = status.clone();
                    std::thread::spawn(move || {
                        rfd::MessageDialog::new()
                            .set_title("Installation Failed")
                            .set_description(&error_msg)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                    });

                    Task::none()
                } else if progress >= 100 {
                    self.installer.progress_rx = None;

                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::InstallationError(error) => {
                self.installer.progress = -1;
                self.installer.status = format!("Error: {}", error);
                self.installer.progress_rx = None;

                std::thread::spawn(move || {
                    rfd::MessageDialog::new()
                        .set_title("Installation Failed")
                        .set_description(&error)
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();
                });

                Task::none()
            }
            Message::InstallationFinished => {
                self.installer.progress = 100;
                self.installer.status = "Finished!".to_string();
                self.installer.progress_rx = None;

                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let device_subscription = subscriptions::device_listener();
        let tray_subscription = subscriptions::tray_subscription();
        let hover_subscription = subscriptions::file_hover_subscription();
        let progress_subscription =
            subscriptions::installation_progress_listener(self.installer.progress_rx.clone());

        Subscription::batch(vec![
            device_subscription,
            tray_subscription,
            hover_subscription,
            progress_subscription,
        ])
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if let Some(login_window) = self.login_windows.get(&window_id) {
            return login_window
                .view()
                .map(move |msg| Message::LoginWindowMessage(window_id, msg));
        }

        let screen_content = match self.current_screen {
            ImpactorScreen::Main => screen::impactor_main::view(self.is_file_hovering),
            ImpactorScreen::Settings => {
                screen::impactor_settings::view(self.account_store.as_ref())
            }
            ImpactorScreen::Installer => screen::impactor_installer::view(
                self.installer.selected_package_file.as_ref(),
                &self.installer.package_options,
            ),
            ImpactorScreen::Progress => {
                screen::installer_progress::view(&self.installer.status, self.installer.progress)
            }
        };

        let mut content = column![];

        if self.current_screen != ImpactorScreen::Settings {
            let device_names: Vec<String> = self.devices.iter().map(|d| d.to_string()).collect();

            let selected_text = self
                .selected_device
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "No Device".into());

            let top_bar = container(
                row![
                    container(text("")).width(Fill),
                    pick_list(
                        device_names,
                        self.selected_device.as_ref().map(|d| d.to_string()),
                        Message::ComboBoxSelected
                    )
                    .placeholder(selected_text.as_str())
                    .width(250),
                    button("⚙").on_press(Message::NavigateToScreen(ImpactorScreen::Settings))
                ]
                .spacing(10),
            )
            .padding(10)
            .width(Fill);

            content = content.push(top_bar);
        } else {
            let settings_top = container(row![
                container(text("")).width(Fill),
                button("Back").on_press(Message::PreviousScreen)
            ])
            .padding(10)
            .width(Fill);

            content = content.push(settings_top);
        }

        content = content.push(container(screen_content).center(Fill).height(Fill));

        match self.current_screen {
            ImpactorScreen::Main => {
                let bottom_bar = container(
                    row![
                        button("Import .ipa / .tipa")
                            .on_press(Message::OpenFileDialog)
                            .width(Fill),
                    ]
                    .spacing(10),
                )
                .padding(10)
                .width(Fill);

                content = content.push(bottom_bar);
            }
            ImpactorScreen::Installer => {
                let (button_enabled, button_label) =
                    match self.installer.package_options.install_mode {
                        plume_utils::SignerInstallMode::Export => (true, "Export"),
                        plume_utils::SignerInstallMode::Install => {
                            (self.selected_device.is_some(), "Install")
                        }
                    };

                let bottom_bar = container(
                    row![
                        button("Back").on_press(Message::PreviousScreen).width(Fill),
                        button(button_label)
                            .on_press_maybe(if button_enabled {
                                Some(Message::StartInstallation)
                            } else {
                                None
                            })
                            .width(Fill),
                    ]
                    .spacing(10),
                )
                .padding(10)
                .width(Fill);

                content = content.push(bottom_bar);
            }
            ImpactorScreen::Progress => {
                let bottom_bar = container(
                    row![
                        button("Back")
                            .on_press_maybe(
                                if self.installer.progress == -1 || self.installer.progress >= 100 {
                                    Some(Message::PreviousScreen)
                                } else {
                                    None
                                }
                            )
                            .width(Fill),
                    ]
                    .spacing(10),
                )
                .padding(10)
                .width(Fill);

                content = content.push(bottom_bar);
            }
            _ => {}
        }

        container(content).into()
    }
}
