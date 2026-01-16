//! Filesystem change detection for application bundles.
//!
//! This module provides real-time monitoring of application directories,
//! detecting when apps are installed, modified, or removed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::indexer::scanner::SCAN_PATHS;

/// Default debounce duration for coalescing filesystem events.
const DEFAULT_DEBOUNCE_MS: u64 = 500;

/// Events that the watcher emits when applications change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A new application was installed.
    AppAdded(PathBuf),
    /// An existing application was modified.
    AppModified(PathBuf),
    /// An application was removed.
    AppRemoved(PathBuf),
}

impl WatchEvent {
    /// Returns the path of the affected application.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::AppAdded(p) | Self::AppModified(p) | Self::AppRemoved(p) => p,
        }
    }
}

/// Configuration for the application watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration in milliseconds.
    pub debounce_ms: u64,
    /// Paths to watch for changes.
    pub watch_paths: Vec<PathBuf>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        let watch_paths = SCAN_PATHS
            .iter()
            .map(|p| {
                if p.starts_with('~') {
                    dirs::home_dir()
                        .map(|h| h.join(&p[2..]))
                        .unwrap_or_else(|| PathBuf::from(p))
                } else {
                    PathBuf::from(p)
                }
            })
            .collect();

        Self {
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            watch_paths,
        }
    }
}

/// Pending event for debouncing.
#[derive(Debug)]
struct PendingEvent {
    event_type: PendingEventType,
    #[allow(dead_code)]
    first_seen: std::time::Instant,
}

#[derive(Debug, Clone, Copy)]
enum PendingEventType {
    Added,
    Modified,
    Removed,
}

/// Internal state for debouncing events.
#[derive(Debug, Default)]
struct DebounceState {
    /// Pending events keyed by path.
    pending: HashMap<PathBuf, PendingEvent>,
}

/// Watches application directories for changes and emits debounced events.
pub struct AppWatcher {
    config: WatcherConfig,
    watcher: Option<RecommendedWatcher>,
    event_tx: Option<mpsc::UnboundedSender<WatchEvent>>,
    debounce_state: Arc<Mutex<DebounceState>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AppWatcher {
    /// Creates a new application watcher with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(WatcherConfig::default())
    }

    /// Creates a new application watcher with custom configuration.
    #[must_use]
    pub fn with_config(config: WatcherConfig) -> Self {
        Self {
            config,
            watcher: None,
            event_tx: None,
            debounce_state: Arc::new(Mutex::new(DebounceState::default())),
            shutdown_tx: None,
        }
    }

