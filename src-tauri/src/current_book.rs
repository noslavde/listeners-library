//! Explicit per-account current-book pin. This selection survives playing a
//! different title and stays separate from activity/history-derived Continue
//! listening eligibility.

use rusqlite::{Connection, OptionalExtension};
use tauri::Emitter;

fn read(c: &Connection) -> Result<Option<String>, String> {
    let value: Option<String> = c
        .query_row(
            "SELECT value FROM preferences WHERE key='current_book'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| "Cannot read current book")?;
    Ok(value.filter(|asin| super::valid_asin(asin)))
}
fn write(c: &Connection, asin: Option<&str>) -> Result<(), String> {
    match asin {
        Some(asin) => {
            if !super::valid_asin(asin) {
                return Err("Choose a valid library title".into());
            }
            let exists: bool = c
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM books WHERE asin=?1)",
                    [asin],
                    |r| r.get(0),
                )
                .map_err(|_| "Cannot check library title")?;
            if !exists {
                return Err("Choose a title from your cached library".into());
            }
            c.execute(
                "INSERT OR REPLACE INTO preferences(key,value) VALUES('current_book',?1)",
                [asin],
            )
            .map_err(|_| "Cannot set current book")?;
        }
        None => {
            c.execute("DELETE FROM preferences WHERE key='current_book'", [])
                .map_err(|_| "Cannot clear current book")?;
        }
    }
    Ok(())
}
#[tauri::command]
pub fn get_current_book(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    super::desktop::player_local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(None);
    }
    read(&super::db(&app)?)
}
#[tauri::command]
pub fn set_current_book(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: Option<String>,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    super::metadata::require_account(&app)?;
    write(&super::db(&app)?, asin.as_deref())?;
    for target in ["main", "mini-player"] {
        let _ = app.emit_to(target, "current-book-changed", ());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_explicit_selection_can_be_replaced_and_cleared_without_touching_history() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE preferences(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE books(asin TEXT PRIMARY KEY); INSERT INTO books VALUES('B012345678'),('B987654321'); INSERT INTO preferences VALUES('listening','keep');").unwrap();
        assert_eq!(read(&c).unwrap(), None);
        write(&c, Some("B012345678")).unwrap();
        assert_eq!(read(&c).unwrap().as_deref(), Some("B012345678"));
        assert!(write(&c, Some("B111111111")).is_err());
        assert!(write(&c, Some("../bad")).is_err());
        assert_eq!(read(&c).unwrap().as_deref(), Some("B012345678"));
        write(&c, Some("B987654321")).unwrap();
        assert_eq!(read(&c).unwrap().as_deref(), Some("B987654321"));
        write(&c, None).unwrap();
        assert_eq!(read(&c).unwrap(), None);
        assert_eq!(
            c.query_row(
                "SELECT value FROM preferences WHERE key='listening'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "keep"
        );
    }
}
