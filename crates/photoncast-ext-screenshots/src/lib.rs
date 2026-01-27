#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

//! Screenshot Browser Extension for PhotonCast
//!
//! Browse and manage screenshots from a configurable folder.

use abi_stable::prefix_type::PrefixTypeTrait;
use abi_stable::sabi_trait::prelude::TD_Opaque;
use abi_stable::std_types::{RBox, RDuration, ROption, RResult, RString, RVec};
use chrono::{DateTime, Local};
use image::GenericImageView;
use photoncast_extension_api::prelude::*;
use photoncast_extension_api::{
    CommandHandlerTrait, ExtensionApiResult, ExtensionManifest, ExtensionSearchProvider_TO,
    Extension_TO,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Supported image extensions
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "heic", "tiff", "bmp"];

/// Thumbnail size (max width or height)
const THUMBNAIL_SIZE: u32 = 400;

/// Gets the thumbnail cache directory
fn get_thumbnail_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("PhotonCast")
        .join("thumbnails")
}

/// Generates a cache key for an image based on path and modification time
fn thumbnail_cache_key(path: &PathBuf, modified: SystemTime) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
        duration.as_secs().hash(&mut hasher);
    }
    format!("{:x}.png", hasher.finish())
}

/// Gets or generates a thumbnail for an image.
/// Returns the thumbnail path if successful, or None if generation failed.
///
/// **Warning:** This function performs image I/O (loading + resizing) and should
/// only be called from background threads to avoid blocking the UI.
fn get_or_create_thumbnail(path: &PathBuf, modified: SystemTime) -> Option<PathBuf> {
    let cache_dir = get_thumbnail_cache_dir();
    let cache_key = thumbnail_cache_key(path, modified);
    let thumbnail_path = cache_dir.join(&cache_key);

    // Return cached thumbnail if it exists
    if thumbnail_path.exists() {
        return Some(thumbnail_path);
    }

    // Create cache directory if needed
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }

    // Load and resize the image
    let img = match image::open(path) {
        Ok(img) => img,
        Err(_) => return None,
    };

    // Only create thumbnail if image is larger than thumbnail size
    let (width, height) = img.dimensions();
    if width <= THUMBNAIL_SIZE && height <= THUMBNAIL_SIZE {
        // Image is small enough, use original
        return Some(path.clone());
    }

    // Resize maintaining aspect ratio
    let thumbnail = img.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);

    // Save thumbnail
    if thumbnail.save(&thumbnail_path).is_err() {
        return None;
    }

    Some(thumbnail_path)
}

/// Looks up a cached thumbnail without performing any image I/O.
/// Returns `Some(path)` if a cached thumbnail exists, `None` otherwise.
fn get_cached_thumbnail(path: &Path) -> Option<PathBuf> {
    let thumb_dir = get_thumbnail_cache_dir();
    let file_name = path.file_name()?.to_str()?;
    let thumb_path = thumb_dir.join(format!("thumb_{file_name}"));
    if thumb_path.exists() {
        Some(thumb_path)
    } else {
        None
    }
}

/// Represents a screenshot file
#[derive(Debug, Clone)]
struct Screenshot {
    path: PathBuf,
    name: String,
    extension: String,
    size_bytes: u64,
    modified: SystemTime,
}

impl Screenshot {
    /// Creates a new screenshot from a file path and metadata
    fn new(path: PathBuf, size_bytes: u64, modified: SystemTime) -> Option<Self> {
        let name = path.file_stem()?.to_string_lossy().to_string();
        let extension = path.extension()?.to_string_lossy().to_lowercase();

        if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            return None;
        }

