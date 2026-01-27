//! Window management search provider.

use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_window::commands::WindowCommandInfo;

/// Provides search results for window management commands.
#[derive(Debug)]
pub struct WindowProvider {
    matcher: FuzzyMatcher,
}

impl Default for WindowProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowProvider {
    /// Creates a new window provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matcher: FuzzyMatcher::default(),
        }
    }

    fn score_match(&mut self, query: &str, target: &str) -> Option<(u32, Vec<usize>)> {
        self.matcher.score(query, target)
    }
}

impl SearchProvider for WindowProvider {
    fn name(&self) -> &'static str {
        "Window Management"
    }

    fn result_type(&self) -> ResultType {
        ResultType::SystemCommand
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut matcher = FuzzyMatcher::default();
        let mut results = Vec::new();

        for cmd in WindowCommandInfo::all() {
            let score = matcher.score(query, cmd.name);
            if let Some((score, indices)) = score {
                results.push(SearchResult {
                    id: SearchResultId::new(format!("window:{}", cmd.id)),
                    title: cmd.name.to_string(),
                    subtitle: cmd.description.to_string(),
                    icon: IconSource::SystemIcon {
                        name: cmd.icon.to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: f64::from(score),
                    match_indices: indices,
                    requires_permissions: false,
                    action: SearchAction::ExecuteWindowCommand {
                        command_id: cmd.id.to_string(),
                    },
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(max_results);
        results
    }
}
