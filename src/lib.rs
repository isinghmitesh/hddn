use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

/// Checks if the specified file is hidden.
///
/// A name that starts with a dot is hidden on every platform. On macOS a file
/// with the `UF_HIDDEN` flag (`chflags hidden`) is hidden too, and on Windows
/// a file with `FILE_ATTRIBUTE_HIDDEN` (`attrib +h`).
///
/// # Arguments
///
/// * `path` - A reference to a `PathBuf` that holds the path of the file.
///
/// # Returns
///
/// * `Ok(true)` if the file is hidden.
/// * `Ok(false)` if the file is not hidden.
/// * `Err` if there is an error accessing the file metadata, or if `path` has
///   no file name or contains a NUL character (`ErrorKind::InvalidInput`).
///   A NUL in a parent directory counts, as does a NUL in a name that starts
///   with a dot. That check does not read the disk.
// Takes `&PathBuf` rather than `&Path` to keep the 0.1 signature.
#[allow(clippy::ptr_arg)]
pub fn is_hidden(path: &PathBuf) -> io::Result<bool> {
    // A real directory entry cannot contain a NUL. Reject it before the
    // dot-name return, which would otherwise report `.a\0b` or `a\0b/.hidden`
    // as hidden. Linux has no later call that would reject it either.
    if path_contains_nul(path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains a NUL character",
        ));
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
    if starts_with_dot(file_name) {
        return Ok(true);
    }
    has_hidden_flag(path)
}

// Raw bytes on Unix, so a non-UTF-8 name still counts. UTF-16 units on
// Windows, so an unpaired surrogate still counts.
#[cfg(unix)]
fn path_contains_nul(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().contains(&0)
}

#[cfg(windows)]
fn path_contains_nul(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().any(|unit| unit == 0)
}

#[cfg(not(any(unix, windows)))]
fn path_contains_nul(path: &Path) -> bool {
    path.as_os_str().to_string_lossy().contains('\0')
}

// The checks below read the name as the OS stores it, so a name that isn't
// valid UTF-8 (possible on Unix) or valid UTF-16 (possible on Windows) works.

#[cfg(unix)]
fn starts_with_dot(name: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    name.as_bytes().first() == Some(&b'.')
}

#[cfg(windows)]
fn starts_with_dot(name: &OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt;
    name.encode_wide().next() == Some(u16::from(b'.'))
}

#[cfg(not(any(unix, windows)))]
fn starts_with_dot(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

#[cfg(target_os = "macos")]
fn has_hidden_flag(path: &Path) -> io::Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // is_hidden already rejected an interior NUL. CString::new still fails
    // if one is present, and `?` turns that into InvalidInput.
    let path_c = CString::new(path.as_os_str().as_bytes())?;
    let mut file_stat: libc::stat = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::stat(path_c.as_ptr(), &mut file_stat) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((file_stat.st_flags & libc::UF_HIDDEN) != 0)
}

#[cfg(target_os = "windows")]
fn has_hidden_flag(path: &Path) -> io::Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::{GetFileAttributesW, INVALID_FILE_ATTRIBUTES};
    use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

    // The W call takes the path as UTF-16. The A call read it in the ANSI
    // code page, so any name with a non-ASCII character failed with NotFound.
    // is_hidden already rejected an interior NUL, so the 0 below is only the
    // terminator GetFileAttributesW requires.
    let mut path_w: Vec<u16> = path.as_os_str().encode_wide().collect();
    path_w.push(0);
    let attributes = unsafe { GetFileAttributesW(path_w.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(io::Error::last_os_error());
    }
    Ok((attributes & FILE_ATTRIBUTE_HIDDEN) != 0)
}