        Some(Self {
            path,
            name,
            extension,
            size_bytes,
            modified,
        })
    }

    /// Formats the file size for display
    fn formatted_size(&self) -> String {
        let kb = self.size_bytes as f64 / 1024.0;
        if kb < 1024.0 {
            format!("{kb:.1} KB")
        } else {
            let mb = kb / 1024.0;
            format!("{mb:.1} MB")
        }
    }

    /// Formats the modified date for display
    fn formatted_date(&self) -> String {
        let datetime: DateTime<Local> = self.modified.into();
        datetime.format("%b %d, %Y %H:%M").to_string()
    }

    /// Gets the duration since the file was modified (age).
    fn modified_duration(&self) -> RDuration {
        let duration = SystemTime::now()
            .duration_since(self.modified)
            .unwrap_or_default();
        RDuration::from_secs(duration.as_secs())
    }

    /// Creates actions for this screenshot
    fn actions(&self) -> RVec<Action> {
        let path_str = self.path.to_string_lossy().to_string();
        let mut actions = RVec::new();

        // Copy to clipboard (primary action)
        // Note: In real implementation, this would copy the image data
        actions.push(Action {
            id: RString::from("copy"),
            title: RString::from("Copy to Clipboard"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("doc.on.doc"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("c")),
            style: ActionStyle::Primary,
            handler: ActionHandler::CopyToClipboard(RString::from(path_str.as_str())),
        });

        // Open in Preview
        actions.push(Action {
            id: RString::from("open"),
            title: RString::from("Open in Preview"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("eye"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("o")),
            style: ActionStyle::Default,
            handler: ActionHandler::OpenFile(RString::from(path_str.as_str())),
        });

        // Reveal in Finder
        let parent_path = self
            .path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        actions.push(Action {
            id: RString::from("reveal"),
            title: RString::from("Reveal in Finder"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("folder"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd_shift("f")),
            style: ActionStyle::Default,
            handler: ActionHandler::OpenFile(RString::from(parent_path)),
        });

        // Quick Look
        actions.push(Action {
            id: RString::from("quicklook"),
            title: RString::from("Quick Look"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("eye.fill"),
            }),
            shortcut: ROption::RSome(Shortcut {
                key: RString::from(" "),
                modifiers: Modifiers {
                    cmd: false,
                    shift: false,
                    alt: false,
                    ctrl: false,
                },
            }),
            style: ActionStyle::Default,
            handler: ActionHandler::OpenFile(RString::from(path_str.as_str())),
        });

        // Delete with confirmation
        actions.push(Action {
            id: RString::from("delete"),
            title: RString::from("Move to Trash"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("trash"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("backspace")),
            style: ActionStyle::Destructive,
            handler: ActionHandler::Callback, // Would trigger delete confirmation
        });

        actions
    }

    /// Creates a list item for this screenshot
    fn to_list_item(&self) -> ListItem {
        let path_str = self.path.to_string_lossy().to_string();

        let mut accessories = RVec::new();

        // File size
        accessories.push(Accessory::Text(RString::from(self.formatted_size())));

        // Modified date
        accessories.push(Accessory::Date(self.modified_duration()));

        // Extension tag
        let color = extension_color(&self.extension);
        accessories.push(Accessory::Tag {
            text: RString::from(self.extension.to_uppercase()),
            color,
        });

        ListItem {
            id: RString::from(path_str.as_str()),
            title: RString::from(self.name.as_str()),
            subtitle: ROption::RSome(RString::from(self.formatted_date())),
            icon: IconSource::FileIcon {
                path: RString::from(path_str.as_str()),
            },
            accessories,
            actions: self.actions(),
            preview: ROption::RSome(self.preview()),
            shortcut: ROption::RNone,
        }
    }

    /// Creates a preview for this screenshot using a cached thumbnail if available.
    /// Falls back to the original image path rather than generating a thumbnail inline.
    fn preview(&self) -> Preview {
        let preview_path = get_cached_thumbnail(&self.path)
            .unwrap_or_else(|| self.path.clone());
        let path_str = preview_path.to_string_lossy().to_string();
        Preview::Image {
            source: RString::from(path_str),
            alt: RString::from(self.name.as_str()),
        }
    }
}

/// Maps file extensions to tag colors
fn extension_color(ext: &str) -> TagColor {
    match ext {
        "png" => TagColor::Blue,
        "jpg" | "jpeg" => TagColor::Green,
        "gif" => TagColor::Purple,
        "webp" => TagColor::Yellow,
        "heic" => TagColor::Orange,
        _ => TagColor::Default,
    }
}

/// Cached result of a directory scan.
struct CachedScan {
    screenshots: Vec<Screenshot>,
    dir_modified: std::time::SystemTime,
}

/// Thread-safe cache for directory scan results.
type ScanCache = Arc<Mutex<Option<CachedScan>>>;

/// Scans a directory for screenshot files
fn scan_screenshots(folder: &str) -> Vec<Screenshot> {
    let path = if folder.starts_with('~') {
        dirs::home_dir()
            .map(|home| home.join(&folder[2..]))
            .unwrap_or_else(|| PathBuf::from(folder))
    } else {
        PathBuf::from(folder)
    };

    let mut screenshots = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            let file_path = entry.path();

            if file_path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    if let Some(screenshot) = Screenshot::new(file_path, metadata.len(), modified) {
                        screenshots.push(screenshot);
                    }
                }
            }
        }
    }

    // Sort by modified date (newest first)
    screenshots.sort_by(|a, b| b.modified.cmp(&a.modified));
    screenshots
}

