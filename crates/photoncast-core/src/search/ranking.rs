//! Result ranking algorithm.
//!
//! This module provides ranking functionality for search results, combining:
//! - Match quality (from nucleo fuzzy matching)
//! - Frecency (frequency + recency)
//! - Boost factors (path-based and match-type boosts)
//! - Tiebreaker logic for deterministic ordering

use std::cmp::Ordering;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::search::SearchResult;

// =============================================================================
// Frecency Score (Task 2.5.2)
// =============================================================================

/// Score combining frequency and recency for ranking.
///
/// Frecency is calculated as `frequency * recency_decay`, where:
/// - `frequency` is the total number of times an item was used
/// - `recency_decay` is an exponential decay factor based on time since last use
///
/// The decay uses a half-life of 72 hours, meaning an item used 72 hours ago
/// has half the recency weight of an item used just now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrecencyScore {
    /// Total action count (launches, executions, etc.).
    pub frequency: u32,
    /// Recency decay factor (0.0 to 1.0).
    pub recency: f64,
}

impl FrecencyScore {
    /// Half-life for recency decay in hours.
    pub const HALF_LIFE_HOURS: f64 = 72.0;

    /// Calculates a frecency score from usage data.
    ///
    /// # Arguments
    ///
    /// * `launch_count` - Total number of times the item was used.
    /// * `last_used` - When the item was last used. If `None`, recency is 0.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use photoncast_core::search::ranking::FrecencyScore;
    ///
    /// // Item used 5 times, last used now
    /// let score = FrecencyScore::calculate(5, Some(SystemTime::now()));
    /// assert!(score.score() > 0.0);
    ///
    /// // Never used item
    /// let zero = FrecencyScore::calculate(0, None);
    /// assert_eq!(zero.score(), 0.0);
    /// ```
    #[must_use]
    pub fn calculate(launch_count: u32, last_used: Option<SystemTime>) -> Self {
        let recency = last_used
            .map(|t| {
                let elapsed = SystemTime::now()
                    .duration_since(t)
                    .unwrap_or(Duration::ZERO);
                let hours = elapsed.as_secs_f64() / 3600.0;
                0.5_f64.powf(hours / Self::HALF_LIFE_HOURS)
            })
            .unwrap_or(0.0);

        Self {
            frequency: launch_count,
            recency,
        }
    }

    /// Creates a frecency score with explicit recency value.
    ///
    /// This is useful for testing or when the recency is calculated externally.
    #[must_use]
    pub const fn new(frequency: u32, recency: f64) -> Self {
        Self { frequency, recency }
    }

    /// Returns the combined frecency score.
    ///
    /// Formula: `frequency * recency`
    #[must_use]
    pub fn score(&self) -> f64 {
        f64::from(self.frequency) * self.recency
    }

    /// Creates a zero frecency score (for never-used items).
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            frequency: 0,
            recency: 0.0,
        }
    }
}

impl Default for FrecencyScore {
    fn default() -> Self {
        Self::zero()
    }
}

// =============================================================================
// Boost Configuration (Task 2.5.3)
// =============================================================================

/// Configuration for boost factors.
///
/// Boosts are multipliers applied to the final score based on:
/// - **Path boosts**: Prefer system and user applications
/// - **Match boosts**: Prefer exact and prefix matches
#[derive(Debug, Clone, PartialEq)]
pub struct BoostConfig {
    /// Boost for system applications (1.2x default for /System/Applications).
    pub system_app_boost: f64,
    /// Boost for user applications (1.1x default for /Applications).
    pub applications_boost: f64,
    /// Boost for exact name matches (2.0x default).
    pub exact_match_boost: f64,
    /// Boost for prefix matches (1.5x default).
    pub prefix_match_boost: f64,
}

impl Default for BoostConfig {
    fn default() -> Self {
        Self {
            system_app_boost: 1.2,
            applications_boost: 1.1,
            exact_match_boost: 2.0,
            prefix_match_boost: 1.5,
        }
    }
}

impl BoostConfig {
    /// Creates a new boost config with custom values.
    #[must_use]
    pub const fn new(
        system_app_boost: f64,
        applications_boost: f64,
        exact_match_boost: f64,
        prefix_match_boost: f64,
    ) -> Self {
        Self {
            system_app_boost,
            applications_boost,
            exact_match_boost,
            prefix_match_boost,
        }
    }

