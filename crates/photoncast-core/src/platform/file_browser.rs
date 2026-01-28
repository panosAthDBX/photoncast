//! File browser for browsing mode in file search.
//!
//! This module provides functionality for navigating the file system
//! directly, triggered by path prefixes like `/`, `~`, `~/`, etc.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::spotlight::FileKind;

/// A directory entry with metadata for display in browsing mode.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Full path to the entry.
    pub path: PathBuf,
    /// Display name (file/folder name).
    pub name: String,
    /// Kind of file (folder, document, image, etc.).
    pub kind: FileKind,
    /// File size in bytes (None for folders).
    pub size: Option<u64>,
    /// Last modified time.
    pub modified: Option<SystemTime>,
    /// Number of items in folder (only for folders).
    pub item_count: Option<usize>,
}

impl DirectoryEntry {
    /// Creates a new directory entry from a path and metadata.
    ///
    /// # Arguments
    ///
    /// * `path` - The full path to the entry
    /// * `metadata` - File system metadata for the entry
    #[must_use]
    pub fn from_path_and_metadata(path: PathBuf, metadata: &std::fs::Metadata) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let kind = FileKind::from_path(&path);
        let is_dir = metadata.is_dir();

        Self {
            path,
            name,
            kind,
            size: if is_dir { None } else { Some(metadata.len()) },
            modified: metadata.modified().ok(),
            item_count: None, // Loaded lazily if needed
        }
    }

    /// Loads the item count for directories.
    ///
    /// This is done lazily to avoid blocking on directory enumeration.
    pub fn load_item_count(&mut self) {
        if self.kind == FileKind::Folder {
            if let Ok(entries) = std::fs::read_dir(&self.path) {
                self.item_count = Some(entries.count());
            }
        }
    }

    /// Returns true if this entry is a directory.
    #[must_use]
    pub fn is_directory(&self) -> bool {
        self.kind == FileKind::Folder || self.kind == FileKind::Application
    }

    /// Returns true if this entry is a symlink.
    #[must_use]
    pub fn is_symlink(&self) -> bool {
        self.path.is_symlink()
    }
}

