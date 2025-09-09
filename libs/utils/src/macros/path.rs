/// Joins multiple path segments into a [`PathBuf`].
///
/// An extension may be specified at the end. If specified, it will override the
/// extension of the last segment.
///
/// This is equivalent to creating a [`PathBuf`] from the first segment and then
/// repeatedly calling [`push`], then finishing with [`set_extension`] if an
/// extension is specified.
///
/// # Note
///
/// The use of [`set_extension`] may lead to some unexpected behavior:
///
/// - If the last component already has an extension, the extension will be
///   replaced.
/// - If the last component is `..`, no extension will be set.
///
/// See the docs for [`set_extension`] for further details.
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
/// [`set_extension`]: std::path::PathBuf::set_extension
#[macro_export]
macro_rules! join_path {
    ($root:expr $(,$parts:expr)* $(,)? $(; $ext:expr)?) => {{
        let mut path = ::std::path::PathBuf::from($root);
        $( path.push($parts); )*
        $( path.set_extension($ext); )?
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
            assert_eq!(&path, Path::new(r#"C:\Windows\System32\notepad.exe"#));
        } else {
            let path = join_path!("/home", "root", "notes"; "txt");
            assert_eq!(&path, Path::new(r#"/home/root/notes.txt"#));
        }
    }
}
