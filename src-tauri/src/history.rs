//! Encrypted listening checkpoints, bounded history and startup conflict
//! protection. Saves confirmed listening/explicit choices, holds startup when
//! positions disagree and never treats metadata-read time as a cloud listening
//! timestamp.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};

const PER_BOOK: i64 = 100;
const TOTAL: i64 = 3000;
const RETENTION_MS: i64 = 90 * 24 * 60 * 60 * 1000;

#[derive(Clone, Serialize)]
pub struct Entry {
    id: i64,
    position: f64,
    duration: f64,
    at: i64,
    kind: String,
}
#[derive(Clone)]
struct Point {
    asin: String,
    generation: u64,
    entry: Entry,
}
struct Gate {
    asin: String,
    pc: Option<Entry>,
    audible: Option<f64>,
    ready_at: Option<Instant>,
}
#[derive(Debug, PartialEq)]
enum Decision {
    Wait,
    Accept,
    Review,
}
impl Gate {
    fn observe(&mut self, v: &super::PlayerState, now: Instant) -> Decision {
        if self.asin != v.asin || !v.available || v.sample {
            return Decision::Wait;
        }
        if self.pc.is_none() {
            return Decision::Accept;
        }
        let ready = self.ready_at.get_or_insert(now);
        if now.saturating_duration_since(*ready).as_millis() < 750 {
            return Decision::Wait;
        }
        if self.audible.is_some() {
            return Decision::Review;
        }
        if self
            .pc
            .as_ref()
            .is_some_and(|pc| (pc.position - v.position).abs() > 15.0)
        {
            self.audible = Some(v.position);
            Decision::Review
        } else {
            Decision::Accept
        }
    }
}
#[derive(Default)]
struct State {
    last_at: i64,
    latest: Option<Point>,
    written: Option<Instant>,
    logged: Option<Instant>,
    paused: bool,
    gate: Option<Gate>,
}
#[derive(Default)]
pub struct History(Mutex<State>);

