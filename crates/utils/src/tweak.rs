use std::{
    env, 
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::{Bundle, Error, PlistInfoTrait, copy_dir_recursively};

pub struct Tweak {
    path: PathBuf,
    app_bundle: PathBuf,
    stage_dir: PathBuf,
}

impl Tweak {
    pub async fn new<P: AsRef<Path>>(tweak_path: P, app_bundle: &Bundle) -> Result<Self, Error> {
        let path = tweak_path.as_ref();
        if !path.exists() {
            return Err(Error::TweakInvalidPath);
        }

        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or(Error::TweakInvalidPath)?;
        
        if !file_name.ends_with(".deb") 
            && !file_name.ends_with(".dylib") 
            && !file_name.ends_with(".framework")
            && !file_name.ends_with(".bundle") {
            return Err(Error::UnsupportedFileType(file_name.to_string()));
        }

        let stage_dir = env::temp_dir().join(format!("plume_tweak_{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&stage_dir).await?;

        Ok(Self { 
            path: path.to_path_buf(),
            app_bundle: app_bundle.bundle_dir().clone(),
            stage_dir,
        })
    }

    pub async fn apply(&self) -> Result<(), Error> {
        let file_name = self.path.file_name()
            .and_then(|n| n.to_str())
            .ok_or(Error::TweakInvalidPath)?;

        if file_name.ends_with(".deb") {
            self.install_deb().await?;
        } else if file_name.ends_with(".framework") {
            self.install_framework(&self.path).await?;
        } else if file_name.ends_with(".bundle") {
            self.install_bundle(&self.path).await?;
        } else if file_name.ends_with(".dylib") {
            self.install_dylib(&self.path).await?;
        }

        // // Cleanup
        // tokio::fs::remove_dir_all(&self.stage_dir).await.ok();

        Ok(())
    }

    async fn install_deb(&self) -> Result<(), Error> {
        use decompress::ExtractOpts;
        
        let extract_dir = self.stage_dir.join("deb_contents");
        tokio::fs::create_dir_all(&extract_dir).await?;
        
        let ar_extract_dir = self.stage_dir.join("ar_contents");
        tokio::fs::create_dir_all(&ar_extract_dir).await?;
        
        let ar_path_sync = self.path.clone();
        let ar_extract_dir_sync = ar_extract_dir.clone();
        
        tokio::task::spawn_blocking(move || {
            decompress::decompress(
                &ar_path_sync,
                &ar_extract_dir_sync,
                &ExtractOpts {
                    strip: 0,
                    detect_content: false,
                    filter: Box::new(|_: &std::path::Path| true),
                    map: Box::new(|p| std::borrow::Cow::Borrowed(p)),
                }
            )
        }).await
            .ok()
            .and_then(|r| r.ok())
            .ok_or_else(|| Error::TweakExtractionFailed("Failed to extract .ar archive".to_string()))?;
        
        for archive_name in ["data.tar.lzma", "data.tar.gz", "data.tar.xz", "data.tar.bz2", "data.tar"] {
            let data_path = ar_extract_dir.join(archive_name);
            if data_path.exists() {
                let extract_dir_sync = extract_dir.clone();
                let data_path_sync = data_path.clone();
                
                tokio::task::spawn_blocking(move || {
                    decompress::decompress(&data_path_sync, &extract_dir_sync, &ExtractOpts {
                        strip: 0,
                        detect_content: false,
                        filter: Box::new(|_: &std::path::Path| true),
                        map: Box::new(|p| std::borrow::Cow::Borrowed(p)),
                    }
                )
                }).await
                    .map_err(|e| Error::TweakExtractionFailed(format!("Failed to extract data.tar: {}", e)))?
                    .map_err(|e| Error::TweakExtractionFailed(format!("Failed to extract data.tar: {}", e)))?;
                
                break;
            }
        }
        
        self.scan_and_install(&extract_dir).await
    }

    async fn scan_and_install(&self, root: &Path) -> Result<(), Error> {
        let search_paths = [
            "Library/MobileSubstrate/DynamicLibraries",
            "usr/lib",
            "Library/Frameworks",
            "Library/Application Support",
            "var/jb/Library/MobileSubstrate/DynamicLibraries",
            "var/jb/usr/lib",
            "var/jb/Library/Frameworks",
            "var/jb/Library/Application Support",
        ];

        for search_path in search_paths {
            let dir = root.join(search_path);
            if dir.exists() {
                self.scan_directory(&dir).await?;
            }
        }

        Ok(())
    }

    async fn scan_directory(&self, dir: &Path) -> Result<(), Error> {
        use futures::future::BoxFuture;
        
        fn scan_recursive<'a>(tweak: &'a Tweak, dir: &'a Path) -> BoxFuture<'a, Result<(), Error>> {
            Box::pin(async move {
                let mut entries = tokio::fs::read_dir(dir).await?;
                
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    let metadata = tokio::fs::symlink_metadata(&path).await?;
                    
                    if metadata.is_symlink() {
                        continue;
                    }
                    
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if path.is_file() && name.ends_with(".dylib") {
                            tweak.install_dylib(&path).await?;
                        } else if path.is_dir() {
                            if name.ends_with(".framework") {
                                tweak.install_framework(&path).await?;
                            } else if name.ends_with(".bundle") {
                                tweak.install_bundle(&path).await?;
                            } else {
                                // Recursively scan subdirectories
                                scan_recursive(tweak, &path).await?;
                            }
                        }
                    }
                }
                
                Ok(())
            })
        }
        
        scan_recursive(self, dir).await
    }

    async fn install_dylib(&self, dylib_path: &Path) -> Result<(), Error> {
        let frameworks_dir = self.app_bundle.join("Frameworks");
        tokio::fs::create_dir_all(&frameworks_dir).await?;
        
        let dylib_name = dylib_path.file_name().ok_or(Error::TweakInvalidPath)?;
        let dest = frameworks_dir.join(dylib_name);
        
        tokio::fs::copy(dylib_path, &dest).await?;

        self.inject_dylib(&dest).await
    }

    async fn install_framework(&self, framework_path: &Path) -> Result<(), Error> {
        let frameworks_dir = self.app_bundle.join("Frameworks");
        tokio::fs::create_dir_all(&frameworks_dir).await?;
        
        let framework_name = framework_path.file_name().ok_or(Error::TweakInvalidPath)?;
        let dest = frameworks_dir.join(framework_name);
        
        copy_dir_recursively(&framework_path, &dest).await?;
        
        // Inject the framework's main dylib
        if let Some(exec_name) = framework_name.to_str().and_then(|n| n.strip_suffix(".framework")) {
            let exec_path = dest.join(exec_name);
            if exec_path.exists() {
                self.inject_dylib(&exec_path).await?;
            }
        }

        Ok(())
    }

    async fn install_bundle(&self, bundle_path: &Path) -> Result<(), Error> {
        let bundle_name = bundle_path.file_name().ok_or(Error::TweakInvalidPath)?;
        let dest = self.app_bundle.join(bundle_name);
        
        copy_dir_recursively(bundle_path, &dest).await
    }

    async fn inject_dylib(&self, dylib_path: &Path) -> Result<(), Error> {
        let bundle = Bundle::new(&self.app_bundle)?;
        let executable_name = bundle.get_executable()
            .ok_or(Error::BundleInfoPlistMissing)?;
        
        let executable_path = self.app_bundle.join(&executable_name);
        if !executable_path.exists() {
            return Err(Error::BundleInfoPlistMissing);
        }

        // Calculate @executable_path relative path
        let relative = dylib_path.strip_prefix(&self.app_bundle)
            .map_err(|_| Error::TweakInvalidPath)?;
        
        let mut inject_path = format!("@executable_path/{}", relative.display());
        
        // Handle CydiaSubstrate → ElleKit compatibility
        // inject_path = inject_path.replace(
        //     "/Library/Frameworks/CydiaSubstrate.framework/CydiaSubstrate",
        //     "@rpath/CydiaSubstrate.framework/CydiaSubstrate"
        // );
        
        // TODO: Implement actual dylib injection into Mach-O binary
        // let _ = (executable_path, inject_path);

        println!("Injecting {:?} into {:?}", inject_path, executable_path.display());

        Ok(())
    }
}
