use tray_icon::{
    TrayIcon,
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
};

pub(crate) fn build_tray_icon(menu: &Menu) -> TrayIcon {
    let icon = load_icon();
    tray_icon::TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip(crate::APP_NAME)
        .with_icon(icon)
        .build()
        .expect("Failed to build tray icon")
}

fn load_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("./tray.png");
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon bytes")
        .to_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

pub(crate) struct ImpactorTray {
    #[allow(dead_code)]
    icon: TrayIcon,
    show_item_id: MenuId,
    quit_item_id: MenuId,
}

impl ImpactorTray {
    pub(crate) fn new() -> Self {
        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Open", true, None);
        let quit_item = MenuItem::new(format!("Quit {}", crate::APP_NAME), true, None);

        let show_item_id = show_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        let _ = tray_menu.append_items(&[&show_item, &PredefinedMenuItem::separator(), &quit_item]);

        let icon = build_tray_icon(&tray_menu);

        Self {
            icon,
            show_item_id,
            quit_item_id,
        }
    }

    pub(crate) fn is_show_clicked(&self, id: &MenuId) -> bool {
        *id == self.show_item_id
    }

    pub(crate) fn is_quit_clicked(&self, id: &MenuId) -> bool {
        *id == self.quit_item_id
    }
}
