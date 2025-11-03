/// Joins multiple path segments into a [`PathBuf`].
///
/// An extension may be specified at the end. If specified, it will override the
/// extension of the last segment.
///
/// This is equivalent to creating a [`PathBuf`] from the first segment and then
/// repeatedly calling [`push`], then finishing with [`add_extension`] if an
/// extension is specified.
///
/// # Panics
///
/// Panics if an extension is provided and it contains a path separator.
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// let path = utils::join_path!("C:\\", "Windows", "System32", "notepad"; "exe");
/// # #[cfg(windows)]
/// assert_eq!(
///     &path,
///     Path::new(r#"C:\Windows\System32\notepad.exe"#)
/// );
/// ```
///
/// [`PathBuf`]: std::path::PathBuf
/// [`push`]: std::path::PathBuf::push
/// [`add_extension`]: std::path::PathBuf::add_extension
#[macro_export]
macro_rules! join_path {
    ($root:expr $(,$parts:expr)* $(,)? $(; $ext:expr)?) => {{
        let mut path = ::std::path::PathBuf::from($root);
        $( path.push($parts); )*
        $( path.add_extension($ext); )?
        path
    }};
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::join_path;

    #[test]
    fn join_os_path() {
        if cfg!(windows) {
            let path = join_path!("C:\\", "Windows", "System32", "notepad"; "exe");
            let eext = join_path!("C:\\", "Users", "Public", "useredit.exe"; "config");
            assert_eq!(&path, Path::new(r#"C:\Windows\System32\notepad.exe"#));
            assert_eq!(&eext, Path::new(r#"C:\Users\Public\useredit.exe.config"#));
        } else {
            let path = join_path!("/home", "root", "notes"; "txt");
            let eext = join_path!("/home", "root", "help.bin"; "conf");
            assert_eq!(&path, Path::new(r#"/home/root/notes.txt"#));
            assert_eq!(&eext, Path::new(r#"/home/root/help.bin.conf"#));
        }
    }
}
