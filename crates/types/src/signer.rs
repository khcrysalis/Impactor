use tokio::fs;
use futures::future::try_join_all;
use plist::Value;
use std::sync::Arc;

use plume_core::{
    CertificateIdentity,
    MobileProvision,
    SettingsScope,
    SigningSettings,
    UnifiedSigner, developer::DeveloperSession,
};

use crate::{
    Bundle, BundleType, Error, PlistInfoTrait, SignerApp, SignerMode, SignerOptions,
};

pub struct Signer {
    certificate: Option<CertificateIdentity>,
    pub options: SignerOptions,
    pub provisioning_files: Vec<MobileProvision>,
}

impl Signer {
    pub fn new(
        certificate: Option<CertificateIdentity>,
        options: SignerOptions,
    ) -> Self {
        Self {
            certificate,
            options,
            provisioning_files: Vec::new(),
        }
    }

    pub async fn modify_bundle(&mut self, bundle: &Bundle, team_id: &Option<String>) -> Result<(), Error> {
        if self.options.mode == SignerMode::None {
            return Ok(());
        }

        let bundles = bundle.collect_bundles_sorted()?
            .into_iter()
            .filter(|b| b.bundle_type().should_have_entitlements())
            .collect::<Vec<_>>();

        if let Some(new_name) = self.options.custom_name.as_ref() {
            bundle.set_name(new_name)?;
        }

        if let Some(new_version) = self.options.custom_version.as_ref() {
            bundle.set_version(new_version)?;
        }

        if self.options.features.support_minimum_os_version {
            bundle.set_info_plist_key("MinimumOSVersion", "7.0")?;
        }

        if self.options.features.support_file_sharing {
            bundle.set_info_plist_key("UIFileSharingEnabled", true)?;
            bundle.set_info_plist_key("UISupportsDocumentBrowser", true)?;
        }

        if self.options.features.support_ipad_fullscreen {
            bundle.set_info_plist_key("UIRequiresFullScreen", true)?;
        }

        if self.options.features.support_game_mode {
            bundle.set_info_plist_key("GCSupportsGameMode", true)?;
        }

        if self.options.features.support_pro_motion {
            bundle.set_info_plist_key("CADisableMinimumFrameDurationOnPhone", true)?;
        }

        let identifier = bundle.get_bundle_identifier();

        if self.options.mode != SignerMode::Adhoc
            && self.options.custom_identifier.is_none()
        {
            if let (Some(identifier), Some(team_id)) = (identifier.as_ref(), team_id.as_ref()) {
                self.options.custom_identifier = Some(format!("{identifier}.{team_id}"));
            }
        }

        if let Some(new_identifier) = self.options.custom_identifier.as_ref() {
            if let Some(orig_identifier) = identifier {
                for embedded_bundle in &bundles {
                    embedded_bundle.set_matching_identifier(&orig_identifier, new_identifier)?;
                }
            }
        }

        if self.options.app == SignerApp::SideStore
            || self.options.app == SignerApp::AltStore
            || self.options.app == SignerApp::LiveContainerAndSideStore
        {
            if let Some(cert_identity) = &self.certificate {
                if let (Some(p12_data), Some(serial_number)) = (&cert_identity.p12_data, &cert_identity.serial_number) {
                    match self.options.app {
                        SignerApp::LiveContainerAndSideStore => {
                            if let Some(embedded_bundle) = bundles.iter()
                                .find(|b| b.bundle_dir()
                                .ends_with("SideStoreApp.framework"))
                            {
                                embedded_bundle.set_info_plist_key("ALTCertificateID", &**serial_number)?;
                                fs::write(
                                    embedded_bundle.bundle_dir().join("ALTCertificate.p12"),
                                    p12_data,
                                ).await?;
                            }
                        }
                        SignerApp::SideStore | SignerApp::AltStore => {
                            bundle.set_info_plist_key("ALTCertificateID", &**serial_number)?;
                            fs::write(bundle.bundle_dir().join("ALTCertificate.p12"), p12_data).await?;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(tweak_files) = self.options.tweaks.as_ref() {
            crate::Tweak::install_ellekit(&bundle).await?;

            for tweak_file in tweak_files {
                let tweak = crate::Tweak::new(tweak_file, bundle).await?;
                tweak.apply().await?;
            }
        }

        if self.options.features.support_liquid_glass {
            bundle.set_info_plist_key("UIDesignRequiresCompatibility", false)?;

            let executable_name = bundle.get_executable()
                .ok_or(Error::BundleInfoPlistMissing)?;

            let executable_path = bundle.bundle_dir().join(&executable_name);
            if !executable_path.exists() {
                return Err(Error::BundleInfoPlistMissing);
            }

            let mut macho = plume_core::MachO::new(&executable_path)?;
            macho.replace_sdk_version("26.0.0")?;
        }

        Ok(())
    }

    pub async fn register_bundle(
        &mut self, 
        bundle: &Bundle,
        session: &DeveloperSession,
        team_id: &String,
    ) -> Result<(), Error> {
        if self.options.mode != SignerMode::Pem {
            return Ok(());
        }

        let bundles = bundle.collect_bundles_sorted()?
            .into_iter()
            .filter(|b| b.bundle_type().should_have_entitlements())
            .collect::<Vec<_>>();
        let signer_settings = &self.options;

        let bundle_arc = Arc::new(bundle.clone());
        let session_arc = Arc::new(session);
        let team_id_arc = Arc::new(team_id.clone());

        let futures = bundles.iter().filter_map(|sub_bundle| {
            let sub_bundle = sub_bundle.clone();
            let bundle = bundle_arc.clone();
            let session = session_arc.clone();
            let team_id = team_id_arc.clone();
            let signer_settings = signer_settings.clone();

            if signer_settings.embedding.single_profile && sub_bundle.bundle_dir() != bundle.bundle_dir() {
                return None;
            }
            if *sub_bundle.bundle_type() != BundleType::AppExtension && *sub_bundle.bundle_type() != BundleType::App {
                return None;
            }

            Some(async move {
                let bundle_executable_name = sub_bundle.get_executable()
                    .ok_or_else(|| Error::Other("Failed to get bundle executable name.".into()))?;
                let bundle_executable_path = sub_bundle.bundle_dir().join(&bundle_executable_name);

                let macho = plume_core::MachO::new(&bundle_executable_path)?;

                let id = sub_bundle.get_bundle_identifier()
                    .ok_or_else(|| Error::Other("Failed to get bundle identifier.".into()))?;

                session.qh_ensure_app_id(&team_id, &sub_bundle.get_name().unwrap_or_default(), &id).await?;

                let app_id_id = session.qh_get_app_id(&team_id, &id).await?
                    .ok_or_else(|| Error::Other("Failed to get ensured app ID.".into()))?;

                if let Some(e) = macho.entitlements().as_ref() {
                    session.v1_request_capabilities_for_entitlements(&team_id, &id, e).await?;
                }

                if let Some(app_groups) = macho.app_groups_for_entitlements() {
                    let mut app_group_ids: Vec<String> = Vec::new();
                    for group in &app_groups {
                        let group = format!("{group}.{team_id}");
                        let group_id = session.qh_ensure_app_group(&team_id, &group, &group).await?;
                        app_group_ids.push(group_id.application_group);
                    }

                    if signer_settings.app == SignerApp::SideStore || signer_settings.app == SignerApp::AltStore {
                        bundle.set_info_plist_key(
                            "ALTAppGroups",
                            Value::Array(app_groups.iter().map(|s| Value::String(format!("{s}.{team_id}"))).collect())
                        )?;
                    }

                    session.qh_assign_app_group(&team_id, &app_id_id.app_id_id, &app_group_ids).await?;
                }

                let profiles = session.qh_get_profile(&team_id, &app_id_id.app_id_id).await?;
                let profile_data = profiles.provisioning_profile.encoded_profile;

                tokio::fs::write(sub_bundle.bundle_dir().join("embedded.mobileprovision"), &profile_data).await?;
                let mobile_provision = MobileProvision::load_with_bytes(profile_data.as_ref().to_vec())?;
                Ok::<_, Error>(mobile_provision)
            })
        });

        let provisionings: Vec<MobileProvision> = try_join_all(futures).await?;
        self.provisioning_files = provisionings;

        Ok(())
    }

    pub async fn sign_bundle(&self, bundle: &Bundle) -> Result<(), Error> {
        if self.options.mode == SignerMode::None {
            return Ok(());
        }

        let bundles = bundle.collect_bundles_sorted()?;

        let settings = Self::build_base_settings(self.certificate.as_ref())?;
        let entitlements_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict/>
</plist>
"#.to_string();

        for bundle in &bundles {
            log::debug!("Signing bundle: {}", bundle.bundle_dir().display());
            Self::sign_single_bundle(
                self,
                bundle, 
                &self.provisioning_files,
                settings.clone(),
                &entitlements_xml,
            )?;
        }

        if let Some(cert) = &self.certificate {
            if let Some(key) = &cert.key {
                key.finish()?;
            }
        }

        Ok(())
    }

    fn sign_single_bundle(
        &self,
        bundle: &Bundle,
        provisioning_files: &[MobileProvision],
        mut settings: SigningSettings<'_>,
        entitlements_xml: &String,
    ) -> Result<(), Error> {
        if *bundle.bundle_type() == BundleType::Unknown {
            return Ok(());
        }

        let mut entitlements_xml = entitlements_xml.clone();

        // Only Apps and AppExtensions should have entitlements from provisioning profiles
        // Dylibs, frameworks, and other components should be signed without entitlements
        // Skip provisioning profile handling for adhoc signing
        if self.options.mode != SignerMode::Adhoc && bundle.bundle_type().should_have_entitlements() && !provisioning_files.is_empty() {
            let mut matched_prov = None;

            for prov in provisioning_files {
                if let (Some(bundle_id), Some(team_id)) = (bundle.get_bundle_identifier(), prov.bundle_id()) {
                    if team_id == bundle_id {
                        matched_prov = Some(prov);
                        break;
                    }
                }
            }

            if let Some(prov) = matched_prov.or_else(|| provisioning_files.first()) {
                let mut prov = prov.clone();

                if let Some(bundle_executable) = bundle.get_executable() {
                    if let Some(bundle_id) = bundle.get_bundle_identifier() {
                        let binary_path = bundle.bundle_dir().join(bundle_executable);
                        prov.merge_entitlements(binary_path, &bundle_id).ok();
                    }
                }

                std::fs::write(
                    bundle.bundle_dir().join("embedded.mobileprovision"),
                    &prov.data,
                )?;

                if let Ok(ent_xml) = prov.entitlements_as_bytes() {
                    entitlements_xml = String::from_utf8_lossy(&ent_xml).to_string();
                }
            }
        }

        if self.options.mode != SignerMode::Adhoc {
            settings.set_entitlements_xml(SettingsScope::Main, entitlements_xml)?;
        }

        UnifiedSigner::new(settings).sign_path_in_place(bundle.bundle_dir())?;

        Ok(())
    }

    fn build_base_settings(
        certificate: Option<&CertificateIdentity>,
    ) -> Result<SigningSettings<'_>, Error> {
        let mut settings = SigningSettings::default();

        if let Some(cert) = certificate {
            cert.load_into_signing_settings(&mut settings)?;
        }

        settings.set_for_notarization(false);
        // TODO: look into shallow options
        settings.set_shallow(false);

        Ok(settings)
    }
}
