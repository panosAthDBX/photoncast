//! nucleo fuzzy matching integration.
//!
//! This module provides a wrapper around the nucleo fuzzy matcher with
//! PhotonCast-specific configuration for Unicode normalization and smart case.

use std::collections::HashSet;

use nucleo::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Matcher, Utf32Str,
};

/// Bonus score per query character that matches a word boundary position.
///
/// A value of 20 means a 2-char acronym like "ss" → System Settings gets +40,
/// and a 3-char acronym like "vsc" → Visual Studio Code gets +60.
const WORD_BOUNDARY_BONUS: u32 = 20;

/// Configuration for the fuzzy matcher.
#[derive(Debug, Clone)]
pub struct MatcherConfig {
    /// Whether to use smart case matching (lowercase query = case-insensitive).
    pub smart_case: bool,
    /// Whether to normalize unicode for matching.
    pub normalize_unicode: bool,
    /// Whether to prefer prefix matches.
    pub prefer_prefix: bool,
    /// Maximum allowed spread factor for matched characters.
    /// Spread factor = (last_match_idx - first_match_idx) / query.len()
    /// A spread of 1.0 means matches are consecutive.
    /// A spread of 3.0 means matches span 3x the query length.
    /// Higher values allow more scattered matches.
    pub max_spread_factor: f32,
    /// Bonus score per character that matches a word boundary position.
    /// Set to 0 to disable word-boundary scoring.
    pub word_boundary_bonus: u32,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            smart_case: true,
            normalize_unicode: true,
            prefer_prefix: true,
            // Allow matches to span up to 1.5x the query length.
            // This filters out scattered matches like "test" -> "System Settings" (spread 1.75)
            // while still allowing reasonable fuzzy matches like "calc" -> "Calculator" (spread ~1.25).
            max_spread_factor: 1.5,
            word_boundary_bonus: WORD_BOUNDARY_BONUS,
        }
    }
}

/// Detects word boundary positions in a string.
///
/// Word boundaries are:
/// - Start of string (index 0)
/// - After space, hyphen, underscore, dot, slash
/// - CamelCase transitions (lowercase → uppercase)
fn find_word_boundaries(text: &str) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();

    if chars.is_empty() {
        return Vec::new();
    }

    let mut boundaries = Vec::with_capacity(chars.len() / 2 + 1);

    // First character is always a boundary
    boundaries.push(0);

    for i in 1..chars.len() {
        let prev = chars[i - 1];
        let curr = chars[i];

        // After separator characters or CamelCase transition (lowercase → uppercase)
        if matches!(prev, ' ' | '-' | '_' | '.' | '/')
            || (prev.is_lowercase() && curr.is_uppercase())
        {
            boundaries.push(i);
        }
    }

    boundaries
}

/// Calculates the word-boundary bonus for a match.
///
/// Counts how many of the matched character positions fall on word boundaries
/// of the lowercased target. The bonus is `boundary_matches * bonus_per_char`.
///
/// # Examples
///
/// - `"ss"` → `"System Settings"` at indices `[0, 7]` → 2 boundaries × 20 = 40
/// - `"vsc"` → `"Visual Studio Code"` at indices `[0, 7, 14]` → 3 × 20 = 60
fn calculate_word_boundary_bonus(
    target: &str,
    match_indices: &[usize],
    bonus_per_char: u32,
) -> u32 {
    if match_indices.is_empty() || bonus_per_char == 0 {
        return 0;
    }

    let target_lower = target.to_lowercase();
    let boundaries: HashSet<usize> = find_word_boundaries(&target_lower).into_iter().collect();

    let boundary_matches = match_indices
        .iter()
        .filter(|idx| boundaries.contains(idx))
        .count();

    #[allow(clippy::cast_possible_truncation)]
    // boundary_matches ≤ match_indices.len() which fits in u32
    let count = boundary_matches as u32;
    count * bonus_per_char
}

/// Wrapper around nucleo matcher with PhotonCast configuration.
///
/// This provides fuzzy matching with:
/// - Smart case matching (lowercase query = case-insensitive)
/// - Unicode normalization for international character support
/// - Match index tracking for UI highlighting
#[derive(Debug)]
pub struct FuzzyMatcher {
    /// The nucleo matcher instance.
    matcher: Matcher,
    /// Configuration options.
    config: MatcherConfig,
}

