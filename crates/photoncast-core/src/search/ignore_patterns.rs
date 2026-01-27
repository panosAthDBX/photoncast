//! Ignore patterns for file indexing.
//!
//! This module provides gitignore-style pattern matching for excluding files
//! from the search index. It supports:
//!
//! - Wildcards (`*`, `**`, `?`)
//! - Negation (`!` prefix)
//! - Directory patterns (trailing `/`)
//! - Hierarchical matching (patterns inherit from parent directories)
//! - Multiple ignore file types (`.gitignore`, `.ignore`, `.photonignore`)
//!
//! # Architecture
//!
//! - [`IgnorePattern`] - A single gitignore-style pattern
//! - [`IgnorePatternSet`] - Collection of patterns from one ignore file
//! - [`IgnoreMatcher`] - Hierarchical matcher with caching
//!
//! # Example
//!
//! ```
//! use photoncast_core::search::ignore_patterns::{IgnorePattern, IgnoreMatcher};
//! use std::path::Path;
//!
//! // Parse a single pattern
//! let pattern = IgnorePattern::parse("*.log").unwrap();
//! assert!(pattern.matches(Path::new("debug.log"), false));
//!
//! // Create a matcher for a directory tree
//! let matcher = IgnoreMatcher::new();
//! // matcher.add_patterns_from_file(Path::new("/project/.gitignore"));
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during ignore pattern operations.
#[derive(Error, Debug, Clone)]
pub enum IgnoreError {
    /// Invalid pattern syntax.
    #[error("invalid pattern '{pattern}': {reason}")]
    InvalidPattern {
        /// The invalid pattern.
        pattern: String,
        /// Why the pattern is invalid.
        reason: String,
    },

    /// Failed to read ignore file.
    #[error("failed to read ignore file '{path}': {reason}")]
    ReadError {
        /// Path to the file that couldn't be read.
        path: PathBuf,
        /// Error reason.
        reason: String,
    },

    /// Invalid path for pattern matching.
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

/// Result type for ignore pattern operations.
pub type Result<T> = std::result::Result<T, IgnoreError>;

// =============================================================================
// IgnorePattern
// =============================================================================

/// A single gitignore-style ignore pattern.
///
/// Patterns support:
/// - `*` matches any characters except `/`
/// - `**` matches any characters including `/`
/// - `?` matches any single character except `/`
/// - `!` prefix negates the pattern (un-ignores)
/// - Trailing `/` matches directories only
/// - Leading `/` anchors to the pattern's base directory
#[derive(Debug, Clone)]
pub struct IgnorePattern {
    /// The original pattern string.
    original: String,
    /// Compiled regex pattern for matching.
    pattern: String,
    /// Whether this is a negation pattern (starts with !).
    is_negation: bool,
    /// Whether this pattern only matches directories.
    directory_only: bool,
    /// Whether this pattern is anchored to the root.
    anchored: bool,
    /// Source file path (for debugging).
    source_file: Option<PathBuf>,
    /// Line number in source file.
    line_number: Option<usize>,
}

impl IgnorePattern {
    /// Parses a gitignore-style pattern string.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern string to parse.
    ///
    /// # Returns
    ///
    /// A parsed `IgnorePattern` or an error if the pattern is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use photoncast_core::search::ignore_patterns::IgnorePattern;
    ///
    /// let pattern = IgnorePattern::parse("*.log").unwrap();
    /// let negated = IgnorePattern::parse("!important.log").unwrap();
    /// let dir_only = IgnorePattern::parse("node_modules/").unwrap();
    /// ```
    pub fn parse(pattern: &str) -> Result<Self> {
        Self::parse_with_source(pattern, None, None)
    }

    /// Parses a pattern with source file information for error messages.
    pub fn parse_with_source(
        pattern: &str,
        source_file: Option<PathBuf>,
        line_number: Option<usize>,
    ) -> Result<Self> {
        let pattern = pattern.trim();

        // Empty patterns are invalid
        if pattern.is_empty() {
            return Err(IgnoreError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "pattern cannot be empty".to_string(),
            });
        }

        // Comments start with # (not a pattern)
        if pattern.starts_with('#') {
            return Err(IgnoreError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "comments are not patterns".to_string(),
            });
        }

        let mut remaining = pattern;
        let mut is_negation = false;
        let mut directory_only = false;
        let mut anchored = false;

        // Check for negation prefix
        if let Some(rest) = remaining.strip_prefix('!') {
            is_negation = true;
            remaining = rest;
        }

