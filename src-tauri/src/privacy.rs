use serde::Serialize;
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};
pub struct Privacy(AtomicBool);
pub fn remember(app: &tauri::AppHandle) -> bool {
    app.try_state::<Privacy>()
        .map(|p| p.0.load(Ordering::SeqCst))
        .unwrap_or(false)
}
pub fn setup(app: &mut tauri::App) -> Result<(), String> {
    use rusqlite::OptionalExtension;
    let db = app.state::<crate::storage::Storage>().open()?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS preferences(key TEXT PRIMARY KEY,value TEXT NOT NULL);").map_err(|_| "Cannot initialize privacy preference")?;
    let value: Option<String> = db
        .query_row(
            "SELECT value FROM preferences WHERE key='remember_website'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| "Cannot read privacy preference")?;
    app.manage(Privacy(AtomicBool::new(value.as_deref() != Some("false"))));
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    encrypted_library: bool,
    remember_website: bool,
    legacy_data: bool,
    development: bool,
}
#[tauri::command]
pub fn privacy_status(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Status, String> {
    super::local(&window)?;
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Data directory unavailable")?;
    Ok(Status {
        encrypted_library: true,
        remember_website: remember(&app),
        legacy_data: !cfg!(debug_assertions)
            && (root.join("account-web-session").exists() || root.join("library.sqlite").exists()),
        development: cfg!(debug_assertions),
    })
}
#[tauri::command]
pub async fn set_website_privacy(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    remember_website: bool,
) -> Result<(), String> {
    super::local(&window)?;
    if remember(&app) == remember_website {
        return Ok(());
    }
    if let Some(w) = app.get_webview_window("metadata-login") {
        w.close()
            .map_err(|_| "Close the sign-in window and retry")?;
    }
    super::clear_official_session(&app)?;
    app.state::<crate::storage::Storage>().open()?
        .execute(
            "INSERT OR REPLACE INTO preferences(key,value) VALUES('remember_website',?1)",
            [if remember_website { "true" } else { "false" }],
        )
        .map_err(|_| "Cannot save privacy preference")?;
    app.state::<Privacy>()
        .0
        .store(remember_website, Ordering::SeqCst);
    let _ = app.emit_to("main", "privacy-changed", ());
    Ok(())
}
fn remove_tree(path: &Path) -> Result<(), String> {
    crate::storage::reject_link(path)?;
    if !path.exists() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "Cannot inspect local data")?;
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|_| "Cannot read local data")? {
            remove_tree(&entry.map_err(|_| "Cannot inspect local data")?.path())?;
        }
        fs::remove_dir(path)
            .map_err(|_| "Local data is still in use. Close other app versions and restart")?;
    } else {
        fs::remove_file(path)
            .map_err(|_| "Local data is still in use. Close other app versions and restart")?;
    }
    Ok(())
}
#[cfg(not(test))]
pub(crate) fn delete_account(service: &str) -> Result<(), String> {
    match keyring::Entry::new(service, "default")
        .map_err(|_| "Credential storage unavailable")?
        .delete_credential()
    {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("Cannot remove saved account credential".into()),
    }
}
pub fn apply_pending_reset(root: &Path) -> Result<(), String> {
    crate::storage::reject_link(root)?;
    let marker = root.join(format!("{}.reset", crate::storage::SCOPE));
    crate::storage::reject_link(&marker)?;
    if !marker.exists() {
        return Ok(());
    }
    let mode = fs::read_to_string(&marker).map_err(|_| "Cannot read data-removal request")?;
    if !matches!(mode.as_str(), "all" | "legacy") {
        return Err("Invalid data-removal request; nothing was removed".into());
    }
    // Take the same profile lock used by storage initialization before any removal.
    let _guard = crate::storage::profile_lock(&root.join(format!("{}.lock", crate::storage::SCOPE)))
        .map_err(|_| "Close the other copy of the app before removing local data")?;
    // The tombstone prevents a reset from importing an older plaintext database.
    fs::write(
        root.join(format!("{}.no-legacy-import", crate::storage::SCOPE)),
        b"1",
    )
    .map_err(|_| "Cannot save data-removal state")?;
    if mode == "all" {
        retry_remove(&root.join(crate::storage::SCOPE))?;
        #[cfg(target_os = "macos")]
        crate::protection::forget_wrapping_key()?;
    }
    if !cfg!(debug_assertions) {
        #[cfg(not(test))]
        delete_account("AudibleDesktop.Metadata.US")?;
        for name in [
            "account-web-session",
            "library.sqlite",
            "library.sqlite-wal",
            "library.sqlite-shm",
            "library.sqlite-journal",
        ] {
            retry_remove(&root.join(name))?;
        }
    }
    fs::remove_file(marker).map_err(|_| "Cannot finish data removal")?;
    Ok(())
}
fn retry_remove(path: &Path) -> Result<(), String> {
    let mut result = Ok(());
    for _ in 0..20 {
        result = remove_tree(path);
        if result.is_ok() {
            return result;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    result
}
#[tauri::command]
pub async fn erase_local_data(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    confirmation: String,
    legacy_only: bool,
) -> Result<(), String> {
    super::local(&window)?;
    if confirmation
        != if legacy_only {
            "REMOVE OLD DATA"
        } else {
            "ERASE"
        }
    {
        return Err("Typed confirmation required".into());
    }
    if legacy_only && cfg!(debug_assertions) {
        return Err("Legacy cleanup is available in the installed app".into());
    }
    // WKWebView's named store is outside the SQLCipher directory. Clear it and
    // wait for WebKit's completion before writing the destructive-reset marker.
    #[cfg(target_os = "macos")]
    {
        if let Some(w) = app.get_webview_window("metadata-login") {
            w.destroy().map_err(|_| "Cannot close sign-in before data removal")?;
        }
        super::clear_official_session(&app)?;
    }
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Data directory unavailable")?;
    // No secrets go into this restart request. Removal happens before any database
    // or account profile opens in the new process; failures stop startup.
    fs::write(
        root.join(format!("{}.reset", crate::storage::SCOPE)),
        if legacy_only {
            b"legacy".as_slice()
        } else {
            b"all".as_slice()
        },
    )
    .map_err(|_| "Cannot request local data removal")?;
    app.restart();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_removal_leaves_sibling_files() {
        let root =
            std::env::temp_dir().join(format!("audible-reset-test-{}", rand::random::<u64>()));
        fs::create_dir_all(root.join("owned/child")).unwrap();
        fs::write(root.join("owned/child/test"), b"dummy").unwrap();
        fs::write(root.join("sibling"), b"keep").unwrap();
        remove_tree(&root.join("owned")).unwrap();
        assert_eq!(fs::read(root.join("sibling")).unwrap(), b"keep");
        fs::remove_dir_all(root).unwrap();
    }
}

/// Disable browser-managed password saving and form autofill in provider views.
#[cfg(windows)]
pub fn harden_view(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows::core::Interface;
    let owner = window.clone();
    window.with_webview(move |view|{
  let outcome=(||->windows::core::Result<()>{unsafe{
   let settings=view.controller().CoreWebView2()?.Settings()?.cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings4>()?;
   settings.SetIsPasswordAutosaveEnabled(false)?;
   settings.SetIsGeneralAutofillEnabled(false)?;
  }Ok(())})();
  if outcome.is_err(){let _=owner.app_handle().emit_to("main","player-error","Cannot apply website privacy settings; update Microsoft WebView2 and retry.");let _=owner.close();}
 }).map_err(|_|"Cannot apply website privacy settings".into())
}

/// Wait for the actual platform completion, not merely submission of deletion.
/// Called on async command workers, never in the Windows UI event handler.
#[cfg(windows)]
pub fn clear_browser_data(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows::core::Interface;
    use webview2_com::{ClearBrowsingDataCompletedHandler, Microsoft::Web::WebView2::Win32::{ICoreWebView2_13, ICoreWebView2Profile2}};
    let (tx, rx) = std::sync::mpsc::channel();
    window.with_webview(move |view| {
        let failed = tx.clone();
        let submitted = (|| -> windows::core::Result<()> { unsafe {
            view.controller().CoreWebView2()?.cast::<ICoreWebView2_13>()?.Profile()?.cast::<ICoreWebView2Profile2>()?
                .ClearBrowsingDataAll(&ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
                    let _ = tx.send(result.is_ok()); Ok(())
                })))?;
        } Ok(()) })();
        if submitted.is_err() { let _ = failed.send(false); }
    }).map_err(|_| "Cannot start website data cleanup")?;
    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(true) => Ok(()),
        _ => Err("Website data cleanup did not complete; retry sign-out before signing in again".into()),
    }
}

