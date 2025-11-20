mod certificate;
mod provision;
mod macho;
mod signer;
mod bundle;

pub use macho::MachO;
pub use provision::MobileProvision;
pub use certificate::CertificateIdentity;
pub use signer::Signer;
pub use bundle::Bundle;
pub use bundle::BundleType;

#[derive(Clone, Debug)]
pub struct SignerSettings {
    pub custom_name: Option<String>,
    pub custom_identifier: Option<String>,
    pub custom_version: Option<String>,

    pub support_minimum_os_version: bool,
    pub support_file_sharing: bool,
    pub support_ipad_fullscreen: bool,
    pub support_game_mode: bool,
    pub support_pro_motion: bool,
    pub should_embed_provisioning: bool,
    pub should_embed_pairing: bool,
    pub should_embed_p12: bool,
    pub should_only_use_main_provisioning: bool,
    pub remove_url_schemes: bool,
    pub export_ipa: bool,
}

impl Default for SignerSettings {
    fn default() -> Self {
        Self {
            custom_name: None,
            custom_identifier: None,
            custom_version: None,
            
            support_minimum_os_version: false,
            support_file_sharing: false,
            support_ipad_fullscreen: false,
            support_game_mode: false,
            support_pro_motion: false,
            should_embed_provisioning: true,
            should_embed_pairing: false,
            should_embed_p12: false,
            should_only_use_main_provisioning: false,
            remove_url_schemes: false,
            export_ipa: false,
        }
    }
}

pub trait PlistInfoTrait {
    fn get_name(&self) -> Option<String>;
    fn get_executable(&self) -> Option<String>;
    fn get_bundle_identifier(&self) -> Option<String>;
    fn get_version(&self) -> Option<String>;
    fn get_build_version(&self) -> Option<String>;
}
