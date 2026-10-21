//! Native preparation of the separate Audible website session from account
//! context. Cookies remain native/browser-side; website verification may still
//! be required even when library API authentication succeeds.

//! Account-owned website session exchange. Cookies stay in native memory and
//! the dedicated private US Amazon/Audible WebView; never in frontend IPC.
use super::*;
#[derive(Clone)]
pub(super) struct WebsiteSession {
    cookie: reqwest::header::HeaderValue,
}
impl WebsiteSession {
    pub(super) async fn connect(app: &tauri::AppHandle) -> Result<Self, String> {
        let (_, value) = exchange(app).await?.ok_or("Connect library API first")?;
        Ok(Self {
            cookie: cookie_header(&value)?,
        })
    }
    pub(super) fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::COOKIE, self.cookie.clone());
        headers.insert(
            "client-id",
            reqwest::header::HeaderValue::from_static("WebPlayerApplication"),
        );
        headers.insert(
            "accept",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "cache-control",
            reqwest::header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            "origin",
            reqwest::header::HeaderValue::from_static("https://www.audible.com"),
        );
        headers
    }
}
async fn exchange(app: &tauri::AppHandle) -> Result<Option<(u64, Value)>, String> {
    let state = app.state::<MetadataState>();
    if state
        .pending
        .lock()
        .map_err(|_| "Login state unavailable")?
        .is_some()
    {
        return Err("Finish account sign-in before opening playback".into());
    }
    let guard = state.auth.lock().await;
    let Some(auth) = guard.as_ref() else {
        return Ok(None);
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let response = http()?
        .post("https://www.amazon.com/ap/exchangetoken/cookies")
        .form(&exchange_body(&auth.refresh))
        .send()
        .await
        .map_err(|_| "Website session exchange network failure")?;
    let value = decode(response)
        .await
        .map_err(|e| format!("Website session exchange failed: {e}"))?;
    if state.generation.load(Ordering::SeqCst) != generation {
        return Err("Account changed during automatic playback sign-in; try again".into());
    }
    Ok(Some((generation, value)))
}
fn exchange_body(refresh: &str) -> [(&'static str, &str); 6] {
    [
        ("app_name", "Audible"),
        ("app_version", "3.56.2"),
        ("source_token", refresh),
        ("source_token_type", "refresh_token"),
        ("requested_token_type", "auth_cookies"),
        ("domain", ".amazon.com"),
    ]
}
fn cookie_header(value: &Value) -> Result<reqwest::header::HeaderValue, String> {
    let domains = value
        .pointer("/response/tokens/cookies")
        .and_then(Value::as_object)
        .ok_or("Website session exchange returned no cookies")?;
    let mut cookies = std::collections::BTreeMap::new();
    // Amazon/Audible US session cookies use the marketplace's SSO. No other
    // domains are accepted, and this header is never used for API/CDN traffic.
    for domain in [".amazon.com", "amazon.com", ".audible.com", "audible.com"] {
        if let Some(entries) = domains.get(domain).and_then(Value::as_array) {
            for entry in entries {
                let Some(name) = entry["Name"].as_str() else {
                    continue;
                };
                let Some(raw) = entry["Value"].as_str() else {
                    continue;
                };
                let value = raw
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(raw);
                if name.is_empty()
                    || name.len() > 128
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
                    || value.is_empty()
                    || value.len() > 12_000
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_graphic() && !b";\"\\,".contains(&b))
                {
                    return Err("Website session contained an unsupported cookie format".into());
                }
                cookies.insert(name.to_owned(), value.to_owned());
            }
        }
    }
    if cookies.is_empty() || cookies.len() > 64 {
        return Err("Website session exchange returned no supported US session cookies".into());
    }
    let joined = cookies
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");
    if joined.len() > 24_000 {
        return Err("Website session exceeds size limit".into());
    }
    let mut header = reqwest::header::HeaderValue::from_str(&joined)
        .map_err(|_| "Cannot prepare protected website session")?;
    header.set_sensitive(true);
    Ok(header)
}
pub(super) fn endpoint(path: &str) -> Result<String, String> {
    let suffix = path
        .strip_prefix("/1.0/content/")
        .ok_or("Website session endpoint not allowed")?;
    let (asin, operation) = suffix
        .split_once('/')
        .ok_or("Website session endpoint not allowed")?;
    if !crate::valid_asin(asin) || !matches!(operation, "licenserequest" | "drmlicense") {
        return Err("Website session endpoint not allowed".into());
    }
    Ok(format!("https://www.audible.com/audible-api{path}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exchange_and_cookie_scope_never_expose_tokens_or_foreign_cookies() {
        assert!(exchange_body("fake-refresh").contains(&("requested_token_type", "auth_cookies")));
        let value = json!({"response":{"tokens":{"cookies":{".amazon.com":[{"Name":"session-id","Value":"\"fake-session\""}],".evil.test":[{"Name":"foreign","Value":"never-send"}]}}}});
        let header = cookie_header(&value).unwrap();
        assert_eq!(header, "session-id=fake-session");
        assert!(header.is_sensitive());
        assert!(!format!("{header:?}").contains("fake-session"));
        let headers = WebsiteSession { cookie: header }.headers();
        assert!(!headers.contains_key("authorization"));
        assert!(!headers.contains_key("x-amz-access-token"));
        assert_eq!(headers.get("client-id").unwrap(), "WebPlayerApplication");
        assert!(headers.get("cookie").unwrap().is_sensitive());
    }
    #[test]
    fn reject_cookie_injection_and_non_playback_destinations() {
        let value = json!({"response":{"tokens":{"cookies":{".amazon.com":[{"Name":"session-id","Value":"fake; injected=value"}]}}}});
        assert!(cookie_header(&value).is_err());
        for path in [
            "https://evil.test/",
            "/1.0/library",
            "/1.0/content/../drmlicense",
            "/1.0/content/B012345678/drmlicense?redirect=evil",
        ] {
            assert!(endpoint(path).is_err());
        }
        assert_eq!(
            endpoint("/1.0/content/B012345678/drmlicense").unwrap(),
            "https://www.audible.com/audible-api/1.0/content/B012345678/drmlicense"
        );
    }
}
