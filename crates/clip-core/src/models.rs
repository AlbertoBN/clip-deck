//! Clip, ClipRepresentation, Group, Rule, AppContext, PasteMode.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteMode {
    #[default]
    Auto,
    Rich,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppContext {
    pub app: String,
    pub window: Option<String>,
}

impl AppContext {
    pub fn new(app: impl Into<String>) -> Self {
        Self { app: app.into(), window: None }
    }

    pub fn with_window(app: impl Into<String>, window: impl Into<String>) -> Self {
        Self { app: app.into(), window: Some(window.into()) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent_group_id: Option<String>,
    pub sort_order: i64,
}

impl Group {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        parent_group_id: Option<String>,
    ) -> Result<Self, crate::errors::CoreError> {
        let id = id.into();
        if parent_group_id.as_deref() == Some(id.as_str()) {
            return Err(crate::errors::CoreError::InvalidGroupParent(id));
        }
        Ok(Self { id, name: name.into(), parent_group_id, sort_order: 0 })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Exclude,
    Ephemeral,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub app_match: String,
    pub window_match: Option<String>,
    pub mime_match: Option<String>,
    pub action: RuleAction,
    pub enabled: bool,
}

impl Rule {
    pub fn new(
        id: impl Into<String>,
        app_match: impl Into<String>,
        window_match: Option<String>,
        mime_match: Option<String>,
        action: RuleAction,
    ) -> Self {
        Self {
            id: id.into(),
            app_match: app_match.into(),
            window_match,
            mime_match,
            action,
            enabled: true,
        }
    }

    pub fn matches(&self, ctx: &AppContext, mime: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if self.app_match != ctx.app {
            return false;
        }
        if let Some(window_match) = &self.window_match {
            if ctx.window.as_deref() != Some(window_match.as_str()) {
                return false;
            }
        }
        if let Some(mime_match) = &self.mime_match {
            if mime_match != mime {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipRepresentation {
    pub mime_type: String,
    pub text_value: Option<String>,
    pub blob_path: Option<String>,
    pub preview_text: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub byte_size: u64,
    pub ordinal: i64,
    pub is_preview: bool,
}

impl ClipRepresentation {
    pub fn new(mime_type: impl Into<String>, ordinal: i64) -> Self {
        Self {
            mime_type: mime_type.into(),
            text_value: None,
            blob_path: None,
            preview_text: None,
            width: None,
            height: None,
            byte_size: 0,
            ordinal,
            is_preview: false,
        }
    }

    pub fn with_text_value(mut self, text_value: impl Into<String>) -> Self {
        self.text_value = Some(text_value.into());
        self
    }

    pub fn with_byte_size(mut self, byte_size: u64) -> Self {
        self.byte_size = byte_size;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_used_at: Option<time::OffsetDateTime>,
    pub source_app: Option<String>,
    pub source_window: Option<String>,
    pub primary_mime: String,
    pub display_text: Option<String>,
    pub content_hash: String,
    pub byte_size: u64,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub is_deleted: bool,
    pub group_id: Option<String>,
    pub paste_mode_default: PasteMode,
    pub metadata: Option<serde_json::Value>,
    pub representations: Vec<ClipRepresentation>,
}

impl Clip {
    pub fn new(
        id: impl Into<String>,
        content_hash: impl Into<String>,
        primary_mime: impl Into<String>,
        representations: Vec<ClipRepresentation>,
    ) -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            id: id.into(),
            created_at: now,
            updated_at: now,
            last_used_at: None,
            source_app: None,
            source_window: None,
            primary_mime: primary_mime.into(),
            display_text: None,
            content_hash: content_hash.into(),
            byte_size: 0,
            is_favorite: false,
            is_pinned: false,
            is_deleted: false,
            group_id: None,
            paste_mode_default: PasteMode::default(),
            metadata: None,
            representations,
        }
    }

    pub fn dedup_key(&self) -> String {
        crate::hashing::dedup_key(&self.content_hash, &self.primary_mime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paste_mode_is_auto() {
        assert_eq!(PasteMode::default(), PasteMode::Auto);
    }

    #[test]
    fn plain_text_paste_mode_round_trips_through_serde() {
        let json = serde_json::to_string(&PasteMode::PlainText).unwrap();
        let round_tripped: PasteMode = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, PasteMode::PlainText);
    }

    #[test]
    fn app_context_with_only_app_name_is_valid() {
        let ctx = AppContext::new("gnome-terminal");
        assert_eq!(ctx.app, "gnome-terminal");
        assert!(ctx.window.is_none());
    }

    #[test]
    fn group_can_reference_a_different_group_as_parent() {
        let group = Group::new("child", "Child", Some("parent".to_string())).unwrap();
        assert_eq!(group.parent_group_id, Some("parent".to_string()));
    }

    #[test]
    fn group_cannot_be_its_own_parent() {
        let result = Group::new("g1", "G1", Some("g1".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn rule_matches_by_app_name_alone() {
        let rule = Rule::new("r1", "1Password", None, None, RuleAction::Exclude);
        let ctx = AppContext::new("1Password");
        assert!(rule.matches(&ctx, "text/plain"));
    }

    #[test]
    fn disabled_rule_never_matches() {
        let mut rule = Rule::new("r1", "1Password", None, None, RuleAction::Exclude);
        rule.enabled = false;
        let ctx = AppContext::new("1Password");
        assert!(!rule.matches(&ctx, "text/plain"));
    }

    #[test]
    fn representation_reports_its_own_byte_size_and_mime_type() {
        let html = ClipRepresentation::new("text/html", 0)
            .with_text_value("<b>hi</b>")
            .with_byte_size(9);
        let text = ClipRepresentation::new("text/plain", 1)
            .with_text_value("hi")
            .with_byte_size(2);
        assert_eq!(html.mime_type, "text/html");
        assert_eq!(html.byte_size, 9);
        assert_eq!(text.mime_type, "text/plain");
        assert_eq!(text.byte_size, 2);
    }

    #[test]
    fn clip_with_two_representations_preserves_both_in_order() {
        let text = ClipRepresentation::new("text/plain", 0).with_text_value("hi");
        let html = ClipRepresentation::new("text/html", 1).with_text_value("<b>hi</b>");
        let clip = Clip::new("c1", "hash1", "text/plain", vec![text.clone(), html.clone()]);
        assert_eq!(clip.representations, vec![text, html]);
    }

    #[test]
    fn new_clip_defaults_to_not_pinned_not_favorite_not_deleted() {
        let clip = Clip::new("c1", "hash1", "text/plain", vec![]);
        assert!(!clip.is_pinned);
        assert!(!clip.is_favorite);
        assert!(!clip.is_deleted);
    }

    #[test]
    fn metadata_payload_round_trips_through_serde() {
        let mut clip = Clip::new("c1", "hash1", "text/plain", vec![]);
        clip.metadata = Some(serde_json::json!({"source": "test"}));
        let json = serde_json::to_string(&clip).unwrap();
        let round_tripped: Clip = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.metadata, clip.metadata);
    }

    #[test]
    fn two_clips_with_identical_hash_and_mime_produce_same_dedup_key() {
        let a = Clip::new("a", "hash1", "text/plain", vec![]);
        let b = Clip::new("b", "hash1", "text/plain", vec![]);
        assert_eq!(a.dedup_key(), b.dedup_key());
    }

    #[test]
    fn same_hash_different_mime_produce_different_dedup_keys() {
        let a = Clip::new("a", "hash1", "text/plain", vec![]);
        let b = Clip::new("b", "hash1", "text/html", vec![]);
        assert_ne!(a.dedup_key(), b.dedup_key());
    }
}
