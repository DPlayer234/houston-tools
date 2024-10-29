fn main() {
    println!("cargo::rerun-if-changed=Cargo.toml");
    utils_build::embed_windows_resources();
}
