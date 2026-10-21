//! Bookmarks and clip ranges are metadata in the account's encrypted database.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub kind: String,
    pub position: f64,
    pub end_position: Option<f64>,
    pub text: String,
}
pub(crate) fn valid(p: &Passage) -> Result<(), String> {
    if !p.position.is_finite() || !(0.0..31_536_000.0).contains(&p.position) || p.text.len() > 10000
    {
        return Err("Choose a valid timestamp and at most 10,000 bytes of text".into());
    }
    match (p.kind.as_str(), p.end_position) {
        ("note", None) if !p.text.trim().is_empty() => Ok(()),
        ("bookmark", None) => Ok(()),
        ("clip", Some(end))
            if end.is_finite()
                && end - p.position >= 1.0
                && end < 31_536_000.0
                && end - p.position <= 300.0 =>
        {
            Ok(())
        }
        _ => Err("Clips need an end after the start, up to five minutes long".into()),
    }
}
pub(crate) fn save(
    c: &Connection,
    asin: &str,
    p: &Passage,
    edit: Option<(i64, &Passage)>,
) -> Result<(), String> {
    valid(p)?;
    if !crate::valid_asin(asin) {
        return Err("Invalid book".into());
    }
    crate::notes::schema(c)?;
    let metadata: String = c
        .query_row("SELECT metadata FROM books WHERE asin=?1", [asin], |r| {
            r.get(0)
        })
        .map_err(|_| "Sync this book before saving bookmarks")?;
    let duration = serde_json::from_str::<serde_json::Value>(&metadata)
        .ok()
        .and_then(|v| v["duration"].as_f64());
    if let Some(duration) = duration.filter(|d| d.is_finite() && *d > 0.0) {
        if p.position >= duration || p.end_position.is_some_and(|end| end > duration) {
            return Err("This range extends beyond the book".into());
        }
    }
    c.execute_batch("SAVEPOINT save_passage")
        .map_err(|_| "Cannot save bookmark")?;
    let result = (|| {
        let id = if let Some((id, expected)) = edit {
            let actual = get(c, asin, id)?;
            if &actual != expected {
                return Err("This item changed. Refresh before editing.".into());
            }
            c.execute(
                "UPDATE notes SET position=?1,text=?2 WHERE rowid=?3 AND asin=?4",
                params![p.position, p.text.trim(), id, asin],
            )
            .map_err(|_| "Cannot edit bookmark")?;
            id
        } else {
            c.execute(
                "INSERT INTO notes(asin,position,text) VALUES(?1,?2,?3)",
                params![asin, p.position, p.text.trim()],
            )
            .map_err(|_| "Cannot save bookmark")?;
            c.last_insert_rowid()
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        c.execute("INSERT INTO note_details(note_id,kind,end_position,created_ms) VALUES(?1,?2,?3,?4) ON CONFLICT(note_id) DO UPDATE SET kind=excluded.kind,end_position=excluded.end_position", params![id,p.kind,p.end_position,now]).map_err(|_|"Cannot save clip details")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = c.execute_batch("ROLLBACK TO save_passage");
    }
    c.execute_batch("RELEASE save_passage")
        .map_err(|_| "Cannot finish bookmark save")?;
    result
}
pub(crate) fn get(c: &Connection, asin: &str, id: i64) -> Result<Passage, String> {
    crate::notes::schema(c)?;
    c.query_row("SELECT COALESCE(d.kind,CASE WHEN n.text='Bookmark' THEN 'bookmark' ELSE 'note' END),n.position,d.end_position,n.text FROM notes n LEFT JOIN note_details d ON d.note_id=n.rowid WHERE n.rowid=?1 AND n.asin=?2", params![id,asin], |r| Ok(Passage{kind:r.get(0)?,position:r.get(1)?,end_position:r.get(2)?,text:r.get(3)?})).map_err(|_|"This saved item is no longer available".into())
}
fn changed(app: &tauri::AppHandle) {
    let _ = app.emit_to("main", "notes-changed", ());
    let _ = app.emit_to("mini-player", "notes-changed", ());
}
#[tauri::command]
pub fn save_highlight(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    passage: Passage,
    id: Option<i64>,
    expected: Option<Passage>,
) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    if id.is_some() != expected.is_some() {
        return Err("Refresh the item before editing".into());
    }
    crate::metadata::with_account_db(&app, generation, |c| {
        save(c, &asin, &passage, id.zip(expected.as_ref()))
    })?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub fn delete_highlight(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    id: i64,
    expected: Passage,
) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    crate::metadata::with_account_db(&app, generation, |c| {
        c.execute_batch("SAVEPOINT delete_passage")
            .map_err(|_| "Cannot delete item")?;
        let result = (|| {
            if get(c, &asin, id)? != expected {
                return Err("This item changed. Refresh before deleting.".into());
            }
            c.execute(
                "DELETE FROM notes WHERE rowid=?1 AND asin=?2",
                params![id, asin],
            )
            .map_err(|_| "Cannot delete item")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = c.execute_batch("ROLLBACK TO delete_passage");
        }
        c.execute_batch("RELEASE delete_passage")
            .map_err(|_| "Cannot finish deleting item")?;
        result
    })?;
    changed(&app);
    Ok(())
}
pub fn quick_bookmark(
    app: &tauri::AppHandle,
    generation: u64,
    s: &crate::PlayerState,
) -> Result<(), String> {
    crate::metadata::with_account_db(app, generation, |c| {
        save(
            c,
            &s.asin,
            &Passage {
                kind: "bookmark".into(),
                position: s.position,
                end_position: None,
                text: String::new(),
            },
            None,
        )
    })?;
    changed(app);
    Ok(())
}
pub fn quick_clip(
    app: &tauri::AppHandle,
    generation: u64,
    s: &crate::PlayerState,
) -> Result<(), String> {
    let end = s.position.floor();
    if end < 1.0 {
        return Err("Listen for at least one second before capturing a clip".into());
    }
    crate::metadata::with_account_db(app, generation, |c| {
        save(
            c,
            &s.asin,
            &Passage {
                kind: "clip".into(),
                position: (end - 30.0).max(0.0),
                end_position: Some(end),
                text: String::new(),
            },
            None,
        )
    })?;
    changed(app);
    Ok(())
}
#[tauri::command]
pub fn show_highlights(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    crate::metadata::require_account(&app)?;
    crate::lifecycle::restore(&app);
    if window.label() == "mini-player" {
        window.close().map_err(|_| "Cannot return to saved items")?;
    }
    app.emit_to("main", "show-highlights", ())
        .map_err(|_| "Cannot open saved items".into())
}
#[tauri::command]
pub fn play_saved_clip(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    id: i64,
) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    let p = crate::metadata::with_account_db(&app, generation, |c| get(c, &asin, id))?;
    valid(&p)?;
    let end = p
        .end_position
        .filter(|_| p.kind == "clip")
        .ok_or("Choose a saved clip")?;
    if crate::history::blocked(&app) {
        return Err("Choose a listening position in History first".into());
    }
    crate::history::flush(&app, "before-clip")?;
    // Read WebView state before acquiring locks used by UI telemetry.
    let w = app
        .get_webview_window("official")
        .ok_or("Open the official player first")?;
    let url = w.url().map_err(|_| "Cannot read player origin")?;
    if !crate::allowed(&url)
        || !matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
        || url.path() != "/webplayer"
        || url
            .query_pairs()
            .any(|(k, v)| k == "isSample" && v == "true")
        || url
            .query_pairs()
            .find(|(k, _)| k == "asin")
            .map(|(_, v)| v.into_owned())
            != Some(asin.clone())
    {
        return Err("Wait for this book's official player".into());
    }
    crate::metadata::with_account_generation(&app, generation, || {
        let state = app.state::<crate::desktop::PlayerHub>();
        let hub = state.0.lock().map_err(|_| "Player unavailable")?;
        let s = hub.snapshot.as_ref().ok_or("Open this book first")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if hub.book.as_ref().and_then(|b| b["asin"].as_str()) != Some(asin.as_str())
            || s.asin != asin
            || !s.available
            || s.sample
            || now.saturating_sub(hub.last_seen.unwrap_or(0)) > 3
            || !s
                .seekable
                .iter()
                .any(|r| p.position >= r.start && end <= r.end)
        {
            return Err("Open this book and wait for the clip range to be seekable".into());
        }
        state.changed();
        let asin = serde_json::to_string(&asin).map_err(|_| "Invalid book")?;
        w.eval(format!("if(new URL(location.href).searchParams.get('asin')==={asin})window.__audibleAdapter?.clip({},{});",p.position,end)).map_err(|_|"Cannot start clip".to_string())
    })
}
#[tauri::command]
pub async fn seek_highlight_position(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    generation: u64,
    asin: String,
    position: f64,
) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    if !crate::valid_asin(&asin)
        || !position.is_finite()
        || !(0.0..31_536_000.0).contains(&position)
    {
        return Err("Invalid saved position".into());
    }
    if crate::history::blocked(&app) {
        return Err("Choose a listening position in History first".into());
    }
    let revision =
        crate::player_sync::seek_saved_position(&app, &asin, generation, position).await?;
    crate::history::chosen(&app, &asin, generation, revision)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE notes(asin TEXT,position REAL,text TEXT);CREATE TABLE books(asin TEXT PRIMARY KEY,metadata TEXT);INSERT INTO books VALUES('B012345678','{\"duration\":600}');INSERT INTO notes VALUES('B012345678',42,'Bookmark');").unwrap();
        c
    }
    fn clip() -> Passage {
        Passage {
            kind: "clip".into(),
            position: 50.,
            end_position: Some(80.),
            text: "A passage".into(),
        }
    }
    #[test]
    fn preserves_old_rows_and_rejects_stale_edits() {
        let c = database();
        let old = get(&c, "B012345678", 1).unwrap();
        assert_eq!(old.kind, "bookmark");
        save(&c, "B012345678", &clip(), None).unwrap();
        let saved = get(&c, "B012345678", 2).unwrap();
        let mut edit = saved.clone();
        edit.end_position = Some(90.);
        save(&c, "B012345678", &edit, Some((2, &saved))).unwrap();
        assert!(save(&c, "B012345678", &clip(), Some((2, &saved))).is_err());
        assert_eq!(get(&c, "B012345678", 1).unwrap(), old);
        assert!(get(&c, "B987654321", 2).is_err());
    }
    #[test]
    fn validates_ranges_and_retains_data_on_failed_write() {
        let c = database();
        for end in [50., 49., f64::NAN, 400., 601.] {
            let mut p = clip();
            p.end_position = Some(end);
            assert!(save(&c, "B012345678", &p, None).is_err());
        }
        assert_eq!(
            c.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn deletion_cleans_details_before_row_id_reuse() {
        let c = database();
        save(&c, "B012345678", &clip(), None).unwrap();
        c.execute("DELETE FROM notes WHERE rowid=2", []).unwrap();
        c.execute("INSERT INTO notes VALUES('B012345678',12,'Bookmark')", [])
            .unwrap();
        assert_eq!(get(&c, "B012345678", 2).unwrap().kind, "bookmark");
    }
}