#[cfg(target_os = "macos")]
pub fn harden_view(_: &tauri::WebviewWindow) -> Result<(), String> {
    // WKWebView has no public switch matching WebView2 password autosave.
    // No private WebKit preference is used; disclose OS-managed autofill.
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn clear_browser_data(window: &tauri::WebviewWindow) -> Result<(), String> {
    use objc2::MainThreadMarker;
    use objc2_foundation::NSDate;
    use objc2_web_kit::{WKWebView, WKWebsiteDataStore};
    let (tx, rx) = std::sync::mpsc::channel();
    window.with_webview(move |view| {
        // The borrowed WKWebView stays on Tauri's UI thread. WebKit retains
        // the completion block until removal finishes; only the sender escapes.
        let Some(mtm) = MainThreadMarker::new() else { return; };
        unsafe {
            let Some(webview) = (view.inner() as *const WKWebView).as_ref() else { return; };
            let store = webview.configuration().websiteDataStore();
            let types = WKWebsiteDataStore::allWebsiteDataTypes(mtm);
            let date = NSDate::dateWithTimeIntervalSince1970(0.0);
            let done = block2::RcBlock::new(move || { let _ = tx.send(()); });
            store.removeDataOfTypes_modifiedSince_completionHandler(&types, &date, &done);
        }
    }).map_err(|_| "Cannot start website data cleanup")?;
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|_| "Website cleanup did not complete; retry sign-out before signing in again".into())
}

/// All provider windows must share the same platform-owned session store.
pub fn provider_view<'a>(
    app: &tauri::AppHandle,
    builder: tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle>,
) -> Result<tauri::WebviewWindowBuilder<'a, tauri::Wry, tauri::AppHandle>, String> {
    let builder = builder.incognito(!remember(app));
    #[cfg(windows)]
    let builder = builder.data_directory(crate::account_web_profile(app)?);
    #[cfg(target_os = "macos")]
    let builder = builder.data_store_identifier(if cfg!(debug_assertions) {
        *b"LmacDevSession01"
    } else {
        *b"LmacExpSession01"
    });
    Ok(builder)
}
