fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/classcii.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resources: {e}");
        }
    }
}
