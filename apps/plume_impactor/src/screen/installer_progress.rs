use iced::Element;
use iced::Length::Fill;
use iced::widget::{column, container, text};

use crate::Message;

pub fn view<'a>(status: &'a str, progress: i32) -> Element<'a, Message> {
    let progress_bar = iced::widget::progress_bar(0.0..=100.0, progress as f32);

    column![
        text(format!("{progress}% – {status}")).size(14),
        progress_bar,
        container(text("")).height(Fill),
    ]
    .padding(10)
    .spacing(16)
    .into()
}
