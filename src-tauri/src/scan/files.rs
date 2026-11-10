use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, PartialEq, Eq)]
pub struct Stamp {
    pub bytes: i64,
    pub modified_at: i64,
    pub token: String,
}

pub enum Pdf {
    Valid(Stamp),
    Invalid(i64),
    Inaccessible,
}

pub fn reparse(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub fn root_key(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if !path.is_absolute() || !metadata.is_dir() || reparse(&metadata) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let path = fs::canonicalize(path)?;
    key(&path)?;
    Ok(path)
}

pub fn key(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::ErrorKind::InvalidData.into())
}

pub fn display(path: &str) -> String {
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
}

fn check_path(root: &Path, path: &Path) -> io::Result<Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !path.starts_with(root) || reparse(&metadata) || fs::canonicalize(path)? != path {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(metadata)
}

pub fn read_directory(root: &Path, path: &Path) -> io::Result<fs::ReadDir> {
    if !check_path(root, path)?.is_dir() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    fs::read_dir(path)
}

pub fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn epoch_nanos(time: SystemTime) -> i128 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    }
}

#[cfg(windows)]
fn opened_path(file: &File) -> io::Result<PathBuf> {
    use std::{
        ffi::OsString,
        os::windows::{ffi::OsStringExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;
    let mut buffer = vec![0u16; 512];
    loop {
        // The handle and writable UTF-16 buffer are valid for the duration of the call.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        } as usize;
        if length == 0 {
            return Err(io::Error::last_os_error());
        }
        if length < buffer.len() {
            return Ok(PathBuf::from(OsString::from_wide(&buffer[..length])));
        }
        buffer.resize(length + 1, 0);
    }
}

fn open_checked(root: &Path, path: &Path) -> io::Result<File> {
    if !check_path(root, path)?.is_file() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || reparse(&metadata) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    #[cfg(windows)]
    if opened_path(&file)? != path {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(file)
}

fn inspect(root: &Path, path: &Path) -> io::Result<Pdf> {
    let mut file = open_checked(root, path)?;
    let metadata = file.metadata()?;
    let bytes = i64::try_from(metadata.len()).map_err(io::Error::other)?;
    let mut header = [0u8; 8];
    if let Err(error) = file.read_exact(&mut header) {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            return Ok(Pdf::Invalid(bytes));
        }
        return Err(error);
    }
    if &header[..5] != b"%PDF-"
        || !header[5].is_ascii_digit()
        || header[6] != b'.'
        || !header[7].is_ascii_digit()
    {
        return Ok(Pdf::Invalid(bytes));
    }
    let modified = epoch_nanos(metadata.modified()?);
    Ok(Pdf::Valid(Stamp {
        bytes,
        modified_at: i64::try_from(modified.div_euclid(1_000_000)).map_err(io::Error::other)?,
        token: modified.to_string(),
    }))
}

pub fn validate(root: &Path, path: &Path) -> Pdf {
    inspect(root, path).unwrap_or(Pdf::Inaccessible)
}
