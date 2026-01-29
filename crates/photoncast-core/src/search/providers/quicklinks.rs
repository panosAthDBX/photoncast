//! Quick links search provider.
//!
//! Supports two search modes:
//! 1. Alias prefix: "g test" matches quicklink with alias "g" and passes "test" as argument
//! 2. Fuzzy search: "google" matches quicklinks by name/keywords
//!
//! Quicklinks are cached in memory and only reloaded when explicitly invalidated.

use parking_lot::RwLock;
use std::path::PathBuf;

use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};
use crate::utils::paths;
use photoncast_quicklinks::{placeholder, QuickLink, QuickLinkIcon, QuickLinksStorage};

/// Provides search results for quick links stored in SQLite.
///
/// Uses an internal cache to avoid loading from database on every keystroke.
/// Call `invalidate_cache()` after modifying quicklinks.
pub struct QuickLinksProvider {
    storage: QuickLinksStorage,
    /// Cached quicklinks (loaded once and reused)
    cache: RwLock<Option<Vec<QuickLink>>>,
}

impl std::fmt::Debug for QuickLinksProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickLinksProvider")
            .field("storage", &self.storage)
            .field("cache", &"<RwLock>")
            .finish()
    }
}

impl QuickLinksProvider {
    /// Opens the quick links provider using the default storage path.
    ///
    /// On first use (empty database), populates with bundled quicklinks.
    pub fn new() -> anyhow::Result<Self> {
        let db_path = default_quicklinks_db_path();
        let storage = QuickLinksStorage::open(db_path)?;

        // Populate bundled quicklinks if database is empty
        if let Err(e) = storage.populate_bundled_if_empty() {
            tracing::warn!(error = %e, "Failed to populate bundled quicklinks");
        }

        Ok(Self {
            storage,
            cache: RwLock::new(None),
        })
    }

    /// Opens the quick links provider using an in-memory store (tests only).
    #[cfg(test)]
    pub fn new_in_memory() -> anyhow::Result<Self> {
        let storage = QuickLinksStorage::open_in_memory()?;
        Ok(Self {
            storage,
            cache: RwLock::new(None),
        })
    }

    /// Creates a provider from an existing storage instance.
    #[must_use]
    pub fn with_storage(storage: QuickLinksStorage) -> Self {
        Self {
            storage,
            cache: RwLock::new(None),
        }
    }

    /// Invalidates the cache, forcing a reload on next search.
    /// Call this after adding, updating, or deleting quicklinks.
    pub fn invalidate_cache(&self) {
        *self.cache.write() = None;
        tracing::debug!("QuickLinks cache invalidated");
    }

    /// Gets quicklinks from cache, loading from storage if needed.
    fn get_cached_links(&self) -> Vec<QuickLink> {
        // Try to read from cache first
        {
            let cache = self.cache.read();
            if let Some(ref links) = *cache {
                return links.clone();
            }
        }

        // Cache miss - load from storage and populate cache
        let links = match self.storage.load_all_sync() {
            Ok(links) => {
                tracing::debug!(
                    count = links.len(),
                    "Loaded quicklinks from storage (cache miss)"
                );
                links
            },
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load quicklinks");
                return Vec::new();
            },
        };

        // Store in cache
        *self.cache.write() = Some(links.clone());

        links
    }

    /// Converts a QuickLink to a SearchResult.
    #[allow(clippy::match_same_arms)]
    fn link_to_result(
        link: &QuickLink,
        score: f64,
        indices: Vec<usize>,
        arguments: &str,
    ) -> SearchResult {
        let icon = match &link.icon {
            QuickLinkIcon::Favicon(path) => IconSource::FileIcon { path: path.clone() },
            QuickLinkIcon::Emoji(emoji) => IconSource::Emoji {
                char: emoji.chars().next().unwrap_or('🔗'),
            },
            QuickLinkIcon::SystemIcon(name) => IconSource::SystemIcon { name: name.clone() },
            QuickLinkIcon::CustomImage(path) => IconSource::FileIcon { path: path.clone() },
            QuickLinkIcon::Default => IconSource::Emoji { char: '🔗' },
        };

        // Build subtitle with alias and argument preview
        let mut subtitle = if arguments.is_empty() {
            link.link.clone()
        } else {
            // Show the URL with argument substituted for preview
            placeholder::substitute_argument(&link.link, arguments)
        };

        if let Some(ref alias) = link.alias {
            subtitle = format!("/{alias} · {subtitle}");
        }

        SearchResult {
            id: SearchResultId::new(format!("quicklink:{}", link.id.as_str())),
            title: link.name.clone(),
            subtitle,
            icon,
            result_type: ResultType::QuickLink,
            score,
            match_indices: indices,
            requires_permissions: false,
            action: SearchAction::ExecuteQuickLink {
                id: link.id.to_string(),
                url_template: link.link.clone(),
                arguments: arguments.to_string(),
            },
        }
    }
}

