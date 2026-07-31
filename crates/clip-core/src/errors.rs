//! Shared error types.

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid MIME type: {0}")]
    InvalidMime(String),
    #[error("invalid rule: {0}")]
    InvalidRule(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_mime_has_non_empty_display() {
        let err = CoreError::InvalidMime("not-a-mime".into());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn invalid_rule_has_non_empty_display() {
        let err = CoreError::InvalidRule("bad rule".into());
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn core_error_implements_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<CoreError>();
    }
}
