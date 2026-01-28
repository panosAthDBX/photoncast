//! Custom commands search provider.
//!
//! This module provides fuzzy search capability over user-defined custom commands,
//! matching against command names, aliases, and keywords.

use parking_lot::RwLock;

use crate::custom_commands::{CustomCommand, CustomCommandStore, StoreError};
use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};

/// Provides search results for custom commands.
///
/// Uses an internal cache to avoid loading from database on every keystroke.
/// Call `invalidate_cache()` after modifying custom commands.
pub struct CustomCommandProvider {
    store: CustomCommandStore,
    /// Cached commands (loaded once and reused).
    cache: RwLock<Option<Vec<CustomCommand>>>,
}

impl std::fmt::Debug for CustomCommandProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomCommandProvider")
            .field("store", &"<CustomCommandStore>")
            .field("cache", &"<RwLock>")
            .finish()
    }
}

impl CustomCommandProvider {
    /// Creates a new custom command provider with the given store.
    #[must_use]
    pub fn new(store: CustomCommandStore) -> Self {
        Self {
            store,
            cache: RwLock::new(None),
        }
    }

    /// Creates a provider using the default store location.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be opened.
    pub fn with_default_store() -> Result<Self, StoreError> {
        let store = CustomCommandStore::open_default()?;
        Ok(Self::new(store))
    }

    /// Creates a provider with an in-memory store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be created.
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, StoreError> {
        let store = CustomCommandStore::open_in_memory()?;
        Ok(Self::new(store))
    }

    /// Invalidates the cache, forcing a reload on next search.
    ///
    /// Call this after adding, updating, or deleting custom commands.
    pub fn invalidate_cache(&self) {
        *self.cache.write() = None;
        tracing::debug!("Custom commands cache invalidated");
    }

    /// Gets custom commands from cache, loading from storage if needed.
    fn get_cached_commands(&self) -> Vec<CustomCommand> {
        // Try to read from cache first
        {
            let cache = self.cache.read();
            if let Some(ref commands) = *cache {
                return commands.clone();
            }
        }

        // Cache miss - load from storage
        let commands = match self.store.list_enabled() {
            Ok(commands) => {
                tracing::debug!(
                    count = commands.len(),
                    "Loaded custom commands from storage (cache miss)"
                );
                commands
            },
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load custom commands");
                return Vec::new();
            },
        };

        // Store in cache
        *self.cache.write() = Some(commands.clone());

        commands
    }

    /// Returns a reference to the underlying store.
    #[must_use]
    pub fn store(&self) -> &CustomCommandStore {
        &self.store
    }

    /// Converts a CustomCommand to a SearchResult.
    fn command_to_result(
        command: &CustomCommand,
        score: f64,
        indices: Vec<usize>,
        arguments: &str,
    ) -> SearchResult {
        // Determine icon
        let icon = match &command.icon {
            Some(icon_name) if icon_name.chars().next().is_some_and(|c| c.is_ascii()) => {
                // Treat as system icon name
                IconSource::SystemIcon {
                    name: icon_name.clone(),
                }
            },
            Some(emoji) => {
                // Treat as emoji
                IconSource::Emoji {
                    char: emoji.chars().next().unwrap_or('⚡'),
                }
            },
            None => IconSource::SystemIcon {
                name: "terminal".to_string(),
            },
        };

        // Build subtitle
        let mut subtitle_parts = Vec::new();

        // Add alias if present
        if let Some(ref alias) = command.alias {
            subtitle_parts.push(format!("/{alias}"));
        }

        // Add command preview (first part, no args)
        let command_preview = if command.command.len() > 50 {
            format!("{}...", &command.command[..47])
        } else {
            command.command.clone()
        };
        subtitle_parts.push(command_preview);

        let subtitle = subtitle_parts.join(" · ");

        SearchResult {
            id: SearchResultId::new(format!("custom_command:{}", command.id)),
            title: command.name.clone(),
            subtitle,
            icon,
            result_type: ResultType::CustomCommand,
            score,
            match_indices: indices,
            requires_permissions: false,
                    action: SearchAction::ExecuteCustomCommand {
                command_id: command.id.clone(),
                arguments: arguments.to_string(),
            },
        }
    }
}

