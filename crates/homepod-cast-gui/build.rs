fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app_icon.ico");
        res.set("ProductName", "Win2Pod Bridge");
        res.set("FileDescription", "Win2Pod Bridge");
        res.set("ProductVersion", "0.1.15.0");
        res.set("FileVersion", "0.1.15.0");
        res.set("LegalCopyright", "Copyright (c) 2024-2026 mesmerli");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}