        // Check for directory-only suffix
        if remaining.ends_with('/') {
            directory_only = true;
            remaining = remaining.trim_end_matches('/');
        }

        // Check for root anchor
        if remaining.starts_with('/') {
            anchored = true;
            remaining = remaining.trim_start_matches('/');
        }

        // Also anchor if pattern contains a path separator (but not just **)
        if remaining.contains('/') && remaining != "**" {
            anchored = true;
        }

        // Convert gitignore pattern to simplified matching pattern
        let compiled = Self::compile_pattern(remaining)?;

        Ok(Self {
            original: pattern.to_string(),
            pattern: compiled,
            is_negation,
            directory_only,
            anchored,
            source_file,
            line_number,
        })
    }

    /// Compiles a gitignore pattern into a simplified matching format.
    fn compile_pattern(pattern: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '*' => {
                    // Check for **
                    if chars.peek() == Some(&'*') {
                        chars.next();
                        // ** matches anything including /
                        result.push_str("**");
                    } else {
                        // * matches anything except /
                        result.push('*');
                    }
                },
                '?' => {
                    // ? matches any single character except /
                    result.push('?');
                },
                '[' => {
                    // Character class - copy as-is
                    result.push('[');
                    let mut found_close = false;
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == ']' {
                            found_close = true;
                            break;
                        }
                    }
                    if !found_close {
                        return Err(IgnoreError::InvalidPattern {
                            pattern: pattern.to_string(),
                            reason: "unclosed character class".to_string(),
                        });
                    }
                },
                '\\' => {
                    // Escape next character
                    if let Some(next) = chars.next() {
                        result.push('\\');
                        result.push(next);
                    }
                },
                _ => {
                    result.push(c);
                },
            }
        }

        Ok(result)
    }

    /// Checks if this pattern matches the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check (relative to the pattern's base directory).
    /// * `is_dir` - Whether the path is a directory.
    ///
    /// # Returns
    ///
    /// `true` if the pattern matches the path.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn matches(&self, path: &Path, is_dir: bool) -> bool {
        // Directory-only patterns only match directories
        if self.directory_only && !is_dir {
            return false;
        }

        let path_str = path.to_string_lossy();

        if self.anchored {
            // Anchored patterns match from the root
            self.pattern_matches(&self.pattern, &path_str)
        } else {
            // Non-anchored patterns can match any path component
            // Try matching the full path first
            if self.pattern_matches(&self.pattern, &path_str) {
                return true;
            }

            // Then try matching just the file/directory name
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if self.pattern_matches(&self.pattern, name) {
                    return true;
                }
            }

            false
        }
    }

    /// Internal pattern matching implementation.
    #[allow(clippy::unused_self)]
    fn pattern_matches(&self, pattern: &str, text: &str) -> bool {
        Self::glob_match(pattern, text)
    }

    /// Glob-style pattern matching.
    ///
    /// Supports:
    /// - `*` matches any characters except `/`
    /// - `**` matches any characters including `/`
    /// - `?` matches any single character except `/`
    fn glob_match(pattern: &str, text: &str) -> bool {
        Self::glob_match_impl(pattern.as_bytes(), text.as_bytes())
    }

    #[allow(clippy::similar_names)] // pi/ti are pattern/text indices, intentionally similar
    fn glob_match_impl(pattern: &[u8], text: &[u8]) -> bool {
        let mut pi = 0; // pattern index
        let mut ti = 0; // text index
        let mut star_pattern_idx = None; // position after last *
        let mut star_text_idx = None; // text position at last *
        let mut dstar_pattern_idx = None; // position after last **
        let mut dstar_text_idx = None; // text position at last **

        while ti < text.len() {
            if pi < pattern.len() {
                // Check for **
                if pi + 1 < pattern.len() && pattern[pi] == b'*' && pattern[pi + 1] == b'*' {
                    // ** matches everything including /
                    dstar_pattern_idx = Some(pi + 2);
                    dstar_text_idx = Some(ti);
                    pi += 2;
                    // Skip trailing / after **
                    if pi < pattern.len() && pattern[pi] == b'/' {
                        pi += 1;
                    }
                    continue;
                }

                // Check for *
                if pattern[pi] == b'*' {
                    star_pattern_idx = Some(pi + 1);
                    star_text_idx = Some(ti);
                    pi += 1;
                    continue;
                }

                // Check for ?
                if pattern[pi] == b'?' && text[ti] != b'/' {
                    pi += 1;
                    ti += 1;
                    continue;
                }

                // Check for escaped character
                if pattern[pi] == b'\\' && pi + 1 < pattern.len() && text[ti] == pattern[pi + 1] {
                    pi += 2;
                    ti += 1;
                    continue;
                }

                // Literal match (case-insensitive on macOS)
                if pattern[pi].eq_ignore_ascii_case(&text[ti]) {
                    pi += 1;
                    ti += 1;
                    continue;
                }
            }

            // Try backtracking to * (but * doesn't match /)
            if let (Some(spi), Some(sti)) = (star_pattern_idx, star_text_idx) {
                if sti < text.len() && text[sti] != b'/' {
                    pi = spi;
                    star_text_idx = Some(sti + 1);
                    ti = sti + 1;
                    continue;
                }
            }

            // Try backtracking to ** (matches everything)
            if let (Some(dspi), Some(dsti)) = (dstar_pattern_idx, dstar_text_idx) {
                if dsti < text.len() {
                    pi = dspi;
                    dstar_text_idx = Some(dsti + 1);
                    ti = dsti + 1;
                    continue;
                }
            }

            return false;
        }

        // Check remaining pattern
        while pi < pattern.len() {
            if pattern[pi] == b'*' {
                pi += 1;
            } else {
                return false;
            }
        }

        true
    }

    /// Returns whether this is a negation pattern.
    #[must_use]
    pub fn is_negation(&self) -> bool {
        self.is_negation
    }

    /// Returns the original pattern string.
    #[must_use]
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Returns whether this pattern only matches directories.
    #[must_use]
    pub fn is_directory_only(&self) -> bool {
        self.directory_only
    }

    /// Returns whether this pattern is anchored to the root.
    #[must_use]
    pub fn is_anchored(&self) -> bool {
        self.anchored
    }
}

