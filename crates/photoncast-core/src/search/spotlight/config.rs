//! Raycast-style search optimization configuration.
//!
//! This module provides smart defaults for file search that prioritize
//! user-relevant files and exclude technical/system artifacts.

use std::path::PathBuf;

/// Directories that should be excluded from search results.
/// These are typically development artifacts, caches, and system files.
pub const EXCLUDED_DIRECTORY_NAMES: &[&str] = &[
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
    "target", // Rust build directory
    // Python
    "__pycache__",
    ".venv",
    "venv",
    ".virtualenv",
    "virtualenv",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    "dist",
    "build",
    "*.egg-info",
    // JavaScript/TypeScript
    ".next",
    ".nuxt",
    ".output",
    ".parcel-cache",
    ".turbo",
    // IDE / Editor
    ".idea",
    ".vscode",
    ".vs",
    // macOS system
    ".Trash",
    ".Spotlight-V100",
    ".fseventsd",
    ".DS_Store",
    // Caches
    ".cache",
    "Cache",
    "Caches",
    "CachedData",
    // Logs
    "logs",
    "*.log",
];

/// Directories within ~/Library that should be excluded.
/// These contain app data, caches, and system files users rarely search for.
pub const EXCLUDED_LIBRARY_SUBDIRS: &[&str] = &[
    "Caches",
    "Logs",
    "Application Support/CrashReporter",
    "Application Support/Slack/Cache",
    "Application Support/Google/Chrome/Default/Cache",
    "Application Support/Firefox/Profiles",
    "Application Support/Code/Cache",
    "Application Support/Code/CachedData",
    "Developer/Xcode/DerivedData",
    "Developer/Xcode/iOS DeviceSupport",
    "Developer/CoreSimulator",
    "Containers",
    "Group Containers",
    "Saved Application State",
    "WebKit",
];

/// File extensions that are typically not user-relevant.
pub const EXCLUDED_EXTENSIONS: &[&str] = &[
    // Compiled/binary
    "o",
    "obj",
    "pyc",
    "pyo",
    "class",
    "dll",
    "dylib",
    "so",
    // Lock files
    "lock",
    "lockb",
    // Temporary
    "tmp",
    "temp",
    "swp",
    "swo",
    // Logs
    "log",
    // Database
    "sqlite-shm",
    "sqlite-wal",
    // Source maps
    "map",
];

/// Content types (UTIs) that should be excluded from search.
pub const EXCLUDED_CONTENT_TYPES: &[&str] = &[
    "com.apple.log",
    "public.log",
    "com.apple.crashreport",
    "dyn.ah62d4rv4ge80e5pe", // .DS_Store
];

/// Returns the primary search scopes - directories users most commonly search.
/// These are searched first and with higher priority.
#[must_use]
pub fn primary_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // User's most accessed directories
        scopes.push(home.join("Desktop"));
        scopes.push(home.join("Documents"));
        scopes.push(home.join("Downloads"));
    }

    // Applications
    scopes.push(PathBuf::from("/Applications"));

    scopes
}

/// Returns secondary search scopes - directories that may contain
/// relevant files but are searched with lower priority.
#[must_use]
pub fn secondary_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();

    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Pictures"));
        scopes.push(home.join("Music"));
        scopes.push(home.join("Movies"));
        scopes.push(home.join("Public"));
    }

    scopes
}

/// Returns all user-relevant search scopes (primary + secondary).
#[must_use]
pub fn all_user_scopes() -> Vec<PathBuf> {
    let mut scopes = primary_search_scopes();
    scopes.extend(secondary_search_scopes());
    scopes
}

/// Returns paths that should be completely excluded from search.
/// Files in these paths will never appear in results.
#[must_use]
pub fn excluded_paths() -> Vec<PathBuf> {
    let mut excluded = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // Library subdirectories with caches and system files
        let library = home.join("Library");
        for subdir in EXCLUDED_LIBRARY_SUBDIRS {
            excluded.push(library.join(subdir));
        }

        // Common development directories in home
        excluded.push(home.join(".cargo"));
        excluded.push(home.join(".rustup"));
        excluded.push(home.join(".npm"));
        excluded.push(home.join(".yarn"));
        excluded.push(home.join(".pnpm"));
        excluded.push(home.join(".local"));
        excluded.push(home.join(".cache"));
        excluded.push(home.join("go"));
    }

    // System directories
    excluded.push(PathBuf::from("/System"));
    excluded.push(PathBuf::from("/private"));
    excluded.push(PathBuf::from("/var"));
    excluded.push(PathBuf::from("/usr"));
    excluded.push(PathBuf::from("/opt"));
    excluded.push(PathBuf::from("/cores"));

    excluded
}

