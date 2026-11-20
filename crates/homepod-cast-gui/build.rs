fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app_icon.ico");
        res.set("ProductName", "Win2Pod Bridge");
        res.set("FileDescription", "Win2Pod Bridge");
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.17".to_string());
        let version_with_zero = format!("{version}.0");
        res.set("ProductVersion", &version_with_zero);
        res.set("FileVersion", &version_with_zero);
        res.set("LegalCopyright", "Copyright (c) 2024-2026 mesmerli");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {e}");
        }
    }
}
