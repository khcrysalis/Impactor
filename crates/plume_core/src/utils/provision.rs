use std::fs;
use std::path::{Path, PathBuf};

use crate::Error;
use crate::utils::TEAM_ID_REGEX;
use plist::{Date, Dictionary, Value};

use super::MachO;

#[derive(Clone)]
pub struct MobileProvision {
    pub data: Vec<u8>,
    entitlements: Dictionary,
    expiration_date: Date,
}

impl MobileProvision {
    pub fn load_with_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = fs::read(path)?;

        Self::load_with_bytes(data)
    }

    pub fn load_with_bytes(data: Vec<u8>) -> Result<Self, Error> {
        let (entitlements, expiration_date) = Self::extract_entitlements_from_prov(&data)?;

        Ok(Self {
            data,
            entitlements,
            expiration_date,
        })
    }

    pub fn merge_entitlements(
        &mut self,
        binary_path: PathBuf,
        new_application_id: &str,
    ) -> Result<(), Error> {
        let macho = MachO::new(&binary_path)?;
        let binary_entitlements = macho
            .entitlements()
            .clone()
            .ok_or(Error::ProvisioningEntitlementsUnknown)?;

        let new_team_id = self
            .entitlements
            .get("com.apple.developer.team-identifier")
            .and_then(Value::as_string)
            .map(|s| s.to_owned());

        crate::utils::merge_entitlements(
            &mut self.entitlements,
            &binary_entitlements,
            &new_team_id,
            &Some(new_application_id.to_string()),
        );

        Ok(())
    }

    pub fn entitlements(&self) -> &Dictionary {
        &self.entitlements
    }

    pub fn expiration_date(&self) -> &Date {
        &self.expiration_date
    }

    pub fn entitlements_as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        Value::Dictionary(self.entitlements.clone()).to_writer_xml(&mut buf)?;
        Ok(buf)
    }

    pub fn bundle_id(&self) -> Option<String> {
        let app_id = self
            .entitlements
            .get("application-identifier")?
            .as_string()?;

        let re = regex::Regex::new(TEAM_ID_REGEX).ok()?;
        let bundle_id = re.replace(app_id, "").to_string();

        Some(bundle_id)
    }

    fn extract_entitlements_from_prov(data: &[u8]) -> Result<(Dictionary, Date), Error> {
        let start = data
            .windows(6)
            .position(|w| w == b"<plist")
            .ok_or(Error::ProvisioningEntitlementsUnknown)?;
        let end = data
            .windows(8)
            .rposition(|w| w == b"</plist>")
            .ok_or(Error::ProvisioningEntitlementsUnknown)?
            + 8;
        let plist_data = &data[start..end];
        let plist = plist::Value::from_reader_xml(plist_data)?;

        let expiration_date = plist
            .as_dictionary()
            .and_then(|d| d.get("ExpirationDate"))
            .and_then(|v| v.as_date());

        let entitlements = plist
            .as_dictionary()
            .and_then(|d| d.get("Entitlements"))
            .and_then(|v| v.as_dictionary())
            .cloned()
            .ok_or(Error::ProvisioningEntitlementsUnknown);

        Ok((
            entitlements?,
            expiration_date.ok_or(Error::ProvisioningEntitlementsUnknown)?,
        ))
    }
}
