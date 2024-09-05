// note: this script is shared by `houston_app` and `azur_lane_data_collector`
// if you make any changes, apply them to the other one also!

fn main() {
    println!("cargo::rerun-if-changed=Cargo.toml");

    #[cfg(windows)]
    windows_resources();
}

#[cfg(windows)]
fn windows_resources() {
    let res = winres::WindowsResource::new();
    if let Err(why) = res.compile() {
        println!("cargo::warning=failed to add windows resources to exe: {why}")
    }
}
