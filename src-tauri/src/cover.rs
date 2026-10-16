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
