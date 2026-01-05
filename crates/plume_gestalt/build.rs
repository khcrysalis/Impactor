fn main() {
    println!("cargo:rustc-link-lib=MobileGestalt");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rerun-if-changed=build.rs");
}
