//! Temporary same-origin loopback bridge. Never exposes account tokens or a
//! generic HTTP/API proxy. Browser and WebView use the same streaming core.
use super::*;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};
use tokio::sync::oneshot;

pub struct BrowserTestState(Mutex<Option<Handle>>);
struct Handle {
    shutdown: oneshot::Sender<()>,
    revoked: Arc<AtomicBool>,
    url: String,
}
impl BrowserTestState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}
struct Gate {
    host: String,
    origin: String,
    ticket: Mutex<Option<String>>,
    cookie: String,
    created: Instant,
    revoked: Arc<AtomicBool>,
}
impl Gate {
    fn live(&self) -> bool {
        !self.revoked.load(Ordering::SeqCst) && self.created.elapsed() < Duration::from_secs(1200)
    }
    fn valid_headers(&self, headers: &HeaderMap, is_post: bool) -> bool {
        self.live()
            && headers.get("host").and_then(|h| h.to_str().ok()) == Some(self.host.as_str())
            && (!is_post
                || headers.get("origin").and_then(|h| h.to_str().ok())
                    == Some(self.origin.as_str()))
    }
    fn pair(&self, ticket: &str) -> bool {
        if !self.live() || self.created.elapsed() > Duration::from_secs(120) {
            return false;
        }
        let Ok(mut stored) = self.ticket.lock() else {
            return false;
        };
        if stored
            .as_deref()
            .map(|s| secret_equal(s, ticket))
            .unwrap_or(false)
        {
            *stored = None;
            true
        } else {
            false
        }
    }
    fn authenticated(&self, headers: &HeaderMap) -> bool {
        self.live()
            && headers
                .get("cookie")
                .and_then(|h| h.to_str().ok())
                .map(|h| {
                    h.split(';').any(|part| {
                        part.trim()
                            .strip_prefix("audible_browser_test=")
                            .map(|v| secret_equal(v, &self.cookie))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
    }
}
fn secret_equal(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}
struct Context {
    app: tauri::AppHandle,
    book: Value,
    gate: Gate,
    session: Mutex<Option<String>>,
    start_lock: tokio::sync::Mutex<()>,
}
type Shared = Arc<Context>;
#[derive(Serialize)]
pub struct BrowserLaunch {
    url: String,
    expires_in_seconds: u64,
}
pub(super) fn stop(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<BrowserTestState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(handle) = guard.take() {
                super::stream::cancel_pending(app);
                handle.revoked.store(true, Ordering::SeqCst);
                let _ = handle.shutdown.send(());
            }
        }
    }
}
#[tauri::command]
pub fn browser_test_stop(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    crate::local(&window)?;
    stop(&app);
    Ok(())
}
#[tauri::command]
pub fn browser_test_open(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    crate::local(&window)?;
    let state = app.state::<BrowserTestState>();
    let guard = state
        .0
        .lock()
        .map_err(|_| "Browser comparison unavailable")?;
    let handle = guard
        .as_ref()
        .filter(|h| !h.revoked.load(Ordering::SeqCst))
        .ok_or("Create a browser test link first")?;
    open::that(&handle.url).map_err(|_| {
        "Cannot open the default browser; copy the test link into Edge or Chrome".into()
    })
}
#[tauri::command]
pub async fn browser_test_start(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<BrowserLaunch, String> {
    crate::local(&window)?;
    if !crate::valid_asin(&asin) || asin.starts_with("DEMO") {
        return Err("Select a synced library title first".into());
    }
    let book = app
        .state::<crate::desktop::PlayerHub>()
        .0
        .lock()
        .map_err(|_| "Player state unavailable")?
        .book
        .clone()
        .filter(|b| b["asin"].as_str() == Some(&asin))
        .ok_or("Select this title in the app first")?;
    if !app
        .asset_resolver()
        .get("browser-test.html".into())
        .map(|a| String::from_utf8_lossy(&a.bytes).contains("browser-test-root"))
        .unwrap_or(false)
    {
        return Err(
            "Browser test assets missing; restart Tauri after building the frontend".into(),
        );
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| "Cannot start local browser comparison")?;
    let host = format!(
        "127.0.0.1:{}",
        listener
            .local_addr()
            .map_err(|_| "Local test port unavailable")?
            .port()
    );
    let origin = format!("http://{host}");
    let ticket = URL_SAFE_NO_PAD.encode(random(32));
    let revoked = Arc::new(AtomicBool::new(false));
    let context = Arc::new(Context {
        app: app.clone(),
        book,
        gate: Gate {
            host,
            origin: origin.clone(),
            ticket: Mutex::new(Some(ticket.clone())),
            cookie: URL_SAFE_NO_PAD.encode(random(32)),
            created: Instant::now(),
            revoked: revoked.clone(),
        },
        session: Mutex::new(None),
        start_lock: tokio::sync::Mutex::new(()),
    });
    let router = Router::new()
        .route("/pair", post(pair))
        .route("/api/call", post(call))
        .route("/", get(asset))
        .route("/{*path}", get(asset))
        .layer(DefaultBodyLimit::max(12_000_000))
        .layer(middleware::from_fn_with_state(context.clone(), guard))
        .with_state(context.clone());
    stop(&app);
    let (tx, rx) = oneshot::channel();
    let url = format!("{origin}/#pair={ticket}");
    *app.state::<BrowserTestState>()
        .0
        .lock()
        .map_err(|_| "Browser test state unavailable")? = Some(Handle {
        shutdown: tx,
        revoked,
        url: url.clone(),
    });
    tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                tokio::select! {_=rx=>{},_=tokio::time::sleep(Duration::from_secs(1200))=>{}}
            })
            .await;
        context.gate.revoked.store(true, Ordering::SeqCst);
        if let Some(id) = context.session.lock().ok().and_then(|mut s| s.take()) {
            let _ = super::stream::end(&context.app, &id);
        }
    });
    // Fragment is consumed once by /pair and removed from browser history.
    // It is a temporary local capability, never an Audible token.
    Ok(BrowserLaunch {
        url,
        expires_in_seconds: 1200,
    })
}
async fn guard(State(ctx): State<Shared>, request: Request<Body>, next: Next) -> Response {
    if !ctx.gate.valid_headers(
        request.headers(),
        request.method() == axum::http::Method::POST,
    ) {
        return (StatusCode::FORBIDDEN, "Local comparison request rejected").into_response();
    }
    let mut response = next.run(request).await;
    for (key,value) in [("cache-control","no-store"),("referrer-policy","no-referrer"),("x-content-type-options","nosniff"),("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; media-src 'self' blob:; worker-src 'self' blob:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")]{response.headers_mut().insert(axum::http::HeaderName::from_static(key),axum::http::HeaderValue::from_static(value));}
    response
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    ticket: String,
}
async fn pair(State(ctx): State<Shared>, Json(pair): Json<Pair>) -> Response {
    if !ctx.gate.pair(&pair.ticket) {
        return (
            StatusCode::FORBIDDEN,
            "Pairing link used or expired; start a new comparison from the app",
        )
            .into_response();
    }
    let cookie = format!(
        "audible_browser_test={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=1200",
        ctx.gate.cookie
    );
    let mut response = Json(json!({"ok":true})).into_response();
    response
        .headers_mut()
        .insert("set-cookie", cookie.parse().unwrap());
    response
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Call {
    Book,
    Start {
        asin: String,
    },
    Fetch {
        id: String,
        uri: String,
        range: Option<String>,
    },
    License {
        id: String,
        challenge: Vec<u8>,
    },
    End {
        id: String,
    },
}
fn owns(ctx: &Context, id: &str) -> Result<(), String> {
    if ctx
        .session
        .lock()
        .map_err(|_| "Comparison state unavailable")?
        .as_deref()
        == Some(id)
    {
        Ok(())
    } else {
        Err("This playback session does not belong to the browser comparison".into())
    }
}
async fn call(
    State(ctx): State<Shared>,
    headers: HeaderMap,
    Json(operation): Json<Call>,
) -> Response {
    if !ctx.gate.authenticated(&headers) {
        return (StatusCode::UNAUTHORIZED,Json(json!({"ok":false,"error":"Browser pairing expired; start a new comparison from the app"}))).into_response();
    }
    let result: Result<Value, String> = async {
        match operation {
            Call::Book => Ok(ctx.book.clone()),
            Call::Start { asin } => {
                if ctx.book["asin"].as_str() != Some(&asin) {
                    return Err("Only the selected comparison title is allowed".into());
                }
                let _guard = ctx.start_lock.lock().await;
                let start = super::stream::start(ctx.app.clone(), asin).await?;
                if !ctx.gate.live() {
                    let _ = super::stream::end(&ctx.app, &start.id);
                    return Err("Browser comparison stopped while loading".into());
                }
                *ctx.session
                    .lock()
                    .map_err(|_| "Comparison state unavailable")? = Some(start.id.clone());
                serde_json::to_value(start).map_err(|_| "Cannot prepare comparison session".into())
            }
            Call::Fetch { id, uri, range } => {
                owns(&ctx, &id)?;
                if uri.len() > 16_000 {
                    return Err("Media address exceeds size limit".into());
                }
                serde_json::to_value(super::stream::fetch(ctx.app.clone(), id, uri, range).await?)
                    .map_err(|_| "Cannot relay encrypted media".into())
            }
            Call::License { id, challenge } => {
                owns(&ctx, &id)?;
                Ok(json!(
                    super::stream::license(ctx.app.clone(), id, challenge).await?
                ))
            }
            Call::End { id } => {
                owns(&ctx, &id)?;
                super::stream::end(&ctx.app, &id)?;
                *ctx.session
                    .lock()
                    .map_err(|_| "Comparison state unavailable")? = None;
                Ok(Value::Null)
            }
        }
    }
    .await;
    match result {
        Ok(result) => Json(json!({"ok":true,"result":result})).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok":false,"error":error})),
        )
            .into_response(),
    }
}
fn asset_path(path: &str) -> Option<String> {
    if path == "/" {
        return Some("browser-test.html".into());
    }
    let name = path.strip_prefix("/assets/")?;
    if name.is_empty()
        || name.contains("..")
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return None;
    }
    if !name.ends_with(".js") && !name.ends_with(".css") {
        return None;
    }
    Some(format!("assets/{name}"))
}
async fn asset(State(ctx): State<Shared>, uri: Uri) -> Response {
    let Some(path) = asset_path(uri.path()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match ctx.app.asset_resolver().get(path) {
        Some(asset) => ([("content-type", asset.mime_type)], asset.bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gate() -> Gate {
        Gate {
            host: "127.0.0.1:32123".into(),
            origin: "http://127.0.0.1:32123".into(),
            ticket: Mutex::new(Some("test-pairing-capability".into())),
            cookie: "test-cookie-capability".into(),
            created: Instant::now(),
            revoked: Arc::new(AtomicBool::new(false)),
        }
    }
    #[test]
    fn pairing_is_one_use_and_expires() {
        let mut gate = gate();
        assert!(!gate.pair("wrong"));
        assert!(gate.pair("test-pairing-capability"));
        assert!(!gate.pair("test-pairing-capability"));
        gate.created = Instant::now() - Duration::from_secs(121);
        *gate.ticket.lock().unwrap() = Some("new".into());
        assert!(!gate.pair("new"));
    }
    #[test]
    fn block_foreign_origin_dns_rebinding_and_expired_cookies() {
        let gate = gate();
        let mut h = HeaderMap::new();
        h.insert("host", "127.0.0.1:32123".parse().unwrap());
        assert!(!gate.valid_headers(&h, true));
        h.insert("origin", "http://evil.test".parse().unwrap());
        assert!(!gate.valid_headers(&h, true));
        h.insert("origin", gate.origin.parse().unwrap());
        assert!(gate.valid_headers(&h, true));
        h.insert("host", "evil.test:32123".parse().unwrap());
        assert!(!gate.valid_headers(&h, true));
        h.insert(
            "cookie",
            "audible_browser_test=test-cookie-capability"
                .parse()
                .unwrap(),
        );
        assert!(gate.authenticated(&h));
        gate.revoked.store(true, Ordering::SeqCst);
        assert!(!gate.authenticated(&h));
    }
    #[test]
    fn only_built_assets_and_playback_operations_are_allowed() {
        for path in [
            "/../Cargo.toml",
            "/assets/../../.env",
            "/assets/%2e%2e.js",
            "/assets/a/b.js",
            "/assets/private.json",
        ] {
            assert!(asset_path(path).is_none());
        }
        assert_eq!(
            asset_path("/assets/player-123.js").unwrap(),
            "assets/player-123.js"
        );
        for operation in [
            json!({"operation":"api_login"}),
            json!({"operation":"fetch","id":"x","uri":"x","range":null,"authorization":"secret"}),
        ] {
            assert!(serde_json::from_value::<Call>(operation).is_err());
        }
    }
}
