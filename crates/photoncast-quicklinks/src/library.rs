//! Bundled quicklinks library.
//!
//! Provides a collection of pre-defined quicklinks for common search engines,
//! developer tools, media sites, and reference resources.

use crate::models::{QuickLink, QuickLinkIcon};

/// A pre-defined quicklink that ships with PhotonCast.
#[derive(Debug, Clone, Copy)]
pub struct BundledQuickLink {
    /// Display name.
    pub name: &'static str,
    /// URL pattern (may contain `{argument}` placeholder).
    pub link: &'static str,
    /// Short alias for quick access.
    pub alias: Option<&'static str>,
    /// Icon (emoji or "system:name").
    pub icon: &'static str,
    /// Category for organization.
    pub category: &'static str,
}

/// Bundled quicklinks that ship with PhotonCast.
pub const BUNDLED_QUICKLINKS: &[BundledQuickLink] = &[
    // Search Engines
    BundledQuickLink {
        name: "Google Search",
        link: "https://google.com/search?q={argument}",
        alias: Some("g"),
        icon: "🔍",
        category: "Search",
    },
    BundledQuickLink {
        name: "DuckDuckGo Search",
        link: "https://duckduckgo.com/?q={argument}",
        alias: Some("ddg"),
        icon: "🦆",
        category: "Search",
    },
    // Developer
    BundledQuickLink {
        name: "GitHub Search",
        link: "https://github.com/search?q={argument}&type=repositories",
        alias: Some("gh"),
        icon: "🐙",
        category: "Developer",
    },
    BundledQuickLink {
        name: "Stack Overflow",
        link: "https://stackoverflow.com/search?q={argument}",
        alias: Some("so"),
        icon: "📚",
        category: "Developer",
    },
    BundledQuickLink {
        name: "npm Search",
        link: "https://www.npmjs.com/search?q={argument}",
        alias: Some("npm"),
        icon: "📦",
        category: "Developer",
    },
    BundledQuickLink {
        name: "crates.io Search",
        link: "https://crates.io/search?q={argument}",
        alias: Some("crate"),
        icon: "🦀",
        category: "Developer",
    },
    BundledQuickLink {
        name: "MDN Web Docs",
        link: "https://developer.mozilla.org/en-US/search?q={argument}",
        alias: Some("mdn"),
        icon: "📖",
        category: "Developer",
    },
    // Media
    BundledQuickLink {
        name: "YouTube Search",
        link: "https://www.youtube.com/results?search_query={argument}",
        alias: Some("yt"),
        icon: "▶️",
        category: "Media",
    },
    BundledQuickLink {
        name: "Spotify Search",
        link: "https://open.spotify.com/search/{argument}",
        alias: Some("sp"),
        icon: "🎵",
        category: "Media",
    },
    // Reference
    BundledQuickLink {
        name: "Wikipedia",
        link: "https://en.wikipedia.org/wiki/Special:Search?search={argument}",
        alias: Some("wiki"),
        icon: "📗",
        category: "Reference",
    },
    BundledQuickLink {
        name: "Google Translate",
        link: "https://translate.google.com/?sl=auto&tl=en&text={argument}",
        alias: Some("tr"),
        icon: "🌐",
        category: "Reference",
    },
    BundledQuickLink {
        name: "Google Maps",
        link: "https://www.google.com/maps/search/{argument}",
        alias: Some("maps"),
        icon: "🗺️",
        category: "Reference",
    },
    // Shopping
    BundledQuickLink {
        name: "Amazon Search",
        link: "https://www.amazon.com/s?k={argument}",
        alias: Some("amz"),
        icon: "🛒",
        category: "Shopping",
    },
    // Social
    BundledQuickLink {
        name: "Twitter/X Search",
        link: "https://twitter.com/search?q={argument}",
        alias: Some("x"),
        icon: "🐦",
        category: "Social",
    },
    BundledQuickLink {
        name: "Reddit Search",
        link: "https://www.reddit.com/search/?q={argument}",
        alias: Some("r"),
        icon: "🤖",
        category: "Social",
    },
];

/// Returns all bundled quicklinks.
#[must_use]
pub fn get_bundled_quicklinks() -> &'static [BundledQuickLink] {
    BUNDLED_QUICKLINKS
}

/// Returns quicklinks filtered by category.
#[must_use]
pub fn get_by_category(category: &str) -> Vec<&'static BundledQuickLink> {
    BUNDLED_QUICKLINKS
        .iter()
        .filter(|link| link.category.eq_ignore_ascii_case(category))
        .collect()
}

