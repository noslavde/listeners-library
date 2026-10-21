//! Development-only protected-stream grant, fetch and license-relay experiment.
//! Keeps native session context and bounded network responses; a granted
//! entitlement is not evidence that the platform license challenge will be
//! accepted.

//! Protected streaming through the platform CDM. No content keys leave the CDM.
use super::*;
#[derive(Clone)]
struct Session {
    id: String,
    asin: String,
    manifest: url::Url,
    generation: u64,
    license_id: Option<String>,
    website: super::website::WebsiteSession,
}
pub struct StreamState(Mutex<Option<Session>>, AtomicU64);
impl StreamState {
    pub fn new() -> Self {
        Self(Mutex::new(None), AtomicU64::new(0))
    }
}
pub(crate) fn invalidate(app: &tauri::AppHandle) {
    super::browser_test::stop(app);
    if let Ok(mut session) = app.state::<StreamState>().0.lock() {
        *session = None;
    }
    let _ = app.emit_to("main", "api-stream-disconnected", ());
}
pub(super) fn cancel_pending(app: &tauri::AppHandle) {
    app.state::<StreamState>().1.fetch_add(1, Ordering::SeqCst);
}
#[derive(Serialize)]
pub struct StreamStart {
    pub(super) id: String,
    manifest: String,
    resume: Option<f64>,
    certificate: Vec<u8>,
    profile: Profile,
    has_license_context: bool,
    license_auth: &'static str,
    grant_kind: &'static str,
    grant_right: &'static str,
    grant_start_seconds: Option<f64>,
    grant_end_seconds: Option<f64>,
    runtime_version: Option<String>,
}
fn certificate_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let encoded = value
        .as_str()
        .or_else(|| value.get("certificate").and_then(Value::as_str))
        .filter(|s| !s.is_empty() && s.len() <= 256_000)
        .ok_or("Audible returned no supported Widevine service certificate")?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid encoded Widevine service certificate")?;
    if bytes.len() < 32 {
        return Err("Widevine service certificate is too short".into());
    }
    Ok(bytes)
}
async fn service_certificate(app: &tauri::AppHandle, grant: &Value) -> Result<Vec<u8>, String> {
    // Request the provider certificate with the grant first. Do not substitute
    // a third-party/test certificate or an extracted device certificate.
    for value in [
        &grant["content_license"]["certificate"],
        &grant["content_license"]["content_metadata"]["certificate"],
    ] {
        if !value.is_null() {
            return certificate_bytes(value);
        }
    }
    // Mirrors Audible's content certificate resource for the granted key system.
    // This Widevine endpoint still needs an authenticated runtime verification.
    let started = std::time::Instant::now();
    let state = app.state::<MetadataState>();
    let mut guard = state.auth.lock().await;
    let client = http()?;
    let token = fresh_token(
        &app,
        guard.as_mut().ok_or("Connect library API first")?,
        &client,
    )
    .await?;
    let response = client
        .get("https://api.audible.com/1.0/content/Widevine/certificate")
        .header("x-amz-access-token", token)
        .send()
        .await
        .map_err(|_| "Widevine service certificate network failure")?;
    let value = decode_stream_response(response, "Widevine service certificate", started).await?;
    certificate_bytes(&value["certificate"])
}
fn is_service_certificate_request(challenge: &[u8]) -> bool {
    challenge == [0x08, 0x04]
}
#[derive(Serialize)]
pub struct MediaResponse {
    data: Vec<u8>,
    content_type: String,
    uri: String,
}
pub(super) fn manifest_location(v: &Value) -> Option<url::Url> {
    let c = &v["content_license"];
    if c["drm_type"].as_str() != Some("Widevine") {
        return None;
    }
    let raw = c["license_response"].as_str()?;
    let u = url::Url::parse(raw).ok()?;
    if secure(&u) {
        Some(u)
    } else {
        None
    }
}
fn secure(u: &url::Url) -> bool {
    u.scheme() == "https"
        && u.port_or_known_default() == Some(443)
        && u.username().is_empty()
        && u.password().is_none()
        && u.host_str()
            .map(|h| {
                h.contains('.')
                    && !h.ends_with(".local")
                    && !h.ends_with(".localhost")
                    && h != "localhost"
                    && h.parse::<std::net::IpAddr>().is_err()
            })
            .unwrap_or(false)
}
fn selected_title(app: &tauri::AppHandle, asin: &str) -> bool {
    app.state::<crate::desktop::PlayerHub>()
        .0
        .lock()
        .map(|s| s.book.as_ref().and_then(|b| b["asin"].as_str()) == Some(asin))
        .unwrap_or(false)
}
pub(super) fn active(app: &tauri::AppHandle) -> bool {
    app.state::<StreamState>()
        .0
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false)
}
fn session(app: &tauri::AppHandle, id: &str) -> Result<Session, String> {
    let s = app
        .state::<StreamState>()
        .0
        .lock()
        .map_err(|_| "Stream state unavailable")?
        .as_ref()
        .filter(|s| s.id == id)
        .cloned()
        .ok_or("Streaming session expired; load the title again")?;
    if app
        .state::<MetadataState>()
        .generation
        .load(Ordering::SeqCst)
        != s.generation
    {
        return Err("API account changed; load the title again".into());
    }
    Ok(s)
}
async fn post(
    path: &str,
    body: Value,
    website: &super::website::WebsiteSession,
) -> Result<Value, String> {
    let stage = if path.ends_with("/drmlicense") {
        "Platform license"
    } else {
        "Streaming grant"
    };
    let started = std::time::Instant::now();
    let client = http()?;
    let response = client
        .post(super::website::endpoint(path)?)
        .headers(website.headers())
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            format!(
                "{stage} network failure: {} ({} ms)",
                if e.is_timeout() {
                    "timeout"
                } else if e.is_connect() {
                    "connection failed"
                } else {
                    "request failed"
                },
                started.elapsed().as_millis()
            )
        })?;
    decode_stream_response(response, stage, started).await
}
fn grant_license_id(grant: &Value) -> Option<String> {
    grant["content_license"]["license_id"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 16_000)
        .map(str::to_owned)
}
fn license_body(challenge: &[u8], license_id: Option<&str>) -> Value {
    let mut body = json!({"consumption_type":"Streaming","drm_type":"Widevine","tenant_id":"Audible","licenseChallenge":STANDARD.encode(challenge)});
    if let Some(id) = license_id {
        body["license_id"] = json!(id);
    }
    body
}

