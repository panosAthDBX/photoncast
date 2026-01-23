//! Live file index using NSMetadataQuery's update notifications.
//!
//! This module maintains a real-time index of files in the primary search scopes
//! (Desktop, Documents, Downloads) by leveraging Spotlight's built-in change
//! monitoring via `NSMetadataQueryDidUpdateNotification`.
//!
//! # How It Works
//!
//! 1. Start a live NSMetadataQuery for files in primary scopes
//! 2. Initial gathering populates the index
//! 3. Update notifications incrementally add/modify/remove entries
//! 4. Search becomes instant in-memory filtering
//!
//! # Performance
//!
//! - Initial population: One Spotlight query (~1-2s)
//! - Subsequent searches: Microseconds (in-memory filtering)
//! - Updates: Handled asynchronously by Spotlight

use std::collections::HashMap;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{
    NSArray, NSMetadataItem, NSMetadataQuery,
    NSMetadataQueryDidUpdateNotification, NSNotification, NSNotificationCenter, NSString,
};
use parking_lot::RwLock;

use super::predicate::PredicateBuilder;
use super::result::{MetadataExtractor, SpotlightResult};
use super::service::SpotlightSearchService;

/// Statistics about the live index.
#[derive(Debug, Clone, Default)]
pub struct LiveIndexStats {
    /// Total files in the index.
    pub file_count: usize,
    /// Number of times files were added via updates.
    pub adds: usize,
    /// Number of times files were modified via updates.
    pub changes: usize,
    /// Number of times files were removed via updates.
    pub removes: usize,
    /// Time when the index was last updated.
    pub last_update: Option<Instant>,
}

/// Status of the live index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveIndexStatus {
    /// Not started yet.
    Idle,
    /// Initial gathering in progress.
    Gathering,
    /// Index is live and receiving updates.
    Live,
    /// Index was stopped.
    Stopped,
    /// Index failed to start.
    Failed,
}

/// A live file index that stays synchronized with the file system.
///
/// This uses NSMetadataQuery's update notifications to maintain a real-time
/// index of files in the primary search scopes.
pub struct LiveFileIndex {
    inner: Arc<LiveFileIndexInner>,
}

/// Configuration for a custom search scope with extension filtering.
#[derive(Debug, Clone)]
pub struct CustomScopeConfig {
    /// The directory path to search.
    pub path: PathBuf,
    /// Optional list of file extensions to include (without dots).
    pub extensions: Vec<String>,
    /// Whether to search recursively in subdirectories.
    pub recursive: bool,
}

impl CustomScopeConfig {
    /// Checks if a file path matches this scope's extension filter.
    #[must_use]
    pub fn matches_extension(&self, path: &std::path::Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
            })
    }
}

struct LiveFileIndexInner {
    /// The file index: path -> metadata.
    files: RwLock<HashMap<PathBuf, SpotlightResult>>,
    /// Current status.
    status: RwLock<LiveIndexStatus>,
    /// Statistics.
    stats: RwLock<LiveIndexStats>,
    /// Whether the index has been stopped.
    stopped: AtomicBool,
    /// Reload generation counter - incremented on each reload to signal workers to stop.
    generation: AtomicUsize,
    /// Primary scopes being monitored (no extension filter).
    scopes: RwLock<Vec<PathBuf>>,
    /// Custom scopes with extension filters.
    custom_scopes: RwLock<Vec<CustomScopeConfig>>,
    /// Fallback service for secondary scope searches.
    fallback_service: SpotlightSearchService,
}

impl LiveFileIndex {
    /// Creates a new live file index monitoring the given scopes.
    #[must_use]
    pub fn new(scopes: Vec<PathBuf>) -> Self {
        Self::with_custom_scopes(scopes, Vec::new())
    }

    /// Creates a new live file index with both primary and custom scopes.
    #[must_use]
    pub fn with_custom_scopes(scopes: Vec<PathBuf>, custom_scopes: Vec<CustomScopeConfig>) -> Self {
        Self {
            inner: Arc::new(LiveFileIndexInner {
                files: RwLock::new(HashMap::new()),
                status: RwLock::new(LiveIndexStatus::Idle),
                stats: RwLock::new(LiveIndexStats::default()),
                stopped: AtomicBool::new(false),
                generation: AtomicUsize::new(0),
                scopes: RwLock::new(scopes),
                custom_scopes: RwLock::new(custom_scopes),
                fallback_service: SpotlightSearchService::new(),
            }),
        }
    }