    /// Starts watching directories and returns a receiver for watch events.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be created or directories cannot be watched.
    pub fn start(&mut self) -> Result<mpsc::UnboundedReceiver<WatchEvent>> {
        // Create channels
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Event>();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

        // Create the notify watcher
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if raw_tx.send(event).is_err() {
                        // Receiver dropped, stop sending
                    }
                },
                Err(e) => {
                    error!("File watcher error: {}", e);
                },
            }
        })
        .context("failed to create file watcher")?;

        self.watcher = Some(watcher);
        self.event_tx = Some(event_tx.clone());
        self.shutdown_tx = Some(shutdown_tx);

        // Watch each configured path
        self.setup_watches()?;

        // Spawn debounce task
        let debounce_state = Arc::clone(&self.debounce_state);
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);

        tokio::spawn(async move {
            let mut debounce_timer: Option<tokio::time::Instant> = None;

            loop {
                tokio::select! {
                    biased;

                    // Check for shutdown
                    _ = &mut shutdown_rx => {
                        debug!("Watcher received shutdown signal");
                        break;
                    }

                    // Process incoming raw events
                    Some(event) = raw_rx.recv() => {
                        if let Some(watch_event) = process_raw_event(&event) {
                            let path = watch_event.path().to_path_buf();
                            let event_type = match &watch_event {
                                WatchEvent::AppAdded(_) => PendingEventType::Added,
                                WatchEvent::AppModified(_) => PendingEventType::Modified,
                                WatchEvent::AppRemoved(_) => PendingEventType::Removed,
                            };

                            let mut state = debounce_state.lock();
                            state.pending.insert(path, PendingEvent {
                                event_type,
                                first_seen: std::time::Instant::now(),
                            });

                            // Reset debounce timer
                            debounce_timer = Some(tokio::time::Instant::now() + debounce_duration);
                        }
                    }

                    // Handle debounce timeout
                    _ = async {
                        if let Some(deadline) = debounce_timer {
                            tokio::time::sleep_until(deadline).await;
                        } else {
                            // No timer set, wait forever (until next event)
                            std::future::pending::<()>().await;
                        }
                    } => {
                        // Flush pending events
                        let events_to_emit: Vec<_> = {
                            let mut state = debounce_state.lock();
                            state.pending.drain().map(|(path, pending)| {
                                match pending.event_type {
                                    PendingEventType::Added => WatchEvent::AppAdded(path),
                                    PendingEventType::Modified => WatchEvent::AppModified(path),
                                    PendingEventType::Removed => WatchEvent::AppRemoved(path),
                                }
                            }).collect()
                        };

                        for event in events_to_emit {
                            debug!("Emitting watch event: {:?}", event);
                            if event_tx.send(event).is_err() {
                                // Receiver dropped
                                break;
                            }
                        }

                        debounce_timer = None;
                    }
                }
            }
        });

        info!("Application watcher started");
        Ok(event_rx)
    }

    /// Sets up watches on all configured paths.
    fn setup_watches(&mut self) -> Result<()> {
        let watcher = self.watcher.as_mut().context("watcher not initialized")?;

        for path in &self.config.watch_paths {
            if path.exists() {
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .with_context(|| format!("failed to watch directory: {}", path.display()))?;
                info!("Watching directory: {}", path.display());
            } else {
                debug!("Skipping non-existent directory: {}", path.display());
            }
        }

        Ok(())
    }

    /// Stops watching all directories.
    pub fn stop(&mut self) {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Drop the watcher
        self.watcher = None;
        self.event_tx = None;

        info!("Application watcher stopped");
    }

    /// Returns true if the watcher is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }
}

impl Default for AppWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AppWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Processes a raw notify event and returns a `WatchEvent` if applicable.
fn process_raw_event(event: &Event) -> Option<WatchEvent> {
    // We only care about events on .app bundles
    let path = event.paths.first()?;

    // Filter to .app bundles only
    if !is_app_bundle(path) {
        return None;
    }

    // Determine event type
    match &event.kind {
        EventKind::Create(_) => {
            debug!("Detected app creation: {}", path.display());
            Some(WatchEvent::AppAdded(path.clone()))
        },
        EventKind::Modify(_) => {
            debug!("Detected app modification: {}", path.display());
            Some(WatchEvent::AppModified(path.clone()))
        },
        EventKind::Remove(_) => {
            debug!("Detected app removal: {}", path.display());
            Some(WatchEvent::AppRemoved(path.clone()))
        },
        _ => None,
    }
}

/// Checks if a path is an .app bundle.
fn is_app_bundle(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
}

// ============================================================================
// Legacy FsWatcher (for backwards compatibility)
// ============================================================================

/// Simple filesystem watcher without debouncing.
///
/// **Deprecated:** Use [`AppWatcher`] instead for production use.
pub struct FsWatcher {
    watcher: Option<RecommendedWatcher>,
    receiver: Option<std::sync::mpsc::Receiver<Result<Event, notify::Error>>>,
}

