use super::{fail, repository::Snapshot, xmp, Failure, Result};
use crate::{metadata::Source, scan::files};
use lopdf::{text_string, Dictionary, Document, IncrementalDocument, Object, Stream};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
pub const LIMIT: i64 = 256 * 1024 * 1024;
pub struct Prepared {
    pub temporary: tempfile::NamedTempFile,
    pub original: [u8; 32],
    pub digest: [u8; 32],
    pub token: String,
}
fn document_error(_: lopdf::Error) -> Failure {
    Failure {
        code: "pdf_unsupported",
    }
}
pub fn prepare(source: &Source, data: &Snapshot) -> Result<Prepared> {
    if source.bytes > LIMIT {
        return fail("pdf_too_large");
    }
    let (mut file, stamp) = source.open().map_err(|error| Failure {
        code: if matches!(
            error.kind(),
            std::io::ErrorKind::InvalidData | std::io::ErrorKind::InvalidInput
        ) {
            "source_changed"
        } else {
            "file_unavailable"
        },
    })?;
    if file.metadata()?.permissions().readonly() {
        return fail("file_unavailable");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if file.metadata()?.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_ENCRYPTED
            != 0
        {
            return fail("pdf_protected");
        }
    }
    let original = crate::metadata::hash(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(source.bytes as usize);
    (&mut file).take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() as i64 > LIMIT {
        return fail("pdf_too_large");
    }
    let doc = std::panic::catch_unwind(|| Document::load_mem(&bytes))
        .map_err(|_| Failure {
            code: "pdf_unsupported",
        })?
        .map_err(document_error)?;
    if doc.trailer.has(b"Encrypt") || doc.is_encrypted() {
        return fail("pdf_protected");
    }
    if doc.objects.values().any(|o| {
        let Ok(dict) = o.as_dict() else { return false };
        dict.get(b"Type")
            .and_then(Object::as_name)
            .is_ok_and(|n| n == b"Sig" || n == b"DocTimeStamp")
            || dict
                .get(b"FT")
                .and_then(Object::as_name)
                .is_ok_and(|n| n == b"Sig")
            || (dict.has(b"ByteRange") && dict.has(b"Contents"))
    }) {
        return fail("pdf_protected");
    }
    let pages = doc.get_pages();
    if pages.is_empty() {
        return fail("pdf_unsupported");
    }
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(document_error)?;
    let mut root = doc.get_dictionary(root_id).map_err(document_error)?.clone();
    if root.has(b"Perms") {
        return fail("pdf_protected");
    }
    let old_xmp = if root.has(b"Metadata") {
        let id = root
            .get(b"Metadata")
            .and_then(Object::as_reference)
            .map_err(document_error)?;
        let stream = doc
            .get_object(id)
            .and_then(Object::as_stream)
            .map_err(document_error)?;
        if stream.content.len() > 2 * 1024 * 1024 {
            return fail("xmp_unsupported");
        }
        Some(if stream.dict.has(b"Filter") {
            stream.decompressed_content().map_err(document_error)?
        } else {
            stream.content.clone()
        })
    } else {
        None
    };
    let xml = xmp::merge(old_xmp.as_deref(), data)?;
    let mut info = if let Ok(object) = doc.trailer.get(b"Info") {
        let (_, value) = doc.dereference(object).map_err(document_error)?;
        value.as_dict().map_err(document_error)?.clone()
    } else {
        Dictionary::new()
    };
    let author = data.authors.join("; ");
    let fields = [
        ("Title", data.title.clone()),
        ("Author", author),
        ("Subject", data.subject.clone().unwrap_or_default()),
        ("Keywords", xmp::keywords(data).join("; ")),
        ("Publisher", data.publisher.clone().unwrap_or_default()),
        ("ISBN", data.isbn.clone().unwrap_or_default()),
        (
            "PublicationYear",
            data.year.map(|n| n.to_string()).unwrap_or_default(),
        ),
        ("Language", data.language.clone().unwrap_or_default()),
        ("Genre", data.genre.clone().unwrap_or_default()),
        (
            "LibraDeskCategories",
            serde_json::to_string(&data.categories)?,
        ),
        ("LibraDeskTags", serde_json::to_string(&data.tags)?),
        (
            "LibraDeskCollections",
            serde_json::to_string(&data.collections)?,
        ),
        ("LibraDeskReadingStatus", data.reading_status.clone()),
        ("LibraDeskFavorite", data.favorite.to_string()),
    ];
    for (key, value) in fields {
        info.set(key, text_string(&value));
    }
    let mut incremental = IncrementalDocument::create_from(bytes, doc);
    let xmp_id = incremental.new_document.add_object(Stream::new(
        lopdf::dictionary! {"Type"=>"Metadata","Subtype"=>"XML"},
        xml,
    ));
    root.set("Metadata", xmp_id);
    incremental.new_document.set_object(root_id, root);
    let info_id = incremental.new_document.add_object(info);
    incremental.new_document.trailer.set("Info", info_id);
    let parent = Path::new(&source.path).parent().ok_or(Failure {
        code: "source_changed",
    })?;
    let _parent_guard = crate::storage_management::directory(parent)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(&format!(".libradesk-edit-{}-", source.id))
        .suffix(".tmp")
        .tempfile_in(parent)?;
    super::file_security::copy_dacl(Path::new(&source.path), temporary.path())?;
    incremental.save_to(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    let validated = Document::load(temporary.path()).map_err(document_error)?;
    if validated.get_pages() != pages {
        return fail("pdf_unsupported");
    }
    let info = validated
        .trailer
        .get(b"Info")
        .and_then(Object::as_reference)
        .and_then(|id| validated.get_dictionary(id))
        .map_err(document_error)?;
    let title = lopdf::decode_text_string(info.get(b"Title").map_err(document_error)?)
        .map_err(document_error)?;
    if title != data.title {
        return fail("pdf_unsupported");
    }
    if files::stamp(&file)? != stamp {
        return fail("source_changed");
    }
    temporary.as_file_mut().seek(SeekFrom::Start(0))?;
    let digest = crate::metadata::hash(temporary.as_file_mut())?;
    Ok(Prepared {
        temporary,
        original,
        digest,
        token: stamp.token,
    })
}
pub fn safe_stage(attempt: &super::repository::Attempt) -> Result<std::path::PathBuf> {
    let source = Path::new(&attempt.path);
    let stage = Path::new(&attempt.stage);
    if stage.parent() != source.parent()
        || !stage.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            n.starts_with(&format!(".libradesk-edit-{}-", attempt.file)) && n.ends_with(".tmp")
        })
    {
        return fail("staging_changed");
    }
    Ok(stage.to_path_buf())
}
pub fn remove_stage(attempt: &super::repository::Attempt) {
    if let Ok(path) = safe_stage(attempt) {
        let _ = fs::remove_file(path);
    }
}
pub fn replace(target: &Path, stage: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
        let stage: Vec<u16> = stage.as_os_str().encode_wide().chain(Some(0)).collect();
        // Checked siblings stay on the same volume. No copy/delete fallback or backup is used.
        let replaced = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                stage.as_ptr(),
                target.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        };
        if replaced == 0 {
            return fail("file_unavailable");
        }
    }
    #[cfg(not(windows))]
    fs::rename(stage, target)?;
    Ok(())
}
