use crate::{
    commands::{with_database, CommandError},
    database::{Database, Error},
    reader::ReaderPreferences,
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StartPage {
    #[default]
    Overview,
    Library,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryView {
    #[default]
    Grid,
    List,
    Compact,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspacePreferences {
    pub start_page: StartPage,
    pub library_view: LibraryView,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    workspace: WorkspacePreferences,
    reader: ReaderPreferences,
    data_path: String,
    version: String,
}
impl Database {
    pub fn workspace_preferences(&self) -> Result<WorkspacePreferences, Error> {
        let value: Option<String> = self
            .0
            .query_row(
                "SELECT value_json FROM settings WHERE scope='app' AND key='workspace'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(value
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default())
    }
    pub fn save_workspace_preferences(&self, input: WorkspacePreferences) -> Result<(), Error> {
        self.0.execute("INSERT INTO settings(scope,key,value_json,updated_at) VALUES('app','workspace',?1,CAST(unixepoch('subsec')*1000 AS INTEGER))
   ON CONFLICT(scope,key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at",
   [serde_json::to_string(&input)?])?;
        Ok(())
    }
}
#[tauri::command]
pub async fn get_settings_snapshot(app: AppHandle) -> Result<SettingsSnapshot, CommandError> {
    let data_path = crate::storage_management::root(&app)?
        .to_string_lossy()
        .into_owned();
    let version = app.package_info().version.to_string();
    with_database(app, move |db| {
        Ok(SettingsSnapshot {
            workspace: db.workspace_preferences()?,
            reader: db.reader_preferences()?,
            data_path,
            version,
        })
    })
    .await
}
#[tauri::command]
pub async fn save_workspace_preferences(
    app: AppHandle,
    input: WorkspacePreferences,
) -> Result<(), CommandError> {
    with_database(app, move |db| db.save_workspace_preferences(input)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_round_trip_defaults_validation_and_theme_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        let db = Database::open(&path).unwrap();
        assert!(matches!(
            db.workspace_preferences().unwrap().start_page,
            StartPage::Overview
        ));
        db.save_workspace_preferences(WorkspacePreferences {
            start_page: StartPage::Library,
            library_view: LibraryView::Compact,
        })
        .unwrap();
        db.set_theme(crate::database::Scope::App, crate::database::Theme::Dark)
            .unwrap();
        db.save_reader_preferences(&ReaderPreferences {
            scale: 175,
            zoom: crate::reader::Zoom::Custom,
            layout: crate::reader::Layout::Single,
            fullscreen: true,
            shortcuts: false,
        })
        .unwrap();
        drop(db);
        let db = Database::open(&path).unwrap();
        assert!(matches!(
            db.workspace_preferences().unwrap().start_page,
            StartPage::Library
        ));
        assert!(matches!(
            db.workspace_preferences().unwrap().library_view,
            LibraryView::Compact
        ));
        assert_eq!(db.reader_preferences().unwrap().scale, 175);
        assert!(db.reader_preferences().unwrap().fullscreen);
        assert!(!db.reader_preferences().unwrap().shortcuts);
        assert_eq!(
            db.preferences(None).unwrap().app,
            crate::database::Theme::Dark
        );
        assert_eq!(
            db.preferences(None).unwrap().reader,
            crate::database::Theme::System
        );
        for payload in [
            r#"{"startPage":"remote"}"#,
            r#"{"libraryView":"unbounded"}"#,
            r#"{"unknown":true}"#,
        ] {
            assert!(serde_json::from_str::<WorkspacePreferences>(payload).is_err());
        }
        db.0.execute(
            "UPDATE settings SET value_json='{}' WHERE scope='app' AND key='workspace'",
            [],
        )
        .unwrap();
        assert!(matches!(
            db.workspace_preferences().unwrap().library_view,
            LibraryView::Grid
        ));
        assert!(db
            .0
            .execute(
                "UPDATE settings SET value_json='broken' WHERE scope='app' AND key='workspace'",
                [],
            )
            .is_err());
        db.0.execute(
            "UPDATE settings SET value_json=?1 WHERE scope='app' AND key='workspace'",
            [r#"{"startPage":"remote"}"#],
        )
        .unwrap();
        assert!(matches!(
            db.workspace_preferences().unwrap().start_page,
            StartPage::Overview
        ));
    }
}