    /// Creates a config with no boosts (all multipliers = 1.0).
    #[must_use]
    pub const fn no_boosts() -> Self {
        Self {
            system_app_boost: 1.0,
            applications_boost: 1.0,
            exact_match_boost: 1.0,
            prefix_match_boost: 1.0,
        }
    }
}

// =============================================================================
// Usage Data (for tiebreaking)
// =============================================================================

/// Usage data for an item, used in tiebreaking.
#[derive(Debug, Clone, Default)]
pub struct UsageData {
    /// Total usage count.
    pub usage_count: u32,
    /// Last used timestamp (for recency tiebreaking).
    pub last_used: Option<SystemTime>,
}

impl UsageData {
    /// Creates new usage data.
    #[must_use]
    pub const fn new(usage_count: u32, last_used: Option<SystemTime>) -> Self {
        Self {
            usage_count,
            last_used,
        }
    }
}

// =============================================================================
// Result Ranker (Tasks 2.5.1, 2.5.4, 2.5.5)
// =============================================================================

/// Ranks search results by combining match quality with usage data.
///
/// The ranking formula is:
/// 1. `base_score = match_score + (frecency * 10.0)`
/// 2. Apply path boosts (system apps, user apps)
/// 3. Apply match type boosts (exact, prefix)
///
/// Tiebreaker order:
/// 1. Usage count (higher wins)
/// 2. Recency (more recent wins)
/// 3. Alphabetical by title (A before Z)
#[derive(Debug, Clone)]
pub struct ResultRanker {
    /// Boost configuration.
    pub boost_config: BoostConfig,
}

impl Default for ResultRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultRanker {
    /// Frecency multiplier in the combined score formula.
    pub const FRECENCY_MULTIPLIER: f64 = 10.0;

