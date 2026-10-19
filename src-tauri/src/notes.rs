use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub asin: String,
    pub position: f64,
    pub text: String,
}

fn read(c: &Connection) -> Result<Vec<Note>, String> {
    let mut q = c
        .prepare("SELECT rowid,asin,position,text FROM notes ORDER BY rowid")
        .map_err(|_| "Cannot read notes")?;
    let rows = q
        .query_map([], |r| {
            Ok(Note {
                id: r.get(0)?,
                asin: r.get(1)?,
                position: r.get(2)?,
                text: r.get(3)?,
            })
        })
        .map_err(|_| "Cannot read notes")?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Cannot read notes".into())
}

fn validate(asin: &str, position: f64, text: &str) -> Result<(), String> {
    if !super::valid_asin(asin)
        || !position.is_finite()
        || !(0.0..31_536_000.0).contains(&position)
        || text.trim().is_empty()
        || text.len() > 10000
    {
        return Err("A note needs a valid book, timestamp and 1–10,000 bytes of text.".into());
    }
    Ok(())
}

// Capture the account's connection while holding its lifecycle lock. A queued
// command must never open the next account's DB using a previous account's note.
fn with_db<T>(
    app: &tauri::AppHandle,
    generation: u64,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    super::metadata::with_account_db(app, generation, f)
}

#[tauri::command]
pub fn notes_session(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<u64, String> {
    super::local(&window)?;
    super::metadata::account_generation(&app)
}

#[tauri::command]
pub fn list_notes(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
) -> Result<Vec<Note>, String> {
    super::local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(Vec::new());
    }
    with_db(&app, generation, read)
}

#[tauri::command]
pub fn save_note(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    position: f64,
    text: String,
) -> Result<(), String> {
    super::local(&window)?;
    validate(&asin, position, &text)?;
    with_db(&app, generation, |c| {
        let owned: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE asin=?1)",
                [&asin],
                |r| r.get(0),
            )
            .map_err(|_| "Cannot check book")?;
        if !owned {
            return Err("Sync this book into your library before adding a note.".into());
        }
        c.execute(
            "INSERT INTO notes(asin,position,text) VALUES(?1,?2,?3)",
            params![asin, position, text.trim()],
        )
        .map_err(|_| "Cannot save note")?;
        Ok(())
    })?;
    let _ = app.emit_to("main", "notes-changed", ());
    Ok(())
}

fn change(
    c: &Connection,
    id: i64,
    asin: &str,
    expected: &str,
    text: Option<&str>,
) -> Result<(), String> {
    if id <= 0 || !super::valid_asin(asin) {
        return Err("Invalid note selection".into());
    }
    let count = if let Some(text) = text {
        validate(asin, 0.0, text)?;
        c.execute(
            "UPDATE notes SET text=?1 WHERE rowid=?2 AND asin=?3 AND text=?4",
            params![text.trim(), id, asin, expected],
        )
    } else {
        c.execute(
            "DELETE FROM notes WHERE rowid=?1 AND asin=?2 AND text=?3",
            params![id, asin, expected],
        )
    }
    .map_err(|_| "Cannot change note")?;
    if count != 1 {
        return Err(
            "This note changed or no longer exists. Reopen the app to refresh your notes.".into(),
        );
    }
    Ok(())
}

