use std::ffi::CString;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use winapi::um::fileapi::GetFileAttributesA;
#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
use libc::{stat, UF_HIDDEN};
#[cfg(target_os = "unix")]
use std::os::unix::fs::MetadataExt;

/// Checks if the specified file is hidden.
///
/// # Arguments
///
/// * `path` - A reference to a `PathBuf` that holds the path of the file.
///
/// # Returns
///
/// * `Ok(true)` if the file is hidden.
/// * `Ok(false)` if the file is not hidden.
/// * `Err` if there is an error accessing the file metadata.
pub fn is_hidden(path: &PathBuf) -> io::Result<bool> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
    if file_name.to_str().map_or(false, |s| s.starts_with('.')) {
        return Ok(true);
    }
    #[cfg(target_os = "windows")]
    {
        let path_c = CString::new(path.to_str().unwrap())?;
        let attributes = unsafe { GetFileAttributesA(path_c.as_ptr()) };
        if attributes == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        return Ok((attributes & FILE_ATTRIBUTE_HIDDEN) != 0);
    }

    #[cfg(target_os = "macos")]
    {
        let mut file_stat: stat = unsafe { std::mem::zeroed() };
        let path_c = CString::new(path.to_str().unwrap())?;
        let ret = unsafe { libc::stat(path_c.as_ptr(), &mut file_stat) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        return Ok((file_stat.st_flags & UF_HIDDEN as u32) != 0);
    }

    // Linux has no hidden attribute, only the leading dot checked above.
    #[cfg(target_os = "linux")]
    {
        return Ok(false);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        return Err(io::Error::new(io::ErrorKind::Other, "Unsupported OS"));
    }
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
        fn interior_nul_is_invalid_input() {
            let path = scratch_dir().join(OsStr::from_bytes(b"a\0b"));
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
        use std::io;
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

        #[test]
        fn interior_nul_is_invalid_input() {
            let err = is_hidden(&scratch_dir().join("a\0b")).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        }
    }
}
