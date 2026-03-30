//! Clipboard monitoring using NSPasteboard.
//!
//! This module monitors the system clipboard for changes and
//! creates clipboard items for storage.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::ClipboardConfig;
use crate::error::{ClipboardError, Result};
use crate::models::{detect_color, detect_url, ClipboardContentType, ClipboardItem};
use crate::storage::ClipboardStorage;
use crate::url_metadata::UrlMetadataFetcher;
use crate::THUMBNAIL_SIZE;

/// Events emitted by the clipboard monitor.
#[derive(Debug, Clone)]
pub enum ClipboardEvent {
    /// A new item was added to clipboard history.
    NewItem(ClipboardItem),
    /// An item was skipped (excluded app, transient, etc.).
    Skipped { reason: String },
    /// An error occurred during monitoring.
    Error { message: String },
}

/// Clipboard monitor that watches for clipboard changes.
#[derive(Debug)]
pub struct ClipboardMonitor {
    /// Storage backend.
    storage: ClipboardStorage,
    /// Configuration.
    config: ClipboardConfig,
    /// Whether monitoring is active.
    running: Arc<AtomicBool>,
    /// Last known change count.
    last_change_count: Arc<AtomicI64>,
    /// Event sender (optional).
    event_tx: Option<mpsc::Sender<ClipboardEvent>>,
    /// URL metadata fetcher.
    url_fetcher: Arc<UrlMetadataFetcher>,
}

impl ClipboardMonitor {
    /// Creates a new clipboard monitor.
    pub fn new(storage: ClipboardStorage, config: ClipboardConfig) -> Self {
        Self {
            storage,
            config,
            running: Arc::new(AtomicBool::new(false)),
            last_change_count: Arc::new(AtomicI64::new(0)),
            event_tx: None,
            url_fetcher: Arc::new(UrlMetadataFetcher::new()),
        }
    }

    /// Creates a monitor with an event channel.
    #[must_use]
    pub fn with_events(mut self, tx: mpsc::Sender<ClipboardEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Starts monitoring the clipboard.
    ///
    /// This runs a polling loop that checks for clipboard changes.
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Clipboard monitoring is disabled");
            return Ok(());
        }

        info!("Starting clipboard monitor");
        self.running.store(true, Ordering::SeqCst);

        // Initialize the change count
        #[cfg(target_os = "macos")]
        {
            let change_count = get_pasteboard_change_count();
            self.last_change_count.store(change_count, Ordering::SeqCst);
        }

        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        while self.running.load(Ordering::SeqCst) {
            match self.check_clipboard().await {
                Ok(Some(item)) => {
                    debug!("New clipboard item: {:?}", item.content_type.type_name());
                    if let Some(tx) = &self.event_tx {
                        let _ = tx.send(ClipboardEvent::NewItem(item)).await;
                    }
                },
                Ok(None) => {
                    // No change
                },
                Err(e) if e.is_skip_storage() => {
                    debug!("Clipboard item skipped: {}", e);
                    if let Some(tx) = &self.event_tx {
                        let _ = tx
                            .send(ClipboardEvent::Skipped {
                                reason: e.to_string(),
                            })
                            .await;
                    }
                },
                Err(e) => {
                    warn!("Clipboard check error: {}", e);
                    if let Some(tx) = &self.event_tx {
                        let _ = tx
                            .send(ClipboardEvent::Error {
                                message: e.to_string(),
                            })
                            .await;
                    }
                },
            }

            tokio::time::sleep(poll_interval).await;
        }