impl SearchProvider for QuickLinksProvider {
    fn name(&self) -> &'static str {
        "Quick Links"
    }

    fn result_type(&self) -> ResultType {
        ResultType::QuickLink
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        tracing::debug!(query = ?query, query_len = query.len(), "QuickLinks search started");

        if query.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = Vec::new();

        // Get quicklinks from cache (loads from storage on first call)
        let all_links = self.get_cached_links();

        let query_lower = query.to_lowercase();
        let query_parts: Vec<&str> = query.split_whitespace().collect();
        let first_word = query_parts.first().map_or("", |s| *s).to_lowercase();

        // 1. Check for exact alias match at start of query
        // e.g., "g test" -> alias "g", arguments "test"
        // Also match by name if the quicklink has argument placeholders but no alias
        for link in &all_links {
            let requires_input = placeholder::requires_user_input(&link.link);

            // Try alias first, then fall back to name if quicklink requires arguments
            let match_term = link.alias.as_ref().map(|a| a.to_lowercase()).or_else(|| {
                if requires_input {
                    Some(link.name.to_lowercase())
                } else {
                    None
                }
            });

            if let Some(term) = match_term {
                // Exact match with arguments (e.g., "g test" or "google test")
                if first_word == term && query_parts.len() > 1 {
                    let arguments = query_parts[1..].join(" ");
                    tracing::debug!(term = %term, arguments = %arguments, "Match with arguments");
                    results.push(Self::link_to_result(link, 10000.0, vec![], &arguments));
                }
                // Exact match without arguments
                else if query_lower == term {
                    tracing::debug!(term = %term, "Exact match");
                    results.push(Self::link_to_result(link, 9000.0, vec![], ""));
                }
            }
        }

        // If we have alias matches, return them immediately (highest priority)
        if !results.is_empty() {
            tracing::debug!(count = results.len(), "Returning alias matches");
            results.truncate(max_results);
            return results;
        }

        // 2. Fuzzy search on name, alias, and keywords
        let mut matcher = FuzzyMatcher::default();

        for link in &all_links {
            let mut best_score: Option<(u32, Vec<usize>)> = None;

            // Check name
            let name_lower = link.name.to_lowercase();
            if let Some((score, indices)) = matcher.score(&query_lower, &name_lower) {
                tracing::debug!(query = %query_lower, name = %name_lower, score, "Fuzzy match on name");
                best_score = Some((score, indices));
            }

            // Check alias (fuzzy)
            if let Some(ref alias) = link.alias {
                if let Some((score, indices)) = matcher.score(&query_lower, &alias.to_lowercase()) {
                    if best_score.as_ref().map_or(true, |(s, _)| score > *s) {
                        best_score = Some((score, indices));
                    }
                }
            }

            // Check keywords
            for keyword in &link.keywords {
                if let Some((score, indices)) = matcher.score(&query_lower, &keyword.to_lowercase())
                {
                    if best_score.as_ref().map_or(true, |(s, _)| score > *s) {
                        best_score = Some((score, indices));
                    }
                }
            }

            if let Some((score, indices)) = best_score {
                results.push(Self::link_to_result(link, f64::from(score), indices, ""));
            }
        }

        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(max_results);
        tracing::debug!(count = results.len(), query = %query, "QuickLinks search returning");
        results
    }
}

fn default_quicklinks_db_path() -> PathBuf {
    paths::data_dir().join("quicklinks.db")
}
