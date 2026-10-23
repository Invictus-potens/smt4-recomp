// Embeds the icon and version info into the launcher's .exe on Windows.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Shin Megami Tensei IV for PC");
        res.set("FileDescription", "smt4-recomp launcher");
        if let Err(error) = res.compile() {
            // a launcher without an icon still works
            println!("cargo:warning=no icon embedded: {error}");
        }
    }
}
