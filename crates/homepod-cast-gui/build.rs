fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app_icon.ico");
        res.set("ProductName", "Win2Pod Bridge");
        res.set("FileDescription", "Win2Pod Bridge AirPlay 2 Sender");
        res.set("LegalCopyright", "Copyright (c) 2026 mesmerli & Open Source Contributors");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}
