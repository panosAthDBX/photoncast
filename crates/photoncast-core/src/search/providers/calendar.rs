//! Calendar commands search provider.

use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_calendar::commands::CalendarCommandInfo;

/// Provides search results for calendar commands.
#[derive(Debug)]
pub struct CalendarProvider;

impl Default for CalendarProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CalendarProvider {
    /// Creates a new calendar provider.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SearchProvider for CalendarProvider {
    fn name(&self) -> &'static str {
        "Calendar"
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

        for cmd in CalendarCommandInfo::all() {
            if let Some((score, indices)) = matcher.score(query, cmd.name) {
                results.push(SearchResult {
                    id: SearchResultId::new(format!("calendar:{}", cmd.id)),
                    title: cmd.name.to_string(),
                    subtitle: cmd.description.to_string(),
                    icon: IconSource::SystemIcon {
                        name: cmd.icon.to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: f64::from(score),
                    match_indices: indices,
                    requires_permissions: false,
                    action: SearchAction::OpenCalendar {
                        command_id: cmd.id.to_string(),
                    },
                });
            }
        }

        results.sort_by(|a, b| b.score.total_cmp(&a.score));
        results.truncate(max_results);
        results
    }
}
