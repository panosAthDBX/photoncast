//! Spotlight result extraction module.
//!
//! This module extracts rich metadata from `NSMetadataItem` objects returned by
//! Spotlight queries and converts them into Rust types.
//!
//! # Architecture
//!
//! - [`SpotlightResult`] - Rust representation of a Spotlight search result
//! - [`MetadataExtractor`] - Extracts attributes from `NSMetadataItem` objects
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::spotlight::result::{MetadataExtractor, SpotlightResult};
//!
//! // After running a Spotlight query...
//! // let query_results: &NSArray<NSMetadataItem> = query.results();
//! // let results: Vec<SpotlightResult> = MetadataExtractor::extract_batch(query_results);
//!
//! // for result in results {
//! //     println!("{}: {} bytes", result.path.display(), result.file_size.unwrap_or(0));
//! // }
//! ```

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSDate, NSMetadataItem, NSNumber, NSString, NSURL};

// =============================================================================
// Metadata Attribute Keys
// =============================================================================

/// Spotlight metadata attribute keys.
///
/// These constants correspond to the `kMDItem*` keys used by Spotlight
/// to store file metadata.
#[cfg(target_os = "macos")]
pub mod keys {
    use objc2_foundation::{ns_string, NSString};

    /// Path to the file (`kMDItemPath`).
    #[inline]
    pub fn path() -> &'static NSString {
        ns_string!("kMDItemPath")
    }

    /// Display name of the file (`kMDItemDisplayName`).
    #[inline]
    pub fn display_name() -> &'static NSString {
        ns_string!("kMDItemDisplayName")
    }

    /// File system name (`kMDItemFSName`).
    #[inline]
    pub fn fs_name() -> &'static NSString {
        ns_string!("kMDItemFSName")
    }

    /// File size in bytes (`kMDItemFSSize`).
    #[inline]
    pub fn fs_size() -> &'static NSString {
        ns_string!("kMDItemFSSize")
    }

    /// Content type UTI (`kMDItemContentType`).
    #[inline]
    pub fn content_type() -> &'static NSString {
        ns_string!("kMDItemContentType")
    }

    /// Content type tree (array of UTIs) (`kMDItemContentTypeTree`).
    #[inline]
    pub fn content_type_tree() -> &'static NSString {
        ns_string!("kMDItemContentTypeTree")
    }

    /// File modification date (`kMDItemFSContentChangeDate`).
    #[inline]
    pub fn fs_content_change_date() -> &'static NSString {
        ns_string!("kMDItemFSContentChangeDate")
    }

    /// File creation date (`kMDItemFSCreationDate`).
    #[inline]
    pub fn fs_creation_date() -> &'static NSString {
        ns_string!("kMDItemFSCreationDate")
    }

    /// Last used date (`kMDItemLastUsedDate`).
    #[inline]
    pub fn last_used_date() -> &'static NSString {
        ns_string!("kMDItemLastUsedDate")
    }

    /// Whether the item is a directory (`kMDItemContentTypeTree` contains `public.folder`).
    /// Note: We check the content type tree for this.
    #[inline]
    pub fn public_folder_uti() -> &'static str {
        "public.folder"
    }
}

// =============================================================================
// SpotlightResult
// =============================================================================

/// A Spotlight search result with extracted metadata.
///
/// This struct represents a file or directory found by a Spotlight query,
/// with rich metadata extracted from the `NSMetadataItem`.
#[derive(Debug, Clone)]
pub struct SpotlightResult {
    /// Full path to the file or directory.
    pub path: PathBuf,

    /// Display name of the file (may differ from filename).
    pub display_name: String,

    /// File size in bytes (None for directories or if unavailable).
    pub file_size: Option<u64>,

    /// Content type UTI (e.g., "public.jpeg", "com.apple.application").
    pub content_type: Option<String>,

    /// Content type inheritance tree (e.g., ["public.jpeg", "public.image", "public.data"]).
    pub content_type_tree: Vec<String>,

