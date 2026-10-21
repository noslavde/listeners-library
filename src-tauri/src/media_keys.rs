//! Windows media-session integration for hardware/headset controls. Routes
//! supported actions into the same native shortcut path and updates display
//! state from player observations.

use tauri::Manager;
#[cfg(windows)]
use windows::{
    core::HSTRING,
    Foundation::TypedEventHandler,
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton,
    },
    Win32::System::WinRT::ISystemMediaTransportControlsInterop,
};
#[cfg(windows)]
struct Session {
    controls: SystemMediaTransportControls,
    _button_token: i64,
    last_title: std::sync::Mutex<String>,
}
pub fn setup(app: &mut tauri::App) {
    #[cfg(windows)]
    {
        let result = (|| -> windows::core::Result<Session> {
            let main = app
                .get_webview_window("main")
                .ok_or_else(windows::core::Error::from_thread)?;
            let hwnd = main
                .hwnd()
                .map_err(|_| windows::core::Error::from_thread())?;
            let interop: ISystemMediaTransportControlsInterop = windows::core::factory::<
                SystemMediaTransportControls,
                ISystemMediaTransportControlsInterop,
            >()?;
            let controls: SystemMediaTransportControls =
                unsafe { interop.GetForWindow(windows::Win32::Foundation::HWND(hwnd.0)) }?;
            controls.SetIsEnabled(false)?;
            controls.SetIsPlayEnabled(true)?;
            controls.SetIsPauseEnabled(true)?;
            controls.SetIsStopEnabled(true)?;
            controls.SetIsNextEnabled(true)?;
            controls.SetIsPreviousEnabled(true)?;
            controls.SetIsFastForwardEnabled(true)?;
            controls.SetIsRewindEnabled(true)?;
            let handle = app.handle().clone();
            let token = controls.ButtonPressed(&TypedEventHandler::new(
                move |_,
                      args: windows::core::Ref<
                    windows::Media::SystemMediaTransportControlsButtonPressedEventArgs,
                >| {
                    if let Some(args) = args.as_ref() {
                        let action = match args.Button()? {
                            SystemMediaTransportControlsButton::Play => Some("play"),
                            SystemMediaTransportControlsButton::Pause
                            | SystemMediaTransportControlsButton::Stop => Some("pause"),
                            SystemMediaTransportControlsButton::Next
                            | SystemMediaTransportControlsButton::FastForward => Some("forward"),
                            SystemMediaTransportControlsButton::Previous
                            | SystemMediaTransportControlsButton::Rewind => Some("back"),
                            _ => None,
                        };
                        if let Some(action) = action {
                            if let Err(e) = super::shortcuts::dispatch(&handle, action) {
                                super::shortcuts::report(&handle, &e);
                            }
                        }
                    }
                    Ok(())
                },
            ))?;
            Ok(Session {
                controls,
                _button_token: token,
                last_title: std::sync::Mutex::new(String::new()),
            })
        })();
        if let Ok(session) = result {
            app.manage(session);
            if let Ok(mut state) = app.state::<super::shortcuts::Shortcuts>().0.lock() {
                state.media_keys = true;
            }
        }
    }
}
pub fn refresh(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        // Keep WinRT calls on the COM-initialized window thread that created
        // this session; the timer invoking refresh runs on the async runtime.
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || refresh_on_main(&handle));
    }
}
#[cfg(windows)]
fn refresh_on_main(app: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        let Some(session) = app.try_state::<Session>() else {
            return;
        };
        let snapshot = super::shortcuts::snapshot(app);
        let _ = session.controls.SetIsEnabled(snapshot.is_some());
        let _ = session.controls.SetPlaybackStatus(match snapshot.as_ref() {
            Some(s) if !s.paused => MediaPlaybackStatus::Playing,
            Some(_) => MediaPlaybackStatus::Paused,
            None => MediaPlaybackStatus::Closed,
        });
        if snapshot.is_none() {
            if let Ok(updater) = session.controls.DisplayUpdater() {
                let _ = updater.ClearAll();
                let _ = updater.Update();
            }
            if let Ok(mut last) = session.last_title.lock() {
                last.clear();
            }
        }
        if let Some(s) = snapshot {
            let mut last = session.last_title.lock().unwrap_or_else(|e| e.into_inner());
            if *last != s.asin {
                let book = app
                    .state::<super::desktop::PlayerHub>()
                    .0
                    .lock()
                    .ok()
                    .and_then(|v| v.book.clone());
                if let Some(book) = book {
                    if let Ok(updater) = session.controls.DisplayUpdater() {
                        let _ = updater.SetType(MediaPlaybackType::Music);
                        if let Ok(music) = updater.MusicProperties() {
                            let _ = music.SetTitle(&HSTRING::from(
                                book["title"].as_str().unwrap_or("Audiobook"),
                            ));
                            let _ = music
                                .SetArtist(&HSTRING::from(book["author"].as_str().unwrap_or("")));
                        }
                        let _ = updater.Update();
                        *last = s.asin;
                    }
                }
            }
        }
    }
}
