//! File query parser for natural language file search.
//!
//! This module provides a query parser that supports:
//! - File type filtering (`.pdf`, `.txt`)
//! - Location queries (`in ~/Desktop`, `in Documents`)
//! - Folder prioritization (trailing `/`)
//! - Parent folder search (`docs/bar`)
//! - Exact phrase matching (`"quoted text"`)
//!
//! # Examples
//!
//! ```
//! use photoncast_core::search::file_query::{FileQuery, FileTypeFilter};
//!
//! // Basic term search
//! let query = FileQuery::parse("report");
//! assert_eq!(query.terms, vec!["report"]);
//!
//! // File type filtering
//! let query = FileQuery::parse(".pdf certificate");
//! assert_eq!(query.file_type, Some(FileTypeFilter::Extension("pdf".to_string())));
//!
//! // Location query
//! let query = FileQuery::parse(".txt in ~/Desktop");
//! assert!(query.location.is_some());
//! ```

use std::path::{Path, PathBuf};

/// Supported file type filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTypeFilter {
    /// Filter by specific extension (e.g., "pdf", "txt").
    Extension(String),
    /// Filter by category.
    Category(FileCategory),
}

/// File type categories for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    /// Documents: pdf, doc, docx, xls, xlsx, ppt, pptx, txt, rtf, odt, pages, numbers, key
    Documents,
    /// Images: jpg, jpeg, png, gif, bmp, svg, webp, ico, tiff, heic, raw
    Images,
    /// Videos: mp4, mov, avi, mkv, wmv, flv, webm, m4v
    Videos,
    /// Audio: mp3, wav, flac, aac, ogg, m4a, wma, aiff
    Audio,
    /// Archives: zip, rar, 7z, tar, gz, bz2, xz, dmg, iso
    Archives,
    /// Code: rs, js, ts, py, rb, go, java, c, cpp, h, swift, kt, cs, php, html, css, json, yaml, toml, md
    Code,
    /// Folders only
    Folders,
}

impl FileCategory {
    /// Returns the file extensions associated with this category.
    #[must_use]
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Documents => &[
                "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "odt", "pages",
                "numbers", "key",
            ],
            Self::Images => &[
                "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "ico", "tiff", "heic", "raw",
            ],
            Self::Videos => &["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v"],
            Self::Audio => &["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff"],
            Self::Archives => &["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "dmg", "iso"],
            Self::Code => &[
                "rs", "js", "ts", "py", "rb", "go", "java", "c", "cpp", "h", "swift", "kt", "cs",
                "php", "html", "css", "json", "yaml", "toml", "md",
            ],
            Self::Folders => &[],
        }
    }

    /// Returns the UTI types for this category (for Spotlight queries).
    #[must_use]
    pub fn uti_types(&self) -> &'static [&'static str] {
        match self {
            Self::Documents => &["public.document", "com.adobe.pdf", "public.plain-text"],
            Self::Images => &["public.image"],
            Self::Videos => &["public.movie"],
            Self::Audio => &["public.audio"],
            Self::Archives => &["public.archive", "com.apple.disk-image-udif"],
            Self::Code => &["public.source-code", "public.script"],
            Self::Folders => &["public.folder"],
        }
    }
}

impl FileTypeFilter {
    /// Checks if a file extension matches this filter.
    #[must_use]
    pub fn matches_extension(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        match self {
            Self::Extension(filter_ext) => filter_ext.to_lowercase() == ext_lower,
            Self::Category(category) => category.extensions().contains(&ext_lower.as_str()),
        }
    }

    /// Returns the extension if this is an Extension filter.
    #[must_use]
    pub fn as_extension(&self) -> Option<&str> {
        match self {
            Self::Extension(ext) => Some(ext),
            Self::Category(_) => None,
        }
    }
}

/// Parsed file search query.
///
/// Contains all components extracted from a natural language file search query.
#[derive(Debug, Clone, Default)]
pub struct FileQuery {
    /// Search terms (words to match in file names).
    pub terms: Vec<String>,
    /// File type filter (e.g., `.pdf`, `.txt`).
    pub file_type: Option<FileTypeFilter>,
    /// Location to search in (e.g., `~/Desktop`, `Documents`).
    pub location: Option<PathBuf>,
    /// Whether to prioritize folders in results (query ends with `/`).
    pub prioritize_folders: bool,
    /// Parent folder filter (e.g., `docs` in `docs/bar`).
    pub parent_folder: Option<String>,
    /// Exact phrase to match (quoted string).
    pub exact_phrase: Option<String>,
}

