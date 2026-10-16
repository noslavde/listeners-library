//! Explicit recovery of the first installer's locally saved account. Credentials
//! stay native and are re-sealed before the current app reports a connection.
use super::*;

fn decode_previous(raw: &str) -> Result<Auth, String> {
    if raw.len() > 1_048_576 {
        return Err("The previous account record is too large.".into());
    }
    let auth: Auth = serde_json::from_str(raw).map_err(|_| {
        "The previous account record could not be read. It has been kept unchanged."
    })?;
    for token in [&auth.access, &auth.refresh] {
        if token.is_empty() || token.len() >= 60_000 {
            return Err(
                "The previous account record is incomplete. It has been kept unchanged.".into(),
            );
        }
    }
    Ok(auth)
}

#[tauri::command]
pub async fn api_restore_previous_account(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    super::super::local(&window)?;
    if cfg!(debug_assertions) {
        return Err("Previous-account recovery is available in the installed app only. Development accounts remain separate.".into());
    }
    if app.get_webview_window("metadata-login").is_some() {
        return Err("Finish or close the sign-in window before restoring an account.".into());
    }
    let state = app.state::<MetadataState>();
    if state.auth.lock().await.is_some() {
        return Err(
            "An account is already connected. Sign out before restoring another account.".into(),
        );
    }
    let generation;
    {
        let pending = state
            .pending
            .lock()
            .map_err(|_| "Login state unavailable")?;
        let mut status = state.login.lock().map_err(|_| "Login state unavailable")?;
        if pending.is_some() || matches!(status.phase, "restoring" | "registering" | "saving") {
            return Err("Finish the current account connection first.".into());
        }
        generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
        status.phase = "restoring";
    }
    login_report(
        &app,
        "restoring",
        "Restoring the first build’s saved account on this PC…",
    );
    let outcome = async {
        let entry = keyring::Entry::new("AudibleDesktop.Metadata.US", "default")
            .map_err(|_| "Windows credential storage is unavailable.")?;
        let raw = zeroize::Zeroizing::new(entry.get_password().map_err(|error| match error {
            keyring::Error::NoEntry => "No account saved by the first build was found for this Windows user. Try Sign in instead.",
            _ => "Windows could not unlock the previous account. The saved record has been kept unchanged.",
        })?);
        let auth = decode_previous(&raw)?;
        drop(raw);
        // No network exchange or credential import into the UI. Expired access
        // tokens follow the original refresh flow when Sync library is chosen.
        state.finish_registration(generation, auth, |auth| save_verified(&app, auth)).await
    }.await;
    if state.generation.load(Ordering::SeqCst) != generation {
        return Err("Account restore canceled.".into());
    }
    match &outcome {
        Ok(()) => login_report(
            &app,
            "connected",
            "Previous account restored and encrypted.",
        ),
        Err(error) => login_report(&app, "failed", error.clone()),
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_old_accounts_without_changing_their_profile_or_refresh_token() {
        let old = decode_previous(
            r#"{"access":"synthetic-access","refresh":"synthetic-refresh","expires":1}"#,
        )
        .unwrap();
        assert_eq!(old.profile, Profile::Ios);
        assert_eq!(old.expires, 1); // expired access is refreshable; do not reject it
        let android = decode_previous(r#"{"profile":"android","access":"synthetic-access","refresh":"synthetic-refresh","expires":1}"#).unwrap();
        assert_eq!(android.profile, Profile::Android);
        assert_eq!(android.refresh, "synthetic-refresh");
    }
    #[test]
    fn rejects_invalid_previous_records_without_echoing_them() {
        for raw in [
            "private-invalid-json",
            r#"{"access":"private","expires":1}"#,
            r#"{"access":"private","refresh":"","expires":1}"#,
        ] {
            let error = decode_previous(raw).err().unwrap();
            assert!(!error.contains("private"));
        }
    }
    #[test]
    fn previous_account_is_resealed_then_restored_without_plaintext_or_ui_tokens() {
        let root =
            std::env::temp_dir().join(format!("audible-restore-test-{}", rand::random::<u64>()));
        let store = crate::storage::Storage::initialize(&root, false).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let auth = decode_previous(r#"{"profile":"android","access":"synthetic-old-access","refresh":"synthetic-old-refresh","expires":1}"#).unwrap();
            let state = MetadataState::with_auth(None);
            state.finish_registration(0, auth, |auth| {
                let raw = zeroize::Zeroizing::new(serde_json::to_vec(auth).unwrap());
                store.save_account_bytes(&raw)?;
                let reopened = store.load_account_bytes()?.unwrap();
                let decoded = decode_previous(std::str::from_utf8(&reopened).unwrap())?;
                assert_eq!(decoded.profile, Profile::Android);
                assert_eq!(decoded.refresh, auth.refresh);
                Ok(())
            }).await.unwrap();
            let public = serde_json::to_string(&state.connection().await.unwrap()).unwrap();
            assert!(!public.contains("synthetic-old"));
            let bytes = std::fs::read(store.dir.join("account.sealed")).unwrap();
            assert!(!bytes.windows(b"synthetic-old".len()).any(|w| w == b"synthetic-old"));
        });
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