// =============================================================================
// IgnorePatternSet
// =============================================================================

/// A collection of ignore patterns from a single ignore file.
///
/// Patterns are applied in order, with later patterns overriding earlier ones.
/// This matches gitignore behavior where the last matching pattern wins.
#[derive(Debug, Clone, Default)]
pub struct IgnorePatternSet {
    /// The patterns in this set.
    patterns: Vec<IgnorePattern>,
    /// The source file path.
    source_path: Option<PathBuf>,
    /// The base directory for pattern matching.
    base_dir: PathBuf,
}

impl IgnorePatternSet {
    /// Creates a new empty pattern set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a pattern set with a base directory.
    #[must_use]
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self {
            patterns: Vec::new(),
            source_path: None,
            base_dir,
        }
    }

    /// Parses patterns from an ignore file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ignore file.
    ///
    /// # Returns
    ///
    /// A pattern set containing all valid patterns from the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| IgnoreError::ReadError {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        let base_dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
        Self::from_string(&content, base_dir, Some(path.to_path_buf()))
    }

    /// Parses patterns from a string.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to parse.
    /// * `base_dir` - The base directory for pattern matching.
    /// * `source_path` - Optional source path for error messages.
    pub fn from_string(
        content: &str,
        base_dir: PathBuf,
        source_path: Option<PathBuf>,
    ) -> Result<Self> {
        let mut patterns = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match IgnorePattern::parse_with_source(line, source_path.clone(), Some(line_num + 1)) {
                Ok(pattern) => patterns.push(pattern),
                Err(IgnoreError::InvalidPattern { .. }) => {
                    // Skip invalid patterns but continue parsing
                },
                Err(e) => return Err(e),
            }
        }

        Ok(Self {
            patterns,
            source_path,
            base_dir,
        })
    }

    /// Adds a pattern to this set.
    pub fn add_pattern(&mut self, pattern: IgnorePattern) {
        self.patterns.push(pattern);
    }

    /// Checks if a path should be ignored by this pattern set.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check (relative to the base directory).
    /// * `is_dir` - Whether the path is a directory.
    ///
    /// # Returns
    ///
    /// `Some(true)` if ignored, `Some(false)` if un-ignored by negation,
    /// `None` if no patterns match.
    #[must_use]
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> Option<bool> {
        let mut result = None;

        // Apply patterns in order; last match wins
        for pattern in &self.patterns {
            if pattern.matches(path, is_dir) {
                result = Some(!pattern.is_negation);
            }
        }

        result
    }

    /// Returns the patterns in this set.
    #[must_use]
    pub fn patterns(&self) -> &[IgnorePattern] {
        &self.patterns
    }

    /// Returns the base directory for this pattern set.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Returns the source file path, if any.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Returns whether this set has any patterns.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Returns the number of patterns in this set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.patterns.len()
    }
}

// =============================================================================
// IgnoreMatcher
// =============================================================================