impl FileQuery {
    /// Creates an empty `FileQuery`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a query string into a `FileQuery`.
    ///
    /// Supports the following syntax:
    /// - `foo` - Simple term search
    /// - `foo bar` - Multiple terms (AND)
    /// - `.pdf` - File type filter
    /// - `.pdf foo` - File type with term
    /// - `foo in ~/Desktop` - Location query
    /// - `foo/` - Folder prioritization
    /// - `docs/bar` - Parent folder search
    /// - `"exact phrase"` - Quoted exact match
    ///
    /// # Examples
    ///
    /// ```
    /// use photoncast_core::search::file_query::FileQuery;
    ///
    /// let query = FileQuery::parse(".pdf report in Documents");
    /// assert!(query.file_type.is_some());
    /// assert!(query.location.is_some());
    /// ```
    #[must_use]
    pub fn parse(query: &str) -> Self {
        let query = query.trim();

        if query.is_empty() {
            return Self::default();
        }

        let mut result = Self::default();

        // Check for folder prioritization (trailing /)
        let (query, prioritize_folders) = if query.ends_with('/') && !query.contains(' ') {
            // Only treat trailing / as folder prioritization if no spaces (not a path)
            let trimmed = query.trim_end_matches('/');
            (trimmed, true)
        } else {
            (query, false)
        };
        result.prioritize_folders = prioritize_folders;

        // Extract quoted phrases first
        let (query, exact_phrase) = Self::extract_quoted_phrase(query);
        result.exact_phrase = exact_phrase;

        // Check for "in location" pattern
        let (query, location) = Self::extract_location(&query);
        result.location = location;

        // Check for parent folder pattern (e.g., docs/bar, a/b/c)
        let (query, parent_folder) = Self::extract_parent_folder(&query);
        result.parent_folder = parent_folder;

        // Parse remaining tokens
        let tokens: Vec<&str> = query.split_whitespace().collect();

        for token in tokens {
            // Check for file type filter (starts with .)
            if let Some(ext) = token.strip_prefix('.') {
                if !ext.is_empty() && result.file_type.is_none() {
                    result.file_type = Some(FileTypeFilter::Extension(ext.to_lowercase()));
                    continue;
                }
            }

            // Regular search term
            if !token.is_empty() {
                result.terms.push(token.to_string());
            }
        }

        result
    }

    /// Extracts a quoted phrase from the query.
    ///
    /// Returns the remaining query and the extracted phrase (if any).
    fn extract_quoted_phrase(query: &str) -> (String, Option<String>) {
        // Find quoted strings
        let mut remaining = String::new();
        let mut phrase = None;
        let mut in_quotes = false;
        let mut current_phrase = String::new();

        for c in query.chars() {
            if c == '"' {
                if in_quotes {
                    // End of quoted phrase
                    if !current_phrase.is_empty() {
                        phrase = Some(current_phrase.clone());
                    }
                    current_phrase.clear();
                    in_quotes = false;
                } else {
                    // Start of quoted phrase
                    in_quotes = true;
                }
            } else if in_quotes {
                current_phrase.push(c);
            } else {
                remaining.push(c);
            }
        }

        // Handle unclosed quote
        if in_quotes && !current_phrase.is_empty() {
            phrase = Some(current_phrase);
        }

        (remaining.trim().to_string(), phrase)
    }

    /// Extracts location from "in <path>" pattern.
    ///
    /// Returns the remaining query and the extracted path (if any).
    fn extract_location(query: &str) -> (String, Option<PathBuf>) {
        // Look for " in " pattern (with spaces)
        if let Some(idx) = query.to_lowercase().find(" in ") {
            let before = query[..idx].trim();
            let after = query[idx + 4..].trim();

            if !after.is_empty() && Self::looks_like_path(after) {
                let path = Self::resolve_path(after);
                return (before.to_string(), Some(path));
            }
        }

        // Check if query starts with "in "
        let lower = query.to_lowercase();
        if lower.starts_with("in ") {
            let after = query[3..].trim();
            if !after.is_empty() && Self::looks_like_path(after) {
                let path = Self::resolve_path(after);
                return (String::new(), Some(path));
            }
        }

        (query.to_string(), None)
    }

