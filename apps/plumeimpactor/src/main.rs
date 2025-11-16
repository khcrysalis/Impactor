#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod frame;
mod keychain;
mod pages;
mod handlers;
mod utils;

use std::{
    env, 
    fs, 
    path::{Path, PathBuf}
};

pub const APP_NAME: &str = concat!(env!("CARGO_PKG_NAME"), " – Version ", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() {
    _ = rustls::crypto::ring::default_provider().install_default().unwrap();

    let _ = wxdragon::main(|_| {
        frame::PlumeFrame::new().show();
    });
}

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Info.plist not found")]
    PackageInfoPlistMissing,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plist error: {0}")]
    Plist(#[from] plist::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Idevice error: {0}")]
    Idevice(#[from] idevice::IdeviceError),
    #[error("GrandSlam error: {0}")]
    GrandSlam(#[from] grand_slam::Error),
}

pub fn get_data_path() -> PathBuf {
    let dir = Path::new(&env::var("HOME").unwrap())
        .join(".config")
        .join("PlumeImpactor");
    
    fs::create_dir_all(&dir).ok();
    
    dir
}