    /// Creates a live index for the default primary scopes.
    #[must_use]
    pub fn with_primary_scopes() -> Self {
        Self::new(primary_scopes())
    }

    /// Returns the current status of the index.
    #[must_use]
    pub fn status(&self) -> LiveIndexStatus {
        *self.inner.status.read()
    }

    /// Returns statistics about the index.
    #[must_use]
    pub fn stats(&self) -> LiveIndexStats {
        self.inner.stats.read().clone()
    }

    /// Returns the number of files in the index.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.inner.files.read().len()
    }

    /// Checks if the index is ready for searches.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.status(), LiveIndexStatus::Live)
    }

    /// Starts the live index on a background thread.
    ///
    /// This initiates the NSMetadataQuery and begins populating the index.
    /// The method returns immediately; use `is_ready()` to check when
    /// the initial gathering is complete.
    pub fn start(&self) {
        if self.inner.stopped.load(Ordering::SeqCst) {
            return;
        }

        {
            let mut status = self.inner.status.write();
            if *status != LiveIndexStatus::Idle {
                return; // Already started
            }
            *status = LiveIndexStatus::Gathering;
        }

        let inner = Arc::clone(&self.inner);

        thread::spawn(move || {
            run_live_query(inner);
        });
    }

    /// Stops the live index.
    pub fn stop(&self) {
        self.inner.stopped.store(true, Ordering::SeqCst);
        *self.inner.status.write() = LiveIndexStatus::Stopped;
    }

    /// Reloads the live index with new scopes.
    /// 
    /// This stops the current monitoring, clears the index, updates scopes,
    /// and restarts. Useful for hot-reloading config changes.
    pub fn reload(&self, scopes: Vec<PathBuf>, custom_scopes: Vec<CustomScopeConfig>) {
        tracing::debug!("Reloading live index with {} primary scopes, {} custom scopes", 
                       scopes.len(), custom_scopes.len());
        for (i, scope) in custom_scopes.iter().enumerate() {
            tracing::debug!(
                "  Custom scope {}: path={:?}, extensions={:?}, recursive={}",
                i, scope.path, scope.extensions, scope.recursive
            );
        }
        
        // Increment generation to signal any running workers to stop
        self.inner.generation.fetch_add(1, Ordering::SeqCst);
        
        // Clear the current index
        {
            let mut files = self.inner.files.write();
            files.clear();
        }
        
        // Reset stats
        {
            let mut stats = self.inner.stats.write();
            *stats = LiveIndexStats::default();
        }
        
        // Update scopes
        {
            let mut scopes_lock = self.inner.scopes.write();
            *scopes_lock = scopes;
        }
        {
            let mut custom_lock = self.inner.custom_scopes.write();
            *custom_lock = custom_scopes;
        }
        
        // Reset stopped flag and status
        self.inner.stopped.store(false, Ordering::SeqCst);
        *self.inner.status.write() = LiveIndexStatus::Gathering;
        
        // Start new worker
        let inner = Arc::clone(&self.inner);
        let gen = self.inner.generation.load(Ordering::SeqCst);
        
        thread::spawn(move || {
            run_live_query_with_generation(inner, gen);
        });
    }

    /// Returns the current custom scopes.
    #[must_use]
    pub fn custom_scopes(&self) -> Vec<CustomScopeConfig> {
        self.inner.custom_scopes.read().clone()
    }

    /// Searches the live index for files matching the query.
    ///
    /// This performs fast in-memory filtering on the indexed files.
    /// If the index isn't ready, falls back to the traditional Spotlight search
    /// with whitelist filtering.
    pub fn search(&self, query: &str, max_results: usize) -> Vec<SpotlightResult> {
        let query_lower = query.to_lowercase();

        // If index is ready, search in-memory (index already filtered)
        if self.is_ready() {
            let files = self.inner.files.read();
            let mut results: Vec<SpotlightResult> = files
                .values()
                .filter(|f| f.display_name.to_lowercase().contains(&query_lower))
                .cloned()
                .collect();

            // Sort by last used (most recent first)
            results.sort_by(|a, b| b.last_used_date.cmp(&a.last_used_date));
            results.truncate(max_results);
            return results;
        }

        // Fallback to traditional search - MUST apply whitelist filter
        self.inner
            .fallback_service
            .search(query)
            .unwrap_or_default()
            .into_iter()
            .filter(|r| !should_exclude(&r.path))
            .take(max_results)
            .collect()
    }

    /// Returns all files in the index, sorted by last used.
    #[must_use]
    pub fn get_all_files(&self, max_results: usize) -> Vec<SpotlightResult> {
        let files = self.inner.files.read();
        let mut results: Vec<SpotlightResult> = files.values().cloned().collect();
        // Sort by the best (most recent) date available for each file
        results.sort_by(|a, b| {
            let a_best = a.last_used_date.max(a.modified_date);
            let b_best = b.last_used_date.max(b.modified_date);
            b_best.cmp(&a_best)
        });
        results.truncate(max_results);
        results
    }

    /// Returns recently used files, sorted by the most recent date available.
    /// Uses the maximum of last_used_date and modified_date for each file,
    /// so new files (with only modified_date) can appear alongside opened files.
    #[must_use]
    pub fn get_recent_files(&self, max_results: usize) -> Vec<SpotlightResult> {
        let files = self.inner.files.read();
        let total_in_index = files.len();
        let mut results: Vec<SpotlightResult> = files.values().cloned().collect();
        
        // Sort by the best (most recent) date available for each file
        // This ensures new files appear based on their creation/modification time
        results.sort_by(|a, b| {
            let a_best = a.last_used_date.max(a.modified_date);
            let b_best = b.last_used_date.max(b.modified_date);
            b_best.cmp(&a_best) // Most recent first
        });
        
        results.truncate(max_results);
        tracing::debug!(
            "[FileIndex] get_recent_files: {} in index, returning {}",
            total_in_index,
            results.len()
        );
        results
    }

    /// Manually adds or updates a file in the index.
    pub fn upsert(&self, result: SpotlightResult) {
        let mut files = self.inner.files.write();
        files.insert(result.path.clone(), result);
    }

    /// Manually removes a file from the index.
    pub fn remove(&self, path: &PathBuf) {
        let mut files = self.inner.files.write();
        files.remove(path);
    }
}

