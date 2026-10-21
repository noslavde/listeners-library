//! Validated preferences, app volume and sleep timers. Preferences use the shared
//! encrypted settings store; live observations enforce time/chapter boundaries
//! and report timer completion to local windows.

use serde::{Deserialize, Serialize};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Preferences {
    pub speed: f64,
    pub volume: f64,
    pub rewind_seconds: u32,
    pub pause_after_chapter: bool,
    pub progress_mode: String,
    pub library_sort: String,
    pub theme: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            speed: 1.0,
            volume: 1.0,
            rewind_seconds: 5,
            pause_after_chapter: false,
            progress_mode: "chapter".into(),
            library_sort: "title".into(),
            theme: "navy-gold-dark".into(),
        }
    }
}
fn valid_theme(id: &str) -> bool {
    id.strip_suffix("-dark")
        .or_else(|| id.strip_suffix("-light"))
        .map(|palette| {
            [
                "navy-gold",
                "crimson",
                "forest",
                "ocean",
                "plum",
                "black-green",
                "white-red",
            ]
            .contains(&palette)
        })
        .unwrap_or(false)
}
impl Preferences {
    fn valid(&self) -> bool {
        self.speed.is_finite()
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
            && (0.5..=3.0).contains(&self.speed)
            && self.rewind_seconds <= 30
            && matches!(self.progress_mode.as_str(), "chapter" | "book")
            && [
                "title", "author", "series", "recent", "shortest", "longest", "progress", "rating",
            ]
            .contains(&self.library_sort.as_str())
            && valid_theme(&self.theme)
    }
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Patch {
    pub speed: Option<f64>,
    pub volume: Option<f64>,
    pub rewind_seconds: Option<u32>,
    pub pause_after_chapter: Option<bool>,
    pub progress_mode: Option<String>,
    pub library_sort: Option<String>,
    pub theme: Option<String>,
}
struct Sleep {
    asin: String,
    deadline: Option<Instant>,
    chapter_end: Option<f64>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepStatus {
    mode: &'static str,
    remaining_seconds: Option<u64>,
    chapter_end: Option<f64>,
}
impl Sleep {
    fn extend(&mut self, now: Instant) -> Result<(), String> {
        let end = self
            .deadline
            .as_mut()
            .ok_or("Choose a timed sleep timer to extend it")?;
        if end.saturating_duration_since(now) > Duration::from_secs(165 * 60) {
            return Err("Sleep timer is already at its limit".into());
        }
        *end = (*end).max(now) + Duration::from_secs(15 * 60);
        Ok(())
    }
    fn status(&self, now: Instant) -> SleepStatus {
        SleepStatus {
            mode: if self.deadline.is_some() {
                "time"
            } else {
                "chapter"
            },
            remaining_seconds: self
                .deadline
                .map(|t| t.saturating_duration_since(now).as_secs_f64().ceil() as u64),
            chapter_end: self.chapter_end,
        }
    }
    fn due(&self, now: Instant, position: Option<f64>) -> bool {
        self.deadline.map(|t| now >= t).unwrap_or_else(|| {
            position
                .zip(self.chapter_end)
                .map(|(p, e)| p >= e)
                .unwrap_or(false)
        })
    }
}
pub struct Listening {
    preferences: Mutex<Preferences>,
    sleep: Mutex<Option<Sleep>>,
}
impl Listening {
    pub fn new() -> Self {
        Self {
            preferences: Mutex::new(Preferences::default()),
            sleep: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
pub struct Status {
    preferences: Preferences,
    sleep: Option<SleepStatus>,
}
fn status(app: &tauri::AppHandle) -> Result<Status, String> {
    let state = app.state::<Listening>();
    let preferences = state
        .preferences
        .lock()
        .map_err(|_| "Listening settings unavailable")?
        .clone();
    let sleep = state
        .sleep
        .lock()
        .map_err(|_| "Sleep timer unavailable")?
        .as_ref()
        .map(|s| s.status(Instant::now()));
    Ok(Status { preferences, sleep })
}
fn notify(app: &tauri::AppHandle) {
    if let Ok(v) = status(app) {
        let _ = app.emit_to("main", "listening-settings", &v);
        let _ = app.emit_to("mini-player", "listening-settings", &v);
    }
}
/// Validate the combined preference patch before persistence; live media
/// behavior is applied by the command/observation paths.
pub fn save(app: &tauri::AppHandle, patch: Patch) -> Result<(), String> {
    let state = app.state::<Listening>();
    let mut guard = state
        .preferences
        .lock()
        .map_err(|_| "Listening settings unavailable")?;
    let mut p = guard.clone();
    if let Some(v) = patch.speed {
        p.speed = v;
    }
    if let Some(v) = patch.volume {
        p.volume = v;
    }
    if let Some(v) = patch.rewind_seconds {
        p.rewind_seconds = v;
    }
    if let Some(v) = patch.pause_after_chapter {
        p.pause_after_chapter = v;
    }
    if let Some(v) = patch.progress_mode {
        p.progress_mode = v;
    }
    if let Some(v) = patch.library_sort {
        p.library_sort = v;
    }
    if let Some(v) = patch.theme {
        p.theme = v;
    }
    if !p.valid() {
        return Err("Invalid listening preference".into());
    }
    let raw = serde_json::to_string(&p).map_err(|_| "Cannot save listening settings")?;
    super::settings_db(app)?
        .execute(
            "INSERT OR REPLACE INTO preferences(key,value) VALUES('listening',?1)",
            [raw],
        )
        .map_err(|_| "Cannot save listening settings")?;
    *guard = p;
    drop(guard);
    notify(app);
    Ok(())
}
#[tauri::command]
pub fn listening_status(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Status, String> {
    super::desktop::player_local(&window)?;
    status(&app)
}
#[tauri::command]
pub fn update_listening_preferences(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    patch: Patch,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    save(&app, patch)
}

fn chapter_end(book: &serde_json::Value, position: f64, duration: f64) -> Option<f64> {
    let starts = book["chapters"].as_array()?;
    let has_current = starts
        .iter()
        .filter_map(|v| v.as_f64())
        .any(|v| v.is_finite() && v >= 0.0 && v <= position);
    if !has_current {
        return None;
    }
    starts
        .iter()
        .filter_map(|v| v.as_f64())
        .filter(|v| v.is_finite() && *v > position && *v <= duration)
        .min_by(f64::total_cmp)
        .or(Some(duration))
        .filter(|end| *end > position)
}
#[tauri::command]
/// Arm a validated time/chapter timer for the current player, distinct from
/// per-book clip replay bounds.
pub fn set_sleep_timer(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    mode: String,
    minutes: Option<u32>,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    let state = app.state::<Listening>();
    if mode == "off" {
        *state.sleep.lock().map_err(|_| "Sleep timer unavailable")? = None;
        notify(&app);
        return Ok(());
    }
    if mode == "extend" {
        let mut sleep = state.sleep.lock().map_err(|_| "Sleep timer unavailable")?;
        let timer = sleep.as_mut().ok_or("Start a timer first")?;
        timer.extend(Instant::now())?;
        drop(sleep);
        notify(&app);
        return Ok(());
    }
    let hub = app.state::<super::desktop::PlayerHub>();
    let current = hub.0.lock().map_err(|_| "Player unavailable")?;
    let s = current
        .snapshot
        .as_ref()
        .filter(|s| s.available && !s.sample)
        .ok_or("Start full-book playback first")?;
    let (deadline, end) = match mode.as_str() {
        "time" => {
            let m = minutes
                .filter(|m| [15, 30, 60].contains(m))
                .ok_or("Choose 15, 30 or 60 minutes")?;
            (
                Some(Instant::now() + Duration::from_secs(m as u64 * 60)),
                None,
            )
        }
        "chapter" => (
            None,
            Some(
                chapter_end(
                    current.book.as_ref().ok_or("Choose a book first")?,
                    s.position,
                    s.duration,
                )
                .ok_or("Chapter information is not available yet")?,
            ),
        ),
        _ => return Err("Unknown sleep timer".into()),
    };
    let asin = s.asin.clone();
    drop(current);
    *state.sleep.lock().map_err(|_| "Sleep timer unavailable")? = Some(Sleep {
        asin,
        deadline,
        chapter_end: end,
    });
    notify(&app);
    Ok(())
}
pub fn cancel_sleep(app: &tauri::AppHandle) {
    if let Ok(mut sleep) = app.state::<Listening>().sleep.lock() {
        *sleep = None;
    }
    notify(app);
}

// Reconfigure idempotently on each valid sample so new documents/media elements
// receive preferences even when their metadata becomes available after page load.
/// Update timer state from live official playback and pause at a reached
/// time/chapter boundary.
pub fn observe(app: &tauri::AppHandle, s: &super::PlayerState) {
    if !s.available || s.sample {
        return;
    }
    let Ok(p) = app
        .state::<Listening>()
        .preferences
        .lock()
        .map(|p| p.clone())
    else {
        return;
    };
    let chapters = app
        .state::<super::desktop::PlayerHub>()
        .0
        .lock()
        .ok()
        .and_then(|hub| {
            hub.book
                .as_ref()
                .and_then(|b| b["chapters"].as_array())
                .cloned()
        })
        .unwrap_or_default();
    if let Some(w) = official(app, Some(&s.asin)) {
        let config = serde_json::json!({"speed":p.speed,"volume":p.volume,"rewind":p.rewind_seconds,"pauseChapter":p.pause_after_chapter,"chapters":chapters});
        let _ = w.eval(format!("window.__audibleAdapter?.configure({config});"));
    }
}
fn official(app: &tauri::AppHandle, asin: Option<&str>) -> Option<tauri::WebviewWindow> {
    let w = app.get_webview_window("official")?;
    let u = w.url().ok()?;
    if !super::allowed(&u)
        || !matches!(u.host_str(), Some("audible.com" | "www.audible.com"))
        || u.path() != "/webplayer"
        || asin
            .map(|a| !u.query_pairs().any(|(k, v)| k == "asin" && v == a))
            .unwrap_or(false)
    {
        return None;
    }
    Some(w)
}
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use rusqlite::OptionalExtension;
    let raw: Option<String> = super::settings_db(app.handle())?
        .query_row(
            "SELECT value FROM preferences WHERE key='listening'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(p) = raw.as_deref().and_then(restore_preferences) {
        let normalized = serde_json::to_string(&p)?;
        if raw.as_deref() != Some(normalized.as_str()) {
            super::settings_db(app.handle())?.execute(
                "UPDATE preferences SET value=?1 WHERE key='listening'",
                [normalized],
            )?;
        }
        *app.state::<Listening>()
            .preferences
            .lock()
            .map_err(|_| "Listening settings unavailable")? = p;
    }
    let app = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            super::media_keys::refresh(&app);
            let state = app.state::<Listening>();
            let Ok(mut sleep) = state.sleep.lock() else {
                continue;
            };
            if let Some(timer) = sleep.as_ref() {
                let current = app
                    .state::<super::desktop::PlayerHub>()
                    .0
                    .lock()
                    .ok()
                    .map(|v| v.clone());
                let snapshot = current
                    .as_ref()
                    .and_then(|h| h.snapshot.as_ref())
                    .filter(|s| s.asin == timer.asin && s.available && !s.sample);
                if timer.due(Instant::now(), snapshot.map(|s| s.position)) {
                    app.state::<super::start_playback::StartPlayback>().cancel();
                    if let Some(w) = official(&app, Some(&timer.asin)) {
                        // Retry until a paused media report confirms the stop.
                        let _ = w.eval("window.__audibleAdapter?.command('pause',0);");
                        if snapshot.map(|s| s.paused).unwrap_or(false) {
                            *sleep = None;
                            let _ = app.emit_to(
                                "main",
                                "official-status",
                                "Sleep timer finished · playback paused",
                            );
                            let _ = app.emit_to("mini-player", "sleep-finished", ());
                        }
                    }
                }
            }
            drop(sleep);
            notify(&app);
        }
    });
    Ok(())
}

fn restore_preferences(raw: &str) -> Option<Preferences> {
    let mut p: Preferences = serde_json::from_str(raw).ok()?;
    if p.volume.is_finite() {
        p.volume = p.volume.clamp(0.0, 1.0);
    }
    p.valid().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_defaults_to_100_and_clamps_old_boost_without_resetting_other_preferences() {
        let p: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(p.volume, 1.0);
        for v in [0.0, 0.5, 1.0] {
            assert!(Preferences {
                volume: v,
                ..p.clone()
            }
            .valid());
        }
        for v in [-0.1, 1.01, 2.0, f64::NAN, f64::INFINITY] {
            assert!(!Preferences {
                volume: v,
                ..p.clone()
            }
            .valid());
        }
        let restored =
            restore_preferences(r#"{"volume":2,"speed":1.75,"theme":"plum-light"}"#).unwrap();
        assert_eq!(restored.volume, 1.0);
        assert_eq!(restored.speed, 1.75);
        assert_eq!(restored.theme, "plum-light");
        assert_eq!(
            restore_preferences(r#"{"volume":0.25}"#).unwrap().volume,
            0.25
        );
    }
    #[test]
    fn themes_validate_and_existing_settings_keep_the_original_default() {
        for palette in [
            "navy-gold",
            "crimson",
            "forest",
            "ocean",
            "plum",
            "black-green",
            "white-red",
        ] {
            for mode in ["dark", "light"] {
                assert!(valid_theme(&format!("{palette}-{mode}")));
            }
        }
        assert!(!valid_theme("unknown-dark"));
        assert!(!valid_theme("forest-system"));
        let old: Preferences = serde_json::from_str(r#"{"speed":1.5,"rewindSeconds":10}"#).unwrap();
        assert!(old.valid());
        assert_eq!(old.theme, "navy-gold-dark");
        assert_eq!(old.speed, 1.5);
    }
    #[test]
    fn extending_keeps_existing_time_and_can_rescue_an_expired_timer() {
        let now = Instant::now();
        let mut timer = Sleep {
            asin: "B012345678".into(),
            deadline: Some(now + Duration::from_secs(600)),
            chapter_end: None,
        };
        timer.extend(now).unwrap();
        assert_eq!(timer.status(now).remaining_seconds, Some(1500));
        let later = now + Duration::from_secs(2000);
        timer.extend(later).unwrap();
        assert_eq!(timer.status(later).remaining_seconds, Some(900));
        assert!(!timer.due(later, None));
        timer.deadline = Some(now + Duration::from_secs(180 * 60));
        assert!(timer.extend(now).is_err());
        timer.deadline = None;
        timer.chapter_end = Some(200.0);
        assert!(timer.extend(now).is_err());
    }
    #[test]
    fn preferences_roundtrip_and_validate() {
        let p = Preferences {
            speed: 1.75,
            volume: 1.0,
            rewind_seconds: 15,
            pause_after_chapter: true,
            progress_mode: "book".into(),
            library_sort: "recent".into(),
            theme: "forest-light".into(),
        };
        let saved = serde_json::to_string(&p).unwrap();
        let loaded: Preferences = serde_json::from_str(&saved).unwrap();
        assert!(loaded.valid());
        assert!(Preferences {
            library_sort: "rating".into(),
            ..p.clone()
        }
        .valid());
        assert_eq!(loaded.speed, 1.75);
        assert!(loaded.pause_after_chapter);
        assert!(!Preferences {
            speed: 99.0,
            ..p.clone()
        }
        .valid());
        assert!(!Preferences {
            rewind_seconds: 999,
            ..p.clone()
        }
        .valid());
        assert!(!Preferences {
            progress_mode: "bad".into(),
            ..p
        }
        .valid());
    }
    #[test]
    fn timed_sleep_uses_elapsed_time_not_playback_speed() {
        let now = Instant::now();
        let timer = Sleep {
            asin: "B012345678".into(),
            deadline: Some(now + Duration::from_secs(900)),
            chapter_end: None,
        };
        assert!(!timer.due(now + Duration::from_secs(899), Some(10000.0)));
        assert!(timer.due(now + Duration::from_secs(900), None));
        assert_eq!(timer.status(now).remaining_seconds, Some(900));
    }
    #[test]
    fn chapter_timer_requires_known_chapters_and_a_future_boundary() {
        let b = serde_json::json!({"chapters":[0,100,300]});
        assert_eq!(chapter_end(&b, 50.0, 500.0), Some(100.0));
        assert_eq!(chapter_end(&b, 100.0, 500.0), Some(300.0));
        assert_eq!(chapter_end(&b, 350.0, 500.0), Some(500.0));
        assert_eq!(chapter_end(&b, 500.0, 500.0), None);
        assert_eq!(
            chapter_end(&serde_json::json!({"chapters":[]}), 50.0, 500.0),
            None
        );
        let timer = Sleep {
            asin: "B012345678".into(),
            deadline: None,
            chapter_end: Some(100.0),
        };
        assert!(!timer.due(Instant::now(), Some(99.0)));
        assert!(!timer.due(Instant::now(), None));
        assert!(timer.due(Instant::now(), Some(100.0)));
    }
}
