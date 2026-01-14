pub(crate) mod general;
mod package;
mod progress;
pub(crate) mod settings;
mod utilties;
mod windows;

use iced::Length::Fill;
use iced::widget::{button, container, pick_list, row, text};
use iced::window;
use iced::{Element, Subscription, Task};

use plume_store::AccountStore;
use plume_utils::{Device, SignerOptions};

use crate::subscriptions;
use crate::tray::ImpactorTray;
use crate::{appearance, defaults};
use windows::login_window;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // Navigation
    NavigateToScreen(ImpactorScreenType),
    NextScreen,
    PreviousScreen,

    // Device management
    ComboBoxSelected(String),
    DeviceConnected(Device),
    DeviceDisconnected(u32),

    // Tray
    TrayMenuClicked(tray_icon::menu::MenuId),
    TrayIconClicked,
    #[cfg(target_os = "linux")]
    GtkTick,

    // Window management
    ShowWindow,
    HideWindow,
    Quit,

    // Login window
    LoginWindowMessage(window::Id, login_window::Message),

    // Screen-specific messages
    MainScreen(general::Message),
    UtilitiesScreen(utilties::Message),
    SettingsScreen(settings::Message),
    InstallerScreen(package::Message),
    ProgressScreen(progress::Message),

    // Installation
    StartInstallation,
}

