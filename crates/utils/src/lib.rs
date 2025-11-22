mod options;
mod package;
mod bundle;
mod device;
mod signer;

pub use options::{
    SignerOptions, // Main
    SignerFeatures, // Feature support options
    SignerEmbedding, // Embedding options
    SignerMode, // Signing mode
    SignerApp // Supported app types
};
pub use package::Package; // Package helper
pub use bundle::{Bundle, BundleType}; // Bundle helper
pub use device::Device; // Device helper
pub use signer::Signer; // Signer

use thiserror::Error as ThisError;
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Info.plist not found")]
    BundleInfoPlistMissing,
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Info.plist not found")]
    PackageInfoPlistMissing,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("GrandSlam error: {0}")]
    GrandSlam(#[from] grand_slam::Error),
    #[error("Idevice error: {0}")]
    Idevice(#[from] idevice::IdeviceError),
    #[error("Codesign error: {0}")]
    Codesign(#[from] grand_slam::AppleCodesignError),
    #[error("Other error: {0}")]
    Other(String),
}

pub trait PlistInfoTrait {
    fn get_name(&self) -> Option<String>;
    fn get_executable(&self) -> Option<String>;
    fn get_bundle_identifier(&self) -> Option<String>;
    fn get_version(&self) -> Option<String>;
    fn get_build_version(&self) -> Option<String>;
}
