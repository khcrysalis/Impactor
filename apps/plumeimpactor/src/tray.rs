use std::collections::HashMap;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu},
};

pub(crate) fn build_tray_icon(menu: &Menu) -> TrayIcon {
    let icon = load_icon();
    TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip(crate::APP_NAME)
        .with_icon(icon)
        .build()
        .expect("Failed to build tray icon")
}

fn load_icon() -> Icon {
    let bytes = include_bytes!("./tray.png");
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon bytes")
        .to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

#[derive(Debug, Clone)]
pub enum TrayAction {
    Show,
    Quit,
    RefreshApp { udid: String, app_path: String },
    ForgetApp { udid: String, app_path: String },
}

pub(crate) struct ImpactorTray {
    icon: Option<TrayIcon>,
    menu: Menu,
    show_item_id: MenuId,
    quit_item_id: MenuId,
    action_map: HashMap<MenuId, TrayAction>,
}

impl ImpactorTray {
    pub(crate) fn new() -> Self {
        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Open", true, None);
        let quit_item = MenuItem::new(format!("Quit {}", crate::APP_NAME), true, None);

        let show_item_id = show_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        let mut action_map = HashMap::new();
        action_map.insert(show_item_id.clone(), TrayAction::Show);
        action_map.insert(quit_item_id.clone(), TrayAction::Quit);

        let _ = tray_menu.append_items(&[&show_item, &PredefinedMenuItem::separator(), &quit_item]);

        Self {
            icon: Some(build_tray_icon(&tray_menu)),
            menu: tray_menu,
            show_item_id,
            quit_item_id,
            action_map,
        }
    }

    pub(crate) fn update_refresh_apps(&mut self, store: &plume_store::AccountStore) {
        log::info!(
            "Updating tray menu with {} refresh devices",
            store.refreshes().len()
        );

        let new_menu = Menu::new();
        let show_item = MenuItem::new("Open", true, None);

        let mut action_map = HashMap::new();
        action_map.insert(show_item.id().clone(), TrayAction::Show);

        let _ = new_menu.append(&show_item);
        let _ = new_menu.append(&PredefinedMenuItem::separator());

        if !store.refreshes().is_empty() {
            let refresh_submenu = Submenu::new("Auto-Refresh Apps", true);

            for (udid, refresh_device) in store.refreshes() {
                let device_name = if refresh_device.is_mac {
                    "This Mac".to_string()
                } else {
                    format!("Device {}", &udid[..8.min(udid.len())])
                };

                if !refresh_device.apps.is_empty() {
                    let device_submenu = Submenu::new(&device_name, true);

                    for app in &refresh_device.apps {
                        let app_name = app
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown App");

                        let scheduled = app.scheduled_refresh.format("%Y-%m-%d %H:%M").to_string();

                        let app_submenu =
                            Submenu::new(&format!("{} ({})", app_name, scheduled), true);

                        let refresh_item = MenuItem::new("Refresh Now", true, None);
                        let forget_item = MenuItem::new("Forget App", true, None);

                        let refresh_id = refresh_item.id().clone();
                        let forget_id = forget_item.id().clone();

                        action_map.insert(
                            refresh_id,
                            TrayAction::RefreshApp {
                                udid: udid.clone(),
                                app_path: app.path.to_string_lossy().to_string(),
                            },
                        );
                        action_map.insert(
                            forget_id,
                            TrayAction::ForgetApp {
                                udid: udid.clone(),
                                app_path: app.path.to_string_lossy().to_string(),
                            },
                        );

                        let _ = app_submenu.append(&refresh_item);
                        let _ = app_submenu.append(&forget_item);
                        let _ = device_submenu.append(&app_submenu);
                    }

                    let _ = refresh_submenu.append(&device_submenu);
                }
            }

            let _ = new_menu.append(&refresh_submenu);
            let _ = new_menu.append(&PredefinedMenuItem::separator());
        }

        let quit_item = MenuItem::new(format!("Quit {}", crate::APP_NAME), true, None);
        action_map.insert(quit_item.id().clone(), TrayAction::Quit);
        let _ = new_menu.append(&quit_item);

        self.show_item_id = show_item.id().clone();
        self.quit_item_id = quit_item.id().clone();

        log::info!("Rebuilding tray icon with new menu");

        self.menu = new_menu;
        self.action_map = action_map;
        if let Some(tray_icon) = &mut self.icon {
            let _ = tray_icon.set_menu(Some(Box::new(self.menu.clone())));
        }
    }

    pub(crate) fn get_action(&self, id: &MenuId) -> Option<&TrayAction> {
        self.action_map.get(id)
    }
}
