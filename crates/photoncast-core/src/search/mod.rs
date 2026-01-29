//! Search engine and providers.
//!
//! This module contains the search engine that orchestrates queries across
//! multiple providers (apps, commands, files).
//!
//! # Architecture
//!
//! The search system is organized as follows:
//!
//! - [`SearchEngine`] - Main orchestrator that dispatches queries to providers
//! - [`FuzzyMatcher`] - Wrapper around nucleo for fuzzy string matching
//! - [`SearchProvider`] - Trait for implementing search sources
//! - [`ResultRanker`] - Ranking algorithm with frecency support
//! - [`FileQuery`] - Natural language file query parser
//!
//! # Providers
//!
//! - [`AppProvider`] - Searches indexed applications
//! - [`CommandProvider`] - Searches system commands
//! - [`FileProvider`] - Searches files via Spotlight
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::{SearchEngine, AppProvider};
//!
//! let mut engine = SearchEngine::new();
//! engine.add_provider(AppProvider::new());
//!
//! // Perform a search
//! let results = engine.search_sync("safari");
//! for group in &results.groups {
//!     println!("{}: {} results", group.result_type.display_name(), group.results.len());
//! }
//! ```

pub mod engine;
pub mod file_index;
pub mod file_query;
pub mod fuzzy;
pub mod ignore_patterns;
pub mod index;
pub mod providers;
pub mod ranking;

#[cfg(target_os = "macos")]
pub mod spotlight;

use std::path::PathBuf;
use std::time::Duration;

pub use engine::{SearchConfig, SearchEngine};
pub use file_query::{FileCategory, FileQuery, FileTypeFilter};
pub use fuzzy::{FuzzyMatcher, MatcherConfig};
pub use ignore_patterns::{
    add_to_photonignore, pattern_for_file, IgnoreError, IgnoreMatcher, IgnorePattern,
    IgnorePatternSet,
};
pub use index::{
    EarlyTerminationConfig, IndexedAppEntry, NoUsageData, SearchIndex, UsageDataProvider,
    UsageRecord,
};
pub use providers::{
    AppProvider, CommandProvider, FileProvider, OptimizedAppProvider, SearchProvider,
};
pub use ranking::{BoostConfig, FrecencyScore, ResultRanker, UsageData};

/// Unique identifier for a search result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchResultId(String);

impl SearchResultId {
    /// Creates a new search result ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SearchResultId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A search result that can be displayed and activated.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Unique identifier for this result.
    pub id: SearchResultId,
    /// Display title.
    pub title: String,
    /// Subtitle/description.
    pub subtitle: String,
    /// Icon source.
    pub icon: IconSource,
    /// Type of result for grouping.
    pub result_type: ResultType,
    /// Match score (higher is better).
    pub score: f64,
    /// Indices of matched characters in the title.
    pub match_indices: Vec<usize>,
    /// Action to perform when activated.
    pub action: SearchAction,
    /// Whether this result requires permissions consent before execution.
    pub requires_permissions: bool,
}

/// Source of an icon for display.
#[derive(Debug, Clone)]
pub enum IconSource {
    /// Icon from an application bundle.
    AppIcon {
        /// Bundle identifier.
        bundle_id: String,
        /// Path to cached icon (if extracted).
        icon_path: Option<PathBuf>,
    },
    /// System icon by name.
    SystemIcon {
        /// Icon name.
        name: String,
    },
    /// File type icon.
    FileIcon {
        /// Path to the file.
        path: PathBuf,
    },
    /// Emoji character.
    Emoji {
        /// The emoji character.
        char: char,
    },
}

/// Type of search result for grouping and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResultType {
    /// Application result.
    Application,
    /// System command.
    SystemCommand,
    /// Quick link result.
    QuickLink,
    /// File result.
    File,
    /// Folder result.
    Folder,
    /// Custom command result.
    CustomCommand,
    /// Extension result.
    Extension,
}

