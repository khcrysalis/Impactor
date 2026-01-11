use iced::widget::{column, container, image, text};
use iced::{Center, Color, Element};

use crate::Message;
use std::sync::OnceLock;

const INSTALL_IMAGE: &[u8] = include_bytes!("./install.png");

pub fn view(_is_hovering: bool) -> Element<'static, Message> {
    static INSTALL_IMAGE_HANDLE: OnceLock<image::Handle> = OnceLock::new();
    let image_handle =
        INSTALL_IMAGE_HANDLE.get_or_init(|| image::Handle::from_bytes(INSTALL_IMAGE));

    let content = column![
        image(image_handle.clone()).width(100),
        text("Drag & drop IPA / TIPA file")
            .size(16)
            .color(Color::from_rgba(0.5, 0.5, 0.5, 0.7))
    ]
    .padding(20)
    .spacing(20)
    .align_x(Center);

    container(content).into()
}