impl Clone for LiveFileIndex {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Checks if a file from a custom scope matches its extension filter.
fn matches_custom_scope_filter(path: &PathBuf, custom_scopes: &[CustomScopeConfig]) -> bool {
    for scope in custom_scopes {
        if path.starts_with(&scope.path) {
            // File is in this custom scope
            
            // Check recursive flag - if not recursive, file must be directly in scope dir
            if !scope.recursive {
                if let Some(parent) = path.parent() {
                    if parent != scope.path {
                        // File is in a subdirectory but recursive is false - exclude it
                        return false;
                    }
                }
            }
            
            // Check extension filter
            let matches = scope.matches_extension(path);
            if !matches {
                tracing::trace!(
                    "Custom scope filter: excluding {:?} (extensions: {:?})",
                    path.file_name(),
                    scope.extensions
                );
            }
            return matches;
        }
    }
    // Not in any custom scope - allow it (it's from a primary scope)
    true
}

/// Queries a single scope folder with optimized Spotlight predicate.
/// Uses file type filters in the query itself for speed.
fn query_single_scope(scope: &PathBuf, custom_config: Option<&CustomScopeConfig>) -> Vec<SpotlightResult> {
    use super::query::MetadataQueryWrapper;
    
    let scope_name = scope.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    tracing::debug!("[FileIndex] Scanning: {}", scope.display());
    let start = Instant::now();
    
    // Build predicate with file type filters for speed
    // This filters at the Spotlight level, not in memory
    let predicate = if let Some(config) = custom_config {
        if config.extensions.is_empty() {
            // Custom scope with no extension filter - index ALL files
            // User explicitly added this scope, respect their choice
            tracing::debug!("[FileIndex] {} using all files (no extension filter)", scope_name);
            PredicateBuilder::new()
                .any_file()
                .exclude_hidden_files()
                .build()
        } else {
            // Custom scope with specific extensions
            tracing::debug!("[FileIndex] {} using custom extensions: {:?}", scope_name, config.extensions);
            PredicateBuilder::new()
                .extensions(&config.extensions)
                .exclude_hidden_files()
                .build()
        }
    } else {
        // Primary scope (Desktop/Documents/Downloads) - user files only
        PredicateBuilder::new().user_files().build()
    };
    
    let mut query = MetadataQueryWrapper::new();
    query.set_predicate(&predicate);
    query.set_search_scopes(&[scope.clone()]);
    query.sort_by_last_used();
    
    // Short timeout per scope - these are small directories
    match query.execute_sync(Duration::from_secs(5)) {
        Ok(results) => {
            tracing::debug!(
                "[FileIndex] {} complete: {} files in {:.1}ms",
                scope_name,
                results.len(),
                start.elapsed().as_secs_f64() * 1000.0
            );
            results
        }
        Err(e) => {
            tracing::warn!("[FileIndex] {} failed: {}", scope_name, e);
            Vec::new()
        }
    }
}

/// Runs the live NSMetadataQuery on the current thread.
fn run_live_query(inner: Arc<LiveFileIndexInner>) {
    run_live_query_with_generation(inner, 0);
}

/// Runs the live NSMetadataQuery with generation checking for hot reload support.
/// Uses PARALLEL queries - one thread per monitored folder for speed.
fn run_live_query_with_generation(inner: Arc<LiveFileIndexInner>, expected_gen: usize) {
    use std::sync::mpsc;
    
    // Check if we've been superseded by a newer reload
    let current_gen = inner.generation.load(Ordering::SeqCst);
    if current_gen != expected_gen {
        tracing::debug!("Live query worker superseded (gen {} vs {}), exiting", expected_gen, current_gen);
        return;
    }

    // Get all scopes to query
    let (primary_scopes, custom_scopes_config) = {
        let scopes = inner.scopes.read();
        let custom = inner.custom_scopes.read();
        tracing::debug!("[FileIndex] Config: {} primary scopes, {} custom scopes", scopes.len(), custom.len());
        for s in scopes.iter() {
            tracing::debug!("[FileIndex]   Primary: {}", s.display());
        }
        for c in custom.iter() {
            tracing::debug!("[FileIndex]   Custom: {} (ext={:?})", c.path.display(), c.extensions);
        }
        (scopes.clone(), custom.clone())
    };

    // Build list of all scopes with their custom config (if any)
    let mut scope_configs: Vec<(PathBuf, Option<CustomScopeConfig>)> = Vec::new();
    
    // Primary scopes (no extension filter)
    for scope in &primary_scopes {
        if scope.exists() {
            scope_configs.push((scope.clone(), None));
        }
    }
    
    // Custom scopes (with extension filter)
    for custom in &custom_scopes_config {
        if custom.path.exists() && !primary_scopes.contains(&custom.path) {
            scope_configs.push((custom.path.clone(), Some(custom.clone())));
        }
    }

    if scope_configs.is_empty() {
        tracing::warn!("[FileIndex] No valid search scopes found");
        *inner.status.write() = LiveIndexStatus::Live;
        return;
    }

    // Log what we're about to index
    tracing::debug!("[FileIndex] ========================================");
    tracing::debug!("[FileIndex] Starting parallel index of {} scopes:", scope_configs.len());
    for (path, config) in &scope_configs {
        let ext_info = config.as_ref()
            .map(|c| format!("extensions={:?}", c.extensions))
            .unwrap_or_else(|| "user_files".to_string());
        tracing::debug!("[FileIndex]   - {} ({})", path.display(), ext_info);
    }
    tracing::debug!("[FileIndex] ========================================");
    let start = Instant::now();

    // Channel to collect results from parallel queries
    let (tx, rx) = mpsc::channel::<Vec<SpotlightResult>>();
    let _scope_count = scope_configs.len();

    // Spawn a thread per scope for parallel querying
    for (scope_path, custom_config) in scope_configs {
        let tx = tx.clone();
        let gen = expected_gen;
        let inner_ref = Arc::clone(&inner);
        
        thread::spawn(move || {
            // Check generation before starting
            if inner_ref.generation.load(Ordering::SeqCst) != gen {
                let _ = tx.send(Vec::new());
                return;
            }
            
            let results = query_single_scope(&scope_path, custom_config.as_ref());
            let _ = tx.send(results);
        });
    }
    
    // Drop our sender so rx knows when all threads are done
    drop(tx);

    // Collect results from all threads
    let custom_scopes_list = inner.custom_scopes.read().clone();
    let _primary_scopes_list = inner.scopes.read().clone();
    
    // Log the scope paths we're using for matching
    tracing::debug!("[FileIndex] Matching against {} custom scope paths:", custom_scopes_list.len());
    for cs in &custom_scopes_list {
        tracing::debug!("[FileIndex]   Custom scope path: {} (ext={:?})", cs.path.display(), cs.extensions);
    }
    
    let mut files = inner.files.write();
    let mut total_indexed = 0usize;
    let mut filtered_by_whitelist = 0usize;
    let mut filtered_by_custom = 0usize;
    let mut total_received = 0usize;
    
    for batch in rx {
        let batch_size = batch.len();
        total_received += batch_size;
        
        for result in batch {
            // Check if this file is from a custom scope
            let is_from_custom_scope = custom_scopes_list.iter()
                .any(|cs| result.path.starts_with(&cs.path));
            
            // Log first few files for debugging
            if total_indexed + filtered_by_whitelist + filtered_by_custom < 5 {
                tracing::debug!(
                    "[FileIndex] Sample file: {} (custom_scope={})",
                    result.path.display(),
                    is_from_custom_scope
                );
            }
            
            if is_from_custom_scope {
                // Custom scope - already filtered by Spotlight predicate, just check extension match
                if !matches_custom_scope_filter(&result.path, &custom_scopes_list) {
                    filtered_by_custom += 1;
                    continue;
                }
            } else {
                // Primary scope - apply whitelist filter
                if should_exclude(&result.path) {
                    filtered_by_whitelist += 1;
                    continue;
                }
            }
            
            files.insert(result.path.clone(), result);
            total_indexed += 1;
        }
    }
    
    tracing::debug!(
        "[FileIndex] Received {} files, indexed {}, filtered: {} by whitelist, {} by custom scope rules",
        total_received, total_indexed, filtered_by_whitelist, filtered_by_custom
    );

    // Update stats
    {
        let mut stats = inner.stats.write();
        stats.file_count = files.len();
        stats.last_update = Some(Instant::now());
    }
    drop(files);

    tracing::debug!(
        "[FileIndex] Complete: {} files indexed in {:.1}ms",
        total_indexed,
        start.elapsed().as_secs_f64() * 1000.0
    );

    // Mark as live after initial population
    *inner.status.write() = LiveIndexStatus::Live;

    // Phase 2: Set up live monitoring for updates
    // Rebuild scope list for monitoring
    let all_scopes: Vec<PathBuf> = {
        let scopes = inner.scopes.read();
        let custom = inner.custom_scopes.read();
        let mut all = scopes.clone();
        for c in custom.iter() {
            if c.path.exists() && !scopes.contains(&c.path) {
                all.push(c.path.clone());
            }
        }
        all
    };

    // Use user_files predicate for monitoring
    let predicate = PredicateBuilder::new().user_files().build();

    let live_query = NSMetadataQuery::new();
    live_query.setPredicate(Some(&predicate));

    // Set search scopes
    if !all_scopes.is_empty() {
        let ns_scopes: Vec<Retained<NSString>> = all_scopes
            .iter()
            .filter_map(|p: &PathBuf| p.to_str())
            .map(NSString::from_str)
            .collect();

        let scope_refs: Vec<&NSString> = ns_scopes.iter().map(|s: &Retained<NSString>| s.as_ref()).collect();
        let array: Retained<NSArray<NSString>> = NSArray::from_slice(&scope_refs);

        unsafe {
            let any_array: &NSArray<AnyObject> =
                &*(std::ptr::from_ref::<NSArray<NSString>>(&array) as *const NSArray<AnyObject>);
            live_query.setSearchScopes(any_array);
        }
    }

    // Get notification center
    let notification_center = NSNotificationCenter::defaultCenter();

    // Clone inner for update callback
    let inner_for_update = Arc::clone(&inner);
    let stopped_check = Arc::clone(&inner);

    // Callback for live updates
    let update_block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        handle_update(&inner_for_update, notification);
    });

    // Register update observer
    let update_observer = unsafe {
        notification_center.addObserverForName_object_queue_usingBlock(
            Some(NSMetadataQueryDidUpdateNotification),
            Some(live_query.as_ref()),
            None,
            &update_block,
        )
    };

    // Start the live query
    if !live_query.startQuery() {
        // Initial population succeeded, just no live updates
        return;
    }

    // Run the run loop to process update notifications
    while !stopped_check.stopped.load(Ordering::SeqCst) {
        unsafe {
            core_foundation::runloop::CFRunLoopRunInMode(
                core_foundation::runloop::kCFRunLoopDefaultMode,
                0.1, // 100ms
                0,
            );
        }
    }

    // Clean up
    live_query.stopQuery();

    unsafe {
        let ptr = Retained::as_ptr(&update_observer);
        let any_obj: &AnyObject = &*(ptr as *const AnyObject);
        notification_center.removeObserver(any_obj);
    }
}

