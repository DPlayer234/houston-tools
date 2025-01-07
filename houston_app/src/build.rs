//! Provides constants about the build environment.

/// The cargo package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The git hash of the repo at the time of the build.
///
/// If the git hash cannot be determined (f.e. because git is unavailable), this
/// is instead "&lt;unknown&gt;". As such, this should be treated as a
/// display-only value.
pub const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(git_hash) => git_hash,
    None => "<unknown>",
};
