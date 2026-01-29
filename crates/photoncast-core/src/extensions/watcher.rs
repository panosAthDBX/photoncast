//! File watcher for hot-reload support in dev mode.
//!
//! Watches `extension.toml` and `.dylib` files for changes and triggers
//! extension reloads with debouncing to avoid duplicate events.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;

/// Events emitted by the extension watcher.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// An extension's files changed and it should be reloaded.
    ExtensionChanged {
        extension_id: String,
        extension_path: PathBuf,
        changed_file: PathBuf,
    },
    /// The watcher encountered an error.
    Error(String),
}

/// Manages file watching for extensions in dev mode.
pub struct ExtensionWatcher {
    watcher: Option<RecommendedWatcher>,
    watched_paths: Arc<RwLock<HashMap<PathBuf, WatchedExtension>>>,
    event_tx: Sender<WatcherEvent>,
    event_rx: Option<Receiver<WatcherEvent>>,
    debounce_duration: Duration,
    last_events: Arc<RwLock<HashMap<String, Instant>>>,
}

#[derive(Debug, Clone)]
struct WatchedExtension {
    id: String,
    root_path: PathBuf,
}

impl ExtensionWatcher {
    /// Creates a new extension watcher with the specified debounce duration.
    ///
    /// # Arguments
    ///
    /// * `debounce_duration` - Minimum time between reload events for the same extension.
    #[must_use]
    pub fn new(debounce_duration: Duration) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            watcher: None,
            watched_paths: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Some(event_rx),
            debounce_duration,
            last_events: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Takes the event receiver. Can only be called once.
    pub fn take_event_receiver(&mut self) -> Option<Receiver<WatcherEvent>> {
        self.event_rx.take()
    }

    /// Starts the file watcher.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be initialized.
    pub fn start(&mut self) -> Result<(), WatcherError> {
        if self.watcher.is_some() {
            return Ok(());
        }

        let watched_paths = Arc::clone(&self.watched_paths);
        let event_tx = self.event_tx.clone();
        let debounce_duration = self.debounce_duration;
        let last_events = Arc::clone(&self.last_events);

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if !is_relevant_event(&event.kind) {
                        return;
                    }

                    for path in &event.paths {
                        if let Some(ext_info) = find_extension_for_path(path, &watched_paths) {
                            // Check if this file is relevant (extension.toml or .dylib)
                            if !is_watched_file(path) {
                                continue;
                            }

                            // Debounce check
                            let mut last = last_events.write();
                            let now = Instant::now();
                            if let Some(last_time) = last.get(&ext_info.id) {
                                if now.duration_since(*last_time) < debounce_duration {
                                    tracing::trace!(
                                        extension_id = %ext_info.id,
                                        "Debouncing reload event"
                                    );
                                    continue;
                                }
                            }
                            last.insert(ext_info.id.clone(), now);
                            drop(last);

                            tracing::debug!(
                                extension_id = %ext_info.id,
                                path = %path.display(),
                                "Extension file changed"
                            );

                            let _ = event_tx.send(WatcherEvent::ExtensionChanged {
                                extension_id: ext_info.id,
                                extension_path: ext_info.root_path,
                                changed_file: path.clone(),
                            });
                        }
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "File watcher error");
                    let _ = event_tx.send(WatcherEvent::Error(e.to_string()));
                },
            }
        })
        .map_err(|e| WatcherError::Init(e.to_string()))?;

        self.watcher = Some(watcher);
        tracing::info!("Extension file watcher started");
        Ok(())
    }

    /// Stops the file watcher.
    pub fn stop(&mut self) {
        if self.watcher.take().is_some() {
            tracing::info!("Extension file watcher stopped");
        }
    }

    /// Watches an extension directory for changes.
    ///
    /// # Arguments
    ///
    /// * `extension_id` - The unique identifier of the extension.
    /// * `path` - The root directory of the extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be watched.
    pub fn watch_extension(&mut self, extension_id: &str, path: &Path) -> Result<(), WatcherError> {
        let watcher = self.watcher.as_mut().ok_or(WatcherError::NotStarted)?;

        // Add to our tracking map
        self.watched_paths.write().insert(
            path.to_path_buf(),
            WatchedExtension {
                id: extension_id.to_string(),
                root_path: path.to_path_buf(),
            },
        );

        // Watch the directory recursively
        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| WatcherError::Watch {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;

        tracing::debug!(
            extension_id = extension_id,
            path = %path.display(),
            "Now watching extension for changes"
        );

        Ok(())
    }

    /// Stops watching an extension directory.
    ///
    /// # Arguments
    ///
    /// * `path` - The root directory of the extension.
    pub fn unwatch_extension(&mut self, path: &Path) {
        if let Some(watcher) = self.watcher.as_mut() {
            let _ = watcher.unwatch(path);
        }
        self.watched_paths.write().remove(path);
        tracing::debug!(path = %path.display(), "Stopped watching extension");
    }

    /// Returns whether the watcher is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }

    /// Returns the number of extensions being watched.
    #[must_use]
    pub fn watched_count(&self) -> usize {
        self.watched_paths.read().len()
    }
}

impl Default for ExtensionWatcher {
    fn default() -> Self {
        Self::new(Duration::from_millis(200))
    }
}

/// Errors that can occur during file watching.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::trivially_copy_pass_by_ref)]
pub enum WatcherError {
    #[error("failed to initialize watcher: {0}")]
    Init(String),
    #[error("watcher not started")]
    NotStarted,
    #[error("failed to watch path {path}: {reason}")]
    Watch { path: PathBuf, reason: String },
}

/// Checks if the event kind is relevant for triggering a reload.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

/// Checks if the file is one we care about watching.
fn is_watched_file(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Watch extension.toml manifest
    if file_name == "extension.toml" {
        return true;
    }

    // Watch dynamic libraries
    if extension == "dylib" || extension == "so" || extension == "dll" {
        return true;
    }

    false
}

/// Finds the extension info for a given file path.
fn find_extension_for_path(
    path: &Path,
    watched_paths: &Arc<RwLock<HashMap<PathBuf, WatchedExtension>>>,
) -> Option<WatchedExtension> {
    let watched = watched_paths.read();
    for (ext_path, ext_info) in watched.iter() {
        if path.starts_with(ext_path) {
            return Some(ext_info.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_watched_file() {
        assert!(is_watched_file(Path::new("/path/to/extension.toml")));
        assert!(is_watched_file(Path::new("/path/to/libext.dylib")));
        assert!(is_watched_file(Path::new("/path/to/libext.so")));
        assert!(is_watched_file(Path::new("/path/to/ext.dll")));
        assert!(!is_watched_file(Path::new("/path/to/readme.md")));
        assert!(!is_watched_file(Path::new("/path/to/main.rs")));
    }

    #[test]
    fn test_watcher_lifecycle() {
        let mut watcher = ExtensionWatcher::new(Duration::from_millis(100));
        assert!(!watcher.is_running());
        assert_eq!(watcher.watched_count(), 0);

        // Can take receiver only once
        assert!(watcher.take_event_receiver().is_some());
        assert!(watcher.take_event_receiver().is_none());
    }
}
