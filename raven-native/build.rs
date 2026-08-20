fn main() {
    println!("cargo:rerun-if-changed=ui");
    println!("cargo:rerun-if-changed=app.manifest");
    slint_build::compile("ui/index.slint").unwrap();

    // Embed the app icon and metadata into the Windows executable
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app.ico");
        res.set_manifest_file("app.manifest");
        res.set("FileDescription", "Raven Notch");
        res.set("ProductName", "Raven Notch");
        res.set("LegalCopyright", "© 2025 Raven");
        res.compile().expect("Failed to compile Windows resources");
    }
}