pub struct Impactor {
    current_screen: ImpactorScreen,
    previous_screen: Option<Box<ImpactorScreen>>,
    devices: Vec<Device>,
    selected_device: Option<Device>,
    tray: Option<ImpactorTray>,
    main_window: Option<window::Id>,
    account_store: Option<AccountStore>,
    login_windows: std::collections::HashMap<window::Id, login_window::LoginWindow>,
    pending_installation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImpactorScreenType {
    Main,
    Utilities,
    Settings,
    Installer,
    Progress,
}

enum ImpactorScreen {
    Main(general::GeneralScreen),
    Utilities(utilties::UtilitiesScreen),
    Settings(settings::SettingsScreen),
    Installer(package::PackageScreen),
    Progress(progress::ProgressScreen),
}

impl Impactor {
    pub fn new() -> (Self, Task<Message>) {
        let tray = ImpactorTray::new();
        let (id, open_task) = window::open(defaults::default_window_settings());

        (
            Self {
                current_screen: ImpactorScreen::Main(general::GeneralScreen::new()),
                previous_screen: None,
                devices: Vec::new(),
                selected_device: None,
                tray: Some(tray),
                main_window: Some(id),
                account_store: Some(Self::init_account_store_sync()),
                login_windows: std::collections::HashMap::new(),
                pending_installation: false,
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ComboBoxSelected(value) => {
                self.selected_device = self
                    .devices
                    .iter()
                    .find(|d| d.to_string() == value)
                    .cloned();

                if let ImpactorScreen::Utilities(_) = self.current_screen {
                    self.current_screen = ImpactorScreen::Utilities(
                        utilties::UtilitiesScreen::new(self.selected_device.clone()),
                    );
                    return Task::done(Message::UtilitiesScreen(utilties::Message::RefreshApps));
                }

                Task::none()
            }
            Message::DeviceConnected(device) => {
                if !self.devices.iter().any(|d| d.device_id == device.device_id) {
                    self.devices.push(device.clone());

                    if self.selected_device.is_none() && device.device_id != u32::MAX {
                        self.selected_device = Some(device.clone());
                    }
                }

                if let ImpactorScreen::Utilities(_) = self.current_screen {
                    self.current_screen = ImpactorScreen::Utilities(
                        utilties::UtilitiesScreen::new(self.selected_device.clone()),
                    );
                    return Task::done(Message::UtilitiesScreen(utilties::Message::RefreshApps));
                }

                Task::none()
            }
            Message::DeviceDisconnected(id) => {
                self.devices.retain(|d| d.device_id != id);

                if self.selected_device.as_ref().map(|d| d.device_id) == Some(id) {
                    self.selected_device = self.devices.first().cloned();
                }

                if let ImpactorScreen::Utilities(_) = self.current_screen {
                    self.current_screen = ImpactorScreen::Utilities(
                        utilties::UtilitiesScreen::new(self.selected_device.clone()),
                    );
                    return Task::done(Message::UtilitiesScreen(utilties::Message::RefreshApps));
                }

                Task::none()
            }
            Message::NavigateToScreen(screen_type) => {
                if screen_type == ImpactorScreenType::Settings {
                    if !matches!(self.current_screen, ImpactorScreen::Progress(_)) {
                        self.previous_screen = Some(Box::new(std::mem::replace(
                            &mut self.current_screen,
                            ImpactorScreen::Main(general::GeneralScreen::new()),
                        )));
                    }
                }

                self.navigate_to_screen(screen_type.clone());

                if screen_type == ImpactorScreenType::Utilities {
                    return Task::done(Message::UtilitiesScreen(utilties::Message::RefreshApps));
                }

                Task::none()
            }
            Message::NextScreen => {
                let next_screen = match self.current_screen {
                    ImpactorScreen::Main(_) => ImpactorScreenType::Installer,
                    ImpactorScreen::Utilities(_) => return Task::none(),
                    ImpactorScreen::Installer(_) => ImpactorScreenType::Progress,
                    ImpactorScreen::Settings(_) => return Task::none(),
                    ImpactorScreen::Progress(_) => return Task::none(),
                };

                self.navigate_to_screen(next_screen);
                Task::none()
            }
            Message::PreviousScreen => match &self.current_screen {
                ImpactorScreen::Main(_) => Task::none(),
                ImpactorScreen::Utilities(_) => {
                    self.navigate_to_screen(ImpactorScreenType::Main);
                    Task::none()
                }
                ImpactorScreen::Installer(_) => {
                    self.navigate_to_screen(ImpactorScreenType::Main);
                    Task::none()
                }
                ImpactorScreen::Progress(_) => {
                    self.navigate_to_screen(ImpactorScreenType::Main);
                    Task::none()
                }
                ImpactorScreen::Settings(_) => {
                    if let Some(prev_screen) = self.previous_screen.take() {
                        self.current_screen = *prev_screen;
                    } else {
                        self.navigate_to_screen(ImpactorScreenType::Main);
                    }
                    Task::none()
                }
            },
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
            #[cfg(target_os = "linux")]
            Message::GtkTick => {
                while gtk::glib::MainContext::default().iteration(false) {}
                Task::none()
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
            Message::LoginWindowMessage(id, msg) => {
                if let Some(login_window) = self.login_windows.get_mut(&id) {
                    let task = login_window.update(msg.clone());

                    if matches!(
                        msg,
                        login_window::Message::LoginSuccess(_)
                            | login_window::Message::LoginCancel
                            | login_window::Message::TwoFactorCancel
                    ) {
                        self.login_windows.remove(&id);
                        self.account_store = Some(Self::init_account_store_sync());

                        if let ImpactorScreen::Settings(_) = self.current_screen {
                            self.current_screen =
                                ImpactorScreen::Settings(settings::SettingsScreen::new());
                        }

                        if self.pending_installation {
                            if matches!(msg, login_window::Message::LoginSuccess(_)) {
                                self.pending_installation = false;
                                return Task::batch(vec![
                                    window::close(id),
                                    Task::done(Message::InstallerScreen(
                                        package::Message::RequestInstallation,
                                    )),
                                ]);
                            }
                        }

                        return window::close(id);
                    }

                    task.map(move |msg| Message::LoginWindowMessage(id, msg))
                } else {
                    Task::none()
                }
            }
            Message::MainScreen(msg) => {
                if let ImpactorScreen::Main(ref mut screen) = self.current_screen {
                    let task = screen.update(msg.clone()).map(Message::MainScreen);

                    if let general::Message::NavigateToInstaller(package) = msg {
                        let options = SignerOptions::default();
                        self.current_screen = ImpactorScreen::Installer(
                            package::PackageScreen::new(Some(package), options),
                        );
                    } else if let general::Message::NavigateToUtilities = msg {
                        self.current_screen = ImpactorScreen::Utilities(
                            utilties::UtilitiesScreen::new(self.selected_device.clone()),
                        );
                        return Task::done(Message::UtilitiesScreen(
                            utilties::Message::RefreshApps,
                        ));
                    }

                    task
                } else {
                    Task::none()
                }
            }
            Message::UtilitiesScreen(msg) => {
                if let ImpactorScreen::Utilities(ref mut screen) = self.current_screen {
                    screen.update(msg).map(Message::UtilitiesScreen)
                } else {
                    Task::none()
                }
            }
            Message::SettingsScreen(msg) => {
                if let ImpactorScreen::Settings(ref mut screen) = self.current_screen {
                    match msg {
                        settings::Message::ShowLogin => {
                            let (login_window, task) = login_window::LoginWindow::new();
                            let id = login_window.window_id().unwrap();
                            self.login_windows.insert(id, login_window);
                            task.map(move |msg| Message::LoginWindowMessage(id, msg))
                        }
                        settings::Message::SelectAccount(index) => {
                            if let Some(store) = &mut self.account_store {
                                let mut emails: Vec<_> = store.accounts().keys().cloned().collect();
                                emails.sort();
                                if let Some(email) = emails.get(index) {
                                    let _ = store.account_select_sync(email);
                                }
                            }
                            Task::none()
                        }
                        settings::Message::RemoveAccount(index) => {
                            if let Some(store) = &mut self.account_store {
                                let mut emails: Vec<_> = store.accounts().keys().cloned().collect();
                                emails.sort();
                                if let Some(email) = emails.get(index) {
                                    let _ = store.accounts_remove_sync(email);
                                }
                            }
                            Task::none()
                        }
                        settings::Message::ExportP12 => {
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

                                    let _ = rt.block_on(async move {
                                        crate::subscriptions::export_certificate(account).await
                                    });
                                });
                            }
                            Task::none()
                        }
                        settings::Message::FetchTeams(ref email) => {
                            if let Some(account_store) = &self.account_store {
                                if let Some(account) = account_store.accounts().get(email) {
                                    let account_clone = account.clone();
                                    let email_clone = email.clone();

                                    return Task::perform(
                                        async move {
                                            let (tx, rx) = std::sync::mpsc::channel();

                                            std::thread::spawn(move || {
                                                let rt = tokio::runtime::Runtime::new().unwrap();
                                                let result = rt.block_on(async move {
                                                    crate::subscriptions::fetch_teams(
                                                        &account_clone,
                                                    )
                                                    .await
                                                    .unwrap_or_else(|e| {
                                                        eprintln!("Failed to fetch teams: {}", e);
                                                        Vec::new()
                                                    })
                                                });
                                                let _ = tx.send(result);
                                            });

                                            rx.recv().unwrap_or_default()
                                        },
                                        move |teams| {
                                            Message::SettingsScreen(settings::Message::TeamsLoaded(
                                                email_clone,
                                                teams,
                                            ))
                                        },
                                    );
                                }
                            }
                            screen.update(msg).map(Message::SettingsScreen)
                        }
                        settings::Message::SelectTeam(ref email, ref team_id) => {
                            if let Some(store) = &mut self.account_store {
                                if let Err(e) =
                                    store.update_account_team_sync(email, team_id.clone())
                                {
                                    eprintln!("Failed to update team: {:?}", e);
                                } else {
                                    self.account_store = Some(Self::init_account_store_sync());
                                }
                            }
                            screen.update(msg).map(Message::SettingsScreen)
                        }
                        _ => screen.update(msg).map(Message::SettingsScreen),
                    }
                } else {
                    Task::none()
                }
            }
            Message::InstallerScreen(msg) => {
                if let ImpactorScreen::Installer(ref mut screen) = self.current_screen {
                    match msg {
                        package::Message::Back => Task::done(Message::PreviousScreen),
                        package::Message::RequestInstallation => {
                            if screen.selected_package.is_none() {
                                return Task::none();
                            }

                            use plume_utils::SignerMode;
                            if matches!(screen.options.mode, SignerMode::Pem) {
                                if self
                                    .account_store
                                    .as_ref()
                                    .and_then(|s| s.selected_account())
                                    .is_none()
                                {
                                    self.pending_installation = true;

                                    let (login_window, task) = login_window::LoginWindow::new();
                                    let id = login_window.window_id().unwrap();
                                    self.login_windows.insert(id, login_window);
                                    return task
                                        .map(move |msg| Message::LoginWindowMessage(id, msg));
                                }
                            }

                            self.start_installation_task()
                        }
                        _ => screen.update(msg).map(Message::InstallerScreen),
                    }
                } else {
                    Task::none()
                }
            }
            Message::ProgressScreen(msg) => {
                if let ImpactorScreen::Progress(ref mut screen) = self.current_screen {
                    match msg {
                        progress::Message::Back => Task::done(Message::PreviousScreen),
                        _ => screen.update(msg).map(Message::ProgressScreen),
                    }
                } else {
                    Task::none()
                }
            }
            Message::StartInstallation => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let device_subscription = subscriptions::device_listener();

        let tray_subscription = subscriptions::tray_subscription();

        let hover_subscription = if let ImpactorScreen::Main(_) = self.current_screen {
            subscriptions::file_hover_subscription()
        } else {
            Subscription::none()
        };

        let progress_subscription =
            if let ImpactorScreen::Progress(ref progress) = self.current_screen {
                subscriptions::installation_progress_listener(progress.progress_rx.clone()).map(
                    |(status, progress_val)| {
                        Message::ProgressScreen(progress::Message::InstallationProgress(
                            status,
                            progress_val,
                        ))
                    },
                )
            } else {
                Subscription::none()
            };

        let close_subscription = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Window(window::Event::CloseRequested) = event {
                return Some(Message::HideWindow);
            }
            None
        });