    /// Checks if a string looks like a path reference.
    ///
    /// Returns true for:
    /// - Paths starting with ~ or /
    /// - Common folder names (Documents, Desktop, etc.)
    fn looks_like_path(s: &str) -> bool {
        // Starts with path indicators
        if s.starts_with('~') || s.starts_with('/') {
            return true;
        }

        // Check for common folder names (case-insensitive)
        let lower = s.to_lowercase();
        let first_word = lower.split_whitespace().next().unwrap_or("");

        matches!(
            first_word,
            "documents"
                | "desktop"
                | "downloads"
                | "pictures"
                | "photos"
                | "movies"
                | "videos"
                | "music"
                | "applications"
        )
    }

    /// Extracts parent folder from "folder/name" pattern.
    ///
    /// Returns the remaining query (just the search term) and the parent folder.
    fn extract_parent_folder(query: &str) -> (String, Option<String>) {
        // Skip if this looks like an absolute path or home path
        if query.starts_with('/') || query.starts_with('~') {
            return (query.to_string(), None);
        }

        // Look for the last / in the query (for patterns like a/b/c)
        if let Some(last_slash) = query.rfind('/') {
            let folder_part = &query[..last_slash];
            let name_part = &query[last_slash + 1..];

            // Only extract if we have both parts
            if !folder_part.is_empty() && !name_part.is_empty() {
                return (name_part.to_string(), Some(folder_part.to_string()));
            }
        }

        (query.to_string(), None)
    }

