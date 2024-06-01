use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::io;

#[cfg(target_os = "windows")]
use winapi::um::winbase::GetFileAttributesA;
#[cfg(target_os = "windows")]
use winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;

#[cfg(target_os = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
use libc::{stat, UF_HIDDEN};

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
        // Check if the file name starts with a dot
        let file_name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
        if file_name.to_str().map_or(false, |s| s.starts_with('.')) {
            return Ok(true);
        }

        // Check the UF_HIDDEN attribute
        let mut file_stat: stat = unsafe { std::mem::zeroed() };
        let path_c = CString::new(path.to_str().unwrap())?;
        let ret = unsafe { libc::stat(path_c.as_ptr(), &mut file_stat) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        return Ok((file_stat.st_flags & UF_HIDDEN as u32) != 0);
    }

    #[cfg(target_os = "linux")]
    {
        let file_name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
        return Ok(file_name.to_str().map_or(false, |s| s.starts_with('.')));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        return Err(io::Error::new(io::ErrorKind::Other, "Unsupported OS"));
    }
}
