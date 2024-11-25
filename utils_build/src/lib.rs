use std::env;
use std::process::Command;

mod ensure;

/// Compiles Windows resources files and instructs Cargo to link them.
///
/// Uses the [`winresource`] crate.
pub fn embed_windows_resources() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        println!("cargo::rerun-if-changed=Cargo.toml");

        let res = winresource::WindowsResource::new();
        if let Err(why) = res.compile() {
            println!("cargo::warning=failed to add windows resources to exe: {why}");
        }
    }
}

/// Includes the git commit hash for the directory.
///
/// This sets the `GIT_HASH` environment variable for the compilation of the crate itself.
/// If this fails, it prints a warning.
///
/// Access it via [`option_env!`]:
///
/// ```no_run
/// match option_env!("GIT_HASH") {
///     Some(git_hash) => println!("git commit is {git_hash}"),
///     None => println!("unknown git commit"),
/// }
/// ```
///
/// If you're _really_ sure that this can't fail, you may also use [`env!`].
pub fn include_git_commit_hash() {
    // Based on <https://stackoverflow.com/a/44407625>
    println!("cargo::rerun-if-changed=.git/HEAD");

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output();

    let output = ensure::ok_or!(output, why => "cannot find git commit hash: {why}");
    ensure::or!(output.status.success(), "`git rev-parse HEAD` exited with non-success error code");

    let git_hash = String::from_utf8(output.stdout);
    let git_hash = ensure::ok_or!(git_hash, _ => "git commit hash is invalid utf-8");
    println!("cargo::rustc-env=GIT_HASH={}", git_hash);
}
