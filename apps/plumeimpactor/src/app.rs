use std::{cell::RefCell, rc::Rc, sync::mpsc as std_mpsc};

use eframe::egui;
use eframe::epaint::ColorImage;

use plume_store::{AccountStore, GsaAccount};
use plume_utils::{Device, Package, PlistInfoTrait, SignerInstallMode, SignerMode, SignerOptions};

use tokio::sync::mpsc;

use tray_icon::{
    TrayIcon,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

use crate::listeners::{
    spawn_certificate_export_handler, spawn_package_handler, spawn_pair_handler,
};
use crate::login::LoginUi;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicIsize, Ordering},
};

// -----------------------------------------------------------------------------
// App
// -----------------------------------------------------------------------------

pub(crate) struct ImpactorApp {
    pub(crate) devices: Vec<Device>,
    pub(crate) selected_device: Option<u32>,
    pub(crate) selected_package: Option<Package>,
    pub(crate) selected_settings: SignerOptions,
    pub(crate) store: Option<AccountStore>,
    pub(crate) is_working: bool,
    pub(crate) is_work_button_enabled: bool,
    pub(crate) working_status: (String, i32),
    pub(crate) receiver: Option<mpsc::UnboundedReceiver<AppMessage>>,
    pub(crate) install_image: Option<egui::TextureHandle>,
    pub(crate) tray_menu_events: Option<std_mpsc::Receiver<MenuEvent>>,
    pub(crate) tray_icon: Rc<RefCell<Option<TrayIcon>>>,
    pub(crate) tray_menu_dirty: bool,
    pub(crate) show_settings: bool,
    pub(crate) sender: Option<mpsc::UnboundedSender<AppMessage>>,
    pub(crate) login_ui: LoginUi,
    #[cfg(target_os = "windows")]
    pub(crate) win32_hwnd: Option<Arc<AtomicIsize>>,
}

impl Default for ImpactorApp {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            selected_device: None,
            selected_package: None,
            selected_settings: SignerOptions::default(),
            store: None,
            is_working: false,
            is_work_button_enabled: false,
            working_status: ("Idle".to_string(), 0),
            receiver: None,
            install_image: None,
            tray_menu_events: None,
            tray_icon: Rc::new(RefCell::new(None)),
            tray_menu_dirty: true,
            show_settings: false,
            sender: None,
            login_ui: LoginUi::default(),
            #[cfg(target_os = "windows")]
            win32_hwnd: None,
        }
    }
}

impl eframe::App for ImpactorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        #[cfg(target_os = "linux")]
        {
            while gtk::glib::MainContext::default().iteration(false) {}
        }

        // ---------------- Async messages ----------------
        if let Some(mut rx) = self.receiver.take() {
            while let Ok(msg) = rx.try_recv() {
                self.handle_message(msg);
            }
            self.receiver = Some(rx);
        }

        #[cfg(target_os = "windows")]
        cache_hwnd(_frame, self.win32_hwnd.as_ref());

        // ---------------- Tray menu events ----------------
        if let Some(rx) = self.tray_menu_events.as_ref() {
            while let Ok(event) = rx.try_recv() {
                match event.id.as_ref() {
                    "open" => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    "quit" => std::process::exit(0),
                    _ => {
                        println!("Unknown tray menu event: {:?}", event.id);
                    }
                }
            }
        }

        // ---------------- Load image ONCE ----------------
        if self.install_image.is_none() {
            if let Ok(image) = load_embedded_install_image() {
                self.install_image =
                    Some(ctx.load_texture("install_png", image, Default::default()));
            }
        }

        // ---------------- Tray menu update ----------------
        if self.tray_menu_dirty {
            if let Some(tray) = self.tray_icon.borrow().as_ref() {
                let menu = build_tray_menu(self);
                let _ = tray.set_menu(Some(Box::new(menu)));
                self.tray_menu_dirty = false;
            }
        }

        // ---------------- UI ----------------
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.show_settings {
                ui.horizontal(|ui| {
                    if ui.button(" + ").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("IPA files", &["ipa", "tipa"])
                            .pick_file()
                        {
                            self.handle_message(AppMessage::PackageSelected(
                                path.display().to_string(),
                            ));
                        }
                    }

                    egui::ComboBox::from_id_salt("device_picker")
                        .selected_text(
                            self.selected_device
                                .and_then(|id| self.devices.iter().find(|d| d.device_id == id))
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| "No device".into()),
                        )
                        .show_ui(ui, |ui| {
                            for dev in &self.devices {
                                ui.selectable_value(
                                    &mut self.selected_device,
                                    Some(dev.device_id),
                                    dev.to_string(),
                                );
                            }
                        });

                    if let Some(device_id) = self.selected_device {
                        if ui.button("Pair").clicked() {
                            let selected_device = self
                                .devices
                                .iter()
                                .find(|d| Some(d.device_id) == Some(device_id));

                            spawn_pair_handler(selected_device.cloned());
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⚙ Settings").clicked() {
                            self.show_settings = true;
                        }
                        let _ = ui.button("Utilities");
                    });
                });

                ui.separator();

                if self.is_working {
                    ui_package_work(ui, self);
                } else if let Some(pkg) = self.selected_package.clone() {
                    ui_package_settings(ui, self, &pkg);
                } else {
                    ui_drag_drop(ui, self);
                }
            } else {
                ui_settings(ui, self);
            }
        });

        crate::login::ui_login(ctx, &mut self.login_ui, self.sender.as_ref());

        ctx.request_repaint();
    }
}