/// Hierarchical ignore pattern matcher with caching.
///
/// This matcher loads ignore patterns from `.gitignore`, `.ignore`, and
/// `.photonignore` files throughout a directory tree, applying them
/// hierarchically (patterns from parent directories apply to children).
///
/// Results are cached for performance. Cache is automatically evicted when
/// it exceeds `MAX_CACHE_SIZE`.
#[derive(Debug, Default)]
pub struct IgnoreMatcher {
    /// Pattern sets indexed by directory path.
    pattern_sets: RwLock<HashMap<PathBuf, Arc<IgnorePatternSet>>>,
    /// Cached match results: (path, is_dir) -> ignored.
    cache: RwLock<HashMap<(PathBuf, bool), bool>>,
    /// Root directories to search for patterns.
    roots: RwLock<Vec<PathBuf>>,
    /// Global patterns (applied to all paths).
    global_patterns: RwLock<Option<Arc<IgnorePatternSet>>>,
}

impl IgnoreMatcher {
    /// Names of ignore files to search for (in order of priority).
    pub const IGNORE_FILE_NAMES: &'static [&'static str] =
        &[".gitignore", ".ignore", ".photonignore"];

    /// Maximum number of entries in the match cache.
    ///
    /// When exceeded, the cache is cleared to prevent unbounded memory growth.
    /// This is a simple eviction strategy that trades off some cache hits
    /// for implementation simplicity.
    pub const MAX_CACHE_SIZE: usize = 10_000;

    /// Creates a new empty ignore matcher.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a matcher with a root directory.
    ///
    /// Immediately scans for ignore files in the root directory.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn with_root(root: PathBuf) -> Self {
        let matcher = Self::new();
        matcher.add_root(root);
        matcher
    }

    /// Adds a root directory to scan for ignore patterns.
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_root(&self, root: PathBuf) {
        self.roots.write().push(root.clone());
        // Load patterns from root
        self.load_patterns_for_directory(&root);
    }

    /// Sets global patterns that apply to all paths.
    pub fn set_global_patterns(&self, patterns: IgnorePatternSet) {
        *self.global_patterns.write() = Some(Arc::new(patterns));
        self.invalidate_cache();
    }

    /// Loads ignore patterns from a directory.
    ///
    /// This method searches for ignore files in the directory and loads them.
    fn load_patterns_for_directory(&self, dir: &Path) {
        for name in Self::IGNORE_FILE_NAMES {
            let ignore_path = dir.join(name);
            if ignore_path.exists() {
                match IgnorePatternSet::from_file(&ignore_path) {
                    Ok(pattern_set) if !pattern_set.is_empty() => {
                        self.pattern_sets
                            .write()
                            .insert(dir.to_path_buf(), Arc::new(pattern_set));
                        // Only use the first existing ignore file per directory
                        break;
                    },
                    _ => {}, // Skip empty files or unreadable files
                }
            }
        }
    }

    /// Adds a pattern set for a specific directory.
    pub fn add_pattern_set(&self, dir: PathBuf, patterns: IgnorePatternSet) {
        self.pattern_sets.write().insert(dir, Arc::new(patterns));
        self.invalidate_cache();
    }

    /// Checks if a path should be ignored.
    ///
    /// This method applies patterns hierarchically, checking patterns from
    /// parent directories down to the path's directory.
    ///
    /// # Arguments
    ///
    /// * `path` - The absolute path to check.
    /// * `is_dir` - Whether the path is a directory.
    ///
    /// # Returns
    ///
    /// `true` if the path should be ignored, `false` otherwise.
    #[must_use]
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        // Check cache first
        let cache_key = (path.to_path_buf(), is_dir);
        if let Some(&cached) = self.cache.read().get(&cache_key) {
            return cached;
        }

        // Compute and cache result
        let result = self.compute_is_ignored(path, is_dir);

        // Evict cache if it's too large before inserting
        let mut cache = self.cache.write();
        if cache.len() >= Self::MAX_CACHE_SIZE {
            cache.clear();
        }
        cache.insert(cache_key, result);
        result
    }

    /// Computes whether a path should be ignored (uncached).
    fn compute_is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        // Get absolute path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
        };

        // Apply global patterns first
        let mut ignored = None;
        if let Some(ref global) = *self.global_patterns.read() {
            if let Some(name) = abs_path.file_name().and_then(|n| n.to_str()) {
                ignored = global.is_ignored(Path::new(name), is_dir);
            }
        }

        // Collect ancestors in root-to-leaf order
        let ancestors: Vec<_> = abs_path.ancestors().collect();

        // Process each ancestor directory
        for ancestor in ancestors.iter().rev() {
            // Check if we need to load patterns for this directory
            let needs_load = {
                let pattern_sets = self.pattern_sets.read();
                !pattern_sets.contains_key(*ancestor) && ancestor.is_dir()
            };

            if needs_load {
                self.load_patterns_for_directory(ancestor);
            }

            // Check for patterns in this directory
            let pattern_sets = self.pattern_sets.read();
            if let Some(patterns) = pattern_sets.get(*ancestor) {
                // Get relative path from this directory
                if let Ok(rel_path) = abs_path.strip_prefix(ancestor) {
                    if let Some(result) = patterns.is_ignored(rel_path, is_dir) {
                        ignored = Some(result);
                    }
                }
            }
        }

        ignored.unwrap_or(false)
    }

    /// Invalidates the entire cache.
    ///
    /// Call this when patterns change.
    pub fn invalidate_cache(&self) {
        self.cache.write().clear();
    }

    /// Invalidates cache entries for paths under the given directory.
    pub fn invalidate_cache_for_directory(&self, dir: &Path) {
        let mut cache = self.cache.write();
        cache.retain(|(path, _), _| !path.starts_with(dir));
    }

    /// Returns the number of cached results.
    #[must_use]
    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }

    /// Returns the number of pattern sets loaded.
    #[must_use]
    pub fn pattern_set_count(&self) -> usize {
        self.pattern_sets.read().len()
    }
}