    /// Last modification date.
    pub modified_date: Option<SystemTime>,

    /// Creation date.
    pub created_date: Option<SystemTime>,

    /// Last used/accessed date.
    pub last_used_date: Option<SystemTime>,

    /// Whether this is a directory.
    pub is_directory: bool,
}

impl SpotlightResult {
    /// Returns the file extension, if any.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Returns the file name without the path.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|n| n.to_str())
    }

    /// Checks if this result matches a specific content type UTI.
    ///
    /// This performs a hierarchical check against the content type tree.
    #[must_use]
    pub fn conforms_to_type(&self, uti: &str) -> bool {
        self.content_type_tree
            .iter()
            .any(|t| t.eq_ignore_ascii_case(uti))
    }

    /// Checks if this is an application bundle.
    #[must_use]
    pub fn is_application(&self) -> bool {
        self.conforms_to_type("com.apple.application")
            || self.conforms_to_type("com.apple.application-bundle")
    }

    /// Checks if this is an image file.
    #[must_use]
    pub fn is_image(&self) -> bool {
        self.conforms_to_type("public.image")
    }

    /// Checks if this is a document.
    #[must_use]
    pub fn is_document(&self) -> bool {
        self.conforms_to_type("public.composite-content") || self.conforms_to_type("public.content")
    }
}

// =============================================================================
// MetadataExtractor
// =============================================================================

/// Extracts metadata from `NSMetadataItem` objects.
///
/// This struct provides methods to convert Spotlight query results
/// into Rust-native [`SpotlightResult`] objects.
#[cfg(target_os = "macos")]
pub struct MetadataExtractor;