impl ResultType {
    /// Returns the display name for this result type.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Application => "Apps",
            Self::SystemCommand => "Commands",
            Self::QuickLink => "Quick Links",
            Self::File => "Files",
            Self::Folder => "Folders",
            Self::CustomCommand => "Custom Commands",
            Self::Extension => "Extensions",
        }
    }

    /// Returns the priority for sorting groups (lower is higher priority).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Application => 0,
            Self::SystemCommand => 1,
            Self::QuickLink => 2,
            Self::CustomCommand => 3,
            Self::Extension => 4,
            Self::File => 5,
            Self::Folder => 6,
        }
    }
}

/// Action to perform when a search result is activated.
#[derive(Debug, Clone)]
pub enum SearchAction {
    /// Launch an application.
    LaunchApp {
        /// Bundle identifier.
        bundle_id: String,
        /// Path to the application.
        path: PathBuf,
    },
    /// Execute a system command.
    ExecuteCommand {
        /// Command identifier.
        command_id: String,
    },
    /// Open a file.
    OpenFile {
        /// Path to the file.
        path: PathBuf,
    },
    /// Reveal a file in Finder.
    RevealInFinder {
        /// Path to reveal.
        path: PathBuf,
    },
    /// Enter File Search Mode (triggered by "Search Files" command).
    EnterFileSearchMode,
    /// Quick Look preview a file (triggered by Cmd+Y in File Search Mode).
    QuickLookFile {
        /// Path to the file to preview.
        path: PathBuf,
    },
    /// Copy text to clipboard.
    CopyToClipboard {
        /// Text to copy.
        text: String,
    },
    /// Open a URL in the default browser.
    OpenUrl {
        /// URL to open.
        url: String,
    },
    /// Execute a quick link with URL template and arguments.
    ExecuteQuickLink {
        /// Quick link ID.
        id: String,
        /// URL template with placeholders.
        url_template: String,
        /// Arguments extracted from the query (space-separated after alias).
        arguments: String,
    },
    /// Open the quick links management UI.
    OpenQuickLinks,
    /// Open the sleep timer UI with an optional expression.
    OpenSleepTimer {
        /// Parsed or raw expression to schedule.
        expression: String,
    },
    /// Open the calendar view for a specific command.
    OpenCalendar {
        /// Calendar command ID.
        command_id: String,
    },
    /// Execute a window management command by ID.
    ExecuteWindowCommand {
        /// Window command ID.
        command_id: String,
    },
    /// Open app management UI for a specific command.
    OpenAppManagement {
        /// App management command ID.
        command_id: String,
    },
    /// Force quit an application by PID.
    ForceQuitApp {
        /// Process ID to force quit.
        pid: u32,
    },
    /// Execute a custom command.
    ExecuteCustomCommand {
        /// Custom command ID.
        command_id: String,
        /// Arguments to pass to the command.
        arguments: String,
    },
    /// Execute an extension command.
    ExecuteExtensionCommand {
        /// Extension ID.
        extension_id: String,
        /// Command ID within the extension.
        command_id: String,
    },
}

/// Grouped search results for display.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    /// Results grouped by type.
    pub groups: Vec<ResultGroup>,
    /// Total count of results across all groups.
    pub total_count: usize,
    /// The original query.
    pub query: String,
    /// Time taken to perform the search.
    pub search_time: Duration,
}

/// A group of search results of the same type.
#[derive(Debug, Clone)]
pub struct ResultGroup {
    /// The type of results in this group.
    pub result_type: ResultType,
    /// The results in this group.
    pub results: Vec<SearchResult>,
}