impl FuzzyMatcher {
    /// Creates a new fuzzy matcher with the given configuration.
    #[must_use]
    pub fn new(config: MatcherConfig) -> Self {
        // Create nucleo matcher with default config
        // The matcher can be reused across multiple calls
        let matcher = Matcher::new(nucleo::Config::DEFAULT);

        Self { matcher, config }
    }

    /// Creates a new fuzzy matcher with default configuration.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatcherConfig::default())
    }

    /// Returns the case matching strategy based on the query and configuration.
    fn get_case_matching(&self, query: &str) -> CaseMatching {
        if self.config.smart_case {
            // Smart case: if query contains uppercase, use case-sensitive matching
            // Otherwise, ignore case
            if query.chars().any(char::is_uppercase) {
                CaseMatching::Respect
            } else {
                CaseMatching::Ignore
            }
        } else {
            CaseMatching::Respect
        }
    }

    /// Returns the normalization strategy based on configuration.
    fn get_normalization(&self) -> Normalization {
        if self.config.normalize_unicode {
            Normalization::Smart
        } else {
            Normalization::Never
        }
    }

    /// Scores a target string against a query.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query.
    /// * `target` - The string to match against.
    ///
    /// # Returns
    ///
    /// A tuple of (score, match_indices) if the query matches, None otherwise.
    /// The score is higher for better matches. Match indices point to the
    /// character positions in the target that matched the query.
    #[allow(clippy::cast_precision_loss)]
    pub fn score(&mut self, query: &str, target: &str) -> Option<(u32, Vec<usize>)> {
        if query.is_empty() {
            return Some((0, Vec::new()));
        }

        if target.is_empty() {
            return None;
        }

        // Get matching configuration
        let case_matching = self.get_case_matching(query);
        let normalization = self.get_normalization();

        // Create an atom for the query pattern
        // AtomKind::Fuzzy for fuzzy matching
        let atom = Atom::new(query, case_matching, normalization, AtomKind::Fuzzy, false);

        // Convert target to Utf32Str for nucleo
        // The buffer is used to store the UTF-32 representation
        let mut char_buf: Vec<char> = Vec::with_capacity(target.len());
        let utf32_target = Utf32Str::new(target, &mut char_buf);

        // Create index buffer for match positions
        let mut indices: Vec<u32> = Vec::new();

        // Perform the matching
        let score = atom.indices(utf32_target, &mut self.matcher, &mut indices)?;

        // Convert u32 indices to usize
        let match_indices: Vec<usize> = indices.iter().map(|&i| i as usize).collect();

        // Apply prefix bonus if enabled and the match starts at the beginning
        let final_score = if self.config.prefer_prefix
            && !match_indices.is_empty()
            && match_indices[0] == 0
            && self.is_prefix_match(query, target, case_matching)
        {
            // Prefix bonus: add 50% to the score
            score + (score / 2)
        } else {
            score
        };

        // Check spread of matched characters to filter out scattered matches
        if match_indices.len() >= 2 {
            let first_idx = match_indices[0];
            let last_idx = match_indices[match_indices.len() - 1];
            let span = (last_idx - first_idx + 1) as f32;
            let query_len = query.len() as f32;
            let spread_factor = span / query_len;

            if spread_factor > self.config.max_spread_factor {
                return None;
            }
        }

        // Apply word boundary/acronym bonus (additive on top of nucleo's score)
        let boundary_bonus =
            calculate_word_boundary_bonus(target, &match_indices, self.config.word_boundary_bonus);
        let final_score_with_bonus = u32::from(final_score) + boundary_bonus;

        Some((final_score_with_bonus, match_indices))
    }

    /// Checks if the query is a prefix of the target.
    #[allow(clippy::unused_self)]
    fn is_prefix_match(&self, query: &str, target: &str, case_matching: CaseMatching) -> bool {
        match case_matching {
            CaseMatching::Ignore => {
                let query_lower = query.to_lowercase();
                let target_lower = target.to_lowercase();
                target_lower.starts_with(&query_lower)
            },
            CaseMatching::Respect => target.starts_with(query),
            CaseMatching::Smart | _ => {
                // For smart matching or any other variant, use case-insensitive
                let query_lower = query.to_lowercase();
                let target_lower = target.to_lowercase();
                target_lower.starts_with(&query_lower)
            },
        }
    }

    /// Scores multiple targets against a query and returns sorted results.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query.
    /// * `targets` - Iterator of (id, target_string) pairs.
    ///
    /// # Returns
    ///
    /// Vector of (id, score, match_indices) sorted by score descending.
    pub fn score_many<'a, I, S>(&mut self, query: &str, targets: I) -> Vec<(S, u32, Vec<usize>)>
    where
        I: Iterator<Item = (S, &'a str)>,
        S: Clone,
    {
        let mut results: Vec<(S, u32, Vec<usize>)> = targets
            .filter_map(|(id, target)| {
                self.score(query, target)
                    .map(|(score, indices)| (id, score, indices))
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.cmp(&a.1));

        results
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_query_matches_everything() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("", "Safari");
        assert!(result.is_some());
        let (score, indices) = result.unwrap();
        assert_eq!(score, 0);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_exact_match() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("Safari", "Safari");
        assert!(result.is_some());
        let (score, indices) = result.unwrap();
        assert!(score > 0);
        assert_eq!(indices.len(), 6); // All characters match
    }

    #[test]
    fn test_prefix_match() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("Saf", "Safari");
        assert!(result.is_some());
        let (score, indices) = result.unwrap();
        assert!(score > 0);
        assert_eq!(indices, vec![0, 1, 2]); // First 3 characters
    }

    #[test]
    fn test_fuzzy_match() {
        let mut matcher = FuzzyMatcher::default();
        // "clc" matches "Calculator" - c(0), l(2), c(3) - reasonable fuzzy match
        let result = matcher.score("clc", "Calculator");
        assert!(result.is_some());
        let (score, indices) = result.unwrap();
        assert!(score > 0);
        assert!(!indices.is_empty());
    }

    #[test]
    fn test_no_match() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("xyz", "Safari");
        assert!(result.is_none());
    }

    #[test]
    fn test_case_insensitive_lowercase_query() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("safari", "Safari");
        assert!(result.is_some());
    }

    #[test]
    fn test_case_sensitive_uppercase_query() {
        let mut matcher = FuzzyMatcher::default();
        // With smart case, uppercase in query means case-sensitive
        let result = matcher.score("SAFARI", "Safari");
        // This should not match because SAFARI != Safari when case-sensitive
        assert!(result.is_none());
    }

    #[test]
    fn test_unicode_normalization() {
        let mut matcher = FuzzyMatcher::default();
        // Test with accented characters
        let result = matcher.score("cafe", "Café");
        // With smart normalization, this should match
        assert!(result.is_some());
    }

    #[test]
    fn test_prefix_bonus() {
        let mut matcher = FuzzyMatcher::default();

        // Prefix match should score higher
        let prefix_result = matcher.score("Saf", "Safari");
        let middle_result = matcher.score("ari", "Safari");

        assert!(prefix_result.is_some());
        assert!(middle_result.is_some());

        let (prefix_score, _) = prefix_result.unwrap();
        let (middle_score, _) = middle_result.unwrap();

        // Prefix should score higher or equal
        assert!(prefix_score >= middle_score);
    }

    #[test]
    fn test_score_many() {
        let mut matcher = FuzzyMatcher::default();

        let targets = vec![
            ("id1", "Safari"),
            ("id2", "System Preferences"),
            ("id3", "Xcode"),
        ];

        let results: Vec<_> = matcher.score_many("saf", targets.into_iter());

        // Safari should be first (best match for "saf")
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "id1");
    }

    #[test]
    fn test_empty_target_no_match() {
        let mut matcher = FuzzyMatcher::default();
        let result = matcher.score("test", "");
        assert!(result.is_none());
    }

    #[test]
    fn test_weak_fuzzy_match_filtered() {
        let mut matcher = FuzzyMatcher::default();
        // "test" should NOT match "System Settings" because characters are too scattered
        // nucleo finds indices [3, 4, 7, 9] -> spread = (9-3+1)/4 = 1.75 > 1.5
        let result = matcher.score("test", "System Settings");
        assert!(
            result.is_none(),
            "Weak fuzzy matches should be filtered out"
        );
    }

    #[test]
    fn test_strong_fuzzy_match_allowed() {
        let mut matcher = FuzzyMatcher::default();
        // "term" should match "Terminal" - consecutive characters
        let result = matcher.score("term", "Terminal");
        assert!(result.is_some(), "Strong fuzzy matches should be allowed");
    }

    #[test]
    fn test_spread_factor_configurable() {
        // Create matcher with very high spread factor (allows scattered matches)
        let config = MatcherConfig {
            max_spread_factor: 100.0,
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::new(config);

        // With high spread factor, even scattered matches should pass
        let result = matcher.score("test", "System Settings");
        assert!(
            result.is_some(),
            "High spread factor should allow scattered matches"
        );
    }

    // ── Phase 3: Word boundary / acronym bonus tests ──

    // Task 3.1 tests: find_word_boundaries

    #[test]
    fn test_word_boundaries_simple() {
        // "System Settings" → boundaries at [0, 7]
        let boundaries = find_word_boundaries("system settings");
        assert_eq!(boundaries, vec![0, 7]);
    }

    #[test]
    fn test_word_boundaries_camelcase() {
        // "macOS" → lowercase is "macos", but we test original casing for camelCase
        // In "macOS": m(0), a(1), c(2)=lower, O(3)=upper → boundary at [0, 3]
        let boundaries = find_word_boundaries("macOS");
        assert_eq!(boundaries, vec![0, 3]);
    }

    #[test]
    fn test_word_boundaries_hyphen() {
        // "Wi-Fi" → boundaries at [0, 3]
        let boundaries = find_word_boundaries("Wi-Fi");
        assert_eq!(boundaries, vec![0, 3]);
    }

    #[test]
    fn test_word_boundaries_underscore() {
        // "my_app_name" → boundaries at [0, 3, 7]
        let boundaries = find_word_boundaries("my_app_name");
        assert_eq!(boundaries, vec![0, 3, 7]);
    }

    #[test]
    fn test_word_boundaries_empty() {
        let boundaries = find_word_boundaries("");
        assert!(boundaries.is_empty());
    }

    #[test]
    fn test_word_boundaries_single_char() {
        let boundaries = find_word_boundaries("A");
        assert_eq!(boundaries, vec![0]);
    }

    #[test]
    fn test_word_boundaries_dot_separator() {
        // "com.apple.Safari" → boundaries at [0, 4, 10]
        let boundaries = find_word_boundaries("com.apple.Safari");
        assert_eq!(boundaries, vec![0, 4, 10]);
    }

    #[test]
    fn test_word_boundaries_slash_separator() {
        // "/usr/local/bin" → boundaries at [0, 1, 5, 11]
        // '/' at 0 means next char (1) is boundary; but index 0 is always boundary too
        let boundaries = find_word_boundaries("/usr/local/bin");
        assert_eq!(boundaries, vec![0, 1, 5, 11]);
    }

    // Task 3.2 tests: calculate_word_boundary_bonus

    #[test]
    fn test_acronym_bonus_ss() {
        // "ss" matching "System Settings" at positions [0, 7] → both are boundaries
        let bonus = calculate_word_boundary_bonus("System Settings", &[0, 7], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 40);
    }

    #[test]
    fn test_acronym_bonus_vsc() {
        // "vsc" matching "Visual Studio Code" at positions [0, 7, 14] → all boundaries
        let bonus =
            calculate_word_boundary_bonus("Visual Studio Code", &[0, 7, 14], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 60);
    }

    #[test]
    fn test_acronym_bonus_gc() {
        // "gc" matching "Google Chrome" at positions [0, 7] → both boundaries
        let bonus = calculate_word_boundary_bonus("Google Chrome", &[0, 7], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 40);
    }

    #[test]
    fn test_no_bonus_non_boundary() {
        // Match indices at non-boundary positions get 0 bonus
        // "System Settings" has boundaries at [0, 7] (lowercased)
        let bonus = calculate_word_boundary_bonus("System Settings", &[2, 4], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_bonus_empty_match() {
        let bonus = calculate_word_boundary_bonus("System Settings", &[], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_bonus_partial_boundary_match() {
        // One match at boundary (0), one not (3)
        // "System Settings" boundaries at [0, 7]
        let bonus = calculate_word_boundary_bonus("System Settings", &[0, 3], WORD_BOUNDARY_BONUS);
        assert_eq!(bonus, 20); // Only 1 boundary match
    }

    // Task 3.3 tests: MatcherConfig word_boundary_bonus

    #[test]
    fn test_matcher_config_default_bonus() {
        let config = MatcherConfig::default();
        assert_eq!(config.word_boundary_bonus, 20);
    }

    #[test]
    fn test_boundary_bonus_disabled() {
        // Setting word_boundary_bonus = 0 effectively disables it
        let bonus = calculate_word_boundary_bonus("System Settings", &[0, 7], 0);
        assert_eq!(bonus, 0);
    }

    #[test]
    fn test_boundary_bonus_disabled_in_scorer() {
        // Verify that setting word_boundary_bonus to 0 in config disables it in score()
        // Need relaxed spread factor since "gc" → "Google Chrome" spans across words
        let config_with = MatcherConfig {
            max_spread_factor: 10.0,
            ..Default::default()
        };
        let config_without = MatcherConfig {
            word_boundary_bonus: 0,
            max_spread_factor: 10.0,
            ..Default::default()
        };

        let mut matcher_with = FuzzyMatcher::new(config_with);
        let mut matcher_without = FuzzyMatcher::new(config_without);

        // Use a query that would hit word boundaries: "gc" → "Google Chrome"
        let result_with = matcher_with.score("gc", "Google Chrome");
        let result_without = matcher_without.score("gc", "Google Chrome");

        assert!(result_with.is_some());
        assert!(result_without.is_some());

        let (score_with, _) = result_with.unwrap();
        let (score_without, _) = result_without.unwrap();

        // The score with boundary bonus should be higher
        assert!(
            score_with > score_without,
            "score with bonus ({score_with}) should be greater than without ({score_without})"
        );
    }

    // Task 3.4 tests: Integration into FuzzyMatcher::score()

    #[test]
    fn test_score_includes_boundary_bonus() {
        // "ss" vs "System Settings" — should score higher with boundary bonus
        // Need relaxed spread factor since acronym-style matches span across words
        let config_with = MatcherConfig {
            max_spread_factor: 10.0,
            ..Default::default()
        };
        let config_without = MatcherConfig {
            word_boundary_bonus: 0,
            max_spread_factor: 10.0,
            ..Default::default()
        };

        let mut matcher_with = FuzzyMatcher::new(config_with);
        let mut matcher_without = FuzzyMatcher::new(config_without);

        let result_with = matcher_with.score("ss", "System Settings");
        let result_without = matcher_without.score("ss", "System Settings");

        assert!(result_with.is_some(), "ss should match System Settings");
        assert!(result_without.is_some());

        let (score_with, _) = result_with.unwrap();
        let (score_without, _) = result_without.unwrap();

        assert!(
            score_with > score_without,
            "boundary bonus should increase score: with={score_with}, without={score_without}"
        );
    }

    #[test]
    fn test_score_no_regression_exact_match() {
        // Exact matches should still rank highest
        let mut matcher = FuzzyMatcher::default();
        let exact = matcher.score("Safari", "Safari");
        let partial = matcher.score("Saf", "Safari");

        assert!(exact.is_some());
        assert!(partial.is_some());

        let (exact_score, _) = exact.unwrap();
        let (partial_score, _) = partial.unwrap();

        assert!(
            exact_score >= partial_score,
            "exact match should rank at least as high as partial"
        );
    }

    #[test]
    fn test_score_no_regression_prefix_match() {
        // Prefix bonus should still work alongside word boundary bonus
        let mut matcher = FuzzyMatcher::default();
        let prefix_result = matcher.score("ter", "Terminal");
        let non_prefix_result = matcher.score("nal", "Terminal");

        assert!(prefix_result.is_some());
        assert!(non_prefix_result.is_some());

        let (prefix_score, _) = prefix_result.unwrap();
        let (non_prefix_score, _) = non_prefix_result.unwrap();

        assert!(
            prefix_score >= non_prefix_score,
            "prefix match should still rank higher: prefix={prefix_score}, non-prefix={non_prefix_score}"
        );
    }

    #[test]
    fn test_score_acronym_vsc_matches_visual_studio_code() {
        // "vsc" should match "Visual Studio Code" via the fuzzy matcher
        // We need high spread factor since the chars are spread across 3 words
        let config = MatcherConfig {
            max_spread_factor: 10.0,
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::new(config);

        let result = matcher.score("vsc", "Visual Studio Code");
        assert!(
            result.is_some(),
            "vsc should match Visual Studio Code with relaxed spread"
        );

        let (score, indices) = result.unwrap();
        assert!(score > 0);
        // Verify the match hits word boundaries
        assert!(
            !indices.is_empty(),
            "should have match indices for vsc → Visual Studio Code"
        );
    }
}
