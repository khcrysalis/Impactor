use std::path::PathBuf;

use clap::Args;
use anyhow::Result;

use plume_core::{CertificateIdentity, MobileProvision};
use plume_shared::get_data_path;
use plume_utils::{Bundle, Signer, SignerOptions};

use crate::commands::account::{get_authenticated_account, teams};

#[derive(Debug, Args)]
pub struct SignArgs {
    /// Path to the app bundle to sign (.app or .ipa)
    #[arg(long = "bundle", value_name = "BUNDLE")]
    pub bundle: PathBuf,
    /// PEM files for certificate and private key
    #[arg(long = "pem", value_name = "PEM", num_args = 1..)]
    pub pem_files: Option<Vec<PathBuf>>,
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
    #[arg(long = "adhoc")]
    pub adhoc: bool,
    /// Perform ad-hoc signing (no certificate required)
    #[arg(long = "tweaks", num_args = 1..)]
    pub tweaks: Option<Vec<PathBuf>>,
}

pub async fn execute(args: SignArgs) -> Result<()> {
    let options = SignerOptions {
        custom_identifier: args.bundle_identifier,
        custom_name: args.name,
        custom_version: args.version,
        ..Default::default()
    };
    
    let bundle = Bundle::new(&args.bundle)?;


    if let Some(tweak_files) = args.tweaks {
        println!("Applying tweaks: {:?}", tweak_files);

        for tweak_file in tweak_files {
            let tweak = plume_utils::Tweak::new(tweak_file, &bundle).await?;
            tweak.apply().await?;
        }
    }
    
    let (mut signer, team_id_opt) = if args.adhoc {
        println!("Using ad-hoc signing (no certificate)");
        (Signer::new(None, options), None)
    } else if let Some(ref pem_files) = args.pem_files {
        println!("Using PEM files: {:?}", pem_files);
        let cert_identity = CertificateIdentity::new_with_paths(
            Some(pem_files.clone())
        ).await?;
        (Signer::new(Some(cert_identity), options), None)
    } else {
        println!("No signing method specified, attempting to use saved Apple ID credentials...");
        
        let session = get_authenticated_account().await?;
        
        println!("Fetching teams...");
        let team_id = teams(&session).await?;
        
        println!("Generating certificate for team {}...", team_id);
        let cert_identity = CertificateIdentity::new_with_session(
            &session,
            get_data_path(),
            None,
            &team_id,
        ).await?;
        
        (Signer::new(Some(cert_identity), options), Some((session, team_id)))
    };

    if let Some(provision_path) = args.provisioning_files {
        let prov = MobileProvision::load_with_path(&provision_path)?;
        signer.provisioning_files.push(prov.clone());
        let p = bundle.bundle_dir().join("embedded.mobileprovision");
        tokio::fs::write(p, prov.data).await?;
    }

    if let Some((session, team_id)) = team_id_opt {
        println!("Modifying bundle...");
        signer.modify_bundle(&bundle, &Some(team_id.clone())).await?;
        
        println!("Registering bundle with Apple Developer...");
        signer.register_bundle(&bundle, &session, &team_id).await?;
        
        println!("Signing bundle...");
        signer.sign_bundle(&bundle).await?;
    } else {
        println!("Modifying bundle...");
        signer.modify_bundle(&bundle, &None).await?;
        
        println!("Signing bundle...");
        signer.sign_bundle(&bundle).await?;
    }

    println!("Signing completed successfully");

    Ok(())
}
