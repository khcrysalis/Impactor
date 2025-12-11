mod default;
pub use default::{DefaultPage, create_default_page};

mod settings;
pub use settings::{LoginDialog, create_login_dialog};
pub use settings::{SettingsDialog, create_settings_dialog};

mod install;
pub use install::{InstallPage, create_install_page};

mod work;
pub use work::{WorkPage, create_work_page};


// TODO: investigate why github actions messes up weird sizing shit
#[cfg(target_os = "linux")]
pub const WINDOW_SIZE: (i32, i32) = (740, 660);
#[cfg(not(target_os = "linux"))]
pub const WINDOW_SIZE: (i32, i32) = (570, 410);

// TODO: investigate why github actions messes up weird sizing shit
#[cfg(target_os = "linux")]
pub const DIALOG_SIZE: (i32, i32) = (500, 500);
#[cfg(not(target_os = "linux"))]
pub const DIALOG_SIZE: (i32, i32) = (400, 300);
