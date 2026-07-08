//! Canonical MIME representation handling.

use std::str::FromStr;

use crate::errors::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimeFamily {
    Text,
    Html,
    Image,
    Other,
}

/// Normalizes a MIME type string to a canonical lowercase `type/subtype` form,
/// stripping any parameters (e.g. `; charset=utf-8`).
pub fn normalize_mime(input: &str) -> Result<String, CoreError> {
    let parsed = mime::Mime::from_str(input).map_err(|_| CoreError::InvalidMime(input.to_string()))?;
    Ok(parsed.essence_str().to_string())
}

/// Classifies a MIME type string into a representation family.
pub fn mime_family(input: &str) -> Result<MimeFamily, CoreError> {
    let parsed = mime::Mime::from_str(input).map_err(|_| CoreError::InvalidMime(input.to_string()))?;
    let family = match (parsed.type_().as_str(), parsed.subtype().as_str()) {
        ("text", "html") => MimeFamily::Html,
        ("text", _) => MimeFamily::Text,
        ("image", _) => MimeFamily::Image,
        _ => MimeFamily::Other,
    };
    Ok(family)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_case_mime_type_normalizes_to_lowercase() {
        assert_eq!(normalize_mime("TEXT/HTML").unwrap(), "text/html");
    }

    #[test]
    fn mime_parameters_are_stripped() {
        assert_eq!(normalize_mime("text/plain; charset=utf-8").unwrap(), "text/plain");
    }

    #[test]
    fn text_plain_classifies_as_text() {
        assert_eq!(mime_family("text/plain").unwrap(), MimeFamily::Text);
    }

    #[test]
    fn image_png_classifies_as_image() {
        assert_eq!(mime_family("image/png").unwrap(), MimeFamily::Image);
    }

    #[test]
    fn unrecognized_mime_type_classifies_as_other() {
        assert_eq!(mime_family("application/x-custom-blob").unwrap(), MimeFamily::Other);
    }

    #[test]
    fn text_html_classifies_as_html() {
        assert_eq!(mime_family("text/html").unwrap(), MimeFamily::Html);
    }

    #[test]
    fn string_without_slash_is_rejected() {
        assert!(normalize_mime("not-a-mime-type").is_err());
    }
}