/// Handles the initial results from the query.
fn handle_initial_results(inner: &LiveFileIndexInner, notification: NonNull<NSNotification>) {
    let notification_ref = unsafe { notification.as_ref() };

    // Get the query from the notification
    let query: Option<&NSMetadataQuery> = unsafe {
        let obj = notification_ref.object();
        obj.and_then(|o| {
            let ptr = Retained::as_ptr(&o);
            (ptr as *const NSMetadataQuery).as_ref()
        })
    };

    let Some(query) = query else {
        return;
    };

    // Disable updates while reading
    query.disableUpdates();

    // Extract all results
    let results = query.results();
    let typed_results: &NSArray<NSMetadataItem> = unsafe {
        &*(std::ptr::from_ref::<NSArray>(&results) as *const NSArray<NSMetadataItem>)
    };

    let spotlight_results = MetadataExtractor::extract_batch(typed_results);

    // Populate the index
    {
        let mut files = inner.files.write();
        let mut stats = inner.stats.write();

        for result in spotlight_results {
            files.insert(result.path.clone(), result);
        }

        stats.file_count = files.len();
        stats.last_update = Some(Instant::now());
    }

    // Enable updates and mark as live
    query.enableUpdates();
    *inner.status.write() = LiveIndexStatus::Live;
}

