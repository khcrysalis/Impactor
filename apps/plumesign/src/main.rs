use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use clap::{Args, Subcommand};
use grand_slam::utils::Certificate;
use grand_slam::utils::MobileProvision;
use grand_slam::utils::Signer;
use grand_slam::utils::SignerSettings;

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
    let cli = Cli::parse();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("--x failed to install rustls crypto provider");

    match &cli.command {
        Commands::Sign(args) => {
            if args.pem_files.len() < 2 {
                eprintln!("--x at least two PEM files (certificate and key) are required via --pem.");
                exit(1);
            }
            
            let signing_key = Certificate::new(args.pem_files.clone().into()).unwrap_or_else(|e| {
                eprintln!("--x failed to create Certificate: {e}");
                exit(1);
            });

            let provisioning_files = args.provisioning_files.iter()
                .map(MobileProvision::load)
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_else(|e| {
                    eprintln!("--x failed to load provisioning profiles: {e}");
                    exit(1);
                });

            let signer_settings = SignerSettings {
                custom_name: args.name.clone(),
                custom_identifier: args.bundle_identifier.clone(),
                custom_build_version: args.version.clone(),
                ..Default::default()
            };

            let signer = Signer::new(Some(signing_key), signer_settings, provisioning_files);

            let target_path = args.bundle.clone();
            if let Err(e) = signer.sign(target_path.clone()) {
                eprintln!("--x failed to sign: {e}");
                exit(1);
            }
            
            println!("--> signed: {:?}", target_path);
        }
    }
}
