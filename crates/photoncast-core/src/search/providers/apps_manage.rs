//! App management search provider.

use crate::search::providers::SearchProvider;
use crate::search::{
    FuzzyMatcher, IconSource, ResultType, SearchAction, SearchResult, SearchResultId,
};
use photoncast_apps::models::RunningApp;
use photoncast_apps::AppManager;

/// Provides search results for app management actions.
#[derive(Debug)]
pub struct AppsProvider {
    matcher: FuzzyMatcher,
    manager: AppManager,
}

impl AppsProvider {
    /// Creates a new app management provider.
    #[must_use]
    pub fn new() -> Self {
        Self {
            matcher: FuzzyMatcher::default(),
            manager: AppManager::new(photoncast_apps::AppsConfig::default()),
        }
    }
}

impl Default for AppsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchProvider for AppsProvider {
    fn name(&self) -> &str {
        "App Management"
    }

    fn result_type(&self) -> ResultType {
        ResultType::SystemCommand
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut matcher = FuzzyMatcher::default();

        let generic_commands = [
            ("Manage Apps", "Open app management", "app-management"),
            (
                "Uninstall App",
                "Remove installed application",
                "app-uninstall",
            ),
            ("Running Apps", "List running applications", "app-running"),
            ("App Sleep", "Manage app sleep settings", "app-sleep"),
        ];

        for (title, description, id) in generic_commands {
            if let Some((score, indices)) = matcher.score(query, title) {
                results.push(SearchResult {
                    id: SearchResultId::new(format!("appmgmt:{}", id)),
                    title: title.to_string(),
                    subtitle: description.to_string(),
                    icon: IconSource::SystemIcon {
                        name: "app".to_string(),
                    },
                    result_type: ResultType::SystemCommand,
                    score: f64::from(score),
                    match_indices: indices,
                    action: SearchAction::OpenAppManagement {
                        command_id: id.to_string(),
                    },
                });
            }
        }

        if let Ok(running) = self.manager.get_running_apps() {
            for app in running {
                if let Some((score, indices)) = matcher.score(query, &app.name) {
                    results.push(running_app_result(&app, score, indices));
                }
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

fn running_app_result(app: &RunningApp, score: u32, indices: Vec<usize>) -> SearchResult {
    let subtitle = if let Some(bundle_id) = &app.bundle_id {
        bundle_id.clone()
    } else {
        format!("PID {}", app.pid)
    };

    SearchResult {
        id: SearchResultId::new(format!("appmgmt:pid:{}", app.pid)),
        title: app.name.clone(),
        subtitle,
        icon: IconSource::Emoji { char: '🧭' },
        result_type: ResultType::SystemCommand,
        score: f64::from(score),
        match_indices: indices,
        action: SearchAction::ForceQuitApp { pid: app.pid },
    }
}