    /// Creates a new result ranker with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            boost_config: BoostConfig::default(),
        }
    }

    /// Creates a new result ranker with custom boost configuration.
    #[must_use]
    pub fn with_config(boost_config: BoostConfig) -> Self {
        Self { boost_config }
    }

    // =========================================================================
    // Task 2.5.1: Pure match quality ranking
    // =========================================================================

    /// Ranks results by match quality only (nucleo score).
    ///
    /// This is the simplest ranking - just sort by the raw match score
    /// from the fuzzy matcher, with higher scores first.
    pub fn rank_by_match_quality(&self, results: &mut [SearchResult]) {
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    }

    // =========================================================================
    // Task 2.5.3: Apply boost factors
    // =========================================================================

    /// Calculates the path boost for a result based on its file path.
    ///
    /// - System applications (/System/Applications): `system_app_boost` (1.2x)
    /// - User applications (/Applications): `applications_boost` (1.1x)
    /// - Other paths: 1.0x (no boost)
    #[must_use]
    pub fn calculate_path_boost(&self, path: Option<&Path>) -> f64 {
        match path {
            Some(p) => {
                let path_str = p.to_string_lossy();
                if path_str.starts_with("/System/Applications") {
                    self.boost_config.system_app_boost
                } else if path_str.starts_with("/Applications") {
                    self.boost_config.applications_boost
                } else {
                    1.0
                }
            },
            None => 1.0,
        }
    }

    /// Calculates the match type boost based on how the query matches the title.
    ///
    /// - Exact match (query == title): `exact_match_boost` (2.0x)
    /// - Prefix match (title.starts_with(query)): `prefix_match_boost` (1.5x)
    /// - Other matches: 1.0x (no boost)
    #[must_use]
    pub fn calculate_match_boost(&self, query: &str, title: &str) -> f64 {
        let query_lower = query.to_lowercase();
        self.calculate_match_boost_precomputed(&query_lower, title)
    }

    /// Like [`calculate_match_boost`] but accepts a pre-lowercased query.
    ///
    /// Use this in hot loops where the same query is compared against many titles
    /// to avoid re-computing `query.to_lowercase()` per result.
    #[must_use]
    fn calculate_match_boost_precomputed(&self, query_lower: &str, title: &str) -> f64 {
        let title_lower = title.to_lowercase();

        if query_lower == title_lower {
            self.boost_config.exact_match_boost
        } else if title_lower.starts_with(query_lower) {
            self.boost_config.prefix_match_boost
        } else {
            1.0
        }
    }

    /// Applies all boosts to a score.
    ///
    /// # Arguments
    ///
    /// * `score` - The base score to boost.
    /// * `query` - The search query.
    /// * `title` - The result title.
    /// * `path` - The result file path (if applicable).
    ///
    /// # Returns
    ///
    /// The boosted score.
    #[must_use]
    pub fn apply_boosts(&self, score: f64, query: &str, title: &str, path: Option<&Path>) -> f64 {
        let path_boost = self.calculate_path_boost(path);
        let match_boost = self.calculate_match_boost(query, title);
        score * path_boost * match_boost
    }

    /// Like [`apply_boosts`] but accepts a pre-lowercased query for hot loops.
    #[must_use]
    fn apply_boosts_precomputed(
        &self,
        score: f64,
        query_lower: &str,
        title: &str,
        path: Option<&Path>,
    ) -> f64 {
        let path_boost = self.calculate_path_boost(path);
        let match_boost = self.calculate_match_boost_precomputed(query_lower, title);
        score * path_boost * match_boost
    }

    // =========================================================================
    // Task 2.5.4: Combined ranking
    // =========================================================================

    /// Calculates the combined score for a result.
    ///
    /// Formula: `final_score = (match_score + (frecency * 10.0)) * boosts`
    ///
    /// # Arguments
    ///
    /// * `match_score` - Raw score from fuzzy matching.
    /// * `frecency` - Frecency score for the result.
    /// * `query` - The search query.
    /// * `title` - The result title.
    /// * `path` - The result file path (if applicable).
    #[must_use]
    pub fn calculate_combined_score(
        &self,
        match_score: f64,
        frecency: &FrecencyScore,
        query: &str,
        title: &str,
        path: Option<&Path>,
    ) -> f64 {
        // Base formula: match_score + (frecency * 10.0)
        let base_score = frecency
            .score()
            .mul_add(Self::FRECENCY_MULTIPLIER, match_score);

        // Apply boosts
        self.apply_boosts(base_score, query, title, path)
    }

    /// Ranks results with frecency integration (simple version).
    ///
    /// This modifies the score in-place and sorts by the combined score.
    ///
    /// # Arguments
    ///
    /// * `results` - Results to rank (modified in place).
    /// * `get_frecency` - Function to get frecency score for a result ID.
    pub fn rank_with_frecency<F>(&self, results: &mut [SearchResult], get_frecency: F)
    where
        F: Fn(&str) -> FrecencyScore,
    {
        for result in results.iter_mut() {
            let frecency = get_frecency(result.id.as_str());
            result.score = frecency
                .score()
                .mul_add(Self::FRECENCY_MULTIPLIER, result.score);
        }

        self.rank_by_match_quality(results);
    }

    // =========================================================================
    // Task 2.5.5: Tiebreaker logic
    // =========================================================================

    /// Compares two results for tiebreaking.
    ///
    /// Order: usage count (higher first) → recency (more recent first) → alphabetical
    ///
    /// This ensures deterministic ordering when scores are equal.
    #[must_use]
    pub fn tiebreaker_compare(
        a_title: &str,
        a_usage: &UsageData,
        b_title: &str,
        b_usage: &UsageData,
    ) -> Ordering {
        // 1. Usage count (higher wins)
        match b_usage.usage_count.cmp(&a_usage.usage_count) {
            Ordering::Equal => {},
            other => return other,
        }

        // 2. Recency (more recent wins)
        match (a_usage.last_used, b_usage.last_used) {
            (Some(a_time), Some(b_time)) => {
                // More recent = larger timestamp = should come first
                match a_time.cmp(&b_time) {
                    Ordering::Equal => {},
                    Ordering::Greater => return Ordering::Less, // a is more recent, a comes first
                    Ordering::Less => return Ordering::Greater, // b is more recent, b comes first
                }
            },
            (Some(_), None) => return Ordering::Less, // a has time, b doesn't, a wins
            (None, Some(_)) => return Ordering::Greater, // b has time, a doesn't, b wins
            (None, None) => {},
        }

        // 3. Alphabetical by title (A before Z)
        a_title.to_lowercase().cmp(&b_title.to_lowercase())
    }

    /// Ranks results with full ranking algorithm including tiebreaking.
    ///
    /// # Arguments
    ///
    /// * `results` - Results to rank (modified in place).
    /// * `query` - The search query.
    /// * `get_frecency` - Function to get frecency score for a result ID.
    /// * `get_usage` - Function to get usage data for a result ID.
    /// * `get_path` - Function to get the file path for a result ID.
    pub fn rank_full<FF, FU, FP>(
        &self,
        results: &mut [SearchResult],
        query: &str,
        get_frecency: FF,
        get_usage: FU,
        get_path: FP,
    ) where
        FF: Fn(&str) -> FrecencyScore,
        FU: Fn(&str) -> UsageData,
        FP: Fn(&str) -> Option<std::path::PathBuf>,
    {
        // Pre-compute lowercased query once for all results
        let query_lower = query.to_lowercase();

        // Calculate combined scores using pre-lowered query
        for result in results.iter_mut() {
            let frecency = get_frecency(result.id.as_str());
            let path = get_path(result.id.as_str());

            let base_score = frecency
                .score()
                .mul_add(Self::FRECENCY_MULTIPLIER, result.score);

            result.score =
                self.apply_boosts_precomputed(base_score, &query_lower, &result.title, path.as_deref());
        }

        // Sort with tiebreaking
        results.sort_by(|a, b| {
            // Primary: score (higher first)
            match b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal) {
                Ordering::Equal => {
                    // Tiebreaker
                    let a_usage = get_usage(a.id.as_str());
                    let b_usage = get_usage(b.id.as_str());
                    Self::tiebreaker_compare(&a.title, &a_usage, &b.title, &b_usage)
                },
                other => other,
            }
        });
    }
}

