//! US account/metadata API and explicit streaming-grant feasibility probe.
//! No audio download, key extraction, imported CDM or DRM decryption.
#[cfg(test)]
mod account_restore;
pub mod browser_test;
mod profile;
pub mod stream;
pub mod upload;
mod website;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use profile::Profile;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use upload::PreparedUpload;
pub struct MetadataState {
    durability: Mutex<()>,
    namespace: Mutex<Option<String>>,
    connected_generation: AtomicU64,
    pending: Mutex<Option<Pending>>,
    generation: AtomicU64,
    auth: tokio::sync::Mutex<Option<Auth>>,
    upload: Mutex<Option<PreparedUpload>>,
    login: Mutex<LoginStatus>,
    diagnostics: Mutex<Vec<String>>,
}
struct Pending {
    profile: Profile,
    verifier: String,
    serial: String,
    created: u64,
    generation: u64,
}
#[derive(Serialize, Deserialize)]
struct Auth {
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    profile: Profile,
    access: String,
    refresh: String,
    expires: u64,
}
impl Drop for Auth {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access.zeroize();
        self.refresh.zeroize();
    }
}
impl Auth {
    fn namespace(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"listeners-library:us:v1:");
        if let Some(id) = self.account_id.as_deref() {
            hash.update(b"account:"); hash.update(id.as_bytes());
        } else {
            // Legacy credentials have no verified identity. Never attach a shared
            // database to them or guess identity from overlapping book ownership.
            hash.update(b"legacy-registration:"); hash.update(self.refresh.as_bytes());
        }
        format!("{:x}", hash.finalize())
    }
}
impl MetadataState {
    pub fn new(app: &tauri::AppHandle) -> Self {
        if app.state::<crate::storage::Storage>().restore_blocked() {
            return Self::with_auth(None);
        }
        let loaded = app.state::<crate::storage::Storage>().load_restorable_account().and_then(|raw| {
            raw.map(|raw| {
                serde_json::from_slice(&raw)
                    .map_err(|_| "Saved account format could not be read".to_owned())
            })
            .transpose()
        });
        match loaded {
            Ok(auth) => Self::with_auth(auth),
            Err(_) => {
                let state = Self::with_auth(None);
                *state.login.lock().unwrap() = LoginStatus {
                    phase: "failed",
                    message: "Saved account could not be unlocked. Your saved data has been kept; try signing in again.".into(),
                };
                state.trace("startup: protected account restore failed".into());
                state
            }
        }
    }
    fn with_auth(auth: Option<Auth>) -> Self {
        let phase = if auth.is_some() { "connected" } else { "idle" };
        Self {
            durability: Mutex::new(()),
            namespace: Mutex::new(auth.as_ref().map(Auth::namespace)),
            connected_generation: AtomicU64::new(if auth.is_some() { 1 } else { 0 }),
            login: Mutex::new(LoginStatus {
                phase,
                message: String::new(),
            }),
            pending: Mutex::new(None),
            generation: AtomicU64::new(0),
            auth: tokio::sync::Mutex::new(auth),
            upload: Mutex::new(None),
            diagnostics: Mutex::new(Vec::new()),
        }
    }
    // Only locally constructed stages and presence flags belong here. Never add
    // provider messages, URLs, response bodies, codes, cookies or token values.
    fn trace(&self, entry: String) {
        if let Ok(mut entries) = self.diagnostics.lock() {
            if entries.len() == 24 {
                entries.remove(0);
            }
            entries.push(entry);
        }
    }
    async fn finish_registration(
        &self,
        generation: u64,
        auth: Auth,
        persist: impl FnOnce(&Auth) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut stored = self.auth.lock().await;
        let _durability = self.durability.lock().map_err(|_| "Account storage unavailable")?;
        if self.generation.load(Ordering::SeqCst) != generation {
            return Err("API sign-in canceled".into());
        }
        persist(&auth)?;
        if self.generation.load(Ordering::SeqCst) != generation {
            return Err("API sign-in canceled".into());
        }
        *self.namespace.lock().map_err(|_| "Account storage unavailable")? = Some(auth.namespace());
        *stored = Some(auth);
        self.connected_generation
            .store(generation + 1, Ordering::SeqCst);
        Ok(())
    }
    fn connected(&self) -> bool {
        self.connected_generation.load(Ordering::SeqCst)
            == self.generation.load(Ordering::SeqCst) + 1
    }
    fn guard_generation(&self, generation: u64) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        let guard = self.durability.lock().map_err(|_| "Account storage unavailable")?;
        if !self.connected() || self.generation.load(Ordering::SeqCst) != generation {
            return Err("Account changed; operation canceled".into());
        }
        Ok(guard)
    }
    async fn connection(&self) -> Result<ConnectionStatus, String> {
        let auth = self.auth.lock().await;
        let login = self
            .login
            .lock()
            .map_err(|_| "Sign-in status unavailable")?
            .clone();
        Ok(ConnectionStatus {
            connected: self.connected() && auth.is_some(),
            profile: auth
                .as_ref()
                .filter(|_| self.connected())
                .map(|a| a.profile),
            phase: if self.connected() && auth.is_some() {
                "connected"
            } else {
                login.phase
            },
            message: login.message,
            version: env!("CARGO_PKG_VERSION"),
            build: if cfg!(debug_assertions) {
                "development"
            } else {
                "installed"
            },
            diagnostics: self
                .diagnostics
                .lock()
                .map_err(|_| "Sign-in diagnostic unavailable")?
                .clone(),
        })
    }
    fn cancel_before_callback(&self) -> bool {
        let cancelled = self
            .pending
            .lock()
            .ok()
            .and_then(|mut p| p.take())
            .is_some();
        if cancelled {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        cancelled
    }
}
pub(crate) fn connected(app: &tauri::AppHandle) -> bool {
    app.try_state::<MetadataState>()
        .is_some_and(|state| state.connected())
}
pub(crate) fn account_namespace(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(state) = app.try_state::<MetadataState>() else { return Ok(None); };
    let namespace = state.namespace.lock().map_err(|_| "Account storage unavailable")?;
    Ok(if state.connected() { namespace.clone() } else { None })
}
pub(crate) fn require_account(app: &tauri::AppHandle) -> Result<(), String> {
    if connected(app) {
        Ok(())
    } else {
        Err("Sign in before accessing your library or playback.".into())
    }
}
fn abort_login(app: &tauri::AppHandle, message: &str) {
    let state = app.state::<MetadataState>();
    let _ = state.cancel_before_callback();
    state.generation.fetch_add(1, Ordering::SeqCst);
    login_report(app, "failed", message);
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Cannot initialize secure API client".into())
}
fn save(app: &tauri::AppHandle, auth: &Auth) -> Result<(), String> {
    let raw =
        zeroize::Zeroizing::new(serde_json::to_vec(auth).map_err(|_| "Cannot encode account")?);
    crate::storage::save_account(app, &raw)
}
fn save_verified(app: &tauri::AppHandle, auth: &Auth) -> Result<(), String> {
    save(app, auth)?;
    let raw = crate::storage::load_account(app)?
        .ok_or("Protected account was saved but could not be reloaded")?;
    let restored: Auth =
        serde_json::from_slice(&raw).map_err(|_| "Protected account verification failed")?;
    if restored.access != auth.access
        || restored.refresh != auth.refresh
        || restored.profile != auth.profile
        || restored.account_id != auth.account_id
    {
        return Err("Protected account verification failed".into());
    }
    app.state::<MetadataState>()
        .trace("protected account: saved and read back".into());
    app.state::<MetadataState>().trace(if auth.account_id.is_some() {
        "account storage: provider identity available"
    } else {
        "account storage: isolated registration (provider identity unavailable)"
    }.into());
    app.state::<crate::storage::Storage>().allow_account_restore()?;
    Ok(())
}
fn random(n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    OsRng.fill_bytes(&mut b);
    b
}
fn client_id(serial: &str, profile: Profile) -> String {
    format!("{serial}#{}", profile.device_type())
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn authorize(p: &Pending) -> url::Url {
    let id = client_id(&p.serial, p.profile);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(p.verifier.as_bytes()));
    let mut url = url::Url::parse("https://www.amazon.com/ap/signin").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("openid.oa2.response_type", "code"),
        ("openid.return_to", "https://www.amazon.com/ap/maplanding"),
        ("openid.oa2.code_challenge_method", "S256"),
        ("openid.assoc_handle", p.profile.assoc()),
        (
            "openid.identity",
            "http://specs.openid.net/auth/2.0/identifier_select",
        ),
        ("pageId", p.profile.page()),
        ("accountStatusPolicy", "P1"),
        (
            "openid.claimed_id",
            "http://specs.openid.net/auth/2.0/identifier_select",
        ),
        ("openid.mode", "checkid_setup"),
        ("openid.ns.oa2", "http://www.amazon.com/ap/ext/oauth/2"),
        ("openid.oa2.client_id", &format!("device:{id}")),
        ("language", "en_US"),
        (
            "openid.ns.pape",
            "http://specs.openid.net/extensions/pape/1.0",
        ),
        ("openid.oa2.code_challenge", &challenge),
        ("marketPlaceId", "AF2M0KC94RCEA"),
        ("forceMobileLayout", "true"),
        ("openid.ns", "http://specs.openid.net/auth/2.0"),
        ("openid.pape.max_auth_age", "0"),
        ("openid.oa2.scope", "device_auth_access"),
    ]);
    url
}
#[derive(Clone, Serialize)]
struct LoginStatus {
    phase: &'static str,
    message: String,
}
fn login_report(app: &tauri::AppHandle, phase: &'static str, message: impl Into<String>) {
    let message = message.into();
    app.state::<MetadataState>()
        .trace(format!("stage: {phase}"));
    if let Ok(mut status) = app.state::<MetadataState>().login.lock() {
        *status = LoginStatus {
            phase,
            message: message.clone(),
        };
    }
    let _ = app.emit_to("main", "official-status", message);
    let _ = app.emit_to("main", "api-account-changed", ());
}
fn callback_code(u: &url::Url) -> Option<String> {
    if u.scheme() != "https"
        || u.host_str() != Some("www.amazon.com")
        || u.path() != "/ap/maplanding"
        || u.port_or_known_default() != Some(443)
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return None;
    }
    let mut codes = u
        .query_pairs()
        .chain(url::form_urlencoded::parse(
            u.fragment().unwrap_or("").as_bytes(),
        ))
        .filter(|(k, _)| k == "openid.oa2.authorization_code")
        .map(|(_, v)| v.into_owned());
    let code = codes.next()?;
    if codes.next().is_some() || code.is_empty() || code.len() >= 8192 {
        return None;
    }
    Some(code)
}
fn receive_callback(app: &tauri::AppHandle, url: &url::Url) -> bool {
    let Some(code) = callback_code(url) else {
        return false;
    };
    let pending = app
        .state::<MetadataState>()
        .pending
        .lock()
        .ok()
        .and_then(|mut p| p.take());
    if let Some(p) = pending {
        login_report(
            app,
            "registering",
            "Amazon sign-in received. Connecting your library…",
        );
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let generation = p.generation;
            let outcome = if now().saturating_sub(p.created) > 600 {
                Err("Sign-in expired; try again".into())
            } else {
                register(&app, p, code).await
            };
            if app
                .state::<MetadataState>()
                .generation
                .load(Ordering::SeqCst)
                != generation
            {
                return;
            }
            match outcome {
                Ok(()) => login_report(&app, "connected", "Audible connected."),
                Err(error) => login_report(&app, "failed", error),
            }
            if let Some(window) = app.get_webview_window("metadata-login") {
                let _ = window.close();
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        });
    }
    true
}
#[tauri::command]
pub async fn api_login(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    profile: Option<Profile>,
) -> Result<(), String> {
    super::local(&window)?;
    if app.state::<crate::storage::Storage>().restore_blocked() {
        crate::clear_official_session(&app)?;
        crate::storage::forget_account(&app)?;
    }
    if app.get_webview_window("metadata-login").is_some() {
        return Err("Finish or close the existing API sign-in window".into());
    }
    let state = app.state::<MetadataState>();
    if state.auth.lock().await.is_some() {
        return Err("Disconnect the current API account before connecting again".into());
    }
    if state
        .pending
        .lock()
        .map_err(|_| "Login state unavailable")?
        .is_some()
        || matches!(
            state
                .login
                .lock()
                .map_err(|_| "Login state unavailable")?
                .phase,
            "restoring" | "registering" | "saving"
        )
    {
        return Err("Finish or cancel the current sign-in first.".into());
    }
    let p = Pending {
        profile: profile.unwrap_or(Profile::Android),
        verifier: URL_SAFE_NO_PAD.encode(random(32)),
        serial: random(20).iter().map(|b| format!("{b:02x}")).collect(),
        created: now(),
        generation: state.generation.fetch_add(1, Ordering::SeqCst) + 1,
    };
    let url = authorize(&p);
    if let Ok(mut entries) = state.diagnostics.lock() {
        entries.clear();
    }
    login_report(&app, "signing-in", "Complete sign-in in the Amazon window.");
    *app.state::<MetadataState>()
        .pending
        .lock()
        .map_err(|_| "Login state unavailable")? = Some(p);
    let handle = app.clone();
    let close_handle = app.clone();
    let profile = crate::account_web_profile(&app)?;
    let login = WebviewWindowBuilder::new(&app, "metadata-login", WebviewUrl::External(url))
        .title("Connect library API · Amazon sign-in")
        .data_directory(profile)
        .incognito(!crate::privacy::remember(&app))
        .devtools(cfg!(debug_assertions))
        .inner_size(650.0, 780.0)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_navigation(move |u| {
            if receive_callback(&handle, u) {
                return false;
            }
            super::allowed(u)
        })
        .on_page_load({
            let app = app.clone();
            move |_, page| {
                if matches!(page.event(), tauri::webview::PageLoadEvent::Finished) {
                    receive_callback(&app, page.url());
                }
            }
        })
        .build()
        .map_err(|_| {
            abort_login(
                &app,
                "Cannot open Amazon sign-in. Close other sign-in windows and retry.",
            );
            "Cannot open API sign-in"
        })?;
    crate::privacy::harden_view(&login).map_err(|error| {
        abort_login(&app, &error);
        let _ = login.close();
        error
    })?;
    login.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
            // Once the callback has been accepted, closing the browser must not
            // invalidate the approved exchange or hide its completion result.
            if close_handle
                .state::<MetadataState>()
                .cancel_before_callback()
            {
                login_report(
                    &close_handle,
                    "cancelled",
                    "Sign-in closed before completion. Try again when ready.",
                );
            }
        }
    });
    Ok(())
}
fn registration_body(p: &Pending, code: &str) -> Value {
    json!({"age_info":{},"auth_data":{"client_id":client_id(&p.serial,p.profile),"authorization_code":code,"code_verifier":p.verifier,"code_algorithm":"SHA-256","client_domain":"DeviceLegacy"},"user_context_map":{"frc":STANDARD.encode(random(313)).trim_end_matches('=')},"cookies":{"website_cookies":[{"Name":"amzn-app-id","Value":"MAPiOSLib/6.0/ToHideRetailLink"},{"Name":"id_pk","Value":"eyJuIjoiMSJ9"}],"domain":".amazon.com"},"registration_data":p.profile.registration(&p.serial),"requested_extensions":["device_info","customer_info"],"requested_token_type":["bearer"]})
}
async fn register(app: &tauri::AppHandle, p: Pending, code: String) -> Result<(), String> {
    let auth = exchange_registration_traced(
        &http()?,
        "https://api.amazon.com/auth/register",
        &p,
        &code,
        |entry| app.state::<MetadataState>().trace(entry),
    )
    .await?;
    login_report(app, "saving", "Protecting your account on this PC…");
    app.state::<MetadataState>()
        .finish_registration(p.generation, auth, |auth| save_verified(app, auth))
        .await
}
#[cfg(test)]
async fn exchange_registration(
    client: &reqwest::Client,
    endpoint: &str,
    p: &Pending,
    code: &str,
) -> Result<Auth, String> {
    exchange_registration_traced(client, endpoint, p, code, |_| {}).await
}
async fn exchange_registration_traced(
    client: &reqwest::Client,
    endpoint: &str,
    p: &Pending,
    code: &str,
    trace: impl Fn(String),
) -> Result<Auth, String> {
    // Production has one fixed destination. Only synthetic unit tests may supply
    // their loopback fixture server; no endpoint is exposed to the frontend.
    if !cfg!(test) && endpoint != "https://api.amazon.com/auth/register" {
        return Err("Unsupported authentication destination".into());
    }
    trace("registration: sending account exchange".into());
    trace(format!(
        "registration context: profile={} · attempt age={}s",
        if p.profile == Profile::Android {
            "Android"
        } else {
            "iOS"
        },
        now().saturating_sub(p.created)
    ));
    let response = client
        .post(endpoint)
        .header("x-amzn-identity-auth-domain", "api.amazon.com")
        .header("user-agent", p.profile.agent())
        .header("accept", "application/json")
        .header("accept-language", "en-US")
        .header("cache-control", "no-store")
        .header("cookie", "amzn-app-id=MAPiOSLib/6.0/ToHideRetailLink")
        .json(&registration_body(p, code))
        .send()
        .await
        .map_err(|_| "API registration network failure")?;
    trace(format!("registration: HTTP {}", response.status().as_u16()));
    let value = decode_registration(response, &trace).await?;
    trace(registration_shape(&value));
    registration_auth(&value, p.profile)
}
async fn decode_registration(
    mut response: reqwest::Response,
    trace: &impl Fn(String),
) -> Result<Value, String> {
    if response.status().is_success() {
        return decode(response).await;
    }
    let status = response.status().as_u16();
    // Error bodies may contain echoed credentials. Inspect only in native memory
    // with a strict limit; never log/return the body or arbitrary server strings.
    let mut bytes = zeroize::Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        format!("Amazon account exchange returned HTTP {status}; error details unavailable")
    })? {
        if bytes.len() + chunk.len() > 65_536 {
            return Err(format!("Amazon account exchange returned HTTP {status}; error details exceeded the safe size limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let reason = serde_json::from_slice::<Value>(&bytes)
        .map(|value| registration_rejection(&value))
        .unwrap_or_else(|_| "unrecognized server response; private details omitted".into());
    trace(format!("registration rejection: {reason}"));
    Err(format!(
        "Amazon account exchange returned HTTP {status} · {reason}"
    ))
}
fn registration_rejection(value: &Value) -> String {
    let code = [
        "/response/error/code",
        "/response/error/errorCode",
        "/error/code",
        "/code",
        "/error",
    ]
    .iter()
    .find_map(|path| value.pointer(path).and_then(Value::as_str));
    let known = match code {
        Some("InvalidToken") => "InvalidToken",
        Some("InvalidRequest") => "InvalidRequest",
        Some("InvalidValue") => "InvalidValue",
        Some("InvalidInput") => "InvalidInput",
        Some("InvalidClient") => "InvalidClient",
        Some("InvalidGrant") => "InvalidGrant",
        Some("InvalidAuthData") => "InvalidAuthData",
        Some("InvalidParameterValue") => "InvalidParameterValue",
        Some("InvalidParameter") => "InvalidParameter",
        Some("InvalidDevice") => "InvalidDevice",
        Some("InvalidDeviceType") => "InvalidDeviceType",
        Some("DuplicateDeviceName") => "DuplicateDeviceName",
        Some("DeviceRegistrationError") => "DeviceRegistrationError",
        Some("MissingParameter") => "MissingParameter",
        Some("Unauthorized") => "Unauthorized",
        Some("BadRequest") => "BadRequest",
        Some("InvalidAuthorizationCode") => "InvalidAuthorizationCode",
        Some("InvalidCodeVerifier") => "InvalidCodeVerifier",
        Some("invalid_grant") => "invalid_grant",
        Some("invalid_client") => "invalid_client",
        Some("invalid_request") => "invalid_request",
        _ => "unrecognized code",
    };
    let message = [
        "/response/error/message",
        "/error/message",
        "/error_description",
        "/message",
    ]
    .iter()
    .filter_map(|path| value.pointer(path).and_then(Value::as_str))
    .map(str::to_ascii_lowercase)
    .collect::<Vec<_>>()
    .join(" ");
    let category = if message.contains("expired") || message.contains("expiration") {
        "server reports an expired authorization"
    } else if message.contains("verifier")
        || message.contains("code_challenge")
        || message.contains("pkce")
    {
        "server reports a code verification problem"
    } else if message.contains("already used") || message.contains("already been used") {
        "server reports an authorization code already used"
    } else if message.contains("authorization code") || message.contains("authorization_code") {
        "server reports an authorization code problem"
    } else if message.contains("client_id")
        || message.contains("client id")
        || message.contains("client identifier")
    {
        "server reports a client identifier problem"
    } else if message.contains("device_type")
        || message.contains("device type")
        || message.contains("device name")
        || message.contains("registration_data")
    {
        "server reports a device registration problem"
    } else {
        "private server message omitted"
    };
    let fields = [
        "client_id",
        "client_domain",
        "code_verifier",
        "code_algorithm",
        "authorization_code",
        "registration_data",
        "device_type",
        "device_name",
        "device_serial",
        "requested_token_type",
        "frc",
    ]
    .into_iter()
    .filter(|field| message.contains(field))
    .collect::<Vec<_>>();
    if fields.is_empty() {
        format!("{known} · {category}")
    } else {
        format!(
            "{known} · {category} · field mentions: {}",
            fields.join(", ")
        )
    }
}
fn registration_shape(value: &Value) -> String {
    let bearer = value.pointer("/response/success/tokens/bearer");
    let present = |key| bearer.and_then(|b| b.get(key)).is_some();
    format!("registration fields: success={} error={} bearer={} access={} refresh={} expiry={} device={}",
        value.pointer("/response/success").is_some(),
        value.pointer("/response/error").is_some() || value.get("error").is_some(),
        bearer.is_some(), present("access_token"), present("refresh_token"), present("expires_in"),
        value.pointer("/response/success/extensions/device_info/device_type").is_some())
}
fn registration_auth(value: &Value, profile: Profile) -> Result<Auth, String> {
    if let Some(device) = value
        .pointer("/response/success/extensions/device_info/device_type")
        .and_then(Value::as_str)
    {
        if device != profile.device_type() {
            return Err(
                "Amazon returned a different API registration profile; connection was not saved"
                    .into(),
            );
        }
    }
    let bearer = value
        .pointer("/response/success/tokens/bearer")
        .ok_or("Amazon did not return account tokens; sign-in was not saved")?;
    Ok(Auth {
        account_id: value.pointer("/response/success/extensions/customer_info/user_id")
            .and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 512).map(str::to_owned),
        profile,
        access: field(bearer, "access_token")?,
        refresh: field(bearer, "refresh_token")?,
        expires: now() + expiry(bearer)?,
    })
}
fn field(v: &Value, k: &str) -> Result<String, String> {
    v[k].as_str()
        .filter(|s| !s.is_empty() && s.len() < 60000)
        .map(str::to_owned)
        .ok_or("Unexpected authentication response".into())
}
fn expiry(value: &Value) -> Result<u64, String> {
    // Amazon may return an integer, a floating-point number, or either as a
    // string. Round down so we never extend the server's stated token lifetime.
    let seconds = value["expires_in"]
        .as_f64()
        .or_else(|| {
            value["expires_in"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .filter(|n| n.is_finite() && *n >= 1.0 && *n < 31_536_000.0)
        .ok_or("Invalid token expiry")?;
    Ok(seconds.floor() as u64)
}
async fn decode(mut response: reqwest::Response) -> Result<Value, String> {
    if !response.status().is_success() {
        return Err(format!(
            "Audible API returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "API response interrupted")?
    {
        if bytes.len() + chunk.len() > 20_000_000 {
            return Err("API response exceeds size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Unexpected API response format".into())
}
async fn fresh_token(
    app: &tauri::AppHandle,
    a: &mut Auth,
    client: &reqwest::Client,
) -> Result<reqwest::header::HeaderValue, String> {
    if a.expires <= now() + 60 {
        let response = client
            .post("https://api.amazon.com/auth/token")
            .form(&[
                ("app_name", a.profile.refresh_app().0),
                ("app_version", a.profile.refresh_app().1),
                ("source_token", &a.refresh),
                ("requested_token_type", "access_token"),
                ("source_token_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|_| "Token refresh network failure")?;
        let v = decode(response).await?;
        a.access = field(&v, "access_token")?;
        a.expires = now() + expiry(&v)?;
        save(app, a)?;
    }
    let mut token = reqwest::header::HeaderValue::from_str(&a.access)
        .map_err(|_| "Invalid protected credential")?;
    token.set_sensitive(true);
    Ok(token)
}
async fn get_page(
    app: &tauri::AppHandle,
    path: &str,
    query: &[(&str, &str)],
) -> Result<(Value, Option<String>), String> {
    require_account(app)?;
    let state = app.state::<MetadataState>();
    let generation = state.generation.load(Ordering::SeqCst);
    let mut guard = state.auth.lock().await;
    if !state.connected() || state.generation.load(Ordering::SeqCst) != generation {
        return Err("Account changed; request canceled".into());
    }
    let a = guard.as_mut().ok_or("Connect library API first")?;
    let client = http()?;
    let token = fresh_token(&app, a, &client).await?;
    let response = client
        .get(format!("https://api.audible.com{path}"))
        .header("x-amz-access-token", token)
        .query(query)
        .send()
        .await
        .map_err(|_| "Library API network failure")?;
    let continuation = response
        .headers()
        .get("Continuation-Token")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let value = decode(response).await?;
    if !state.connected() || state.generation.load(Ordering::SeqCst) != generation {
        return Err("Account changed; response discarded".into());
    }
    Ok((value, continuation))
}
async fn get(app: &tauri::AppHandle, path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
    Ok(get_page(app, path, query).await?.0)
}
#[tauri::command]
pub async fn api_status(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<bool, String> {
    super::local(&window)?;
    Ok(connected(&app))
}
#[derive(Serialize)]
pub struct ConnectionStatus {
    connected: bool,
    profile: Option<Profile>,
    phase: &'static str,
    message: String,
    version: &'static str,
    build: &'static str,
    diagnostics: Vec<String>,
}
#[tauri::command]
pub async fn api_connection(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<ConnectionStatus, String> {
    super::local(&window)?;
    app.state::<MetadataState>().connection().await
}

#[derive(Serialize)]
pub struct StreamingProbe {
    status: String,
    drm: Option<String>,
    has_content_location: bool,
    has_license_payload: bool,
    message: String,
}
fn streaming_body(systems: &[String]) -> Result<Value, String> {
    let mut drms = Vec::new();
    if systems.iter().any(|s| s == "com.widevine.alpha") {
        drms.push("Widevine");
    }
    if systems.iter().any(|s| {
        matches!(
            s.as_str(),
            "com.microsoft.playready" | "com.microsoft.playready.recommendation"
        )
    }) {
        drms.push("PlayReady");
    }
    if drms.is_empty() {
        return Err("No supported DRM runtime was supplied; no license request made".into());
    }
    Ok(
        json!({"consumption_type":"Streaming","quality":"High","chapter_titles_type":"Tree","response_groups":"content_reference,chapter_info,last_position_heard,content_url","supported_media_features":{"drm_types":drms,"codecs":["mp4a.40.2"],"chapter_titles_type":"Tree"}}),
    )
}
fn streaming_probe(v: &Value) -> StreamingProbe {
    let c = &v["content_license"];
    let urls = &c["content_metadata"]["content_url"];
    StreamingProbe{status:c["status_code"].as_str().unwrap_or("not reported").chars().take(100).collect(),drm:c["drm_type"].as_str().map(|s|s.chars().take(100).collect()),has_content_location:stream::manifest_location(v).is_some()||urls.as_object().map(|o|!o.is_empty()).unwrap_or(false),has_license_payload:c.get("license_response").map(|v|!v.is_null()&&v.as_str()!=Some("")).unwrap_or(false),message:"Streaming-grant probe finished. Widevine may carry its manifest URL in license_response. Use Load direct playback to test the media and platform license exchange".into()}
}
#[tauri::command]
pub async fn api_streaming_probe(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
    key_systems: Vec<String>,
) -> Result<StreamingProbe, String> {
    super::local(&window)?;
    if !super::valid_asin(&asin) || key_systems.len() > 3 {
        return Err("Invalid streaming probe request".into());
    }
    let c = super::db(&app)?;
    let owned = c
        .query_row("SELECT COUNT(*) FROM books WHERE asin=?1", [&asin], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| "Cannot check library")?
        > 0;
    if !owned {
        return Err("Sync this title into your API library first".into());
    }
    drop(c);
    let body = streaming_body(&key_systems)?;
    let state = app.state::<MetadataState>();
    let mut auth = state.auth.lock().await;
    let auth = auth.as_mut().ok_or("Connect library API first")?;
    let client = http()?;
    let token = fresh_token(&app, auth, &client).await?;
    let response = client
        .post(format!(
            "https://api.audible.com/1.0/content/{asin}/licenserequest"
        ))
        .header("x-amz-access-token", token)
        .json(&body)
        .send()
        .await
        .map_err(|_| "Streaming grant request failed; no audio was loaded")?;
    let v = decode(response).await?;
    Ok(streaming_probe(&v))
}
#[tauri::command]
pub async fn api_disconnect(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    super::local(&window)?;
    // Invalidate access before waiting for network/auth locks or clearing cookies.
    app.state::<MetadataState>()
        .generation
        .fetch_add(1, Ordering::SeqCst);
    let durable_cleanup = {
        let state = app.state::<MetadataState>();
        let _guard = state.durability.lock().map_err(|_| "Account storage unavailable")?;
        app.state::<crate::storage::Storage>().block_account_restore()
    };
    let _ = app.emit("account-signed-out", ());
    login_report(&app, "signing-out", "Signing out and stopping playback…");
    let playback_cleanup = crate::desktop::reset_player(&app);
    let login_cleanup = if let Some(w) = app.get_webview_window("metadata-login") {
        w.destroy().map_err(|_| "Cannot close API login".to_owned())
    } else {
        Ok(())
    };
    let pending_cleanup = app
        .state::<MetadataState>()
        .pending
        .lock()
        .map(|mut pending| *pending = None)
        .map_err(|_| "Login state unavailable".to_owned());
    let state = app.state::<MetadataState>();
    let mut auth = state.auth.lock().await;
    let token_cleanup = crate::storage::forget_account(&app);
    *auth = None;
    drop(auth);
    // The old restore button must not bypass an intentional installed-app logout.
    let legacy_cleanup: Result<(), String> = {
        #[cfg(not(test))]
        {
            if !cfg!(debug_assertions) {
                crate::privacy::delete_account("AudibleDesktop.Metadata.US")
            } else {
                Ok(())
            }
        }
        #[cfg(test)]
        {
            Ok(())
        }
    };
    stream::invalidate(&app);
    let website_cleanup = crate::clear_official_session(&app);
    for result in [
        durable_cleanup,
        token_cleanup,
        legacy_cleanup,
        website_cleanup,
        playback_cleanup,
        login_cleanup,
        pending_cleanup,
    ] {
        if let Err(error) = result {
            login_report(&app, "failed", format!("Account access stopped, but sign-out cleanup failed: {error}. Close the app and retry before sharing this Windows session."));
            return Err(error);
        }
    }
    login_report(
        &app,
        "idle",
        "Signed out. Playback stopped and your library is hidden.",
    );
    Ok(())
}
#[tauri::command]
pub async fn api_sync(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Vec<Value>, String> {
    super::local(&window)?;
    require_account(&app)?;
    let generation = app
        .state::<MetadataState>()
        .generation
        .load(Ordering::SeqCst);
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut continuation: Option<String> = None;
    let mut tokens = std::collections::HashSet::new();
    for _ in 0..1000 {
        let mut query = vec![
            ("num_results", "100"),
            (
                "response_groups",
                "contributors,product_attrs,series,media,customer_rights",
            ),
            ("sort_by", "-PurchaseDate"),
        ];
        if let Some(ref token) = continuation {
            query.push(("continuation_token", token));
        }
        let (v, next) = get_page(&app, "/1.0/library", &query).await?;
        let rows = v["items"]
            .as_array()
            .ok_or("Library API returned no items array")?;
        for row in rows {
            let asin = row["asin"].as_str().ok_or("Library item missing ASIN")?;
            if seen.insert(asin.to_owned()) {
                items.push(row.clone());
            }
        }
        match next {
            Some(ref token) if !tokens.insert(token.clone()) => {
                return Err(
                    "Library API repeated a continuation token; cache was not changed".into(),
                )
            }
            Some(token) => continuation = Some(token),
            None => {
                if !connected(&app)
                    || app
                        .state::<MetadataState>()
                        .generation
                        .load(Ordering::SeqCst)
                        != generation
                {
                    return Err("Account changed; library sync was canceled.".into());
                }
                let raw = serde_json::to_string(&items).map_err(|_| "Cannot encode library")?;
                let state = app.state::<MetadataState>();
                let _guard = state.guard_generation(generation)?;
                return super::cache_library(app.clone(), window, raw, true);
            }
        }
    }
    Err("Library exceeds pagination limit; cache was not changed".into())
}
#[derive(Clone, Serialize)]
pub struct Chapter {
    pub title: String,
    pub start: f64,
    pub length: Option<f64>,
}
#[derive(Serialize)]
pub struct CardPositionRefresh {
    pub updated: usize,
    pub warnings: Vec<String>,
}
fn save_cloud_position(app: &tauri::AppHandle, asin: &str, position: f64) -> Result<(), String> {
    super::db(app)?
        .execute(
            "INSERT OR REPLACE INTO cloud_positions(asin,position,checked) VALUES(?1,?2,?3)",
            rusqlite::params![asin, position, now() as i64],
        )
        .map_err(|_| "Cannot cache Audible position")?;
    let _ = app.emit_to("main", "card-positions-changed", ());
    Ok(())
}
#[tauri::command]
pub async fn api_refresh_card_positions(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<CardPositionRefresh, String> {
    super::local(&window)?;
    let state = app.state::<MetadataState>();
    let generation = state.generation.load(Ordering::SeqCst);
    if app.state::<MetadataState>().auth.lock().await.is_none() {
        return Err("Connect library API to refresh Audible positions".into());
    }
    let asins = {
        let c = super::db(&app)?;
        let mut q = c
            .prepare("SELECT asin FROM books ORDER BY asin")
            .map_err(|_| "Cannot read cached titles")?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|_| "Cannot read cached titles")?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Cannot read cached titles")?
    };
    let mut result = CardPositionRefresh {
        updated: 0,
        warnings: Vec::new(),
    };
    for chunk in asins.chunks(20) {
        let query = chunk.join(",");
        match get(&app, "/1.0/annotations/lastpositions", &[("asins", &query)]).await {
            Ok(v) => {
                let _guard = state.guard_generation(generation)?;
                for asin in chunk {
                    if let Some(position) = annotation_position(&v, asin) {
                        save_cloud_position(&app, asin, position)?;
                        result.updated += 1;
                    }
                }
            }
            Err(e) => {
                result.warnings.push(e);
                break;
            }
        }
    }
    if result.updated == 0 && result.warnings.is_empty() && !asins.is_empty() {
        result.warnings.push(
            "Audible returned no supported card positions; previous values were retained".into(),
        );
    }
    Ok(result)
}
#[derive(Serialize)]
pub struct CloudMetadata {
    pub position: Option<f64>,
    pub chapters: Vec<f64>,
    pub chapter_details: Vec<Chapter>,
    pub duration: Option<f64>,
    pub position_source: String,
    pub warnings: Vec<String>,
}
fn numeric(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .filter(|n| n.is_finite() && *n >= 0.0 && *n < 31_536_000_000.0)
}
fn position_from(v: &Value) -> Option<f64> {
    numeric(&v["position_ms"]).map(|n| n / 1000.0)
}
fn annotation_position(v: &Value, asin: &str) -> Option<f64> {
    let rows = v
        .as_array()
        .or_else(|| v.get("last_positions").and_then(Value::as_array))
        .or_else(|| v.get("lastpositions").and_then(Value::as_array))
        .or_else(|| v.get("items").and_then(Value::as_array));
    if let Some(rows) = rows {
        return rows
            .iter()
            .find(|r| r["asin"].as_str() == Some(asin))
            .and_then(position_from);
    }
    v.get("last_positions")
        .and_then(|p| p.get(asin))
        .or_else(|| v.get(asin))
        .and_then(position_from)
}
fn walk_chapters(rows: &Value, depth: usize, out: &mut Vec<Chapter>) {
    if depth > 16 || out.len() >= 2000 {
        return;
    }
    if let Some(rows) = rows.as_array() {
        for c in rows {
            if out.len() >= 2000 {
                break;
            }
            if let Some(ms) = numeric(&c["start_offset_ms"]) {
                let title = c["title"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.chars().take(500).collect())
                    .unwrap_or_else(|| format!("Chapter {}", out.len() + 1));
                out.push(Chapter {
                    title,
                    start: ms / 1000.0,
                    length: numeric(&c["length_ms"]).map(|n| n / 1000.0),
                });
            }
            walk_chapters(&c["chapters"], depth + 1, out);
        }
    }
}
pub fn cloud_metadata(v: &Value) -> CloudMetadata {
    let m = v.get("content_metadata").unwrap_or(v);
    let position = position_from(&m["last_position_heard"]);
    let mut details = Vec::new();
    walk_chapters(&m["chapter_info"]["chapters"], 0, &mut details);
    details.sort_by(|a, b| a.start.total_cmp(&b.start));
    details.dedup_by(|a, b| a.start == b.start);
    let duration = numeric(&m["chapter_info"]["runtime_length_ms"])
        .map(|n| n / 1000.0)
        .or_else(|| numeric(&m["chapter_info"]["runtime_length_sec"]));
    CloudMetadata {
        position,
        chapters: details.iter().map(|c| c.start).collect(),
        chapter_details: details,
        duration,
        position_source: if position.is_some() {
            "content metadata"
        } else {
            "unavailable"
        }
        .into(),
        warnings: Vec::new(),
    }
}
async fn details_request(
    app: &tauri::AppHandle,
    asin: &str,
    layout: &str,
) -> Result<Value, String> {
    get(
        app,
        &format!("/1.0/content/{asin}/metadata"),
        &[
            ("drm_type", "Adrm"),
            ("quality", "High"),
            ("chapter_titles_type", layout),
            (
                "response_groups",
                "last_position_heard,content_reference,chapter_info",
            ),
        ],
    )
    .await
}
fn cache_details(app: &tauri::AppHandle, asin: &str, m: &CloudMetadata) -> Result<(), String> {
    let c = super::db(app)?;
    let raw = c
        .query_row("SELECT metadata FROM books WHERE asin=?1", [asin], |r| {
            r.get::<_, String>(0)
        })
        .map_err(|_| "Title not in cached library")?;
    let mut book: Value = serde_json::from_str(&raw).map_err(|_| "Invalid cached book")?;
    if !m.chapters.is_empty() {
        book["chapters"] = json!(m.chapters);
        book["chapterDetails"] = json!(m.chapter_details);
    }
    if let Some(duration) = m.duration {
        book["duration"] = json!(duration);
    }
    c.execute(
        "UPDATE books SET metadata=?1 WHERE asin=?2",
        rusqlite::params![book.to_string(), asin],
    )
    .map_err(|_| "Cannot cache chapter metadata")?;
    let hub = app.state::<super::desktop::PlayerHub>();
    let mut state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    if state.book.as_ref().and_then(|b| b["asin"].as_str()) == Some(asin) {
        state.book = Some(book.clone());
        for label in ["main", "mini-player"] {
            let _ = app.emit_to(label, "player-book-details", book.clone());
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn api_position(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<CloudMetadata, String> {
    super::local(&window)?;
    let state = app.state::<MetadataState>();
    let generation = state.generation.load(Ordering::SeqCst);
    if !super::valid_asin(&asin) {
        return Err("Invalid ASIN".into());
    }
    if app.state::<MetadataState>().auth.lock().await.is_none() {
        return Err(
            "Connect library API to load cloud progress and chapters; playback sign-in is separate"
                .into(),
        );
    }
    let mut result = match details_request(&app, &asin, "Tree").await {
        Ok(v) => cloud_metadata(&v),
        Err(e) => {
            let mut m = cloud_metadata(&json!({}));
            m.warnings.push(format!("Chapter metadata request: {e}"));
            m
        }
    };
    if result.chapters.is_empty() {
        match details_request(&app, &asin, "Flat").await {
            Ok(v) => {
                let fallback = cloud_metadata(&v);
                if result.position.is_none() {
                    result.position = fallback.position;
                    result.position_source = fallback.position_source;
                }
                if !fallback.chapters.is_empty() {
                    result.chapters = fallback.chapters;
                    result.chapter_details = fallback.chapter_details;
                    result.duration = fallback.duration;
                }
            }
            Err(e) => result
                .warnings
                .push(format!("Chapter fallback request: {e}")),
        }
    }
    match get(&app, "/1.0/annotations/lastpositions", &[("asins", &asin)]).await {
        Ok(v) => {
            if let Some(position) = annotation_position(&v, &asin) {
                result.position = Some(position);
                result.position_source = "last-positions API".into();
            } else {
                result.warnings.push(
                    "Last-positions API returned no supported position for this title".into(),
                );
            }
        }
        Err(e) => result.warnings.push(format!("Last-positions API: {e}")),
    };
    if result.chapters.is_empty() {
        result
            .warnings
            .push("Audible returned no usable chapter offsets for this title".into());
    }
    let _guard = state.guard_generation(generation)?;
    if !result.chapters.is_empty() {
        if let Err(e) = cache_details(&app, &asin, &result) {
            result.warnings.push(e)
        }
    }
    if let Some(position) = result.position {
        if let Err(e) = save_cloud_position(&app, &asin, position) {
            result.warnings.push(e);
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_profile_is_consistent_across_pkce_client_and_registration() {
        for profile in [Profile::Ios, Profile::Android] {
            let pending = Pending {
                profile,
                verifier: "fake-test-verifier".into(),
                serial: "TESTSERIAL".into(),
                created: 0,
                generation: 1,
            };
            let url = authorize(&pending);
            let pairs = url
                .query_pairs()
                .collect::<std::collections::HashMap<_, _>>();
            let client = client_id(&pending.serial, profile);
            assert_eq!(
                pairs.get("openid.oa2.client_id").unwrap().as_ref(),
                format!("device:{client}")
            );
            assert_eq!(
                pairs.get("openid.assoc_handle").unwrap().as_ref(),
                profile.assoc()
            );
            assert_eq!(pairs.get("pageId").unwrap().as_ref(), profile.page());
            assert!(!url.as_str().contains("fake-test-verifier"));
            let body = registration_body(&pending, "fake-test-code");
            assert_eq!(body["auth_data"]["client_id"], client);
            assert_eq!(
                body["registration_data"]["device_type"],
                profile.device_type()
            );
            assert_eq!(body["requested_token_type"], json!(["bearer"]));
            assert_eq!(
                body["registration_data"]["device_model"],
                if profile == Profile::Android {
                    "OnePlus8"
                } else {
                    "iPhone"
                }
            );
        }
    }
    #[test]
    fn existing_credentials_remain_ios_until_explicit_reconnection() {
        let auth: Auth = serde_json::from_value(
            json!({"access":"fake-access","refresh":"fake-refresh","expires":1}),
        )
        .unwrap();
        assert_eq!(auth.profile, Profile::Ios);
        let android = Auth {
            account_id: auth.account_id.clone(),
            profile: Profile::Android,
            access: auth.access.clone(),
            refresh: auth.refresh.clone(),
            expires: auth.expires,
        };
        let roundtrip: Auth =
            serde_json::from_value(serde_json::to_value(android).unwrap()).unwrap();
        assert_eq!(roundtrip.profile, Profile::Android);
    }
    #[test]
    fn standalone_probe_requests_streaming_without_decryption_formats() {
        let body = streaming_body(&["com.widevine.alpha".into()]).unwrap();
        assert_eq!(body["consumption_type"], "Streaming");
        assert_eq!(
            body["supported_media_features"]["drm_types"],
            json!(["Widevine"])
        );
        assert!(streaming_body(&["unknown".into()]).is_err());
    }
    #[test]
    fn standalone_probe_never_returns_urls_or_license_bytes() {
        let v = json!({"content_license":{"status_code":"Granted","drm_type":"Widevine","license_response":"private-license-payload","content_metadata":{"content_url":{"streaming_url":"https://example.invalid/private-signed-url"}}}});
        let result = serde_json::to_string(&streaming_probe(&v)).unwrap();
        assert!(!result.contains("private-license-payload"));
        assert!(!result.contains("private-signed-url"));
        assert!(result.contains("Granted"));
    }
    #[test]
    fn widevine_probe_recognizes_manifest_instead_of_reporting_missing_location() {
        let probe = streaming_probe(
            &json!({"content_license":{"status_code":"Granted","drm_type":"Widevine","license_response":"https://media.example.test/manifest.mpd?secret=synthetic"}}),
        );
        assert!(probe.has_content_location);
        assert!(!serde_json::to_string(&probe)
            .unwrap()
            .contains("secret=synthetic"));
    }
    #[test]
    fn nested_chapters_preserve_titles_and_convert_offsets() {
        let m = cloud_metadata(
            &json!({"content_metadata":{"chapter_info":{"runtime_length_ms":"7200000","chapters":[{"title":"Introduction","start_offset_ms":"0","length_ms":"60000","chapters":[{"title":"First chapter","start_offset_ms":"60000","length_ms":120000}]}]}}}),
        );
        assert_eq!(m.chapters, vec![0.0, 60.0]);
        assert_eq!(m.chapter_details[1].title, "First chapter");
        assert_eq!(m.chapter_details[1].length, Some(120.0));
        assert_eq!(m.duration, Some(7200.0));
    }
    #[test]
    fn annotation_position_matches_only_the_requested_title() {
        let v = json!({"last_positions":[{"asin":"B012345678","position_ms":"123000"},{"asin":"B098765432","position_ms":456000}]});
        assert_eq!(annotation_position(&v, "B012345678"), Some(123.0));
        assert_eq!(annotation_position(&v, "B098765432"), Some(456.0));
        assert_eq!(annotation_position(&v, "B011111111"), None);
    }
    #[test]
    fn callback_is_exact_and_https() {
        assert!(callback_code(
            &url::Url::parse(
                "https://www.amazon.com/ap/maplanding?openid.oa2.authorization_code=synthetic"
            )
            .unwrap()
        )
        .is_some());
        for s in [
            "https://evil.test/ap/maplanding?openid.oa2.authorization_code=synthetic",
            "https://www.amazon.com/ap/signin?openid.oa2.authorization_code=synthetic",
            "http://www.amazon.com/ap/maplanding?openid.oa2.authorization_code=synthetic",
        ] {
            assert!(callback_code(&url::Url::parse(s).unwrap()).is_none());
        }
    }
    #[test]
    fn milliseconds_are_not_seconds() {
        let m = cloud_metadata(
            &json!({"content_metadata":{"last_position_heard":{"position_ms":123450},"chapter_info":{"chapters":[{"start_offset_ms":0},{"start_offset_ms":60000}]}}}),
        );
        assert_eq!(m.position, Some(123.45));
        assert_eq!(m.chapters, vec![0.0, 60.0]);
        assert!(cloud_metadata(&json!({})).position.is_none());
        assert_eq!(
            cloud_metadata(
                &json!({"content_metadata":{"last_position_heard":{"position_ms":"123450"}}})
            )
            .position,
            Some(123.45)
        );
    }
}

#[cfg(test)]
mod login_handoff_tests {
    use super::*;
    #[test]
    fn callback_accepts_fragment_codes_without_weakening_origin_checks() {
        assert_eq!(
            callback_code(
                &url::Url::parse(
                    "https://www.amazon.com/ap/maplanding#openid.oa2.authorization_code=dummy-code"
                )
                .unwrap()
            )
            .as_deref(),
            Some("dummy-code")
        );
        for value in ["https://evil.test/ap/maplanding#openid.oa2.authorization_code=dummy", "https://www.amazon.com:8443/ap/maplanding#openid.oa2.authorization_code=dummy", "https://user@www.amazon.com/ap/maplanding#openid.oa2.authorization_code=dummy", "https://www.amazon.com/ap/maplanding?openid.oa2.authorization_code=one#openid.oa2.authorization_code=two"]{assert!(callback_code(&url::Url::parse(value).unwrap()).is_none());}
    }
    #[test]
    fn closing_browser_after_callback_does_not_cancel_approved_exchange() {
        let state = MetadataState::with_auth(None);
        state.generation.store(7, Ordering::SeqCst);
        *state.pending.lock().unwrap() = Some(Pending {
            profile: Profile::Android,
            verifier: "dummy".into(),
            serial: "dummy".into(),
            created: now(),
            generation: 7,
        });
        let approved = state.pending.lock().unwrap().take().unwrap();
        assert!(!state.cancel_before_callback());
        assert_eq!(state.generation.load(Ordering::SeqCst), approved.generation);
    }
    #[test]
    fn closing_browser_before_callback_invalidates_pending_login() {
        let state = MetadataState::with_auth(None);
        state.generation.store(7, Ordering::SeqCst);
        *state.pending.lock().unwrap() = Some(Pending {
            profile: Profile::Android,
            verifier: "dummy".into(),
            serial: "dummy".into(),
            created: now(),
            generation: 7,
        });
        assert!(state.cancel_before_callback());
        assert!(state.pending.lock().unwrap().is_none());
        assert_eq!(state.generation.load(Ordering::SeqCst), 8);
    }
}

#[cfg(test)]
mod login_tests;
