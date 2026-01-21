//! Quick links data models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a quick link.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuickLinkId(String);

impl QuickLinkId {
    /// Creates a new `QuickLinkId`.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generates a new unique ID.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for QuickLinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for QuickLinkId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for QuickLinkId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<i64> for QuickLinkId {
    fn from(id: i64) -> Self {
        Self(id.to_string())
    }
}

/// Icon type for quick links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
#[derive(Default)]
pub enum QuickLinkIcon {
    /// Cached favicon from URL.
    Favicon(PathBuf),
    /// Single emoji character.
    Emoji(String),
    /// SF Symbol name (macOS system icons).
    SystemIcon(String),
    /// User-provided custom image.
    CustomImage(PathBuf),
    /// Default globe icon.
    #[default]
    Default,
}


impl QuickLinkIcon {
    /// Parses icon from a string representation.
    ///
    /// Formats:
    /// - Emoji: just the emoji character (e.g., "🔍")
    /// - Favicon: "favicon:/path/to/icon.png"
    /// - SystemIcon: "system:globe"
    /// - CustomImage: "custom:/path/to/image.png"
    /// - Default: "default" or empty
    pub fn from_string(s: &str) -> Self {
        let s = s.trim();

        if s.is_empty() || s == "default" {
            return Self::Default;
        }

        if let Some(path) = s.strip_prefix("favicon:") {
            return Self::Favicon(PathBuf::from(path));
        }

        if let Some(name) = s.strip_prefix("system:") {
            return Self::SystemIcon(name.to_string());
        }

        if let Some(path) = s.strip_prefix("custom:") {
            return Self::CustomImage(PathBuf::from(path));
        }

        // Assume it's an emoji if it's a short string with non-ASCII
        if s.chars().count() <= 4 && !s.is_ascii() {
            return Self::Emoji(s.to_string());
        }

        // Fall back to default
        Self::Default
    }

    /// Converts icon to a string representation.
    pub fn to_string_repr(&self) -> String {
        match self {
            Self::Favicon(path) => format!("favicon:{}", path.display()),
            Self::Emoji(emoji) => emoji.clone(),
            Self::SystemIcon(name) => format!("system:{name}"),
            Self::CustomImage(path) => format!("custom:{}", path.display()),
            Self::Default => "default".to_string(),
        }
    }

    /// Returns the icon path if this is a file-based icon.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::Favicon(path) | Self::CustomImage(path) => Some(path),
            _ => None,
        }
    }
}

/// Quick link data structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickLink {
    /// Unique identifier.
    pub id: QuickLinkId,

    /// Display name.
    pub name: String,

    /// URL/path (can contain {argument} placeholder for dynamic URLs).
    pub link: String,

    /// Bundle ID for "Open With" application (optional).
    pub open_with: Option<String>,

    /// Icon for the quick link.
    #[serde(default)]
    pub icon: QuickLinkIcon,

    /// Short alias keyword for quick access (optional).
    pub alias: Option<String>,

    /// Serialized hotkey (JSON format, optional).
    pub hotkey: Option<String>,

    /// Keywords for search matching.
    pub keywords: Vec<String>,

    /// Tags for organization.
    pub tags: Vec<String>,

    /// When the link was created.
    pub created_at: DateTime<Utc>,

    /// When the link was last accessed.
    pub accessed_at: Option<DateTime<Utc>>,

    /// Number of times accessed.
    pub access_count: u64,
}

impl QuickLink {
    #[allow(clippy::literal_string_with_formatting_args)]
    const ARGUMENT_PLACEHOLDER: &'static str = "{argument}";
    #[allow(clippy::literal_string_with_formatting_args)]
    const QUERY_PLACEHOLDER: &'static str = "{query}";

    /// Creates a new quick link.
    pub fn new(name: impl Into<String>, link: impl Into<String>) -> Self {
        Self {
            id: QuickLinkId::generate(),
            name: name.into(),
            link: link.into(),
            open_with: None,
            icon: QuickLinkIcon::Default,
            alias: None,
            hotkey: None,
            keywords: Vec::new(),
            tags: Vec::new(),
            created_at: Utc::now(),
            accessed_at: None,
            access_count: 0,
        }
    }

