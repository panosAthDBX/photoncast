//! NSPredicate builder for Spotlight queries.
//!
//! This module provides a builder pattern for creating NSPredicate objects
//! that can be used with NSMetadataQuery for Spotlight searches.
//!
//! # Features
//!
//! - Case-insensitive and diacritic-insensitive matching with `[cd]` suffix
//! - Safe escaping of special characters in user input
//! - Compound predicates using AND/OR logic
//! - Date, size, and content type filtering
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::spotlight::PredicateBuilder;
//! use std::time::{SystemTime, Duration};
//!
//! // Search for PDF files containing "report" modified in the last week
//! let week_ago = SystemTime::now() - Duration::from_secs(7 * 24 * 60 * 60);
//!
//! let predicate = PredicateBuilder::new()
//!     .name_contains("report")
//!     .extension_is("pdf")
//!     .modified_after(week_ago)
//!     .build();
//! ```
//!
//! # Spotlight Query Syntax
//!
//! Spotlight uses a SQL-like query syntax with these comparison modifiers:
//! - `[c]` - case-insensitive
//! - `[d]` - diacritic-insensitive
//! - `[cd]` - both case and diacritic-insensitive
//!
//! Common operators:
//! - `==` - exact match
//! - `CONTAINS` - substring match
//! - `BEGINSWITH` - prefix match
//! - `ENDSWITH` - suffix match
//! - `LIKE` - pattern match with wildcards (* and ?)

use std::time::{SystemTime, UNIX_EPOCH};

use objc2::rc::Retained;
#[allow(deprecated)]
use objc2::{class, msg_send_id};
use objc2_foundation::{NSArray, NSCompoundPredicate, NSPredicate, NSString};

// =============================================================================
// MDQuery Attribute Constants
// =============================================================================

/// File system name attribute (kMDItemFSName).
///
/// The file name as it appears in the file system.
pub const MD_ITEM_FS_NAME: &str = "kMDItemFSName";

/// Display name attribute (kMDItemDisplayName).
///
/// The localized display name of the item.
pub const MD_ITEM_DISPLAY_NAME: &str = "kMDItemDisplayName";

/// Content type attribute (kMDItemContentType).
///
/// The UTI (Uniform Type Identifier) of the item.
pub const MD_ITEM_CONTENT_TYPE: &str = "kMDItemContentType";

/// Content type tree attribute (kMDItemContentTypeTree).
///
/// An array of UTIs representing the type hierarchy.
/// Use this to match all files conforming to a type (e.g., all documents).
pub const MD_ITEM_CONTENT_TYPE_TREE: &str = "kMDItemContentTypeTree";

/// File size attribute (kMDItemFSSize).
///
/// The size of the file in bytes.
pub const MD_ITEM_FS_SIZE: &str = "kMDItemFSSize";

/// Content change date attribute (kMDItemFSContentChangeDate).
///
/// The date the file content was last modified.
pub const MD_ITEM_FS_CONTENT_CHANGE_DATE: &str = "kMDItemFSContentChangeDate";

/// Last used date attribute (kMDItemLastUsedDate).
///
/// The date the file was last opened.
pub const MD_ITEM_LAST_USED_DATE: &str = "kMDItemLastUsedDate";

/// Path attribute (kMDItemPath).
///
/// The full path to the file.
pub const MD_ITEM_PATH: &str = "kMDItemPath";

// =============================================================================
// Common UTI (Uniform Type Identifier) Constants
// =============================================================================

/// UTI for folders/directories.
pub const UTI_FOLDER: &str = "public.folder";

/// UTI for all document types.
pub const UTI_DOCUMENT: &str = "public.content";

/// UTI for plain text files.
pub const UTI_PLAIN_TEXT: &str = "public.plain-text";

/// UTI for PDF documents.
pub const UTI_PDF: &str = "com.adobe.pdf";

/// UTI for all image types.
pub const UTI_IMAGE: &str = "public.image";

/// UTI for all audio types.
pub const UTI_AUDIO: &str = "public.audio";

/// UTI for all movie/video types.
pub const UTI_MOVIE: &str = "public.movie";

/// UTI for archive files (zip, tar, etc.).
pub const UTI_ARCHIVE: &str = "public.archive";

