//! Explicit one-shot last-position test. No automatic account writes.
use super::*;
pub(super) struct PreparedUpload {
    ticket: String,
    asin: String,
    acr: String,
    position: f64,
    duration: f64,
    cloud: f64,
    created: u64,
    generation: u64,
}
#[derive(Serialize)]
pub struct UploadPreview {
    ticket: String,
    position: f64,
    cloud_position: f64,
    expires_in: u64,
}
#[derive(Serialize)]
pub struct UploadReceipt {
    pub position: f64,
    pub observed: Option<f64>,
    pub verified: bool,
    pub message: String,
}
fn millis(position: f64) -> Result<u64, String> {
    if !position.is_finite() || !(0.0..31_536_000.0).contains(&position) {
        return Err("Invalid playback position".into());
    }
    Ok((position * 1000.0).round() as u64)
}
fn upload_body(asin: &str, acr: &str, position: f64) -> Result<Value, String> {
    if !crate::valid_asin(asin) || acr.is_empty() || acr.len() > 4096 {
        return Err("Missing or invalid content reference".into());
    }
    Ok(json!({"asin":asin,"acr":acr,"position_ms":millis(position)?}))
}
fn reference(v: &Value) -> Option<String> {
    v["content_metadata"]["content_reference"]["acr"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .map(str::to_owned)
}
fn unchanged(previous: f64, current: f64) -> bool {
    previous.is_finite() && current.is_finite() && (previous - current).abs() <= 0.001
}
fn verified(requested: f64, observed: Option<f64>) -> bool {
    observed
        .map(|p| p.is_finite() && (p - requested).abs() <= 2.0)
        .unwrap_or(false)
}
fn capture(app: &tauri::AppHandle, asin: &str) -> Result<(f64, f64), String> {
    if app.get_webview_window("official").is_none() && !super::stream::active(app) {
        return Err("Load direct playback for this title first".into());
    }
    let hub = app.state::<crate::desktop::PlayerHub>();
    let state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    let book = state.book.as_ref().ok_or("Choose a title first")?;
    if book["asin"].as_str() != Some(asin) {
        return Err("The selected title changed; prepare again".into());
    }
    let s = state
        .snapshot
        .as_ref()
        .ok_or("Wait for playback controls")?;
    if s.sample {
        return Err("Sample playback cannot be uploaded as book progress".into());
    }
    if !s.available || !s.paused || now().saturating_sub(state.last_seen.unwrap_or(0)) > 3 {
        return Err("Pause playback and wait for a fresh player report before uploading".into());
    }
    let runtime = book["duration"].as_f64().unwrap_or(0.0);
    if runtime <= 0.0
        || s.duration <= 0.0
        || (runtime - s.duration).abs() > 120.0_f64.max(runtime * 0.01)
    {
        return Err("The reported player timeline does not match the book runtime; upload disabled until whole-book time is verified".into());
    }
    if s.position > s.duration {
        return Err("Position outside book timeline".into());
    }
    millis(s.position)?;
    Ok((s.position, s.duration))
}
async fn metadata(app: &tauri::AppHandle, asin: &str) -> Result<Value, String> {
    get(
        app,
        &format!("/1.0/content/{asin}/metadata"),
        &[
            ("drm_type", "Adrm"),
            ("quality", "High"),
            ("chapter_titles_type", "Flat"),
            (
                "response_groups",
                "last_position_heard,content_reference,chapter_info",
            ),
        ],
    )
    .await
}
#[tauri::command]
pub fn api_cancel_upload(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    crate::local(&window)?;
    *app.state::<MetadataState>()
        .upload
        .lock()
        .map_err(|_| "Upload state unavailable")? = None;
    Ok(())
}
#[tauri::command]
pub async fn api_prepare_upload(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    asin: String,
) -> Result<UploadPreview, String> {
    crate::local(&window)?;
    if !crate::valid_asin(&asin) {
        return Err("Invalid ASIN".into());
    }
    let (position, duration) = capture(&app, &asin)?;
    let state = app.state::<MetadataState>();
    let generation = state.generation.load(Ordering::SeqCst);
    let v = metadata(&app, &asin).await?;
    let cloud = cloud_metadata(&v)
        .position
        .ok_or("No supported cloud position was returned; upload disabled")?;
    if cloud > position + 2.0 {
        return Err("Cloud position is ahead of this player; resume the cloud position before preparing an upload".into());
    }
    let acr = reference(&v).ok_or(
        "Metadata did not provide a content reference; no license request or upload was made",
    )?;
    if state.generation.load(Ordering::SeqCst) != generation {
        return Err("Account connection changed; prepare again".into());
    }
    let ticket = URL_SAFE_NO_PAD.encode(random(24));
    *state
        .upload
        .lock()
        .map_err(|_| "Upload state unavailable")? = Some(PreparedUpload {
        ticket: ticket.clone(),
        asin,
        acr,
        position,
        duration,
        cloud,
        created: now(),
        generation,
    });
    Ok(UploadPreview {
        ticket,
        position,
        cloud_position: cloud,
        expires_in: 60,
    })
}
#[tauri::command]
pub async fn api_commit_upload(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    ticket: String,
) -> Result<UploadReceipt, String> {
    crate::local(&window)?;
    let state = app.state::<MetadataState>();
    let p = {
        let mut pending = state
            .upload
            .lock()
            .map_err(|_| "Upload state unavailable")?;
        if pending.as_ref().map(|p| p.ticket.as_str()) != Some(ticket.as_str()) {
            return Err("Prepare an upload preview first".into());
        }
        pending.take().ok_or("No upload preview")?
    };
    if now().saturating_sub(p.created) > 60
        || state.generation.load(Ordering::SeqCst) != p.generation
    {
        return Err("Upload preview expired or account changed; prepare again".into());
    }
    let (current, duration) = capture(&app, &p.asin)?;
    if (current - p.position).abs() > 0.25 || (duration - p.duration).abs() > 0.25 {
        return Err("Playback position changed; prepare again".into());
    }
    let v = metadata(&app, &p.asin).await?;
    let cloud = cloud_metadata(&v)
        .position
        .ok_or("Cloud position is unavailable; no upload made")?;
    if !unchanged(p.cloud, cloud) || reference(&v).as_deref() != Some(p.acr.as_str()) {
        return Err("Cloud progress or content version changed since preview; no upload made. Check cloud position and prepare again".into());
    }
    let body = upload_body(&p.asin, &p.acr, p.position)?;
    {
        let mut auth = state.auth.lock().await;
        let a = auth.as_mut().ok_or("Connect library API first")?;
        if state.generation.load(Ordering::SeqCst) != p.generation {
            return Err("Account changed; no upload made".into());
        }
        let client = http()?;
        let token = fresh_token(&app, a, &client).await?;
        if now().saturating_sub(p.created) > 60 {
            return Err("Upload preview expired during preflight; no upload made".into());
        }
        let (current, _) = capture(&app, &p.asin)?;
        if (current - p.position).abs() > 0.25 {
            return Err("Playback changed; no upload made".into());
        }
        let response=client.put(format!("https://api.audible.com/1.0/lastpositions/{}",p.asin)).header("x-amz-access-token",token).json(&body).send().await.map_err(|_|"Upload result is unknown after a network failure; check cloud position before retrying")?;
        if !response.status().is_success() {
            return Err(format!(
                "Position upload returned HTTP {}; automatic retries are disabled",
                response.status().as_u16()
            ));
        }
    }
    let readback = metadata(&app, &p.asin)
        .await
        .ok()
        .and_then(|v| cloud_metadata(&v).position);
    let confirmed = verified(p.position, readback);
    Ok(UploadReceipt {
        position: p.position,
        observed: readback,
        verified: confirmed,
        message: if confirmed {
            "Position read-back matches · now verify the same title on mobile and web".into()
        } else {
            "Audible accepted the request, but read-back did not confirm the position · do not assume mobile sync succeeded".into()
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_payload_uses_milliseconds() {
        let body = upload_body("B012345678", "CR!synthetic", 123.456).unwrap();
        assert_eq!(body["position_ms"], 123456);
        assert_eq!(body["asin"], "B012345678");
        assert_eq!(body["acr"], "CR!synthetic");
        assert!(upload_body("../bad", "CR!synthetic", 0.0).is_err());
        assert!(upload_body("B012345678", "", 0.0).is_err());
        assert!(millis(f64::NAN).is_err());
        assert!(millis(-1.0).is_err());
    }
    #[test]
    fn readback_is_required() {
        assert!(verified(123.0, Some(124.0)));
        assert!(!verified(123.0, None));
        assert!(!verified(123.0, Some(0.0)));
    }
    #[test]
    fn any_cloud_change_requires_new_preview() {
        assert!(unchanged(120.0, 120.0));
        assert!(!unchanged(120.0, 121.0));
        assert!(!unchanged(120.0, 100.0));
    }
    #[test]
    fn reference_is_metadata_only() {
        assert_eq!(
            reference(&json!({"content_metadata":{"content_reference":{"acr":"CR!synthetic"}}})),
            Some("CR!synthetic".into())
        );
        assert!(reference(&json!({"license_response":"not-used"})).is_none());
    }
}
