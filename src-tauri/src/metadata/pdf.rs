use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

use lopdf::{decode_text_string, Document, Object};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::scan::files::{self, Stamp};

const NATIVE_PARSE_LIMIT: i64 = 32 * 1024 * 1024;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date_raw: Option<String>,
    pub page_count: Option<i64>,
}

impl RawMetadata {
    pub fn is_bounded(&self) -> bool {
        [
            &self.title,
            &self.author,
            &self.subject,
            &self.creator,
            &self.producer,
            &self.creation_date_raw,
        ]
        .into_iter()
        .flatten()
        .all(|value| value.len() <= 256 * 1024)
            && self
                .page_count
                .is_none_or(|count| count > 0 && count <= i32::MAX as i64)
    }
}

#[derive(Clone)]
pub struct Source {
    pub id: i64,
    pub revision: i64,
    pub path: String,
    pub bytes: i64,
    pub modified_at: i64,
    pub token: Option<String>,
}

impl Source {
    pub fn open(&self) -> io::Result<(File, Stamp)> {
        let path = Path::new(&self.path);
        let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
        let file = files::open_checked(parent, path)?;
        let stamp = files::stamp(&file)?;
        if stamp.bytes != self.bytes
            || stamp.modified_at != self.modified_at
            || self
                .token
                .as_ref()
                .is_some_and(|token| *token != stamp.token)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "PDF changed since it was scanned",
            ));
        }
        Ok((file, stamp))
    }
}

pub struct Analysis {
    pub digest: [u8; 32],
    pub stamp: Stamp,
    pub metadata: Option<RawMetadata>,
}

pub fn filename_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let title = stem
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() {
        "Untitled PDF".into()
    } else {
        title
    }
}

pub fn hash(reader: &mut impl Read) -> io::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 128 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(hasher.finalize().into());
        }
        hasher.update(&buffer[..count]);
    }
}

fn extract(document: &Document) -> Option<RawMetadata> {
    if document.is_encrypted() {
        return None;
    }
    let info = document
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|object| match object {
            Object::Reference(id) => document.get_dictionary(*id).ok(),
            other => other.as_dict().ok(),
        });
    let field = |key: &[u8]| {
        let object = info?.get(key).ok()?;
        let object = match object {
            Object::Reference(id) => document.get_object(*id).ok()?,
            other => other,
        };
        decode_text_string(object).ok()
    };
    let pages = document.page_iter().count() as i64;
    let raw = RawMetadata {
        title: field(b"Title"),
        author: field(b"Author"),
        subject: field(b"Subject"),
        creator: field(b"Creator"),
        producer: field(b"Producer"),
        creation_date_raw: field(b"CreationDate"),
        page_count: (pages > 0).then_some(pages),
    };
    (pages > 0 && raw.is_bounded()).then_some(raw)
}

pub fn analyze(source: &Source) -> io::Result<Analysis> {
    let (mut file, stamp) = source.open()?;
    let digest = hash(&mut file)?;
    let metadata = if stamp.bytes <= NATIVE_PARSE_LIMIT {
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::with_capacity(stamp.bytes as usize);
        (&mut file)
            .take(NATIVE_PARSE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        // A malformed document must not stop the queue or discard its already computed hash.
        std::panic::catch_unwind(|| {
            Document::load_mem(&bytes)
                .ok()
                .and_then(|doc| extract(&doc))
        })
        .ok()
        .flatten()
    } else {
        None
    };
    if files::stamp(&file)? != stamp {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PDF changed during analysis",
        ));
    }
    Ok(Analysis {
        digest,
        stamp,
        metadata,
    })
}