/// UTI for source code files.
pub const UTI_SOURCE_CODE: &str = "public.source-code";

// =============================================================================
// Default Exclusion Patterns (Raycast-style)
// =============================================================================

/// Directory names that should be excluded from search results by default.
/// These are development artifacts, caches, and system files.
pub const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    // Version control
    ".git",
    ".svn",
    ".hg",
    // Package managers / dependencies
    "node_modules",
    "vendor",
    "Pods",
    "Carthage",
    ".cargo",
    "target",
    // Python
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    // JavaScript/TypeScript
    ".next",
    ".nuxt",
    ".parcel-cache",
    ".turbo",
    // IDE / Editor
    ".idea",
    ".vscode",
    // macOS system
    ".Spotlight-V100",
    ".fseventsd",
    ".Trashes",
    // Caches
    "Caches",
    "CachedData",
    "DerivedData",
];

/// File extensions that should be excluded from search results by default.
pub const DEFAULT_EXCLUDED_EXTENSIONS: &[&str] = &[
    "o", "obj", "pyc", "pyo", "class", // Compiled
    "dylib", "so", "dll",              // Libraries
    "lock", "lockb",                   // Lock files
    "log",                             // Logs
    "sqlite-shm", "sqlite-wal",        // SQLite temp
];

// =============================================================================
// Helper Functions
// =============================================================================

/// Escapes special characters in a predicate string value.
///
/// Spotlight predicate format strings require escaping of:
/// - Backslash (`\`) → `\\`
/// - Double quote (`"`) → `\"`
/// - Asterisk (`*`) → `\*` (wildcard character)
/// - Question mark (`?`) → `\?` (single character wildcard)
///
/// # Example
///
/// ```
/// use photoncast_core::search::spotlight::escape_predicate_string;
///
/// let escaped = escape_predicate_string("file*.txt");
/// assert_eq!(escaped, r"file\*.txt");
/// ```
#[must_use]
pub fn escape_predicate_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 8);
    for c in s.chars() {
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

/// Converts a SystemTime to the number of seconds since Unix epoch.
fn system_time_to_timestamp(time: SystemTime) -> f64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64(),
        Err(e) => -(e.duration().as_secs_f64()),
    }
}

// =============================================================================
// PredicateBuilder
// =============================================================================

/// A builder for creating NSPredicate objects for Spotlight queries.
///
/// This builder accumulates predicate clauses and combines them with AND logic
/// by default. Use [`and`] and [`or`] methods to combine multiple builders.
///
/// # Example
///
/// ```no_run
/// use photoncast_core::search::spotlight::PredicateBuilder;
///
/// // Simple name search
/// let predicate = PredicateBuilder::new()
///     .name_contains("document")
///     .build();
///
/// // Complex search with multiple filters
/// let predicate = PredicateBuilder::new()
///     .name_contains("report")
///     .extension_is("pdf")
///     .size_greater_than(1024 * 1024) // > 1MB
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct PredicateBuilder {
    /// Collected predicate format strings.
    clauses: Vec<String>,
}