#[cfg(target_os = "windows")]
fn cache_hwnd(frame: &mut eframe::Frame, win32_hwnd: Option<&Arc<AtomicIsize>>) {
    let Some(win32_hwnd) = win32_hwnd else {
        return;
    };
    if win32_hwnd.load(Ordering::Acquire) != 0 {
        return;
    }
    let Ok(handle) = frame.window_handle() else {
        return;
    };
    if let RawWindowHandle::Win32(handle) = handle.as_raw() {
        win32_hwnd.store(handle.hwnd.get(), Ordering::Release);
    }
}

// -----------------------------------------------------------------------------
// egui (Components)
// -----------------------------------------------------------------------------

fn build_tray_menu(app: &ImpactorApp) -> Menu {
    let menu = Menu::new();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    menu.append(&MenuItem::with_id("open", "Open", true, None))
        .unwrap();

    #[cfg(target_os = "linux")]
    {
        menu.append(&MenuItem::new("Devices", false, None)).unwrap();
        if app.devices.is_empty() {
            menu.append(&MenuItem::new("  No devices connected", false, None))
                .unwrap();
        } else {
            for dev in &app.devices {
                menu.append(&MenuItem::new(format!("  {}", dev), true, None))
                    .unwrap();
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let devices_submenu = Submenu::new("Devices", true);
        if app.devices.is_empty() {
            devices_submenu
                .append(&MenuItem::new("No devices connected", false, None))
                .unwrap();
        } else {
            for dev in &app.devices {
                devices_submenu
                    .append(&MenuItem::new(dev.to_string(), true, None))
                    .unwrap();
            }
        }
        menu.append(&devices_submenu).unwrap();
    }
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&MenuItem::with_id("quit", "Quit Impactor", true, None))
        .unwrap();

    menu
}

fn ui_drag_drop(ui: &mut egui::Ui, app: &mut ImpactorApp) {
    let available = ui.available_size();
    let drag_rect = ui.allocate_exact_size(available, egui::Sense::hover()).0;

    let fixed_size = egui::Vec2::new(128.0, 128.0);
    let spacing = 8.0;
    let text_height = ui.fonts_mut(|f| f.row_height(&egui::TextStyle::Heading.resolve(ui.style())));
    let total_height = fixed_size.y + spacing + text_height;

    let top = drag_rect.center().y - total_height / 2.0;
    let image_rect = egui::Rect::from_min_size(
        egui::Pos2::new(drag_rect.center().x - fixed_size.x / 2.0, top),
        fixed_size,
    );

    if let Some(texture) = &app.install_image {
        ui.painter().image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    let text_pos = egui::Pos2::new(drag_rect.center().x, top + fixed_size.y + spacing);
    ui.painter().text(
        text_pos,
        egui::Align2::CENTER_TOP,
        "Drag & Drop IPA Here",
        egui::TextStyle::Heading.resolve(ui.style()),
        ui.visuals().weak_text_color(),
    );

    ui.ctx().input(|i| {
        for file in &i.raw.dropped_files {
            if let Some(path) = &file.path {
                if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("ipa" | "tipa")
                ) {
                    app.handle_message(AppMessage::PackageSelected(path.display().to_string()));
                }
            }
        }
    });
}

fn ui_package_settings(ui: &mut egui::Ui, app: &mut ImpactorApp, pkg: &Package) {
    let mut cancel_clicked = false;
    let mut install_clicked = false;

    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Name:");
                let pkg_name = pkg.get_name().unwrap_or_default();
                let mut name = app
                    .selected_settings
                    .custom_name
                    .clone()
                    .unwrap_or_else(|| pkg_name.clone());

                if ui.text_edit_singleline(&mut name).changed() {
                    if name != pkg_name {
                        app.selected_settings.custom_name = Some(name);
                    } else {
                        app.selected_settings.custom_name = None;
                    }
                }

                ui.label("Identifier:");
                let pkg_id = pkg.get_bundle_identifier().unwrap_or_default();
                let mut id = app
                    .selected_settings
                    .custom_identifier
                    .clone()
                    .unwrap_or_else(|| pkg_id.clone());

                if ui.text_edit_singleline(&mut id).changed() {
                    if id != pkg_id {
                        app.selected_settings.custom_identifier = Some(id);
                    } else {
                        app.selected_settings.custom_identifier = None;
                    }
                }

                ui.label("Version:");
                let pkg_ver = pkg.get_version().unwrap_or_default();
                let mut ver = app
                    .selected_settings
                    .custom_version
                    .clone()
                    .unwrap_or_else(|| pkg_ver.clone());

                if ui.text_edit_singleline(&mut ver).changed() {
                    if ver != pkg_ver {
                        app.selected_settings.custom_version = Some(ver);
                    } else {
                        app.selected_settings.custom_version = None;
                    }
                }

                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label("Tweaks:");

                    if let Some(tweaks) = &mut app.selected_settings.tweaks {
                        let mut remove_indices = Vec::new();

                        egui::Grid::new("tweaks_grid")
                            .striped(true)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                for (i, tweak_path) in tweaks.iter().enumerate() {
                                    ui.label(tweak_path.display().to_string());

                                    if ui.button("Remove").clicked() {
                                        remove_indices.push(i);
                                    }

                                    ui.end_row();
                                }
                            });

                        for &i in remove_indices.iter().rev() {
                            tweaks.remove(i);
                        }
                    }

                    egui::Grid::new("tweaks_actions")
                        .spacing([8.0, 0.0])
                        .show(ui, |ui| {
                            if ui.button("Add Tweak").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(
                                        "Tweak files",
                                        &["deb", "dylib", "framework", "bundle", "appex"],
                                    )
                                    .pick_file()
                                {
                                    match &mut app.selected_settings.tweaks {
                                        Some(vec) => vec.push(path),
                                        None => app.selected_settings.tweaks = Some(vec![path]),
                                    }
                                }
                            }

                            if ui.button("Add Bundle").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                        if ["framework", "bundle", "appex"].contains(&ext) {
                                            match &mut app.selected_settings.tweaks {
                                                Some(vec) => vec.push(path),
                                                None => {
                                                    app.selected_settings.tweaks = Some(vec![path])
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            ui.end_row();
                        });
                });
            });

            ui.vertical(|ui| {
                ui.label("General:");
                ui.checkbox(
                    &mut app.selected_settings.features.support_minimum_os_version,
                    "Try to support older versions (7+)",
                );
                ui.checkbox(
                    &mut app.selected_settings.features.support_file_sharing,
                    "Force File Sharing",
                );
                ui.checkbox(
                    &mut app.selected_settings.features.support_ipad_fullscreen,
                    "Force iPad Fullscreen",
                );
                ui.checkbox(
                    &mut app.selected_settings.features.support_game_mode,
                    "Force Game Mode",
                );
                ui.checkbox(
                    &mut app.selected_settings.features.support_pro_motion,
                    "Force Pro Motion",
                );

                ui.add_space(8.0);
                ui.label("Advanced:");
                ui.checkbox(
                    &mut app.selected_settings.embedding.single_profile,
                    "Only Register Main Bundle",
                );
                ui.checkbox(
                    &mut app.selected_settings.features.support_liquid_glass,
                    "Force Liquid Glass (26+)",
                );

                egui::Grid::new("settings_dropdowns")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Mode:");
                        egui::ComboBox::from_id_salt("install_mode")
                            .selected_text(match app.selected_settings.install_mode {
                                SignerInstallMode::Install => "Install",
                                SignerInstallMode::Export => "Export",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.selected_settings.install_mode,
                                    SignerInstallMode::Install,
                                    "Install",
                                );
                                ui.selectable_value(
                                    &mut app.selected_settings.install_mode,
                                    SignerInstallMode::Export,
                                    "Export",
                                );
                            });
                        ui.end_row();

                        ui.label("Signing:");
                        egui::ComboBox::from_id_salt("signing_mode")
                            .selected_text(match app.selected_settings.mode {
                                SignerMode::Pem => "Apple ID",
                                SignerMode::Adhoc => "Adhoc",
                                SignerMode::None => "Modify",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut app.selected_settings.mode,
                                    SignerMode::Pem,
                                    "Apple ID",
                                );
                                ui.selectable_value(
                                    &mut app.selected_settings.mode,
                                    SignerMode::Adhoc,
                                    "Adhoc",
                                );
                                ui.selectable_value(
                                    &mut app.selected_settings.mode,
                                    SignerMode::None,
                                    "Modify",
                                );
                            });
                        ui.end_row();
                    });
            });
        });

        ui.add_space(ui.available_size().y - 18.0);

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (button_enabled, button_label) = match app.selected_settings.install_mode {
                    SignerInstallMode::Export => (true, "Export"),
                    SignerInstallMode::Install => (app.selected_device.is_some(), "Install"),
                };

                let button = ui.add_enabled(button_enabled, egui::Button::new(button_label));
                install_clicked |= button.clicked();
                cancel_clicked |= ui.button("Cancel").clicked();
            });
        });
    });

    if cancel_clicked {
        app.handle_message(AppMessage::PackageDeselected);
    }
    if install_clicked {
        app.handle_message(AppMessage::WorkStarted);

        let Some(package) = app.selected_package.clone() else {
            app.handle_message(AppMessage::Error("No package selected".to_string()));
            return;
        };

        let (tx, rx) = mpsc::unbounded_channel::<AppMessage>();
        app.receiver = Some(rx);

        let selected_device = app
            .devices
            .iter()
            .find(|d| Some(d.device_id) == app.selected_device);

        spawn_package_handler(
            selected_device.cloned(),
            package,
            app.store
                .as_ref()
                .and_then(|s| s.selected_account().cloned()),
            app.selected_settings.clone(),
            move |status: String, progress: i32| {
                let _ = tx.send(AppMessage::WorkProgress(status, progress));
            },
        );
    }
}