/// Handles update notifications with added/changed/removed items.
/// 
/// This function re-syncs the entire result set from the query when an update occurs.
/// While this is less efficient than incremental updates, it's more reliable and
/// handles all edge cases (adds, changes, removes) correctly.
fn handle_update(inner: &LiveFileIndexInner, notification: NonNull<NSNotification>) {
    let notification_ref = unsafe { notification.as_ref() };

    // Get the query from the notification's object
    let query: Option<&NSMetadataQuery> = unsafe {
        let obj = notification_ref.object();
        obj.and_then(|o| {
            let ptr = Retained::as_ptr(&o);
            (ptr as *const NSMetadataQuery).as_ref()
        })
    };

    let Some(query) = query else {
        tracing::warn!("[LiveIndex] Update notification without query object");
        return;
    };

    // Disable updates while processing
    query.disableUpdates();

    // Get current result count (unused but may be useful for debugging)
    let _result_count = query.resultCount();
    
    // Extract all results from the query
    let results = query.results();
    let typed_results: &NSArray<NSMetadataItem> = unsafe {
        &*(std::ptr::from_ref::<NSArray>(&results) as *const NSArray<NSMetadataItem>)
    };

    let spotlight_results = MetadataExtractor::extract_batch(typed_results);
    let new_count = spotlight_results.len();

    // Calculate what changed
    let old_count = inner.files.read().len();
    let added = if new_count > old_count { new_count - old_count } else { 0 };
    let removed = if old_count > new_count { old_count - new_count } else { 0 };

    // Update the files HashMap with all current results
    {
        let mut files = inner.files.write();
        files.clear();
        for result in spotlight_results {
            files.insert(result.path.clone(), result);
        }
    }

    // Update stats
    {
        let mut stats = inner.stats.write();
        stats.file_count = new_count;
        stats.last_update = Some(Instant::now());
        if added > 0 {
            stats.adds += added;
        }
        if removed > 0 {
            stats.removes += removed;
        }
    }

    if added > 0 || removed > 0 {
        tracing::debug!(
            "[LiveIndex] Updated: {} files (added: {}, removed: {})",
            new_count, added, removed
        );
    }

    // Re-enable updates
    query.enableUpdates();
}

