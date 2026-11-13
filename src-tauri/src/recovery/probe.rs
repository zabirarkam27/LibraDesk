use super::repository::RegisteredFile;
use crate::{
    metadata::{analyze, Analysis, Source},
    scan::files::{self, Stamp},
};
use std::{
    io::{self, Read},
    path::Path,
};

pub struct Observation {
    pub status: &'static str,
    pub stamp: Option<Stamp>,
    pub analysis: Option<Analysis>,
    _read_guard: Option<std::fs::File>,
}
fn unavailable(error: &io::Error) -> Observation {
    Observation {
        status: if matches!(error.raw_os_error(), Some(15 | 21 | 53 | 55 | 64 | 67))
            || matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) {
            "missing"
        } else {
            "inaccessible"
        },
        stamp: None,
        analysis: None,
        _read_guard: None,
    }
}
pub fn inspect(file: &RegisteredFile) -> Observation {
    let path = Path::new(&file.source.path);
    let result = (|| -> io::Result<_> {
        let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
        let mut handle = files::open_checked(parent, path)?;
        let stamp = files::stamp(&handle)?;
        let mut header = [0; 8];
        if handle.read_exact(&mut header).is_err()
            || &header[..5] != b"%PDF-"
            || !header[5].is_ascii_digit()
            || header[6] != b'.'
            || !header[7].is_ascii_digit()
        {
            return Ok(Observation {
                status: "invalid",
                stamp: Some(stamp),
                analysis: None,
                _read_guard: Some(handle),
            });
        }
        let changed = stamp.bytes != file.source.bytes
            || stamp.modified_at != file.source.modified_at
            || file
                .source
                .token
                .as_ref()
                .is_some_and(|token| token != &stamp.token);
        let analysis = if changed {
            let current = Source {
                id: file.source.id,
                revision: file.source.revision,
                path: file.source.path.clone(),
                bytes: stamp.bytes,
                modified_at: stamp.modified_at,
                token: Some(stamp.token.clone()),
            };
            Some(analyze(&current)?)
        } else {
            None
        };
        Ok(Observation {
            status: "available",
            stamp: Some(stamp),
            analysis,
            _read_guard: Some(handle),
        })
    })();
    result.unwrap_or_else(|error| unavailable(&error))
}
pub fn verified_handle(source: &Source, expected: &[u8]) -> io::Result<std::fs::File> {
    let (mut handle, stamp) = source.open()?;
    let digest = crate::metadata::hash(&mut handle)?;
    if digest.as_slice() != expected || files::stamp(&handle)? != stamp {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(handle)
}
pub fn replacement(path: &Path) -> io::Result<(Source, Analysis)> {
    if !path.is_absolute() || !files::is_pdf(path) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if files::reparse(&metadata) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let path = std::fs::canonicalize(path)?;
    let handle = files::open_checked(path.parent().ok_or(io::ErrorKind::InvalidInput)?, &path)?;
    let stamp = files::stamp(&handle)?;
    let source = Source {
        id: 0,
        revision: 1,
        path: files::key(&path)?.into(),
        bytes: stamp.bytes,
        modified_at: stamp.modified_at,
        token: Some(stamp.token),
    };
    let registered = RegisteredFile {
        source: source.clone(),
        hash: None,
        availability: "available".into(),
    };
    if inspect(&registered).status != "available" {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let (mut checked, _) = source.open()?;
    use std::io::{Seek, SeekFrom};
    let tail_length = source.bytes.min(4096);
    checked.seek(SeekFrom::End(-tail_length))?;
    let mut tail = Vec::with_capacity(tail_length as usize);
    checked.read_to_end(&mut tail)?;
    if !tail.windows(5).any(|window| window == b"%%EOF") {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let analysis = analyze(&source)?;
    if source.bytes <= 32 * 1024 * 1024 && analysis.metadata.is_none() {
        checked.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::with_capacity(source.bytes as usize);
        checked.read_to_end(&mut bytes)?;
        let valid = std::panic::catch_unwind(|| {
            lopdf::Document::load_mem(&bytes)
                .is_ok_and(|doc| doc.is_encrypted() || doc.page_iter().next().is_some())
        })
        .unwrap_or(false);
        if !valid {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    Ok((source, analysis))
}
