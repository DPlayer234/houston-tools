/// Compiles Windows resources files and instructs Cargo to link them.
///
/// Uses the [`winresource`] crate.
pub fn embed_windows_resources() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        let res = winresource::WindowsResource::new();
        if let Err(why) = res.compile() {
            println!("cargo::warning=failed to add windows resources to exe: {why}")
        }
    }
}