fn ui_package_work(ui: &mut egui::Ui, app: &mut ImpactorApp) {
    ui.label("Preparing application before installation/or export, this will take a moment. Do not disconnect the device until finished.");
    ui.add_space(12.0);
    ui.label(format!("{}", app.working_status.0));
    ui.add_space(6.0);

    let progress = (app.working_status.1 as f32 / 100.0).clamp(0.0, 1.0);
    ui.add(
        egui::ProgressBar::new(progress)
            .show_percentage()
            .desired_height(18.0)
            .desired_width(ui.available_width() - 12.0),
    );
    ui.add_space(12.0);

    // Bottom-right "Back" button without manual add_space hacks
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
        ui.horizontal(|ui| {
            let button = ui.add_enabled(app.is_work_button_enabled, egui::Button::new("Back"));
            if button.clicked() {
                app.handle_message(AppMessage::WorkFinished);
            }
        });
    });
}

fn ui_settings(ui: &mut egui::Ui, app: &mut ImpactorApp) {
    if let Some(store) = &app.store {
        ui.heading("Accounts");
        ui.separator();

        let accounts: Vec<_> = store.accounts().values().cloned().collect();
        let selected_email = store.selected_account().map(|a| a.email().clone());

        let mut select_index: Option<usize> = None;
        let mut remove_indices: Vec<usize> = Vec::new();

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                egui::Grid::new("accounts_grid")
                    .striped(true)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.strong("Email");
                        ui.strong("Name");
                        ui.strong("");
                        ui.end_row();

                        for (index, account) in accounts.iter().enumerate() {
                            let is_selected = selected_email.as_deref() == Some(account.email());

                            if ui.selectable_label(is_selected, account.email()).clicked() {
                                select_index = Some(index);
                            }

                            ui.label(account.first_name());

                            if ui.button("Remove").clicked() {
                                remove_indices.push(index);
                            }

                            ui.end_row();
                        }
                    });
            });

        // Process selection and removals after UI borrow ends
        if let Some(index) = select_index {
            app.handle_message(AppMessage::AccountSelected(index));
        }
        for index in remove_indices {
            app.handle_message(AppMessage::AccountRemoved(index));
        }

        ui.add_space(8.0);

        if ui.button("Add Account").clicked() {
            app.login_ui.open();
        }

        ui.add_space(8.0);
    }

    if let Some(store) = &app.store {
        ui.heading("Misc");
        ui.separator();

        if let Some(account) = store.selected_account() {
            if ui.button("Export Certificate").clicked() {
                spawn_certificate_export_handler(account.clone());
            }
        }
    }

    ui.add_space(ui.available_size().y - 18.0);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                app.show_settings = false;
            }
        });
    });
}

