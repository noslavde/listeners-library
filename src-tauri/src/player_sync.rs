use serde::Serialize;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::Manager;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn ready<'a>(
    hub: &'a super::desktop::NowPlaying,
    asin: &str,
    now: u64,
) -> Result<&'a super::PlayerState, String> {
    if hub.book.as_ref().and_then(|b| b["asin"].as_str()) != Some(asin) {
        return Err("The book changed; position sync canceled.".into());
    }
    let s = hub
        .snapshot
        .as_ref()
        .ok_or("Wait for playback to load, then sync again.")?;
    if s.asin != asin
        || !s.available
        || s.sample
        || s.duration <= 0.0
        || now.saturating_sub(hub.last_seen.unwrap_or(0)) > 3
    {
        return Err("Wait for full-book playback to load, then sync again.".into());
    }
    Ok(s)
}

fn valid_target(s: &super::PlayerState, position: f64) -> Result<(), String> {
    if !position.is_finite() || position < 0.0 || position > s.duration {
        return Err(
            "Audible's saved position does not fit this book's timeline; no seek was sent.".into(),
        );
    }
    if !s
        .seekable
        .iter()
        .any(|r| position >= r.start && position <= r.end)
    {
        return Err("Audible's saved position is not seekable yet. Show the official player or wait for loading, then sync again.".into());
    }
    Ok(())
}

#[derive(Serialize)]
pub struct SyncResult {
    position: f64,
}

/// Read the provider first. Never publish a local checkpoint as part of a handoff.
/// Every asynchronous boundary rechecks both the account and the player revision.
#[tauri::command]
pub async fn resync_player(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<SyncResult, String> {
    super::desktop::player_local(&window)?;
    let generation = super::metadata::account_generation(&app)?;
    if !super::valid_asin(&asin) {
        return Err("Invalid book".into());
    }
    let hub = app.state::<super::desktop::PlayerHub>();
    let revision = {
        let state = hub.0.lock().map_err(|_| "Player state unavailable")?;
        ready(&state, &asin, now())?;
        hub.changed()
    };
    let position = tokio::time::timeout(
        Duration::from_secs(20),
        super::metadata::latest_position(&app, &asin, generation),
    )
    .await
    .map_err(|_| "Audible took too long to return a position. Check your connection and try again.")??;
    super::metadata::with_account_generation(&app, generation, || {
        let state = hub.0.lock().map_err(|_| "Player state unavailable")?;
        if hub.revision() != revision {
            return Err(
                "Playback changed while checking Audible; sync canceled. Try again when ready."
                    .into(),
            );
        }
        let s = ready(&state, &asin, now())?;
        valid_target(s, position)?;
        let official = app
            .get_webview_window("official")
            .ok_or("Open the official player first")?;
        let url = official.url().map_err(|_| "Player page unavailable")?;
        if !super::allowed(&url)
            || !matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
            || url.path() != "/webplayer"
        {
            return Err("Open full-book playback before syncing.".into());
        }
        let book = serde_json::to_string(&asin).map_err(|_| "Invalid book")?;
        // Recheck the actual page at execution time, after WebView's eval queue.
        official.eval(format!("if(new URL(location.href).searchParams.get('asin')==={book})window.__audibleAdapter?.command('seek',{position});"))
            .map_err(|_| "Could not send the saved position to Audible".to_owned())
    })?;
    let started = Instant::now();
    loop {
        let confirmed = super::metadata::with_account_generation(&app, generation, || {
            let state = hub.0.lock().map_err(|_| "Player state unavailable")?;
            if hub.revision() != revision {
                return Err("Playback changed; position sync canceled.".into());
            }
            let s = ready(&state, &asin, now())?;
            let advanced = if s.paused {
                0.0
            } else {
                started.elapsed().as_secs_f64() * s.speed
            };
            Ok(s.position >= position - 1.0 && s.position <= position + advanced + 2.0)
        })?;
        if confirmed {
            return Ok(SyncResult { position });
        }
        if started.elapsed() > Duration::from_secs(5) {
            return Err("Audible did not confirm the saved position. Check the timeline or show the official player before continuing.".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hub() -> super::super::desktop::NowPlaying {
        super::super::desktop::NowPlaying {
            book: Some(serde_json::json!({"asin":"B012345678"})),
            snapshot: Some(serde_json::from_value(serde_json::json!({"asin":"B012345678","position":100,"duration":600,"speed":1,"paused":true,"available":true,"seekable":[{"start":0,"end":600}]})).unwrap()),
            last_seen: Some(10),
        }
    }
    #[test]
    fn requires_the_same_full_book_and_fresh_telemetry() {
        let mut h = hub();
        assert!(ready(&h, "B012345678", 13).is_ok());
        assert!(ready(&h, "B012345678", 14).is_err());
        assert!(ready(&h, "B987654321", 10).is_err());
        h.snapshot.as_mut().unwrap().asin = "B987654321".into();
        assert!(ready(&h, "B012345678", 10).is_err());
        h.snapshot.as_mut().unwrap().asin = "B012345678".into();
        h.snapshot.as_mut().unwrap().sample = true;
        assert!(ready(&h, "B012345678", 10).is_err());
    }
    #[test]
    fn permits_cloud_positions_in_either_direction_but_never_guesses_a_range() {
        let h = hub();
        let s = h.snapshot.as_ref().unwrap();
        for p in [0.0, 50.0, 500.0, 600.0] {
            assert!(valid_target(s, p).is_ok());
        }
        for p in [-1.0, 601.0, f64::NAN, f64::INFINITY] {
            assert!(valid_target(s, p).is_err());
        }
        let mut partial = s.clone();
        partial.seekable[0].end = 200.0;
        assert!(valid_target(&partial, 500.0).is_err());
    }
    #[test]
    fn every_new_intent_invalidates_an_in_flight_sync_even_for_the_same_book() {
        let hub = super::super::desktop::PlayerHub::new();
        let request = hub.changed();
        assert_eq!(request, hub.revision());
        hub.changed();
        assert_ne!(request, hub.revision());
    }
}