// =============================================================================
// Tests (Tasks 2.5.6 and 2.5.7)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{IconSource, SearchAction, SearchResultId};
    use std::path::PathBuf;
    use std::time::Duration;

    // =========================================================================
    // Helper functions
    // =========================================================================

    fn create_test_result(id: &str, title: &str, score: f64) -> SearchResult {
        SearchResult {
            id: SearchResultId::new(id),
            title: title.to_string(),
            subtitle: String::new(),
            icon: IconSource::SystemIcon {
                name: "test".to_string(),
            },
            result_type: crate::search::ResultType::Application,
            score,
            match_indices: Vec::new(),
            requires_permissions: false,
            action: SearchAction::OpenFile {
                path: PathBuf::from("/test"),
            },
        }
    }

    fn assert_float_eq(actual: f64, expected: f64) {
        let diff = (actual - expected).abs();
        assert!(diff < 1e-6, "expected {expected}, got {actual}");
    }

    // =========================================================================
    // Task 2.5.6: Frecency unit tests
    // =========================================================================

    #[test]
    fn test_frecency_zero() {
        let score = FrecencyScore::zero();
        assert_eq!(score.frequency, 0);
        assert_float_eq(score.recency, 0.0);
        assert_float_eq(score.score(), 0.0);
    }

    #[test]
    fn test_frecency_none_last_used() {
        let score = FrecencyScore::calculate(10, None);
        assert_eq!(score.frequency, 10);
        assert_float_eq(score.recency, 0.0);
        assert_float_eq(score.score(), 0.0);
    }

    #[test]
    fn test_frecency_just_now() {
        let score = FrecencyScore::calculate(5, Some(SystemTime::now()));
        assert_eq!(score.frequency, 5);
        // Recency should be very close to 1.0
        assert!(score.recency > 0.99);
        // Score should be close to frequency
        assert!(score.score() > 4.95);
    }

    #[test]
    fn test_frecency_half_life() {
        // 72 hours ago = half-life
        let half_life_ago = SystemTime::now() - Duration::from_secs(72 * 3600);
        let score = FrecencyScore::calculate(10, Some(half_life_ago));

        // Recency should be approximately 0.5
        assert!((score.recency - 0.5).abs() < 0.01);
        // Score should be approximately 5.0
        assert!((score.score() - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_frecency_two_half_lives() {
        // 144 hours ago = two half-lives
        let two_half_lives_ago = SystemTime::now() - Duration::from_secs(144 * 3600);
        let score = FrecencyScore::calculate(10, Some(two_half_lives_ago));

        // Recency should be approximately 0.25
        assert!((score.recency - 0.25).abs() < 0.01);
        // Score should be approximately 2.5
        assert!((score.score() - 2.5).abs() < 0.1);
    }

    #[test]
    fn test_frecency_new() {
        let score = FrecencyScore::new(5, 0.8);
        assert_eq!(score.frequency, 5);
        assert_float_eq(score.recency, 0.8);
        assert_float_eq(score.score(), 4.0);
    }

    // =========================================================================
    // Task 2.5.6: Boost unit tests
    // =========================================================================

    #[test]
    fn test_boost_config_default() {
        let config = BoostConfig::default();
        assert_float_eq(config.system_app_boost, 1.2);
        assert_float_eq(config.applications_boost, 1.1);
        assert_float_eq(config.exact_match_boost, 2.0);
        assert_float_eq(config.prefix_match_boost, 1.5);
    }

    #[test]
    fn test_boost_config_no_boosts() {
        let config = BoostConfig::no_boosts();
        assert_float_eq(config.system_app_boost, 1.0);
        assert_float_eq(config.applications_boost, 1.0);
        assert_float_eq(config.exact_match_boost, 1.0);
        assert_float_eq(config.prefix_match_boost, 1.0);
    }

    #[test]
    fn test_path_boost_system_app() {
        let ranker = ResultRanker::new();
        let path = Path::new("/System/Applications/Safari.app");
        let boost = ranker.calculate_path_boost(Some(path));
        assert_float_eq(boost, 1.2);
    }

    #[test]
    fn test_path_boost_user_app() {
        let ranker = ResultRanker::new();
        let path = Path::new("/Applications/Visual Studio Code.app");
        let boost = ranker.calculate_path_boost(Some(path));
        assert_float_eq(boost, 1.1);
    }

    #[test]
    fn test_path_boost_other() {
        let ranker = ResultRanker::new();
        let path = Path::new("/Users/test/Downloads/app.app");
        let boost = ranker.calculate_path_boost(Some(path));
        assert_float_eq(boost, 1.0);
    }

    #[test]
    fn test_path_boost_none() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_path_boost(None);
        assert_float_eq(boost, 1.0);
    }

    #[test]
    fn test_match_boost_exact() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_match_boost("Safari", "Safari");
        assert_float_eq(boost, 2.0);
    }

    #[test]
    fn test_match_boost_exact_case_insensitive() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_match_boost("safari", "Safari");
        assert_float_eq(boost, 2.0);
    }

    #[test]
    fn test_match_boost_prefix() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_match_boost("Saf", "Safari");
        assert_float_eq(boost, 1.5);
    }

    #[test]
    fn test_match_boost_prefix_case_insensitive() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_match_boost("saf", "Safari");
        assert_float_eq(boost, 1.5);
    }

    #[test]
    fn test_match_boost_fuzzy() {
        let ranker = ResultRanker::new();
        let boost = ranker.calculate_match_boost("sfr", "Safari");
        assert_float_eq(boost, 1.0);
    }

    #[test]
    fn test_apply_boosts_combined() {
        let ranker = ResultRanker::new();
        let path = Path::new("/System/Applications/Safari.app");

        // System app (1.2) + exact match (2.0) = 2.4x
        let boosted = ranker.apply_boosts(100.0, "Safari", "Safari", Some(path));
        assert_float_eq(boosted, 240.0);
    }

    // =========================================================================
    // Task 2.5.6: Tiebreaker unit tests
    // =========================================================================

    #[test]
    fn test_tiebreaker_usage_count() {
        let usage_a = UsageData::new(5, None);
        let usage_b = UsageData::new(10, None);

        // b has higher usage, should come first
        let result = ResultRanker::tiebreaker_compare("A", &usage_a, "B", &usage_b);
        assert_eq!(result, Ordering::Greater);
    }

    #[test]
    fn test_tiebreaker_recency() {
        let now = SystemTime::now();
        let earlier = now - Duration::from_secs(3600);

        let usage_a = UsageData::new(5, Some(now));
        let usage_b = UsageData::new(5, Some(earlier));

        // a is more recent, should come first
        let result = ResultRanker::tiebreaker_compare("A", &usage_a, "B", &usage_b);
        assert_eq!(result, Ordering::Less);
    }

    #[test]
    fn test_tiebreaker_alphabetical() {
        let usage_a = UsageData::new(5, None);
        let usage_b = UsageData::new(5, None);

        // Same usage, no recency, alphabetical: A before B
        let result = ResultRanker::tiebreaker_compare("Apple", &usage_a, "Banana", &usage_b);
        assert_eq!(result, Ordering::Less);
    }

    #[test]
    fn test_tiebreaker_alphabetical_case_insensitive() {
        let usage_a = UsageData::new(5, None);
        let usage_b = UsageData::new(5, None);

        // Case-insensitive: apple == Apple
        let result = ResultRanker::tiebreaker_compare("apple", &usage_a, "Banana", &usage_b);
        assert_eq!(result, Ordering::Less);
    }

    #[test]
    fn test_tiebreaker_some_vs_none() {
        let now = SystemTime::now();
        let usage_a = UsageData::new(5, Some(now));
        let usage_b = UsageData::new(5, None);

        // a has recency, b doesn't, a wins
        let result = ResultRanker::tiebreaker_compare("A", &usage_a, "B", &usage_b);
        assert_eq!(result, Ordering::Less);
    }

    // =========================================================================
    // Task 2.5.6: Ranking unit tests
    // =========================================================================

    #[test]
    fn test_rank_by_match_quality() {
        let ranker = ResultRanker::new();
        let mut results = vec![
            create_test_result("1", "Low", 50.0),
            create_test_result("2", "High", 100.0),
            create_test_result("3", "Medium", 75.0),
        ];

        ranker.rank_by_match_quality(&mut results);

        assert_eq!(results[0].title, "High");
        assert_eq!(results[1].title, "Medium");
        assert_eq!(results[2].title, "Low");
    }

    #[test]
    fn test_rank_with_frecency() {
        let ranker = ResultRanker::new();
        let mut results = vec![
            create_test_result("1", "Rarely Used", 100.0),
            create_test_result("2", "Frequently Used", 50.0),
        ];

        // Frequently Used has higher frecency (10 uses, recency 1.0 = score 100)
        ranker.rank_with_frecency(&mut results, |id| {
            if id == "2" {
                FrecencyScore::new(10, 1.0)
            } else {
                FrecencyScore::zero()
            }
        });

        // Frequently Used: 50 + (10 * 1.0 * 10) = 150
        // Rarely Used: 100 + 0 = 100
        assert_eq!(results[0].title, "Frequently Used");
        assert_eq!(results[1].title, "Rarely Used");
    }

    #[test]
    fn test_calculate_combined_score() {
        let ranker = ResultRanker::new();
        let frecency = FrecencyScore::new(5, 1.0);
        let path = Path::new("/Applications/Test.app");

        // base: 100 + (5 * 10) = 150
        // path boost: 1.1
        // prefix match boost: 1.5
        // final: 150 * 1.1 * 1.5 = 247.5
        let score = ranker.calculate_combined_score(100.0, &frecency, "Te", "Test", Some(path));
        assert!((score - 247.5).abs() < 0.01);
    }

    #[test]
    fn test_rank_full() {
        let ranker = ResultRanker::new();
        let mut results = vec![
            create_test_result("1", "Safari", 100.0),
            create_test_result("2", "Settings", 100.0),
            create_test_result("3", "Slack", 100.0),
        ];

        let get_frecency = |id: &str| -> FrecencyScore {
            match id {
                "1" => FrecencyScore::new(5, 1.0),
                "2" => FrecencyScore::new(10, 1.0),
                "3" => FrecencyScore::zero(),
                _ => FrecencyScore::zero(),
            }
        };

        let get_usage = |id: &str| -> UsageData {
            match id {
                "1" => UsageData::new(5, Some(SystemTime::now())),
                "2" => UsageData::new(10, Some(SystemTime::now())),
                "3" => UsageData::new(0, None),
                _ => UsageData::default(),
            }
        };

        let get_path = |id: &str| -> Option<PathBuf> {
            match id {
                "1" => Some(PathBuf::from("/System/Applications/Safari.app")),
                "2" => Some(PathBuf::from("/System/Applications/Settings.app")),
                "3" => Some(PathBuf::from("/Applications/Slack.app")),
                _ => None,
            }
        };

        ranker.rank_full(&mut results, "s", get_frecency, get_usage, get_path);

        // Settings has highest frecency (10 vs 5 vs 0)
        assert_eq!(results[0].title, "Settings");
        assert_eq!(results[1].title, "Safari");
        assert_eq!(results[2].title, "Slack");
    }
}

