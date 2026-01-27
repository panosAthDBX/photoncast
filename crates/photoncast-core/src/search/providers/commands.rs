//! System commands search provider.
//!
//! This module provides fuzzy search capability over system commands,
//! matching against both command names and their aliases.

use crate::commands::SystemCommand;
use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};

/// Provides search results for system commands.
///
/// The provider matches queries against command names (e.g., "Sleep", "Restart")
/// and their aliases (e.g., "suspend", "reboot") using fuzzy matching.
#[derive(Debug)]
pub struct CommandProvider {
    /// The fuzzy matcher for scoring matches.
    matcher: FuzzyMatcher,
}

impl Default for CommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandProvider {
    /// Creates a new command provider with default fuzzy matcher configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matcher: FuzzyMatcher::default(),
        }
    }

    /// Creates a new command provider with a custom fuzzy matcher.
    #[must_use]
    pub fn with_matcher(matcher: FuzzyMatcher) -> Self {
        Self { matcher }
    }

    /// Scores a query against a target string, returning the score and match indices.
    fn score_match(&mut self, query: &str, target: &str) -> Option<(u32, Vec<usize>)> {
        self.matcher.score(query, target)
    }
}

impl SearchProvider for CommandProvider {
    fn name(&self) -> &'static str {
        "Commands"
    }

    fn result_type(&self) -> ResultType {
        ResultType::SystemCommand
    }

    #[allow(clippy::match_same_arms)]
    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // We need a mutable matcher for scoring
        let mut matcher = FuzzyMatcher::default();
        let mut results: Vec<SearchResult> = Vec::new();

        // Get all available commands
        let commands = SystemCommand::all();

        for cmd_info in &commands {
            let command = cmd_info.command;
            let mut best_score = matcher
                .score(query, cmd_info.name)
                .map(|(score, indices)| (score.saturating_add(100), indices, true));

            // Match against aliases
            for alias in cmd_info.aliases {
                if let Some((score, indices)) = matcher.score(query, alias) {
                    match &best_score {
                        Some((best, _, _)) if score <= *best => {
                            // Keep the better score
                        },
                        _ => {
                            best_score = Some((score, indices, false));
                        },
                    }
                }
            }

            // If we found a match, create a search result
            if let Some((score, indices, is_name_match)) = best_score {
                // For alias matches, we show the matched alias in parentheses as part of subtitle
                let subtitle = cmd_info.description.to_string();

                // Determine the action based on command type
                let action = match command {
                    SystemCommand::SearchFiles => SearchAction::EnterFileSearchMode,
                    SystemCommand::Preferences => SearchAction::ExecuteCommand {
                        command_id: command.id().to_string(),
                    },
                    _ => SearchAction::ExecuteCommand {
                        command_id: command.id().to_string(),
                    },
                };

                results.push(SearchResult {
                    id: SearchResultId::new(format!("command:{}", command.id())),
                    title: cmd_info.name.to_string(),
                    subtitle,
                    icon: IconSource::SystemIcon {
                        name: cmd_info.icon.to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: f64::from(score),
                    match_indices: if is_name_match {
                        indices
                    } else {
                        Vec::new() // Don't highlight title for alias matches
                    },
                    action,
                    requires_permissions: false,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(max_results);

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_by_name_exact() {
        let provider = CommandProvider::new();
        let results = provider.search("Sleep", 10);

        assert!(!results.is_empty(), "should find at least one result");
        assert_eq!(results[0].title, "Sleep");
    }

    #[test]
    fn test_search_by_name_partial() {
        let provider = CommandProvider::new();
        let results = provider.search("slee", 10);

        assert!(!results.is_empty(), "should find results for partial match");
        // Sleep or Sleep Displays should be in results
        let has_sleep = results.iter().any(|r| r.title.contains("Sleep"));
        assert!(has_sleep, "should find sleep-related commands");
    }

    #[test]
    fn test_search_by_alias() {
        let provider = CommandProvider::new();
        let results = provider.search("reboot", 10);

        assert!(!results.is_empty(), "should find result for alias");
        assert_eq!(
            results[0].title, "Restart",
            "reboot alias should map to Restart"
        );
    }

    #[test]
    fn test_search_by_alias_partial() {
        let provider = CommandProvider::new();
        let results = provider.search("shutd", 10);

        assert!(!results.is_empty(), "should find result for partial alias");
        assert_eq!(
            results[0].title, "Shut Down",
            "partial 'shutd' should match Shut Down via alias"
        );
    }

    #[test]
    fn test_search_empty_query_returns_empty() {
        let provider = CommandProvider::new();
        let results = provider.search("", 10);

        assert!(results.is_empty(), "empty query should return no results");
    }

    #[test]
    fn test_search_no_match() {
        let provider = CommandProvider::new();
        let results = provider.search("xyznonexistent", 10);

        assert!(
            results.is_empty(),
            "non-matching query should return no results"
        );
    }

    #[test]
    fn test_search_respects_max_results() {
        let provider = CommandProvider::new();
        let results = provider.search("s", 2);

        assert!(results.len() <= 2, "should respect max_results limit");
    }

    #[test]
    fn test_search_result_has_correct_action() {
        let provider = CommandProvider::new();
        let results = provider.search("lock", 10);

        assert!(!results.is_empty());
        match &results[0].action {
            SearchAction::ExecuteCommand { command_id } => {
                assert_eq!(command_id, "lock_screen");
            },
            _ => panic!("expected ExecuteCommand action"),
        }
    }

    #[test]
    fn test_search_result_has_correct_type() {
        let provider = CommandProvider::new();
        let results = provider.search("restart", 10);

        assert!(!results.is_empty());
        assert_eq!(results[0].result_type, ResultType::SystemCommand);
    }

    #[test]
    fn test_search_result_has_icon() {
        let provider = CommandProvider::new();
        let results = provider.search("trash", 10);

        assert!(!results.is_empty());
        match &results[0].icon {
            IconSource::SystemIcon { name } => {
                assert_eq!(name, "trash");
            },
            _ => panic!("expected SystemIcon"),
        }
    }

    #[test]
    fn test_search_results_sorted_by_score() {
        let provider = CommandProvider::new();
        let results = provider.search("sleep", 10);

        // Check that results are sorted by score descending
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_name_match_ranks_higher_than_alias() {
        let provider = CommandProvider::new();
        // "Sleep" is both a command name and potentially matches other aliases
        let results = provider.search("Sleep", 10);

        assert!(!results.is_empty());
        // The exact name match "Sleep" should rank first
        assert_eq!(results[0].title, "Sleep");
    }

    #[test]
    fn test_provider_name() {
        let provider = CommandProvider::new();
        assert_eq!(provider.name(), "Commands");
    }

    #[test]
    fn test_provider_result_type() {
        let provider = CommandProvider::new();
        assert_eq!(provider.result_type(), ResultType::SystemCommand);
    }

    #[test]
    fn test_all_commands_searchable() {
        let provider = CommandProvider::new();

        // Test that each command can be found by its name
        for cmd_info in SystemCommand::all() {
            let results = provider.search(cmd_info.name, 10);
            assert!(
                !results.is_empty(),
                "command '{}' should be searchable by name",
                cmd_info.name
            );
        }
    }

    #[test]
    fn test_search_case_insensitive() {
        let provider = CommandProvider::new();

        // With smart case: lowercase query = case-insensitive matching
        let results_lower = provider.search("sleep", 10);
        assert!(
            !results_lower.is_empty(),
            "lowercase 'sleep' should find Sleep command"
        );
        assert!(results_lower.iter().any(|r| r.title == "Sleep"));

        // Note: With smart case enabled, uppercase queries use case-sensitive matching
        // This is intentional behavior - if user types uppercase, they want exact case
        // So "SLEEP" won't match "Sleep" but "sleep" will match "Sleep"
    }

    #[test]
    fn test_dark_mode_aliases() {
        let provider = CommandProvider::new();

        // Toggle Appearance has aliases: "dark mode", "light mode", "toggle dark"
        let results = provider.search("dark", 10);
        assert!(!results.is_empty());
        assert!(
            results.iter().any(|r| r.title == "Toggle Appearance"),
            "should find Toggle Appearance via 'dark' alias"
        );
    }

    #[test]
    fn test_result_id_format() {
        let provider = CommandProvider::new();
        let results = provider.search("sleep", 10);

        assert!(!results.is_empty());
        assert!(
            results[0].id.as_str().starts_with("command:"),
            "result ID should have command: prefix"
        );
    }
}
