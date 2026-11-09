fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/favicon.ico");
        res.set("ProductName", "VampYTD");
        res.set("FileDescription", "VampYTD Downloader & Native Messaging Bridge");
        res.set("LegalCopyright", "Copyright (c) 2026 VampDued");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
        }
    }
}

