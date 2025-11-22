use utils::{PlistInfoTrait, SignerOptions, Package};
use wxdragon::prelude::*;

#[derive(Clone)]
pub struct InstallPage {
    pub panel: Panel,
    pub cancel_button: Button,
    pub install_button: Button,
    
    custom_name_textfield: TextCtrl,
    custom_identifier_textfield: TextCtrl,
    custom_version_textfield: TextCtrl,
    support_older_versions_checkbox: CheckBox,
    support_file_sharing_checkbox: CheckBox,
    ipad_fullscreen_checkbox: CheckBox,
    game_mode_checkbox: CheckBox,
    pro_motion_checkbox: CheckBox,
    skip_registering_extensions_checkbox: CheckBox,
    
    original_name: Option<String>,
    original_identifier: Option<String>,
    original_version: Option<String>,
}

pub fn create_install_page(frame: &Frame) -> InstallPage {
    let panel = Panel::builder(frame).build();

    let main_sizer = BoxSizer::builder(Orientation::Vertical).build();

    let settings_sizer = BoxSizer::builder(Orientation::Horizontal).build();

    let textfields_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let bundle_name_label = StaticText::builder(&panel)
        .with_label("Name:")
        .build();
    let custom_name_textfield = TextCtrl::builder(&panel)
        .with_value("")
        .build();
    let bundle_identifier_label = StaticText::builder(&panel)
        .with_label("Identifier:")
        .build();
    let custom_identifier_textfield = TextCtrl::builder(&panel)
        .with_value("")
        .build();
    let bundle_version_label = StaticText::builder(&panel)
        .with_label("Version:")
        .build();
    let custom_version_textfield = TextCtrl::builder(&panel)
        .with_value("")
        .build();
    textfields_sizer.add(&bundle_name_label, 0, SizerFlag::Bottom, 6);
    textfields_sizer.add(&custom_name_textfield, 0, SizerFlag::Expand | SizerFlag::Left, 8);
    textfields_sizer.add(&bundle_identifier_label, 0, SizerFlag::Top | SizerFlag::Bottom, 6);
    textfields_sizer.add(&custom_identifier_textfield, 0, SizerFlag::Expand | SizerFlag::Left, 8);
    textfields_sizer.add(&bundle_version_label, 0, SizerFlag::Top | SizerFlag::Bottom, 6);
    textfields_sizer.add(&custom_version_textfield, 0, SizerFlag::Expand | SizerFlag::Left, 8);

    let checkbox_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let general_label = StaticText::builder(&panel)
        .with_label("General:")
        .build();
    let support_older_versions_checkbox = CheckBox::builder(&panel)
        .with_label("Try to support older versions (7+)")
        .build();
    let support_file_sharing_checkbox = CheckBox::builder(&panel)
        .with_label("Force File Sharing")
        .build();
    let ipad_fullscreen_checkbox = CheckBox::builder(&panel)
        .with_label("Force iPad Fullscreen")
        .build();
    let game_mode_checkbox = CheckBox::builder(&panel)
        .with_label("Force Game Mode")
        .build();
    let pro_motion_checkbox = CheckBox::builder(&panel)
        .with_label("Force Pro Motion")
        .build();
    let advanced_label = StaticText::builder(&panel)
        .with_label("Advanced:")
        .build();
    let skip_registering_extensions_checkbox = CheckBox::builder(&panel)
        .with_label("Only Register Main Bundle")
        .build();
    checkbox_sizer.add(&general_label, 0, SizerFlag::Bottom, 6);
    checkbox_sizer.add(&support_older_versions_checkbox, 0, SizerFlag::Expand | SizerFlag::Left, 8);
    checkbox_sizer.add(&support_file_sharing_checkbox, 0, SizerFlag::Expand | SizerFlag::Top | SizerFlag::Left, 8);
    checkbox_sizer.add(&ipad_fullscreen_checkbox, 0, SizerFlag::Expand | SizerFlag::Top | SizerFlag::Left, 8);
    checkbox_sizer.add(&game_mode_checkbox, 0, SizerFlag::Expand | SizerFlag::Top | SizerFlag::Left, 8);
    checkbox_sizer.add(&pro_motion_checkbox, 0, SizerFlag::Expand | SizerFlag::Top | SizerFlag::Left | SizerFlag::Bottom, 8);
    checkbox_sizer.add(&advanced_label, 0, SizerFlag::Top | SizerFlag::Bottom, 6);
    checkbox_sizer.add(&skip_registering_extensions_checkbox, 0, SizerFlag::Expand | SizerFlag::Left, 8);

    settings_sizer.add_sizer(&textfields_sizer, 1, SizerFlag::Expand | SizerFlag::Right, 13);
    settings_sizer.add_sizer(&checkbox_sizer, 1, SizerFlag::Expand, 13);

    main_sizer.add_sizer(&settings_sizer, 0, SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right, 13);

    main_sizer.add_stretch_spacer(1);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();

    let cancel_button = Button::builder(&panel)
        .with_label("Cancel")
        .build();
    let install_button = Button::builder(&panel)
        .with_label("Install")
        .build();
    install_button.enable(false);

    button_sizer.add_stretch_spacer(1);
    button_sizer.add(&cancel_button, 0, SizerFlag::Right, 13);
    button_sizer.add(&install_button, 0, SizerFlag::All, 0);

    main_sizer.add_sizer(
        &button_sizer,
        0,
        SizerFlag::Right | SizerFlag::Bottom | SizerFlag::Expand,
        13,
    );

    panel.set_sizer(main_sizer, true);

    InstallPage {
        panel,
        cancel_button,
        install_button,
        
        custom_name_textfield,
        custom_identifier_textfield,
        custom_version_textfield,
        support_older_versions_checkbox,
        support_file_sharing_checkbox,
        ipad_fullscreen_checkbox,
        game_mode_checkbox,
        pro_motion_checkbox,
        skip_registering_extensions_checkbox,
        
        original_name: None,
        original_identifier: None,
        original_version: None,
    }
}