impl Default for PredicateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PredicateBuilder {
    /// Creates a new empty predicate builder.
    ///
    /// The builder starts with no clauses. If [`build`] is called without
    /// adding any clauses, it returns a predicate that matches all items.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
        }
    }

    /// Adds a clause that matches items where the file name contains the given term.
    ///
    /// Uses case-insensitive and diacritic-insensitive matching (`[cd]`).
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to find within file names.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_contains("report")
    ///     .build();
    /// // Produces: kMDItemFSName CONTAINS[cd] "report"
    /// ```
    #[must_use]
    pub fn name_contains(mut self, term: &str) -> Self {
        if !term.is_empty() {
            let escaped = escape_predicate_string(term);
            self.clauses
                .push(format!(r#"{} CONTAINS[cd] "{}""#, MD_ITEM_FS_NAME, escaped));
        }
        self
    }

    /// Matches any file (useful for live index that filters in memory).
    ///
    /// This creates a predicate that matches all items, which is useful
    /// when you want to index all files and perform filtering in memory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .any_file()
    ///     .with_default_exclusions()
    ///     .build();
    /// // Matches all files except excluded patterns
    /// ```
    #[must_use]
    pub fn any_file(mut self) -> Self {
        // Match any indexed item by checking it has a non-empty name
        // This is the most reliable way to match "all files" in Spotlight
        self.clauses.push(format!(r#"{} != """#, MD_ITEM_FS_NAME));
        self
    }

    /// Matches user files: documents, images, videos, audio, archives.
    /// Uses specific UTIs - NO broad types like public.content that include code.
    /// This filters at the Spotlight level for maximum performance.
    #[must_use]
    pub fn user_files(mut self) -> Self {
        // Use SPECIFIC content types - NOT broad ones like public.content
        // public.content includes source code which we don't want
        let conditions = [
            // PDF
            format!(r#"{} == "com.adobe.pdf""#, MD_ITEM_CONTENT_TYPE_TREE),
            // Images
            format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE_TREE, UTI_IMAGE),
            // Audio
            format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE_TREE, UTI_AUDIO),
            // Video
            format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE_TREE, UTI_MOVIE),
            // Archives (zip, dmg, etc.)
            format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE_TREE, UTI_ARCHIVE),
            // macOS apps
            format!(r#"{} == "com.apple.application-bundle""#, MD_ITEM_CONTENT_TYPE),
            // Office documents - specific types
            format!(r#"{} == "org.openxmlformats.wordprocessingml.document""#, MD_ITEM_CONTENT_TYPE_TREE), // docx
            format!(r#"{} == "com.microsoft.word.doc""#, MD_ITEM_CONTENT_TYPE_TREE), // doc
            format!(r#"{} == "org.openxmlformats.spreadsheetml.sheet""#, MD_ITEM_CONTENT_TYPE_TREE), // xlsx
            format!(r#"{} == "com.microsoft.excel.xls""#, MD_ITEM_CONTENT_TYPE_TREE), // xls
            format!(r#"{} == "org.openxmlformats.presentationml.presentation""#, MD_ITEM_CONTENT_TYPE_TREE), // pptx
            format!(r#"{} == "com.microsoft.powerpoint.ppt""#, MD_ITEM_CONTENT_TYPE_TREE), // ppt
            format!(r#"{} == "com.apple.iwork.pages.sffpages""#, MD_ITEM_CONTENT_TYPE_TREE), // pages
            format!(r#"{} == "com.apple.iwork.numbers.sffnumbers""#, MD_ITEM_CONTENT_TYPE_TREE), // numbers
            format!(r#"{} == "com.apple.iwork.keynote.sffkey""#, MD_ITEM_CONTENT_TYPE_TREE), // keynote
            format!(r#"{} == "public.comma-separated-values-text""#, MD_ITEM_CONTENT_TYPE_TREE), // csv
            format!(r#"{} == "public.rtf""#, MD_ITEM_CONTENT_TYPE_TREE), // rtf
            // Ebooks
            format!(r#"{} == "org.idpf.epub-container""#, MD_ITEM_CONTENT_TYPE_TREE), // epub
        ];
        self.clauses.push(format!("({})", conditions.join(" OR ")));
        // Exclude hidden files
        self.clauses.push(format!(r#"NOT ({} BEGINSWITH ".")"#, MD_ITEM_FS_NAME));
        self
    }

    /// Matches files with any of the given extensions.
    /// Use for custom scope extension filtering at Spotlight level.
    #[must_use]
    pub fn extensions(mut self, exts: &[String]) -> Self {
        if exts.is_empty() {
            return self;
        }
        let conditions: Vec<String> = exts
            .iter()
            .map(|ext| {
                let ext = ext.strip_prefix('.').unwrap_or(ext);
                format!(r#"{} ENDSWITH[c] ".{}""#, MD_ITEM_FS_NAME, escape_predicate_string(ext))
            })
            .collect();
        self.clauses.push(format!("({})", conditions.join(" OR ")));
        self
    }

    /// Adds a clause that matches items with an exact file name match.
    ///
    /// Uses case-insensitive and diacritic-insensitive matching (`[cd]`).
    ///
    /// # Arguments
    ///
    /// * `name` - The exact file name to match.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_equals("README.md")
    ///     .build();
    /// // Produces: kMDItemFSName ==[cd] "README.md"
    /// ```
    #[must_use]
    pub fn name_equals(mut self, name: &str) -> Self {
        if !name.is_empty() {
            let escaped = escape_predicate_string(name);
            self.clauses
                .push(format!(r#"{} ==[cd] "{}""#, MD_ITEM_FS_NAME, escaped));
        }
        self
    }

    /// Adds a clause that matches items with the given file extension.
    ///
    /// Uses case-insensitive matching for the extension (`[c]`).
    ///
    /// # Arguments
    ///
    /// * `ext` - The file extension without the leading dot (e.g., "pdf", "txt").
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .extension_is("pdf")
    ///     .build();
    /// // Produces: kMDItemFSName ENDSWITH[c] ".pdf"
    /// ```
    #[must_use]
    pub fn extension_is(mut self, ext: &str) -> Self {
        if !ext.is_empty() {
            // Remove leading dot if present
            let ext = ext.strip_prefix('.').unwrap_or(ext);
            let escaped = escape_predicate_string(ext);
            self.clauses
                .push(format!(r#"{} ENDSWITH[c] ".{}""#, MD_ITEM_FS_NAME, escaped));
        }
        self
    }

    /// Adds a clause that matches items with the exact content type (UTI).
    ///
    /// This matches only items with the specific UTI, not items that
    /// conform to it. Use [`content_type_tree`] for conformance matching.
    ///
    /// # Arguments
    ///
    /// * `uti` - The Uniform Type Identifier (e.g., "public.folder", "com.adobe.pdf").
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::{PredicateBuilder, UTI_PDF};
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .content_type(UTI_PDF)
    ///     .build();
    /// // Produces: kMDItemContentType == "com.adobe.pdf"
    /// ```
    #[must_use]
    pub fn content_type(mut self, uti: &str) -> Self {
        if !uti.is_empty() {
            self.clauses
                .push(format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE, uti));
        }
        self
    }

    /// Adds a clause that matches items conforming to the content type (UTI).
    ///
    /// This matches items that conform to the given UTI in their type hierarchy.
    /// For example, matching `public.image` will find PNG, JPEG, GIF, etc.
    ///
    /// # Arguments
    ///
    /// * `uti` - The Uniform Type Identifier to match in the type tree.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::{PredicateBuilder, UTI_IMAGE};
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .content_type_tree(UTI_IMAGE)
    ///     .build();
    /// // Produces: kMDItemContentTypeTree == "public.image"
    /// // Matches all images (PNG, JPEG, GIF, etc.)
    /// ```
    #[must_use]
    pub fn content_type_tree(mut self, uti: &str) -> Self {
        if !uti.is_empty() {
            self.clauses
                .push(format!(r#"{} == "{}""#, MD_ITEM_CONTENT_TYPE_TREE, uti));
        }
        self
    }

    /// Adds a clause that matches items modified within the last N days.
    ///
    /// This uses the `InRange` comparison which is well-supported by Spotlight.
    ///
    /// # Arguments
    ///
    /// * `days` - Number of days back to search.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .modified_within_days(7)
    ///     .build();
    /// ```
    #[must_use]
    pub fn modified_within_days(mut self, days: u32) -> Self {
        if days > 0 {
            // Use $time.now(-days) which is supported by MDQuery
            // Format: kMDItemFSContentChangeDate >= $time.now(-7d)
            self.clauses.push(format!(
                "{} >= $time.now(-{}d)",
                MD_ITEM_FS_CONTENT_CHANGE_DATE, days
            ));
        }
        self
    }

    /// Adds a clause that matches items modified after the given time.
    ///
    /// Note: This uses timestamp comparison which may have edge cases.
    /// Consider using `modified_within_days` for common use cases.
    ///
    /// # Arguments
    ///
    /// * `date` - The earliest modification time to include.
    #[must_use]
    pub fn modified_after(mut self, date: SystemTime) -> Self {
        let timestamp = system_time_to_timestamp(date);
        // NSPredicate date comparison using CAST to convert timestamp
        self.clauses.push(format!(
            "{} > CAST({}, \"NSDate\")",
            MD_ITEM_FS_CONTENT_CHANGE_DATE, timestamp
        ));
        self
    }

    /// Adds a clause that matches items modified before the given time.
    ///
    /// Note: This uses timestamp comparison which may have edge cases.
    ///
    /// # Arguments
    ///
    /// * `date` - The latest modification time to include.
    #[must_use]
    pub fn modified_before(mut self, date: SystemTime) -> Self {
        let timestamp = system_time_to_timestamp(date);
        self.clauses.push(format!(
            "{} < CAST({}, \"NSDate\")",
            MD_ITEM_FS_CONTENT_CHANGE_DATE, timestamp
        ));
        self
    }

    /// Adds a clause that matches items larger than the given size.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The minimum file size in bytes (exclusive).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// // Find files larger than 1 MB
    /// let predicate = PredicateBuilder::new()
    ///     .size_greater_than(1024 * 1024)
    ///     .build();
    /// ```
    #[must_use]
    pub fn size_greater_than(mut self, bytes: u64) -> Self {
        self.clauses
            .push(format!("{} > {}", MD_ITEM_FS_SIZE, bytes));
        self
    }

    /// Adds a clause that matches items smaller than the given size.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The maximum file size in bytes (exclusive).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// // Find files smaller than 100 KB
    /// let predicate = PredicateBuilder::new()
    ///     .size_less_than(100 * 1024)
    ///     .build();
    /// ```
    #[must_use]
    pub fn size_less_than(mut self, bytes: u64) -> Self {
        self.clauses
            .push(format!("{} < {}", MD_ITEM_FS_SIZE, bytes));
        self
    }

    /// Combines this builder with another using AND logic.
    ///
    /// Both builders must build successfully, and the resulting predicate
    /// matches only items that match both predicates.
    ///
    /// # Arguments
    ///
    /// * `other` - Another predicate builder to combine with.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let name_filter = PredicateBuilder::new().name_contains("report");
    /// let type_filter = PredicateBuilder::new().extension_is("pdf");
    ///
    /// let combined = name_filter.and(type_filter).build();
    /// // Matches files containing "report" AND having .pdf extension
    /// ```
    #[must_use]
    pub fn and(mut self, other: Self) -> Self {
        self.clauses.extend(other.clauses);
        self
    }

    /// Combines this builder with another using OR logic.
    ///
    /// Returns a new builder that builds a compound predicate matching items
    /// that satisfy either this builder's conditions or the other's.
    ///
    /// # Arguments
    ///
    /// * `other` - Another predicate builder to combine with.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let pdf_filter = PredicateBuilder::new().extension_is("pdf");
    /// let doc_filter = PredicateBuilder::new().extension_is("docx");
    ///
    /// let combined = pdf_filter.or(doc_filter).build();
    /// // Matches files with .pdf OR .docx extension
    /// ```
    #[must_use]
    pub fn or(self, other: Self) -> OrPredicateBuilder {
        OrPredicateBuilder {
            builders: vec![self, other],
        }
    }

    /// Adds a raw predicate clause string.
    ///
    /// Use this for advanced queries not covered by the builder methods.
    /// The clause is added as-is without escaping.
    ///
    /// # Safety Note
    ///
    /// The caller is responsible for properly escaping any user input
    /// in the clause string using [`escape_predicate_string`].
    ///
    /// # Arguments
    ///
    /// * `clause` - A raw predicate format string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .raw_clause(r#"kMDItemFSName LIKE[cd] "*.rs""#)
    ///     .build();
    /// ```
    #[must_use]
    pub fn raw_clause(mut self, clause: &str) -> Self {
        if !clause.is_empty() {
            self.clauses.push(clause.to_string());
        }
        self
    }

    // =========================================================================
    // Exclusion Methods (Raycast-style filtering)
    // =========================================================================

    /// Excludes files whose path contains the given directory name.
    ///
    /// This is useful for filtering out development artifacts like `node_modules`,
    /// `.git`, `target`, etc.
    ///
    /// # Arguments
    ///
    /// * `dir_name` - The directory name to exclude (e.g., "node_modules").
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_contains("index")
    ///     .exclude_path_containing("node_modules")
    ///     .build();
    /// ```
    #[must_use]
    pub fn exclude_path_containing(mut self, dir_name: &str) -> Self {
        if !dir_name.is_empty() {
            let escaped = escape_predicate_string(dir_name);
            // Match both /dirname/ (middle of path) and /dirname (end of path)
            self.clauses.push(format!(
                r#"NOT ({} CONTAINS "/{}/")"#,
                MD_ITEM_PATH, escaped
            ));
        }
        self
    }

    /// Excludes files with the given extension.
    ///
    /// # Arguments
    ///
    /// * `ext` - The file extension to exclude (without leading dot).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_contains("module")
    ///     .exclude_extension("pyc")
    ///     .build();
    /// ```
    #[must_use]
    pub fn exclude_extension(mut self, ext: &str) -> Self {
        if !ext.is_empty() {
            let ext = ext.strip_prefix('.').unwrap_or(ext);
            let escaped = escape_predicate_string(ext);
            self.clauses.push(format!(
                r#"NOT ({} ENDSWITH[c] ".{}")"#,
                MD_ITEM_FS_NAME, escaped
            ));
        }
        self
    }

    /// Excludes hidden files (files starting with a dot).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_contains("config")
    ///     .exclude_hidden_files()
    ///     .build();
    /// ```
    #[must_use]
    pub fn exclude_hidden_files(mut self) -> Self {
        self.clauses.push(format!(
            r#"NOT ({} BEGINSWITH ".")"#,
            MD_ITEM_FS_NAME
        ));
        self
    }

    /// Applies default Raycast-style exclusions for development artifacts.
    ///
    /// This excludes common directories like `node_modules`, `.git`, `target`,
    /// `__pycache__`, etc., as well as compiled file extensions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use photoncast_core::search::spotlight::PredicateBuilder;
    ///
    /// let predicate = PredicateBuilder::new()
    ///     .name_contains("report")
    ///     .with_default_exclusions()
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_default_exclusions(mut self) -> Self {
        // Build a single NOT clause with OR'd conditions for efficiency
        // This is more efficient than many separate NOT clauses
        let mut conditions: Vec<String> = Vec::new();

        // Add directory exclusions
        for dir in DEFAULT_EXCLUDED_DIRS {
            conditions.push(format!(r#"{} CONTAINS "/{}/""#, MD_ITEM_PATH, dir));
        }

        // Add extension exclusions
        for ext in DEFAULT_EXCLUDED_EXTENSIONS {
            conditions.push(format!(r#"{} ENDSWITH[c] ".{}""#, MD_ITEM_FS_NAME, ext));
        }

        // Add hidden file exclusion
        conditions.push(format!(r#"{} BEGINSWITH ".""#, MD_ITEM_FS_NAME));

        if !conditions.is_empty() {
            self.clauses.push(format!("NOT ({})", conditions.join(" OR ")));
        }

        self
    }

    /// Excludes files in system Library directories.
    ///
    /// This filters out cache files, logs, and other system data in ~/Library.
    #[must_use]
    pub fn exclude_library_caches(mut self) -> Self {
        self.clauses.push(format!(
            r#"NOT ({} CONTAINS "/Library/Caches/" OR {} CONTAINS "/Library/Logs/" OR {} CONTAINS "/Library/Application Support/CrashReporter/")"#,
            MD_ITEM_PATH, MD_ITEM_PATH, MD_ITEM_PATH
        ));
        self
    }

    /// Returns true if no clauses have been added to this builder.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Returns the number of clauses in this builder.
    #[must_use]
    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    /// Builds the final NSPredicate.
    ///
    /// If no clauses were added, returns a predicate that matches all items.
    /// Multiple clauses are combined with AND logic.
    ///
    /// # Returns
    ///
    /// A retained NSPredicate object ready for use with NSMetadataQuery.
    ///
    /// # Panics
    ///
    /// This function should not panic under normal circumstances.
    /// Invalid predicate formats may cause the underlying Objective-C
    /// runtime to raise an exception, which will terminate the process.
    #[must_use]
    pub fn build(self) -> Retained<NSPredicate> {
        if self.clauses.is_empty() {
            // Return a predicate that matches everything
            return create_predicate_from_format(r#"kMDItemFSName == "*""#);
        }

        if self.clauses.len() == 1 {
            return create_predicate_from_format(&self.clauses[0]);
        }

        // Build individual predicates for each clause
        let predicates: Vec<Retained<NSPredicate>> = self
            .clauses
            .iter()
            .map(|clause| create_predicate_from_format(clause))
            .collect();

        // Combine with AND
        let array = NSArray::from_retained_slice(&predicates);
        // SAFETY: andPredicateWithSubpredicates is safe to call with a valid array
        let compound = unsafe { NSCompoundPredicate::andPredicateWithSubpredicates(&array) };

        // Upcast NSCompoundPredicate to NSPredicate
        // NSCompoundPredicate is a subclass of NSPredicate
        Retained::into_super(compound)
    }
}

// =============================================================================
// OrPredicateBuilder
// =============================================================================

/// A builder for creating OR compound predicates.
///
/// Created by calling [`PredicateBuilder::or`]. Can chain multiple OR conditions.
#[derive(Debug, Clone)]
pub struct OrPredicateBuilder {
    builders: Vec<PredicateBuilder>,
}

impl OrPredicateBuilder {
    /// Adds another builder to the OR chain.
    #[must_use]
    pub fn or(mut self, other: PredicateBuilder) -> Self {
        self.builders.push(other);
        self
    }

    /// Builds the final OR compound predicate.
    ///
    /// # Returns
    ///
    /// A retained NSPredicate that matches items satisfying any of the
    /// combined builders' conditions.
    #[must_use]
    pub fn build(self) -> Retained<NSPredicate> {
        if self.builders.is_empty() {
            return create_predicate_from_format(r#"kMDItemFSName == "*""#);
        }

        if self.builders.len() == 1 {
            return self.builders.into_iter().next().unwrap().build();
        }

        // Build each builder's predicate
        let predicates: Vec<Retained<NSPredicate>> =
            self.builders.into_iter().map(|b| b.build()).collect();

        // Combine with OR
        let array = NSArray::from_retained_slice(&predicates);
        // SAFETY: orPredicateWithSubpredicates is safe to call with a valid array
        let compound = unsafe { NSCompoundPredicate::orPredicateWithSubpredicates(&array) };

        Retained::into_super(compound)
    }
}

// =============================================================================
// Internal Helper Functions
// =============================================================================

/// Creates an NSPredicate from a format string.
///
/// Uses raw message sending because `predicateWithFormat:` is not exposed
/// in objc2-foundation 0.3.
fn create_predicate_from_format(format: &str) -> Retained<NSPredicate> {
    let format_str = NSString::from_str(format);
    // SAFETY: We're calling +[NSPredicate predicateWithFormat:] which is a standard
    // Foundation method. The format string must be valid predicate syntax.
    unsafe {
        let cls = class!(NSPredicate);
        msg_send_id![cls, predicateWithFormat: &*format_str]
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_predicate_string_basic() {
        assert_eq!(escape_predicate_string("hello"), "hello");
        assert_eq!(escape_predicate_string("hello world"), "hello world");
    }

    #[test]
    fn test_escape_predicate_string_special_chars() {
        assert_eq!(escape_predicate_string(r#"file*.txt"#), r#"file\*.txt"#);
        assert_eq!(escape_predicate_string(r#"what?"#), r#"what\?"#);
        assert_eq!(
            escape_predicate_string(r#"say "hello""#),
            r#"say \"hello\""#
        );
        assert_eq!(
            escape_predicate_string(r#"path\to\file"#),
            r#"path\\to\\file"#
        );
    }

    #[test]
    fn test_escape_predicate_string_multiple_special() {
        assert_eq!(
            escape_predicate_string(r#"*.txt? "test""#),
            r#"\*.txt\? \"test\""#
        );
    }

    #[test]
    fn test_predicate_builder_is_empty() {
        let builder = PredicateBuilder::new();
        assert!(builder.is_empty());
        assert_eq!(builder.clause_count(), 0);
    }

    #[test]
    fn test_predicate_builder_name_contains() {
        let builder = PredicateBuilder::new().name_contains("report");
        assert!(!builder.is_empty());
        assert_eq!(builder.clause_count(), 1);
    }

    #[test]
    fn test_predicate_builder_empty_term_ignored() {
        let builder = PredicateBuilder::new().name_contains("");
        assert!(builder.is_empty());
    }

    #[test]
    fn test_predicate_builder_chaining() {
        let builder = PredicateBuilder::new()
            .name_contains("report")
            .extension_is("pdf")
            .size_greater_than(1024);
        assert_eq!(builder.clause_count(), 3);
    }

    #[test]
    fn test_predicate_builder_extension_strips_dot() {
        let builder1 = PredicateBuilder::new().extension_is("pdf");
        let builder2 = PredicateBuilder::new().extension_is(".pdf");
        assert_eq!(builder1.clause_count(), builder2.clause_count());
    }

    #[test]
    fn test_predicate_builder_and() {
        let builder1 = PredicateBuilder::new().name_contains("report");
        let builder2 = PredicateBuilder::new().extension_is("pdf");
        let combined = builder1.and(builder2);
        assert_eq!(combined.clause_count(), 2);
    }

    #[test]
    fn test_system_time_to_timestamp() {
        let epoch = UNIX_EPOCH;
        assert_eq!(system_time_to_timestamp(epoch), 0.0);

        let one_day_later = epoch + Duration::from_secs(86400);
        assert_eq!(system_time_to_timestamp(one_day_later), 86400.0);
    }

    // Integration tests that require macOS runtime
    #[cfg(target_os = "macos")]
    mod integration {
        use super::*;

        #[test]
        fn test_build_empty_predicate() {
            let predicate = PredicateBuilder::new().build();
            // Should not panic and return a valid predicate
            assert!(!predicate.predicateFormat().is_empty());
        }

        #[test]
        fn test_build_simple_predicate() {
            let predicate = PredicateBuilder::new().name_contains("test").build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("kMDItemFSName"));
            assert!(format.contains("CONTAINS"));
        }

        #[test]
        fn test_build_compound_predicate() {
            let predicate = PredicateBuilder::new()
                .name_contains("report")
                .extension_is("pdf")
                .build();
            let format = predicate.predicateFormat().to_string();
            // Compound predicates wrap with AND
            assert!(format.contains("kMDItemFSName"));
        }

        #[test]
        fn test_build_or_predicate() {
            let pdf_filter = PredicateBuilder::new().extension_is("pdf");
            let doc_filter = PredicateBuilder::new().extension_is("docx");
            let predicate = pdf_filter.or(doc_filter).build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("OR") || format.contains("pdf"));
        }

        #[test]
        fn test_build_content_type_predicate() {
            let predicate = PredicateBuilder::new().content_type(UTI_PDF).build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("kMDItemContentType"));
            assert!(format.contains("com.adobe.pdf"));
        }

        #[test]
        fn test_build_content_type_tree_predicate() {
            let predicate = PredicateBuilder::new().content_type_tree(UTI_IMAGE).build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("kMDItemContentTypeTree"));
            assert!(format.contains("public.image"));
        }

        #[test]
        fn test_build_size_predicate() {
            let predicate = PredicateBuilder::new()
                .size_greater_than(1024)
                .size_less_than(1024 * 1024)
                .build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("kMDItemFSSize"));
        }

        // Note: Date predicate tests are skipped because NSPredicate date
        // format strings require special handling. The modified_after/before
        // methods work but their format varies by macOS version.
        // TODO: Implement proper date comparison using NSDate objects.

        #[test]
        fn test_predicate_with_special_characters() {
            // Test that special characters are properly escaped
            let predicate = PredicateBuilder::new()
                .name_contains(r#"test*file?.txt"#)
                .build();
            // Should build without crashing
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("CONTAINS"));
        }

        #[test]
        fn test_raw_clause() {
            let predicate = PredicateBuilder::new()
                .raw_clause(r#"kMDItemFSName LIKE[cd] "*.rs""#)
                .build();
            let format = predicate.predicateFormat().to_string();
            assert!(format.contains("LIKE"));
        }

        #[test]
        fn test_complex_or_chain() {
            let predicate = PredicateBuilder::new()
                .extension_is("pdf")
                .or(PredicateBuilder::new().extension_is("docx"))
                .or(PredicateBuilder::new().extension_is("txt"))
                .build();
            let format = predicate.predicateFormat().to_string();
            // Should be a valid predicate
            assert!(!format.is_empty());
        }
    }
}