impl SearchResults {
    /// Creates an empty search results container.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            groups: Vec::new(),
            total_count: 0,
            query: String::new(),
            search_time: Duration::ZERO,
        }
    }

    /// Returns true if there are no results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_count == 0
    }

    /// Returns a flat iterator over all results.
    pub fn iter(&self) -> impl Iterator<Item = &SearchResult> {
        self.groups.iter().flat_map(|g| g.results.iter())
    }

    /// Returns the result at the given flat index across all groups.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&SearchResult> {
        self.iter().nth(index)
    }

    /// Returns the number of results in a specific group.
    #[must_use]
    pub fn group_count(&self, result_type: ResultType) -> usize {
        self.groups
            .iter()
            .find(|g| g.result_type == result_type)
            .map_or(0, |g| g.results.len())
    }

    /// Returns grouped results with shortcut indices for UI rendering.
    ///
    /// This method converts the internal groups into `GroupedResult` items
    /// with proper shortcut start indices calculated for each group.
    /// Groups are already sorted by priority (Apps → Commands → Files → Folders).
    #[must_use]
    pub fn grouped(&self) -> Vec<GroupedResult> {
        let mut result = Vec::with_capacity(self.groups.len());
        let mut shortcut_start: usize = 0;

        for group in &self.groups {
            result.push(GroupedResult {
                result_type: group.result_type,
                name: group.result_type.display_name(),
                items: group.results.clone(),
                shortcut_start,
            });
            shortcut_start += group.results.len();
        }

        result
    }

    /// Returns the group index containing the flat result index.
    ///
    /// Returns `None` if the index is out of bounds.
    #[must_use]
    pub fn group_index_for_result(&self, flat_index: usize) -> Option<usize> {
        let mut offset = 0;
        for (group_idx, group) in self.groups.iter().enumerate() {
            if flat_index < offset + group.results.len() {
                return Some(group_idx);
            }
            offset += group.results.len();
        }
        None
    }

    /// Returns the flat index of the first result in the given group.
    ///
    /// Returns `None` if the group index is out of bounds.
    #[must_use]
    pub fn first_index_in_group(&self, group_index: usize) -> Option<usize> {
        if group_index >= self.groups.len() {
            return None;
        }
        Some(
            self.groups[..group_index]
                .iter()
                .map(|g| g.results.len())
                .sum(),
        )
    }

    /// Returns the flat index of the first result in the next group.
    ///
    /// If the current selection is in the last group, wraps to the first group.
    /// Returns `None` if there are no groups.
    #[must_use]
    pub fn next_group_start(&self, current_flat_index: usize) -> Option<usize> {
        if self.groups.is_empty() {
            return None;
        }

        let current_group = self.group_index_for_result(current_flat_index)?;
        let next_group = (current_group + 1) % self.groups.len();
        self.first_index_in_group(next_group)
    }

    /// Returns the flat index of the first result in the previous group.
    ///
    /// If the current selection is in the first group, wraps to the last group.
    /// Returns `None` if there are no groups.
    #[must_use]
    pub fn previous_group_start(&self, current_flat_index: usize) -> Option<usize> {
        if self.groups.is_empty() {
            return None;
        }

        let current_group = self.group_index_for_result(current_flat_index)?;
        let prev_group = if current_group == 0 {
            self.groups.len() - 1
        } else {
            current_group - 1
        };
        self.first_index_in_group(prev_group)
    }
}

/// A grouped result with items for UI rendering.
#[derive(Debug, Clone)]
pub struct GroupedResult {
    /// The type of results in this group.
    pub result_type: ResultType,
    /// Display name for the group (e.g., "Apps", "Commands").
    pub name: &'static str,
    /// The items in this group.
    pub items: Vec<SearchResult>,
    /// Starting flat index for shortcuts (for ⌘1-9 calculation).
    pub shortcut_start: usize,
}

impl GroupedResult {
    /// Returns the shortcut hint for this group (e.g., "⌘1-5").
    ///
    /// Returns `None` if no shortcuts are available for this group
    /// (shortcut_start >= 9 or empty group).
    #[must_use]
    pub fn shortcut_hint(&self) -> Option<String> {
        if self.items.is_empty() || self.shortcut_start >= 9 {
            return None;
        }

        let end = (self.shortcut_start + self.items.len()).min(9);
        if end == self.shortcut_start + 1 {
            Some(format!("⌘{}", self.shortcut_start + 1))
        } else {
            Some(format!("⌘{}-{}", self.shortcut_start + 1, end))
        }
    }