    /// Resolves a path string to a `PathBuf`.
    ///
    /// Handles:
    /// - `~` and `~/` for home directory
    /// - Common folder names (Documents, Desktop, Downloads, etc.)
    /// - Absolute paths
    fn resolve_path(path_str: &str) -> PathBuf {
        let path_str = path_str.trim();

        // Handle home directory
        if path_str == "~" || path_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                if path_str == "~" {
                    return home;
                }
                return home.join(&path_str[2..]);
            }
        }

        // Handle common folder names
        let lower = path_str.to_lowercase();
        if let Some(home) = dirs::home_dir() {
            match lower.as_str() {
                "documents" => return home.join("Documents"),
                "desktop" => return home.join("Desktop"),
                "downloads" => return home.join("Downloads"),
                "pictures" | "photos" => return home.join("Pictures"),
                "movies" | "videos" => return home.join("Movies"),
                "music" => return home.join("Music"),
                "applications" => return PathBuf::from("/Applications"),
                _ => {},
            }
        }

        // Return as-is for absolute paths or unknown patterns
        PathBuf::from(path_str)
    }

    /// Escapes special characters in a value for use in Spotlight predicates.
    ///
    /// Spotlight predicates use NSPredicate format where certain characters have special meaning:
    /// - `*` and `?` are wildcards
    /// - `\` is an escape character
    /// - `"` delimits strings
    fn escape_spotlight_value(value: &str) -> String {
        let mut result = String::with_capacity(value.len() * 2);
        for c in value.chars() {
            match c {
                '\\' => result.push_str("\\\\"),
                '"' => result.push_str("\\\""),
                '*' => result.push_str("\\*"),
                '?' => result.push_str("\\?"),
                _ => result.push(c),
            }
        }
        result
    }

    /// Generates a Spotlight predicate string for `mdfind`.
    ///
    /// This can be used with `mdfind -interpret` for complex queries.
    ///
    /// # Returns
    ///
    /// A predicate string suitable for Spotlight queries.
    #[must_use]
    pub fn to_spotlight_predicate(&self) -> String {
        let mut predicates: Vec<String> = Vec::new();

        // Add file type predicate
        if let Some(ref file_type) = self.file_type {
            match file_type {
                FileTypeFilter::Extension(ext) => {
                    let escaped_ext = Self::escape_spotlight_value(ext);
                    predicates.push(format!("kMDItemFSName == \"*.{escaped_ext}\"c"));
                },
                FileTypeFilter::Category(category) => {
                    if *category == FileCategory::Folders {
                        predicates.push("kMDItemContentType == \"public.folder\"".to_string());
                    } else {
                        let uti_predicates: Vec<String> = category
                            .uti_types()
                            .iter()
                            .map(|uti| format!("kMDItemContentTypeTree == \"{uti}\""))
                            .collect();
                        if !uti_predicates.is_empty() {
                            predicates.push(format!("({})", uti_predicates.join(" || ")));
                        }
                    }
                },
            }
        }

        // Add folder-only predicate if prioritizing folders
        if self.prioritize_folders && self.file_type.is_none() {
            predicates.push("kMDItemContentType == \"public.folder\"".to_string());
        }

        // Add exact phrase predicate
        if let Some(ref phrase) = self.exact_phrase {
            let escaped = Self::escape_spotlight_value(phrase);
            predicates.push(format!("kMDItemDisplayName == \"*{escaped}*\"c"));
        }

        // Add term predicates
        for term in &self.terms {
            let escaped = Self::escape_spotlight_value(term);
            predicates.push(format!("kMDItemDisplayName == \"*{escaped}*\"c"));
        }

        // Join with AND
        if predicates.is_empty() {
            "*".to_string() // Match all
        } else {
            predicates.join(" && ")
        }
    }

    /// Generates the `-name` argument for basic `mdfind` queries.
    ///
    /// This is simpler than the full predicate and works with `mdfind -name`.
    #[must_use]
    pub fn to_mdfind_name_query(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Add terms
        for term in &self.terms {
            parts.push(term.clone());
        }

        // Add exact phrase
        if let Some(ref phrase) = self.exact_phrase {
            parts.push(phrase.clone());
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(" ")
        }
    }

    /// Checks if a file matches this query.
    ///
    /// This is used for post-filtering Spotlight results or for
    /// filtering files during directory traversal.
    ///
    /// # Arguments
    ///
    /// * `path` - The full path to the file
    /// * `name` - The file name (for efficiency, to avoid re-extracting)
    ///
    /// # Returns
    ///
    /// `true` if the file matches all query criteria.
    #[must_use]
    pub fn matches_file(&self, path: &Path, name: &str) -> bool {
        let name_lower = name.to_lowercase();

        // Check file type filter
        if let Some(ref file_type) = self.file_type {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !file_type.matches_extension(ext) {
                    return false;
                }
            } else {
                // No extension, check if it's a folder filter
                if let FileTypeFilter::Category(FileCategory::Folders) = file_type {
                    if !path.is_dir() {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Check folder prioritization
        if self.prioritize_folders && !path.is_dir() {
            return false;
        }

        // Check exact phrase
        if let Some(ref phrase) = self.exact_phrase {
            if !name_lower.contains(&phrase.to_lowercase()) {
                return false;
            }
        }

        // Check search terms (all must match)
        for term in &self.terms {
            if !name_lower.contains(&term.to_lowercase()) {
                return false;
            }
        }

        // Check parent folder
        if let Some(ref parent) = self.parent_folder {
            let path_str = path.to_string_lossy().to_lowercase();
            let parent_lower = parent.to_lowercase();

            // Check if any ancestor folder matches the parent filter
            // Support multi-level patterns like "a/b" by checking if the path contains it
            if !path_str.contains(&parent_lower) {
                return false;
            }
        }

        // Check location
        if let Some(ref location) = self.location {
            if !path.starts_with(location) {
                return false;
            }
        }

        true
    }

    /// Returns true if this query is empty (no search criteria).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
            && self.file_type.is_none()
            && self.location.is_none()
            && self.exact_phrase.is_none()
            && self.parent_folder.is_none()
            && !self.prioritize_folders
    }

    /// Returns the primary search text for display purposes.
    #[must_use]
    pub fn display_text(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref phrase) = self.exact_phrase {
            parts.push(format!("\"{phrase}\""));
        }

        for term in &self.terms {
            parts.push(term.clone());
        }

        if let Some(FileTypeFilter::Extension(ext)) = &self.file_type {
            parts.push(format!(".{ext}"));
        }

        if let Some(ref location) = self.location {
            parts.push(format!("in {}", location.display()));
        }

        if let Some(ref parent) = self.parent_folder {
            parts.push(format!("in {parent}/"));
        }

        if self.prioritize_folders {
            parts.push("(folders)".to_string());
        }

        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_empty_query() {
        let query = FileQuery::parse("");
        assert!(query.terms.is_empty());
        assert!(query.file_type.is_none());
        assert!(query.location.is_none());
        assert!(!query.prioritize_folders);
        assert!(query.parent_folder.is_none());
        assert!(query.exact_phrase.is_none());
        assert!(query.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let query = FileQuery::parse("   ");
        assert!(query.is_empty());
    }

    #[test]
    fn test_parse_simple_term() {
        let query = FileQuery::parse("report");
        assert_eq!(query.terms, vec!["report"]);
        assert!(query.file_type.is_none());
        assert!(query.location.is_none());
    }

    #[test]
    fn test_parse_multiple_terms() {
        let query = FileQuery::parse("foo bar baz");
        assert_eq!(query.terms, vec!["foo", "bar", "baz"]);
    }

    // =========================================================================
    // File Type Filtering Tests
    // =========================================================================

    #[test]
    fn test_parse_file_type_only() {
        let query = FileQuery::parse(".pdf");
        assert!(query.terms.is_empty());
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
    }

    #[test]
    fn test_parse_file_type_with_term() {
        let query = FileQuery::parse(".pdf certificate");
        assert_eq!(query.terms, vec!["certificate"]);
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
    }

    #[test]
    fn test_parse_file_type_after_term() {
        let query = FileQuery::parse("certificate .pdf");
        assert_eq!(query.terms, vec!["certificate"]);
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
    }

    #[test]
    fn test_parse_file_type_uppercase() {
        let query = FileQuery::parse(".PDF");
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
    }

    #[test]
    fn test_parse_multiple_file_types_uses_first() {
        let query = FileQuery::parse(".pdf .txt");
        // Only first file type is used
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
        // Second becomes a term (with dot stripped logic doesn't apply here)
        // Actually .txt will be ignored as it's also a file type pattern
    }

    // =========================================================================
    // Location Query Tests
    // =========================================================================

    #[test]
    fn test_parse_location_home_tilde() {
        let query = FileQuery::parse("report in ~");
        assert_eq!(query.terms, vec!["report"]);
        assert!(query.location.is_some());
        // Location should be home directory
        assert_eq!(query.location, dirs::home_dir());
    }

    #[test]
    fn test_parse_location_home_with_path() {
        let query = FileQuery::parse(".txt in ~/Desktop");
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("txt".to_string()))
        );
        assert!(query.location.is_some());
        let expected = dirs::home_dir().map(|h| h.join("Desktop"));
        assert_eq!(query.location, expected);
    }

    #[test]
    fn test_parse_location_common_folder_documents() {
        let query = FileQuery::parse("report in Documents");
        assert_eq!(query.terms, vec!["report"]);
        let expected = dirs::home_dir().map(|h| h.join("Documents"));
        assert_eq!(query.location, expected);
    }

    #[test]
    fn test_parse_location_common_folder_downloads() {
        let query = FileQuery::parse("file in Downloads");
        let expected = dirs::home_dir().map(|h| h.join("Downloads"));
        assert_eq!(query.location, expected);
    }

    #[test]
    fn test_parse_location_common_folder_desktop() {
        let query = FileQuery::parse("file in Desktop");
        let expected = dirs::home_dir().map(|h| h.join("Desktop"));
        assert_eq!(query.location, expected);
    }

    #[test]
    fn test_parse_location_applications() {
        let query = FileQuery::parse("safari in Applications");
        assert_eq!(query.location, Some(PathBuf::from("/Applications")));
    }

    #[test]
    fn test_parse_location_absolute_path() {
        let query = FileQuery::parse("file in /tmp");
        assert_eq!(query.location, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_parse_location_only() {
        let query = FileQuery::parse("in Desktop");
        assert!(query.terms.is_empty());
        let expected = dirs::home_dir().map(|h| h.join("Desktop"));
        assert_eq!(query.location, expected);
    }

    // =========================================================================
    // Folder Prioritization Tests
    // =========================================================================

    #[test]
    fn test_parse_folder_prioritization() {
        let query = FileQuery::parse("downloads/");
        assert!(query.prioritize_folders);
        assert_eq!(query.terms, vec!["downloads"]);
    }

    #[test]
    fn test_parse_folder_prioritization_preserves_term() {
        let query = FileQuery::parse("projects/");
        assert!(query.prioritize_folders);
        assert_eq!(query.terms, vec!["projects"]);
    }

    #[test]
    fn test_parse_no_folder_prioritization_with_spaces() {
        // Trailing / with spaces should not trigger folder prioritization
        let query = FileQuery::parse("foo bar/");
        assert!(!query.prioritize_folders);
    }

    // =========================================================================
    // Parent Folder Search Tests
    // =========================================================================

    #[test]
    fn test_parse_parent_folder_simple() {
        let query = FileQuery::parse("docs/bar");
        assert_eq!(query.parent_folder, Some("docs".to_string()));
        assert_eq!(query.terms, vec!["bar"]);
    }

    #[test]
    fn test_parse_parent_folder_multi_level() {
        let query = FileQuery::parse("a/b/c");
        assert_eq!(query.parent_folder, Some("a/b".to_string()));
        assert_eq!(query.terms, vec!["c"]);
    }

    #[test]
    fn test_parse_parent_folder_not_absolute_path() {
        // Absolute paths should not be treated as parent folder patterns
        let query = FileQuery::parse("/Users/test");
        assert!(query.parent_folder.is_none());
    }

    #[test]
    fn test_parse_parent_folder_not_home_path() {
        // Home paths should not be treated as parent folder patterns
        let query = FileQuery::parse("~/Documents");
        assert!(query.parent_folder.is_none());
    }

    // =========================================================================
    // Exact Phrase Tests
    // =========================================================================

    #[test]
    fn test_parse_exact_phrase() {
        let query = FileQuery::parse("\"exact phrase\"");
        assert_eq!(query.exact_phrase, Some("exact phrase".to_string()));
        assert!(query.terms.is_empty());
    }

    #[test]
    fn test_parse_exact_phrase_with_terms() {
        let query = FileQuery::parse("foo \"exact phrase\" bar");
        assert_eq!(query.exact_phrase, Some("exact phrase".to_string()));
        assert_eq!(query.terms, vec!["foo", "bar"]);
    }

    #[test]
    fn test_parse_exact_phrase_unclosed() {
        let query = FileQuery::parse("\"unclosed phrase");
        assert_eq!(query.exact_phrase, Some("unclosed phrase".to_string()));
    }

    #[test]
    fn test_parse_exact_phrase_empty() {
        let query = FileQuery::parse("foo \"\" bar");
        assert!(query.exact_phrase.is_none());
        assert_eq!(query.terms, vec!["foo", "bar"]);
    }

    // =========================================================================
    // Complex Query Tests
    // =========================================================================

    #[test]
    fn test_parse_complex_query() {
        let query = FileQuery::parse(".pdf certificate in Documents");
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
        assert_eq!(query.terms, vec!["certificate"]);
        let expected = dirs::home_dir().map(|h| h.join("Documents"));
        assert_eq!(query.location, expected);
    }

    #[test]
    fn test_parse_all_features() {
        let query = FileQuery::parse(".pdf \"annual report\" budget in Documents");
        assert_eq!(
            query.file_type,
            Some(FileTypeFilter::Extension("pdf".to_string()))
        );
        assert_eq!(query.exact_phrase, Some("annual report".to_string()));
        assert_eq!(query.terms, vec!["budget"]);
        assert!(query.location.is_some());
    }

    // =========================================================================
    // Spotlight Predicate Tests
    // =========================================================================

    #[test]
    fn test_spotlight_predicate_empty() {
        let query = FileQuery::parse("");
        assert_eq!(query.to_spotlight_predicate(), "*");
    }

    #[test]
    fn test_spotlight_predicate_simple_term() {
        let query = FileQuery::parse("report");
        let predicate = query.to_spotlight_predicate();
        assert!(predicate.contains("kMDItemDisplayName"));
        assert!(predicate.contains("report"));
    }

    #[test]
    fn test_spotlight_predicate_file_type() {
        let query = FileQuery::parse(".pdf");
        let predicate = query.to_spotlight_predicate();
        assert!(predicate.contains("kMDItemFSName"));
        assert!(predicate.contains("*.pdf"));
    }

    #[test]
    fn test_spotlight_predicate_folder_priority() {
        let query = FileQuery::parse("downloads/");
        let predicate = query.to_spotlight_predicate();
        assert!(predicate.contains("public.folder"));
    }

    #[test]
    fn test_spotlight_predicate_escapes_special_chars() {
        // Test escaping of wildcards and special characters
        let query = FileQuery::parse("foo*bar");
        let predicate = query.to_spotlight_predicate();
        // The `*` in the term should be escaped as `\*`
        assert!(predicate.contains("foo\\*bar"), "predicate: {}", predicate);

        let query = FileQuery::parse("what?");
        let predicate = query.to_spotlight_predicate();
        // The `?` should be escaped
        assert!(predicate.contains("what\\?"), "predicate: {}", predicate);

        let query = FileQuery::parse("path\\to");
        let predicate = query.to_spotlight_predicate();
        // The `\` should be escaped as `\\`
        assert!(predicate.contains("path\\\\to"), "predicate: {}", predicate);

        // Test quote escaping by constructing the query directly
        // (parsing "hello"world" would interpret it as a quoted phrase)
        let mut query = FileQuery::new();
        query.terms.push("hello\"world".to_string());
        let predicate = query.to_spotlight_predicate();
        // Quotes in the term should be escaped
        assert!(
            predicate.contains("hello\\\"world"),
            "predicate: {}",
            predicate
        );
    }

    // =========================================================================
    // mdfind Name Query Tests
    // =========================================================================

    #[test]
    fn test_mdfind_name_query_simple() {
        let query = FileQuery::parse("report");
        assert_eq!(query.to_mdfind_name_query(), "report");
    }

    #[test]
    fn test_mdfind_name_query_multiple_terms() {
        let query = FileQuery::parse("foo bar");
        assert_eq!(query.to_mdfind_name_query(), "foo bar");
    }

    #[test]
    fn test_mdfind_name_query_with_phrase() {
        let query = FileQuery::parse("\"exact phrase\"");
        assert_eq!(query.to_mdfind_name_query(), "exact phrase");
    }

    #[test]
    fn test_mdfind_name_query_empty() {
        let query = FileQuery::parse(".pdf");
        // File type only, no name query
        assert_eq!(query.to_mdfind_name_query(), "");
    }

    // =========================================================================
    // File Matching Tests
    // =========================================================================

    #[test]
    fn test_matches_file_simple_term() {
        let query = FileQuery::parse("report");
        assert!(query.matches_file(Path::new("/test/report.pdf"), "report.pdf"));
        assert!(query.matches_file(Path::new("/test/my_report.txt"), "my_report.txt"));
        assert!(!query.matches_file(Path::new("/test/document.pdf"), "document.pdf"));
    }

    #[test]
    fn test_matches_file_case_insensitive() {
        let query = FileQuery::parse("REPORT");
        assert!(query.matches_file(Path::new("/test/report.pdf"), "report.pdf"));
        assert!(query.matches_file(Path::new("/test/Report.pdf"), "Report.pdf"));
    }

    #[test]
    fn test_matches_file_with_extension() {
        let query = FileQuery::parse(".pdf");
        assert!(query.matches_file(Path::new("/test/report.pdf"), "report.pdf"));
        assert!(!query.matches_file(Path::new("/test/report.txt"), "report.txt"));
    }

    #[test]
    fn test_matches_file_with_location() {
        let mut query = FileQuery::default();
        query.terms = vec!["report".to_string()];
        query.location = Some(PathBuf::from("/Users/test/Documents"));

        assert!(query.matches_file(Path::new("/Users/test/Documents/report.pdf"), "report.pdf"));
        assert!(!query.matches_file(Path::new("/Users/test/Desktop/report.pdf"), "report.pdf"));
    }

    #[test]
    fn test_matches_file_with_parent_folder() {
        let query = FileQuery::parse("docs/report");
        assert!(query.matches_file(Path::new("/Users/test/docs/report.pdf"), "report.pdf"));
        assert!(query.matches_file(Path::new("/Users/test/my_docs/report.txt"), "report.txt"));
        assert!(!query.matches_file(Path::new("/Users/test/other/report.pdf"), "report.pdf"));
    }

    #[test]
    fn test_matches_file_exact_phrase() {
        let query = FileQuery::parse("\"annual report\"");
        assert!(query.matches_file(
            Path::new("/test/annual report 2024.pdf"),
            "annual report 2024.pdf"
        ));
        assert!(!query.matches_file(Path::new("/test/report annual.pdf"), "report annual.pdf"));
    }

    #[test]
    fn test_matches_file_multiple_terms() {
        let query = FileQuery::parse("annual report");
        // Both terms must match
        assert!(query.matches_file(Path::new("/test/annual_report.pdf"), "annual_report.pdf"));
        assert!(query.matches_file(Path::new("/test/report_annual.pdf"), "report_annual.pdf"));
        assert!(!query.matches_file(Path::new("/test/annual_summary.pdf"), "annual_summary.pdf"));
    }

    // =========================================================================
    // FileTypeFilter Tests
    // =========================================================================

    #[test]
    fn test_file_type_filter_extension_matches() {
        let filter = FileTypeFilter::Extension("pdf".to_string());
        assert!(filter.matches_extension("pdf"));
        assert!(filter.matches_extension("PDF"));
        assert!(!filter.matches_extension("txt"));
    }

    #[test]
    fn test_file_type_filter_category_matches() {
        let filter = FileTypeFilter::Category(FileCategory::Documents);
        assert!(filter.matches_extension("pdf"));
        assert!(filter.matches_extension("doc"));
        assert!(filter.matches_extension("txt"));
        assert!(!filter.matches_extension("jpg"));
    }

    // =========================================================================
    // FileCategory Tests
    // =========================================================================

    #[test]
    fn test_file_category_extensions() {
        assert!(FileCategory::Documents.extensions().contains(&"pdf"));
        assert!(FileCategory::Images.extensions().contains(&"jpg"));
        assert!(FileCategory::Videos.extensions().contains(&"mp4"));
        assert!(FileCategory::Audio.extensions().contains(&"mp3"));
        assert!(FileCategory::Archives.extensions().contains(&"zip"));
        assert!(FileCategory::Code.extensions().contains(&"rs"));
        assert!(FileCategory::Folders.extensions().is_empty());
    }

    #[test]
    fn test_file_category_uti_types() {
        assert!(!FileCategory::Documents.uti_types().is_empty());
        assert!(FileCategory::Images.uti_types().contains(&"public.image"));
        assert!(FileCategory::Folders.uti_types().contains(&"public.folder"));
    }

    // =========================================================================
    // Display Text Tests
    // =========================================================================

    #[test]
    fn test_display_text_simple() {
        let query = FileQuery::parse("report");
        assert_eq!(query.display_text(), "report");
    }

    #[test]
    fn test_display_text_complex() {
        let query = FileQuery::parse(".pdf \"annual\" report");
        let display = query.display_text();
        assert!(display.contains("\"annual\""));
        assert!(display.contains("report"));
        assert!(display.contains(".pdf"));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_dot_only() {
        let query = FileQuery::parse(".");
        // Single dot should not be treated as file type
        assert!(query.file_type.is_none());
    }

    #[test]
    fn test_parse_in_as_term() {
        // "in" without following path should be treated as term
        let query = FileQuery::parse("sign in form");
        // "in form" is not a valid location (form is not a known folder)
        assert!(query.location.is_none());
        // All words should be terms (note: "in" gets included in the before part)
        assert!(query.terms.contains(&"sign".to_string()));
        // "in" and "form" are included as part of the remaining query
        assert!(query.terms.contains(&"in".to_string()));
        assert!(query.terms.contains(&"form".to_string()));
    }

    #[test]
    fn test_parse_slash_in_term() {
        // Make sure we handle edge cases with slashes
        let query = FileQuery::parse("config.json");
        assert!(query.parent_folder.is_none());
        assert_eq!(query.terms, vec!["config.json"]);
    }
}