        info!("Clipboard monitor stopped");
        Ok(())
    }

    /// Stops monitoring the clipboard.
    pub fn stop(&self) {
        info!("Stopping clipboard monitor");
        self.running.store(false, Ordering::SeqCst);
    }

    /// Returns whether the monitor is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Checks the clipboard for changes.
    async fn check_clipboard(&self) -> Result<Option<ClipboardItem>> {
        #[cfg(target_os = "macos")]
        {
            self.check_clipboard_macos().await
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Stub for non-macOS platforms
            Ok(None)
        }
    }

    /// macOS-specific clipboard check.
    #[cfg(target_os = "macos")]
    async fn check_clipboard_macos(&self) -> Result<Option<ClipboardItem>> {
        let current_count = get_pasteboard_change_count();
        let last_count = self.last_change_count.load(Ordering::SeqCst);

        if current_count == last_count {
            return Ok(None);
        }

        // Update change count
        self.last_change_count
            .store(current_count, Ordering::SeqCst);

        // Check for transient flag
        if is_transient_content() {
            return Err(ClipboardError::TransientItem);
        }

        // Get source app info
        let (source_app, source_bundle_id) = get_source_app_info();

        // Check exclusion list
        if let Some(bundle_id) = &source_bundle_id {
            if self.config.is_excluded(bundle_id) {
                return Err(ClipboardError::ExcludedApp {
                    bundle_id: bundle_id.clone(),
                });
            }
        }

        // Try to read clipboard content
        let content_type = self.read_clipboard_content()?;

        // Create item
        let mut item = ClipboardItem::new(content_type);
        item.source_app = source_app;
        item.source_bundle_id = source_bundle_id;

        // Store the item
        self.storage.store_async(item.clone()).await?;

        // If it's a URL, fetch metadata in background and update stored item
        if let ClipboardContentType::Link { ref url, .. } = item.content_type {
            let fetcher = Arc::clone(&self.url_fetcher);
            let url_clone = url.clone();
            let images_path = self.config.images_path();
            let storage = self.storage.clone();
            let item_id = item.id.clone();

            tokio::spawn(async move {
                match fetcher.fetch(&url_clone, &images_path).await {
                    Ok(metadata) => {
                        debug!("Fetched URL metadata: title={:?}", metadata.title);
                        // Update the stored item with the fetched metadata
                        if let Err(e) = storage
                            .update_url_metadata_async(
                                item_id,
                                metadata.title,
                                metadata.favicon_path,
                            )
                            .await
                        {
                            warn!("Failed to update URL metadata: {}", e);
                        }
                    },
                    Err(e) => {
                        debug!("Failed to fetch URL metadata for {}: {}", url_clone, e);
                    },
                }
            });
        }

        Ok(Some(item))
    }

    /// Reads clipboard content and returns the appropriate content type.
    #[cfg(target_os = "macos")]
    fn read_clipboard_content(&self) -> Result<ClipboardContentType> {
        use std::collections::HashSet;

        use objc2_app_kit::NSPasteboard;

        // Get pasteboard
        let pasteboard = NSPasteboard::generalPasteboard();
        let types = pasteboard.types();

        let Some(types) = types else {
            return Err(ClipboardError::clipboard_access("No types available"));
        };

        // Collect all type strings once into a HashSet for O(1) lookups
        // instead of iterating the NSArray for each has_type call.
        let available_types: HashSet<String> = (0..types.count())
            .map(|i| types.objectAtIndex(i).to_string())
            .collect();

        // Check for image first (PNG, TIFF, JPEG)
        if self.config.store_images && has_type_set(&available_types, "public.png")
            || has_type_set(&available_types, "public.tiff")
            || has_type_set(&available_types, "public.jpeg")
        {
            if let Some(content_type) = self.read_image_content(&pasteboard)? {
                return Ok(content_type);
            }
        }

        // Check for files
        if has_type_set(&available_types, "NSFilenamesPboardType")
            || has_type_set(&available_types, "public.file-url")
        {
            if let Some(content_type) = Self::read_file_content(&pasteboard) {
                return Ok(content_type);
            }
        }

        // Check for rich text (HTML, RTF)
        if has_type_set(&available_types, "public.html")
            || has_type_set(&available_types, "public.rtf")
        {
            if let Some(content_type) = Self::read_rich_text_content(&pasteboard) {
                return Ok(content_type);
            }
        }

        // Check for plain text
        if has_type_set(&available_types, "public.utf8-plain-text")
            || has_type_set(&available_types, "NSStringPboardType")
        {
            if let Some(content_type) = self.read_text_content(&pasteboard) {
                return Ok(content_type);
            }
        }

        Err(ClipboardError::clipboard_access(
            "No supported content type",
        ))
    }

    /// Reads plain text content.
    #[cfg(target_os = "macos")]
    #[allow(clippy::unused_self)]
    fn read_text_content(
        &self,
        pasteboard: &objc2_app_kit::NSPasteboard,
    ) -> Option<ClipboardContentType> {
        use objc2_app_kit::NSPasteboardTypeString;

        let string = unsafe { pasteboard.stringForType(NSPasteboardTypeString) };

        let text = string?.to_string();

        if text.is_empty() {
            return None;
        }

        // Check if it's a color
        if let Some((hex, rgb)) = detect_color(&text) {
            return Some(ClipboardContentType::Color {
                hex,
                rgb,
                display_name: None,
            });
        }

        // Check if it's a URL - just return the content type,
        // URL metadata will be fetched after the item is stored
        if let Some(url) = detect_url(&text) {
            return Some(ClipboardContentType::Link {
                url,
                title: None,
                favicon_path: None,
            });
        }

        Some(ClipboardContentType::text(text))
    }

    /// Reads rich text content.
    #[cfg(target_os = "macos")]
    fn read_rich_text_content(
        pasteboard: &objc2_app_kit::NSPasteboard,
    ) -> Option<ClipboardContentType> {
        use objc2_app_kit::{NSPasteboardTypeHTML, NSPasteboardTypeRTF, NSPasteboardTypeString};

        // Get plain text
        let plain = unsafe { pasteboard.stringForType(NSPasteboardTypeString) }?.to_string();

        if plain.is_empty() {
            return None;
        }

        // Get HTML
        let html = unsafe { pasteboard.stringForType(NSPasteboardTypeHTML) }.map(|s| s.to_string());

        // Get RTF
        let rtf = unsafe { pasteboard.stringForType(NSPasteboardTypeRTF) }.map(|s| s.to_string());

        if html.is_some() || rtf.is_some() {
            Some(ClipboardContentType::RichText { plain, html, rtf })
        } else {
            Some(ClipboardContentType::text(plain))
        }
    }

    /// Reads image content.
    #[cfg(target_os = "macos")]
    fn read_image_content(
        &self,
        pasteboard: &objc2_app_kit::NSPasteboard,
    ) -> Result<Option<ClipboardContentType>> {
        use objc2_app_kit::{NSPasteboardTypePNG, NSPasteboardTypeTIFF};

        // Try to get image data
        let image_data = unsafe {
            pasteboard
                .dataForType(NSPasteboardTypePNG)
                .or_else(|| pasteboard.dataForType(NSPasteboardTypeTIFF))
        };

        let Some(data) = image_data else {
            return Ok(None);
        };

        // Get bytes from NSData
        // Safety: We're not mutating the data while the slice is alive
        let bytes: &[u8] = unsafe { data.as_bytes_unchecked() };

        // Check size limit
        if bytes.len() as u64 > self.config.max_image_size {
            return Err(ClipboardError::TooLarge {
                size: bytes.len() as u64,
                max: self.config.max_image_size,
            });
        }

        // Generate unique filename
        let id = uuid::Uuid::new_v4();
        let image_path = self.config.images_path().join(format!("{}.png", id));
        let thumbnail_path = self
            .config
            .thumbnails_path()
            .join(format!("{}_thumb.png", id));

        // Save image and generate thumbnail
        let img = image::load_from_memory(bytes)
            .map_err(|e| ClipboardError::image(format!("Failed to load image: {}", e)))?;

        let dimensions = (img.width(), img.height());

        // Save full image
        img.save(&image_path)
            .map_err(|e| ClipboardError::image(format!("Failed to save image: {}", e)))?;

        // Generate and save thumbnail
        let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
        thumbnail
            .save(&thumbnail_path)
            .map_err(|e| ClipboardError::image(format!("Failed to save thumbnail: {}", e)))?;

        Ok(Some(ClipboardContentType::Image {
            path: image_path,
            thumbnail_path,
            size_bytes: bytes.len() as u64,
            dimensions,
        }))
    }

    /// Reads file content.
    ///
    /// Note: File reading from clipboard is complex due to objc2 API changes.
    /// This implementation uses a simpler approach via NSPasteboard string methods.
    #[cfg(target_os = "macos")]
    const fn read_file_content(
        _pasteboard: &objc2_app_kit::NSPasteboard,
    ) -> Option<ClipboardContentType> {
        // NOTE: File reading from clipboard is not yet implemented. The objc2 API
        // for readObjectsForClasses:options: is complex and requires careful handling
        // of NSURL types. Files copied in Finder currently appear as text URLs which
        // are detected through the existing text/image detection path.
        None
    }
}