/// Resolves the folder path (expanding `~`) to an absolute [`PathBuf`].
fn resolve_folder_path(folder: &str) -> PathBuf {
    if folder.starts_with('~') {
        dirs::home_dir()
            .map(|home| home.join(&folder[2..]))
            .unwrap_or_else(|| PathBuf::from(folder))
    } else {
        PathBuf::from(folder)
    }
}

/// Returns screenshots from cache if the directory has not been modified since the last scan.
/// Falls back to a full `scan_screenshots` on cache miss.
fn scan_screenshots_cached(folder: &str, cache: &ScanCache) -> Vec<Screenshot> {
    let path = resolve_folder_path(folder);

    // Check if cache is valid
    let dir_modified = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok();

    if let Some(ref dir_mod) = dir_modified {
        let cached = cache.lock().unwrap();
        if let Some(ref scan) = *cached {
            if scan.dir_modified == *dir_mod {
                return scan.screenshots.clone();
            }
        }
    }

    // Cache miss — perform full scan
    let screenshots = scan_screenshots(folder);

    // Update cache
    if let Some(dir_mod) = dir_modified {
        let mut cached = cache.lock().unwrap();
        *cached = Some(CachedScan {
            screenshots: screenshots.clone(),
            dir_modified: dir_mod,
        });
    }

    screenshots
}

/// Command handler for browsing screenshots
struct BrowseScreenshotsHandler {
    scan_cache: ScanCache,
}

