fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Specter Master CLI");
        res.set("FileDescription", "Specter - Autonomous Browser Automation & Distributed Mesh Runtime");
        res.set("CompanyName", "TuQuet");
        res.set("LegalCopyright", "Copyright (c) 2026 TuQuet. MIT License.");
        res.set("ProductVersion", "1.2.0.0");
        res.set("FileVersion", "1.2.0.0");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile windows resource: {}", e);
        }
    }
}
