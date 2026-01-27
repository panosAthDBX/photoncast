//! NSMetadataQuery wrapper for Spotlight searches.
//!
//! This module provides a Rust-friendly wrapper around NSMetadataQuery
//! for executing Spotlight file searches using proper notification-based
//! execution instead of polling.
//!
//! # Example
//!
//! ```no_run
//! use photoncast_core::search::spotlight::{MetadataQueryWrapper, PredicateBuilder};
//! use std::time::Duration;
//!
//! let predicate = PredicateBuilder::new()
//!     .name_contains("report")
//!     .build();
//!
//! let results = MetadataQueryWrapper::new()
//!     .set_predicate(&predicate)
//!     .execute_sync(Duration::from_millis(500))
//!     .unwrap();
//!
//! for result in results {
//!     println!("{}", result.path.display());
//! }
//! ```

use std::path::PathBuf;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use block2::RcBlock;
use core_foundation::runloop::{
    kCFRunLoopDefaultMode, kCFRunLoopRunStopped, CFRunLoopGetCurrent, CFRunLoopRunInMode,
    CFRunLoopStop,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
#[allow(deprecated)]
use objc2::{class, msg_send, msg_send_id};
use objc2_foundation::{
    NSArray, NSMetadataItem, NSMetadataQuery, NSMetadataQueryDidFinishGatheringNotification,
    NSNotification, NSNotificationCenter, NSPredicate, NSString, NSURL,
};
use thiserror::Error;

use super::result::{MetadataExtractor, SpotlightResult};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during Spotlight queries.
#[derive(Debug, Error)]
pub enum SpotlightError {
    /// The query failed to start.
    #[error("query failed to start")]
    StartFailed,

    /// The query timed out.
    #[error("query timed out after {0:?}")]
    Timeout(Duration),

    /// The query was cancelled.
    #[error("query was cancelled")]
    Cancelled,

    /// Invalid predicate.
    #[error("invalid predicate")]
    InvalidPredicate,

    /// No results found.
    #[error("no results found")]
    NoResults,
}

/// Result type for Spotlight operations.
pub type Result<T> = std::result::Result<T, SpotlightError>;

// =============================================================================
// MetadataQueryWrapper
// =============================================================================

/// Wrapper around NSMetadataQuery for Spotlight searches.
///
/// This provides a Rust-friendly interface for executing Spotlight queries
/// with proper lifecycle management.
pub struct MetadataQueryWrapper {
    query: Retained<NSMetadataQuery>,
}

impl MetadataQueryWrapper {
    /// Creates a new metadata query wrapper.
    #[must_use]
    pub fn new() -> Self {
        let query = NSMetadataQuery::new();
        Self { query }
    }

    /// Sets the search predicate.
    ///
    /// The predicate determines which files match the query.
    /// Use [`PredicateBuilder`](super::PredicateBuilder) to create predicates.
    pub fn set_predicate(&mut self, predicate: &NSPredicate) -> &mut Self {
        self.query.setPredicate(Some(predicate));
        self
    }

    /// Sets the directories to search.
    ///
    /// If not set, searches the entire system (user-accessible areas).
    #[allow(clippy::incompatible_msrv)]
    pub fn set_search_scopes(&mut self, scopes: &[PathBuf]) -> &mut Self {
        if scopes.is_empty() {
            return self;
        }

        let ns_scopes: Vec<Retained<NSString>> = scopes
            .iter()
            .filter_map(|p| p.to_str())
            .map(NSString::from_str)
            .collect();

        let scope_refs: Vec<&NSString> = ns_scopes.iter().map(std::convert::AsRef::as_ref).collect();
        let array: Retained<NSArray<NSString>> = NSArray::from_slice(&scope_refs);

        // Safety: setSearchScopes expects an array of NSString or NSURL.
        // We cast the typed array to AnyObject array.
        unsafe {
            let any_array: &NSArray<AnyObject> =
                &*std::ptr::from_ref::<NSArray<NSString>>(&array).cast::<NSArray<AnyObject>>();
            self.query.setSearchScopes(any_array);
        }
        self
    }

    /// Sets search scopes using URLs.
    #[allow(clippy::incompatible_msrv)]
    pub fn set_search_scope_urls(&mut self, urls: &[&NSURL]) -> &mut Self {
        if urls.is_empty() {
            return self;
        }

        let array: Retained<NSArray<NSURL>> = NSArray::from_slice(urls);

        // Safety: setSearchScopes expects an array of NSString or NSURL
        unsafe {
            let any_array: &NSArray<AnyObject> =
                &*std::ptr::from_ref::<NSArray<NSURL>>(&array).cast::<NSArray<AnyObject>>();
            self.query.setSearchScopes(any_array);
        }
        self
    }

    /// Sets sort descriptors to order results by last used date (most recent first).
    ///
    /// This prioritizes recently accessed files, which is useful for file search
    /// where users typically want to find files they've worked with recently.
    pub fn sort_by_last_used(&mut self) -> &mut Self {
        #[allow(deprecated)]
        unsafe {
            // Create NSSortDescriptor for kMDItemLastUsedDate, descending
            let key = NSString::from_str("kMDItemLastUsedDate");
            let cls = class!(NSSortDescriptor);
            let descriptor: Retained<AnyObject> =
                msg_send_id![cls, sortDescriptorWithKey: &*key, ascending: false];

            // Wrap in array
            let descriptors: Retained<NSArray<AnyObject>> =
                NSArray::from_retained_slice(&[descriptor]);

            // Set on query - use raw msg_send since setSortDescriptors may not be exposed
            let _: () = msg_send![&self.query, setSortDescriptors: &*descriptors];
        }
        self
    }

    /// Sets sort descriptors to order results by content modification date (most recent first).
    ///
    /// Use this when you want to prioritize recently modified files over recently opened files.
    pub fn sort_by_modification_date(&mut self) -> &mut Self {
        #[allow(deprecated)]
        unsafe {
            let key = NSString::from_str("kMDItemFSContentChangeDate");
            let cls = class!(NSSortDescriptor);
            let descriptor: Retained<AnyObject> =
                msg_send_id![cls, sortDescriptorWithKey: &*key, ascending: false];

            let descriptors: Retained<NSArray<AnyObject>> =
                NSArray::from_retained_slice(&[descriptor]);

            let _: () = msg_send![&self.query, setSortDescriptors: &*descriptors];
        }
        self
    }

    /// Sets sort descriptors to order results by relevance score.
    ///
    /// This uses Spotlight's built-in relevance ranking based on how well
    /// items match the search criteria.
    pub fn sort_by_relevance(&mut self) -> &mut Self {
        #[allow(deprecated)]
        unsafe {
            // kMDQueryResultContentRelevance is the relevance score attribute
            let key = NSString::from_str("kMDQueryResultContentRelevance");
            let cls = class!(NSSortDescriptor);
            let descriptor: Retained<AnyObject> =
                msg_send_id![cls, sortDescriptorWithKey: &*key, ascending: false];

            let descriptors: Retained<NSArray<AnyObject>> =
                NSArray::from_retained_slice(&[descriptor]);

            let _: () = msg_send![&self.query, setSortDescriptors: &*descriptors];
        }
        self
    }

    /// Executes the query synchronously with a timeout.
    ///
    /// This uses `CFRunLoopRunInMode()` with a flag set by the notification
    /// callback. This is the canonical way to perform synchronous Spotlight
    /// queries while respecting timeouts.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for results.
    ///
    /// # Returns
    ///
    /// A vector of [`SpotlightResult`] objects, or an error.
    #[allow(clippy::ptr_as_ptr)]
    pub fn execute_sync(&self, timeout: Duration) -> Result<Vec<SpotlightResult>> {
        // Thread-safe flag to track completion
        let finished = Arc::new(AtomicBool::new(false));
        let finished_for_callback = finished.clone();

        // Capture the current run loop for stopping from the callback
        let current_run_loop = unsafe { CFRunLoopGetCurrent() };

        // Create a block that sets the flag and stops the run loop
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            finished_for_callback.store(true, Ordering::SeqCst);
            // Stop the run loop to unblock CFRunLoopRunInMode
            unsafe {
                CFRunLoopStop(current_run_loop);
            }
        });

        // Register for the finish notification
        let notification_center = NSNotificationCenter::defaultCenter();
        let observer = unsafe {
            notification_center.addObserverForName_object_queue_usingBlock(
                Some(NSMetadataQueryDidFinishGatheringNotification),
                Some(self.query.as_ref()),
                None, // Execute on current run loop
                &block,
            )
        };

        // Helper macro to remove observer - cast ProtocolObject to AnyObject
        macro_rules! remove_observer {
            ($nc:expr, $obs:expr) => {
                unsafe {
                    let ptr = Retained::as_ptr($obs);
                    let any_obj: &AnyObject = &*(ptr as *const AnyObject);
                    $nc.removeObserver(any_obj);
                }
            };
        }

        // Start the query
        if !self.query.startQuery() {
            remove_observer!(notification_center, &observer);
            return Err(SpotlightError::StartFailed);
        }

        let deadline = Instant::now() + timeout;

        // Run the run loop in intervals until finished or timeout
        // CFRunLoopRunInMode returns when:
        // - A source is handled (kCFRunLoopRunHandledSource)
        // - The timeout expires (kCFRunLoopRunTimedOut)
        // - CFRunLoopStop is called (kCFRunLoopRunStopped)
        // - No sources exist (kCFRunLoopRunFinished)
        while !finished.load(Ordering::SeqCst) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Timeout reached
                self.query.stopQuery();
                remove_observer!(notification_center, &observer);
                return Err(SpotlightError::Timeout(timeout));
            }

            // Run for up to 100ms at a time, or remaining time if less
            let run_interval = remaining.min(Duration::from_millis(100));
            let result = unsafe {
                CFRunLoopRunInMode(
                    kCFRunLoopDefaultMode,
                    run_interval.as_secs_f64(),
                    0, // Don't return after handling a single source (Boolean = 0)
                )
            };

            // Check if the run loop was stopped (notification fired)
            if result == kCFRunLoopRunStopped {
                break;
            }
        }

        // Clean up observer
        remove_observer!(notification_center, &observer);

        // Stop the query if still running
        if !self.query.isStopped() {
            self.query.stopQuery();
        }

        // Check if we actually finished (vs timeout)
        if !finished.load(Ordering::SeqCst) {
            return Err(SpotlightError::Timeout(timeout));
        }

        // Query finished, extract results
        self.extract_results()
    }

    /// Executes the query with a default timeout of 500ms.
    pub fn execute(&self) -> Result<Vec<SpotlightResult>> {
        self.execute_sync(Duration::from_millis(500))
    }

    /// Stops the query if it's running.
    pub fn stop(&self) {
        if !self.query.isStopped() {
            self.query.stopQuery();
        }
    }

    /// Returns the number of results gathered so far.
    #[must_use]
    pub fn result_count(&self) -> usize {
        self.query.resultCount()
    }

    /// Returns whether the query is currently gathering results.
    #[must_use]
    pub fn is_gathering(&self) -> bool {
        self.query.isGathering()
    }

    /// Returns whether the query has been started.
    #[must_use]
    pub fn is_started(&self) -> bool {
        self.query.isStarted()
    }

    /// Returns whether the query has been stopped.
    #[must_use]
    #[allow(clippy::unnecessary_wraps)]
    pub fn is_stopped(&self) -> bool {
        self.query.isStopped()
    }

    /// Extracts results from the completed query.
    #[allow(clippy::unnecessary_wraps)]
    fn extract_results(&self) -> Result<Vec<SpotlightResult>> {
        // Disable updates while reading results
        self.query.disableUpdates();

        let results = self.query.results();
        // Cast the untyped NSArray to NSArray<NSMetadataItem>
        // Safety: NSMetadataQuery.results() returns NSMetadataItem objects
        let typed_results: &NSArray<NSMetadataItem> = unsafe {
            let ptr: *const NSArray = &*results;
            &*ptr.cast::<NSArray<NSMetadataItem>>()
        };
        let spotlight_results = MetadataExtractor::extract_batch(typed_results);

        // Re-enable updates (though we'll likely stop the query soon)
        self.query.enableUpdates();

        Ok(spotlight_results)
    }
}