impl CommandHandlerTrait for BrowseScreenshotsHandler {
    fn handle(&self, ctx: ExtensionContext, args: CommandArguments) -> ExtensionApiResult<()> {
        // Get screenshots folder from preferences
        let prefs = ctx.host.get_preferences().unwrap_or(PreferenceValues {
            values: RVec::new(),
        });

        let folder = prefs
            .values
            .iter()
            .find_map(|t| {
                if t.0.as_str() == "screenshots_folder" {
                    if let PreferenceValue::Directory(ref s) = t.1 {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "~/Documents/screenshots".to_string());

        // Scan for screenshots (uses cache when directory is unchanged)
        let screenshots = scan_screenshots_cached(&folder, &self.scan_cache);

        // Filter by query if provided
        let query = args.query.as_ref().map(|s| s.as_str()).unwrap_or("");
        let query_lower = query.to_lowercase();

        let filtered: Vec<&Screenshot> = if query.is_empty() {
            screenshots.iter().collect()
        } else {
            screenshots
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&query_lower))
                .collect()
        };

        // Build list items
        let items: RVec<ListItem> = filtered.iter().map(|s| s.to_list_item()).collect();

        let sections = RVec::from(vec![ListSection {
            title: ROption::RSome(RString::from(format!("Screenshots ({})", filtered.len()))),
            items,
        }]);

        let view = ExtensionView::List(ListView {
            title: RString::from("Browse Screenshots"),
            search_bar: ROption::RSome(SearchBarConfig {
                placeholder: RString::from("Filter screenshots..."),
                throttle_ms: 100,
            }),
            sections,
            empty_state: ROption::RSome(EmptyState {
                icon: ROption::RSome(IconSource::SystemIcon {
                    name: RString::from("photo"),
                }),
                title: RString::from("No screenshots found"),
                description: ROption::RSome(RString::from(format!("No images found in {folder}"))),
                actions: RVec::new(),
            }),
            show_preview: true,
        });

        match ctx.host.render_view(view) {
            RResult::ROk(_) => ExtensionApiResult::ROk(()),
            RResult::RErr(e) => ExtensionApiResult::RErr(e),
        }
    }
}

/// Screenshot Browser Extension
pub struct ScreenshotBrowserExtension {
    ctx: Option<ExtensionContext>,
    scan_cache: ScanCache,
}

impl ScreenshotBrowserExtension {
    fn new() -> Self {
        Self {
            ctx: None,
            scan_cache: Arc::new(Mutex::new(None)),
        }
    }
}

impl Extension for ScreenshotBrowserExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: RString::from("com.photoncast.screenshots"),
            name: RString::from("Screenshot Browser"),
            version: RString::from("1.0.0"),
            description: ROption::RSome(RString::from("Browse and manage screenshots")),
            author: ROption::RSome(RString::from("PhotonCast")),
            license: ROption::RSome(RString::from("MIT")),
            homepage: ROption::RSome(RString::from("https://github.com/photoncast/photoncast")),
            min_photoncast_version: ROption::RNone,
            api_version: 1,
        }
    }

    fn activate(&mut self, ctx: ExtensionContext) -> ExtensionApiResult<()> {
        self.ctx = Some(ctx);
        ExtensionApiResult::ROk(())
    }

    fn deactivate(&mut self) -> ExtensionApiResult<()> {
        self.ctx = None;
        // Clear the scan cache
        if let Ok(mut cached) = self.scan_cache.lock() {
            *cached = None;
        }
        ExtensionApiResult::ROk(())
    }

    fn on_startup(&mut self, ctx: &ExtensionContext) -> ExtensionApiResult<()> {
        // Pre-cache thumbnails for all screenshots in the configured folder
        let prefs = ctx.host.get_preferences().unwrap_or(PreferenceValues {
            values: RVec::new(),
        });

        let folder = prefs
            .values
            .iter()
            .find_map(|t| {
                if t.0.as_str() == "screenshots_folder" {
                    if let PreferenceValue::Directory(ref s) = t.1 {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "~/Documents/screenshots".to_string());

        // Spawn background task to pre-populate scan cache and generate thumbnails
        let scan_cache = Arc::clone(&self.scan_cache);
        std::thread::spawn(move || {
            let screenshots = scan_screenshots(&folder);

            // Pre-populate the scan cache
            let path = resolve_folder_path(&folder);
            if let Ok(dir_mod) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                if let Ok(mut cached) = scan_cache.lock() {
                    *cached = Some(CachedScan {
                        screenshots: screenshots.clone(),
                        dir_modified: dir_mod,
                    });
                }
            }

            // Generate thumbnails in the background
            let mut thumb_count = 0;
            for screenshot in &screenshots {
                if get_or_create_thumbnail(&screenshot.path, screenshot.modified).is_some() {
                    thumb_count += 1;
                }
            }
            if thumb_count > 0 {
                tracing::debug!(
                    count = thumb_count,
                    folder = folder.as_str(),
                    "Pre-cached screenshot thumbnails"
                );
            }
        });

        ExtensionApiResult::ROk(())
    }

    fn search_provider(&self) -> ROption<ExtensionSearchProvider_TO<'static, RBox<()>>> {
        // This extension uses view mode, not search mode
        ROption::RNone
    }

    fn commands(&self) -> RVec<ExtensionCommand> {
        RVec::from(vec![ExtensionCommand {
            id: RString::from("browse"),
            name: RString::from("Browse Screenshots"),
            mode: CommandMode::View,
            keywords: RVec::from(vec![
                RString::from("screenshot"),
                RString::from("image"),
                RString::from("capture"),
                RString::from("snip"),
            ]),
            handler: CommandHandler::new(BrowseScreenshotsHandler {
                scan_cache: Arc::clone(&self.scan_cache),
            }),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("photo.on.rectangle"),
            }),
            subtitle: ROption::RSome(RString::from("Browse and manage screenshots")),
            permissions: RVec::from(vec![
                RString::from("clipboard"),
                RString::from("filesystem"),
            ]),
        }])
    }
}

/// Creates the extension instance (called by PhotonCast)
#[no_mangle]
pub extern "C" fn create_extension() -> ExtensionBox {
    Extension_TO::from_value(ScreenshotBrowserExtension::new(), TD_Opaque)
}

#[abi_stable::export_root_module]
fn instantiate_root_module() -> ExtensionApiRootModule_Ref {
    ExtensionApiRootModule { create_extension }.leak_into_prefix()
}
