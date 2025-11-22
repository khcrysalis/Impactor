use std::{
    fs, 
    path::PathBuf
};
use plist::Value;
use super::PlistInfoTrait;
use crate::Error;

#[derive(Debug, Clone)]
pub struct Bundle {
    dir: PathBuf,
    _type: BundleType,
    info_plist_file: PathBuf,
}

impl Bundle {
    pub fn new<P: Into<PathBuf>>(bundle_path: P) -> Result<Self, Error> {
        let path = bundle_path.into();
        let info_plist_path = path.join("Info.plist");

        if !info_plist_path.exists() {
            return Err(Error::BundleInfoPlistMissing);
        }

        let bundle_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(BundleType::from_extension)
            .unwrap_or(BundleType::Unknown);

        Ok(Self {
            dir: path,
            _type: bundle_type,
            info_plist_file: info_plist_path,
        })
    }
    
    pub fn bundle_dir(&self) -> &PathBuf {
        &self.dir
    }
    
    pub fn bundle_type(&self) -> &BundleType {
        &self._type
    }
    
    pub fn collect_nested_bundles(&self) -> Result<Vec<Bundle>, Error> {
        collect_embeded_bundles_from_dir(&self.dir)
    }

    pub fn collect_bundles_sorted(&self) -> Result<Vec<Bundle>, Error> {
        let mut bundles = self.collect_nested_bundles()?;
        bundles.push(self.clone());
        bundles.sort_by_key(|b| b.bundle_dir().components().count());
        bundles.reverse();
        
        Ok(bundles)
    }
}

impl Bundle {
    pub fn set_info_plist_key<V: Into<Value>>(
        &self,
        key: &str,
        value: V,
    ) -> Result<(), Error> {
        let mut plist = Value::from_file(&self.info_plist_file)?;
        if let Some(dict) = plist.as_dictionary_mut() {
            dict.insert(key.to_string(), value.into());
        }
        plist.to_file_xml(&self.info_plist_file)?;
        
        Ok(())
    }
    
    // TODO: we need to support changing lproj infoplist strings so localized names change as well
    pub fn set_name(&self, new_name: &str) -> Result<(), Error> {
        self.set_info_plist_key("CFBundleDisplayName", new_name)?;
        self.set_info_plist_key("CFBundleName", new_name)
    }
    
    pub fn set_version(&self, new_version: &str) -> Result<(), Error> {
        self.set_info_plist_key("CFBundleShortVersionString", new_version)?;
        self.set_info_plist_key("CFBundleVersion", new_version)
    }
    
    pub fn set_bundle_identifier(&self, new_identifier: &str) -> Result<(), Error> {
        self.set_info_plist_key("CFBundleIdentifier", new_identifier)
    }

    pub fn set_matching_identifier(&self, old_identifier: &str, new_identifier: &str) -> Result<(), Error> {
        let mut did_change = false;
        let mut plist = Value::from_file(&self.info_plist_file)?;

        // CFBundleIdentifier
        if let Some(dict) = plist.as_dictionary_mut() {
            if let Some(Value::String(old_value)) = dict.get("CFBundleIdentifier") {
                let new_value = old_value.replace(old_identifier, new_identifier);
                if old_value != &new_value {
                    dict.insert("CFBundleIdentifier".to_string(), Value::String(new_value));
                    did_change = true;
                }
            }

            // WKCompanionAppBundleIdentifier
            if let Some(Value::String(old_value)) = dict.get("WKCompanionAppBundleIdentifier") {
                let new_value = old_value.replace(old_identifier, new_identifier);
                if old_value != &new_value {
                    dict.insert("WKCompanionAppBundleIdentifier".to_string(), Value::String(new_value));
                    did_change = true;
                }
            }

            // NSExtension → NSExtensionAttributes → WKAppBundleIdentifier
            if let Some(Value::Dictionary(extension_dict)) = dict.get_mut("NSExtension") {
                if let Some(Value::Dictionary(attributes)) = extension_dict.get_mut("NSExtensionAttributes") {
                    if let Some(Value::String(old_value)) = attributes.get("WKAppBundleIdentifier") {
                        let new_value = old_value.replace(old_identifier, new_identifier);
                        if old_value != &new_value {
                            attributes.insert("WKAppBundleIdentifier".to_string(), Value::String(new_value));
                            did_change = true;
                        }
                    }
                }
            }
        }

        if did_change {
            plist.to_file_xml(&self.info_plist_file)?;
        }

        Ok(())
    }
}

macro_rules! get_plist_string {
    ($self:ident, $key:expr) => {{
        let plist = Value::from_file(&$self.info_plist_file).ok()?;
        plist
            .as_dictionary()
            .and_then(|dict| dict.get($key))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
    }};
}

impl PlistInfoTrait for Bundle {
    fn get_name(&self) -> Option<String> {
        get_plist_string!(self, "CFBundleDisplayName")
            .or_else(|| get_plist_string!(self, "CFBundleName"))
            .or_else(|| self.get_executable())
    }

    fn get_executable(&self) -> Option<String> {
        get_plist_string!(self, "CFBundleExecutable")
    }

    fn get_bundle_identifier(&self) -> Option<String> {
        get_plist_string!(self, "CFBundleIdentifier")
    }

    fn get_version(&self) -> Option<String> {
        get_plist_string!(self, "CFBundleShortVersionString")
    }

    fn get_build_version(&self) -> Option<String> {
        get_plist_string!(self, "CFBundleVersion")
    }
}

fn collect_embeded_bundles_from_dir(dir: &PathBuf) -> Result<Vec<Bundle>, Error> {
    let mut bundles = Vec::new();
    
    fn is_bundle_dir(name: &str) -> bool {
        if let Some((_, ext)) = name.rsplit_once('.') {
            BundleType::from_extension(ext).is_some()
        } else {
            false
        }
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        if path.file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n.ends_with(".storyboardc"))
        {
            continue;
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if is_bundle_dir(name) {
                if let Ok(bundle) = Bundle::new(&path) {
                    if bundle.info_plist_file.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map_or(false, |n| n.ends_with(".storyboardc"))
                    {
                        continue;
                    }

                    bundles.push(bundle.clone());

                    if bundle._type != BundleType::App {
                        if let Ok(embedded) = bundle.collect_nested_bundles() {
                            bundles.extend(embedded);
                        }
                    }
                    continue;
                }
            }
        }

        if path.is_dir() {
            if let Ok(mut sub_bundles) = collect_embeded_bundles_from_dir(&path) {
                bundles.append(&mut sub_bundles);
            }
        }
    }

    Ok(bundles)
}

#[derive(Debug, Clone, PartialEq)]
pub enum BundleType {
    App,
    AppExtension,
    Framework,
    Unknown
}

impl BundleType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "app" => Some(BundleType::App),
            "appex" => Some(BundleType::AppExtension),
            "framework" => Some(BundleType::Framework),
            _ => Some(BundleType::Unknown),
        }
    }
}