    /// Sets keywords.
    #[must_use]
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Sets tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Sets icon.
    #[must_use]
    pub fn with_icon(mut self, icon: QuickLinkIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Sets alias.
    #[must_use]
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Sets hotkey.
    #[must_use]
    pub fn with_hotkey(mut self, hotkey: impl Into<String>) -> Self {
        self.hotkey = Some(hotkey.into());
        self
    }

    /// Sets open_with bundle ID.
    #[must_use]
    pub fn with_open_with(mut self, bundle_id: impl Into<String>) -> Self {
        self.open_with = Some(bundle_id.into());
        self
    }

    /// Checks if this is a dynamic URL (contains {argument} or {query} placeholder).
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        self.link.contains(Self::ARGUMENT_PLACEHOLDER)
            || self.link.contains(Self::QUERY_PLACEHOLDER)
    }

    /// Substitutes argument into URL if it's dynamic.
    #[must_use]
    pub fn substitute_argument(&self, argument: &str) -> String {
        self.link
            .replace(Self::ARGUMENT_PLACEHOLDER, argument)
            .replace(Self::QUERY_PLACEHOLDER, argument)
    }

    /// Alias for backward compatibility.
    #[must_use]
    pub fn substitute_query(&self, query: &str) -> String {
        self.substitute_argument(query)
    }

    /// Increments access count and updates accessed_at.
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.accessed_at = Some(Utc::now());
    }

    /// Returns all searchable text for this link.
    #[must_use]
    pub fn searchable_text(&self) -> String {
        let mut parts = vec![self.name.clone(), self.link.clone()];
        if let Some(ref alias) = self.alias {
            parts.push(alias.clone());
        }
        parts.extend(self.keywords.iter().cloned());
        parts.extend(self.tags.iter().cloned());
        parts.join(" ")
    }

    /// Returns the URL field (alias for link, for backward compatibility).
    #[must_use]
    pub fn url(&self) -> &str {
        &self.link
    }

    /// Returns the title field (alias for name, for backward compatibility).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.name
    }
}

/// TOML representation of quick links for export/import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickLinksToml {
    /// List of quick links.
    pub links: Vec<QuickLinkToml>,
}

/// TOML representation of a single quick link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickLinkToml {
    /// Display name.
    pub name: String,

    /// URL/path (can contain {argument} placeholder).
    pub link: String,

    /// Short alias keyword for quick access (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    /// Icon (emoji, or prefixed string like "favicon:", "system:", "custom:").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Bundle ID for "Open With" application (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_with: Option<String>,

    /// Hotkey (e.g., "cmd+shift+g").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,

    /// Keywords (optional).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// Tags (optional).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl From<QuickLink> for QuickLinkToml {
    fn from(link: QuickLink) -> Self {
        let icon = match &link.icon {
            QuickLinkIcon::Default => None,
            other => Some(other.to_string_repr()),
        };

        Self {
            name: link.name,
            link: link.link,
            alias: link.alias,
            icon,
            open_with: link.open_with,
            hotkey: link.hotkey,
            keywords: link.keywords,
            tags: link.tags,
        }
    }
}

