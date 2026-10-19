use std::{sync::mpsc, time::Duration};
use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub(crate) const FLAGS: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_window_state::Builder::default()
        .with_state_flags(FLAGS)
        .with_filter(|label| matches!(label, "main" | "mini-player"))
        .build()
}

struct Checkpoint(mpsc::Sender<()>);

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    app.manage(Checkpoint(tx));
    let handle = app.handle().clone();
    // Dev rebuilds can terminate the process without a normal Exit event.
    // Persist after dragging/resizing settles, rather than only at shutdown.
    std::thread::Builder::new()
        .name("window-state-checkpoint".into())
        .spawn(move || {
            while rx.recv().is_ok() {
                while rx.recv_timeout(Duration::from_millis(350)).is_ok() {}
                if handle.save_window_state(FLAGS).is_err() {
                    eprintln!("Could not save window placement");
                }
            }
        })?;
    Ok(())
}

pub fn changed(window: &tauri::Window, event: &WindowEvent) {
    if !matches!(window.label(), "main" | "mini-player") {
        return;
    }
    match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if let Some(checkpoint) = window.app_handle().try_state::<Checkpoint>() {
                let _ = checkpoint.0.send(());
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            if window.label() == "main" {
                api.prevent_close();
                crate::lifecycle::prepare_exit(window.app_handle());
                // Hidden official/player windows must not keep a headless
                // process alive after the user closes the library.
                window.app_handle().exit(0);
                return;
            }
            if window.label() == "mini-player" && !crate::lifecycle::closing() {
                crate::diagnostics::record(crate::diagnostics::Code::MiniClosed);
                let hub = window.app_handle().state::<crate::desktop::PlayerHub>();
                if let Ok(_state) = hub.0.lock() {
                    hub.changed();
                }
                crate::cover::dismiss(window.app_handle());
                window
                    .app_handle()
                    .state::<crate::start_playback::StartPlayback>()
                    .cancel();
                if let Some(main) = window.app_handle().get_webview_window("main") {
                    let _ = main.unminimize();
                    let _ = main.show();
                    let _ = main.set_focus();
                }
            }
            if window.app_handle().save_window_state(FLAGS).is_err() {
                eprintln!("Could not save window placement");
            }
        }
        WindowEvent::Destroyed
            if window.label() == "mini-player" && !crate::lifecycle::closing() =>
        {
            crate::lifecycle::restore(window.app_handle());
        }
        _ => {}
    }
}
