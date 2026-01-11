fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../package/windows/icon.ico");
        res.compile().unwrap();
    }
}