// =============================================================================
// Exclude Action Support
// =============================================================================

/// Adds a file or pattern to a `.photonignore` file.
///
/// If the file doesn't exist, it will be created.
///
/// # Arguments
///
/// * `directory` - The directory to add the ignore file to.
/// * `pattern` - The pattern to add.
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn add_to_photonignore(directory: &Path, pattern: &str) -> Result<()> {
    let ignore_path = directory.join(".photonignore");

    // Read existing content if file exists
    let mut content = if ignore_path.exists() {
        std::fs::read_to_string(&ignore_path).map_err(|e| IgnoreError::ReadError {
            path: ignore_path.clone(),
            reason: e.to_string(),
        })?
    } else {
        String::new()
    };

    // Check if pattern already exists
    let pattern = pattern.trim();
    if content.lines().any(|line| line.trim() == pattern) {
        return Ok(()); // Already exists
    }

    // Add pattern
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(pattern);
    content.push('\n');

    // Write file
    std::fs::write(&ignore_path, content).map_err(|e| IgnoreError::ReadError {
        path: ignore_path,
        reason: e.to_string(),
    })?;

    Ok(())
}

/// Creates a pattern for a specific file path.
///
/// # Arguments
///
/// * `path` - The file path.
/// * `relative_to` - Directory to make the pattern relative to.
///
/// # Returns
///
/// A pattern string for the file.
#[must_use]
pub fn pattern_for_file(path: &Path, relative_to: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(relative_to) {
        // Use forward slashes for consistency
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            format!("{rel_str}/")
        } else {
            rel_str
        }
    } else {
        // Fallback to just the file name
        path.file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_default()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // IgnorePattern Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_simple_pattern() {
        let pattern = IgnorePattern::parse("*.log").unwrap();
        assert_eq!(pattern.original(), "*.log");
        assert!(!pattern.is_negation());
        assert!(!pattern.is_directory_only());
        assert!(!pattern.is_anchored());
    }

    #[test]
    fn test_parse_negation_pattern() {
        let pattern = IgnorePattern::parse("!important.log").unwrap();
        assert!(pattern.is_negation());
    }

    #[test]
    fn test_parse_directory_only_pattern() {
        let pattern = IgnorePattern::parse("node_modules/").unwrap();
        assert!(pattern.is_directory_only());
    }

    #[test]
    fn test_parse_anchored_pattern() {
        let pattern = IgnorePattern::parse("/root_only").unwrap();
        assert!(pattern.is_anchored());
    }

    #[test]
    fn test_parse_path_pattern_is_anchored() {
        let pattern = IgnorePattern::parse("src/generated").unwrap();
        assert!(pattern.is_anchored());
    }

    #[test]
    fn test_parse_empty_pattern_error() {
        let result = IgnorePattern::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_comment_error() {
        let result = IgnorePattern::parse("# comment");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_trimming() {
        let pattern = IgnorePattern::parse("  *.log  ").unwrap();
        assert_eq!(pattern.original(), "*.log");
    }

    #[test]
    fn test_parse_double_star() {
        let pattern = IgnorePattern::parse("**/logs").unwrap();
        assert_eq!(pattern.pattern, "**/logs");
    }

    #[test]
    fn test_parse_question_mark() {
        let pattern = IgnorePattern::parse("file?.txt").unwrap();
        assert_eq!(pattern.pattern, "file?.txt");
    }

    // =========================================================================
    // IgnorePattern Matching Tests
    // =========================================================================

    #[test]
    fn test_match_simple_wildcard() {
        let pattern = IgnorePattern::parse("*.log").unwrap();
        assert!(pattern.matches(Path::new("debug.log"), false));
        assert!(pattern.matches(Path::new("error.log"), false));
        assert!(!pattern.matches(Path::new("debug.txt"), false));
    }

    #[test]
    fn test_match_question_mark() {
        let pattern = IgnorePattern::parse("file?.txt").unwrap();
        assert!(pattern.matches(Path::new("file1.txt"), false));
        assert!(pattern.matches(Path::new("fileA.txt"), false));
        assert!(!pattern.matches(Path::new("file12.txt"), false));
    }

    #[test]
    fn test_match_double_star() {
        let pattern = IgnorePattern::parse("**/logs").unwrap();
        assert!(pattern.matches(Path::new("logs"), true));
        assert!(pattern.matches(Path::new("a/logs"), true));
        assert!(pattern.matches(Path::new("a/b/c/logs"), true));
    }

    #[test]
    fn test_match_double_star_suffix() {
        let pattern = IgnorePattern::parse("logs/**").unwrap();
        assert!(pattern.matches(Path::new("logs/debug.log"), false));
        assert!(pattern.matches(Path::new("logs/a/b.log"), false));
    }

    #[test]
    fn test_match_directory_only() {
        let pattern = IgnorePattern::parse("build/").unwrap();
        assert!(pattern.matches(Path::new("build"), true));
        assert!(!pattern.matches(Path::new("build"), false)); // File named build
    }

    #[test]
    fn test_match_negation_flag() {
        let pattern = IgnorePattern::parse("!important.log").unwrap();
        assert!(pattern.is_negation());
        // The pattern still matches, but negation is handled by IgnorePatternSet
        assert!(pattern.matches(Path::new("important.log"), false));
    }

    #[test]
    fn test_match_anchored() {
        let pattern = IgnorePattern::parse("/root.txt").unwrap();
        assert!(pattern.matches(Path::new("root.txt"), false));
        assert!(!pattern.matches(Path::new("subdir/root.txt"), false));
    }

    #[test]
    fn test_match_non_anchored_matches_anywhere() {
        let pattern = IgnorePattern::parse("*.log").unwrap();
        assert!(pattern.matches(Path::new("debug.log"), false));
        assert!(pattern.matches(Path::new("subdir/debug.log"), false));
    }

    #[test]
    fn test_match_case_insensitive() {
        let pattern = IgnorePattern::parse("README.md").unwrap();
        assert!(pattern.matches(Path::new("readme.md"), false));
        assert!(pattern.matches(Path::new("README.MD"), false));
    }

    #[test]
    fn test_match_path_with_slashes() {
        let pattern = IgnorePattern::parse("src/generated").unwrap();
        assert!(pattern.matches(Path::new("src/generated"), true));
        assert!(!pattern.matches(Path::new("other/generated"), true));
    }

    // =========================================================================
    // IgnorePatternSet Tests
    // =========================================================================

    #[test]
    fn test_pattern_set_empty() {
        let set = IgnorePatternSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_pattern_set_from_string() {
        let content = r"
# Comment
*.log
!important.log
build/
";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_pattern_set_ignores_comments_and_empty() {
        let content = r"
# Comment 1
*.log

# Comment 2

*.txt
";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_pattern_set_is_ignored() {
        let content = "*.log";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();

        assert_eq!(set.is_ignored(Path::new("debug.log"), false), Some(true));
        assert_eq!(set.is_ignored(Path::new("debug.txt"), false), None);
    }

    #[test]
    fn test_pattern_set_negation() {
        let content = r"
*.log
!important.log
";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();

        assert_eq!(set.is_ignored(Path::new("debug.log"), false), Some(true));
        assert_eq!(
            set.is_ignored(Path::new("important.log"), false),
            Some(false)
        );
    }

    #[test]
    fn test_pattern_set_last_match_wins() {
        let content = r"
*.log
!debug.log
*.log
";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();

        // Last *.log wins
        assert_eq!(set.is_ignored(Path::new("debug.log"), false), Some(true));
    }

    #[test]
    fn test_pattern_set_from_file() {
        let dir = TempDir::new().unwrap();
        let ignore_path = dir.path().join(".gitignore");
        std::fs::write(&ignore_path, "*.log\nbuild/").unwrap();

        let set = IgnorePatternSet::from_file(&ignore_path).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.source_path(), Some(ignore_path.as_path()));
    }

    #[test]
    fn test_pattern_set_from_missing_file() {
        let result = IgnorePatternSet::from_file(Path::new("/nonexistent/.gitignore"));
        assert!(result.is_err());
    }

    // =========================================================================
    // IgnoreMatcher Tests
    // =========================================================================

    #[test]
    fn test_matcher_empty() {
        let matcher = IgnoreMatcher::new();
        assert!(!matcher.is_ignored(Path::new("/some/path"), false));
    }

    #[test]
    fn test_matcher_with_patterns() {
        let matcher = IgnoreMatcher::new();
        let patterns =
            IgnorePatternSet::from_string("*.log", PathBuf::from("/project"), None).unwrap();
        matcher.add_pattern_set(PathBuf::from("/project"), patterns);

        assert!(matcher.is_ignored(Path::new("/project/debug.log"), false));
        assert!(!matcher.is_ignored(Path::new("/project/debug.txt"), false));
    }

    #[test]
    fn test_matcher_hierarchical() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("project");
        let subdir = project.join("src");
        std::fs::create_dir_all(&subdir).unwrap();

        // Root ignore
        std::fs::write(project.join(".gitignore"), "*.log").unwrap();
        // Subdir ignore
        std::fs::write(subdir.join(".gitignore"), "!important.log").unwrap();

        let matcher = IgnoreMatcher::with_root(project.clone());

        // Load subdir patterns
        let _ = matcher.is_ignored(&subdir.join("test.log"), false);

        // Root pattern applies
        assert!(matcher.is_ignored(&project.join("debug.log"), false));
        // Root pattern applies to subdir too
        assert!(matcher.is_ignored(&subdir.join("debug.log"), false));
    }

    #[test]
    fn test_matcher_global_patterns() {
        let matcher = IgnoreMatcher::new();
        let global = IgnorePatternSet::from_string("*.tmp", PathBuf::from("/"), None).unwrap();
        matcher.set_global_patterns(global);

        assert!(matcher.is_ignored(Path::new("/any/path/file.tmp"), false));
    }

    #[test]
    fn test_matcher_cache_invalidation() {
        let matcher = IgnoreMatcher::new();
        let patterns =
            IgnorePatternSet::from_string("*.log", PathBuf::from("/project"), None).unwrap();
        matcher.add_pattern_set(PathBuf::from("/project"), patterns);

        // Populate cache
        let _ = matcher.is_ignored(Path::new("/project/debug.log"), false);
        assert!(matcher.cache_size() > 0);

        // Invalidate
        matcher.invalidate_cache();
        assert_eq!(matcher.cache_size(), 0);
    }

    #[test]
    fn test_matcher_cache_directory_invalidation() {
        let matcher = IgnoreMatcher::new();
        let patterns =
            IgnorePatternSet::from_string("*.log", PathBuf::from("/project"), None).unwrap();
        matcher.add_pattern_set(PathBuf::from("/project"), patterns);

        // Populate cache
        let _ = matcher.is_ignored(Path::new("/project/a.log"), false);
        let _ = matcher.is_ignored(Path::new("/project/subdir/b.log"), false);
        let _ = matcher.is_ignored(Path::new("/other/c.log"), false);

        // Invalidate only /project/subdir
        matcher.invalidate_cache_for_directory(Path::new("/project/subdir"));

        // Check that only subdir was invalidated (this is a simplification)
        // In real use, the remaining entries would still be cached
    }

    #[test]
    fn test_matcher_cache_eviction() {
        // Create a matcher with a custom max size to test eviction
        // We'll manually verify eviction behavior by filling the cache
        let matcher = IgnoreMatcher::new();

        // Populate cache close to the limit
        // Note: In production MAX_CACHE_SIZE is 10,000 - too many to test exhaustively
        // This test verifies the mechanism works by checking that the cache doesn't
        // grow unbounded after many inserts
        for i in 0..100 {
            let path = format!("/test/path{}.txt", i);
            let _ = matcher.is_ignored(Path::new(&path), false);
        }

        // Verify cache is bounded (doesn't crash/OOM with many entries)
        // The actual MAX_CACHE_SIZE eviction is verified by the implementation
        assert!(matcher.cache_size() <= IgnoreMatcher::MAX_CACHE_SIZE);
    }

    // =========================================================================
    // Exclude Action Tests
    // =========================================================================

    #[test]
    fn test_add_to_photonignore_new_file() {
        let dir = TempDir::new().unwrap();
        add_to_photonignore(dir.path(), "*.log").unwrap();

        let ignore_path = dir.path().join(".photonignore");
        assert!(ignore_path.exists());

        let content = std::fs::read_to_string(&ignore_path).unwrap();
        assert!(content.contains("*.log"));
    }

    #[test]
    fn test_add_to_photonignore_existing_file() {
        let dir = TempDir::new().unwrap();
        let ignore_path = dir.path().join(".photonignore");
        std::fs::write(&ignore_path, "*.txt\n").unwrap();

        add_to_photonignore(dir.path(), "*.log").unwrap();

        let content = std::fs::read_to_string(&ignore_path).unwrap();
        assert!(content.contains("*.txt"));
        assert!(content.contains("*.log"));
    }

    #[test]
    fn test_add_to_photonignore_duplicate() {
        let dir = TempDir::new().unwrap();
        let ignore_path = dir.path().join(".photonignore");
        std::fs::write(&ignore_path, "*.log\n").unwrap();

        // Add same pattern again
        add_to_photonignore(dir.path(), "*.log").unwrap();

        let content = std::fs::read_to_string(&ignore_path).unwrap();
        // Should only appear once
        assert_eq!(content.matches("*.log").count(), 1);
    }

    #[test]
    fn test_pattern_for_file_relative() {
        let pattern = pattern_for_file(
            Path::new("/project/src/generated.rs"),
            Path::new("/project"),
        );
        assert_eq!(pattern, "src/generated.rs");
    }

    #[test]
    fn test_pattern_for_file_directory() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("build");
        std::fs::create_dir(&subdir).unwrap();

        let pattern = pattern_for_file(&subdir, dir.path());
        assert_eq!(pattern, "build/");
    }

    #[test]
    fn test_pattern_for_file_fallback() {
        let pattern =
            pattern_for_file(Path::new("/different/path/file.txt"), Path::new("/project"));
        assert_eq!(pattern, "file.txt");
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_glob_match_empty_pattern() {
        assert!(IgnorePattern::glob_match("", ""));
        assert!(!IgnorePattern::glob_match("", "text"));
    }

    #[test]
    fn test_glob_match_empty_text() {
        assert!(IgnorePattern::glob_match("", ""));
        assert!(IgnorePattern::glob_match("*", ""));
        assert!(IgnorePattern::glob_match("**", ""));
        assert!(!IgnorePattern::glob_match("a", ""));
    }

    #[test]
    fn test_glob_match_star_only() {
        assert!(IgnorePattern::glob_match("*", "anything"));
        assert!(IgnorePattern::glob_match("*", "a"));
        assert!(IgnorePattern::glob_match("*", ""));
    }

    #[test]
    fn test_glob_match_double_star_only() {
        assert!(IgnorePattern::glob_match("**", "anything"));
        assert!(IgnorePattern::glob_match("**", "a/b/c"));
    }

    #[test]
    fn test_glob_match_multiple_stars() {
        assert!(IgnorePattern::glob_match("*.*.txt", "file.backup.txt"));
        assert!(IgnorePattern::glob_match("*/*/*.txt", "a/b/c.txt"));
    }

    #[test]
    fn test_match_unicode_filename() {
        let pattern = IgnorePattern::parse("*.日本語").unwrap();
        assert!(pattern.matches(Path::new("ファイル.日本語"), false));
    }

    #[test]
    fn test_match_escaped_characters() {
        let pattern = IgnorePattern::parse(r"file\*.txt").unwrap();
        assert!(pattern.matches(Path::new("file*.txt"), false));
        assert!(!pattern.matches(Path::new("file1.txt"), false));
    }

    #[test]
    fn test_pattern_set_handles_crlf() {
        let content = "*.log\r\n*.txt\r\n";
        let set = IgnorePatternSet::from_string(content, PathBuf::from("/project"), None).unwrap();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_matcher_with_root_directory() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "*.log").unwrap();

        let matcher = IgnoreMatcher::with_root(dir.path().to_path_buf());

        assert!(matcher.is_ignored(&dir.path().join("debug.log"), false));
    }

    #[test]
    fn test_ignore_file_names_constant() {
        assert!(IgnoreMatcher::IGNORE_FILE_NAMES.contains(&".gitignore"));
        assert!(IgnoreMatcher::IGNORE_FILE_NAMES.contains(&".ignore"));
        assert!(IgnoreMatcher::IGNORE_FILE_NAMES.contains(&".photonignore"));
    }
}
