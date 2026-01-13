use iced::widget::{button, column, container, scrollable, text};
use iced::{Alignment, Element, Length, Task, window};
use iced_aw::SelectionList;

use crate::appearance;

#[derive(Debug, Clone)]
pub enum Message {
    SelectTeam(usize),
    Confirm,
    Cancel,
}

pub struct TeamSelectionWindow {
    teams: Vec<String>,
    pub selected_index: Option<usize>,
}

impl TeamSelectionWindow {
    pub fn settings() -> window::Settings {
        window::Settings {
            size: iced::Size::new(500.0, 400.0),
            position: window::Position::Centered,
            resizable: false,
            decorations: true,
            ..Default::default()
        }
    }

    pub fn new(teams: Vec<String>) -> Self {
        Self {
            teams,
            selected_index: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTeam(index) => {
                self.selected_index = Some(index);
                Task::none()
            }
            Message::Confirm | Message::Cancel => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = text("Select Developer Team")
            .size(24)
            .width(Length::Fill)
            .align_x(Alignment::Center);

        let description = text("Multiple developer teams are available. Please select one:")
            .size(14)
            .width(Length::Fill);

        let team_labels: &'static [String] = Box::leak(self.teams.clone().into_boxed_slice());

        let selection_list = SelectionList::new_with(
            team_labels,
            |index, _| Message::SelectTeam(index),
            appearance::THEME_FONT_SIZE.into(),
            5.0,
            iced_aw::style::selection_list::primary,
            self.selected_index,
            appearance::p_font(),
        );

        let list_container = container(scrollable(selection_list))
            .height(Length::Fill)
            .style(|theme: &iced::Theme| container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: theme.palette().background.scale_alpha(0.5),
                    radius: appearance::THEME_CORNER_RADIUS.into(),
                },
                ..Default::default()
            });

        let buttons = iced::widget::row![
            button(text("Cancel").align_x(Alignment::Center))
                .on_press(Message::Cancel)
                .style(appearance::s_button)
                .width(Length::Fill),
            button(text("Confirm").align_x(Alignment::Center))
                .on_press_maybe(self.selected_index.map(|_| Message::Confirm))
                .style(appearance::p_button)
                .width(Length::Fill),
        ]
        .spacing(appearance::THEME_PADDING);

        container(
            column![title, description, list_container, buttons]
                .spacing(appearance::THEME_PADDING)
                .padding(appearance::THEME_PADDING),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
