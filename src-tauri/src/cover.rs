use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub(crate) fn dismiss(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("cover-artwork") {
        let _ = w.destroy();
    }
}

#[derive(serde::Serialize)]
pub struct Artwork {
    title: String,
    url: String,
}
fn artwork(app: &tauri::AppHandle) -> Result<Artwork, String> {
    super::metadata::require_account(app)?;
    let hub = app.state::<super::desktop::PlayerHub>();
    let state = hub.0.lock().map_err(|_| "Player state unavailable")?;
    let book = state.book.as_ref().ok_or("Choose a book first")?;
    Ok(Artwork {
        title: book["title"].as_str().unwrap_or("Book cover").to_owned(),
        url: from_book(book).ok_or("Cover artwork is unavailable for this book")?,
    })
}
#[tauri::command]
pub async fn open_cover(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    super::desktop::player_local(&window)?;
    artwork(&app)?;
    if let Some(w) = app.get_webview_window("cover-artwork") {
        w.show().map_err(|_| "Cannot show cover")?;
        return w.set_focus().map_err(|_| "Cannot focus cover".into());
    }
    let mut width: f64 = 640.0;
    let mut height: f64 = 700.0;
    if let Ok(Some(monitor)) = window.current_monitor() {
        width = width
            .min(monitor.work_area().size.width as f64 / monitor.scale_factor() - 60.0)
            .max(240.0);
        height = height
            .min(monitor.work_area().size.height as f64 / monitor.scale_factor() - 100.0)
            .max(240.0);
    }
    WebviewWindowBuilder::new(
        &app,
        "cover-artwork",
        WebviewUrl::App("index.html?cover=1".into()),
    )
    .title("Listener’s Library · cover artwork")
    .inner_size(width, height)
    .min_inner_size(240.0, 240.0)
    .center()
    .always_on_top(true)
    .resizable(true)
    .devtools(cfg!(debug_assertions))
    .on_navigation(super::local_origin)
    .parent(&window)
    .map_err(|_| "Cannot attach cover window")?
    .build()
    .map_err(|_| "Cannot open cover artwork")?;
    Ok(())
}
fn cover_local(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "cover-artwork" || !window.url().is_ok_and(|u| super::local_origin(&u)) {
        return Err("Local cover window required".into());
    }
    Ok(())
}
#[tauri::command]
pub fn current_cover(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Artwork, String> {
    cover_local(&window)?;
    artwork(&app)
}
#[tauri::command]
pub fn close_cover(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    cover_local(&window)?;
    window.close().map_err(|_| "Cannot close cover")?;
    if let Some(mini) = app.get_webview_window("mini-player") {
        let _ = mini.set_focus();
    }
    Ok(())
}

pub fn safe_url(raw: &str) -> Option<String> {
    if raw.len() > 2048 {
        return None;
    }
    let u = url::Url::parse(raw).ok()?;
    (u.scheme() == "https"
        && u.username().is_empty()
        && u.password().is_none()
        && u.port_or_known_default() == Some(443)
        && matches!(
            u.host_str(),
            Some("m.media-amazon.com" | "images-na.ssl-images-amazon.com")
        )
        && u.path().starts_with("/images/I/")
        && u.query().is_none()
        && u.fragment().is_none())
    .then(|| u.to_string())
}
pub fn from_book(b: &serde_json::Value) -> Option<String> {
    b["coverUrl"].as_str().and_then(safe_url).or_else(|| {
        ["500", "1215", "252", "200"]
            .iter()
            .filter_map(|size| b["product_images"][*size].as_str())
            .find_map(safe_url)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allows_only_public_art_without_account_context() {
        assert!(safe_url("https://m.media-amazon.com/images/I/example.jpg").is_some());
        for raw in [
            "https://evil.test/images/I/a.jpg",
            "https://m.media-amazon.com/images/I/a.jpg?token=private",
            "https://user@m.media-amazon.com/images/I/a.jpg",
            "https://m.media-amazon.com:999/images/I/a.jpg",
            "http://m.media-amazon.com/images/I/a.jpg",
        ] {
            assert!(safe_url(raw).is_none());
        }
        assert_eq!(from_book(&serde_json::json!({"product_images":{"500":"https://m.media-amazon.com/images/I/example.jpg"}})).unwrap(),"https://m.media-amazon.com/images/I/example.jpg");
    }
}