/// Builds an MDQuery exclusion clause for directory names.
/// Returns a predicate string fragment like:
/// `NOT (kMDItemPath CONTAINS "/node_modules/" OR kMDItemPath CONTAINS "/.git/")`
#[must_use]
pub fn build_path_exclusion_clause() -> String {
    let conditions: Vec<String> = EXCLUDED_DIRECTORY_NAMES
        .iter()
        .filter(|name| !name.contains('*')) // Skip wildcards for now
        .map(|name| format!("kMDItemPath CONTAINS \"/{}\"", name))
        .collect();

    if conditions.is_empty() {
        String::new()
    } else {
        format!("NOT ({})", conditions.join(" OR "))
    }
}

/// Builds an MDQuery exclusion clause for hidden files.
/// Hidden files start with a dot.
#[must_use]
pub fn build_hidden_file_exclusion() -> String {
    "kMDItemFSName != '.*'".to_string()
}

/// Checks if a path should be excluded based on configuration.
#[must_use]
pub fn should_exclude_path(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();

    // Check for excluded directory names in path
    for excluded_dir in EXCLUDED_DIRECTORY_NAMES {
        if !excluded_dir.contains('*') {
            let pattern = format!("/{}/", excluded_dir);
            if path_str.contains(&pattern) {
                return true;
            }
            // Also check if the file is directly in an excluded dir
            if path_str.ends_with(&format!("/{}", excluded_dir)) {
                return true;
            }
        }
    }

    // Check for excluded extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if EXCLUDED_EXTENSIONS.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // Check for hidden files (starting with .)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') && name != ".." {
            return true;
        }
    }

    false
}

/// Filter configuration for search results.
#[derive(Debug, Clone)]
pub struct SearchFilter {
    /// Exclude hidden files (starting with .)
    pub exclude_hidden: bool,
    /// Exclude files in development directories (node_modules, .git, etc.)
    pub exclude_dev_artifacts: bool,
    /// Exclude cache and temporary files
    pub exclude_caches: bool,
    /// Only include files modified within this many days (0 = no limit)
    pub max_age_days: u32,
    /// Minimum file size in bytes (0 = no limit)
    pub min_size_bytes: u64,
    /// Maximum file size in bytes (0 = no limit)
    pub max_size_bytes: u64,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            exclude_hidden: true,
            exclude_dev_artifacts: true,
            exclude_caches: true,
            max_age_days: 0,
            min_size_bytes: 0,
            max_size_bytes: 0,
        }
    }
}

impl SearchFilter {
    /// Creates a filter optimized for general file search (Raycast-style).
    #[must_use]
    pub fn raycast_style() -> Self {
        Self {
            exclude_hidden: true,
            exclude_dev_artifacts: true,
            exclude_caches: true,
            max_age_days: 0,
            min_size_bytes: 0,
            max_size_bytes: 0,
        }
    }

    /// Creates a filter that includes everything (for advanced users).
    #[must_use]
    pub fn include_all() -> Self {
        Self {
            exclude_hidden: false,
            exclude_dev_artifacts: false,
            exclude_caches: false,
            max_age_days: 0,
            min_size_bytes: 0,
            max_size_bytes: 0,
        }
    }

    /// Creates a filter for recent files only.
    #[must_use]
    pub fn recent_only(days: u32) -> Self {
        Self {
            max_age_days: days,
            ..Self::raycast_style()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_exclude_node_modules() {
        assert!(should_exclude_path(std::path::Path::new(
            "/Users/test/project/node_modules/lodash/index.js"
        )));
    }

    #[test]
    fn test_should_exclude_git() {
        assert!(should_exclude_path(std::path::Path::new(
            "/Users/test/project/.git/objects/pack"
        )));
    }

    #[test]
    fn test_should_exclude_hidden() {
        assert!(should_exclude_path(std::path::Path::new(
            "/Users/test/.bashrc"
        )));
    }

    #[test]
    fn test_should_not_exclude_normal_file() {
        assert!(!should_exclude_path(std::path::Path::new(
            "/Users/test/Documents/report.pdf"
        )));
    }

    #[test]
    fn test_should_exclude_pyc() {
        assert!(should_exclude_path(std::path::Path::new(
            "/Users/test/project/module.pyc"
        )));
    }

    #[test]
    fn test_primary_scopes_not_empty() {
        let scopes = primary_search_scopes();
        assert!(!scopes.is_empty());
    }

    #[test]
    fn test_exclusion_clause() {
        let clause = build_path_exclusion_clause();
        assert!(clause.contains("node_modules"));
        assert!(clause.contains(".git"));
        assert!(clause.contains("NOT"));
    }
}
