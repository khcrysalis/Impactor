pub mod default;
pub use default::{DefaultPage, create_default_page};

pub mod install;
pub use install::{InstallPage, create_install_page};

pub mod login;
pub use login::{create_login_dialog, create_account_dialog};

#[cfg(target_os = "linux")]
pub const WINDOW_SIZE: (i32, i32) = (700, 660);
#[cfg(not(target_os = "linux"))]
pub const WINDOW_SIZE: (i32, i32) = (530, 410);

#[cfg(target_os = "linux")]
pub const DIALOG_SIZE: (i32, i32) = (500, 300);
#[cfg(not(target_os = "linux"))]
pub const DIALOG_SIZE: (i32, i32) = (400, 200);
