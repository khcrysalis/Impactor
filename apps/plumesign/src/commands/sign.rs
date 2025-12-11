use std::path::PathBuf;

use clap::Args;
use anyhow::Result;

use plume_core::{CertificateIdentity, MobileProvision};
use plume_shared::get_data_path;
use plume_utils::{Bundle, Signer, SignerMode, SignerOptions};

use crate::commands::account::{get_authenticated_account, teams};

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct SignArgs {
    /// Path to the app bundle to sign (.app or .ipa)
    #[arg(long = "bundle", value_name = "BUNDLE")]
    pub bundle: PathBuf,
    /// PEM files for certificate and private key
    #[arg(long = "pem", value_name = "PEM", num_args = 1..)]
    pub pem_files: Option<Vec<PathBuf>>,
    /// Use Apple ID credentials for signing
    #[arg(long = "apple-id")]
    pub apple_id: bool,
    /// Provisioning profile files to embed
    #[arg(long = "provision", value_name = "PROVISION")]
    pub provisioning_files: Option<PathBuf>,
    /// Custom bundle identifier to set
    #[arg(long = "custom-identifier", value_name = "BUNDLE_ID")]
    pub bundle_identifier: Option<String>,
    /// Custom bundle name to set
    #[arg(long = "custom-name", value_name = "NAME")]
    pub name: Option<String>,
    /// Custom bundle version to set
    #[arg(long = "custom-version", value_name = "VERSION")]
    pub version: Option<String>,
    /// Perform ad-hoc signing (no certificate required)
    #[arg(long = "tweaks", num_args = 1..)]
    pub tweaks: Option<Vec<PathBuf>>,
}

pub async fn execute(args: SignArgs) -> Result<()> {
    let mut options = SignerOptions {
        custom_identifier: args.bundle_identifier,
        custom_name: args.name,
        custom_version: args.version,
        tweaks: args.tweaks,
        ..Default::default()
    };
    
    let bundle = Bundle::new(&args.bundle)?;
    
    let (mut signer, team_id_opt) = if let Some(ref pem_files) = args.pem_files {
        let cert_identity = CertificateIdentity::new_with_paths(
            Some(pem_files.clone())
        ).await?;

        options.mode = SignerMode::Pem;
        (Signer::new(Some(cert_identity), options), None)
    } else if args.apple_id {
        let session = get_authenticated_account().await?;
        let team_id = teams(&session).await?;
        let cert_identity = CertificateIdentity::new_with_session(
            &session,
            get_data_path(),
            None,
            &team_id,
        ).await?;

        options.mode = SignerMode::Pem;
        (Signer::new(Some(cert_identity), options), Some((session, team_id)))
    } else {
        options.mode = SignerMode::Adhoc;
        (Signer::new(None, options), None)
    };

    if let Some(provision_path) = args.provisioning_files {
        let prov = MobileProvision::load_with_path(&provision_path)?;
        signer.provisioning_files.push(prov.clone());
        let p = bundle.bundle_dir().join("embedded.mobileprovision");
        tokio::fs::write(p, prov.data).await?;
    }

    if let Some((session, team_id)) = team_id_opt {
        signer.modify_bundle(&bundle, &Some(team_id.clone())).await?;
        signer.register_bundle(&bundle, &session, &team_id).await?;
        signer.sign_bundle(&bundle).await?;
    } else {
        signer.modify_bundle(&bundle, &None).await?;
        signer.sign_bundle(&bundle).await?;
    }

    Ok(())
}

