#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
    embed_windows_manifest(&pkg_name);
}

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn embed_windows_manifest(name: &str) {
    use embed_manifest::manifest::{ActiveCodePage, Setting, SupportedOS::*};
    use embed_manifest::{embed_manifest, new_manifest};

    {
        let manifest = new_manifest(name)
            .supported_os(Windows7..=Windows10)
            .active_code_page(ActiveCodePage::Utf8)
            .heap_type(embed_manifest::manifest::HeapType::SegmentHeap)
            .dpi_awareness(embed_manifest::manifest::DpiAwareness::Unaware)
            .long_path_aware(Setting::Enabled);

        embed_manifest(manifest).unwrap();
    }

    {
        let mut res = winres::WindowsResource::new();
        res.set_resource_file("../../package/windows/custom.rc");
        res.compile().unwrap();
    }
}