/// File browser for navigating the file system in browsing mode.
#[derive(Debug, Clone)]
pub struct FileBrowser {
    /// Current browsing path.
    current_path: PathBuf,
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl FileBrowser {
    /// Creates a new file browser starting at the home directory.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_path: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    /// Creates a new file browser starting at the specified path.
    #[must_use]
    pub fn with_path(path: PathBuf) -> Self {
        Self { current_path: path }
    }

    /// Returns the current browsing path.
    #[must_use]
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    /// Checks if a query string triggers browsing mode.
    ///
    /// Browsing mode is triggered by:
    /// - `/` - root directory
    /// - `~` or `~/` - home directory
    /// - `/Users/...` or other absolute paths
    /// - `$HOME/...` or `${HOME}/...` - environment variable paths
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    #[must_use]
    pub fn is_browsing_mode(query: &str) -> bool {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Check for path prefixes
        trimmed.starts_with('/') || trimmed.starts_with('~') || trimmed.starts_with('$')
    }

    /// Parses a query string into a path for browsing mode.
    ///
    /// Returns `None` if the query doesn't represent a valid browsing path.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    #[must_use]
    pub fn parse_path(query: &str) -> Option<PathBuf> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Expand environment variables first
        let expanded = Self::expand_env_vars(trimmed);

        // Handle tilde expansion
        let path_str = if expanded.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                if expanded == "~" {
                    home.to_string_lossy().to_string()
                } else if let Some(rest) = expanded.strip_prefix("~/") {
                    format!("{}/{}", home.to_string_lossy(), rest)
                } else {
                    // Handle ~username (not supported, return as-is)
                    expanded
                }
            } else {
                expanded
            }
        } else {
            expanded
        };

        let path = PathBuf::from(&path_str);

        // Only return if it starts with a valid path character
        if path_str.starts_with('/') || path_str.starts_with('~') {
            Some(path)
        } else {
            None
        }
    }

    /// Expands environment variables in a path string.
    ///
    /// Supports both `$VAR` and `${VAR}` syntax.
    ///
    /// # Arguments
    ///
    /// * `path` - The path string potentially containing environment variables
    #[must_use]
    #[allow(clippy::format_push_string)]
    pub fn expand_env_vars(path: &str) -> String {
        let mut result = String::with_capacity(path.len());
        let mut chars = path.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                // Check for ${VAR} or $VAR syntax
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        var_name.push(chars.next().unwrap());
                    }
                    if let Ok(value) = std::env::var(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Keep original if not found
                        result.push_str(&format!("${{{var_name}}}"));
                    }
                } else {
                    // $VAR syntax - collect alphanumeric and underscore using peek
                    let mut var_name = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_alphanumeric() || next_c == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if var_name.is_empty() {
                        result.push('$');
                    } else if let Ok(value) = std::env::var(&var_name) {
                        result.push_str(&value);
                    } else {
                        // Keep original if not found
                        result.push('$');
                        result.push_str(&var_name);
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Lists the contents of a directory.
    ///
    /// Returns entries sorted with folders first, then files, both alphabetically.
    /// Handles permission errors gracefully by returning an empty vector for
    /// inaccessible directories.
    ///
    /// # Arguments
    ///
    /// * `path` - The directory path to list
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or is not a directory.
    pub fn list_directory(path: &Path) -> Result<Vec<DirectoryEntry>, io::Error> {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Path does not exist: {}", path.display()),
            ));
        }

        if !path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Path is not a directory: {}", path.display()),
            ));
        }

        let read_dir = match std::fs::read_dir(path) {
            Ok(rd) => rd,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                // Return empty for permission denied - don't propagate error
                return Ok(Vec::new());
            },
            Err(e) => return Err(e),
        };

        let mut entries: Vec<DirectoryEntry> = read_dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;
                Some(DirectoryEntry::from_path_and_metadata(
                    entry.path(),
                    &metadata,
                ))
            })
            .collect();

        // Sort: folders first, then files, alphabetically within each group
        entries.sort_by(|a, b| {
            let a_is_dir = a.is_directory();
            let b_is_dir = b.is_directory();

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }

    /// Navigates to a new path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to navigate to
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path;
    }

    /// Navigates to the parent directory.
    ///
    /// Returns `false` if already at root (no parent to navigate to).
    pub fn go_up(&mut self) -> bool {
        if let Some(parent) = self.current_path.parent() {
            if parent.as_os_str().is_empty() {
                // Already at root
                return false;
            }
            self.current_path = parent.to_path_buf();
            true
        } else {
            false
        }
    }

    /// Enters a folder, updating the current path.
    ///
    /// Returns `true` if the entry is a folder and navigation succeeded.
    ///
    /// # Arguments
    ///
    /// * `entry` - The directory entry to enter
    #[allow(clippy::assigning_clones)]
    pub fn enter_folder(&mut self, entry: &DirectoryEntry) -> bool {
        if entry.is_directory() {
            self.current_path = entry.path.clone();
            true
        } else {
            false
        }
    }

    /// Expands a file entry to its full path string.
    ///
    /// # Arguments
    ///
    /// * `entry` - The directory entry to expand
    #[must_use]
    pub fn expand_path(entry: &DirectoryEntry) -> String {
        entry.path.to_string_lossy().to_string()
    }

    /// Resolves a symlink entry to its target path.
    ///
    /// Returns the original path if not a symlink or if resolution fails.
    ///
    /// # Arguments
    ///
    /// * `entry` - The directory entry (potentially a symlink)
    #[must_use]
    #[allow(clippy::map_unwrap_or)]
    pub fn resolve_symlink(entry: &DirectoryEntry) -> PathBuf {
        if entry.is_symlink() {
            std::fs::read_link(&entry.path)
                .map(|target| {
                    // If relative, resolve against parent
                    if target.is_relative() {
                        if let Some(parent) = entry.path.parent() {
                            parent.join(&target).canonicalize().unwrap_or(target)
                        } else {
                            target
                        }
                    } else {
                        target
                    }
                })
                .unwrap_or_else(|_| entry.path.clone())
        } else {
            entry.path.clone()
        }
    }

    /// Filters entries by a search term.
    ///
    /// Performs case-insensitive fuzzy matching on entry names.
    ///
    /// # Arguments
    ///
    /// * `entries` - The entries to filter
    /// * `filter` - The filter string
    #[must_use]
    pub fn filter_entries<'a>(
        entries: &'a [DirectoryEntry],
        filter: &str,
    ) -> Vec<&'a DirectoryEntry> {
        if filter.is_empty() {
            return entries.iter().collect();
        }

        let filter_lower = filter.to_lowercase();

        entries
            .iter()
            .filter(|entry| {
                let name_lower = entry.name.to_lowercase();
                // Simple substring match - could be enhanced with fuzzy matching
                name_lower.contains(&filter_lower)
            })
            .collect()
    }

    /// Extracts the filter portion from a browsing query.
    ///
    /// Given a path like `~/Documents/foo`, extracts `foo` as the filter
    /// if `~/Documents/foo` doesn't exist but `~/Documents` does.
    ///
    /// # Arguments
    ///
    /// * `query` - The full query string
    #[must_use]
    pub fn extract_filter(query: &str) -> Option<(PathBuf, String)> {
        let path = Self::parse_path(query)?;

        // If path exists and is a directory, no filter
        if path.is_dir() {
            return Some((path, String::new()));
        }

        // If path doesn't exist, check if parent exists
        if let Some(parent) = path.parent() {
            if parent.is_dir() {
                let filter = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                return Some((parent.to_path_buf(), filter));
            }
        }

        // Can't extract valid path and filter
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // FileBrowser::is_browsing_mode Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_browsing_mode_root() {
        assert!(FileBrowser::is_browsing_mode("/"));
        assert!(FileBrowser::is_browsing_mode("/Users"));
        assert!(FileBrowser::is_browsing_mode("/Applications/Safari.app"));
    }

    #[test]
    fn test_is_browsing_mode_home() {
        assert!(FileBrowser::is_browsing_mode("~"));
        assert!(FileBrowser::is_browsing_mode("~/"));
        assert!(FileBrowser::is_browsing_mode("~/Documents"));
    }

    #[test]
    fn test_is_browsing_mode_env_var() {
        assert!(FileBrowser::is_browsing_mode("$HOME"));
        assert!(FileBrowser::is_browsing_mode("$HOME/Documents"));
        assert!(FileBrowser::is_browsing_mode("${HOME}/Documents"));
    }

    #[test]
    fn test_is_browsing_mode_regular_query() {
        assert!(!FileBrowser::is_browsing_mode("firefox"));
        assert!(!FileBrowser::is_browsing_mode("my document"));
        assert!(!FileBrowser::is_browsing_mode(""));
        assert!(!FileBrowser::is_browsing_mode("   "));
    }

    // -------------------------------------------------------------------------
    // FileBrowser::expand_env_vars Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_expand_env_vars_home() {
        // Set a test environment variable
        std::env::set_var("TEST_VAR", "/test/path");

        let result = FileBrowser::expand_env_vars("$TEST_VAR/subdir");
        assert_eq!(result, "/test/path/subdir");

        let result = FileBrowser::expand_env_vars("${TEST_VAR}/subdir");
        assert_eq!(result, "/test/path/subdir");

        std::env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_expand_env_vars_multiple() {
        std::env::set_var("VAR1", "first");
        std::env::set_var("VAR2", "second");

        let result = FileBrowser::expand_env_vars("$VAR1/$VAR2/end");
        assert_eq!(result, "first/second/end");

        std::env::remove_var("VAR1");
        std::env::remove_var("VAR2");
    }

    #[test]
    fn test_expand_env_vars_undefined() {
        let result = FileBrowser::expand_env_vars("$UNDEFINED_VAR_12345/path");
        assert_eq!(result, "$UNDEFINED_VAR_12345/path");

        let result = FileBrowser::expand_env_vars("${UNDEFINED_VAR_12345}/path");
        assert_eq!(result, "${UNDEFINED_VAR_12345}/path");
    }

    #[test]
    fn test_expand_env_vars_no_vars() {
        let result = FileBrowser::expand_env_vars("/regular/path");
        assert_eq!(result, "/regular/path");
    }

    #[test]
    fn test_expand_env_vars_dollar_sign_only() {
        let result = FileBrowser::expand_env_vars("price is $");
        assert_eq!(result, "price is $");
    }

    // -------------------------------------------------------------------------
    // FileBrowser::parse_path Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_path_root() {
        let path = FileBrowser::parse_path("/");
        assert_eq!(path, Some(PathBuf::from("/")));

        let path = FileBrowser::parse_path("/Users");
        assert_eq!(path, Some(PathBuf::from("/Users")));
    }

    #[test]
    fn test_parse_path_home() {
        let path = FileBrowser::parse_path("~");
        assert!(path.is_some());
        // Should resolve to actual home directory
        if let Some(home) = dirs::home_dir() {
            assert_eq!(path.unwrap(), home);
        }
    }

    #[test]
    fn test_parse_path_home_subdir() {
        let path = FileBrowser::parse_path("~/Documents");
        assert!(path.is_some());
        if let Some(home) = dirs::home_dir() {
            assert_eq!(path.unwrap(), home.join("Documents"));
        }
    }

    #[test]
    fn test_parse_path_invalid() {
        let path = FileBrowser::parse_path("regular query");
        assert!(path.is_none());

        let path = FileBrowser::parse_path("");
        assert!(path.is_none());
    }

    // -------------------------------------------------------------------------
    // FileBrowser::list_directory Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_list_directory_root() {
        // Root should always exist and be listable
        let result = FileBrowser::list_directory(Path::new("/"));
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_list_directory_home() {
        if let Some(home) = dirs::home_dir() {
            let result = FileBrowser::list_directory(&home);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_list_directory_nonexistent() {
        let result = FileBrowser::list_directory(Path::new("/nonexistent/path/12345"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_list_directory_not_a_directory() {
        // Try to list a file as a directory
        let result = FileBrowser::list_directory(Path::new("/etc/hosts"));
        if let Err(e) = result {
            assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
        }
        // Note: /etc/hosts might not exist on all systems
    }

    #[test]
    fn test_list_directory_sorting() {
        // Create a temp directory with known contents
        let temp_dir = std::env::temp_dir().join("photoncast_test_sorting");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create files and folders
        std::fs::create_dir(temp_dir.join("alpha_folder")).unwrap();
        std::fs::create_dir(temp_dir.join("beta_folder")).unwrap();
        std::fs::write(temp_dir.join("alpha_file.txt"), "").unwrap();
        std::fs::write(temp_dir.join("beta_file.txt"), "").unwrap();
        std::fs::write(temp_dir.join("Aardvark.txt"), "").unwrap();

        let result = FileBrowser::list_directory(&temp_dir).unwrap();

        // Folders should come first, then files, both alphabetically
        let names: Vec<&str> = result.iter().map(|e| e.name.as_str()).collect();

        // Find where folders end and files begin
        let folder_end = names
            .iter()
            .position(|n| {
                !result
                    .iter()
                    .find(|e| e.name == *n)
                    .unwrap()
                    .is_directory()
            })
            .unwrap_or(names.len());

        // Verify folders come first
        for (i, entry) in result.iter().enumerate() {
            if i < folder_end {
                assert!(entry.is_directory(), "Expected folder at position {i}");
            } else {
                assert!(!entry.is_directory(), "Expected file at position {i}");
            }
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // -------------------------------------------------------------------------
    // FileBrowser Navigation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_file_browser_new() {
        let browser = FileBrowser::new();
        if let Some(home) = dirs::home_dir() {
            assert_eq!(browser.current_path(), home.as_path());
        }
    }

    #[test]
    fn test_file_browser_navigate_to() {
        let mut browser = FileBrowser::new();
        browser.navigate_to(PathBuf::from("/tmp"));
        assert_eq!(browser.current_path(), Path::new("/tmp"));
    }

    #[test]
    fn test_file_browser_go_up() {
        let mut browser = FileBrowser::with_path(PathBuf::from("/Users/test/Documents"));
        assert!(browser.go_up());
        assert_eq!(browser.current_path(), Path::new("/Users/test"));

        assert!(browser.go_up());
        assert_eq!(browser.current_path(), Path::new("/Users"));

        assert!(browser.go_up());
        assert_eq!(browser.current_path(), Path::new("/"));

        // At root, should return false
        assert!(!browser.go_up());
    }

    // -------------------------------------------------------------------------
    // FileBrowser::filter_entries Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_filter_entries_empty_filter() {
        let entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("file2.txt", false),
        ];

        let filtered = FileBrowser::filter_entries(&entries, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_entries_substring_match() {
        let entries = vec![
            create_test_entry("document.pdf", false),
            create_test_entry("photo.jpg", false),
            create_test_entry("my_document.txt", false),
        ];

        let filtered = FileBrowser::filter_entries(&entries, "doc");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|e| e.name == "document.pdf"));
        assert!(filtered.iter().any(|e| e.name == "my_document.txt"));
    }

    #[test]
    fn test_filter_entries_case_insensitive() {
        let entries = vec![
            create_test_entry("README.md", false),
            create_test_entry("readme.txt", false),
            create_test_entry("other.txt", false),
        ];

        let filtered = FileBrowser::filter_entries(&entries, "readme");
        assert_eq!(filtered.len(), 2);
    }

    // -------------------------------------------------------------------------
    // FileBrowser::extract_filter Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_filter_existing_directory() {
        // /tmp should exist on most systems
        let result = FileBrowser::extract_filter("/tmp");
        assert!(result.is_some());
        let (path, filter) = result.unwrap();
        assert_eq!(path, PathBuf::from("/tmp"));
        assert!(filter.is_empty());
    }

    #[test]
    fn test_extract_filter_with_filter() {
        // Assuming /tmp exists but /tmp/nonexistent12345 doesn't
        let result = FileBrowser::extract_filter("/tmp/nonexistent12345");
        if let Some((path, filter)) = result {
            assert_eq!(path, PathBuf::from("/tmp"));
            assert_eq!(filter, "nonexistent12345");
        }
    }

    // -------------------------------------------------------------------------
    // DirectoryEntry Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_directory_entry_is_symlink() {
        // Create a temp symlink for testing
        let temp_dir = std::env::temp_dir().join("photoncast_test_symlink");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let target = temp_dir.join("target.txt");
        std::fs::write(&target, "content").unwrap();

        let link = temp_dir.join("link.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        #[cfg(unix)]
        {
            let metadata = std::fs::metadata(&link).unwrap();
            let entry = DirectoryEntry::from_path_and_metadata(link.clone(), &metadata);
            // Note: is_symlink checks the path directly, not metadata
            assert!(entry.is_symlink());
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_symlink() {
        let temp_dir = std::env::temp_dir().join("photoncast_test_resolve_symlink");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let target = temp_dir.join("target.txt");
        std::fs::write(&target, "content").unwrap();

        let link = temp_dir.join("link.txt");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).unwrap();

            let metadata = std::fs::metadata(&link).unwrap();
            let entry = DirectoryEntry::from_path_and_metadata(link, &metadata);
            let resolved = FileBrowser::resolve_symlink(&entry);
            assert_eq!(resolved.file_name().unwrap(), "target.txt");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // -------------------------------------------------------------------------
    // Helper Functions
    // -------------------------------------------------------------------------

    fn create_test_entry(name: &str, is_dir: bool) -> DirectoryEntry {
        DirectoryEntry {
            path: PathBuf::from(format!("/test/{name}")),
            name: name.to_string(),
            kind: if is_dir {
                FileKind::Folder
            } else {
                FileKind::File
            },
            size: if is_dir { None } else { Some(100) },
            modified: None,
            item_count: None,
        }
    }
}
