//! HTML sanitization for preview rendering.

/// Sanitizes an HTML representation before it is rendered in the preview
/// pane: strips executable script and event-handler attributes while
/// preserving benign formatting tags.
pub fn sanitize_html(input: &str) -> String {
    ammonia::clean(input)
}

#[tauri::command]
pub fn sanitize_clip_html(html: String) -> String {
    sanitize_html(&html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_tags_are_stripped() {
        let sanitized = sanitize_html("<script>alert('x')</script><p>hi</p>");
        assert!(!sanitized.to_lowercase().contains("<script"));
    }

    #[test]
    fn benign_formatting_tags_survive() {
        let sanitized = sanitize_html(r#"<b>bold</b> <a href="https://example.com">link</a>"#);
        assert!(sanitized.contains("<b>bold</b>"));
        assert!(sanitized.contains("href=\"https://example.com/\"") || sanitized.contains("href=\"https://example.com\""));
    }
}
