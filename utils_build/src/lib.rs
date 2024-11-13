use std::env;
use std::process::Command;

/// Compiles Windows resources files and instructs Cargo to link them.
///
/// Uses the [`winresource`] crate.
pub fn embed_windows_resources() {
    println!("cargo::rerun-if-changed=Cargo.toml");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        let res = winresource::WindowsResource::new();
        if let Err(why) = res.compile() {
            println!("cargo::warning=failed to add windows resources to exe: {why}");
        }
    }
}

pub fn include_git_commit_hash() {
    // Based on <https://stackoverflow.com/a/44407625>
    println!("cargo::rerun-if-changed=.git/HEAD");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output();

    match output {
        Ok(output) => {
            match String::from_utf8(output.stdout) {
                Ok(git_hash) => println!("cargo::rustc-env=GIT_HASH={}", git_hash),
                Err(_) => println!("cargo::warning=git commit hash is invalid utf-8"),
            }
        }
        Err(why) => println!("cargo::warning=cannot find git commit hash: {why}"),
    };
}
