use crate::{ImpactorInstaller, Message};
use iced::Task;
use plume_utils::PlistInfoTrait;

pub fn handle_message(installer: &mut ImpactorInstaller, message: &Message) -> Task<Message> {
    match message {
        Message::UpdateCustomName(name) => {
            let pkg_name = installer
                .selected_package_file
                .as_ref()
                .and_then(|p| p.get_name())
                .unwrap_or_default();

            if name != &pkg_name {
                installer.package_options.custom_name = Some(name.clone());
            } else {
                installer.package_options.custom_name = None;
            }
            Task::none()
        }
        Message::UpdateCustomIdentifier(id) => {
            let pkg_id = installer
                .selected_package_file
                .as_ref()
                .and_then(|p| p.get_bundle_identifier())
                .unwrap_or_default();

            if id != &pkg_id {
                installer.package_options.custom_identifier = Some(id.clone());
            } else {
                installer.package_options.custom_identifier = None;
            }
            Task::none()
        }
        Message::UpdateCustomVersion(ver) => {
            let pkg_ver = installer
                .selected_package_file
                .as_ref()
                .and_then(|p| p.get_version())
                .unwrap_or_default();

            if ver != &pkg_ver {
                installer.package_options.custom_version = Some(ver.clone());
            } else {
                installer.package_options.custom_version = None;
            }
            Task::none()
        }
        Message::ToggleMinimumOsVersion(value) => {
            installer
                .package_options
                .features
                .support_minimum_os_version = *value;
            Task::none()
        }
        Message::ToggleFileSharing(value) => {
            installer.package_options.features.support_file_sharing = *value;
            Task::none()
        }
        Message::ToggleIpadFullscreen(value) => {
            installer.package_options.features.support_ipad_fullscreen = *value;
            Task::none()
        }
        Message::ToggleGameMode(value) => {
            installer.package_options.features.support_game_mode = *value;
            Task::none()
        }
        Message::ToggleProMotion(value) => {
            installer.package_options.features.support_pro_motion = *value;
            Task::none()
        }
        Message::ToggleSingleProfile(value) => {
            installer.package_options.embedding.single_profile = *value;
            Task::none()
        }
        Message::ToggleLiquidGlass(value) => {
            installer.package_options.features.support_liquid_glass = *value;
            Task::none()
        }
        Message::UpdateSignerMode(mode) => {
            installer.package_options.mode = *mode;
            Task::none()
        }
        Message::UpdateInstallMode(mode) => {
            installer.package_options.install_mode = *mode;
            Task::none()
        }
        Message::AddTweak => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .add_filter("Tweak files", &["deb", "dylib"])
                    .set_title("Select Tweak File")
                    .pick_file()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::AddTweakSelected,
        ),
        Message::AddTweakSelected(path) => {
            if let Some(path) = path {
                match &mut installer.package_options.tweaks {
                    Some(vec) => vec.push(path.clone()),
                    None => installer.package_options.tweaks = Some(vec![path.clone()]),
                }
            }
            Task::none()
        }
        Message::AddBundle => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Select Bundle Folder")
                    .pick_folder()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::AddBundleSelected,
        ),
        Message::AddBundleSelected(path) => {
            if let Some(path) = path {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ["framework", "bundle", "appex"].contains(&ext) {
                        match &mut installer.package_options.tweaks {
                            Some(vec) => vec.push(path.clone()),
                            None => installer.package_options.tweaks = Some(vec![path.clone()]),
                        }
                    }
                }
            }
            Task::none()
        }
        Message::RemoveTweak(index) => {
            if let Some(tweaks) = &mut installer.package_options.tweaks {
                if *index < tweaks.len() {
                    tweaks.remove(*index);
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
