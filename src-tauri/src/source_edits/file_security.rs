use super::Result;
use std::path::Path;
#[cfg(windows)]
pub fn copy_dacl(source: &Path, target: &Path) -> Result<()> {
    use std::{io, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Security::{
        GetFileSecurityW, GetSecurityDescriptorControl, SetFileSecurityW,
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED,
        UNPROTECTED_DACL_SECURITY_INFORMATION,
    };
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut needed = 0u32;
    // Ask Windows for the self-relative security descriptor size.
    unsafe {
        GetFileSecurityW(
            source_wide.as_ptr(),
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            0,
            &mut needed,
        )
    };
    if needed == 0 || needed > 1024 * 1024 {
        return Err(io::Error::last_os_error().into());
    }
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    let descriptor = buffer.as_mut_ptr().cast();
    // The u64 buffer is aligned and large enough for the requested descriptor.
    if unsafe {
        GetFileSecurityW(
            source_wide.as_ptr(),
            DACL_SECURITY_INFORMATION,
            descriptor,
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error().into());
    }
    let mut control = 0u16;
    let mut revision = 0u32;
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let inheritance = if control & SE_DACL_PROTECTED != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // Apply access rules before writing any document bytes to the temporary file.
    if unsafe {
        SetFileSecurityW(
            target_wide.as_ptr(),
            DACL_SECURITY_INFORMATION | inheritance,
            descriptor,
        )
    } == 0
    {
        return Err(io::Error::last_os_error().into());
    }
    copy_zone(source, target)?;
    Ok(())
}
#[cfg(windows)]
fn copy_zone(source: &Path, target: &Path) -> Result<()> {
    use std::io::{Read, Write};
    let mut origin = source.as_os_str().to_os_string();
    origin.push(":Zone.Identifier");
    let file = match std::fs::File::open(origin) {
        Ok(file) => file,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || matches!(error.raw_os_error(), Some(1 | 50 | 123)) =>
        {
            return Ok(())
        }
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return super::fail("file_unavailable");
    }
    let mut destination = target.as_os_str().to_os_string();
    destination.push(":Zone.Identifier");
    let mut stream = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(destination)?;
    stream.write_all(&bytes)?;
    stream.sync_all()?;
    Ok(())
}
#[cfg(not(windows))]
pub fn copy_dacl(source: &Path, target: &Path) -> Result<()> {
    std::fs::set_permissions(target, std::fs::metadata(source)?.permissions())?;
    Ok(())
}
