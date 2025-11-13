pub mod default;
pub use default::{DefaultPage, create_default_page};

pub mod install;
pub use install::{InstallPage, create_install_page};

pub mod login;
pub use login::{create_login_dialog, create_account_dialog};
