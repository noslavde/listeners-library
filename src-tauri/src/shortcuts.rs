use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub global: Vec<String>,
    pub unavailable: Vec<String>,
    pub media_keys: bool,
}
pub struct Shortcuts(pub Mutex<Status>);
impl Shortcuts {
    pub fn new() -> Self {
        Self(Mutex::new(Status::default()))
    }
}
pub fn snapshot(app: &tauri::AppHandle) -> Option<super::PlayerState> {
    if !super::metadata::connected(app) {
        return None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hub = app.state::<super::desktop::PlayerHub>();
    let state = hub.0.lock().ok()?;
    state
        .snapshot
        .as_ref()
        .filter(|s| {
            s.available && !s.sample && now.saturating_sub(state.last_seen.unwrap_or(0)) <= 3
        })
        .cloned()
}
pub fn dispatch(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    super::metadata::require_account(app)?;
    if action == "mini" {
        if let Some(w) = app.get_webview_window("mini-player") {
            if w.is_visible().unwrap_or(false) {
                return w.close().map_err(|_| "Cannot return to the library".into());
            }
        }
        if snapshot(app).is_none() {
            return Err("Start a book before opening the mini player".into());
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = super::desktop::enter_mini_player(app.clone()).await {
                report(&app, &e);
            }
        });
        return Ok(());
    }
    let s = snapshot(app).ok_or("Wait for full-book playback before using this shortcut")?;
    if action == "bookmark" {
        super::db(app)?
            .execute(
                "INSERT INTO notes(asin,position,text) VALUES(?1,?2,'Bookmark')",
                rusqlite::params![s.asin, s.position],
            )
            .map_err(|_| "Cannot save bookmark")?;
        let _ = app.emit_to("main", "notes-changed", ());
        let _ = app.emit_to(
            "main",
            "official-status",
            "Bookmark saved at the current position",
        );
        let _ = app.emit_to("mini-player", "shortcut-notice", "Bookmark saved");
        return Ok(());
    }
    let (command, value) = match action {
        "toggle" => (if s.paused { "play" } else { "pause" }, 0.0),
        "play" => ("play", 0.0),
        "pause" => ("pause", 0.0),
        "back" => ("seek", (s.position - 15.0).max(0.0)),
        "forward" => ("seek", (s.position + 15.0).min(s.duration)),
        _ => return Err("Unknown player shortcut".into()),
    };
    super::send_player_command(app.clone(), command.into(), value)
}
pub fn report(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit_to("main", "official-status", message);
    let _ = app.emit_to("mini-player", "player-error", message);
}
#[tauri::command]
pub fn player_shortcut(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    action: String,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    dispatch(&app, &action)
}
#[tauri::command]
pub fn shortcut_status(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Status, String> {
    super::desktop::player_local(&window)?;
    app.state::<Shortcuts>()
        .0
        .lock()
        .map(|v| v.clone())
        .map_err(|_| "Shortcut status unavailable".into())
}
pub fn setup(app: &mut tauri::App) {
    #[cfg(windows)]
    {
        let app = app.handle().clone();
        let _ = std::thread::Builder::new()
            .name("player-hotkeys".into())
            .spawn(move || {
                use windows::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*};
                let bindings = [
                    (1, VK_SPACE.0 as u32, "Ctrl+Alt+Space", "toggle"),
                    (2, VK_LEFT.0 as u32, "Ctrl+Alt+Left", "back"),
                    (3, VK_RIGHT.0 as u32, "Ctrl+Alt+Right", "forward"),
                    (4, b'M' as u32, "Ctrl+Alt+M", "mini"),
                    (5, b'B' as u32, "Ctrl+Alt+B", "bookmark"),
                ];
                let mut registered = Vec::new();
                let mut missing = Vec::new();
                for (id, key, label, _) in bindings {
                    if unsafe {
                        RegisterHotKey(None, id, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, key)
                    }
                    .is_ok()
                    {
                        registered.push((id, label.to_owned()));
                    } else {
                        missing.push(label.to_owned());
                    }
                }
                if let Ok(mut s) = app.state::<Shortcuts>().0.lock() {
                    s.global = registered.iter().map(|(_, label)| label.clone()).collect();
                    s.unavailable = missing;
                    let _ = app.emit_to("main", "shortcut-status", s.clone());
                    let _ = app.emit_to("mini-player", "shortcut-status", s.clone());
                }
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                    if msg.message == WM_HOTKEY {
                        if let Some((_, _, _, action)) = bindings
                            .iter()
                            .find(|(id, _, _, _)| *id == msg.wParam.0 as i32)
                        {
                            if let Err(e) = dispatch(&app, action) {
                                report(&app, &e);
                            }
                        }
                    }
                }
                for (id, _) in registered {
                    let _ = unsafe { UnregisterHotKey(None, id) };
                }
            });
    }
}