// -----------------------------------------------------------------------------
// Messages
// -----------------------------------------------------------------------------

pub(crate) enum AppMessage {
    DeviceConnected(Device),
    DeviceDisconnected(u32),
    Error(String),
    PackageSelected(String),
    PackageDeselected,
    WorkStarted,
    WorkProgress(String, i32),
    WorkFinished,
    AccountStoreInitialized(AccountStore),
    AccountAdded(GsaAccount),
    AccountRemoved(usize),
    AccountSelected(usize),
    LoginNeedsTwoFactor(std_mpsc::Sender<Result<String, String>>),
    LoginFailed(String),
}

impl ImpactorApp {
    fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::DeviceConnected(device) => {
                if !self.devices.iter().any(|d| d.device_id == device.device_id) {
                    self.devices.push(device);
                    self.tray_menu_dirty = true;

                    if self.selected_device.is_none() {
                        if let Some(first) = self.devices.first() {
                            self.selected_device = Some(first.device_id);
                        }
                    }
                }
            }
            AppMessage::DeviceDisconnected(id) => {
                self.devices.retain(|d| d.device_id != id);
                self.tray_menu_dirty = true;
            }
            AppMessage::PackageSelected(path) => {
                let path = std::path::PathBuf::from(path);

                if let Ok(pkg) = Package::new(path) {
                    pkg.load_into_signer_options(&mut self.selected_settings);
                    self.selected_settings.mode = SignerMode::Pem;
                    self.selected_settings.install_mode = SignerInstallMode::Install;
                    self.selected_package = Some(pkg);
                } else {
                    self.handle_message(AppMessage::Error(
                        "Failed to load package. Is it a valid IPA or TIPA file?".into(),
                    ));
                }
            }
            AppMessage::PackageDeselected => {
                if let Some(pkg) = self.selected_package.as_ref() {
                    pkg.clone().remove_package_stage();
                }
                self.selected_package = None;
                self.selected_settings = SignerOptions::default();
            }
            AppMessage::WorkStarted => {
                self.is_working = true;
            }
            AppMessage::WorkProgress(status, progress) => {
                println!("{} - {}%", status, progress);
                self.working_status = (status, progress);

                if progress == 100 || progress == 1 {
                    self.is_work_button_enabled = true;
                }
            }
            AppMessage::WorkFinished => {
                self.is_working = false;
                self.is_work_button_enabled = false;
                self.working_status = ("Idle".to_string(), 0);
                self.handle_message(AppMessage::PackageDeselected);
            }
            AppMessage::Error(err) => eprintln!("{err}"),
            AppMessage::AccountStoreInitialized(store) => {
                self.store = Some(store);
            }
            AppMessage::AccountAdded(account) => {
                let Some(store) = &mut self.store else {
                    return;
                };

                _ = store.accounts_add_sync(account);
                self.login_ui.success();
            }
            AppMessage::AccountRemoved(index) => {
                let Some(store) = &mut self.store else {
                    return;
                };

                let accounts: Vec<_> = store.accounts().keys().cloned().collect();
                if let Some(email) = accounts.get(index).cloned() {
                    _ = store.accounts_remove_sync(&email);
                }
            }
            AppMessage::AccountSelected(index) => {
                let Some(store) = &mut self.store else {
                    return;
                };

                let accounts: Vec<_> = store.accounts().keys().cloned().collect();
                if let Some(email) = accounts.get(index).cloned() {
                    _ = store.account_select_sync(&&email);
                }
            }
            AppMessage::LoginNeedsTwoFactor(tx) => {
                self.login_ui.request_two_factor(tx);
            }
            AppMessage::LoginFailed(error) => {
                self.login_ui.fail(error);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Images
// -----------------------------------------------------------------------------

fn load_embedded_install_image() -> Result<ColorImage, String> {
    const INSTALL_PNG: &[u8] = include_bytes!("./install.png");
    let image = image::load_from_memory(INSTALL_PNG).map_err(|e| e.to_string())?;
    let size = [image.width() as usize, image.height() as usize];
    let image = image.to_rgba8();
    Ok(ColorImage::from_rgba_unmultiplied(size, &image))
}
