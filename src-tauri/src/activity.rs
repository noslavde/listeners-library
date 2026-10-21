//! Meaningful-listening credit for the Continue shelf. Samples natural playback
//! progression, rejects seek jumps and pins pending credit to the captured
//! account generation before persistence.

use serde::Serialize;
use std::{sync::Mutex, time::Instant};
use tauri::{Emitter, Manager};
#[derive(Clone)]
struct Sample {
    asin: String,
    position: f64,
    speed: f64,
    playing: bool,
    at: Instant,
}
#[derive(Clone)]
struct Pending {
    generation: u64,
    asin: String,
    seconds: f64,
    last_listened: i64,
    position: f64,
    duration: f64,
}
pub struct Activity(Mutex<(Option<Sample>, Option<Pending>)>);
impl Activity {
    pub fn new() -> Self {
        Self(Mutex::new((None, None)))
    }
}
fn credit(previous: &Sample, current: &super::PlayerState, now: Instant) -> f64 {
    let elapsed = now.saturating_duration_since(previous.at).as_secs_f64();
    let advanced = current.position - previous.position;
    if previous.asin != current.asin
        || !previous.playing
        || !current.available
        || current.sample
        || elapsed > 3.0
        || elapsed <= 0.0
        || advanced <= 0.0
        || advanced > elapsed * previous.speed + 2.0
    {
        return 0.0;
    }
    elapsed.min(advanced / previous.speed)
}
fn persist(app: &tauri::AppHandle, p: Pending) {
    // Never resolve the destination from a later account session.
    if super::metadata::with_account_db(app, p.generation, |c| {
        write_activity(c, &p).map_err(|_| "Cannot save listening activity".into())
    })
    .is_ok()
    {
        let _ = app.emit_to("main", "library-activity-changed", ());
    }
}
pub fn reset(app: &tauri::AppHandle) {
    if let Ok(mut tracker) = app.state::<Activity>().0.lock() {
        *tracker = (None, None);
    }
}
/// Flush pending listening credit for its captured account before abandoning
/// the current sample.
pub(crate) fn finish(app: &tauri::AppHandle) {
    if let Ok(mut tracker) = app.state::<Activity>().0.lock() {
        if let Some(p) = tracker.1.take() {
            persist(app, p);
        }
        tracker.0 = None;
    }
}
fn write_activity(c: &rusqlite::Connection, p: &Pending) -> rusqlite::Result<usize> {
    c.execute("INSERT INTO listening_activity(asin,listened_seconds,last_listened,position,duration,hidden) VALUES(?1,?2,?3,?4,?5,0) ON CONFLICT(asin) DO UPDATE SET listened_seconds=listened_seconds+excluded.listened_seconds,position=CASE WHEN excluded.last_listened>=last_listened THEN excluded.position ELSE position END,duration=excluded.duration,last_listened=MAX(last_listened,excluded.last_listened)",rusqlite::params![p.asin,p.seconds,p.last_listened,p.position,p.duration])
}
/// Count plausible natural playback deltas only; seeking and cloud refreshes
/// must not populate Continue listening by themselves.
pub fn observe(app: &tauri::AppHandle, s: &super::PlayerState) {
    let Ok(generation) = super::metadata::account_generation(app) else {
        return;
    };
    let state = app.state::<Activity>();
    let Ok(mut tracker) = state.0.lock() else {
        return;
    };
    let now = Instant::now();
    if tracker
        .1
        .as_ref()
        .map(|p| p.asin != s.asin || p.generation != generation)
        .unwrap_or(false)
    {
        if let Some(p) = tracker.1.take() {
            persist(app, p);
        }
    }
    let seconds = tracker
        .0
        .as_ref()
        .map(|prev| credit(prev, s, now))
        .unwrap_or(0.0);
    if seconds > 0.0 {
        if let Err(e) = super::history::heard(app, s) {
            let _ = app.emit_to("mini-player", "player-error", e);
        }
        let p = tracker.1.get_or_insert_with(|| Pending {
            generation,
            asin: s.asin.clone(),
            seconds: 0.0,
            last_listened: 0,
            position: s.position,
            duration: s.duration,
        });
        p.seconds += seconds;
        p.last_listened = epoch();
        p.position = s.position;
        p.duration = s.duration;
    }
    tracker.0 = Some(Sample {
        asin: s.asin.clone(),
        position: s.position,
        speed: s.speed,
        playing: s.available && !s.sample && !s.paused,
        at: now,
    });
    if tracker
        .1
        .as_ref()
        .map(|p| p.seconds >= 10.0 || s.paused || !s.available)
        .unwrap_or(false)
    {
        if let Some(p) = tracker.1.take() {
            persist(app, p);
        }
    }
}
fn epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    listened_seconds: f64,
    last_listened: i64,
    position: f64,
    duration: f64,
    hidden: bool,
}
#[tauri::command]
pub fn library_activity(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<std::collections::HashMap<String, Entry>, String> {
    super::local(&window)?;
    if !super::metadata::connected(&app) {
        return Ok(std::collections::HashMap::new());
    }
    let c = super::db(&app)?;
    let mut q=c.prepare("SELECT asin,listened_seconds,last_listened,position,duration,hidden FROM listening_activity").map_err(|_|"Cannot read listening history")?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Entry {
                    listened_seconds: r.get(1)?,
                    last_listened: r.get(2)?,
                    position: r.get(3)?,
                    duration: r.get(4)?,
                    hidden: r.get(5)?,
                },
            ))
        })
        .map_err(|_| "Cannot read listening history")?;
    rows.collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|_| "Invalid listening history".into())
}
#[tauri::command]
pub fn dismiss_continue(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
    hidden: bool,
) -> Result<(), String> {
    super::local(&window)?;
    if !super::valid_asin(&asin) {
        return Err("Invalid title".into());
    }
    set_hidden(&app, &asin, hidden)
}
pub fn set_hidden(app: &tauri::AppHandle, asin: &str, hidden: bool) -> Result<(), String> {
    super::db(app)?
        .execute(
            "UPDATE listening_activity SET hidden=?1 WHERE asin=?2",
            rusqlite::params![hidden, asin],
        )
        .map_err(|_| "Cannot update Continue listening")?;
    let _ = app.emit_to("main", "library-activity-changed", ());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_history_flush_cannot_restore_a_dismissed_book_or_replace_newer_progress() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE listening_activity(asin TEXT PRIMARY KEY,listened_seconds REAL,last_listened INTEGER,position REAL,duration REAL,hidden INTEGER);").unwrap();
        let recent = Pending {
            generation: 0,
            asin: "B012345678".into(),
            seconds: 300.0,
            last_listened: 200,
            position: 800.0,
            duration: 1000.0,
        };
        write_activity(&c, &recent).unwrap();
        c.execute("UPDATE listening_activity SET hidden=1", [])
            .unwrap();
        write_activity(
            &c,
            &Pending {
                seconds: 10.0,
                last_listened: 100,
                position: 500.0,
                ..recent
            },
        )
        .unwrap();
        let row: (f64, i64, f64, bool) = c
            .query_row(
                "SELECT listened_seconds,last_listened,position,hidden FROM listening_activity",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(row, (310.0, 200, 800.0, true));
    }
    fn state(position: f64) -> super::super::PlayerState {
        serde_json::from_value(serde_json::json!({"asin":"B012345678","position":position,"duration":1000,"speed":2,"paused":false,"available":true})).unwrap()
    }
    #[test]
    fn counts_listening_but_not_seek_stall_pause_or_cloud_refresh() {
        let now = Instant::now();
        let prev = Sample {
            asin: "B012345678".into(),
            position: 100.0,
            speed: 2.0,
            playing: true,
            at: now,
        };
        assert_eq!(
            credit(
                &prev,
                &state(101.0),
                now + std::time::Duration::from_millis(500)
            ),
            0.5
        );
        assert_eq!(
            credit(
                &prev,
                &state(800.0),
                now + std::time::Duration::from_millis(500)
            ),
            0.0
        );
        assert_eq!(
            credit(
                &prev,
                &state(100.0),
                now + std::time::Duration::from_secs(1)
            ),
            0.0
        );
        assert_eq!(
            credit(
                &Sample {
                    playing: false,
                    ..prev.clone()
                },
                &state(101.0),
                now + std::time::Duration::from_secs(1)
            ),
            0.0
        );
        assert_eq!(
            credit(
                &prev,
                &state(101.0),
                now + std::time::Duration::from_secs(60)
            ),
            0.0
        );
    }
}
