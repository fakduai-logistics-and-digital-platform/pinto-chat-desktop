//! Embeds the app icon and version info into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/icon.ico");
    // Baked into env::DEFAULT_API; rebuild when CI changes it.
    println!("cargo:rerun-if-env-changed=PINTO_BUILD_API_BASE_URL");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/icon.ico")
            .set("ProductName", "Pinto Chat")
            .set("FileDescription", "Pinto Chat");
        if let Err(err) = res.compile() {
            println!("cargo:warning=could not embed Windows icon: {err}");
        }
    }
}
