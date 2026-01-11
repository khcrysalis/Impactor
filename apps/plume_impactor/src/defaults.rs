pub(crate) fn default_settings() -> iced::Settings {
    iced::Settings {
        default_font: iced::Font {
            family: iced::font::Family::Monospace,
            weight: iced::font::Weight::Normal,
            stretch: iced::font::Stretch::Normal,
            style: iced::font::Style::Normal,
        },
        default_text_size: 12.into(),
        ..Default::default()
    }
}

pub(crate) fn default_window_settings() -> iced::window::Settings {
    iced::window::Settings {
        size: iced::Size::new(555.0, 300.0),
        position: iced::window::Position::Centered,
        exit_on_close_request: false,
        resizable: false,
        icon: Some(load_window_icon()),
        platform_specific: iced::window::settings::PlatformSpecific {
            titlebar_transparent: true,
            title_hidden: true,
            fullsize_content_view: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn load_window_icon() -> iced::window::Icon {
    let bytes = include_bytes!(
        "../../../package/linux/icons/hicolor/64x64/apps/dev.khcrysalis.PlumeImpactor.png"
    );
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon bytes")
        .to_rgba8();
    let (width, height) = image.dimensions();
    iced::window::icon::from_rgba(image.into_raw(), width, height).unwrap()
}

pub fn get_data_path() -> std::path::PathBuf {
    let base = if cfg!(windows) {
        std::env::var("APPDATA").unwrap()
    } else {
        std::env::var("HOME").unwrap() + "/.config"
    };

    let dir = std::path::Path::new(&base).join("PlumeImpactor");

    std::fs::create_dir_all(&dir).ok();

    dir
}