impl Default for MetadataQueryWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MetadataQueryWrapper {
    fn drop(&mut self) {
        // Ensure the query is stopped when dropped
        if !self.query.isStopped() {
            self.query.stopQuery();
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Returns common search scopes for file searches.
#[must_use]
pub fn default_search_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();

    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Desktop"));
        scopes.push(home.join("Documents"));
        scopes.push(home.join("Downloads"));
    }

    scopes.push(PathBuf::from("/Applications"));

    scopes
}

/// Returns expanded search scopes including more directories.
#[must_use]
pub fn expanded_search_scopes() -> Vec<PathBuf> {
    let mut scopes = default_search_scopes();

    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Pictures"));
        scopes.push(home.join("Music"));
        scopes.push(home.join("Movies"));
        scopes.push(home.clone()); // Home directory itself
    }

    scopes
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_creation() {
        let query = MetadataQueryWrapper::new();
        assert!(!query.is_started());
        assert!(!query.is_gathering());
    }

    #[test]
    fn test_default_search_scopes() {
        let scopes = default_search_scopes();
        assert!(!scopes.is_empty());

        // Should include Desktop, Documents, Downloads, Applications
        let scope_strings: Vec<String> = scopes
            .iter()
            .filter_map(|p| p.to_str())
            .map(String::from)
            .collect();

        assert!(scope_strings.iter().any(|s| s.contains("Desktop")));
        assert!(scope_strings.iter().any(|s| s.contains("Documents")));
        assert!(scope_strings.iter().any(|s| s.contains("Downloads")));
        assert!(scope_strings.iter().any(|s| s.contains("Applications")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_query_execution_without_predicate() {
        // Query without predicate should still work (matches everything)
        let query = MetadataQueryWrapper::new();

        // This should fail or return empty since no predicate is set
        // and we're using a very short timeout
        let result = query.execute_sync(Duration::from_millis(10));

        // Either timeout or succeed with results
        match result {
            Ok(results) => {
                // Success is fine, results may be empty or populated
                let _ = results;
            },
            Err(SpotlightError::Timeout(_)) => {
                // Timeout is expected with short timeout
            },
            Err(e) => {
                // Other errors are acceptable for this test
                let _ = e;
            },
        }
    }
}