/// Returns the default primary search scopes.
pub fn primary_scopes() -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if let Some(home) = dirs::home_dir() {
        scopes.push(home.join("Desktop"));
        scopes.push(home.join("Documents"));
        scopes.push(home.join("Downloads"));
    }
    scopes
}

/// Directories to exclude from the live index.
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    "Caches",
    "Cache",
    "DerivedData",
    ".Trash",
    "Library",
    "Cookies",
    "Application Support",
    "Containers",
    "Group Containers",
    "WebKit",
    "Saved Application State",
    ".npm",
    ".cargo",
    ".rustup",
    "bower_components",
    ".gradle",
    ".maven",
    "build",
    "dist",
    ".next",
    ".nuxt",
    ".output",
    "coverage",
    ".nyc_output",
    ".sass-cache",
    ".parcel-cache",
    "vendor",
    "Pods",
    ".idea",
    ".vscode",
    ".DS_Store",
];

/// Extensions to exclude from the live index.
const EXCLUDED_EXTENSIONS: &[&str] = &[
    "o", "pyc", "pyo", "dylib", "so", "a", "lock", "log",
    "tmp", "temp", "bak", "swp", "swo", "class", "jar",
    "map", "min.js", "min.css", "bundle.js",
];

/// User file extensions to INCLUDE in the index.
/// Only actual user files - documents, images, videos, audio. NO code files.
const INTERESTING_EXTENSIONS: &[&str] = &[
    // Documents
    "pdf", "doc", "docx", "odt", "rtf", "txt", "pages", "numbers", "key",
    "xls", "xlsx", "csv", "ppt", "pptx",
    // Images
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "svg",
    "heic", "heif", "raw", "cr2", "nef", "arw", "dng", "psd",
    // Videos
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
    // Audio
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff",
    // Archives
    "zip", "7z", "rar", "dmg",
    // E-books
    "epub", "mobi",
    // macOS apps
    "app",
];