impl InstallPage {
    pub fn set_settings(&mut self, settings: &SignerOptions, package: Option<&Package>) {
        self.support_older_versions_checkbox.set_value(settings.features.support_minimum_os_version);
        self.support_file_sharing_checkbox.set_value(settings.features.support_file_sharing);
        self.ipad_fullscreen_checkbox.set_value(settings.features.support_ipad_fullscreen);
        self.game_mode_checkbox.set_value(settings.features.support_game_mode);
        self.pro_motion_checkbox.set_value(settings.features.support_pro_motion);
        self.skip_registering_extensions_checkbox.set_value(settings.embedding.single_profile);
        
        if let Some(package) = package {
            if let Some(ref name) = package.get_name() {
                self.custom_name_textfield.set_value(name);
                self.original_name = Some(name.clone());
            } else {
                self.custom_name_textfield.set_value("");
                self.original_name = None;
            }
            
            if let Some(ref identifier) = package.get_bundle_identifier() {
                self.custom_identifier_textfield.set_value(identifier);
                self.original_identifier = Some(identifier.clone());
            } else {
                self.custom_identifier_textfield.set_value("");
                self.original_identifier = None;
            }
            
            if let Some(ref version) = package.get_version() {
                self.custom_version_textfield.set_value(version);
                self.original_version = Some(version.clone());
            } else {
                self.custom_version_textfield.set_value("");
                self.original_version = None;
            }
        } else {
            self.custom_name_textfield.set_value("");
            self.custom_identifier_textfield.set_value("");
            self.custom_version_textfield.set_value("");
            self.original_name = None;
            self.original_identifier = None;
            self.original_version = None;
        }
    }
    
    pub fn update_fields(&self, settings: &mut SignerOptions) {
        settings.features.support_minimum_os_version = self.support_older_versions_checkbox.get_value();
        settings.features.support_file_sharing = self.support_file_sharing_checkbox.get_value();
        settings.features.support_ipad_fullscreen = self.ipad_fullscreen_checkbox.get_value();
        settings.features.support_game_mode = self.game_mode_checkbox.get_value();
        settings.features.support_pro_motion = self.pro_motion_checkbox.get_value();
        settings.embedding.single_profile = self.skip_registering_extensions_checkbox.get_value();

        if let Some(ref original_name) = self.original_name {
            let current_name = self.custom_name_textfield.get_value();
            if &current_name != original_name {
                settings.custom_name = Some(current_name.to_string());
            }
        }

        if let Some(ref original_identifier) = self.original_identifier {
            let current_identifier = self.custom_identifier_textfield.get_value();
            if &current_identifier != original_identifier {
                settings.custom_identifier = Some(current_identifier.to_string());
            }
        }

        if let Some(ref original_version) = self.original_version {
            let current_version = self.custom_version_textfield.get_value();
            if &current_version != original_version {
                settings.custom_version = Some(current_version.to_string());
            }
        }
    }
}

impl InstallPage {
    pub fn set_cancel_handler(&self, on_cancel: impl Fn() + 'static) {
        self.cancel_button.on_click(move |_evt| {
            on_cancel();
        });
    }

    pub fn set_install_handler(&self, on_install: impl Fn() + 'static) {
        self.install_button.on_click(move |_evt| {
            on_install();
        });
    }
}
