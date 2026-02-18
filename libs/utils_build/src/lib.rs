//! Shared utilities for build scripts.
#![warn(missing_docs)]

use std::env;
use std::path::PathBuf;
use std::process::Command;

mod ensure;

/// Compiles Windows resources files and instructs Cargo to link them.
///
/// Uses the [`winresource`] crate.
pub fn embed_windows_resources() {
    let target_os =
        env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS env var should be set");

    if target_os == "windows" {
        println!("cargo::rerun-if-changed=Cargo.toml");

        let res = winresource::WindowsResource::new();
        let output = res.compile();
        ensure::ok_or!(output, why => "failed to add windows resources to exe: {why}");
    }
}

/// Includes the git commit hash for the directory.
///
/// This sets the `GIT_HASH` environment variable for the compilation of the
/// crate itself. If this fails, it prints a warning.
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
    let output = Command::new("git")
        .args([
            "rev-parse",
            "HEAD",
            "--symbolic-full-name",
            "HEAD",
            "--show-toplevel",
        ])
        .output();

    let output = ensure::ok_or!(output, why => "cannot find git commit hash: {why}");
    ensure::or!(
        output.status.success(),
        "`git rev-parse` exited with non-success error code"
    );

    let output = String::from_utf8(output.stdout);
    let output = ensure::ok_or!(output, _ => "`git rev-parse` output is invalid utf-8");

    let mut lines = output.lines();
    let git_hash = ensure::some_or!(lines.next(), "could not find git commit hash");
    let git_ref = ensure::some_or!(lines.next(), "could not find git ref");
    let git_root = ensure::some_or!(lines.next(), "could not find git root directory");
    ensure::none_or!(lines.next(), _ => "unexpected `git rev-parse` output");

    println!("cargo::rustc-env=GIT_HASH={git_hash}");

    let mut git_dir = PathBuf::new();
    git_dir.push(git_root);
    git_dir.push(".git");

    let head_path = git_dir.join("HEAD");
    println!("cargo::rerun-if-changed={}", head_path.display());

    if git_ref != "HEAD" {
        let ref_path = git_dir.join(git_ref);
        println!("cargo::rerun-if-changed={}", ref_path.display());
    }
}
