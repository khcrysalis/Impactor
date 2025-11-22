use std::path::PathBuf;

use clap::Parser;

use clap::{Args, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Sign(SignArgs),
}

#[derive(Debug, Args)]
pub struct SignArgs {
    #[arg(long = "pem", value_name = "PEM", num_args = 1.., required = true, help = "PEM files for certificate and private key")]
    pub pem_files: Vec<PathBuf>,

    #[arg(long = "provision", value_name = "PROVISION", num_args = 1.., required = true, help = "Provisioning profile files to embed")]
    pub provisioning_files: Vec<PathBuf>,

    #[arg(value_name = "BUNDLE", long = "bundle", required = true, help = "Path to the app bundle to sign")]
    pub bundle: PathBuf,

    #[arg(long = "custom-identifier", value_name = "BUNDLE_ID", help = "Custom bundle identifier to set")]
    pub bundle_identifier: Option<String>,

    #[arg(long = "custom-name", value_name = "NAME", help = "Custom bundle name to set")]
    pub name: Option<String>,

    #[arg(long = "custom-version", value_name = "VERSION", help = "Custom bundle version to set")]
    pub version: Option<String>,
}

#[tokio::main]
async fn main() {
    todo!();
}
