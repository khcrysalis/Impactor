use std::fs;
use std::path::PathBuf;

use apple_codesign::{SigningSettings, UnifiedSigner};

use crate::Error;

use super::{CertificateIdentity, MobileProvision};
use super::SignerSettings;
use super::{Bundle, BundleType, PlistInfoTrait};

pub struct Signer {
    certificate: Option<CertificateIdentity>,
    settings: SignerSettings,
    provisioning_files: Vec<MobileProvision>,
}

impl Signer {
    pub fn new(
        certificate: Option<CertificateIdentity>,
        settings: SignerSettings,
        provisioning_files: Vec<MobileProvision>,
    ) -> Self {
        Self {
            certificate,
            settings,
            provisioning_files,
        }
    }

    pub fn sign_path(&self, path: PathBuf) -> Result<(), Error> {
        let bundle = Bundle::new(path)?;
        self.sign_bundle(&bundle)
    }

    pub fn sign_bundle(&self, bundle: &Bundle) -> Result<(), Error> {
        let bundles = bundle.collect_bundles_sorted()?;
        
        for bundle in &bundles {
            let mut settings = self.build_base_settings()?;

            if bundle._type == BundleType::AppExtension || bundle._type == BundleType::App {
                let mut matched_prov = None;

                for prov in &self.provisioning_files {
                    if let (Some(bundle_id), Some(team_id)) = (bundle.get_bundle_identifier(), prov.bundle_id()) {
                        if team_id == bundle_id {
                            matched_prov = Some(prov);
                            break;
                        }
                    }
                }

                let mut prov = matched_prov.unwrap_or_else(|| &self.provisioning_files[0]).clone();

                if let Some(bundle_id) = bundle.get_bundle_identifier() {
                    prov.replace_wildcard_in_entitlements(&bundle_id);
                }

                if let Some(bundle_executable) = bundle.get_executable() {
                    let binary_path = bundle.dir().join(bundle_executable);
                    prov.merge_entitlements(binary_path).ok(); // if it fails we can ignore 
                }

                if self.settings.should_embed_provisioning {
                    fs::write(bundle.dir().join("embedded.mobileprovision"), &prov.provision_data)?;
                }

                if let Ok(ent_xml) = prov.entitlements_as_bytes() {
                    settings.set_entitlements_xml(
                        apple_codesign::SettingsScope::Main, 
                        String::from_utf8_lossy(&ent_xml)
                    )?;
                }
            }
            
            UnifiedSigner::new(settings).sign_path_in_place(bundle.dir())?;
        }

        if let Some(cert) = &self.certificate {
            if let Some(key) = &cert.key {
                key.finish()?;
            }
        }

        Ok(())
    }

    fn build_base_settings(&self) -> Result<SigningSettings<'_>, Error> {
        let mut settings = SigningSettings::default();
        if let Some(cert) = &self.certificate {
            cert.load_into_signing_settings(&mut settings)?;
            settings.set_team_id_from_signing_certificate();
        }
        settings.set_for_notarization(false);
        settings.set_shallow(false);
        settings.set_team_id_from_signing_certificate();
        Ok(settings)
    }
}
