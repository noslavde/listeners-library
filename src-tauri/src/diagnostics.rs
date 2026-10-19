//! Deliberately lossy diagnostics. No free-form message or private data fields.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Instant,
};
use tauri_plugin_dialog::DialogExt;

const LIMIT: usize = 128;
const MAX_BYTES: u64 = 65536;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Code {
    SessionStarted,
    OpeningProtectedStorage,
    RestoringAccount,
    StartingServices,
    StartupReady,
    StartupFailed,
    LibraryShown,
    WindowRestoreFailed,
    MiniOpened,
    MiniClosed,
    ExitRequested,
    CleanExit,
    RustPanic,
    ProgressSaveFailed,
    ReactError,
    ScriptError,
    PromiseRejected,
    PreviousReportDiscarded,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Event {
    elapsed_ms: u64,
    code: Code,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    previous_unclean: bool,
    previous: Vec<Event>,
    current: Vec<Event>,
}
impl Journal {
    fn push(&mut self, code: Code, elapsed_ms: u64) -> bool {
        if matches!(
            code,
            Code::ReactError | Code::ScriptError | Code::PromiseRejected
        ) && self.current.iter().any(|e| e.code == code)
        {
            return false;
        }
        if self.current.len() == LIMIT {
            self.current.remove(0);
        }
        self.current.push(Event { elapsed_ms, code });
        true
    }
}
struct Recorder {
    path: PathBuf,
    start: Instant,
    journal: Journal,
}
static RECORDER: OnceLock<Mutex<Recorder>> = OnceLock::new();

fn load(path: &Path) -> Option<Journal> {
    crate::storage::reject_link(path).ok()?;
    let mut file = fs::File::open(path).ok()?;
    use std::io::Read;
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let journal: Journal = serde_json::from_slice(&bytes).ok()?;
    if journal.current.len() > LIMIT || journal.previous.len() > LIMIT {
        return None;
    }
    Some(journal)
}
fn next(previous: Option<Journal>) -> Journal {
    let events = previous.map(|v| v.current).unwrap_or_default();
    Journal {
        previous_unclean: !events.is_empty()
            && events.last().is_none_or(|e| e.code != Code::CleanExit),
        previous: events,
        current: Vec::new(),
    }
}
pub fn initialize(root: &Path) -> Result<(), String> {
    let dir = root.join(format!("diagnostics-{}", crate::storage::SCOPE));
    crate::storage::reject_link(&dir)?;
    fs::create_dir_all(&dir).map_err(|_| "Cannot create diagnostics directory")?;
    crate::protection::restrict_directory(&dir)?;
    let path = dir.join("events.json");
    let old = load(&path);
    let discarded = path.exists() && old.is_none();
    RECORDER
        .set(Mutex::new(Recorder {
            path,
            start: Instant::now(),
            journal: next(old),
        }))
        .map_err(|_| "Diagnostics already initialized")?;
    record(Code::SessionStarted);
    if discarded {
        record(Code::PreviousReportDiscarded);
    }
    Ok(())
}
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|_| {
        // Panic payloads and source paths can contain secrets. Never copy them.
        record(Code::RustPanic);
    }));
}
pub fn record(code: Code) {
    let Some(recorder) = RECORDER.get() else {
        return;
    };
    // A panic while holding the recorder must not deadlock the panic hook.
    let Ok(mut state) = recorder.try_lock() else {
        return;
    };
    let elapsed_ms = (state.start.elapsed().as_millis() as u64 / 100) * 100;
    if !state.journal.push(code, elapsed_ms) {
        return;
    }
    if let Ok(bytes) = serde_json::to_vec(&state.journal) {
        let _ = crate::notes::write_export(&state.path, &bytes);
    }
}
fn report(journal: &Journal) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "format": "listeners-library-diagnostics", "schema": 1,
        "app_version": env!("CARGO_PKG_VERSION"), "platform": "windows",
        "architecture": std::env::consts::ARCH, "development": cfg!(debug_assertions),
        "previous_session_ended_uncleanly": journal.previous_unclean,
        "previous_session": journal.previous, "current_session": journal.current,
        "limitations": "Unclean shutdown is not proof of a crash. Native process crashes, power loss and forced termination may leave no error event. No memory dumps or raw errors are collected."
    })).map_err(|_| "Cannot prepare diagnostic report".into())
}
#[tauri::command]
pub fn diagnostic_report(window: tauri::WebviewWindow) -> Result<String, String> {
    crate::desktop::player_local(&window)?;
    let state = RECORDER
        .get()
        .ok_or("Diagnostics unavailable")?
        .lock()
        .map_err(|_| "Diagnostics unavailable")?;
    report(&state.journal)
}
#[tauri::command]
pub fn record_ui_error(window: tauri::WebviewWindow, code: Code) -> Result<(), String> {
    crate::desktop::player_local(&window)?;
    if !matches!(
        code,
        Code::ReactError | Code::ScriptError | Code::PromiseRejected
    ) {
        return Err("Unsupported diagnostic event".into());
    }
    record(code);
    Ok(())
}
#[tauri::command]
pub async fn export_diagnostics(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    preview: String,
) -> Result<bool, String> {
    crate::desktop::player_local(&window)?;
    // Export precisely the reviewed snapshot, never arbitrary caller content.
    if preview.len() as u64 > MAX_BYTES {
        return Err("Invalid report".into());
    }
    let current = diagnostic_report(window.clone())?;
    if current != preview {
        return Err("The report changed. Review it again before saving.".into());
    }
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Save reviewed diagnostic report")
            .add_filter("Diagnostic JSON", &["json"])
            .set_file_name("listeners-library-diagnostics.json")
            .blocking_save_file()
    })
    .await
    .map_err(|_| "Cannot open save dialog")?;
    let Some(file) = file else {
        return Ok(false);
    };
    let path = file.into_path().map_err(|_| "Choose a local JSON file")?;
    if path
        .extension()
        .and_then(|v| v.to_str())
        .is_none_or(|v| !v.eq_ignore_ascii_case("json"))
    {
        return Err("Choose a .json file".into());
    }
    crate::notes::write_export(&path, preview.as_bytes())?;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_allowlisted_data_survives() {
        assert!(serde_json::from_str::<Journal>(
            r#"{"previous_unclean":false,"previous":[],"current":[],"token":"secret"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Event>(
            r#"{"elapsed_ms":0,"code":"react-error","message":"secret"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Event>(
            r#"{"elapsed_ms":0,"code":"https://signed-in-url"}"#
        )
        .is_err());
        let text = report(&next(None)).unwrap();
        for forbidden in [
            "username",
            "hostname",
            "account_id",
            "access_token",
            "asin",
            "C:\\Users",
        ] {
            assert!(!text.contains(forbidden));
        }
    }
    #[test]
    fn classifies_unclean_termination_without_claiming_crash() {
        let mut j = Journal::default();
        j.current.push(Event {
            elapsed_ms: 0,
            code: Code::SessionStarted,
        });
        assert!(next(Some(j)).previous_unclean);
        let mut j = Journal::default();
        j.current.push(Event {
            elapsed_ms: 0,
            code: Code::CleanExit,
        });
        assert!(!next(Some(j)).previous_unclean);
        assert!(!next(None).previous_unclean);
    }
    #[test]
    fn bounds_events_and_coalesces_repeated_frontend_errors() {
        let mut j = Journal::default();
        for i in 0..1000 {
            j.push(Code::LibraryShown, i);
        }
        assert_eq!(j.current.len(), LIMIT);
        j.push(Code::ScriptError, 1000);
        j.push(Code::ScriptError, 1001);
        assert_eq!(j.current.len(), LIMIT);
        assert_eq!(
            j.current
                .iter()
                .filter(|e| e.code == Code::ScriptError)
                .count(),
            1
        );
        j.push(Code::CleanExit, 1002);
        assert!(!next(Some(j)).previous_unclean);
    }
    #[test]
    fn rejects_oversized_and_malformed_saved_logs() {
        let dir =
            std::env::temp_dir().join(format!("ll-diagnostic-test-{:x}", rand::random::<u64>()));
        fs::create_dir(&dir).unwrap();
        let file = dir.join("events.json");
        fs::write(&file, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(load(&file).is_none());
        fs::write(&file, b"not json").unwrap();
        assert!(load(&file).is_none());
        fs::remove_file(file).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
