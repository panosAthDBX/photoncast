//! nucleo fuzzy matching integration.
//!
//! This module provides a wrapper around the nucleo fuzzy matcher with
//! PhotonCast-specific configuration for Unicode normalization and smart case.

use nucleo::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Matcher, Utf32Str,
};

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
        }
    }
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
            if query.chars().any(|c| c.is_uppercase()) {
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

        Some((u32::from(final_score), match_indices))
    }

    /// Checks if the query is a prefix of the target.
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
}