fn epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub(crate) fn schema(c: &Connection) -> Result<(), String> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS progress_checkpoints(asin TEXT PRIMARY KEY,position REAL NOT NULL,duration REAL NOT NULL,observed_ms INTEGER NOT NULL,kind TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS listening_log(id INTEGER PRIMARY KEY,asin TEXT NOT NULL,position REAL NOT NULL,duration REAL NOT NULL,observed_ms INTEGER NOT NULL,kind TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS listening_log_book_time ON listening_log(asin,observed_ms DESC,id DESC);")
        .map_err(|_| "Cannot prepare listening recovery history".into())
}
fn entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        position: row.get(1)?,
        duration: row.get(2)?,
        at: row.get(3)?,
        kind: row.get(4)?,
    })
}
fn checkpoint(c: &Connection, asin: &str) -> Result<Option<Entry>, String> {
    c.query_row(
        "SELECT 0,position,duration,observed_ms,kind FROM progress_checkpoints WHERE asin=?1",
        [asin],
        entry,
    )
    .optional()
    .map_err(|_| "Cannot read saved listening position".into())
}
fn write(c: &Connection, p: &Point, log: bool) -> Result<(), String> {
    let e = &p.entry;
    c.execute_batch("SAVEPOINT listening_checkpoint")
        .map_err(|_| "Cannot save listening position")?;
    let result = (|| -> rusqlite::Result<()> {
        let updated=c.execute("INSERT INTO progress_checkpoints VALUES(?1,?2,?3,?4,?5) ON CONFLICT(asin) DO UPDATE SET position=excluded.position,duration=excluded.duration,observed_ms=excluded.observed_ms,kind=excluded.kind WHERE excluded.observed_ms>=observed_ms",params![p.asin,e.position,e.duration,e.at,e.kind])?;
        if updated > 0 {
            c.execute("INSERT INTO playback_positions VALUES(?1,?2,?3) ON CONFLICT(asin) DO UPDATE SET position=excluded.position,updated=excluded.updated",params![p.asin,e.position,e.at/1000])?;
        }
        if log {
            c.execute("INSERT INTO listening_log(asin,position,duration,observed_ms,kind) SELECT ?1,?2,?3,?4,?5 WHERE NOT EXISTS(SELECT 1 FROM listening_log WHERE asin=?1 AND position=?2 AND observed_ms=?4 AND kind=?5)",params![p.asin,e.position,e.duration,e.at,e.kind])?;
            c.execute("DELETE FROM listening_log WHERE asin=?1 AND id NOT IN(SELECT id FROM listening_log WHERE asin=?1 ORDER BY observed_ms DESC,id DESC LIMIT ?2)",params![p.asin,PER_BOOK])?;
            c.execute("DELETE FROM listening_log WHERE observed_ms<?1 OR id NOT IN(SELECT id FROM listening_log ORDER BY observed_ms DESC,id DESC LIMIT ?2)",params![e.at.saturating_sub(RETENTION_MS),TOTAL])?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = c.execute_batch("ROLLBACK TO listening_checkpoint; RELEASE listening_checkpoint");
        return Err(
            "Could not save listening recovery history; your previous checkpoints were retained."
                .into(),
        );
    }
    c.execute_batch("RELEASE listening_checkpoint")
        .map_err(|_| "Cannot commit listening position".into())
}
fn persist(app: &tauri::AppHandle, p: Point, log: bool) -> Result<(), String> {
    super::metadata::with_account_db(app, p.generation, |c| {
        schema(c)?;
        write(c, &p, log)
    })?;
    let _ = app.emit_to("main", "card-positions-changed", ());
    if log {
        notify(app);
    }
    Ok(())
}
fn write_choice(c: &Connection, p: &Point) -> Result<(), String> {
    if let Some(previous) = checkpoint(c, &p.asin)? {
        write(
            c,
            &Point {
                asin: p.asin.clone(),
                generation: p.generation,
                entry: previous,
            },
            true,
        )?;
    }
    write(c, p, true)
}
fn notify(app: &tauri::AppHandle) {
    for label in ["main", "mini-player"] {
        let _ = app.emit_to(label, "listening-history-changed", ());
    }
}
pub(crate) fn reset(app: &tauri::AppHandle) {
    if let Ok(mut s) = app.state::<History>().0.lock() {
        *s = State::default();
    }
}

// Only activity::credit-confirmed advancement enters this function. Loading,
// polling a cloud position, and an unconfirmed seek never become "heard now".
/// Credit confirmed natural listening observations; initial loading or paused
/// telemetry must not overwrite an existing checkpoint.
pub(crate) fn heard(app: &tauri::AppHandle, v: &super::PlayerState) -> Result<(), String> {
    let generation = super::metadata::account_generation(app)?;
    let pending = {
        let history = app.state::<History>();
        let mut s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        if s.gate.is_some() {
            return Ok(());
        }
        let at = epoch_ms().max(s.last_at.saturating_add(1));
        s.last_at = at;
        let p = Point {
            asin: v.asin.clone(),
            generation,
            entry: Entry {
                id: 0,
                position: v.position,
                duration: v.duration,
                at,
                kind: "listening".into(),
            },
        };
        s.latest = Some(p.clone());
        let now = Instant::now();
        let log = s
            .logged
            .is_none_or(|t| now.duration_since(t).as_secs() >= 60);
        if log
            || s.written
                .is_none_or(|t| now.duration_since(t).as_secs() >= 5)
        {
            s.written = Some(now);
            if log {
                s.logged = Some(now);
            }
            Some((p, log))
        } else {
            None
        }
    };
    if let Some((p, log)) = pending {
        persist(app, p, log)?;
    }
    Ok(())
}
/// Persist pending confirmed position/history for a pause, switch or close; no
/// new listening is invented merely because shutdown occurs.
pub(crate) fn flush(app: &tauri::AppHandle, kind: &str) -> Result<(), String> {
    let p = {
        let history = app.state::<History>();
        let s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        // Flush the time of the last actual advancement, not the time of this call.
        let p = s.latest.clone().map(|mut p| {
            p.entry.kind = kind.into();
            p
        });
        p
    };
    if let Some(p) = p {
        persist(app, p.clone(), true)?;
        let history = app.state::<History>();
        let mut s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        if s.latest.as_ref().is_some_and(|v| {
            v.asin == p.asin && v.generation == p.generation && v.entry.at == p.entry.at
        }) {
            s.latest = None;
            s.written = None;
            s.logged = None;
        }
    }
    Ok(())
}
pub(crate) fn settled(app: &tauri::AppHandle, v: &super::PlayerState) {
    let pause = {
        let history = app.state::<History>();
        let Ok(mut s) = history.0.lock() else { return };
        let changed = v.paused && !s.paused;
        s.paused = v.paused;
        changed
    };
    if pause {
        if let Err(e) = flush(app, "pause") {
            let _ = app.emit_to("mini-player", "player-error", e);
        }
    }
}
fn seed(c: &Connection, asin: &str, duration: f64) -> Result<(), String> {
    if checkpoint(c, asin)?.is_some() {
        return Ok(());
    }
    let mut candidates = Vec::new();
    for (query, kind) in [
        (
            "SELECT position,updated FROM playback_positions WHERE asin=?1",
            "previous-PC-checkpoint",
        ),
        (
            "SELECT position,last_listened FROM listening_activity WHERE asin=?1",
            "previous-listening",
        ),
    ] {
        if let Some((position, at)) = c
            .query_row(query, [asin], |r| {
                Ok((r.get::<_, f64>(0)?, r.get::<_, i64>(1)?))
            })
            .optional()
            .map_err(|_| "Cannot preserve older listening positions")?
        {
            if position.is_finite() && position >= 0.0 && at > 0 {
                candidates.push(Entry {
                    id: 0,
                    position,
                    duration,
                    at: at.saturating_mul(1000),
                    kind: kind.into(),
                });
            }
        }
    }
    candidates.sort_by_key(|e| e.at);
    // Older playback_positions timestamps include paused player observations.
    // Prefer credited listening, while retaining both as recovery candidates.
    let preferred = candidates
        .iter()
        .find(|e| e.kind == "previous-listening")
        .cloned()
        .or_else(|| candidates.last().cloned());
    c.execute_batch("SAVEPOINT recovery_seed")
        .map_err(|_| "Cannot preserve older progress")?;
    let result = (|| -> Result<(), String> {
        for e in candidates {
            write(
                c,
                &Point {
                    asin: asin.into(),
                    generation: 0,
                    entry: e,
                },
                true,
            )?;
        }
        if let Some(e) = preferred {
            c.execute("UPDATE progress_checkpoints SET position=?1,duration=?2,observed_ms=?3,kind=?4 WHERE asin=?5",params![e.position,e.duration,e.at,e.kind,asin]).map_err(|_|"Cannot preserve confirmed listening")?;
            c.execute(
                "UPDATE playback_positions SET position=?1,updated=?2 WHERE asin=?3",
                params![e.position, e.at / 1000, asin],
            )
            .map_err(|_| "Cannot preserve confirmed listening")?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = c.execute_batch("ROLLBACK TO recovery_seed; RELEASE recovery_seed");
        return Err(e);
    }
    c.execute_batch("RELEASE recovery_seed")
        .map_err(|_| "Cannot finish progress preservation".into())
}
/// Prepare a book/account-specific startup comparison before playback is
/// released.
pub(crate) fn prepare(app: &tauri::AppHandle, asin: &str, generation: u64) -> Result<(), String> {
    flush(app, "switch")?;
    super::metadata::with_account_db(app, generation, |c| {
        schema(c)?;
        seed(c, asin, 0.0)?;
        let pc = checkpoint(c, asin)?;
        let history = app.state::<History>();
        let mut s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        *s = State {
            last_at: pc.as_ref().map(|e| e.at).unwrap_or(0),
            gate: Some(Gate {
                asin: asin.into(),
                pc,
                audible: None,
                ready_at: None,
            }),
            ..State::default()
        };
        Ok(())
    })
}
pub(crate) fn blocked(app: &tauri::AppHandle) -> bool {
    app.state::<History>()
        .0
        .lock()
        .map(|s| s.gate.is_some())
        .unwrap_or(true)
}
/// Decide whether startup remains held for a saved-position choice; callers
/// must honor this before auto-play or mini-window transitions.
pub(crate) fn guard_start(app: &tauri::AppHandle, v: &super::PlayerState) -> bool {
    let mut announce = false;
    let held = {
        let history = app.state::<History>();
        let Ok(mut s) = history.0.lock() else {
            return true;
        };
        if let Some(gate) = s.gate.as_mut() {
            let was_review = gate.audible.is_some();
            match gate.observe(v, Instant::now()) {
                Decision::Wait => true,
                Decision::Review => {
                    announce = !was_review;
                    true
                }
                Decision::Accept => {
                    s.gate = None;
                    announce = true;
                    false
                }
            }
        } else {
            false
        }
    };
    if announce {
        notify(app);
    }
    held
}

#[derive(Serialize)]
pub struct Conflict {
    pc: Entry,
    audible: f64,
}
#[derive(Serialize)]
pub struct Status {
    asin: String,
    generation: u64,
    waiting: bool,
    conflict: Option<Conflict>,
    entries: Vec<Entry>,
}
fn current(app: &tauri::AppHandle, asin: &str) -> Result<super::PlayerState, String> {
    let hub = app.state::<super::desktop::PlayerHub>();
    let hub = hub.0.lock().map_err(|_| "Player unavailable")?;
    let s = hub.snapshot.as_ref().ok_or("Wait for playback to load")?;
    if hub.book.as_ref().and_then(|b| b["asin"].as_str()) != Some(asin)
        || s.asin != asin
        || s.sample
        || !s.available
    {
        return Err("The playing book changed; recovery canceled".into());
    }
    Ok(s.clone())
}
#[tauri::command]
pub fn listening_history(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<Status, String> {
    super::desktop::player_local(&window)?;
    let generation = super::metadata::account_generation(&app)?;
    if !super::valid_asin(&asin) {
        return Err("Invalid title".into());
    }
    let hub = app.state::<super::desktop::PlayerHub>();
    if hub
        .0
        .lock()
        .map_err(|_| "Player unavailable")?
        .book
        .as_ref()
        .and_then(|b| b["asin"].as_str())
        != Some(asin.as_str())
    {
        return Err("Select this book before reading its listening history".into());
    }
    let (waiting, conflict) = {
        let history = app.state::<History>();
        let s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        (
            s.gate.as_ref().is_some_and(|g| g.asin == asin),
            s.gate.as_ref().filter(|g| g.asin == asin).and_then(|g| {
                Some(Conflict {
                    pc: g.pc.clone()?,
                    audible: g.audible?,
                })
            }),
        )
    };
    let entries = super::metadata::with_account_db(&app, generation, |c| {
        schema(c)?;
        let mut q=c.prepare("SELECT id,position,duration,observed_ms,kind FROM listening_log WHERE asin=?1 ORDER BY observed_ms DESC,id DESC LIMIT 30").map_err(|_|"Cannot read listening log")?;
        let rows = q
            .query_map([&asin], entry)
            .map_err(|_| "Cannot read listening log")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot read listening log".into())
    })?;
    Ok(Status {
        asin,
        generation,
        waiting,
        conflict,
        entries,
    })
}
/// Record an explicitly confirmed position choice, preserving a useful recovery
/// trail rather than treating it as passive listening.
pub(crate) fn chosen(
    app: &tauri::AppHandle,
    asin: &str,
    generation: u64,
    revision: u64,
) -> Result<(), String> {
    super::metadata::with_account_db(app, generation, |c| {
        schema(c)?;
        let hub = app.state::<super::desktop::PlayerHub>();
        let hub_state = hub.0.lock().map_err(|_| "Player unavailable")?;
        let v = hub_state.snapshot.as_ref().ok_or("Wait for playback")?;
        if hub.revision() != revision
            || hub_state.book.as_ref().and_then(|b| b["asin"].as_str()) != Some(asin)
            || v.asin != asin
            || !v.available
            || v.sample
        {
            return Err("Playback changed; recovery canceled.".into());
        }
        let p = Point {
            asin: asin.into(),
            generation,
            entry: Entry {
                id: 0,
                position: v.position,
                duration: v.duration,
                at: epoch_ms().max(
                    checkpoint(c, asin)?
                        .map(|e| e.at.saturating_add(1))
                        .unwrap_or(0),
                ),
                kind: "chosen-position".into(),
            },
        };
        write_choice(c, &p)?;
        let history = app.state::<History>();
        let mut s = history
            .0
            .lock()
            .map_err(|_| "Listening history unavailable")?;
        if s.gate.as_ref().is_some_and(|g| g.asin == asin) {
            s.gate = None;
        }
        s.latest = None;
        s.written = None;
        s.logged = None;
        s.last_at = p.entry.at;
        Ok(())
    })?;
    notify(app);
    Ok(())
}
#[tauri::command]
pub fn accept_audible_position(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
    generation: u64,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    super::metadata::with_account_generation(&app, generation, || Ok(()))?;
    let revision = app.state::<super::desktop::PlayerHub>().revision();
    chosen(&app, &asin, generation, revision)
}
#[tauri::command]
/// Restore a selected dated checkpoint only after native seek confirmation for
/// the same account/book/player lifetime.
pub async fn restore_listening_position(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
    generation: u64,
    id: i64,
) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    current(&app, &asin)?;
    let e = super::metadata::with_account_db(&app, generation, |c| {
        schema(c)?;
        if id == 0 {
            return checkpoint(c, &asin)?.ok_or("Saved PC position is unavailable".into());
        }
        c.query_row("SELECT id,position,duration,observed_ms,kind FROM listening_log WHERE asin=?1 AND id=?2",params![asin,id],entry).map_err(|_|"That recovery entry is no longer available".into())
    })?;
    let revision =
        super::player_sync::seek_saved_position(&app, &asin, generation, e.position).await?;
    chosen(&app, &asin, generation, revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn playback(position: f64) -> super::super::PlayerState {
        serde_json::from_value(serde_json::json!({"asin":"B012345678","position":position,"duration":1000,"speed":1,"paused":true,"available":true})).unwrap()
    }
    #[test]
    fn stale_or_ahead_player_loads_require_a_choice_and_cannot_replace_the_pc_candidate() {
        let now = Instant::now();
        for opened in [100.0, 950.0] {
            let mut gate = Gate {
                asin: "B012345678".into(),
                pc: Some(point(800.0, 10000).entry),
                audible: None,
                ready_at: None,
            };
            assert_eq!(gate.observe(&playback(opened), now), Decision::Wait);
            assert_eq!(
                gate.observe(&playback(opened), now + std::time::Duration::from_secs(1)),
                Decision::Review
            );
            for i in 2..5 {
                assert_eq!(
                    gate.observe(&playback(opened), now + std::time::Duration::from_secs(i)),
                    Decision::Review
                );
            }
            assert_eq!(gate.pc.unwrap().position, 800.0);
        }
    }
    #[test]
    fn first_listen_and_matching_resume_do_not_need_user_approval() {
        let now = Instant::now();
        let mut gate = Gate {
            asin: "B012345678".into(),
            pc: None,
            audible: None,
            ready_at: None,
        };
        let mut sample = playback(0.0);
        sample.sample = true;
        assert_eq!(gate.observe(&sample, now), Decision::Wait);
        assert_eq!(gate.observe(&playback(0.0), now), Decision::Accept);
        gate.pc = Some(point(800.0, 10000).entry);
        assert_eq!(gate.observe(&playback(805.0), now), Decision::Wait);
        assert_eq!(
            gate.observe(&playback(805.0), now + std::time::Duration::from_secs(1)),
            Decision::Accept
        );
    }
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE playback_positions(asin TEXT PRIMARY KEY,position REAL,updated INTEGER);CREATE TABLE listening_activity(asin TEXT PRIMARY KEY,position REAL,last_listened INTEGER);").unwrap();
        schema(&c).unwrap();
        c
    }
    fn point(position: f64, at: i64) -> Point {
        Point {
            asin: "B012345678".into(),
            generation: 0,
            entry: Entry {
                id: 0,
                position,
                duration: 1000.0,
                at,
                kind: "listening".into(),
            },
        }
    }
    #[test]
    fn latest_means_time_not_furthest_and_late_writes_cannot_roll_it_back() {
        let c = db();
        write(&c, &point(900.0, 100000), true).unwrap();
        write(&c, &point(200.0, 200000), true).unwrap();
        write(&c, &point(950.0, 150000), true).unwrap();
        write(&c, &point(950.0, 199999), true).unwrap();
        assert_eq!(
            checkpoint(&c, "B012345678").unwrap().unwrap().position,
            200.0
        );
        assert_eq!(
            c.query_row(
                "SELECT position FROM playback_positions WHERE asin='B012345678'",
                [],
                |r| r.get::<_, f64>(0)
            )
            .unwrap(),
            200.0
        );
    }
    #[test]
    fn choosing_an_older_position_keeps_the_last_checkpoint_for_undo_without_duplicate_log_rows() {
        let c = db();
        let old = point(800.0, 100000);
        write(&c, &old, false).unwrap();
        let mut choice = point(100.0, 200000);
        choice.entry.kind = "chosen-position".into();
        write_choice(&c, &choice).unwrap();
        write_choice(&c, &choice).unwrap();
        assert_eq!(
            checkpoint(&c, "B012345678").unwrap().unwrap().position,
            100.0
        );
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM listening_log WHERE position=800",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM listening_log", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn bounds_log_size_per_book_globally_and_by_age() {
        let c = db();
        for i in 0..130 {
            write(&c, &point(i as f64, 100000 + i), true).unwrap();
        }
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM listening_log", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            PER_BOOK
        );
        write(&c, &point(500.0, RETENTION_MS + 1000000), true).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM listening_log", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        for n in 0..31 {
            let mut p = point(1.0, RETENTION_MS + 2000000);
            p.asin = format!("B{n:09}");
            for i in 0..100 {
                p.entry.at += i;
                write(&c, &p, true).unwrap();
            }
        }
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM listening_log", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            TOTAL
        );
    }
    #[test]
    fn migration_preserves_both_old_positions_once() {
        let c = db();
        c.execute(
            "INSERT INTO playback_positions VALUES('B012345678',100,200)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO listening_activity VALUES('B012345678',800,199)",
            [],
        )
        .unwrap();
        seed(&c, "B012345678", 1000.0).unwrap();
        seed(&c, "B012345678", 1000.0).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM listening_log", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            checkpoint(&c, "B012345678").unwrap().unwrap().position,
            800.0
        );
    }
}
