//! Shared native player state and main/mini window commands. PlayerHub
//! revisions invalidate stale control work; fresh official-player observations
//! are distinct from persisted listening checkpoints.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
#[derive(Clone, Serialize)]
pub struct NowPlaying {
    pub book: Option<Value>,
    pub snapshot: Option<super::PlayerState>,
    pub last_seen: Option<u64>,
}
pub struct PlayerHub(pub Mutex<NowPlaying>, AtomicU64);
impl PlayerHub {
    pub(crate) fn revision(&self) -> u64 {
        self.1.load(Ordering::SeqCst)
    }
    pub(crate) fn changed(&self) -> u64 {
        self.1.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn new() -> Self {
        Self(
            Mutex::new(NowPlaying {
                book: None,
                snapshot: None,
                last_seen: None,
            }),
            AtomicU64::new(0),
        )
    }
}
pub(crate) fn reset_player(app: &tauri::AppHandle) -> Result<(), String> {
    super::history::reset(app);
    super::cover::dismiss(app);
    super::activity::reset(app);
    super::listening::cancel_sleep(app);
    app.state::<super::start_playback::StartPlayback>().cancel();
    if let Some(w) = app.get_webview_window("official") {
        let _ = w.eval("document.querySelectorAll('audio,video').forEach(media=>{media.pause();media.muted=true;});");
    }
    let hub = app.state::<PlayerHub>();
    let mut state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    hub.changed();
    *state = NowPlaying {
        book: None,
        snapshot: None,
        last_seen: None,
    };
    drop(state);
    super::media_keys::refresh(app);
    if let Some(w) = app.get_webview_window("mini-player") {
        w.destroy().map_err(|_| "Cannot close mini player")?;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    super::db(app)?
        .execute("DELETE FROM preferences WHERE key='current_book'", [])
        .map_err(|_| "Cannot clear current book")?;
    let _ = app.emit("current-book-changed", ());
    Ok(())
}
/// Accept commands only from an allowlisted local main/mini window. Remote
/// Audible pages are not general-purpose IPC clients.
pub fn player_local(w: &tauri::WebviewWindow) -> Result<(), String> {
    if !matches!(w.label(), "main" | "mini-player")
        || !w.url().is_ok_and(|u| super::local_origin(&u))
    {
        return Err("Local player UI required".into());
    }
    Ok(())
}
#[tauri::command]
pub fn select_api_title(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<(), String> {
    super::local(&window)?;
    if !super::valid_asin(&asin) || asin.starts_with("DEMO") {
        return Err("Select an API library title".into());
    }
    selected(&app, &asin)
}
pub fn selected(app: &tauri::AppHandle, asin: &str) -> Result<(), String> {
    let generation = super::metadata::account_generation(app)?;
    selected_in_session(app, asin, generation)
}
pub(crate) fn selected_in_session(
    app: &tauri::AppHandle,
    asin: &str,
    generation: u64,
) -> Result<(), String> {
    super::metadata::with_account_generation(app, generation, || Ok(()))?;
    super::cover::dismiss(app);
    super::listening::cancel_sleep(app);
    super::metadata::with_account_db(app, generation, |c| {
        let raw = c
            .query_row("SELECT metadata FROM books WHERE asin=?1", [asin], |r| {
                r.get::<_, String>(0)
            })
            .ok();
        let book=raw.and_then(|s|serde_json::from_str::<Value>(&s).ok()).unwrap_or_else(||json!({"asin":asin,"title":"Audible title","author":"","series":"","volume":0,"duration":0,"chapters":[]}));
        let state = app.state::<PlayerHub>();
        let mut current = state.0.lock().map_err(|_| "Player state unavailable")?;
        state.changed();
        *current = NowPlaying {
            book: Some(book.clone()),
            snapshot: None,
            last_seen: None,
        };
        drop(current);
        for label in ["main", "mini-player"] {
            let _ = app.emit_to(label, "player-book", book.clone());
        }
        Ok(())
    })
}
/// Update live player observations and freshness under the hub lock;
/// persistence/credit policies belong to history and activity modules.
pub fn record(app: &tauri::AppHandle, s: &super::PlayerState) {
    if !super::metadata::connected(app) {
        return;
    }
    let hub = app.state::<PlayerHub>();
    let Ok(mut state) = hub.0.lock() else { return };
    let asin = state
        .book
        .as_ref()
        .and_then(|b| b["asin"].as_str())
        .unwrap_or("")
        .to_owned();
    if asin.is_empty() || asin != s.asin {
        return;
    }
    state.snapshot = Some(s.clone());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    state.last_seen = Some(now);
}
#[derive(Serialize, Deserialize)]
pub struct SavedPosition {
    position: f64,
    updated: i64,
}
#[tauri::command]
/// Return account-scoped saved card positions without fetching the network or
/// pretending a cached timestamp is a mobile listening time.
pub fn list_card_positions(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Value, String> {
    super::local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(json!({}));
    }
    let c = super::db(&app)?;
    let mut result = serde_json::Map::new();
    for (table, field, column) in [
        ("playback_positions", "local", "updated"),
        ("cloud_positions", "cloud", "checked"),
    ] {
        let mut q = c
            .prepare(&format!("SELECT asin,position,{column} FROM {table}"))
            .map_err(|_| "Cannot read card positions")?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| "Cannot read card positions")?;
        for row in rows {
            let (asin, position, updated) = row.map_err(|_| "Invalid saved position")?;
            let entry = result.entry(asin).or_insert_with(|| json!({}));
            entry[field] = json!({"position":position,"updated":updated});
        }
    }
    Ok(Value::Object(result))
}
#[tauri::command]
pub fn local_position(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<Option<SavedPosition>, String> {
    super::local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(None);
    }
    if !super::valid_asin(&asin) {
        return Err("Invalid ASIN".into());
    }
    use rusqlite::OptionalExtension;
    super::db(&app)?
        .query_row(
            "SELECT position,updated FROM playback_positions WHERE asin=?1",
            [asin],
            |r| {
                Ok(SavedPosition {
                    position: r.get(0)?,
                    updated: r.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|_| "Cannot read saved position".into())
}
#[tauri::command]
/// Provide the selected book and live snapshot to a trusted local player
/// surface.
pub fn current_player(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<NowPlaying, String> {
    player_local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(NowPlaying {
            book: None,
            snapshot: None,
            last_seen: None,
        });
    }
    Ok(app
        .state::<PlayerHub>()
        .0
        .lock()
        .map_err(|_| "Player state unavailable")?
        .clone())
}
#[tauri::command]
pub async fn open_mini_player(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    super::local(&window)?;
    enter_mini_player(app).await
}
/// Show a usable mini window before hiding the main/official windows,
/// preserving a visible recovery route if creation fails.
pub(crate) async fn enter_mini_player(app: tauri::AppHandle) -> Result<(), String> {
    if crate::lifecycle::closing() {
        return Err("The app is closing".into());
    }
    super::metadata::require_account(&app)?;
    if let Some(w) = app.get_webview_window("mini-player") {
        w.unminimize().map_err(|_| "Cannot restore mini player")?;
        w.show().map_err(|_| "Cannot show mini player")?;
        w.set_focus().map_err(|_| "Cannot focus mini player")?;
        hide_ready_official(&app)?;
        return Ok(());
    }
    let mini = WebviewWindowBuilder::new(
        &app,
        "mini-player",
        WebviewUrl::App("index.html?mini=1".into()),
    )
    .title("Listener’s Library · mini player")
    .inner_size(380.0, 240.0)
    .min_inner_size(340.0, 220.0)
    .always_on_top(true)
    .decorations(false)
    .devtools(cfg!(debug_assertions))
    .resizable(true)
    .build()
    .map_err(|_| "Cannot open mini player")?;
    // Keep saved placement, but migrate the old tall layout once and never
    // restore an expanded panel as the next session's default height.
    let old_layout = super::db(&app)
        .ok()
        .map(|c| {
            c.query_row(
                "SELECT EXISTS(SELECT 1 FROM preferences WHERE key='compact_mini_v1')",
                [],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false)
        })
        .unwrap_or(false);
    let width = if old_layout {
        mini.inner_size()
            .ok()
            .map(|s| s.width as f64 / mini.scale_factor().unwrap_or(1.0))
            .unwrap_or(380.0)
    } else {
        380.0
    };
    mini.set_size(tauri::LogicalSize::new(width.max(340.0), 240.0))
        .map_err(|_| "Cannot size mini player")?;
    if let Ok(c) = super::db(&app) {
        let _ = c.execute(
            "INSERT OR REPLACE INTO preferences(key,value) VALUES('compact_mini_v1','1')",
            [],
        );
    }
    hide_ready_official(&app)?;
    Ok(())
}
#[tauri::command]
pub fn resize_mini_player(window: tauri::WebviewWindow, panel: String) -> Result<(), String> {
    if window.label() != "mini-player" {
        return Err("Mini player required".into());
    }
    let height = match panel.as_str() {
        "none" => 240.0,
        "chapters" => 330.0,
        "sleep" => 330.0,
        "settings" => 510.0,
        "history" => 490.0,
        _ => return Err("Unknown player panel".into()),
    };
    let size = window.inner_size().map_err(|_| "Cannot read player size")?;
    let scale = window
        .scale_factor()
        .map_err(|_| "Cannot read display scale")?;
    window
        .set_size(tauri::LogicalSize::new(
            (size.width as f64 / scale).max(340.0),
            height,
        ))
        .map_err(|_| "Cannot resize player".into())
}
fn hide_ready_official(app: &tauri::AppHandle) -> Result<(), String> {
    if crate::lifecycle::closing() {
        return Err("The app is closing".into());
    }
    // Never hide the library unless its replacement is actually visible.
    let mini = app
        .get_webview_window("mini-player")
        .ok_or("Mini player closed")?;
    mini.unminimize()
        .map_err(|_| "Cannot restore mini player")?;
    mini.show().map_err(|_| "Cannot show mini player")?;
    let ready = app
        .state::<PlayerHub>()
        .0
        .lock()
        .map_err(|_| "Player state unavailable")?
        .snapshot
        .as_ref()
        .map(|s| s.available && !s.sample)
        .unwrap_or(false);
    if ready {
        if let Some(w) = app.get_webview_window("official") {
            w.hide().map_err(|_| "Cannot hide official player")?;
        }
    }
    if let Some(main) = app.get_webview_window("main") {
        main.hide().map_err(|_| "Cannot hide library window")?;
    }
    crate::diagnostics::record(crate::diagnostics::Code::MiniOpened);
    Ok(())
}
#[tauri::command]
pub fn drag_mini_player(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "mini-player" {
        return Err("Mini player required".into());
    }
    window
        .start_dragging()
        .map_err(|_| "Cannot move mini player".into())
}
#[tauri::command]
pub async fn close_mini_player(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    if window.label() != "mini-player" {
        return Err("Mini player required".into());
    }
    super::cover::dismiss(&app);
    show_library(app, window.clone()).await?;
    window
        .close()
        .map_err(|_| "Cannot close mini player".into())
}
#[tauri::command]
pub async fn official_visibility(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    visible: bool,
) -> Result<(), String> {
    player_local(&window)?;
    super::metadata::require_account(&app)?;
    let w = app
        .get_webview_window("official")
        .ok_or("Open official playback first")?;
    if visible {
        w.show().map_err(|_| "Cannot show official player")?;
        w.set_focus().map_err(|_| "Cannot focus official player")?;
    } else {
        let playing = app
            .state::<PlayerHub>()
            .0
            .lock()
            .map_err(|_| "Player state unavailable")?
            .snapshot
            .as_ref()
            .map(|s| s.available)
            .unwrap_or(false);
        if !playing {
            return Err("Start an imported title and wait for playback controls before hiding the official window".into());
        }
        w.hide().map_err(|_| "Cannot hide official player")?;
    }
    Ok(())
}
#[tauri::command]
pub async fn show_library(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    player_local(&window)?;
    let main = app
        .get_webview_window("main")
        .ok_or("Library window closed")?;
    main.unminimize().map_err(|_| "Cannot restore library")?;
    main.show().map_err(|_| "Cannot show library")?;
    main.set_focus().map_err(|_| "Cannot focus library")?;
    Ok(())
}