impl From<QuickLinkToml> for QuickLink {
    fn from(toml: QuickLinkToml) -> Self {
        let icon = toml
            .icon
            .as_ref()
            .map_or(QuickLinkIcon::Default, |s| QuickLinkIcon::from_string(s));

        Self {
            id: QuickLinkId::generate(),
            name: toml.name,
            link: toml.link,
            open_with: toml.open_with,
            icon,
            alias: toml.alias,
            hotkey: toml.hotkey,
            keywords: toml.keywords,
            tags: toml.tags,
            created_at: Utc::now(),
            accessed_at: None,
            access_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_link_creation() {
        let link = QuickLink::new("GitHub", "https://github.com");
        assert_eq!(link.name, "GitHub");
        assert_eq!(link.link, "https://github.com");
        assert!(!link.is_dynamic());
        assert!(matches!(link.icon, QuickLinkIcon::Default));
    }

    #[test]
    fn test_dynamic_url_with_argument() {
        let link = QuickLink::new("GitHub Search", "https://github.com/search?q={argument}");
        assert!(link.is_dynamic());

        let url = link.substitute_argument("rust");
        assert_eq!(url, "https://github.com/search?q=rust");
    }

    #[test]
    fn test_dynamic_url_with_query() {
        let link = QuickLink::new("GitHub Search", "https://github.com/search?q={query}");
        assert!(link.is_dynamic());

        let url = link.substitute_query("rust");
        assert_eq!(url, "https://github.com/search?q=rust");
    }

    #[test]
    fn test_mark_accessed() {
        let mut link = QuickLink::new("Test", "https://example.com");
        assert_eq!(link.access_count, 0);
        assert!(link.accessed_at.is_none());

        link.mark_accessed();
        assert_eq!(link.access_count, 1);
        assert!(link.accessed_at.is_some());
    }

    #[test]
    fn test_searchable_text() {
        let link = QuickLink::new("GitHub", "https://github.com")
            .with_keywords(vec!["gh".to_string(), "git".to_string()])
            .with_tags(vec!["dev".to_string()])
            .with_alias("gh");

        let text = link.searchable_text();
        assert!(text.contains("GitHub"));
        assert!(text.contains("github.com"));
        assert!(text.contains("gh"));
        assert!(text.contains("dev"));
    }

    #[test]
    fn test_quick_link_with_all_fields() {
        let link = QuickLink::new("Google Search", "https://google.com/search?q={argument}")
            .with_alias("g")
            .with_icon(QuickLinkIcon::Emoji("🔍".to_string()))
            .with_open_with("com.apple.Safari")
            .with_hotkey("cmd+shift+g")
            .with_keywords(vec!["search".to_string()])
            .with_tags(vec!["web".to_string()]);

        assert_eq!(link.name, "Google Search");
        assert_eq!(link.alias, Some("g".to_string()));
        assert_eq!(link.icon, QuickLinkIcon::Emoji("🔍".to_string()));
        assert_eq!(link.open_with, Some("com.apple.Safari".to_string()));
        assert_eq!(link.hotkey, Some("cmd+shift+g".to_string()));
    }

    #[test]
    fn test_icon_from_string() {
        // Emoji
        assert_eq!(
            QuickLinkIcon::from_string("🔍"),
            QuickLinkIcon::Emoji("🔍".to_string())
        );

        // Favicon
        assert_eq!(
            QuickLinkIcon::from_string("favicon:/path/to/icon.png"),
            QuickLinkIcon::Favicon(PathBuf::from("/path/to/icon.png"))
        );

        // System icon
        assert_eq!(
            QuickLinkIcon::from_string("system:globe"),
            QuickLinkIcon::SystemIcon("globe".to_string())
        );

        // Custom image
        assert_eq!(
            QuickLinkIcon::from_string("custom:/path/to/image.png"),
            QuickLinkIcon::CustomImage(PathBuf::from("/path/to/image.png"))
        );

        // Default
        assert_eq!(QuickLinkIcon::from_string("default"), QuickLinkIcon::Default);
        assert_eq!(QuickLinkIcon::from_string(""), QuickLinkIcon::Default);
    }

    #[test]
    fn test_icon_to_string() {
        assert_eq!(
            QuickLinkIcon::Emoji("🔍".to_string()).to_string_repr(),
            "🔍"
        );
        assert_eq!(
            QuickLinkIcon::Favicon(PathBuf::from("/path/icon.png")).to_string_repr(),
            "favicon:/path/icon.png"
        );
        assert_eq!(
            QuickLinkIcon::SystemIcon("globe".to_string()).to_string_repr(),
            "system:globe"
        );
        assert_eq!(QuickLinkIcon::Default.to_string_repr(), "default");
    }

    #[test]
    fn test_toml_conversion() {
        let link = QuickLink::new("GitHub", "https://github.com")
            .with_alias("gh")
            .with_icon(QuickLinkIcon::Emoji("🐙".to_string()))
            .with_open_with("com.apple.Safari")
            .with_hotkey("cmd+g");

        let toml: QuickLinkToml = link.into();
        assert_eq!(toml.name, "GitHub");
        assert_eq!(toml.link, "https://github.com");
        assert_eq!(toml.alias, Some("gh".to_string()));
        assert_eq!(toml.icon, Some("🐙".to_string()));
        assert_eq!(toml.open_with, Some("com.apple.Safari".to_string()));
        assert_eq!(toml.hotkey, Some("cmd+g".to_string()));

        // Convert back
        let link2: QuickLink = toml.into();
        assert_eq!(link2.name, "GitHub");
        assert_eq!(link2.alias, Some("gh".to_string()));
        assert_eq!(link2.icon, QuickLinkIcon::Emoji("🐙".to_string()));
    }

    #[test]
    fn test_backward_compat_accessors() {
        let link = QuickLink::new("Test", "https://test.com");
        assert_eq!(link.title(), "Test");
        assert_eq!(link.url(), "https://test.com");
    }
}