        Subscription::batch(vec![
            device_subscription,
            tray_subscription,
            hover_subscription,
            progress_subscription,
            close_subscription,
        ])
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        use iced::widget::{column, container};

        if let Some(login_window) = self.login_windows.get(&window_id) {
            return login_window
                .view()
                .map(move |msg| Message::LoginWindowMessage(window_id, msg));
        }

        let has_device = self.selected_device.is_some();
        let screen_content = self.view_current_screen(has_device);
        let top_bar = self.view_top_bar();

        container(column(vec![top_bar, screen_content]).spacing(appearance::THEME_PADDING))
            .padding(appearance::THEME_PADDING)
            .into()
    }

    fn view_current_screen(&self, has_device: bool) -> Element<'_, Message> {
        match &self.current_screen {
            ImpactorScreen::Main(screen) => screen.view().map(Message::MainScreen),
            ImpactorScreen::Utilities(screen) => screen.view().map(Message::UtilitiesScreen),
            ImpactorScreen::Settings(screen) => screen
                .view(&self.account_store)
                .map(Message::SettingsScreen),
            ImpactorScreen::Installer(screen) => {
                screen.view(has_device).map(Message::InstallerScreen)
            }
            ImpactorScreen::Progress(screen) => screen.view().map(Message::ProgressScreen),
        }
    }

    fn view_top_bar(&self) -> Element<'_, Message> {
        let device_names: Vec<String> = self.devices.iter().map(|d| d.to_string()).collect();
        let selected_device_name = self.selected_device.as_ref().map(|d| d.to_string());
        let placeholder_str = selected_device_name
            .as_ref()
            .map(String::as_str)
            .unwrap_or("No Device");

        let right_button = if matches!(self.current_screen, ImpactorScreen::Settings(_)) {
            button(appearance::icon(appearance::CHEVRON_BACK))
                .on_press(Message::PreviousScreen)
                .style(appearance::s_button)
        } else if matches!(self.current_screen, ImpactorScreen::Utilities(_)) {
            button(appearance::icon(appearance::CHEVRON_BACK))
                .on_press(Message::PreviousScreen)
                .style(appearance::s_button)
        } else {
            button(appearance::icon(appearance::GEAR))
                .style(appearance::s_button)
                .on_press(Message::NavigateToScreen(ImpactorScreenType::Settings))
        };

        container(
            row![
                container(text("")).width(Fill),
                pick_list(
                    device_names,
                    selected_device_name.clone(),
                    Message::ComboBoxSelected
                )
                .style(appearance::s_pick_list)
                .placeholder(placeholder_str)
                .width(250),
                right_button
            ]
            .spacing(appearance::THEME_PADDING),
        )
        .width(Fill)
        .into()
    }

    fn navigate_to_screen(&mut self, screen_type: ImpactorScreenType) {
        match screen_type {
            ImpactorScreenType::Main => {
                self.current_screen = ImpactorScreen::Main(general::GeneralScreen::new());
            }
            ImpactorScreenType::Utilities => {
                self.current_screen = ImpactorScreen::Utilities(utilties::UtilitiesScreen::new(
                    self.selected_device.clone(),
                ));
            }
            ImpactorScreenType::Settings => {
                self.current_screen = ImpactorScreen::Settings(settings::SettingsScreen::new());
            }
            ImpactorScreenType::Progress => {
                self.current_screen = ImpactorScreen::Progress(progress::ProgressScreen::new());
            }
            _ => {}
        }
    }

    fn start_installation_task(&mut self) -> Task<Message> {
        if let ImpactorScreen::Installer(installer) = &self.current_screen {
            let Some(package) = installer.selected_package.clone() else {
                return Task::none();
            };

            let device = self.selected_device.clone();
            let options = installer.options.clone();
            let account = self
                .account_store
                .as_ref()
                .and_then(|s| s.selected_account().cloned());

            let (tx, rx) = std::sync::mpsc::channel();
            let progress_rx = std::sync::Arc::new(std::sync::Mutex::new(rx));

            let mut progress_screen = progress::ProgressScreen::new();
            progress_screen.start_installation(progress_rx.clone());
            self.current_screen = ImpactorScreen::Progress(progress_screen);

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let tx_error = tx.clone();
                rt.block_on(async move {
                    match subscriptions::run_installation(
                        &package,
                        device.as_ref(),
                        &options,
                        account.as_ref(),
                        &tx,
                    )
                    .await
                    {
                        Ok(_) => {
                            let _ = tx.send(("Installation complete!".to_string(), 100));
                        }
                        Err(e) => {
                            let _ = tx_error.send((format!("Error: {}", e), -1));
                        }
                    }
                });
            });

            Task::none()
        } else {
            Task::none()
        }
    }
}
