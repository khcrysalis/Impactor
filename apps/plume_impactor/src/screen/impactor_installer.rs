use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Center, Element, Fill, Length};
use plume_utils::{Package, PlistInfoTrait, SignerInstallMode, SignerMode, SignerOptions};

use crate::Message;

pub fn view<'a>(package: Option<&'a Package>, options: &'a SignerOptions) -> Element<'a, Message> {
    let Some(pkg) = package else {
        return column![
            text("No package selected").size(32),
            text("Go back and select a file").size(16),
        ]
        .padding(20)
        .spacing(20)
        .align_x(Center)
        .into();
    };

    let pkg_name = pkg.get_name().unwrap_or_default();
    let pkg_id = pkg.get_bundle_identifier().unwrap_or_default();
    let pkg_ver = pkg.get_version().unwrap_or_default();

    let name_value = options.custom_name.clone().unwrap_or(pkg_name.clone());
    let id_value = options.custom_identifier.clone().unwrap_or(pkg_id.clone());
    let ver_value = options.custom_version.clone().unwrap_or(pkg_ver.clone());

    let left_column = column![
        text("Name:").size(12),
        text_input("App name", &name_value)
            .on_input(Message::UpdateCustomName)
            .padding(8),
        text("Identifier:").size(12),
        text_input("Bundle identifier", &id_value)
            .on_input(Message::UpdateCustomIdentifier)
            .padding(8),
        text("Version:").size(12),
        text_input("Version", &ver_value)
            .on_input(Message::UpdateCustomVersion)
            .padding(8),
        text("Tweaks:").size(12),
        view_tweaks(options),
        row![
            button("Add Tweak").on_press(Message::AddTweak),
            button("Add Bundle").on_press(Message::AddBundle),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .width(Fill);

    let right_column = column![
        text("General:").size(12),
        checkbox(options.features.support_minimum_os_version)
            .label("Try to support older versions (7+)")
            .on_toggle(Message::ToggleMinimumOsVersion),
        checkbox(options.features.support_file_sharing)
            .label("Force File Sharing")
            .on_toggle(Message::ToggleFileSharing),
        checkbox(options.features.support_ipad_fullscreen)
            .label("Force iPad Fullscreen")
            .on_toggle(Message::ToggleIpadFullscreen),
        checkbox(options.features.support_game_mode)
            .label("Force Game Mode")
            .on_toggle(Message::ToggleGameMode),
        checkbox(options.features.support_pro_motion)
            .label("Force Pro Motion")
            .on_toggle(Message::ToggleProMotion),
        text("Advanced:").size(12),
        checkbox(options.embedding.single_profile)
            .label("Only Register Main Bundle")
            .on_toggle(Message::ToggleSingleProfile),
        checkbox(options.features.support_liquid_glass)
            .label("Force Liquid Glass (26+)")
            .on_toggle(Message::ToggleLiquidGlass),
        text("Mode:").size(12),
        pick_list(
            &[SignerInstallMode::Install, SignerInstallMode::Export][..],
            Some(options.install_mode),
            Message::UpdateInstallMode
        )
        .placeholder("Select mode"),
        text("Signing:").size(12),
        pick_list(
            &[SignerMode::Pem, SignerMode::Adhoc, SignerMode::None][..],
            Some(options.mode),
            Message::UpdateSignerMode
        )
        .placeholder("Select signing method"),
    ]
    .spacing(8)
    .width(Fill);

    let content =
        scrollable(column![row![left_column, right_column].spacing(20).padding(10),].spacing(16));

    container(content).width(Fill).height(Fill).into()
}

fn view_tweaks<'a>(options: &'a SignerOptions) -> Element<'a, Message> {
    let tweaks = options.tweaks.as_ref();

    if let Some(tweaks) = tweaks {
        if tweaks.is_empty() {
            return text("No tweaks added").size(12).into();
        }

        let mut tweak_list = column![].spacing(4);

        for (i, tweak) in tweaks.iter().enumerate() {
            let tweak_row = row![
                text(tweak.file_name().and_then(|n| n.to_str()).unwrap_or("???"))
                    .size(12)
                    .width(Fill),
                button("Remove").on_press(Message::RemoveTweak(i))
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            tweak_list = tweak_list.push(tweak_row);
        }

        scrollable(tweak_list).height(Length::Fixed(100.0)).into()
    } else {
        text("No tweaks added").size(12).into()
    }
}