/// Gets the current pasteboard change count.
#[cfg(target_os = "macos")]
fn get_pasteboard_change_count() -> i64 {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.changeCount() as i64
}

/// Checks if the pasteboard content is transient.
#[cfg(target_os = "macos")]
fn is_transient_content() -> bool {
    use objc2_app_kit::NSPasteboard;

    let pasteboard = NSPasteboard::generalPasteboard();
    let types = pasteboard.types();

    if let Some(types) = types {
        // Check for transient type
        let transient_type = "org.nspasteboard.TransientType";
        for i in 0..types.count() {
            let t = types.objectAtIndex(i);
            if t.to_string() == transient_type {
                return true;
            }
        }
    }

    false
}

/// Gets information about the source application.
#[cfg(target_os = "macos")]
fn get_source_app_info() -> (Option<String>, Option<String>) {
    use objc2_app_kit::NSWorkspace;

    // Get frontmost application
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication();

    let Some(app) = app else {
        return (None, None);
    };

    let name = app.localizedName().map(|n| n.to_string());
    let bundle_id = app.bundleIdentifier().map(|b| b.to_string());
    (name, bundle_id)
}

/// Checks if a type array contains a specific type.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn has_type(
    types: &objc2_foundation::NSArray<objc2_foundation::NSString>,
    type_name: &str,
) -> bool {
    for i in 0..types.count() {
        let t = types.objectAtIndex(i);
        if t.to_string().contains(type_name) {
            return true;
        }
    }
    false
}

/// Set-based variant of [`has_type`] for pre-collected type strings.
///
/// Uses substring matching (via `contains`) to stay consistent with
/// the original `has_type` behaviour.
#[cfg(target_os = "macos")]
fn has_type_set(available: &std::collections::HashSet<String>, type_name: &str) -> bool {
    available.iter().any(|t| t.contains(type_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ClipboardConfig {
        ClipboardConfig::default()
    }

    #[test]
    fn test_monitor_creation() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");
        let monitor = ClipboardMonitor::new(storage, config);

        assert!(!monitor.is_running());
    }

    #[tokio::test]
    async fn test_monitor_start_stop() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");
        let monitor = ClipboardMonitor::new(storage, config);

        // Test initial state
        assert!(!monitor.is_running());

        // Test that we can set running state
        monitor
            .running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(monitor.is_running());

        // Test stop
        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[test]
    fn test_event_channel() {
        let config = create_test_config();
        let storage = ClipboardStorage::open_in_memory(&config).expect("should open");
        let (tx, _rx) = mpsc::channel(100);
        let monitor = ClipboardMonitor::new(storage, config).with_events(tx);

        assert!(monitor.event_tx.is_some());
    }
}