// Linux has no hidden attribute, only the leading dot checked above.
// is_hidden rejects a NUL before this runs.
#[cfg(target_os = "linux")]
fn has_hidden_flag(_path: &Path) -> io::Result<bool> {
    Ok(false)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn has_hidden_flag(_path: &Path) -> io::Result<bool> {
    Err(io::Error::new(io::ErrorKind::Other, "Unsupported OS"))
}

#[cfg(test)]
mod tests {
    use super::is_hidden;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// An empty folder under the system temp folder, unique to one test and
    /// removed when the test ends.
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn scratch_dir() -> Scratch {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("hddn-test-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    #[test]
    fn dot_name_is_hidden_without_touching_the_disk() {
        let dir = scratch_dir();
        assert!(is_hidden(&dir.join(".missing")).unwrap());
        assert!(is_hidden(&dir.join(".DS_Store")).unwrap());
    }

    #[test]
    fn dot_file_and_dot_folder_are_hidden() {
        let dir = scratch_dir();
        fs::write(dir.join(".env"), b"x").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        assert!(is_hidden(&dir.join(".env")).unwrap());
        assert!(is_hidden(&dir.join(".git")).unwrap());
    }

    #[test]
    fn plain_file_and_folder_are_not_hidden() {
        let dir = scratch_dir();
        fs::write(dir.join("report.pdf"), b"x").unwrap();
        fs::create_dir(dir.join("Photos")).unwrap();
        assert!(!is_hidden(&dir.join("report.pdf")).unwrap());
        assert!(!is_hidden(&dir.join("Photos")).unwrap());
    }

    #[test]
    fn path_without_a_file_name_is_invalid_input() {
        let err = is_hidden(&PathBuf::from("/")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    // These names cannot exist on disk. The error comes from the path bytes,
    // on macOS, Linux and Windows, before any metadata call.
    #[test]
    fn nul_in_the_name_or_a_parent_is_invalid_input() {
        let dir = scratch_dir();
        let paths = [
            dir.join("a\0b"),
            dir.join(".a\0b"),
            dir.join("a\0b").join(".hidden"),
        ];
        for path in &paths {
            let err = is_hidden(path).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "expected InvalidInput for {path:?}"
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn missing_file_is_not_found() {
        let err = is_hidden(&scratch_dir().join("missing.txt")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn non_ascii_names_are_read() {
        let dir = scratch_dir();
        fs::create_dir(dir.join("Fotos de café")).unwrap();
        fs::write(dir.join("रिपोर्ट.pdf"), b"x").unwrap();
        fs::write(dir.join("日本語.txt"), b"x").unwrap();
        assert!(!is_hidden(&dir.join("Fotos de café")).unwrap());
        assert!(!is_hidden(&dir.join("रिपोर्ट.pdf")).unwrap());
        assert!(!is_hidden(&dir.join("日本語.txt")).unwrap());
    }

    #[cfg(unix)]
    mod unix {
        use super::{is_hidden, scratch_dir};
        use std::ffi::OsStr;
        use std::io;
        use std::os::unix::ffi::OsStrExt;

        #[test]
        fn non_utf8_dot_name_is_hidden() {
            let path = scratch_dir().join(OsStr::from_bytes(b".caf\xe9"));
            assert!(is_hidden(&path).unwrap());
        }

        // APFS refuses to create a name that isn't UTF-8, so this checks the
        // path handling: the name reaches stat instead of panicking.
        #[cfg(target_os = "macos")]
        #[test]
        fn missing_non_utf8_name_is_not_found() {
            let path = scratch_dir().join(OsStr::from_bytes(b"caf\xe9"));
            let err = is_hidden(&path).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }

        #[test]
        fn nul_in_a_non_utf8_dot_name_is_invalid_input() {
            let path = scratch_dir().join(OsStr::from_bytes(b".\xff\0"));
            let err = is_hidden(&path).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn chflags_hidden_file_is_hidden() {
        let dir = scratch_dir();
        let path = dir.join("Icon.png");
        std::fs::write(&path, b"x").unwrap();
        assert!(!is_hidden(&path).unwrap());
        let status = std::process::Command::new("chflags")
            .arg("hidden")
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(is_hidden(&path).unwrap());
    }

    #[cfg(target_os = "windows")]
    mod windows {
        use super::{is_hidden, scratch_dir};
        use std::fs;
        use std::path::Path;
        use std::process::Command;

        fn attrib_hidden(path: &Path) {
            let status = Command::new("attrib").arg("+h").arg(path).status().unwrap();
            assert!(status.success());
        }

        #[test]
        fn attrib_hidden_file_and_folder_are_hidden() {
            let dir = scratch_dir();
            fs::write(dir.join("desktop.ini"), b"x").unwrap();
            fs::create_dir(dir.join("Cache")).unwrap();
            assert!(!is_hidden(&dir.join("desktop.ini")).unwrap());
            attrib_hidden(&dir.join("desktop.ini"));
            attrib_hidden(&dir.join("Cache"));
            assert!(is_hidden(&dir.join("desktop.ini")).unwrap());
            assert!(is_hidden(&dir.join("Cache")).unwrap());
        }

        #[test]
        fn attrib_hidden_non_ascii_file_is_hidden() {
            let dir = scratch_dir();
            let path = dir.join("café.txt");
            fs::write(&path, b"x").unwrap();
            attrib_hidden(&path);
            assert!(is_hidden(&path).unwrap());
        }
    }
}
