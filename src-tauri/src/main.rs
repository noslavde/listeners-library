#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod activity;
mod cover;
mod current_book;
mod desktop;
mod diagnostics;
mod history;
mod highlights;
mod launch;
mod lifecycle;
mod listening;
mod media_keys;
mod metadata;
mod notes;
mod privacy;
mod player_sync;
mod protection;
mod shortcuts;
mod start_playback;
mod storage;
mod window_state;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
struct OfficialLaunch(std::sync::Mutex<Option<String>>);

fn settings_db(app: &tauri::AppHandle) -> Result<Connection, String> {
    let c = app.state::<storage::Storage>().open()?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS preferences(key TEXT PRIMARY KEY,value TEXT NOT NULL);").map_err(|_| "Settings database setup failed")?;
    Ok(c)
}

fn db(app: &tauri::AppHandle) -> Result<Connection, String> {
    let namespace = metadata::account_namespace(app)?;
    let c = app.state::<storage::Storage>().open_profile(namespace.as_deref())?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS listening_activity(asin TEXT PRIMARY KEY,listened_seconds REAL NOT NULL,last_listened INTEGER NOT NULL,position REAL NOT NULL,duration REAL NOT NULL,hidden INTEGER NOT NULL DEFAULT 0);").map_err(|_| "Listening history setup failed")?;
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS preferences(key TEXT PRIMARY KEY,value TEXT NOT NULL);",
    )
    .map_err(|_| "Settings database setup failed")?;
    c.execute_batch("CREATE TABLE IF NOT EXISTS notes(asin TEXT NOT NULL, position REAL NOT NULL, text TEXT NOT NULL); CREATE TABLE IF NOT EXISTS books(asin TEXT PRIMARY KEY, metadata TEXT NOT NULL); CREATE TABLE IF NOT EXISTS playback_positions(asin TEXT PRIMARY KEY,position REAL NOT NULL,updated INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS cloud_positions(asin TEXT PRIMARY KEY,position REAL NOT NULL,checked INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS official_launches(asin TEXT PRIMARY KEY,url TEXT NOT NULL);").map_err(|_|"Database setup failed")?;
    history::schema(&c)?;
    Ok(c)
}
fn local(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" || !window.url().is_ok_and(|u| local_origin(&u)) {
        return Err("Local UI required".into());
    }
    Ok(())
}
pub(crate) fn local_origin(url:&url::Url)->bool{
    let origin=matches!((url.scheme(),url.host_str(),url.port()),("tauri",Some("localhost"),None)|("http"|"https",Some("tauri.localhost"),None));
    let dev=cfg!(debug_assertions)&&url.scheme()=="http"&&matches!(url.host_str(),Some("127.0.0.1"|"localhost"))&&url.port()==Some(1420);
    (origin||dev)&&url.username().is_empty()&&url.password().is_none()&&matches!(url.path(),"/"|"/index.html")
}
#[tauri::command]
fn inspect_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    target: String,
) -> Result<(), String> {
    desktop::player_local(&window)?;
    if !matches!(
        target.as_str(),
        "main" | "official" | "mini-player" | "metadata-login"
    ) {
        return Err("Unsupported inspector target".into());
    }
    #[cfg(debug_assertions)]
    {
        let target = app
            .get_webview_window(&target)
            .ok_or("Open that window before inspecting it")?;
        target.open_devtools();
        Ok(())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = app;
        Err("Developer tools are available when running the development build".into())
    }
}
#[tauri::command]
fn import_library(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    raw: String,
) -> Result<Vec<serde_json::Value>, String> {
    cache_library(app, window, raw, false)
}
fn cache_library(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    raw: String,
    replace: bool,
) -> Result<Vec<serde_json::Value>, String> {
    local(&window)?;
    if raw.len() > 20_000_000 {
        return Err("Library file exceeds 20 MB".into());
    }
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|_| "Invalid JSON")?;
    let rows = v
        .as_array()
        .or_else(|| v.get("items").and_then(|x| x.as_array()))
        .or_else(|| v.get("result").and_then(|x| x.as_array()))
        .ok_or("Expected array or items/result array")?;
    let mut c = db(&app)?;
    let tx = c.transaction().map_err(|_| "Cannot begin import")?;
    if replace {
        tx.execute("DELETE FROM books", [])
            .map_err(|_| "Cannot replace cached library")?;
    }
    let mut books = vec![];
    for b in rows {
        let book = normalize_book(b)?;
        let asin = book["asin"].as_str().ok_or("Missing normalized ASIN")?;
        tx.execute(
            "INSERT OR REPLACE INTO books VALUES(?1,?2)",
            params![asin, book.to_string()],
        )
        .map_err(|_| "Cannot cache library")?;
        books.push(book);
    }
    tx.commit().map_err(|_| "Cannot commit import")?;
    Ok(books)
}
fn normalize_book(b: &serde_json::Value) -> Result<serde_json::Value, String> {
    let asin = b
        .get("asin")
        .and_then(|x| x.as_str())
        .ok_or("Missing ASIN")?;
    if !valid_asin(asin) {
        return Err("Invalid ASIN".into());
    }
    let title = b
        .get("title")
        .and_then(|x| x.as_str())
        .ok_or("Missing title")?;
    if title.is_empty() || title.len() > 10000 {
        return Err("Invalid title length".into());
    }
    let authors = b
        .get("authors")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get("name").and_then(|x| x.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let series = b
        .get("series")
        .and_then(|x| x.as_str())
        .map(str::to_owned)
        .or_else(|| {
            b.get("series")
                .and_then(|x| x.as_array())
                .and_then(|x| x.first())
                .and_then(|x| x.get("title"))
                .and_then(|x| x.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let book = serde_json::json!({"asin":asin,"title":title,"author":b.get("author").and_then(|x|x.as_str()).unwrap_or(&authors),"series":series,"volume":b.get("volume").and_then(|x|x.as_f64()).or_else(||b.get("series").and_then(|x|x.as_array()).and_then(|a|a.first()).and_then(|x|x.get("sequence")).and_then(|x|x.as_f64().or_else(||x.as_str().and_then(|s|s.parse::<f64>().ok())))).unwrap_or(0.0),"duration":b.get("duration").and_then(|x|x.as_f64()).unwrap_or_else(||b.get("runtime_length_min").and_then(|x|x.as_f64()).unwrap_or(0.0)*60.0),"contentDeliveryType":b.get("content_delivery_type").or_else(||b.get("contentDeliveryType")).and_then(|x|x.as_str()).filter(|x|matches!(*x,"SinglePartBook"|"MultiPartBook"|"PodcastEpisode")).unwrap_or("SinglePartBook"),"chapters":b.get("chapters").and_then(|x|x.as_array()).map(|a|a.iter().filter_map(|x|x.as_f64()).collect::<Vec<_>>()).unwrap_or_default()});
    let mut book = book;
    if let Some(cover) = cover::from_book(b) {
        book["coverUrl"] = serde_json::json!(cover);
    }
    Ok(book)
}
#[tauri::command]
fn load_library(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<serde_json::Value>, String> {
    local(&window)?;
    if !metadata::connected(&app) {
        return Ok(Vec::new());
    }
    let c = db(&app)?;
    let mut q = c
        .prepare("SELECT metadata FROM books")
        .map_err(|_| "Cannot read library")?;
    let rows = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| "Cannot read library")?;
    rows.map(|r| {
        serde_json::from_str(&r.map_err(|_| "Invalid row")?).map_err(|_| "Invalid metadata".into())
    })
    .collect()
}
fn valid_asin(s: &str) -> bool {
    s.len() == 10
        && s.bytes()
            .all(|x| x.is_ascii_uppercase() || x.is_ascii_digit())
}
fn allowed(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some("audible.com" | "www.audible.com" | "amazon.com" | "www.amazon.com")
        )
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeekRange {
    start: f64,
    end: f64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerState {
    #[serde(default)]
    asin: String,
    position: f64,
    duration: f64,
    speed: f64,
    #[serde(default = "default_volume")]
    volume: f64,
    paused: bool,
    available: bool,
    #[serde(default)]
    sample: bool,
    #[serde(default)]
    website_signed_in: Option<bool>,
    #[serde(default)]
    website_consumable: Option<bool>,
    #[serde(default)]
    seekable: Vec<SeekRange>,
    #[serde(default)]
    buffered: Vec<SeekRange>,
}
impl PlayerState {
    fn valid(&self) -> bool {
        self.position.is_finite()
            && self.duration.is_finite()
            && self.speed.is_finite()
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
            && self.position >= 0.0
            && self.duration >= 0.0
            && self.duration <= 31_536_000.0
            && self.position <= self.duration
            && (0.5..=3.0).contains(&self.speed)
            && self.seekable.len() <= 64
            && self.buffered.len() <= 64
            && self.seekable.iter().chain(self.buffered.iter()).all(|r| {
                r.start.is_finite()
                    && r.end.is_finite()
                    && r.start >= 0.0
                    && r.end >= r.start
                    && r.end <= 31_536_000.0
            })
    }
}
fn default_volume() -> f64 {
    1.0
}
pub(crate) fn account_web_profile(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(storage::data_dir(app)?.join("account-web-session"))
}
fn start_library_launch(app: &tauri::AppHandle) {
    let pending = app
        .state::<OfficialLaunch>()
        .0
        .lock()
        .ok()
        .and_then(|v| v.clone());
    let Some(asin) = pending else {
        return;
    };
    let title = app
        .state::<desktop::PlayerHub>()
        .0
        .lock()
        .ok()
        .and_then(|hub| {
            hub.book
                .as_ref()
                .and_then(|b| b["title"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let allow_search = launch::may_search(app, &asin);
    let args = serde_json::to_string(&(asin, title, allow_search)).unwrap();
    if let Some(w) = app.get_webview_window("official") {
        let _ = w.eval(format!("({})(...{args});", include_str!("launch.js")));
    }
}
fn open(app: &tauri::AppHandle, url: url::Url, expected_generation:Option<u64>) -> Result<(), String> {
    let generation=expected_generation.unwrap_or(metadata::account_generation(app)?);
    metadata::with_account_generation(app,generation,||Ok(()))?;
    let _ = app.emit_to("main", "playback-login-status", "Using the remembered local website session. Audible may require sign-in if it has expired.");
    if let Some(w) = app.get_webview_window("official") {
        w.navigate(url).map_err(|_| "Navigation failed")?;
        w.show().map_err(|_| "Cannot show player")?;
        return Ok(());
    }
    let profile = account_web_profile(app)?;
    let handle = app.clone();
    let page_handle = app.clone();
    let popup_handle = app.clone();
    let instance=launch::next_instance(app);
    // Both account sign-in and official playback share this local browser profile.
    let official=WebviewWindowBuilder::new(app, "official", WebviewUrl::External(url))
        .title("Official Audible · remembered local session")
        .data_directory(profile)
        .incognito(!privacy::remember(app))
        .devtools(cfg!(debug_assertions))
        .on_new_window(move |url, _| {
            // Listen now can use window.open. Keep it in the existing profile
            // rather than losing the action or creating an unauthenticated view.
            if playback_session_current(&popup_handle,generation,instance) && allowed(&url) {
                let app = popup_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if !playback_session_current(&app,generation,instance) { return; }
                    if let Some(w) = app.get_webview_window("official") {
                        let _ = w.navigate(url);
                        let _ = w.show();
                    }
                });
            }
            tauri::webview::NewWindowResponse::Deny
        })
        .inner_size(1000.0, 760.0)
        .initialization_script(include_str!("volume.js"))
        .initialization_script(include_str!("bridge.js"))
        .initialization_script(include_str!("library_ready.js"))
        .on_page_load(move |_, payload| {
            if !playback_session_current(&page_handle,generation,instance){return;}
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && allowed(payload.url()) && payload.url().path().starts_with("/ap/") {
                let _ = page_handle.emit_to("main", "playback-login-status", "Audible/Amazon requested sign-in or verification. This website session will be remembered locally.");
            }
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && allowed(payload.url())
                && matches!(payload.url().host_str(), Some("audible.com" | "www.audible.com"))
                && payload.url().path().starts_with("/library") {
                start_library_launch(&page_handle);
            }
            let host = payload.url().host_str().unwrap_or("unknown");
            let phase = match payload.event() {
                tauri::webview::PageLoadEvent::Started => "Loading",
                tauri::webview::PageLoadEvent::Finished => "Page loaded",
            };
            let _ = page_handle.emit_to(
                "main",
                "official-status",
                format!("{phase}: {host} · authenticate only in the official window"),
            );
        })
        .on_navigation(move |u| {
            if !playback_session_current(&handle,generation,instance){return false;}
            if !metadata::connected(&handle) { return false; }
            if u.scheme() == "audible-launch" {
                let from_library = handle.get_webview_window("official").and_then(|w| w.url().ok())
                    .map(|url| allowed(&url) && matches!(url.host_str(), Some("audible.com" | "www.audible.com")) && url.path().starts_with("/library")).unwrap_or(false);
                if from_library {
                    if u.query_pairs().any(|(k,v)|k=="code"&&v=="ready") {
                        let app=handle.clone();
                        tauri::async_runtime::spawn(async move {start_library_launch(&app);});
                        return false;
                    }
                    let asin = u.query_pairs().find(|(k,_)|k=="asin").map(|(_,v)|v.into_owned());
                    let code = u.query_pairs().find(|(k,_)|k=="code").map(|(_,v)|v.into_owned());
                    if let Ok(mut pending) = handle.state::<OfficialLaunch>().0.lock() {
                        if pending.is_some() && *pending == asin {
                            if code.as_deref()==Some("searching") {
                                if let Some(ref asin)=asin{launch::searched(&handle,asin);}
                                let _=handle.emit_to("main","official-status","Searching your Audible library for the selected title…");
                                return false;
                            }
                            if code.as_deref()==Some("opening") {
                                let _=handle.emit_to("main","official-status","Clicking Listen now for the highlighted title; waiting for Audible to open playback…");
                                return false;
                            }
                            *pending = None;
                            let message = match code.as_deref() {
                                Some("clicked") => "Starting the selected title with Audible's Listen now control; press Play there if requested",
                                Some("highlighted") => "Selected title scrolled into view and highlighted in the official library; use its Listen now control",
                                Some("failed") => "Automatic launch encountered a page-control error; the selected title is highlighted for manual Listen now",
                                Some("ambiguous") => "Selected title highlighted in the official library; more than one Listen now control matched, so choose it there",
                                _ => "Find the selected title in the official library and choose Listen now; automatic matching did not find a unique control on this page",
                            };
                            let _ = handle.emit_to("main", "official-status", message);
                        }
                    }
                }
                return false;
            }
            if u.scheme()=="audible-control"{if u.query_pairs().any(|(k,v)|k=="code"&&v=="play_blocked"){if let Some(w)=handle.get_webview_window("official"){let _=w.show();let _=w.set_focus();}}let code=u.query_pairs().find(|(k,_)|k=="code").map(|(_,v)|v.into_owned());let message=match code.as_deref(){Some("unsupported_seek")=>"The official media element cannot seek to that position yet; use its chapter controls or wait for content to load",Some("chapter_unavailable")=>"The official chapter control could not be matched. Open Official player and its chapter list, then try again; no out-of-range seek was sent",Some("chapter_ambiguous")=>"More than one official chapter matches that title; select it directly in the official player",Some("play_blocked")=>"Playback was refused by the official media element; start playback in the official window",_=>"Official playback command could not be completed"};let _=handle.emit_to("main","official-status",message);let _=handle.emit_to("mini-player","player-error",message);return false;}
            if u.scheme() == "audible-library" {
                let origin_ok = handle
                    .get_webview_window("official")
                    .and_then(|w| w.url().ok())
                    .map(|url| {
                        allowed(&url)
                            && matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
                            && url.path().starts_with("/library")
                    })
                    .unwrap_or(false);
                if origin_ok {
                    if let Some((_, payload)) = u.query_pairs().find(|(k, _)| k == "payload") {
                        if payload.len() <= 150_000 {
                            if let Ok(rows) = serde_json::from_str::<Vec<VisibleBook>>(&payload) {
                                if rows.len() <= 100
                                    && rows.iter().all(|b| {
                                        valid_asin(&b.asin)
                                            && !b.title.is_empty()
                                            && b.title.len() <= 4000
                                            && b.author.is_empty()
                                            && b.series.is_empty()
                                            && b.volume == 0.0
                                            && b.duration == 0.0
                                            && b.chapters.is_empty()
                                    })
                                {
                                    let _ = handle.emit_to("main", "library-visible", rows);
                                }
                            }
                        }
                    }
                }
                return false;
            }
            if u.scheme() == "audible-state" {
                if let Some((_, payload)) = u.query_pairs().find(|(k, _)| k == "payload") {
                    if payload.len() < 4096 {
                        if let Ok(v) = serde_json::from_str::<PlayerState>(&payload) {
                            if v.valid() {
                                let matching_title = handle.state::<desktop::PlayerHub>().0.lock()
                                    .map(|hub| hub.book.as_ref().and_then(|b| b["asin"].as_str()) == Some(v.asin.as_str()))
                                    .unwrap_or(false);
                                if !matching_title {
                                    return false;
                                }
                                if v.sample {
                                    let flag = |value: Option<bool>| match value { Some(true) => "yes", Some(false) => "no", None => "not reported" };
                                    let message = format!("Audible is serving a sample · website signed in: {} · full access recognized: {}. Open Audible sign-in / library and use Listen now for this title.", flag(v.website_signed_in), flag(v.website_consumable));
                                    let _ = handle.emit_to("main", "official-status", &message);
                                    let _ = handle.emit_to("mini-player", "player-error", &message);
                                }
                                launch::observe(&handle, &v);
                                desktop::record(&handle, &v);
                                let held=history::guard_start(&handle,&v);
                                if let Some(w)=handle.get_webview_window("official") { let _=w.eval(format!("window.__audibleAdapter?.hold({held});")); }
                                if !held {
                                    activity::observe(&handle, &v);
                                    history::settled(&handle,&v);
                                }
                                listening::observe(&handle, &v);
                                match handle.state::<start_playback::StartPlayback>().update(&v.asin,v.available,v.sample,v.paused,held) {
                                    Some(start_playback::Action::Play)=>{
                                        if let Some(w)=handle.get_webview_window("official") {
                                            let _=w.eval("window.__audibleAdapter?.command('play',0);");
                                        }
                                    },
                                    Some(start_playback::Action::Mini)=>{
                                        let app=handle.clone();let asin=v.asin.clone();
                                        tauri::async_runtime::spawn(async move {
                                            let current=app.state::<desktop::PlayerHub>().0.lock().ok().map(|hub|hub.snapshot.as_ref().map(|s|s.asin==asin&&s.available&&!s.sample&&!s.paused).unwrap_or(false)).unwrap_or(false);
                                            if current {
                                                if let Err(error)=desktop::enter_mini_player(app.clone()).await {let _=app.emit_to("main","official-status",error);}
                                                else {let _=app.emit_to("main","official-status","Playing in the mini player");}
                                            }
                                        });
                                    },
                                    Some(start_playback::Action::Review)=>{
                                        let app=handle.clone();
                                        tauri::async_runtime::spawn(async move { if let Err(e)=desktop::enter_mini_player(app.clone()).await { let _=app.emit_to("main","official-status",e); } });
                                    },
                                    None=>{}
                                }
                                let _ = handle.emit_to("main", "player-state", v.clone());
                                let _ = handle.emit_to("mini-player", "player-state", v);
                            }
                        }
                    }
                }
                return false;
            }
            let permitted = allowed(u);
            if !permitted {
                let _ = handle.emit_to(
                    "main",
                    "official-status",
                    format!(
                        "Navigation blocked: {} · only approved official login hosts are allowed",
                        u.host_str().unwrap_or("unsupported destination")
                    ),
                );
            }
            permitted
        })
        .build()
        .map_err(|_| "Cannot open official player")?;
    if !playback_session_current(app,generation,instance){let _=official.destroy();return Err("Account changed; playback opening canceled.".into());}
    privacy::harden_view(&official)?;

    Ok(())
}
fn playback_session_current(app:&tauri::AppHandle,generation:u64,instance:u64)->bool{
    launch::active_instance(app,instance)&&metadata::account_generation(app).is_ok_and(|g|g==generation)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisibleBook {
    asin: String,
    title: String,
    author: String,
    series: String,
    volume: f64,
    duration: f64,
    chapters: Vec<f64>,
}
#[tauri::command]
async fn sync_visible_library(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    local(&window)?;
    let w = app
        .get_webview_window("official")
        .ok_or("Sign in first and open the official library")?;
    let url = w.url().map_err(|_| "Cannot inspect official page")?;
    if !allowed(&url)
        || !matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
        || !url.path().starts_with("/library")
    {
        return Err("Open your library in the official window before syncing".into());
    }
    w.eval(include_str!("library.js"))
        .map_err(|_| "Cannot read visible library".into())
}
#[tauri::command]
async fn open_login(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    local(&window)?;
    app.state::<start_playback::StartPlayback>().cancel();
    *app.state::<OfficialLaunch>()
        .0
        .lock()
        .map_err(|_| "Playback launch unavailable")? = None;
    metadata::stream::invalidate(&app);
    open(
        &app,
        url::Url::parse("https://www.audible.com/library/titles").unwrap(),
        None,
    )
}
#[tauri::command]
async fn open_player(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<(), String> {
    local(&window)?;
    metadata::require_account(&app)?;
    if !valid_asin(&asin) || asin.starts_with("DEMO") {
        return Err("Choose an owned title from your library first".into());
    }
    let generation=metadata::account_generation(&app)?;
    let launcher=app.state::<launch::LaunchHistory>();
    let _opening=launcher.opening.lock().await;
    metadata::with_account_generation(&app,generation,||Ok(()))?;
    activity::set_hidden(&app, &asin, false)?;
    let already_ready = app
        .state::<desktop::PlayerHub>()
        .0
        .lock()
        .ok()
        .map(|hub| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            hub.snapshot
                .as_ref()
                .map(|s| s.asin == asin && s.available && !s.sample)
                .unwrap_or(false)
                && now.saturating_sub(hub.last_seen.unwrap_or(0)) < 3
        })
        .unwrap_or(false);
    if already_ready {
        if let Some(w) = app.get_webview_window("official") {
            if w.url()
                .ok()
                .and_then(|u| launch::clean_url(&u, &asin))
                .is_some()
            {
                if !history::blocked(&app){app.state::<start_playback::StartPlayback>().arm(&asin);}
                return desktop::enter_mini_player(app.clone()).await;
            }
        }
    }
    // A previous document can still produce popup/navigation callbacks after a
    // navigation request. Retire that instance before starting a different book.
    if let Some(w)=app.get_webview_window("official"){
        let _=w.eval("window.__audibleLaunchCancel?.();window.__audibleAdapter?.hold(true);document.querySelectorAll('audio,video').forEach(m=>m.pause());");
        history::flush(&app,"switch")?;
        activity::finish(&app);
        metadata::with_account_generation(&app,generation,||{
            launch::next_instance(&app);
            w.destroy().map_err(|_|"Cannot close the previous playback window".into())
        })?;
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(3);
        while app.get_webview_window("official").is_some(){
            if std::time::Instant::now()>=deadline{return Err("The previous player is still closing. Wait a moment and try again.".into());}
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
    }
    metadata::with_account_generation(&app,generation,||Ok(()))?;
    history::prepare(&app,&asin,generation)?;
    activity::finish(&app);
    app.state::<start_playback::StartPlayback>().arm(&asin);
    metadata::stream::invalidate(&app);
    desktop::selected_in_session(&app, &asin, generation)?;
    let target=metadata::with_account_generation(&app,generation,||{
    *app.state::<OfficialLaunch>()
        .0
        .lock()
        .map_err(|_| "Playback launch unavailable")? = Some(asin.clone());
    let remembered = launch::remembered(&app, &asin);
    launch::selecting(&app, &asin, remembered.is_some());
    Ok(remembered.unwrap_or_else(|| url::Url::parse("https://www.audible.com/library/titles").unwrap()))
    })?;
    open(&app,target,Some(generation))
}
#[tauri::command]
fn player_command(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    action: String,
    value: f64,
) -> Result<(), String> {
    desktop::player_local(&window)?;
    send_player_command(app, action, value)
}
pub(crate) fn send_player_command(
    app: tauri::AppHandle,
    action: String,
    value: f64,
) -> Result<(), String> {
    metadata::require_account(&app)?;
    if !value.is_finite() {
        return Err("Invalid value".into());
    }
    if history::blocked(&app) && matches!(action.as_str(),"play"|"seek"|"recover") {
        return Err("Choose a saved position in Listening history before continuing.".into());
    }
    if ![
        "play",
        "pause",
        "seek",
        "speed",
        "volume",
        "pauseChapter",
        "rewind",
        "recover",
    ]
    .contains(&action.as_str())
    {
        return Err("Unknown action".into());
    }
    if action == "speed" && !(0.5..=3.0).contains(&value) {
        return Err("Invalid speed".into());
    }
    if action == "volume" && !(0.0..=1.0).contains(&value) {
        return Err("Invalid app volume".into());
    }
    let w = app
        .get_webview_window("official")
        .ok_or("Open official player first")?;
    let current = w.url().map_err(|_| "No player URL")?;
    if !allowed(&current)
        || !matches!(current.host_str(), Some("audible.com" | "www.audible.com"))
        || current.path() != "/webplayer"
    {
        return Err("Unsupported origin".into());
    }
    if action == "seek" {
        let state = app.state::<desktop::PlayerHub>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let hub = state.0.lock().map_err(|_| "Player state unavailable")?;
        let s = hub
            .snapshot
            .as_ref()
            .ok_or("Wait for media state before seeking")?;
        if now.saturating_sub(hub.last_seen.unwrap_or(0)) > 3
            || !s
                .seekable
                .iter()
                .any(|r| value >= r.start && value <= r.end)
        {
            return Err("Requested position is outside the official media element's current seekable range; no seek was sent".into());
        }
    }
    let a = serde_json::to_string(&action).unwrap();
    match action.as_str() {
        "volume" => listening::save(
            &app,
            listening::Patch {
                volume: Some(value),
                ..Default::default()
            },
        )?,
        "speed" => listening::save(
            &app,
            listening::Patch {
                speed: Some(value),
                ..Default::default()
            },
        )?,
        "rewind" => {
            if !(0.0..=30.0).contains(&value) || value.fract() != 0.0 {
                return Err("Invalid rewind amount".into());
            }
            listening::save(
                &app,
                listening::Patch {
                    rewind_seconds: Some(value as u32),
                    ..Default::default()
                },
            )?;
        }
        "pauseChapter" => listening::save(
            &app,
            listening::Patch {
                pause_after_chapter: Some(value != 0.0),
                ..Default::default()
            },
        )?,
        _ => (),
    }
    let hub = app.state::<desktop::PlayerHub>();
    let _state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    hub.changed();
    w.eval(format!("window.__audibleAdapter?.command({a},{value});"))
        .map_err(|_| "Player command failed".into())
}
#[tauri::command]
fn select_chapter(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    position: f64,
    title: String,
) -> Result<(), String> {
    desktop::player_local(&window)?;
    metadata::require_account(&app)?;
    if history::blocked(&app){return Err("Choose a saved position before selecting a chapter.".into());}
    if !position.is_finite()
        || !(0.0..31_536_000.0).contains(&position)
        || title.is_empty()
        || title.len() > 4000
    {
        return Err("Invalid chapter selection".into());
    }
    let w = app
        .get_webview_window("official")
        .ok_or("Open official playback first")?;
    let url = w.url().map_err(|_| "No official page")?;
    if !allowed(&url)
        || !matches!(url.host_str(), Some("audible.com" | "www.audible.com"))
        || url.path() != "/webplayer"
    {
        return Err("Open the title in the official player before choosing a chapter".into());
    }
    let args = serde_json::to_string(&serde_json::json!({"position":position,"title":title}))
        .map_err(|_| "Cannot encode chapter")?;
    let hub = app.state::<desktop::PlayerHub>();
    let _state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    hub.changed();
    w.eval(format!("window.__audibleAdapter?.chapter({args});"))
        .map_err(|_| "Cannot send chapter selection".into())
}
#[tauri::command]
async fn end_session(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    local(&window)?;
    clear_official_session(&app)
}
pub(crate) fn clear_official_session(app: &tauri::AppHandle) -> Result<(), String> {
    launch::next_instance(app);
    listening::cancel_sleep(app);
    app.state::<start_playback::StartPlayback>().cancel();
    *app.state::<OfficialLaunch>()
        .0
        .lock()
        .map_err(|_| "Playback launch unavailable")? = None;
    let profile = account_web_profile(app)?;
    if let Some(w) = app.get_webview_window("official") {
        w.destroy().map_err(|_| "Cannot stop official website before sign-out")?;
    }
    let w = {
        if !profile.exists() {
            return Ok(());
        }
        // Logout must clear persisted cookies even when playback is closed.
        // This function is called only by async commands: cookie reads must not
        // run inside a synchronous UI event handler on Windows.
        WebviewWindowBuilder::new(
            app,
            "account-session-cleanup",
            WebviewUrl::External(url::Url::parse("about:blank").unwrap()),
        )
        .data_directory(profile)
        .incognito(!privacy::remember(app))
        .visible(false)
        .devtools(false)
        .on_navigation(|_| false)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()
        .map_err(|_| "Cannot open local session for sign-out")?
    };
    let outcome = (|| -> Result<(), String> {
        privacy::clear_browser_data(&w)?;
        for cookie in w
            .cookies()
            .map_err(|_| "Cannot read local session for sign-out")?
        {
            w.delete_cookie(cookie)
                .map_err(|_| "Cannot remove website sign-in cookie")?;
        }
        if !w
            .cookies()
            .map_err(|_| "Cannot verify website sign-out")?
            .is_empty()
        {
            return Err("Website sign-out could not be verified; close the app and retry".into());
        }
        Ok(())
    })();
    let closed = w
        .destroy()
        .map_err(|_| "Could not close official session".to_owned());
    outcome?;
    closed?;
    Ok(())
}
fn main() {
    diagnostics::install_panic_hook();
    let instance = match lifecycle::Instance::acquire() {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(_) => { startup_error(); return; }
    };
    if let Err(error) = tauri::Builder::default()
        .plugin(window_state::plugin())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let root = app.path().app_local_data_dir()?;
            privacy::apply_pending_reset(&root)?;
            diagnostics::initialize(&root)?;
            diagnostics::record(diagnostics::Code::OpeningProtectedStorage);
            app.manage(storage::Storage::initialize(
                &root,
                !cfg!(debug_assertions)
                    && !root
                        .join(format!("{}.no-legacy-import", storage::SCOPE))
                        .exists(),
            )?);
            diagnostics::record(diagnostics::Code::RestoringAccount);
            app.manage(metadata::MetadataState::new(app.handle()));
            diagnostics::record(diagnostics::Code::StartingServices);
            privacy::setup(app)?;
            window_state::setup(app)?;
            listening::setup(app)?;
            media_keys::setup(app);
            shortcuts::setup(app);
            instance.listen(app.handle().clone())?;
            lifecycle::restore(app.handle());
            diagnostics::record(diagnostics::Code::StartupReady);
            Ok(())
            })();
            if result.is_err() {
                diagnostics::record(diagnostics::Code::StartupFailed);
                startup_error();
            }
            result
        })
        .on_window_event(window_state::changed)
        .manage(metadata::stream::StreamState::new())
        .manage(metadata::browser_test::BrowserTestState::new())
        .manage(desktop::PlayerHub::new())
        .manage(OfficialLaunch(std::sync::Mutex::new(None)))
        .manage(launch::LaunchHistory::new())
        .manage(start_playback::StartPlayback::new())
        .manage(listening::Listening::new())
        .manage(activity::Activity::new())
        .manage(history::History::default())
        .manage(shortcuts::Shortcuts::new())
        .invoke_handler(tauri::generate_handler![
            highlights::save_highlight,
            highlights::delete_highlight,
            highlights::play_saved_clip,
            highlights::show_highlights,
            highlights::seek_highlight_position,
            diagnostics::diagnostic_report,
            diagnostics::record_ui_error,
            diagnostics::export_diagnostics,
            privacy::privacy_status,
            privacy::set_website_privacy,
            privacy::erase_local_data,
            current_book::get_current_book,
            current_book::set_current_book,
            activity::library_activity,
            activity::dismiss_continue,
            shortcuts::player_shortcut,
            shortcuts::shortcut_status,
            listening::listening_status,
            listening::update_listening_preferences,
            listening::set_sleep_timer,
            #[cfg(debug_assertions)]
            inspect_window,
            metadata::api_login,
            metadata::api_status,
            metadata::api_connection,
            #[cfg(debug_assertions)]
            metadata::api_streaming_probe,
            #[cfg(debug_assertions)]
            metadata::stream::api_stream_start,
            #[cfg(debug_assertions)]
            metadata::stream::api_stream_fetch,
            #[cfg(debug_assertions)]
            metadata::stream::api_stream_license,
            #[cfg(debug_assertions)]
            metadata::stream::api_stream_state,
            #[cfg(debug_assertions)]
            metadata::stream::api_player_command,
            #[cfg(debug_assertions)]
            metadata::stream::api_stream_end,
            #[cfg(debug_assertions)]
            metadata::browser_test::browser_test_start,
            #[cfg(debug_assertions)]
            metadata::browser_test::browser_test_open,
            #[cfg(debug_assertions)]
            metadata::browser_test::browser_test_stop,
            #[cfg(debug_assertions)]
            desktop::select_api_title,
            metadata::api_sync,
            metadata::api_position,
            metadata::api_refresh_card_positions,
            desktop::list_card_positions,
            #[cfg(debug_assertions)]
            metadata::upload::api_prepare_upload,
            #[cfg(debug_assertions)]
            metadata::upload::api_commit_upload,
            #[cfg(debug_assertions)]
            metadata::upload::api_cancel_upload,
            metadata::api_disconnect,
            desktop::open_mini_player,
            desktop::resize_mini_player,
            desktop::drag_mini_player,
            desktop::close_mini_player,
            desktop::current_player,
            player_sync::resync_player,
            history::listening_history,
            history::accept_audible_position,
            history::restore_listening_position,
            cover::open_cover,
            cover::current_cover,
            cover::close_cover,
            desktop::official_visibility,
            desktop::show_library,
            desktop::local_position,
            open_login,
            #[cfg(debug_assertions)]
            sync_visible_library,
            end_session,
            open_player,
            player_command,
            select_chapter,
            notes::notes_session,
            notes::save_note,
            notes::list_notes,
            notes::edit_note,
            notes::delete_note,
            notes::export_notes,
            #[cfg(debug_assertions)]
            import_library,
            load_library
        ])
        .build(tauri::generate_context!())
        .map(|app| app.run(|app, event| match event {
            tauri::RunEvent::Ready => lifecycle::restore(app),
            tauri::RunEvent::ExitRequested { .. } => lifecycle::prepare_exit(app),
            tauri::RunEvent::Exit => diagnostics::record(diagnostics::Code::CleanExit),
            _ => {}
        }))
    {
        let _ = error;
        diagnostics::record(diagnostics::Code::StartupFailed);
        startup_error();
    }
}
fn startup_error() {
        static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if SHOWN.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let text:Vec<u16>="Listener’s Library could not start. Close older copies in Task Manager and retry. Your saved data has not been reset; do not delete the database key. A diagnostic event may be available under Troubleshooting after a successful restart.".encode_utf16().chain(Some(0)).collect();
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                w!("Listener’s Library — protected storage"),
                MB_OK | MB_ICONERROR,
            );
        }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restrict_navigation() {
        assert!(allowed(
            &url::Url::parse("https://www.audible.com/webplayer").unwrap()
        ));
        assert!(!allowed(
            &url::Url::parse("https://audible.com.evil.test/").unwrap()
        ));
        assert!(!allowed(&url::Url::parse("http://audible.com/").unwrap()));
    }
    #[test]
    fn restrict_asin() {
        assert!(valid_asin("B012345678"));
        assert!(!valid_asin("../evil"));
    }
    #[test]
    fn local_commands_reject_provider_pages_and_lookalike_origins(){
        for raw in ["https://www.audible.com/","https://tauri.localhost.evil.test/","https://user@tauri.localhost/","https://tauri.localhost:8443/","file:///index.html"]{
            assert!(!local_origin(&url::Url::parse(raw).unwrap()));
        }
        assert!(local_origin(&url::Url::parse("http://tauri.localhost/index.html?mini=1").unwrap()));
        assert!(local_origin(&url::Url::parse("tauri://localhost/").unwrap()));
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;
    #[test]
    fn reject_unapproved_login_origins() {
        for u in [
            "https://evil.audible.com/",
            "https://www.audible.com:8443/",
            "https://user@www.amazon.com/",
            "https://amazon.com.evil.test/",
        ] {
            assert!(!allowed(&url::Url::parse(u).unwrap()));
        }
        assert!(allowed(
            &url::Url::parse("https://www.amazon.com/ap/signin").unwrap()
        ));
    }
    #[test]
    fn accept_only_playback_telemetry() {
        assert!(serde_json::from_str::<PlayerState>(r#"{"position":0,"duration":1,"speed":1,"paused":true,"available":true,"cookie":"secret"}"#).is_err());
        assert!(!PlayerState {
            asin: String::new(),
            sample: false,
            website_signed_in: None,
            website_consumable: None,
            seekable: Vec::new(),
            buffered: Vec::new(),
            position: 100.0,
            duration: 10.0,
            speed: 1.0,
            volume: 1.0,
            paused: true,
            available: true
        }
        .valid());
    }
}

#[cfg(test)]
mod library_tests {
    use super::*;
    #[test]
    fn normalize_actual_api_shape() {
        let b=normalize_book(&serde_json::json!({"asin":"B012345678","title":"Synthetic","authors":[{"name":"A"},{"name":"B"}],"series":[{"title":"Test series","sequence":"2.5"}],"runtime_length_min":120,"access_token":"never-store-this"})).unwrap();
        assert_eq!(b["author"], "A, B");
        assert_eq!(b["series"], "Test series");
        assert_eq!(b["volume"], 2.5);
        assert_eq!(b["duration"], 7200.0);
        assert!(b.get("access_token").is_none());
    }
    #[test]
    fn reject_invalid_metadata_before_cache() {
        assert!(normalize_book(&serde_json::json!({"asin":"../bad","title":"No"})).is_err());
        assert!(normalize_book(&serde_json::json!({"asin":"B012345678","title":""})).is_err());
    }
}
