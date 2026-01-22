//! Sleep timer search provider.

use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_timer::commands::TimerCommand;
use photoncast_timer::parser::parse_timer_expression;
use photoncast_timer::scheduler::TimerAction;

/// Provides search results for timer commands.
#[derive(Debug)]
pub struct TimerProvider {
    matcher: FuzzyMatcher,
}

impl Default for TimerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerProvider {
    /// Creates a new timer provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matcher: FuzzyMatcher::default(),
        }
    }

    #[allow(dead_code)]
    fn score_match(&mut self, query: &str, target: &str) -> Option<(u32, Vec<usize>)> {
        self.matcher.score(query, target)
    }

    /// Get command info for a parsed timer action
    fn command_for_action(action: &TimerAction) -> Option<TimerCommand> {
        TimerCommand::all()
            .into_iter()
            .find(|cmd| cmd.action == *action)
    }
}

fn command_action(command: &TimerCommand, query: &str, is_fuzzy_match: bool) -> SearchAction {
    if command.name == "Cancel Timer" {
        SearchAction::OpenSleepTimer {
            expression: "cancel".to_string(),
        }
    } else if command.name == "Show Timer" {
        SearchAction::OpenSleepTimer {
            expression: "status".to_string(),
        }
    } else if is_fuzzy_match {
        // For fuzzy matches (like typing "timer"), show status instead of trying to parse
        SearchAction::OpenSleepTimer {
            expression: "status".to_string(),
        }
    } else {
        SearchAction::OpenSleepTimer {
            expression: query.to_string(),
        }
    }
}

impl SearchProvider for TimerProvider {
    fn name(&self) -> &str {
        "Sleep Timer"
    }

    fn result_type(&self) -> ResultType {
        ResultType::SystemCommand
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // Check for special commands: cancel and show
        if query_lower.contains("cancel") || query_lower.contains("stop") {
            results.push(SearchResult {
                id: SearchResultId::new("sleep_timer:Cancel Timer"),
                title: "Cancel Timer".to_string(),
                subtitle: "Cancel active timer".to_string(),
                icon: IconSource::SystemIcon {
                    name: "x-circle".to_string(),
                },
                result_type: ResultType::SystemCommand,
                score: 10000.0,
                match_indices: vec![],
                action: SearchAction::OpenSleepTimer {
                    expression: "cancel".to_string(),
                },
            });
            return results;
        }

        if query_lower.contains("show")
            || query_lower.contains("status")
            || query_lower == "active timer"
        {
            results.push(SearchResult {
                id: SearchResultId::new("sleep_timer:Show Timer"),
                title: "Show Timer".to_string(),
                subtitle: "View active timer status".to_string(),
                icon: IconSource::SystemIcon {
                    name: "clock".to_string(),
                },
                result_type: ResultType::SystemCommand,
                score: 10000.0,
                match_indices: vec![],
                action: SearchAction::OpenSleepTimer {
                    expression: "status".to_string(),
                },
            });
            return results;
        }

        // Check if the query is a parseable timer expression
        // This handles queries like "lock in 30 sec", "sleep in 5 minutes", etc.
        if let Ok(parsed) = parse_timer_expression(query) {
            if let Some(cmd) = Self::command_for_action(&parsed.action) {
                let formatted_time = parsed
                    .execute_at
                    .with_timezone(&chrono::Local)
                    .format("%H:%M:%S");
                results.push(SearchResult {
                    id: SearchResultId::new(format!("sleep_timer:{}", cmd.name)),
                    title: cmd.name.to_string(),
                    subtitle: format!("{} at {}", cmd.description, formatted_time),
                    icon: IconSource::SystemIcon {
                        name: cmd.icon.to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: 10000.0, // High score for exact timer expressions
                    match_indices: vec![],
                    action: SearchAction::OpenSleepTimer {
                        expression: query.to_string(),
                    },
                });
                return results;
            }
        }

        // Fall back to fuzzy matching for partial queries
        let mut matcher = FuzzyMatcher::default();

        for cmd in TimerCommand::all() {
            let mut best_score = matcher
                .score(query, cmd.name)
                .map(|(score, indices)| (score.saturating_add(100), indices));

            for example in cmd.examples {
                if let Some((score, indices)) = matcher.score(query, example) {
                    match &best_score {
                        Some((best, _)) if score <= *best => {},
                        _ => best_score = Some((score, indices)),
                    }
                }
            }

            if let Some((score, indices)) = best_score {
                let action = command_action(&cmd, query, true);

                // For fuzzy matches, show example in subtitle
                let subtitle = if !cmd.examples.is_empty() {
                    format!("{} (e.g., \"{}\")", cmd.description, cmd.examples[0])
                } else {
                    cmd.description.to_string()
                };

                results.push(SearchResult {
                    id: SearchResultId::new(format!("sleep_timer:{}", cmd.name)),
                    title: cmd.name.to_string(),
                    subtitle,
                    icon: IconSource::SystemIcon {
                        name: cmd.icon.to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: f64::from(score),
                    match_indices: indices,
                    action,
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