impl SearchProvider for CustomCommandProvider {
    fn name(&self) -> &'static str {
        "Custom Commands"
    }

    fn result_type(&self) -> ResultType {
        ResultType::CustomCommand
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        tracing::debug!(query = ?query, query_len = query.len(), "Custom commands search started");

        if query.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = Vec::new();
        let commands = self.get_cached_commands();

        let query_lower = query.to_lowercase();
        let query_parts: Vec<&str> = query.split_whitespace().collect();
        let first_word = query_parts.first().map_or("", |s| *s).to_lowercase();

        // 1. Check for exact alias match at start of query
        // e.g., "gc commit message" -> alias "gc", arguments "commit message"
        for command in &commands {
            if let Some(ref alias) = command.alias {
                let alias_lower = alias.to_lowercase();

                // Exact alias with arguments
                if first_word == alias_lower && query_parts.len() > 1 {
                    let arguments = query_parts[1..].join(" ");
                    tracing::debug!(
                        alias = %alias,
                        arguments = %arguments,
                        "Alias match with arguments"
                    );
                    results.push(Self::command_to_result(
                        command,
                        10000.0,
                        vec![],
                        &arguments,
                    ));
                }
                // Exact alias without arguments
                else if query_lower == alias_lower {
                    tracing::debug!(alias = %alias, "Exact alias match");
                    results.push(Self::command_to_result(command, 9000.0, vec![], ""));
                }
            }
        }

        // If we have alias matches, return them immediately
        if !results.is_empty() {
            tracing::debug!(count = results.len(), "Returning alias matches");
            results.truncate(max_results);
            return results;
        }

        // 2. Fuzzy search on name and keywords
        let mut matcher = FuzzyMatcher::default();

        for command in &commands {
            let mut best_score: Option<(u32, Vec<usize>)> = None;

            // Match on name
            let name_lower = command.name.to_lowercase();
            if let Some((score, indices)) = matcher.score(&query_lower, &name_lower) {
                // Boost name matches
                let boosted_score = score.saturating_add(100);
                tracing::debug!(
                    query = %query_lower,
                    name = %name_lower,
                    score = boosted_score,
                    "Fuzzy match on name"
                );
                best_score = Some((boosted_score, indices));
            }

            // Match on alias (fuzzy, in case partial typing)
            if let Some(ref alias) = command.alias {
                let alias_lower = alias.to_lowercase();
                if let Some((score, indices)) = matcher.score(&query_lower, &alias_lower) {
                    if best_score.as_ref().map_or(true, |(s, _)| score > *s) {
                        best_score = Some((score, indices));
                    }
                }
            }

            // Match on keywords
            for keyword in &command.keywords {
                let keyword_lower = keyword.to_lowercase();
                if let Some((score, indices)) = matcher.score(&query_lower, &keyword_lower) {
                    if best_score.as_ref().map_or(true, |(s, _)| score > *s) {
                        best_score = Some((score, indices));
                    }
                }
            }

            if let Some((score, indices)) = best_score {
                results.push(Self::command_to_result(
                    command,
                    f64::from(score),
                    indices,
                    "",
                ));
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.total_cmp(&a.score));

        results.truncate(max_results);
        tracing::debug!(count = results.len(), query = %query, "Custom commands search returning");
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider() -> CustomCommandProvider {
        let store = CustomCommandStore::open_in_memory().expect("should open in-memory store");

        // Add some test commands
        store
            .create(
                &CustomCommand::builder("Git Commit", "git commit -m {query}")
                    .alias("gc")
                    .keywords(vec!["version control".to_string(), "save".to_string()])
                    .build(),
            )
            .expect("should create");

        store
            .create(
                &CustomCommand::builder("Open in VS Code", "code {query}")
                    .alias("vsc")
                    .keywords(vec!["editor".to_string(), "ide".to_string()])
                    .icon("terminal")
                    .build(),
            )
            .expect("should create");

        store
            .create(
                &CustomCommand::builder("List Files", "ls -la")
                    .keywords(vec!["directory".to_string()])
                    .build(),
            )
            .expect("should create");

        CustomCommandProvider::new(store)
    }

    #[test]
    fn test_search_by_name() {
        let provider = create_test_provider();
        let results = provider.search("Git Commit", 10);

        assert!(!results.is_empty(), "should find results");
        assert_eq!(results[0].title, "Git Commit");
    }

    #[test]
    fn test_search_by_alias_exact() {
        let provider = create_test_provider();
        let results = provider.search("gc", 10);

        assert!(!results.is_empty(), "should find results");
        assert_eq!(results[0].title, "Git Commit");
    }

    #[test]
    fn test_search_by_alias_with_args() {
        let provider = create_test_provider();
        let results = provider.search("gc initial commit", 10);

        assert!(!results.is_empty(), "should find results");
        assert_eq!(results[0].title, "Git Commit");

        match &results[0].action {
            SearchAction::ExecuteCustomCommand { arguments, .. } => {
                assert_eq!(arguments, "initial commit");
            },
            _ => panic!("expected ExecuteCustomCommand action"),
        }
    }

    #[test]
    fn test_search_by_keyword() {
        let provider = create_test_provider();
        let results = provider.search("version control", 10);

        assert!(!results.is_empty(), "should find results");
        assert_eq!(results[0].title, "Git Commit");
    }

    #[test]
    fn test_search_fuzzy() {
        let provider = create_test_provider();
        let results = provider.search("git", 10);

        assert!(!results.is_empty(), "should find results for partial match");
        assert!(results.iter().any(|r| r.title == "Git Commit"));
    }

    #[test]
    fn test_search_empty_query() {
        let provider = create_test_provider();
        let results = provider.search("", 10);

        assert!(results.is_empty(), "empty query should return no results");
    }

    #[test]
    fn test_search_no_match() {
        let provider = create_test_provider();
        let results = provider.search("nonexistent", 10);

        assert!(
            results.is_empty(),
            "nonexistent query should return no results"
        );
    }

    #[test]
    fn test_search_respects_max_results() {
        let provider = create_test_provider();
        let results = provider.search("c", 1);

        assert!(results.len() <= 1, "should respect max_results limit");
    }

    #[test]
    fn test_result_type() {
        let provider = create_test_provider();
        let results = provider.search("git", 10);

        assert!(!results.is_empty());
        assert_eq!(results[0].result_type, ResultType::CustomCommand);
    }

    #[test]
    fn test_result_id_format() {
        let provider = create_test_provider();
        let results = provider.search("git", 10);

        assert!(!results.is_empty());
        assert!(
            results[0].id.as_str().starts_with("custom_command:"),
            "result ID should have custom_command: prefix"
        );
    }

    #[test]
    fn test_invalidate_cache() {
        let provider = create_test_provider();

        // First search to populate cache
        let results1 = provider.search("git", 10);
        assert!(!results1.is_empty());

        // Add a new command
        provider
            .store()
            .create(&CustomCommand::new("New Command", "echo new"))
            .expect("should create");

        // Search again (should still use cached data)
        let _results2 = provider.search("new", 10);
        // May or may not find "new" depending on cache state

        // Invalidate and search again
        provider.invalidate_cache();
        let results3 = provider.search("new", 10);
        assert!(
            !results3.is_empty(),
            "should find new command after cache invalidation"
        );
    }

    #[test]
    fn test_provider_name() {
        let provider = create_test_provider();
        assert_eq!(provider.name(), "Custom Commands");
    }

    #[test]
    fn test_provider_result_type() {
        let provider = create_test_provider();
        assert_eq!(provider.result_type(), ResultType::CustomCommand);
    }

    #[test]
    fn test_disabled_commands_not_returned() {
        let store = CustomCommandStore::open_in_memory().expect("should open store");

        let mut cmd = CustomCommand::new("Test Disabled", "echo disabled");
        cmd.enabled = false;
        store.create(&cmd).expect("should create");

        store
            .create(&CustomCommand::new("Test Enabled", "echo enabled"))
            .expect("should create");

        let provider = CustomCommandProvider::new(store);
        // Search for "test" to match both command names
        let results = provider.search("test", 10);

        // Only enabled command should be returned
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Enabled");
    }

    #[test]
    fn test_icon_handling() {
        let store = CustomCommandStore::open_in_memory().expect("should open store");

        // Command with system icon
        store
            .create(
                &CustomCommand::builder("Test With Icon", "echo 1")
                    .icon("terminal")
                    .build(),
            )
            .expect("should create");

        // Command with emoji
        store
            .create(
                &CustomCommand::builder("Test With Emoji", "echo 2")
                    .icon("🚀")
                    .build(),
            )
            .expect("should create");

        // Command without icon
        store
            .create(&CustomCommand::new("Test No Icon", "echo 3"))
            .expect("should create");

        let provider = CustomCommandProvider::new(store);
        // Search for "test" to match all command names
        let results = provider.search("test", 10);

        assert_eq!(results.len(), 3);

        // Check icon types
        let with_icon = results.iter().find(|r| r.title == "Test With Icon").unwrap();
        assert!(matches!(with_icon.icon, IconSource::SystemIcon { .. }));

        let with_emoji = results.iter().find(|r| r.title == "Test With Emoji").unwrap();
        assert!(matches!(with_emoji.icon, IconSource::Emoji { .. }));

        let no_icon = results.iter().find(|r| r.title == "Test No Icon").unwrap();
        assert!(matches!(no_icon.icon, IconSource::SystemIcon { .. })); // Default terminal icon
    }
}
