//! Windows single-instance activation, hidden-window recovery and orderly
//! shutdown. Closing the main app must end playback/process ownership rather
//! than leave an inaccessible background instance.

//! No tray mode: closing the library exits; launching again restores it.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject},
    },
};

static CLOSING: AtomicBool = AtomicBool::new(false);
pub fn closing() -> bool {
    CLOSING.load(Ordering::SeqCst)
}

struct Handle(HANDLE);
unsafe impl Send for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
pub struct Instance {
    _mutex: Handle,
    event: Handle,
}
fn name(kind: &str) -> Vec<u16> {
    use sha2::{Digest, Sha256};
    // Separate users and development/installed builds. Never exported or logged.
    let user = std::env::var_os("LOCALAPPDATA").unwrap_or_default();
    let hash = Sha256::digest(user.to_string_lossy().as_bytes());
    format!(
        "Local\\ListenersLibrary.{}.{:x}.{kind}",
        crate::storage::SCOPE,
        hash
    )
    .encode_utf16()
    .chain(Some(0))
    .collect()
}
impl Instance {
    pub fn acquire() -> Result<Option<Self>, &'static str> {
        Self::acquire_named(&name("instance"), &name("activate"))
    }
    fn acquire_named(mutex_name: &[u16], event_name: &[u16]) -> Result<Option<Self>, &'static str> {
        for attempt in 0..=5 {
            unsafe {
                let mutex = Handle(
                    CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr()))
                        .map_err(|_| "Cannot check the running app")?,
                );
                let existing = GetLastError() == ERROR_ALREADY_EXISTS;
                // Creating/opening the event in either process retains an early
                // activation signal until setup has installed its listener.
                let event = Handle(
                    CreateEventW(None, false, false, PCWSTR(event_name.as_ptr()))
                        .map_err(|_| "Cannot restore the running app")?,
                );
                if existing {
                    // Tauri spawns its replacement just before the old process
                    // exits. Drop our references while waiting, so we do not keep
                    // the old mutex alive ourselves and mistake restart for a
                    // second launch. Ordinary activation waits at most 250 ms.
                    if attempt < 5 {
                        drop(event);
                        drop(mutex);
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }
                    SetEvent(event.0).map_err(|_| "Cannot signal the running app")?;
                    return Ok(None);
                }
                return Ok(Some(Self {
                    _mutex: mutex,
                    event,
                }));
            }
        }
        Err("Cannot check the running app")
    }
    pub fn listen(self, app: tauri::AppHandle) -> Result<(), std::io::Error> {
        std::thread::Builder::new()
            .name("app-activation".into())
            .spawn(move || {
                // Keep BOTH kernel objects alive for the entire primary process.
                let instance = self;
                loop {
                    let status = unsafe { WaitForSingleObject(instance.event.0, 2000) };
                    if status != WAIT_OBJECT_0 && status != WAIT_TIMEOUT {
                        break;
                    }
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if status == WAIT_OBJECT_0 {
                            restore(&handle);
                        } else {
                            recover_hidden(&handle);
                        }
                    });
                }
            })?;
        Ok(())
    }
}
fn recover_hidden(app: &tauri::AppHandle) {
    if closing() {
        return;
    }
    // Recovery for a mini-window destruction/transition race. A normally
    // minimized window still has a taskbar entry and must stay minimized.
    if let Some(main) = app.get_webview_window("main") {
        if !main.is_visible().unwrap_or(true)
            && app
                .get_webview_window("mini-player")
                .is_none_or(|w| !w.is_visible().unwrap_or(true))
        {
            restore(app);
        }
    } else {
        prepare_exit(app);
        app.exit(0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn second_launch_signals_first_and_releases_after_exit() {
        let id = rand::random::<u64>();
        let mutex: Vec<u16> = format!("Local\\LL-Test-{id}-mutex")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let event: Vec<u16> = format!("Local\\LL-Test-{id}-event")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let first = Instance::acquire_named(&mutex, &event).unwrap().unwrap();
        assert!(Instance::acquire_named(&mutex, &event).unwrap().is_none());
        assert_eq!(
            unsafe { WaitForSingleObject(first.event.0, 1000) },
            WAIT_OBJECT_0
        );
        assert_ne!(
            unsafe { WaitForSingleObject(first.event.0, 0) },
            WAIT_OBJECT_0
        );
        drop(first);
        assert!(Instance::acquire_named(&mutex, &event).unwrap().is_some());
    }
    #[test]
    fn replacement_can_start_while_previous_process_finishes_exiting() {
        let id = rand::random::<u64>();
        let mutex: Vec<u16> = format!("Local\\LL-Restart-Test-{id}-mutex")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let event: Vec<u16> = format!("Local\\LL-Restart-Test-{id}-event")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let first = Instance::acquire_named(&mutex, &event).unwrap().unwrap();
        let finishing = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            drop(first);
        });
        let replacement = Instance::acquire_named(&mutex, &event).unwrap();
        finishing.join().unwrap();
        assert!(replacement.is_some());
    }
}
/// Recover the main window from hidden/minimized state and cancel pending
/// automatic transitions before focusing it.
pub fn restore(app: &tauri::AppHandle) {
    if closing() {
        return;
    }
    app.state::<crate::start_playback::StartPlayback>().cancel();
    if let Some(main) = app.get_webview_window("main") {
        let result = main
            .unminimize()
            .and_then(|_| main.show())
            .and_then(|_| main.set_focus());
        if result.is_ok() {
            crate::diagnostics::record(crate::diagnostics::Code::LibraryShown);
        } else {
            crate::diagnostics::record(crate::diagnostics::Code::WindowRestoreFailed);
        }
    }
}
/// Coordinate idempotent shutdown: cancel launch intent, pause playback, flush
/// confirmed progress and save placement before releasing process ownership.
pub fn prepare_exit(app: &tauri::AppHandle) {
    if CLOSING.swap(true, Ordering::SeqCst) {
        return;
    }
    crate::diagnostics::record(crate::diagnostics::Code::ExitRequested);
    app.state::<crate::start_playback::StartPlayback>().cancel();
    crate::launch::next_instance(app);
    if let Some(w) = app.get_webview_window("official") {
        let _ = w.eval("window.__audibleAdapter?.hold(true);document.querySelectorAll('audio,video').forEach(m=>m.pause())");
    }
    if crate::metadata::connected(app) {
        if crate::history::flush(app, "close").is_err() {
            crate::diagnostics::record(crate::diagnostics::Code::ProgressSaveFailed);
        }
        crate::activity::finish(app);
    }
    let _ =
        tauri_plugin_window_state::AppHandleExt::save_window_state(app, crate::window_state::FLAGS);
}
