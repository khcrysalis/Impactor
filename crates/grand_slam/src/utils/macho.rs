use std::{collections::HashSet, fs};
use std::path::Path;

use apple_codesign::MachFile;
use plist::{Dictionary, Value};

use crate::{Error, developer::v1::capabilities::Capability};

/// Represents a Mach-O file and its entitlements.
pub struct MachO {
    _macho_file: MachFile<'static>,
    pub entitlements: Option<Dictionary>,
}

impl MachO {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let macho_data = fs::read(path)?;
        // Leak the data for 'static lifetime required by MachFile.
        let macho_data = Box::leak(macho_data.into_boxed_slice());
        let macho_file = MachFile::parse(macho_data)?;
        let entitlements = Self::extract_entitlements(&macho_file)?;

        Ok(MachO {
            _macho_file: macho_file,
            entitlements,
        })
    }

    fn extract_entitlements(macho_file: &MachFile<'_>) -> Result<Option<Dictionary>, Error> {
        let macho = macho_file.nth_macho(0)?;
        
        if let Some(embedded_sig) = macho.code_signature()? {
            if let Ok(Some(slot)) = embedded_sig.entitlements() {
                let value = Value::from_reader_xml(slot.to_string().as_bytes())?;
                if let Value::Dictionary(dict) = value {
                    return Ok(Some(dict));
                }
            }
        }
        
        Ok(None)
    }

    pub fn app_groups_for_entitlements(&self) -> Option<Vec<String>> {
        self.entitlements
            .as_ref()
            .and_then(|e| e.get("com.apple.security.application-groups")?.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_string().map(|s| s.to_string())).collect())
    }

    pub fn capabilities_for_entitlements(&self, capabilities: &[Capability]) -> Option<Vec<String>> {
        let entitlements = self.entitlements.as_ref()?;
        let ent_keys: HashSet<_> = entitlements.keys().collect();

        let capabilities_to_enable: Vec<String> = capabilities
            .iter()
            .filter_map(|cap| {
                cap.attributes.entitlements.as_ref().and_then(|ent_list| {
                    if ent_list.iter().any(|e| ent_keys.contains(&e.profile_key)) {
                        Some(cap.id.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        if capabilities_to_enable.is_empty() {
            None
        } else {
            Some(capabilities_to_enable)
        }
    }
}