// Emit only protocol enum values we recognize, never arbitrary server text.
fn grant_kind(grant: &Value) -> &'static str {
    match grant["content_license"]["license_response_type"].as_str() {
        Some("FullTitle") => "full title",
        Some("Preview") => "preview",
        Some("CatalogSample") => "catalog sample",
        None => "not reported",
        _ => "unrecognized type omitted",
    }
}
fn grant_right(grant: &Value) -> &'static str {
    match grant["content_license"]["granted_right"].as_str() {
        Some("Ownership") => "ownership",
        Some("AYCL") => "membership catalog",
        None => "not reported",
        _ => "unrecognized right omitted",
    }
}
fn grant_offset(grant: &Value, field: &str) -> Option<f64> {
    grant["content_license"]["playback_info"][field]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 3_600_000_000.0)
        .map(|v| v / 1000.0)
}
fn server_error_code(value: &Value) -> &'static str {
    let mut present = false;
    for path in [
        "/error_code",
        "/code",
        "/error/code",
        "/error/error_code",
        "/errors/0/code",
        "/__type",
    ] {
        if let Some(raw) = value.pointer(path).and_then(Value::as_str) {
            present = true;
            // Match a fixed vocabulary, not merely a character/length check:
            // opaque credentials can also look like enum identifiers.
            match raw {
                "untrusted_client" => return "untrusted_client",
                "UntrustedClient" => return "UntrustedClient",
                "InvalidRequest" => return "InvalidRequest",
                "invalid_request" => return "invalid_request",
                "Unauthorized" => return "Unauthorized",
                "unauthorized_client" => return "unauthorized_client",
                "invalid_token" => return "invalid_token",
                "UnsupportedCdmVersion" => return "UnsupportedCdmVersion",
                "InvalidLicenseChallenge" => return "InvalidLicenseChallenge",
                _ => {}
            }
        }
    }
    if present {
        "unrecognized code omitted"
    } else {
        "not reported"
    }
}