    /// Returns the number of items in this group.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if this group is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// =============================================================================
// Search Event Handler Interface (Task 2.4.7)
// =============================================================================
//
// This section defines the interface for wiring search to the UI.
// The actual implementation will be in the UI layer, but these types
// define the contract between the search engine and the UI.

/// Event sent when search results are available.
#[derive(Debug, Clone)]
pub struct SearchCompletedEvent {
    /// The query that was searched.
    pub query: String,
    /// The search results.
    pub results: SearchResults,
}

/// Event sent when a search error occurs.
#[derive(Debug, Clone)]
pub struct SearchErrorEvent {
    /// The query that caused the error.
    pub query: String,
    /// Error message.
    pub error: String,
}

/// Trait for handling search events.
///
/// This trait should be implemented by UI components that need to
/// respond to search results.
pub trait SearchEventHandler: Send + Sync {
    /// Called when search results are available.
    fn on_search_completed(&self, event: SearchCompletedEvent);

    /// Called when a search error occurs.
    fn on_search_error(&self, event: SearchErrorEvent);
}

/// Pattern for async search tasks.
///
/// This module provides utilities for running searches asynchronously
/// and delivering results to the UI.
pub mod async_search {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Message types for the search task.
    #[derive(Debug)]
    pub enum SearchMessage {
        /// A new search query to execute.
        Query(String),
        /// Cancel the current search.
        Cancel,
        /// Shutdown the search task.
        Shutdown,
    }

    /// Result types from the search task.
    #[derive(Debug)]
    pub enum SearchResponse {
        /// Search completed successfully.
        Completed(SearchCompletedEvent),
        /// Search encountered an error.
        Error(SearchErrorEvent),
        /// Search was cancelled.
        Cancelled,
    }

    /// Creates a channel pair for search communication.
    ///
    /// Returns (sender for queries, receiver for results).
    #[must_use]
    pub fn create_search_channel(
        buffer_size: usize,
    ) -> (mpsc::Sender<SearchMessage>, mpsc::Receiver<SearchMessage>) {
        mpsc::channel(buffer_size)
    }

    /// Creates a channel pair for search responses.
    ///
    /// Returns (sender for results, receiver for UI).
    #[must_use]
    pub fn create_response_channel(
        buffer_size: usize,
    ) -> (mpsc::Sender<SearchResponse>, mpsc::Receiver<SearchResponse>) {
        mpsc::channel(buffer_size)
    }

    /// Spawns an async search task.
    ///
    /// This task listens for search queries and sends results back.
    /// It handles debouncing and cancellation internally.
    ///
    /// # Arguments
    ///
    /// * `engine` - The search engine to use.
    /// * `query_rx` - Receiver for search queries.
    /// * `result_tx` - Sender for search results.
    /// * `debounce_ms` - Debounce time in milliseconds.
    pub async fn run_search_task(
        engine: Arc<SearchEngine>,
        mut query_rx: mpsc::Receiver<SearchMessage>,
        result_tx: mpsc::Sender<SearchResponse>,
        debounce_ms: u64,
    ) {
        use tokio::time::{sleep, Duration};

        let debounce = Duration::from_millis(debounce_ms);
        let mut pending_query: Option<String> = None;

        loop {
            tokio::select! {
                // Check for new messages
                msg = query_rx.recv() => {
                    match msg {
                        Some(SearchMessage::Query(query)) => {
                            pending_query = Some(query);
                        }
                        Some(SearchMessage::Cancel) => {
                            pending_query = None;
                            let _ = result_tx.send(SearchResponse::Cancelled).await;
                        }
                        Some(SearchMessage::Shutdown) | None => {
                            break;
                        }
                    }
                }

                // Debounce timer
                () = sleep(debounce), if pending_query.is_some() => {
                    if let Some(query) = pending_query.take() {
                        let results = engine.search(&query).await;
                        let event = SearchCompletedEvent {
                            query,
                            results,
                        };
                        let _ = result_tx.send(SearchResponse::Completed(event)).await;
                    }
                }
            }
        }
    }
}