impl FsWatcher {
    /// Creates a new filesystem watcher.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            watcher: None,
            receiver: None,
        }
    }

    /// Starts watching the specified directories.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be created or directories cannot be watched.
    pub fn start(&mut self, paths: &[&Path]) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        let watcher = notify::recommended_watcher(move |res| {
            tx.send(res).ok();
        })
        .context("failed to create file watcher")?;

        self.watcher = Some(watcher);
        self.receiver = Some(rx);

        // Watch each path
        if let Some(ref mut watcher) = self.watcher {
            for path in paths {
                if path.exists() {
                    watcher
                        .watch(path, RecursiveMode::NonRecursive)
                        .with_context(|| {
                            format!("failed to watch directory: {}", path.display())
                        })?;
                }
            }
        }

        Ok(())
    }

    /// Stops watching all directories.
    pub fn stop(&mut self) {
        self.watcher = None;
        self.receiver = None;
    }

    /// Returns the next event, blocking for up to the specified duration.
    pub fn next_event(&self, timeout: Duration) -> Option<Event> {
        self.receiver
            .as_ref()
            .and_then(|rx| rx.recv_timeout(timeout).ok())
            .and_then(Result::ok)
    }

    /// Returns true if the watcher is running.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.watcher.is_some()
    }
}

impl Default for FsWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_app_bundle() {
        assert!(is_app_bundle(Path::new("/Applications/Safari.app")));
        assert!(is_app_bundle(Path::new("/Applications/Firefox.APP")));
        assert!(is_app_bundle(Path::new("~/Applications/MyApp.app")));
        assert!(!is_app_bundle(Path::new("/Applications/Safari")));
        assert!(!is_app_bundle(Path::new("/Applications/file.txt")));
        assert!(!is_app_bundle(Path::new("/Applications/folder")));
    }

    #[test]
    fn test_watch_event_path() {
        let path = PathBuf::from("/Applications/Test.app");

        let added = WatchEvent::AppAdded(path.clone());
        assert_eq!(added.path(), path);

        let modified = WatchEvent::AppModified(path.clone());
        assert_eq!(modified.path(), path);

        let removed = WatchEvent::AppRemoved(path.clone());
        assert_eq!(removed.path(), path);
    }

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, DEFAULT_DEBOUNCE_MS);
        assert!(!config.watch_paths.is_empty());
    }

    #[test]
    fn test_app_watcher_creation() {
        let watcher = AppWatcher::new();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_app_watcher_with_custom_config() {
        let config = WatcherConfig {
            debounce_ms: 1000,
            watch_paths: vec![PathBuf::from("/tmp")],
        };
        let watcher = AppWatcher::with_config(config);
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_legacy_fs_watcher_creation() {
        let watcher = FsWatcher::new();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_process_raw_event_filters_non_apps() {
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::Any),
            paths: vec![PathBuf::from("/Applications/file.txt")],
            attrs: Default::default(),
        };
        assert!(process_raw_event(&event).is_none());
    }

    #[test]
    fn test_process_raw_event_detects_app_creation() {
        let path = PathBuf::from("/Applications/NewApp.app");
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let result = process_raw_event(&event);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), WatchEvent::AppAdded(path));
    }

    #[test]
    fn test_process_raw_event_detects_app_modification() {
        let path = PathBuf::from("/Applications/ExistingApp.app");
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let result = process_raw_event(&event);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), WatchEvent::AppModified(path));
    }

    #[test]
    fn test_process_raw_event_detects_app_removal() {
        let path = PathBuf::from("/Applications/DeletedApp.app");
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::Any),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let result = process_raw_event(&event);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), WatchEvent::AppRemoved(path));
    }

    #[test]
    fn test_process_raw_event_ignores_other_events() {
        let path = PathBuf::from("/Applications/Test.app");
        let event = Event {
            kind: EventKind::Access(notify::event::AccessKind::Any),
            paths: vec![path],
            attrs: Default::default(),
        };

        assert!(process_raw_event(&event).is_none());
    }

    #[test]
    fn test_watch_event_equality() {
        let path = PathBuf::from("/Applications/Test.app");

        assert_eq!(
            WatchEvent::AppAdded(path.clone()),
            WatchEvent::AppAdded(path.clone())
        );

        assert_ne!(
            WatchEvent::AppAdded(path.clone()),
            WatchEvent::AppModified(path.clone())
        );
    }
}