// Classify known server errors without returning server-controlled text, IDs,
// request bodies, signed URLs, or authentication material to the diagnostics UI.
fn rejection_reason(value: &Value) -> &'static str {
    let nested = [
        "/error/message",
        "/error/error_message",
        "/status/error_message",
        "/errors/0/message",
        "/error/code",
        "/error/error_code",
    ]
    .iter()
    .filter_map(|path| value.pointer(path).and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    let code = value
        .get("error_code")
        .or_else(|| value.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let message = value
        .get("message")
        .or_else(|| value.get("error_message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let text = format!("{code} {message} {nested}");
    if text.contains("device")
        && (text.contains("unsupported")
            || text.contains("invalid")
            || text.contains("not support")
            || text.contains("not allowed"))
    {
        "device registration rejected"
    } else if text.contains("unsupportedcdm") || text.contains("unsupported_cdm") {
        "protected playback runtime version rejected (UnsupportedCdmVersion)"
    } else if text.contains("verified media path") {
        "protected playback verification rejected (Verified Media Path)"
    } else if text.contains("untrusted") {
        "playback request client rejected (untrusted client; runtime cause unconfirmed)"
    } else if text.contains("unauthoriz")
        || text.contains("authentication")
        || text.contains("token")
    {
        "account authentication rejected"
    } else if text.contains("entitle") || text.contains("ownership") || text.contains("rights") {
        "title rights rejected"
    } else if text.contains("challenge") {
        "platform challenge rejected"
    } else if text.contains("invalid") || text.contains("argument") || text.contains("parameter") {
        "request parameters rejected"
    } else {
        "server rejected request; unrecognized details omitted"
    }
}
async fn decode_stream_response(
    mut response: reqwest::Response,
    stage: &str,
    started: std::time::Instant,
) -> Result<Value, String> {
    let status = response.status();
    let limit = if status.is_success() {
        20_000_000
    } else {
        32_000
    };
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| format!("{stage} response transfer interrupted"))?
    {
        if bytes.len() + chunk.len() > limit {
            return Err(format!(
                "{stage} HTTP {}: response exceeds size limit",
                status.as_u16()
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_stream_response(
        status.as_u16(),
        &bytes,
        stage,
        started.elapsed().as_millis(),
    )
}
fn parse_stream_response(
    status: u16,
    bytes: &[u8],
    stage: &str,
    elapsed_ms: u128,
) -> Result<Value, String> {
    let parsed = serde_json::from_slice::<Value>(bytes);
    if !(200..300).contains(&status) {
        let reason = parsed
            .as_ref()
            .map(rejection_reason)
            .unwrap_or("non-JSON rejection; response body omitted");
        let code = parsed
            .as_ref()
            .map(server_error_code)
            .unwrap_or("not reported");
        return Err(format!(
            "{stage} HTTP {status}: {reason} · server code: {code} ({elapsed_ms} ms)"
        ));
    }
    parsed.map_err(|_| format!("{stage} HTTP {status}: unexpected JSON response format"))
}
#[tauri::command]
pub async fn api_stream_start(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<StreamStart, String> {
    crate::local(&window)?;
    start(app, asin).await
}
pub(super) async fn start(app: tauri::AppHandle, asin: String) -> Result<StreamStart, String> {
    let request_generation = app.state::<StreamState>().1.fetch_add(1, Ordering::SeqCst) + 1;
    if !crate::valid_asin(&asin) {
        return Err("Invalid ASIN".into());
    }
    let exists = {
        let c = crate::db(&app)?;
        c.query_row("SELECT COUNT(*) FROM books WHERE asin=?1", [&asin], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| "Cannot check library")?
            > 0
    };
    if !exists {
        return Err("Sync this title into the library first".into());
    }
    let generation = app
        .state::<MetadataState>()
        .generation
        .load(Ordering::SeqCst);
    let mut body = streaming_body(&["com.widevine.alpha".into()])?;
    body["supported_media_features"]["previews"] = json!(false);
    body["supported_media_features"]["catalog_samples"] = json!(false);
    body["response_groups"] =
        json!("content_reference,chapter_info,last_position_heard,content_url,certificate");
    let website = super::website::WebsiteSession::connect(&app).await?;
    body["asin"] = json!(asin);
    body["tenant_id"] = json!("Audible");
    body["use_adaptive_bit_rate"] = json!(true);
    body["spatial"] = json!(false);
    let v = post(
        &format!("/1.0/content/{asin}/licenserequest"),
        body,
        &website,
    )
    .await?;
    if v["content_license"]["status_code"].as_str() != Some("Granted") {
        return Err("Audible did not grant this streaming request".into());
    }
    let grant_kind = grant_kind(&v);
    if matches!(grant_kind, "preview" | "catalog sample") {
        return Err(format!("Audible granted a {grant_kind} instead of full-title playback; the website session has not established full access"));
    }
    let grant_right = grant_right(&v);
    let grant_start_seconds = grant_offset(&v, "start_offset_ms");
    let grant_end_seconds = grant_offset(&v, "end_offset_ms");
    let manifest = manifest_location(&v)
        .ok_or("The granted response did not contain a supported HTTPS Widevine manifest")?;
    let certificate = service_certificate(&app, &v).await?;
    let profile = app
        .state::<MetadataState>()
        .auth
        .lock()
        .await
        .as_ref()
        .ok_or("Connect library API first")?
        .profile;
    if !selected_title(&app, &asin) {
        return Err(
            "Selected title changed while loading; select Load direct playback again".into(),
        );
    }
    if generation
        != app
            .state::<MetadataState>()
            .generation
            .load(Ordering::SeqCst)
    {
        return Err("API account changed during grant request".into());
    }
    let id = URL_SAFE_NO_PAD.encode(random(24));
    if app.state::<StreamState>().1.load(Ordering::SeqCst) != request_generation {
        return Err("A newer playback load or stopped comparison superseded this request".into());
    }
    let resume =
        cloud_metadata(&json!({"content_metadata":v["content_license"]["content_metadata"]}))
            .position;
    let license_id = grant_license_id(&v);
    let has_license_context = license_id.is_some();
    crate::desktop::selected(&app, &asin)?;
    *app.state::<StreamState>()
        .0
        .lock()
        .map_err(|_| "Stream state unavailable")? = Some(Session {
        id: id.clone(),
        asin,
        manifest,
        generation,
        license_id,
        website,
    });
    Ok(StreamStart {
        id,
        manifest: "audible-media:manifest".into(),
        resume,
        certificate,
        profile,
        has_license_context,
        license_auth: "website-session-cookie",
        grant_kind,
        grant_right,
        grant_start_seconds,
        grant_end_seconds,
        runtime_version: tauri::webview_version()
            .ok()
            .filter(|v| v.len() < 80 && v.chars().all(|c| c.is_ascii_digit() || c == '.')),
    })
}
#[tauri::command]
pub async fn api_stream_fetch(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    uri: String,
    range: Option<String>,
) -> Result<MediaResponse, String> {
    crate::local(&window)?;
    fetch(app, id, uri, range).await
}
pub(super) async fn fetch(
    app: tauri::AppHandle,
    id: String,
    uri: String,
    range: Option<String>,
) -> Result<MediaResponse, String> {
    let s = session(&app, &id)?;
    let mut target = if uri == "audible-media:manifest" {
        s.manifest.clone()
    } else {
        url::Url::parse(&uri).map_err(|_| "Invalid media resource")?
    };
    if !secure(&target) || target.origin() != s.manifest.origin() {
        return Err(
            "The stream requested a different CDN origin; it was not authorized by this session"
                .into(),
        );
    }
    if let Some(ref range) = range {
        if range.len() > 80
            || !range.starts_with("bytes=")
            || !range[6..]
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'-' || b == b',')
        {
            return Err("Invalid media byte range".into());
        }
    }
    let client = http()?;
    let media_token = if s
        .manifest
        .host_str()
        .map(|h| h == "audible.com" || h.ends_with(".audible.com"))
        .unwrap_or(false)
    {
        let state = app.state::<MetadataState>();
        let mut auth = state.auth.lock().await;
        Some(
            fresh_token(
                &app,
                auth.as_mut().ok_or("Connect library API first")?,
                &client,
            )
            .await?,
        )
    } else {
        None
    };
    for _ in 0..4 {
        session(&app, &id)?;
        let mut req = client.get(target.clone());
        if let Some(ref token) = media_token {
            req = req.header("x-amz-access-token", token.clone());
        }
        if let Some(ref range) = range {
            req = req.header("range", range);
        }
        let mut response = req
            .send()
            .await
            .map_err(|_| "Encrypted media resource could not be fetched")?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|h| h.to_str().ok())
                .ok_or("Media redirect missing location")?;
            target = target
                .join(location)
                .map_err(|_| "Invalid media redirect")?;
            if !secure(&target) || target.origin() != s.manifest.origin() {
                return Err(
                    "Cross-origin media redirect blocked; stream integration needs review".into(),
                );
            }
            continue;
        }
        if !response.status().is_success() {
            return Err(format!(
                "Encrypted media request returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        let mut data = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Encrypted media transfer interrupted")?
        {
            if data.len() + chunk.len() > 32_000_000 {
                return Err("Media chunk exceeds prototype size limit".into());
            }
            data.extend_from_slice(&chunk);
        }
        session(&app, &id)?;
        return Ok(MediaResponse {
            data,
            content_type,
            uri: target.to_string(),
        });
    }
    Err("Too many media redirects".into())
}
#[tauri::command]
pub async fn api_stream_license(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    challenge: Vec<u8>,
) -> Result<Vec<u8>, String> {
    crate::local(&window)?;
    license(app, id, challenge).await
}
pub(super) async fn license(
    app: tauri::AppHandle,
    id: String,
    challenge: Vec<u8>,
) -> Result<Vec<u8>, String> {
    if challenge.is_empty() || challenge.len() > 2_000_000 {
        return Err("Invalid platform license challenge".into());
    }
    if is_service_certificate_request(&challenge) {
        return Err("Widevine requested a service certificate instead of a book license; reload to initialize the Audible certificate".into());
    }
    let s = session(&app, &id)?;
    let v = post(
        &format!("/1.0/content/{}/drmlicense", s.asin),
        license_body(&challenge, s.license_id.as_deref()),
        &s.website,
    )
    .await?;
    session(&app, &id)?;
    let raw = v["license"]
        .as_str()
        .filter(|s| s.len() < 4_000_000)
        .ok_or("Audible returned no supported platform license envelope")?;
    STANDARD
        .decode(raw)
        .map_err(|_| "Invalid encoded platform license response".into())
}
#[tauri::command]
pub fn api_stream_state(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    state: crate::PlayerState,
) -> Result<(), String> {
    crate::local(&window)?;
    let s = session(&app, &id)?;
    if state.asin != s.asin || state.sample || !state.valid() {
        return Err("Invalid standalone playback state".into());
    }
    if !selected_title(&app, &s.asin) {
        return Err("Ignoring playback state for a previously selected title".into());
    }
    crate::desktop::record(&app, &state);
    for label in ["main", "mini-player"] {
        let _ = app.emit_to(label, "player-state", state.clone());
    }
    Ok(())
}
#[tauri::command]
pub fn api_player_command(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    action: String,
    value: f64,
) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    if !value.is_finite()
        || !matches!(
            action.as_str(),
            "play" | "pause" | "seek" | "speed" | "rewind" | "recover" | "pauseChapter"
        )
    {
        return Err("Invalid standalone player command".into());
    }
    if action == "speed" && !(0.5..=3.0).contains(&value) {
        return Err("Invalid speed".into());
    }
    if app
        .state::<StreamState>()
        .0
        .lock()
        .map_err(|_| "Stream state unavailable")?
        .is_none()
    {
        return Err("Load direct playback first".into());
    }
    app.emit_to(
        "main",
        "api-player-command",
        json!({"action":action,"value":value}),
    )
    .map_err(|_| "Cannot reach standalone audio engine".into())
}
#[tauri::command]
pub fn api_stream_end(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
) -> Result<(), String> {
    crate::local(&window)?;
    end(&app, &id)
}
pub(super) fn end(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<StreamState>();
    let mut s = state.0.lock().map_err(|_| "Stream state unavailable")?;
    if s.as_ref().map(|s| s.id.as_str()) == Some(id) {
        *s = None;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grant_diagnostics_distinguish_samples_and_do_not_echo_unknown_values() {
        let full = json!({"content_license":{"license_response_type":"FullTitle","granted_right":"Ownership","playback_info":{"start_offset_ms":0,"end_offset_ms":15617000}}});
        assert_eq!(grant_kind(&full), "full title");
        assert_eq!(grant_right(&full), "ownership");
        assert_eq!(grant_offset(&full, "end_offset_ms"), Some(15617.0));
        assert_eq!(
            grant_kind(&json!({"content_license":{"license_response_type":"Preview"}})),
            "preview"
        );
        assert_eq!(
            grant_kind(&json!({"content_license":{"license_response_type":"CatalogSample"}})),
            "catalog sample"
        );
        let unknown = json!({"content_license":{"license_response_type":"private-account-value","granted_right":"private-token","playback_info":{"end_offset_ms":-1}}});
        assert_eq!(grant_kind(&unknown), "unrecognized type omitted");
        assert_eq!(grant_right(&unknown), "unrecognized right omitted");
        assert_eq!(grant_offset(&unknown, "end_offset_ms"), None);
    }
    #[test]
    fn error_codes_require_exact_known_protocol_values() {
        assert_eq!(
            server_error_code(&json!({"error":{"code":"untrusted_client"}})),
            "untrusted_client"
        );
        assert_eq!(
            server_error_code(&json!({"message":"untrusted client"})),
            "not reported"
        );
        for raw in [
            "secretOpaqueCredential",
            "user@example.test",
            "untrusted_client token=private",
        ] {
            assert_eq!(
                server_error_code(&json!({"error_code":raw})),
                "unrecognized code omitted"
            );
        }
    }
    #[test]
    fn drm_request_carries_only_its_own_grant_context() {
        let grant = json!({"content_license":{"license_id":"fake-grant-context","access_token":"never-forward"}});
        let id = grant_license_id(&grant).unwrap();
        let challenge = vec![1, 2, 3, 4, 5];
        let body = license_body(&challenge, Some(&id));
        assert_eq!(body["license_id"], "fake-grant-context");
        assert_eq!(
            STANDARD
                .decode(body["licenseChallenge"].as_str().unwrap())
                .unwrap(),
            challenge
        );
        assert_eq!(body["consumption_type"], "Streaming");
        assert!(body.get("access_token").is_none());
        assert!(license_body(&challenge, None).get("license_id").is_none());
        assert!(grant_license_id(&json!({"content_license":{"license_id":""}})).is_none());
    }
    #[test]
    fn recognize_certificate_handshake_and_decode_only_bounded_provider_certificate() {
        assert!(is_service_certificate_request(&[8, 4]));
        assert!(!is_service_certificate_request(&[8, 1]));
        assert!(!is_service_certificate_request(&[8, 4, 0]));
        let bytes = vec![42u8; 64];
        assert_eq!(
            certificate_bytes(&json!(STANDARD.encode(&bytes))).unwrap(),
            bytes
        );
        assert_eq!(
            certificate_bytes(&json!({"certificate":STANDARD.encode(&bytes)})).unwrap(),
            bytes
        );
        assert!(certificate_bytes(&json!("https://private.example.test/certificate")).is_err());
        assert!(certificate_bytes(&json!(STANDARD.encode([8, 4]))).is_err());
        assert!(certificate_bytes(&Value::Null).is_err());
    }
    #[test]
    fn license_http_failure_preserves_status_and_timing_without_disclosing_body() {
        let body=br#"{"error_code":"InvalidRequest","message":"Invalid device; user@example.test token=secret https://private.test"}"#;
        let error = parse_stream_response(403, body, "Platform license", 45).unwrap_err();
        assert_eq!(
            error,
            "Platform license HTTP 403: device registration rejected · server code: InvalidRequest (45 ms)"
        );
        let error = parse_stream_response(
            502,
            b"<html>secret upstream response</html>",
            "Platform license",
            50,
        )
        .unwrap_err();
        assert!(error.contains("HTTP 502"));
        assert!(!error.contains("secret"));
        assert!(
            parse_stream_response(200, br#"{"license":"opaque"}"#, "Platform license", 10).is_ok()
        );
    }
    #[test]
    fn classify_license_rejections_without_returning_private_server_text() {
        assert_eq!(
            rejection_reason(&json!({"message":"Verified Media Path validation failed; secret"})),
            "protected playback verification rejected (Verified Media Path)"
        );
        assert_eq!(
            rejection_reason(&json!({"error_code":"untrusted_client","message":"secret"})),
            "playback request client rejected (untrusted client; runtime cause unconfirmed)"
        );
        assert_eq!(
            rejection_reason(
                &json!({"error":{"message":"Unsupported device; secret account data"}})
            ),
            "device registration rejected"
        );
        assert_eq!(
            rejection_reason(
                &json!({"error":{"code":"UnsupportedCdmVersion","message":"secret runtime data"}})
            ),
            "protected playback runtime version rejected (UnsupportedCdmVersion)"
        );
        let value = json!({"error_code":"InvalidRequest","message":"Invalid device https://private.test/?token=secret user@example.test"});
        assert_eq!(rejection_reason(&value), "device registration rejected");
        assert_eq!(
            rejection_reason(&json!({"message":"Invalid license challenge"})),
            "platform challenge rejected"
        );
        assert_eq!(
            rejection_reason(&json!({"error_code":"Unauthorized","message":"private"})),
            "account authentication rejected"
        );
        assert_eq!(
            rejection_reason(&json!({"message":"secret opaque license payload"})),
            "server rejected request; unrecognized details omitted"
        );
    }
    #[test]
    fn widevine_manifest_is_in_license_response() {
        let v = json!({"content_license":{"drm_type":"Widevine","license_response":"https://media.example.test/manifest.mpd?private=token"}});
        assert!(manifest_location(&v).is_some());
        assert!(!secure(
            &url::Url::parse("http://media.example.test/manifest").unwrap()
        ));
        assert!(!secure(
            &url::Url::parse("https://127.0.0.1/manifest").unwrap()
        ));
        assert!(!secure(
            &url::Url::parse("https://user@media.example.test/manifest").unwrap()
        ));
    }
    #[test]
    fn voucher_is_not_a_manifest() {
        assert!(manifest_location(
            &json!({"content_license":{"drm_type":"Adrm","license_response":"opaque-voucher"}})
        )
        .is_none());
    }
}
