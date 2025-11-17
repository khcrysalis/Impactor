use std::fs;
use std::path::{Path, PathBuf};

use plist::{Dictionary, Value};
use uuid::Uuid;
use crate::Error;

use super::MachO;

#[derive(Clone)]
pub struct MobileProvision {
    provision_file: PathBuf,
    provisioning_plist: Value,
    entitlements: Dictionary,
}

impl MobileProvision {
    pub fn load<P: AsRef<Path>>(provision_path: P) -> Result<Self, Error> {
        let path = provision_path.as_ref();
        
        if !path.exists() {
            return Err(Error::ProvisioningEntitlementsUnknown);
        }

        let profile_data = fs::read(path)?;
        let provisioning_plist = Self::extract_plist_from_file(&profile_data)?;
        let entitlements = Self::extract_entitlements(&provisioning_plist)?;
        
        Ok(Self {
            provision_file: path.to_path_buf(),
            provisioning_plist,
            entitlements,
        })
    }
    
    // TODO: make this better....
    pub fn load_from_bytes(data: &[u8]) -> Result<Self, Error> {
        let provisioning_plist = Self::extract_plist_from_file(data)?;
        let entitlements = Self::extract_entitlements(&provisioning_plist)?;
        
        let temp_file_path = std::env::temp_dir().join(format!("plume_provision_{:08}", Uuid::new_v4().to_string().to_uppercase()));
        fs::create_dir_all(&temp_file_path)?;
        let provision_file = temp_file_path.join(format!("{}.mobileprovision", Uuid::new_v4().to_string().to_uppercase()));
        fs::write(&provision_file, data)?;

        Ok(Self {
            provision_file,
            provisioning_plist,
            entitlements,
        })
    }

    pub fn file_path(&self) -> &Path {
        &self.provision_file
    }

    pub fn entitlements(&self) -> &Dictionary {
        &self.entitlements
    }
    
    pub fn replace_wildcard_in_entitlements(&mut self, new_application_id: &str) {
        for value in self.entitlements.values_mut() {
            match value {
                Value::String(s) => {
                    if s.contains('*') {
                        *s = s.replace('*', new_application_id);
                    }
                }
                Value::Array(arr) => {
                    for item in arr.iter_mut() {
                        if let Value::String(s) = item {
                            if s.contains('*') {
                                *s = s.replace('*', new_application_id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    pub fn merge_entitlements(&mut self, binary_path: PathBuf) -> Result<(), Error> {
        let macho = MachO::new(&binary_path)?;
        let binary_entitlements = macho.entitlements.ok_or(Error::ProvisioningEntitlementsUnknown)?;
        
        if let Some(Value::Array(other_groups)) = binary_entitlements.get("keychain-access-groups") {
            self.entitlements.insert("keychain-access-groups".to_string(), Value::Array(other_groups.clone()));
        }

        let new_team_id = self.entitlements
            .get("com.apple.developer.team-identifier")
            .and_then(Value::as_string)
            .map(|s| s.to_owned());
        let old_team_id = binary_entitlements
            .get("com.apple.developer.team-identifier")
            .and_then(Value::as_string)
            .map(|s| s.to_owned())
            .or(Some("AAAAA11111".to_string()));

        if let (Some(new_id), Some(old_id)) = (new_team_id.as_ref(), old_team_id.as_ref()) {
            if let Some(Value::Array(groups)) = self.entitlements.get_mut("keychain-access-groups") {
                for group in groups.iter_mut() {
                    if let Value::String(s) = group {
                        if s.contains(old_id) {
                            *s = s.replace(old_id, new_id);
                        }
                    }
                }
            }
        }
                
        Ok(())
    }

    pub fn entitlements_as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        Value::Dictionary(self.entitlements.clone()).to_writer_xml(&mut buf)?;
        Ok(buf)
    }

    pub fn bundle_id(&self) -> Option<String> {
        let app_id = self.entitlements.get("application-identifier")?.as_string()?;

        let prefix = self.provisioning_plist
            .as_dictionary()?
            .get("ApplicationIdentifierPrefix")?
            .as_array()?
            .get(0)?
            .as_string();

        if let Some(prefix) = prefix {
            app_id.strip_prefix(prefix)
                .map(|rest| rest.trim_start_matches('.').to_string())
                .or_else(|| Some(app_id.to_string()))
        } else {
            Some(app_id.to_string())
        }
    }

    fn extract_plist_from_file(data: &[u8]) -> Result<Value, Error> {
        let start = data.windows(6).position(|w| w == b"<plist").ok_or(Error::ProvisioningEntitlementsUnknown)?;
        let end = data.windows(8).rposition(|w| w == b"</plist>").ok_or(Error::ProvisioningEntitlementsUnknown)? + 8;
        let plist_data = &data[start..end];
        let plist = plist::Value::from_reader_xml(plist_data)?;
        Ok(plist)
    }

    fn extract_entitlements(plist: &Value) -> Result<Dictionary, Error> {
        plist
            .as_dictionary()
            .and_then(|d| d.get("Entitlements"))
            .and_then(|v| v.as_dictionary())
            .cloned()
            .ok_or(Error::ProvisioningEntitlementsUnknown)
    }
}
