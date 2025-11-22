pub mod auth;
pub mod developer;
mod utils;

use plist::Dictionary;
use serde_json::Value;

use crate::auth::account::request::RequestType;

pub use apple_codesign::{SigningSettings, SettingsScope, UnifiedSigner, AppleCodesignError};
pub use omnisette::AnisetteConfiguration;

pub use utils::MachO;
pub use utils::MobileProvision;
pub use utils::CertificateIdentity;

trait SessionRequestTrait {
    async fn qh_send_request(&self, endpoint: &str, payload: Option<Dictionary>) -> Result<Dictionary, Error>;
    async fn v1_send_request(&self, url: &str, body: Option<Value>, request_type: Option<RequestType>) -> Result<Value, Error>;
}

use thiserror::Error as ThisError;
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Executable not found")]
    BundleExecutableMissing,

    #[error("Entitlements not found")]
    ProvisioningEntitlementsUnknown,
    
    #[error("Missing certificate PEM data")]
    CertificatePemMissing,
    #[error("Certificate error: {0}")]
    Certificate(String),
    
    #[error("Developer session error {0}: {1}")]
    DeveloperSession(i64, String),
    #[error("Request to developer session failed")]
    DeveloperSessionRequestFailed,
    
    #[error("Authentication SRP error {0}: {1}")]
    AuthSrpWithMessage(i64, String),
    #[error("Authentication SRP error")]
    AuthSrp,
    #[error("Authentication extra step required: {0}")]
    ExtraStep(String),
    #[error("Bad 2FA code")]
    Bad2faCode,
    #[error("Failed to parse")]
    Parse,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("Codesign error: {0}")]
    Codesign(#[from] apple_codesign::AppleCodesignError),
    #[error("Certificate PEM error: {0}")]
    Pem(#[from] pem::PemError),
    #[error("X509 certificate error: {0}")]
    X509(#[from] x509_certificate::X509CertificateError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Anisette error: {0}")]
    Anisette(#[from] omnisette::AnisetteError),
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("RSA error: {0}")]
    Rsa(#[from] rsa::Error),
    #[error("PKCS1 RSA error: {0}")]
    PKCS1(#[from] rsa::pkcs1::Error),
    #[error("PKCS8 RSA error: {0}")]
    PKCS8(#[from] rsa::pkcs8::Error),
    #[error("RCGen error: {0}")]
    RcGen(#[from] rcgen::RcgenError),
}
