use std::{env, fs};
use std::io::Read;

use std::path::PathBuf;

use plist::Dictionary;
use uuid::Uuid;
use zip::ZipArchive;

use grand_slam::Bundle;
use grand_slam::utils::PlistInfoTrait;
use crate::Error;

#[derive(Debug, Clone)]
pub struct Package {
    package_file: PathBuf,
    stage_dir: PathBuf,
    stage_payload_dir: PathBuf,
    info_plist_dictionary: Dictionary,
}

impl Package {
    pub fn new(package_file: PathBuf) -> Result<Self, Error> {
        let stage_dir = env::temp_dir().join(format!("plume_stage_{:08}", Uuid::new_v4().to_string().to_uppercase()));
        let out_package_file = stage_dir.join("stage.ipa");

        fs::create_dir_all(&stage_dir).ok();
        fs::copy(&package_file, &out_package_file)?;
        let info_plist_dictionary = Self::get_info_plist_contents(&out_package_file)?;

        Ok(Self {
            package_file: out_package_file,
            stage_dir: stage_dir.clone(),
            stage_payload_dir: stage_dir.join("Payload"),
            info_plist_dictionary,
        })
    }
    
    pub fn get_package_bundle(&self) -> Result<Bundle, Error> {
        let file = fs::File::open(&self.package_file)?;
        let mut archive = ZipArchive::new(file)?;
        archive.extract(&self.stage_dir)?;
        
        let app_dir = fs::read_dir(&self.stage_payload_dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.is_dir() && p.extension().and_then(|e| e.to_str()) == Some("app"))
            .ok_or_else(|| Error::PackageInfoPlistMissing)?;

        Ok(Bundle::new(app_dir)?)
    }

    fn get_info_plist_contents(package_file: &PathBuf) -> Result<Dictionary, Error> {
        let mut archive = ZipArchive::new(fs::File::open(package_file)?)?;
        let info_name = {
            let mut names = archive.file_names();
            names
                .find(|name| name.starts_with("Payload/") 
                    && name.ends_with(".app/Info.plist")
                    && name.matches('/').count() == 2)
                .ok_or(Error::PackageInfoPlistMissing)?
                .to_string()
        };
        let mut entry = archive.by_name(&info_name)?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        Ok(plist::from_bytes(&buf)?)
    }
}

macro_rules! get_plist_dict_value {
    ($self:ident, $key:expr) => {{
        $self.info_plist_dictionary
            .get($key)
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
    }};
}

impl PlistInfoTrait for Package {
    fn get_name(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleDisplayName")
            .or_else(|| get_plist_dict_value!(self, "CFBundleName"))
            .or_else(|| self.get_executable())
    }

    fn get_executable(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleExecutable")
    }

    fn get_bundle_identifier(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleIdentifier")
    }

    fn get_version(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleShortVersionString")
    }

    fn get_build_version(&self) -> Option<String> {
        get_plist_dict_value!(self, "CFBundleVersion")
    }
}

// impl Drop for Package {
//     fn drop(&mut self) {
//         fs::remove_dir_all(&self.stage_dir).ok();
//     }
// }