#[cfg(target_os = "macos")]
impl MetadataExtractor {
    /// Extracts a [`SpotlightResult`] from an `NSMetadataItem`.
    ///
    /// Returns `None` if the essential path attribute is missing.
    ///
    /// # Safety
    ///
    /// This function uses unsafe Objective-C interop but is safe to call
    /// as long as the `item` is a valid `NSMetadataItem`.
    #[must_use]
    pub fn extract(item: &NSMetadataItem) -> Option<SpotlightResult> {
        // Path is required - without it we can't return a valid result
        let path = Self::get_path_attribute(item)?;

        // Get display name, falling back to file name from path
        let display_name = Self::get_string_attribute(item, keys::display_name())
            .or_else(|| Self::get_string_attribute(item, keys::fs_name()))
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            });

        // Get file size
        let file_size = Self::get_number_attribute(item, keys::fs_size()).map(|n| n as u64);

        // Get content type
        let content_type = Self::get_string_attribute(item, keys::content_type());

        // Get content type tree
        let content_type_tree = Self::get_string_array_attribute(item, keys::content_type_tree());

        // Determine if directory by checking content type tree
        let is_directory = content_type_tree
            .iter()
            .any(|t| t == keys::public_folder_uti());

        // Get dates
        let modified_date = Self::get_date_attribute(item, keys::fs_content_change_date());
        let created_date = Self::get_date_attribute(item, keys::fs_creation_date());
        let last_used_date = Self::get_date_attribute(item, keys::last_used_date());

        Some(SpotlightResult {
            path,
            display_name,
            file_size,
            content_type,
            content_type_tree,
            modified_date,
            created_date,
            last_used_date,
            is_directory,
        })
    }

    /// Extracts multiple [`SpotlightResult`]s from an `NSArray` of `NSMetadataItem`s.
    ///
    /// Items that cannot be extracted (e.g., missing path) are skipped.
    #[must_use]
    pub fn extract_batch(items: &NSArray<NSMetadataItem>) -> Vec<SpotlightResult> {
        let count = items.count();
        let mut results = Vec::with_capacity(count);

        for i in 0..count {
            // SAFETY: Index is within bounds (0..count)
            let item = unsafe { items.objectAtIndex(i) };
            if let Some(result) = Self::extract(&item) {
                results.push(result);
            }
        }

        results
    }

    // =========================================================================
    // Attribute Extraction Helpers
    // =========================================================================

    /// Extracts a string attribute from an `NSMetadataItem`.
    ///
    /// Returns `None` if the attribute is missing or not a string.
    #[must_use]
    pub fn get_string_attribute(item: &NSMetadataItem, key: &NSString) -> Option<String> {
        // SAFETY: valueForAttribute is safe to call with any key
        let value: Option<Retained<AnyObject>> = unsafe { item.valueForAttribute(key) };

        value.and_then(|obj| {
            // Try to interpret as NSString
            // SAFETY: We're checking if the object responds to NSString methods
            let ns_string: Option<&NSString> = unsafe { obj.downcast_ref() };
            ns_string.map(|s| s.to_string())
        })
    }

    /// Extracts a numeric attribute from an `NSMetadataItem`.
    ///
    /// Returns `None` if the attribute is missing or not a number.
    #[must_use]
    pub fn get_number_attribute(item: &NSMetadataItem, key: &NSString) -> Option<i64> {
        // SAFETY: valueForAttribute is safe to call with any key
        let value: Option<Retained<AnyObject>> = unsafe { item.valueForAttribute(key) };

        value.and_then(|obj| {
            // Try to interpret as NSNumber
            // SAFETY: We're checking if the object responds to NSNumber methods
            let ns_number: Option<&NSNumber> = unsafe { obj.downcast_ref() };
            ns_number.map(|n| n.as_i64())
        })
    }

    /// Extracts a date attribute from an `NSMetadataItem` as `SystemTime`.
    ///
    /// Returns `None` if the attribute is missing or not a date.
    #[must_use]
    pub fn get_date_attribute(item: &NSMetadataItem, key: &NSString) -> Option<SystemTime> {
        // SAFETY: valueForAttribute is safe to call with any key
        let value: Option<Retained<AnyObject>> = unsafe { item.valueForAttribute(key) };

        value.and_then(|obj| {
            // Try to interpret as NSDate
            // SAFETY: We're checking if the object responds to NSDate methods
            let ns_date: Option<&NSDate> = unsafe { obj.downcast_ref() };
            ns_date.and_then(|date| {
                // SAFETY: timeIntervalSince1970 is always safe to call on NSDate
                let timestamp = unsafe { date.timeIntervalSince1970() };
                nsdate_to_system_time(timestamp)
            })
        })
    }

    /// Extracts a URL attribute from an `NSMetadataItem` as a `PathBuf`.
    ///
    /// Returns `None` if the attribute is missing or not a URL.
    #[must_use]
    pub fn get_url_attribute(item: &NSMetadataItem, key: &NSString) -> Option<PathBuf> {
        // SAFETY: valueForAttribute is safe to call with any key
        let value: Option<Retained<AnyObject>> = unsafe { item.valueForAttribute(key) };

        value.and_then(|obj| {
            // Try to interpret as NSURL
            // SAFETY: We're checking if the object responds to NSURL methods
            let ns_url: Option<&NSURL> = unsafe { obj.downcast_ref() };
            ns_url.and_then(|url| {
                // SAFETY: path() is safe to call on NSURL
                let path_string = unsafe { url.path() }?;
                Some(PathBuf::from(path_string.to_string()))
            })
        })
    }

    /// Extracts an array of strings from an `NSMetadataItem`.
    ///
    /// Returns an empty vector if the attribute is missing or not an array.
    #[must_use]
    pub fn get_string_array_attribute(item: &NSMetadataItem, key: &NSString) -> Vec<String> {
        // SAFETY: valueForAttribute is safe to call with any key
        let value: Option<Retained<AnyObject>> = unsafe { item.valueForAttribute(key) };

        value
            .and_then(|obj| {
                // Try to interpret as NSArray
                // SAFETY: We're checking if the object is an NSArray
                let ns_array: Option<&NSArray<AnyObject>> = unsafe { obj.downcast_ref() };
                ns_array.map(|array| {
                    let count = array.count();
                    let mut result = Vec::with_capacity(count);

                    for i in 0..count {
                        // SAFETY: Index is within bounds
                        let element = unsafe { array.objectAtIndex(i) };
                        // Try to interpret each element as NSString
                        let ns_string: Option<&NSString> = unsafe { element.downcast_ref() };
                        if let Some(s) = ns_string {
                            result.push(s.to_string());
                        }
                    }

                    result
                })
            })
            .unwrap_or_default()
    }

    /// Extracts the path attribute specifically (using kMDItemPath).
    ///
    /// This is a common operation so it gets its own method.
    #[must_use]
    fn get_path_attribute(item: &NSMetadataItem) -> Option<PathBuf> {
        Self::get_string_attribute(item, keys::path()).map(PathBuf::from)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Converts an NSDate timestamp (seconds since Unix epoch) to `SystemTime`.
///
/// Returns `None` if the timestamp cannot be represented as a `SystemTime`
/// (e.g., dates before the Unix epoch on some platforms).
#[cfg(target_os = "macos")]
fn nsdate_to_system_time(timestamp: f64) -> Option<SystemTime> {
    if timestamp >= 0.0 {
        // Positive timestamp: after Unix epoch
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let secs = timestamp.trunc() as u64;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let nanos = ((timestamp.fract()) * 1_000_000_000.0) as u32;
        UNIX_EPOCH.checked_add(Duration::new(secs, nanos))
    } else {
        // Negative timestamp: before Unix epoch
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let secs = (-timestamp).trunc() as u64;
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let nanos = ((-timestamp).fract() * 1_000_000_000.0) as u32;
        UNIX_EPOCH.checked_sub(Duration::new(secs, nanos))
    }
}

// =============================================================================
// Non-macOS Stubs
// =============================================================================

/// Stub for `MetadataExtractor` on non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub struct MetadataExtractor;

#[cfg(not(target_os = "macos"))]
impl MetadataExtractor {
    /// Stub: Returns `None` on non-macOS platforms.
    #[must_use]
    pub fn extract<T>(_item: &T) -> Option<SpotlightResult> {
        None
    }

    /// Stub: Returns empty vector on non-macOS platforms.
    #[must_use]
    pub fn extract_batch<T>(_items: &T) -> Vec<SpotlightResult> {
        Vec::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SpotlightResult Tests (Cross-Platform)
    // =========================================================================

    #[test]
    fn test_spotlight_result_extension() {
        let result = SpotlightResult {
            path: PathBuf::from("/Users/test/document.pdf"),
            display_name: "document.pdf".to_string(),
            file_size: Some(1024),
            content_type: Some("com.adobe.pdf".to_string()),
            content_type_tree: vec![
                "com.adobe.pdf".to_string(),
                "public.data".to_string(),
                "public.item".to_string(),
            ],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: false,
        };

        assert_eq!(result.extension(), Some("pdf"));
        assert_eq!(result.file_name(), Some("document.pdf"));
    }

    #[test]
    fn test_spotlight_result_no_extension() {
        let result = SpotlightResult {
            path: PathBuf::from("/Users/test/Makefile"),
            display_name: "Makefile".to_string(),
            file_size: Some(512),
            content_type: None,
            content_type_tree: vec![],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: false,
        };

        assert_eq!(result.extension(), None);
        assert_eq!(result.file_name(), Some("Makefile"));
    }

    #[test]
    fn test_spotlight_result_conforms_to_type() {
        let result = SpotlightResult {
            path: PathBuf::from("/Users/test/image.jpg"),
            display_name: "image.jpg".to_string(),
            file_size: Some(2048),
            content_type: Some("public.jpeg".to_string()),
            content_type_tree: vec![
                "public.jpeg".to_string(),
                "public.image".to_string(),
                "public.data".to_string(),
                "public.item".to_string(),
            ],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: false,
        };

        assert!(result.conforms_to_type("public.image"));
        assert!(result.conforms_to_type("public.jpeg"));
        assert!(result.conforms_to_type("PUBLIC.IMAGE")); // Case insensitive
        assert!(!result.conforms_to_type("public.video"));
        assert!(result.is_image());
        assert!(!result.is_application());
    }

    #[test]
    fn test_spotlight_result_is_application() {
        let result = SpotlightResult {
            path: PathBuf::from("/Applications/Safari.app"),
            display_name: "Safari".to_string(),
            file_size: None,
            content_type: Some("com.apple.application-bundle".to_string()),
            content_type_tree: vec![
                "com.apple.application-bundle".to_string(),
                "com.apple.bundle".to_string(),
                "com.apple.package".to_string(),
                "public.directory".to_string(),
            ],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: true,
        };

        assert!(result.is_application());
        assert!(result.is_directory);
        assert!(!result.is_image());
    }

    #[test]
    fn test_spotlight_result_directory() {
        let result = SpotlightResult {
            path: PathBuf::from("/Users/test/Documents"),
            display_name: "Documents".to_string(),
            file_size: None,
            content_type: Some("public.folder".to_string()),
            content_type_tree: vec![
                "public.folder".to_string(),
                "public.directory".to_string(),
                "public.item".to_string(),
            ],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: true,
        };

        assert!(result.is_directory);
        assert!(!result.is_image());
        assert!(!result.is_application());
    }

    // =========================================================================
    // Timestamp Conversion Tests (Cross-Platform)
    // =========================================================================

    #[cfg(target_os = "macos")]
    #[test]
    fn test_nsdate_to_system_time_positive() {
        // Test a known timestamp: 2024-01-01 00:00:00 UTC = 1704067200
        let timestamp = 1704067200.0;
        let system_time = nsdate_to_system_time(timestamp);

        assert!(system_time.is_some());
        let st = system_time.unwrap();
        let duration = st.duration_since(UNIX_EPOCH).unwrap();
        assert_eq!(duration.as_secs(), 1704067200);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_nsdate_to_system_time_with_fractional() {
        let timestamp = 1704067200.5;
        let system_time = nsdate_to_system_time(timestamp);

        assert!(system_time.is_some());
        let st = system_time.unwrap();
        let duration = st.duration_since(UNIX_EPOCH).unwrap();
        assert_eq!(duration.as_secs(), 1704067200);
        assert!(duration.subsec_nanos() > 400_000_000); // ~0.5 seconds in nanos
        assert!(duration.subsec_nanos() < 600_000_000);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_nsdate_to_system_time_epoch() {
        let timestamp = 0.0;
        let system_time = nsdate_to_system_time(timestamp);

        assert!(system_time.is_some());
        let st = system_time.unwrap();
        let duration = st.duration_since(UNIX_EPOCH).unwrap();
        assert_eq!(duration.as_secs(), 0);
    }

    // =========================================================================
    // macOS-Specific Integration Tests
    // =========================================================================

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::*;

        /// Test that keys module produces valid NSString constants.
        #[test]
        fn test_metadata_keys() {
            // Just verify these don't panic
            let _ = keys::path();
            let _ = keys::display_name();
            let _ = keys::fs_name();
            let _ = keys::fs_size();
            let _ = keys::content_type();
            let _ = keys::content_type_tree();
            let _ = keys::fs_content_change_date();
            let _ = keys::fs_creation_date();
            let _ = keys::last_used_date();
        }

        /// Integration test that requires running on macOS with actual Spotlight data.
        /// This test is ignored by default as it requires a real macOS environment.
        #[test]
        #[ignore = "requires macOS Spotlight integration"]
        fn test_extract_from_real_metadata_item() {
            // This test would require creating an NSMetadataQuery,
            // running it, and extracting results. It's marked as ignored
            // because it requires actual Spotlight integration which is
            // tested at a higher level.
            //
            // To run: cargo test --package photoncast-core -- --ignored
        }
    }
}