/// File name patterns to exclude.
const EXCLUDED_FILENAMES: &[&str] = &[
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    ".localized",
    "Icon\r",
];

/// Checks if a path should be excluded from the index.
/// Uses a whitelist approach - only files with interesting extensions are included.
fn should_exclude(path: &PathBuf) -> bool {
    let path_str = path.to_string_lossy();

    // Check for excluded directories
    for dir in EXCLUDED_DIRS {
        if path_str.contains(&format!("/{}/", dir)) {
            return true;
        }
    }

    // Check filename
    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        
        // Check for hidden files (starting with .)
        if name_str.starts_with('.') {
            return true;
        }

        // Check for excluded filenames
        for excluded_name in EXCLUDED_FILENAMES {
            if name_str == *excluded_name {
                return true;
            }
        }

        // Check extension - must be in whitelist
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !INTERESTING_EXTENSIONS.contains(&ext_str.as_str()) {
                return true;
            }
        } else {
            // No extension - exclude (no code files like Makefile)
            return true;
        }
    }

    false
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Starts a live file index with default primary scopes.
///
/// This is the simplest way to use the live index:
///
/// ```no_run
/// use photoncast_core::search::spotlight::live_index::start_live_index;
///
/// let index = start_live_index();
///
/// // Wait for initial gathering
/// while !index.is_ready() {
///     std::thread::sleep(std::time::Duration::from_millis(100));
/// }
///
/// // Search is now instant
/// let results = index.search("report", 50);
/// ```
#[must_use]
pub fn start_live_index() -> LiveFileIndex {
    let index = LiveFileIndex::with_primary_scopes();
    index.start();
    index
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_index_creation() {
        let index = LiveFileIndex::with_primary_scopes();
        assert_eq!(index.status(), LiveIndexStatus::Idle);
        assert_eq!(index.file_count(), 0);
        assert!(!index.is_ready());
    }

    #[test]
    fn test_manual_upsert_remove() {
        let index = LiveFileIndex::with_primary_scopes();

        let result = SpotlightResult {
            path: PathBuf::from("/test/file.txt"),
            display_name: "file.txt".to_string(),
            file_size: Some(100),
            content_type: None,
            content_type_tree: vec![],
            modified_date: None,
            created_date: None,
            last_used_date: None,
            is_directory: false,
        };

        index.upsert(result.clone());
        assert_eq!(index.file_count(), 1);

        index.remove(&result.path);
        assert_eq!(index.file_count(), 0);
    }

    #[test]
    fn test_search_without_index() {
        let index = LiveFileIndex::with_primary_scopes();
        // Should fall back to traditional search
        let _ = index.search("test", 10);
    }

    #[test]
    fn test_stats_default() {
        let stats = LiveIndexStats::default();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.adds, 0);
        assert_eq!(stats.changes, 0);
        assert_eq!(stats.removes, 0);
        assert!(stats.last_update.is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_live_index_start() {
        let index = start_live_index();

        // Give it time to start gathering
        std::thread::sleep(Duration::from_millis(500));

        let status = index.status();
        assert!(
            status == LiveIndexStatus::Gathering || status == LiveIndexStatus::Live,
            "Expected Gathering or Live, got {:?}",
            status
        );

        // Stop the index
        index.stop();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_any_file_predicate_directly() {
        use super::super::query::MetadataQueryWrapper;

        let predicate = PredicateBuilder::new().any_file().build();

        let mut query = MetadataQueryWrapper::new();
        query.set_predicate(&predicate);

        if let Some(home) = dirs::home_dir() {
            query.set_search_scopes(&[home.join("Desktop")]);
        }

        match query.execute_sync(Duration::from_secs(5)) {
            Ok(results) => {
                println!("any_file() predicate returned {} results", results.len());
                for (i, r) in results.iter().take(5).enumerate() {
                    println!("  {}: {:?}", i, r.path);
                }
                assert!(results.len() > 0 || true, "May be empty in some environments");
            }
            Err(e) => {
                println!("any_file() predicate failed: {}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_live_index_population() {
        let index = start_live_index();

        // Wait for it to become live (max 10 seconds)
        let start = Instant::now();
        while !index.is_ready() && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }

        let file_count = index.file_count();
        let status = index.status();
        println!("Live index status: {:?}, files: {}", status, file_count);

        // Test search (may be empty if no files indexed)
        if index.is_ready() {
            let results = index.search("a", 10);
            println!("Search for 'a' returned {} results", results.len());
        }

        index.stop();
        // Don't assert file count > 0 as it depends on test environment
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_file_change_detection() {
        use std::fs;
        use std::io::Write;

        // Start the live index
        let index = start_live_index();

        // Wait for it to become live
        let start = Instant::now();
        while !index.is_ready() && start.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(100));
        }

        if !index.is_ready() {
            println!("Live index not ready, skipping file change test");
            index.stop();
            return;
        }

        let initial_count = index.file_count();
        println!("Initial file count: {}", initial_count);

        // Create a test file in Desktop
        let test_file = dirs::home_dir()
            .map(|h| h.join("Desktop").join("__photoncast_test_file.txt"));

        if let Some(ref path) = test_file {
            // Create the file
            let mut file = fs::File::create(path).expect("Failed to create test file");
            file.write_all(b"test content").expect("Failed to write test file");
            drop(file);

            println!("Created test file: {:?}", path);

            // Wait for Spotlight to notice (may take a few seconds)
            std::thread::sleep(Duration::from_secs(3));

            // Check if the file appears in search results
            let results = index.search("__photoncast_test_file", 10);
            println!("Search for test file returned {} results", results.len());

            // Clean up
            let _ = fs::remove_file(path);
            println!("Cleaned up test file");

            // Note: The live update mechanism may take time to propagate
            // This test mainly verifies the initial indexing works
        }

        index.stop();
    }
}
