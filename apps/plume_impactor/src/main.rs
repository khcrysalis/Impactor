#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::widget::{button, column, container, text};
use iced::window;
use iced::{Center, Element, Fill, Subscription, Task};
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

pub const APP_NAME: &str = concat!("Impactor – Version ", env!("CARGO_PKG_VERSION"));

fn main() -> iced::Result {
    env_logger::init();
    let _ = rustls::crypto::ring::default_provider().install_default();

    iced::daemon(Counter::new, Counter::update, Counter::view)
        .subscription(Counter::subscription)
        .title(APP_NAME)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
    AsyncIncrement,
    AsyncOperationComplete(i64),
    TrayMenuClicked(tray_icon::menu::MenuId),
    TrayIconClicked,
    ShowWindow,
    HideWindow,
    WindowClosed(window::Id),
    Quit,
}

struct Counter {
    value: i64,
    loading: bool,
    status: String,
    tray_icon: Option<TrayIcon>,
    quit_item_id: tray_icon::menu::MenuId,
    show_item_id: tray_icon::menu::MenuId,
    hide_item_id: tray_icon::menu::MenuId,
    main_window: Option<window::Id>,
}

impl Counter {
    fn new() -> (Self, Task<Message>) {
        let tray_menu = Menu::new();
        let quit_item = MenuItem::new("Quit", true, None);
        let show_item = MenuItem::new("Show Window", true, None);
        let hide_item = MenuItem::new("Hide Window", true, None);

        let quit_item_id = quit_item.id().clone();
        let show_item_id = show_item.id().clone();
        let hide_item_id = hide_item.id().clone();

        let _ = tray_menu.append_items(&[
            &show_item,
            &hide_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ]);

        let tray_icon = build_tray_icon(&tray_menu);

        let (id, open_task) = window::open(window::Settings {
            size: iced::Size::new(540.0, 400.0),
            position: window::Position::Centered,
            exit_on_close_request: false,
            icon: Some(load_window_icon()),
            ..Default::default()
        });

        (
            Self {
                value: 0,
                loading: false,
                status: String::from("Ready"),
                tray_icon: Some(tray_icon),
                quit_item_id,
                show_item_id,
                hide_item_id,
                main_window: Some(id),
            },
            open_task.discard(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Increment => {
                self.value += 1;
                Task::none()
            }
            Message::Decrement => {
                self.value -= 1;
                Task::none()
            }
            Message::AsyncIncrement => {
                self.loading = true;
                Task::perform(
                    async move {
                        _ = iced::futures::future::ready(()).await;
                        10
                    },
                    Message::AsyncOperationComplete,
                )
            }
            Message::AsyncOperationComplete(val) => {
                self.value += val;
                self.loading = false;
                Task::none()
            }

            Message::TrayIconClicked => Task::done(Message::ShowWindow),

            Message::TrayMenuClicked(id) => {
                if id == self.quit_item_id {
                    Task::done(Message::Quit)
                } else if id == self.show_item_id {
                    Task::done(Message::ShowWindow)
                } else if id == self.hide_item_id {
                    Task::done(Message::HideWindow)
                } else {
                    Task::none()
                }
            }

            Message::ShowWindow => {
                if let Some(id) = self.main_window {
                    window::gain_focus(id)
                } else {
                    let (id, open_task) = window::open(window::Settings {
                        size: iced::Size::new(540.0, 400.0),
                        position: window::Position::Centered,
                        exit_on_close_request: false,
                        icon: Some(load_window_icon()),
                        ..Default::default()
                    });
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

            Message::WindowClosed(id) => {
                if self.main_window == Some(id) {
                    self.main_window = None;
                }
                Task::none()
            }

            Message::Quit => {
                self.tray_icon.take();
                std::process::exit(0);
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tray_subscription = Subscription::run(|| {
            iced::stream::channel(
                100,
                |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use iced::futures::{SinkExt, StreamExt};
                    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded::<Message>();

                    std::thread::spawn(move || {
                        let menu_channel = MenuEvent::receiver();
                        let tray_channel = TrayIconEvent::receiver();
                        loop {
                            if let Ok(event) = menu_channel.try_recv() {
                                let _ = tx.unbounded_send(Message::TrayMenuClicked(event.id));
                            }

                            if let Ok(event) = tray_channel.try_recv() {
                                match event {
                                    TrayIconEvent::DoubleClick {
                                        button: tray_icon::MouseButton::Left,
                                        ..
                                    } => {
                                        let _ = tx.unbounded_send(Message::TrayIconClicked);
                                    }
                                    _ => {}
                                }
                            }
                            std::thread::sleep(std::time::Duration::from_millis(32));
                        }
                    });

                    while let Some(message) = rx.next().await {
                        let _ = output.send(message).await;
                    }
                },
            )
        });

        let window_events = window::events().filter_map(|(_id, event)| match event {
            window::Event::CloseRequested => Some(Message::HideWindow),
            _ => None,
        });

        Subscription::batch(vec![tray_subscription, window_events])
    }

    fn view(&self, _window: window::Id) -> Element<'_, Message> {
        let content = column![
            text(format!("Count: {}", self.value)).size(50),
            button("Increment").on_press(Message::Increment).width(150),
            button("Decrement").on_press(Message::Decrement).width(150),
            button(if self.loading {
                "Loading..."
            } else {
                "Async +10"
            })
            .on_press_maybe((!self.loading).then_some(Message::AsyncIncrement))
            .width(150),
            button("Hide to Tray")
                .on_press(Message::HideWindow)
                .width(150),
        ]
        .padding(20)
        .spacing(15)
        .align_x(Center);

        container(content).center(Fill).into()
    }
}

fn build_tray_icon(menu: &Menu) -> TrayIcon {
    let icon = load_icon();
    TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip(APP_NAME)
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

fn load_window_icon() -> window::Icon {
    let bytes = include_bytes!(
        "../../../package/linux/icons/hicolor/64x64/apps/dev.khcrysalis.PlumeImpactor.png"
    );
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon bytes")
        .to_rgba8();
    let (width, height) = image.dimensions();
    window::icon::from_rgba(image.into_raw(), width, height).unwrap()
}
