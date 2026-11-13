pub mod commands;
mod duplicates;
pub(crate) mod probe;
mod repository;
#[cfg(test)]
mod tests;

pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use tauri::Emitter;
        let db = loop {
            let _access = crate::maintenance::ACCESS
                .read()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(db) = crate::commands::database_path(&app)
                .ok()
                .and_then(|path| crate::database::Database::open(&path).ok())
            {
                break db;
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        };
        {
            let _access = crate::maintenance::ACCESS
                .read()
                .unwrap_or_else(|error| error.into_inner());
            let _ = db.start_verification();
        }
        let mut last_event = std::time::Instant::now() - std::time::Duration::from_secs(2);
        let mut catalog_dirty = false;
        loop {
            let result = (|| -> Result<bool, crate::database::Error> {
                let _access = crate::maintenance::ACCESS
                    .read()
                    .unwrap_or_else(|error| error.into_inner());
                if db.verification_due()? {
                    db.start_verification()?;
                }
                let Some(file) = db.verification_next()? else {
                    let finished = db.finish_verification()?;
                    if finished || catalog_dirty {
                        let _ = app.emit("recovery-updated", ());
                        if catalog_dirty {
                            let _ = app.emit("catalog-updated", ());
                            catalog_dirty = false;
                        }
                        last_event = std::time::Instant::now();
                    }
                    return Ok(false);
                };
                let observation = probe::inspect(&file);
                catalog_dirty |=
                    observation.status != file.availability || observation.analysis.is_some();
                db.apply_verification(&file, observation)?;
                if last_event.elapsed() >= std::time::Duration::from_secs(1) {
                    let _ = app.emit("recovery-updated", file.source.id);
                    if catalog_dirty {
                        let _ = app.emit("catalog-updated", ());
                        catalog_dirty = false;
                    }
                    last_event = std::time::Instant::now();
                }
                Ok(true)
            })();
            std::thread::sleep(std::time::Duration::from_millis(
                if matches!(result, Ok(true)) {
                    100
                } else {
                    1500
                },
            ));
        }
    });
}
