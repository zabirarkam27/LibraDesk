use std::{
    fs,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use crate::scan::files::reparse;

use super::{pdf::Source, repository::CachedCover};

pub const MAX_IMAGE_BYTES: usize = 512 * 1024;
const MAX_RANGE: i64 = 256 * 1024;
pub const MAX_COVER_SOURCE: i64 = 256 * 1024 * 1024;

pub fn read_range(source: &Source, offset: i64, length: i64) -> io::Result<Vec<u8>> {
    if source.bytes > MAX_COVER_SOURCE
        || offset < 0
        || !(1..=MAX_RANGE).contains(&length)
        || offset
            .checked_add(length)
            .is_none_or(|end| end > source.bytes)
    {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let (mut file, _) = source.open()?;
    file.seek(SeekFrom::Start(offset as u64))?;
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub fn cache_path(root: &Path, id: i64, revision: i64) -> io::Result<PathBuf> {
    let directory = root.join("thumbnails");
    for path in [root, directory.as_path()] {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir() || reparse(&metadata) {
            return Err(io::ErrorKind::InvalidInput.into());
        }
    }
    Ok(directory.join(format!("{id}-{revision}.png")))
}

fn check_png(bytes: &[u8]) -> io::Result<(i64, i64)> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut reader = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let (width, height) = (reader.info().width, reader.info().height);
    if !(1..=240).contains(&width) || !(1..=320).contains(&height) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let size = reader
        .output_buffer_size()
        .ok_or(io::ErrorKind::InvalidData)?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok((i64::from(width), i64::from(height)))
}

pub fn save_png(root: &Path, source: &Source, bytes: &[u8]) -> io::Result<CachedCover> {
    let (width, height) = check_png(bytes)?;
    let path = cache_path(root, source.id, source.revision)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(&path).map_err(io::Error::other)?;
    Ok(CachedCover {
        relative_path: format!("{}-{}.png", source.id, source.revision),
        width,
        height,
        bytes: bytes.len() as i64,
    })
}

pub fn load_png(root: &Path, id: i64, revision: i64) -> io::Result<Vec<u8>> {
    let path = cache_path(root, id, revision)?;
    // Windows can redirect a packaged app's Local AppData to its package cache.
    // Use the checked AppData directories and generated name, rejecting reparse files on open.
    let file = crate::scan::files::open_regular_readonly(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    check_png(&bytes)?;
    Ok(bytes)
}
