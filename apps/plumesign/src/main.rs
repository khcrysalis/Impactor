use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use clap::{Args, Subcommand};
use grand_slam::{CertificateIdentity, Bundle, MobileProvision, Signer};
use grand_slam::utils::{PlistInfoTrait, SignerSettings};

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
            
            let signing_key = CertificateIdentity::new_with_paths(args.pem_files.clone().into()).await.unwrap_or_else(|e| {
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
                custom_version: args.version.clone(),
                ..Default::default()
            };

            let bundle = Bundle::new(args.bundle.clone()).unwrap_or_else(|e| {
                eprintln!("--x failed to load bundle: {e}");
                exit(1);
            });

            if let Some(new_name) = signer_settings.custom_name.as_ref() {
                if let Err(e) = bundle.set_name(new_name) {
                    eprintln!("--x Failed to set new name: {}", e);
                    exit(1);
                }
            }

            if let Some(new_version) = signer_settings.custom_version.as_ref() {
                if let Err(e) = bundle.set_version(new_version) {
                    eprintln!("--x Failed to set new version: {}", e);
                    exit(1);
                }
            }

            if let Some(new_identifier) = &signer_settings.custom_identifier {
                let original_identifier = bundle.get_bundle_identifier().unwrap();

                match bundle.collect_bundles_sorted() {
                    Ok(bundles) => {
                        for b in bundles {
                            if let Err(e) = b.set_matching_identifier(&original_identifier, new_identifier) {
                                eprintln!("--x Failed to set new identifier: {}", e);
                                exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("--x Failed to collect bundles: {}", e);
                        exit(1);
                    }
                }
            }

            let signer = Signer::new(Some(signing_key), signer_settings, provisioning_files);

            let target_path = args.bundle.clone();
            if let Err(e) = signer.sign_path(target_path.clone()) {
                eprintln!("--x failed to sign: {e}");
                exit(1);
            }
            
            println!("--> signed: {:?}", target_path);
        }
    }
}
