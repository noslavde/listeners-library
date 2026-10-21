//! Account-scoped official-player launch history and callback-instance
//! validation. Remembers only cleaned full-player URLs for a selected book;
//! stale windows must not launch or update a later selection.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Manager};
pub struct LaunchHistory {
    instance: AtomicU64,
    pub(crate) opening: tokio::sync::Mutex<()>,
    urls: Mutex<HashMap<String, String>>,
    reused: Mutex<Option<String>>,
    searches: Mutex<std::collections::HashSet<String>>,
}
impl LaunchHistory {
    pub fn new() -> Self {
        Self {
            instance: AtomicU64::new(0),
            opening: tokio::sync::Mutex::new(()),
            urls: Mutex::new(HashMap::new()),
            reused: Mutex::new(None),
            searches: Mutex::new(std::collections::HashSet::new()),
        }
    }
}
/// Allocate a callback lifetime so events from a replaced official window
/// cannot complete the newer book launch.
pub(crate) fn next_instance(app: &tauri::AppHandle) -> u64 {
    app.state::<LaunchHistory>()
        .instance
        .fetch_add(1, Ordering::SeqCst)
        + 1
}
pub(crate) fn active_instance(app: &tauri::AppHandle, id: u64) -> bool {
    app.state::<LaunchHistory>().instance.load(Ordering::SeqCst) == id
}
/// Accept only a cleaned official full-player URL for the requested ASIN; never
/// retain arbitrary signed-in navigation URLs.
pub fn clean_url(url: &url::Url, asin: &str) -> Option<url::Url> {
    if !super::allowed(url)
        || !matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
        || url.path() != "/webplayer"
        || !super::valid_asin(asin)
    {
        return None;
    }
    let values = url.query_pairs().collect::<HashMap<_, _>>();
    if values.get("asin").map(|s| s.as_ref()) != Some(asin)
        || values.get("isSample").map(|s| s.as_ref()) == Some("true")
    {
        return None;
    }
    let mut clean = url::Url::parse("https://www.audible.com/webplayer").ok()?;
    clean.query_pairs_mut().append_pair("asin", asin);
    for (key, allowed) in [
        (
            "contentDeliveryType",
            &["SinglePartBook", "MultiPartBook", "PodcastEpisode"][..],
        ),
        ("initialCPLaunch", &["true", "false"][..]),
    ] {
        if let Some(value) = values.get(key) {
            if allowed.contains(&value.as_ref()) {
                clean.query_pairs_mut().append_pair(key, value);
            }
        }
    }
    // Always let Audible choose its cloud position, never cache a timestamp.
    clean
        .query_pairs_mut()
        .append_pair("isSample", "false")
        .append_pair("overrideLph", "false");
    Some(clean)
}
pub fn remembered(app: &tauri::AppHandle, asin: &str) -> Option<url::Url> {
    let raw = super::db(app)
        .ok()?
        .query_row(
            "SELECT url FROM official_launches WHERE asin=?1",
            [asin],
            |r| r.get::<_, String>(0),
        )
        .ok()?;
    clean_url(&url::Url::parse(&raw).ok()?, asin)
}
pub fn selecting(app: &tauri::AppHandle, asin: &str, reused: bool) {
    if let Ok(mut searches) = app.state::<LaunchHistory>().searches.lock() {
        searches.remove(asin);
    }
    if let Ok(mut pending) = app.state::<LaunchHistory>().reused.lock() {
        *pending = reused.then(|| asin.to_owned());
    }
}
pub fn may_search(app: &tauri::AppHandle, asin: &str) -> bool {
    app.state::<LaunchHistory>()
        .searches
        .lock()
        .map(|s| !s.contains(asin))
        .unwrap_or(false)
}
pub fn searched(app: &tauri::AppHandle, asin: &str) {
    if let Ok(mut searches) = app.state::<LaunchHistory>().searches.lock() {
        searches.insert(asin.to_owned());
    }
}
/// Remember a valid full-player launch after matching current account/book
/// state; a sample is not a reusable owned-title route.
pub fn observe(app: &tauri::AppHandle, state: &super::PlayerState) {
    let Ok(generation) = super::metadata::account_generation(app) else {
        return;
    };
    let key = format!("{generation}:{}", state.asin);
    let history = app.state::<LaunchHistory>();
    if state.sample {
        let fallback = history.reused.lock().ok().and_then(|mut p| {
            if p.as_deref() == Some(&state.asin) {
                p.take()
            } else {
                None
            }
        });
        if let Some(asin) = fallback {
            let instance = history.instance.load(Ordering::SeqCst);
            if let Ok(mut urls) = history.urls.lock() {
                urls.remove(&key);
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if super::metadata::with_account_db(&app, generation, |db| {
                    db.execute("DELETE FROM official_launches WHERE asin=?1", [&asin])
                        .map(|_| ())
                        .map_err(|_| "Cannot reset saved player launch".into())
                })
                .is_err()
                {
                    return;
                }
                if let Some(w) = app.get_webview_window("official") {
                    let matching = app
                        .state::<super::desktop::PlayerHub>()
                        .0
                        .lock()
                        .map(|hub| {
                            hub.book.as_ref().and_then(|b| b["asin"].as_str())
                                == Some(asin.as_str())
                        })
                        .unwrap_or(false);
                    if !matching
                        || !active_instance(&app, instance)
                        || !super::metadata::account_generation(&app).is_ok_and(|g| g == generation)
                    {
                        return;
                    }
                    let _ = w.navigate(
                        url::Url::parse("https://www.audible.com/library/titles").unwrap(),
                    );
                    let _ = w.show();
                }
                let _ = app.emit_to(
                    "main",
                    "official-status",
                    "Saved player launch opened a sample; retrying through the official library",
                );
            });
        }
        return;
    }
    if !state.available {
        return;
    }
    let Some(url) = app
        .get_webview_window("official")
        .and_then(|w| w.url().ok())
        .and_then(|u| clean_url(&u, &state.asin))
    else {
        return;
    };
    if let Ok(mut reused) = history.reused.lock() {
        *reused = None;
    }
    if let Ok(mut pending) = app.state::<super::OfficialLaunch>().0.lock() {
        if pending.as_deref() == Some(&state.asin) {
            *pending = None;
        }
    }
    let Ok(mut urls) = history.urls.lock() else {
        return;
    };
    if urls.get(&key) == Some(&url.to_string()) {
        return;
    }
    urls.insert(key, url.to_string());
    let app = app.clone();
    let asin = state.asin.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if super::metadata::with_account_db(&app, generation, |db| {
            db.execute(
                "INSERT OR REPLACE INTO official_launches(asin,url) VALUES(?1,?2)",
                rusqlite::params![asin, url.as_str()],
            )
            .map_err(|_| "Cannot remember playback launch".into())
        })
        .is_ok()
        {
            let _ = app.emit_to(
                "main",
                "official-status",
                "Title launch remembered locally; Listen can now reopen this book directly",
            );
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_keeps_only_safe_title_context_and_cloud_resume() {
        let raw=url::Url::parse("https://www.audible.com/webplayer?asin=B012345678&contentDeliveryType=MultiPartBook&overrideLph=true&position=123&token=private&sessionId=secret#private").unwrap();
        let clean = clean_url(&raw, "B012345678").unwrap();
        assert!(clean.as_str().contains("contentDeliveryType=MultiPartBook"));
        assert!(clean.as_str().contains("overrideLph=false"));
        assert!(!clean.as_str().contains("private"));
        assert!(!clean.as_str().contains("secret"));
        assert!(!clean.as_str().contains("position"));
    }
    #[test]
    fn rejects_samples_other_titles_and_unapproved_origins() {
        for raw in [
            "https://www.audible.com/webplayer?asin=B012345678&isSample=true",
            "https://www.audible.com/webplayer?asin=B987654321",
            "https://evil.test/webplayer?asin=B012345678",
            "https://www.audible.com/library?asin=B012345678",
        ] {
            assert!(clean_url(&url::Url::parse(raw).unwrap(), "B012345678").is_none());
        }
    }
}
