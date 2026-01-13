use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Center, Element, Fill, Task};
use plume_store::AccountStore;

use crate::appearance;

#[derive(Debug, Clone)]
pub enum Message {
    ShowLogin,
    SelectAccount(usize),
    RemoveAccount(usize),
    ExportP12,
}

#[derive(Debug)]
pub struct SettingsScreen {
    pub account_store: Option<AccountStore>,
}

impl SettingsScreen {
    pub fn new(account_store: Option<AccountStore>) -> Self {
        Self { account_store }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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

                        let _ = rt.block_on(async move {
                            crate::subscriptions::export_certificate(account).await
                        });
                    });
                }
                Task::none()
            }
            Message::ShowLogin => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let Some(store) = &self.account_store else {
            return column![text("Loading accounts...")]
                .spacing(appearance::THEME_PADDING)
                .padding(appearance::THEME_PADDING)
                .into();
        };

        let mut accounts: Vec<_> = store.accounts().iter().collect();
        accounts.sort_by_key(|(email, _)| *email);

        let selected_index = store
            .selected_account()
            .and_then(|acc| accounts.iter().position(|(e, _)| *e == acc.email()));

        let mut content = column![].spacing(appearance::THEME_PADDING);

        if !accounts.is_empty() {
            content = content.push(self.view_account_list(&accounts, selected_index));
        } else {
            content = content.push(text("No accounts added yet"));
        }

        content = content.push(self.view_account_buttons(selected_index));
        content.into()
    }

    fn view_account_list(
        &self,
        accounts: &[(&String, &plume_store::GsaAccount)],
        selected_index: Option<usize>,
    ) -> Element<'_, Message> {
        let account_list = accounts.iter().enumerate().fold(
            column![].spacing(5.0),
            |content, (index, (_, account))| {
                let name = if !account.first_name().is_empty() {
                    format!("{} ({})", account.first_name(), account.email())
                } else {
                    account.email().to_string()
                };
                let marker = if Some(index) == selected_index {
                    " [✓] "
                } else {
                    " [ ] "
                };
                let style = if Some(index) == selected_index {
                    appearance::p_button
                } else {
                    appearance::s_button
                };

                content.push(
                    button(
                        text(format!("{}{}", marker, name))
                            .size(appearance::THEME_FONT_SIZE)
                            .align_x(Alignment::Start),
                    )
                        .on_press(Message::SelectAccount(index))
                        .style(style)
                        .width(Fill),
                )
            },
        );

        container(scrollable(account_list))
            .height(Fill)
            .style(|theme: &iced::Theme| container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: theme.palette().background.scale_alpha(0.5),
                    radius: appearance::THEME_CORNER_RADIUS.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_account_buttons(&self, selected_index: Option<usize>) -> Element<'_, Message> {
        let mut buttons = row![
            button(text("Add Account").align_x(Center))
                .on_press(Message::ShowLogin)
                .style(appearance::s_button)
        ]
        .spacing(appearance::THEME_PADDING);

        if let Some(index) = selected_index {
            buttons = buttons
                .push(
                    button(text("Remove Selected").align_x(Center))
                        .on_press(Message::RemoveAccount(index))
                        .style(appearance::s_button),
                )
                .push(
                    button(text("Export P12").align_x(Center))
                        .on_press(Message::ExportP12)
                        .style(appearance::s_button),
                );
        }

        buttons.align_y(Alignment::Center).into()
    }
}