pub(crate) fn write_export(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::{fs, fs::OpenOptions, io::Write};
    crate::storage::reject_link(path)?;
    let parent = path.parent().ok_or("Choose a local file")?;
    let temporary = parent.join(format!(
        ".listeners-library-export-{:032x}.tmp",
        rand::random::<u128>()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Cannot create export file")?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Cannot finish export file")?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| "Cannot replace selected export file")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[tauri::command]
pub fn edit_note(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    id: i64,
    asin: String,
    text: String,
    expected: String,
) -> Result<(), String> {
    super::local(&window)?;
    with_db(&app, generation, |c| {
        change(c, id, &asin, &expected, Some(&text))
    })?;
    let _ = app.emit_to("main", "notes-changed", ());
    Ok(())
}
#[tauri::command]
pub fn delete_note(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    id: i64,
    asin: String,
    expected: String,
) -> Result<(), String> {
    super::local(&window)?;
    with_db(&app, generation, |c| change(c, id, &asin, &expected, None))?;
    let _ = app.emit_to("main", "notes-changed", ());
    Ok(())
}

#[tauri::command]
pub async fn export_notes(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    old: bool,
    confirmation: String,
) -> Result<bool, String> {
    super::local(&window)?;
    if confirmation
        != if old {
            "EXPORT OLD NOTES"
        } else {
            "EXPORT NOTES"
        }
    {
        return Err(
            "Confirm the export first. The exported JSON contains readable, unencrypted notes."
                .into(),
        );
    }
    // Pin both account generation and a data snapshot before opening the dialog.
    let notes = with_db(&app, generation, |c| {
        if !old {
            return read(c);
        }
        let legacy = app.state::<crate::storage::Storage>().open()?;
        let exists: bool = legacy
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| "Cannot inspect old notes")?;
        if exists {
            read(&legacy)
        } else {
            Ok(Vec::new())
        }
    })?;
    if notes.is_empty() {
        return Err("No notes were found in this store. Nothing was exported.".into());
    }
    let file = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .set_title("Export readable notes — choose a private location")
                .add_filter("Notes JSON", &["json"])
                .set_file_name(if old {
                    "listeners-library-old-notes.json"
                } else {
                    "listeners-library-notes.json"
                })
                .blocking_save_file()
        }
    })
    .await
    .map_err(|_| "Cannot open export dialog")?;
    let Some(file) = file else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|_| "Choose a local JSON file")?;
    if path
        .extension()
        .and_then(|v| v.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("json"))
    {
        return Err("Choose a .json file".into());
    }
    let payload=serde_json::to_vec_pretty(&serde_json::json!({"format":"listeners-library-notes","version":1,"source":if old {"unassigned-old-store"}else{"current-account"},"notes":notes})).map_err(|_|"Cannot prepare notes export")?;
    // No frontend-provided filesystem path, tokens, cookies or database key.
    super::metadata::with_account_generation(&app, generation, || write_export(&path, &payload))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_rows_keep_their_ids_and_edits_cannot_touch_another_book() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE notes(asin TEXT,position REAL,text TEXT); INSERT INTO notes VALUES('B012345678',42,'keep'),('B987654321',99,'other');").unwrap();
        let rows = read(&c).unwrap();
        assert_eq!(rows[0].position, 42.0);
        assert!(change(&c, rows[0].id, "B987654321", "keep", Some("wrong book")).is_err());
        change(&c, rows[0].id, &rows[0].asin, "keep", Some("edited")).unwrap();
        assert_eq!(read(&c).unwrap()[0].text, "edited");
        assert!(change(&c, rows[0].id, &rows[0].asin, "keep", None).is_err());
        change(&c, rows[0].id, &rows[0].asin, "edited", None).unwrap();
        assert_eq!(read(&c).unwrap()[0].text, "other");
    }
    #[test]
    fn reject_invalid_notes() {
        for (p, t) in [(f64::NAN, "x"), (-1.0, "x"), (0.0, "   ")] {
            assert!(validate("B012345678", p, t).is_err());
        }
        assert!(validate("../private", 0.0, "x").is_err());
    }
    #[test]
    fn export_replaces_only_the_chosen_file_after_a_complete_write() {
        let root =
            std::env::temp_dir().join(format!("listeners-notes-test-{}", rand::random::<u64>()));
        std::fs::create_dir(&root).unwrap();
        let selected = root.join("notes.json");
        std::fs::write(&selected, b"old backup").unwrap();
        write_export(&selected, b"synthetic notes").unwrap();
        assert_eq!(std::fs::read(&selected).unwrap(), b"synthetic notes");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