// =============================================================================
// Property Tests (Task 2.5.7)
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::search::{IconSource, SearchAction, SearchResultId};
    use proptest::prelude::*;
    use std::path::PathBuf;

    fn create_test_result(id: &str, title: &str, score: f64) -> SearchResult {
        SearchResult {
            id: SearchResultId::new(id),
            title: title.to_string(),
            subtitle: String::new(),
            icon: IconSource::SystemIcon {
                name: "test".to_string(),
            },
            result_type: crate::search::ResultType::Application,
            score,
            match_indices: Vec::new(),
            requires_permissions: false,
            action: SearchAction::OpenFile {
                path: PathBuf::from("/test"),
            },
        }
    }

    proptest! {
        // Task 2.5.7: Test ranking is deterministic (same input → same output)
        #[test]
        fn test_ranking_deterministic(
            scores in prop::collection::vec(0.0f64..1000.0, 1..20),
        ) {
            let ranker = ResultRanker::new();

            let create_results = || -> Vec<SearchResult> {
                scores
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| create_test_result(&format!("id{}", i), &format!("Title{}", i), s))
                    .collect()
            };

            // Rank twice with same input
            let mut results1 = create_results();
            let mut results2 = create_results();

            ranker.rank_by_match_quality(&mut results1);
            ranker.rank_by_match_quality(&mut results2);

            // Results should be in the same order
            for (r1, r2) in results1.iter().zip(results2.iter()) {
                prop_assert_eq!(r1.id.as_str(), r2.id.as_str());
                prop_assert!((r1.score - r2.score).abs() < 0.001);
            }
        }

        // Task 2.5.7: Test exact matches always rank higher than partial
        #[test]
        fn test_exact_match_ranks_higher(
            base_score in 1.0f64..100.0,
            query in "[a-z]{3,10}",
        ) {
            let ranker = ResultRanker::new();

            // Create two results: one exact match, one partial
            let exact_title = query.clone();
            let partial_title = format!("{}extra", query);

            let exact_score = ranker.apply_boosts(base_score, &query, &exact_title, None);
            let partial_score = ranker.apply_boosts(base_score, &query, &partial_title, None);

            // Exact match should have higher or equal score
            prop_assert!(exact_score >= partial_score);
        }

        // Test frecency score is non-negative
        #[test]
        fn test_frecency_non_negative(
            frequency in 0u32..1000,
            recency in 0.0f64..1.0,
        ) {
            let score = FrecencyScore::new(frequency, recency);
            prop_assert!(score.score() >= 0.0);
        }

        // Test boosts are multiplicative (order doesn't matter)
        #[test]
        fn test_boost_commutativity(
            base_score in 1.0f64..1000.0,
            path_boost in 1.0f64..2.0,
            match_boost in 1.0f64..3.0,
        ) {
            // Both orderings should give the same result
            let result1 = base_score * path_boost * match_boost;
            let result2 = base_score * match_boost * path_boost;

            prop_assert!((result1 - result2).abs() < 0.001);
        }

        // Test tiebreaker is transitive
        #[test]
        fn test_tiebreaker_transitivity(
            usage_a in 0u32..100,
            usage_b in 0u32..100,
            usage_c in 0u32..100,
        ) {
            let data_a = UsageData::new(usage_a, None);
            let data_b = UsageData::new(usage_b, None);
            let data_c = UsageData::new(usage_c, None);

            let ab = ResultRanker::tiebreaker_compare("A", &data_a, "B", &data_b);
            let bc = ResultRanker::tiebreaker_compare("B", &data_b, "C", &data_c);
            let ac = ResultRanker::tiebreaker_compare("A", &data_a, "C", &data_c);

            // If A <= B and B <= C, then A <= C
            if ab != Ordering::Greater && bc != Ordering::Greater {
                prop_assert_ne!(ac, Ordering::Greater);
            }
            // If A >= B and B >= C, then A >= C
            if ab != Ordering::Less && bc != Ordering::Less {
                prop_assert_ne!(ac, Ordering::Less);
            }
        }
    }
}
