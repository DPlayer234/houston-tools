pub fn embed_windows_resources() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        // compiling and linking windows resources doesn't seem to really work under linux
        #[cfg(windows)]
        windows_resources();

        #[cfg(not(windows))]
        println!("cargo::warning=skipping adding windows resources to exe, compile on windows");
    }
}

#[cfg(windows)]
fn windows_resources() {
    let res = winres::WindowsResource::new();
    if let Err(why) = res.compile() {
        println!("cargo::warning=failed to add windows resources to exe: {why}")
    }
}
