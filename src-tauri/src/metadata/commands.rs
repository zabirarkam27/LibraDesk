use std::{io::ErrorKind, path::PathBuf};

use tauri::{ipc::Response, AppHandle, Emitter};

use crate::commands::{unavailable, with_database, CommandError};

use super::{
    cover::{self, MAX_COVER_SOURCE},
    pdf::RawMetadata,
    repository::{CoverTask, DuplicateGroup, Edit, Field, ReviewRow},
    EVENT,
};

fn rejected(code: &'static str, message: &'static str) -> CommandError {
    CommandError {
        code,
        message,
        retryable: false,
    }
}

async fn cache_root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    let root = crate::storage_management::root(app)?;
    let generation = with_database(app.clone(), |db| db.cover_cache_generation()).await?;
    super::resolve_cache_root(&root, generation.as_deref()).map_err(|_| unavailable())
}

fn stale_cover() -> CommandError {
    rejected(
        "cover_stale",
        "The PDF changed or the cover job is no longer current.",
    )
}

#[tauri::command]
pub async fn next_cover_task(app: AppHandle) -> Result<Option<CoverTask>, CommandError> {
    with_database(app, |db| Ok(db.next_cover()?)).await
}

#[tauri::command]
pub async fn read_cover_range(
    app: AppHandle,
    file_id: i64,
    revision: i64,
    generation: u64,
    offset: i64,
    length: i64,
) -> Result<Response, CommandError> {
    if generation != crate::maintenance::generation() {
        return Err(stale_cover());
    }
    let source = with_database(app, move |db| Ok(db.cover_source(file_id, revision)?))
        .await?
        .ok_or_else(stale_cover)?;
    let bytes =
        tauri::async_runtime::spawn_blocking(move || cover::read_range(&source, offset, length))
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| stale_cover())?;
    Ok(Response::new(bytes))
}

#[tauri::command]
pub async fn finish_cover(
    app: AppHandle,
    file_id: i64,
    revision: i64,
    generation: u64,
    png: Option<Vec<u8>>,
    raw: Option<RawMetadata>,
) -> Result<(), CommandError> {
    if generation != crate::maintenance::generation() {
        return Err(stale_cover());
    }
    let Some(source) = with_database(app.clone(), move |db| {
        Ok(db.cover_source(file_id, revision)?)
    })
    .await?
    else {
        return Ok(());
    };
    let root = cache_root(&app).await?;
    let checked_source = source.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .read()
            .unwrap_or_else(|error| error.into_inner());
        if generation != crate::maintenance::generation() {
            return (None, false, "cover_stale");
        }
        if checked_source.bytes > MAX_COVER_SOURCE {
            return (None, false, "cover_size_limit");
        }
        if checked_source.open().is_err() {
            return (None, false, "source_unavailable");
        }
        let image = png
            .as_deref()
            .and_then(|bytes| cover::save_png(&root, &checked_source, bytes).ok());
        (image, true, "cover_unavailable")
    })
    .await
    .map_err(|_| unavailable())?;
    let (image, current, error) = prepared;
    let raw = raw.filter(RawMetadata::is_bounded);
    with_database(app.clone(), move |db| {
        if generation != crate::maintenance::generation() {
            return Ok(());
        }
        if current {
            db.complete_cover(&source, image, raw)?;
        } else {
            db.fail_processing(file_id, revision, "cover", error)?;
        }
        Ok(())
    })
    .await?;
    let _ = app.emit(EVENT, ());
    Ok(())
}

#[tauri::command]
pub async fn get_book_cover(
    app: AppHandle,
    file_id: i64,
    revision: i64,
) -> Result<Response, CommandError> {
    let version = with_database(app.clone(), move |db| Ok(db.cover_revision(file_id)?)).await?;
    if version != Some(revision) {
        return Err(stale_cover());
    }
    if revision < 0 {
        let image = with_database(app, move |db| db.custom_cover(file_id, revision))
            .await?
            .ok_or_else(stale_cover)?;
        cover::check_png(&image).map_err(|_| stale_cover())?;
        return Ok(Response::new(image));
    }
    let root = cache_root(&app).await?;
    let result =
        tauri::async_runtime::spawn_blocking(move || cover::load_png(&root, file_id, revision))
            .await
            .map_err(|_| unavailable())?;
    match result {
        Ok(bytes) => Ok(Response::new(bytes)),
        Err(error) => {
            if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::InvalidData) {
                with_database(app.clone(), move |db| {
                    Ok(db.requeue_cover(file_id, revision)?)
                })
                .await?;
                let _ = app.emit(EVENT, ());
            }
            Err(stale_cover())
        }
    }
}

#[tauri::command]
pub async fn metadata_review_page(
    app: AppHandle,
    before: Option<i64>,
    missing_only: bool,
) -> Result<Vec<ReviewRow>, CommandError> {
    with_database(app, move |db| Ok(db.review_page(before, missing_only)?)).await
}

#[tauri::command]
pub async fn save_book_metadata(
    app: AppHandle,
    mut ids: Vec<i64>,
    edits: Vec<Edit>,
) -> Result<(), CommandError> {
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty()
        || ids.len() > 50
        || ids[0] < 1
        || edits.is_empty()
        || edits.len() > 4
        || edits.iter().enumerate().any(|(index, edit)| {
            edit.value.chars().count() > 4096
                || (edit.field == Field::Title && edit.value.trim().is_empty())
                || edits[..index]
                    .iter()
                    .any(|earlier| earlier.field == edit.field)
        })
    {
        return Err(rejected(
            "invalid_edit",
            "Select at least one book and field. A title cannot be empty.",
        ));
    }
    let saved = with_database(app.clone(), move |db| Ok(db.save_metadata(&ids, &edits)?)).await?;
    if !saved {
        return Err(rejected(
            "book_missing",
            "A selected book is no longer in the catalog.",
        ));
    }
    let _ = app.emit(EVENT, ());
    Ok(())
}

fn parse_hash(hash: &str) -> Result<Vec<u8>, CommandError> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        if let Ok(bytes) = (0..64)
            .step_by(2)
            .map(|index| u8::from_str_radix(&hash[index..index + 2], 16))
            .collect()
        {
            return Ok(bytes);
        }
    }
    Err(rejected(
        "invalid_digest",
        "The duplicate group ID is invalid.",
    ))
}

#[tauri::command]
pub async fn list_duplicate_groups(
    app: AppHandle,
    after: Option<String>,
) -> Result<Vec<DuplicateGroup>, CommandError> {
    let after = after.as_deref().map(parse_hash).transpose()?;
    with_database(app, move |db| Ok(db.duplicate_groups(after)?)).await
}

#[tauri::command]
pub async fn set_duplicate_review(
    app: AppHandle,
    hash: String,
    reviewed: bool,
) -> Result<(), CommandError> {
    let hash = parse_hash(&hash)?;
    if !with_database(app.clone(), move |db| {
        Ok(db.review_duplicate(&hash, reviewed)?)
    })
    .await?
    {
        return Err(rejected(
            "group_changed",
            "This duplicate group changed. Refresh and try again.",
        ));
    }
    let _ = app.emit(EVENT, ());
    Ok(())
}
