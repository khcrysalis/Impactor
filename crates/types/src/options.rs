/// Settings for the signer process.
#[derive(Clone, Debug)]
pub struct SignerOptions {
    /// Custom app name override.
    pub custom_name: Option<String>,
    /// Custom bundle identifier override.
    pub custom_identifier: Option<String>,
    /// Custom version override.
    pub custom_version: Option<String>,
    /// Feature support options.
    pub features: SignerFeatures,
    /// Embedding options.
    pub embedding: SignerEmbedding,
    /// Mode.
    pub mode: SignerMode,
    /// Installation mode.
    pub install_mode: SignerInstallMode,
    /// Tweaks to apply before signing.
    pub tweaks: Option<Vec<std::path::PathBuf>>,
    /// App type.
    pub app: SignerApp,
}

impl Default for SignerOptions {
    fn default() -> Self {
        SignerOptions {
            custom_name: None,
            custom_identifier: None,
            custom_version: None,
            features: SignerFeatures::default(),
            embedding: SignerEmbedding::default(),
            mode: SignerMode::default(),
            install_mode: SignerInstallMode::default(),
            tweaks: None,
            app: SignerApp::Default,
        }
    }
}

impl SignerOptions {
    pub fn new_for_app(app: SignerApp) -> Self {
        let mut settings = Self {
            app,
            ..Self::default()
        };

        match app {
            SignerApp::LiveContainer | 
            SignerApp::LiveContainerAndSideStore => {
                settings.embedding.single_profile = true;
            }
            _ => {}
        }

        settings
    }
}

#[derive(Clone, Debug, Default)]
pub struct SignerFeatures {
    pub support_minimum_os_version: bool,
    pub support_file_sharing: bool,
    pub support_ipad_fullscreen: bool,
    pub support_game_mode: bool,
    pub support_pro_motion: bool,
    pub support_liquid_glass: bool,
    pub remove_url_schemes: bool,
}

/// Embedding options.
#[derive(Clone, Debug, Default)]
pub struct SignerEmbedding {
    pub single_profile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerInstallMode {
    Install,
    InstallMac,
    Export,
}

impl Default for SignerInstallMode {
    fn default() -> Self {
        SignerInstallMode::Export
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerMode {
    Pem,
    Adhoc,
    None,
}

impl Default for SignerMode {
    fn default() -> Self {
        SignerMode::Adhoc
    }
}

/// Supported app types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerApp {
    Default,
    Antrag,
    Feather,
    Protokolle,
    AltStore,
    SideStore,
    LiveContainer,
    LiveContainerAndSideStore,
    StikDebug,
    SparseBox
}

impl SignerApp {
    pub fn from_bundle_identifier(identifier: Option<impl AsRef<str>>) -> Self {
        match identifier.as_ref().map(|s| s.as_ref()) {
            Some("com.kdt.livecontainer") => SignerApp::LiveContainer,
            Some("thewonderofyou.syslog") => SignerApp::Protokolle,
            Some("thewonderofyou.antrag2") => SignerApp::Antrag,
            Some("thewonderofyou.Feather") => SignerApp::Feather,
            Some("com.SideStore.SideStore") => SignerApp::SideStore,
            Some("com.rileytestut.AltStore") => SignerApp::AltStore,
            Some("com.stik.js") => SignerApp::StikDebug,
            Some("com.kdt.SparseBox") => SignerApp::SparseBox,
            _ => SignerApp::Default,
        }
    }

    pub fn supports_pairing_file(&self) -> bool {
        !matches!(self, SignerApp::Default | SignerApp::LiveContainer | SignerApp::AltStore)
    }

    pub fn pairing_file_path(&self) -> Option<&'static str> {
        use SignerApp::*;
        match self {
            Antrag | Feather | Protokolle | StikDebug | SparseBox => Some("/Documents/pairingFile.plist"),
            SideStore => Some("/Documents/ALTPairingFile.mobiledevicepairing"),
            LiveContainerAndSideStore => Some("/Documents/SideStore/Documents/ALTPairingFile.mobiledevicepairing"),
            _ => None,
        }
    }
}