/// Returns all unique categories in the bundled quicklinks.
#[must_use]
pub fn get_categories() -> Vec<&'static str> {
    let mut categories: Vec<&'static str> = BUNDLED_QUICKLINKS
        .iter()
        .map(|link| link.category)
        .collect();
    categories.sort();
    categories.dedup();
    categories
}

/// Converts a bundled quicklink to a user-editable `QuickLink`.
#[must_use]
pub fn to_quicklink(bundled: &BundledQuickLink) -> QuickLink {
    let mut keywords = Vec::new();

    // Add alias as a keyword if present
    if let Some(alias) = bundled.alias {
        keywords.push(alias.to_string());
    }

    let icon = QuickLinkIcon::from_string(bundled.icon);

    let mut quicklink = QuickLink::new(bundled.name, bundled.link)
        .with_keywords(keywords)
        .with_tags(vec![bundled.category.to_string()])
        .with_icon(icon);

    // Set the alias if present
    if let Some(alias) = bundled.alias {
        quicklink = quicklink.with_alias(alias);
    }

    quicklink
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_bundled_quicklinks_not_empty() {
        assert!(!BUNDLED_QUICKLINKS.is_empty());
    }

    #[test]
    fn test_all_links_have_valid_urls() {
        for link in BUNDLED_QUICKLINKS {
            assert!(
                link.link.starts_with("https://"),
                "Link '{}' should start with https://",
                link.name
            );
        }
    }

    #[test]
    fn test_all_links_have_argument_placeholder() {
        for link in BUNDLED_QUICKLINKS {
            assert!(
                link.link.contains("{argument}"),
                "Link '{}' should contain {{argument}} placeholder",
                link.name
            );
        }
    }

    #[test]
    fn test_all_aliases_are_unique() {
        let aliases: Vec<&str> = BUNDLED_QUICKLINKS
            .iter()
            .filter_map(|link| link.alias)
            .collect();

        let unique_aliases: HashSet<&str> = aliases.iter().copied().collect();

        assert_eq!(
            aliases.len(),
            unique_aliases.len(),
            "All aliases should be unique"
        );
    }

    #[test]
    fn test_all_names_are_unique() {
        let names: Vec<&str> = BUNDLED_QUICKLINKS.iter().map(|link| link.name).collect();
        let unique_names: HashSet<&str> = names.iter().copied().collect();

        assert_eq!(
            names.len(),
            unique_names.len(),
            "All names should be unique"
        );
    }

    #[test]
    fn test_get_bundled_quicklinks() {
        let links = get_bundled_quicklinks();
        assert_eq!(links.len(), BUNDLED_QUICKLINKS.len());
    }

    #[test]
    fn test_get_by_category() {
        let search_links = get_by_category("Search");
        assert!(!search_links.is_empty());

        for link in &search_links {
            assert_eq!(link.category, "Search");
        }
    }

    #[test]
    fn test_get_by_category_case_insensitive() {
        let search_links = get_by_category("search");
        assert!(!search_links.is_empty());
    }

    #[test]
    fn test_get_categories() {
        let categories = get_categories();
        assert!(categories.contains(&"Search"));
        assert!(categories.contains(&"Developer"));
        assert!(categories.contains(&"Media"));
        assert!(categories.contains(&"Reference"));
        assert!(categories.contains(&"Shopping"));
        assert!(categories.contains(&"Social"));
    }

    #[test]
    fn test_to_quicklink() {
        let bundled = &BUNDLED_QUICKLINKS[0];
        let quicklink = to_quicklink(bundled);

        assert_eq!(quicklink.name, bundled.name);
        assert_eq!(quicklink.link, bundled.link);
        assert!(quicklink.is_dynamic());

        if let Some(alias) = bundled.alias {
            assert!(quicklink.keywords.contains(&alias.to_string()));
            assert_eq!(quicklink.alias.as_deref(), Some(alias));
        }

        assert!(quicklink.tags.contains(&bundled.category.to_string()));
    }

    #[test]
    fn test_quicklink_substitution() {
        let bundled = &BUNDLED_QUICKLINKS[0]; // Google Search
        let quicklink = to_quicklink(bundled);
        let url = quicklink.substitute_argument("rust programming");

        assert!(url.contains("rust programming"));
        assert!(!url.contains("{argument}"));
    }
}
