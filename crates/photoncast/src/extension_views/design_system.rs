//! Design System Enforcement for Extension Views.
//!
//! Provides centralized constraints and helpers to ensure consistent UX
//! across all extension views. Enforces:
//!
//! - Icon size constraints (16x16, 24x24, 32x32)
//! - Thumbnail size limits (list: 64x64, preview: 256x256)
//! - Text truncation rules (single line with ellipsis)
//! - Semantic tag color mapping
//! - Typography definitions (SF Pro Text)
//! - Animation timing constants

use gpui::{px, Hsla, Pixels};
use photoncast_extension_api::TagColor;

use super::ExtensionViewColors;

// ============================================================================
// Icon Size Constraints
// ============================================================================

/// Standard icon sizes supported by the design system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// Small icon (16x16) - inline text, accessories.
    Small,
    /// Medium icon (24x24) - list items, buttons.
    Medium,
    /// Large icon (32x32) - grid items, headers.
    Large,
}

impl IconSize {
    /// Returns the pixel dimension for this icon size.
    #[must_use]
    pub const fn pixels(&self) -> f32 {
        match self {
            Self::Small => 16.0,
            Self::Medium => 24.0,
            Self::Large => 32.0,
        }
    }

    /// Returns the pixel dimension as gpui::Pixels.
    #[must_use]
    pub fn to_px(&self) -> Pixels {
        px(self.pixels())
    }
}

/// Icon size constants in pixels.
pub mod icon_sizes {
    use gpui::{px, Pixels};

    /// Small icon size (16x16).
    pub const SMALL: Pixels = px(16.0);
    /// Medium icon size (24x24).
    pub const MEDIUM: Pixels = px(24.0);
    /// Large icon size (32x32).
    pub const LARGE: Pixels = px(32.0);
}

/// Scales an icon dimension to the nearest valid icon size.
///
/// Valid sizes are: 16x16, 24x24, 32x32.
/// Dimensions smaller than 20 → 16, 20-28 → 24, larger → 32.
///
/// # Arguments
///
/// * `size` - The original icon size in pixels.
///
/// # Returns
///
/// The scaled icon size as gpui::Pixels.
///
/// # Example
///
/// ```ignore
/// let scaled = scale_icon(18.0); // Returns px(16.0)
/// let scaled = scale_icon(25.0); // Returns px(24.0)
/// let scaled = scale_icon(48.0); // Returns px(32.0)
/// ```
#[must_use]
pub fn scale_icon(size: f32) -> Pixels {
    if size < 20.0 {
        px(16.0)
    } else if size < 28.0 {
        px(24.0)
    } else {
        px(32.0)
    }
}

/// Returns the appropriate IconSize enum for a given dimension.
///
/// # Arguments
///
/// * `size` - The original icon size in pixels.
///
/// # Returns
///
/// The appropriate `IconSize` variant.
#[must_use]
pub fn get_icon_size(size: f32) -> IconSize {
    if size < 20.0 {
        IconSize::Small
    } else if size < 28.0 {
        IconSize::Medium
    } else {
        IconSize::Large
    }
}

// ============================================================================
// Thumbnail/Image Size Constraints
// ============================================================================

/// Thumbnail size constraints for different contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailContext {
    /// List item thumbnails (max 64x64).
    List,
    /// Preview pane thumbnails (max 256x256).
    Preview,
    /// Grid item thumbnails (max 120x120).
    Grid,
}

impl ThumbnailContext {
    /// Returns the maximum dimension for this thumbnail context.
    #[must_use]
    pub const fn max_size(&self) -> f32 {
        match self {
            Self::List => 64.0,
            Self::Preview => 256.0,
            Self::Grid => 120.0,
        }
    }

    /// Returns the maximum dimension as gpui::Pixels.
    #[must_use]
    pub fn max_px(&self) -> Pixels {
        px(self.max_size())
    }
}

/// Thumbnail size constants in pixels.
pub mod thumbnail_sizes {
    use gpui::{px, Pixels};

    /// Maximum list thumbnail size (64x64).
    pub const LIST_MAX: Pixels = px(64.0);
    /// Maximum preview thumbnail size (256x256).
    pub const PREVIEW_MAX: Pixels = px(256.0);
    /// Maximum grid thumbnail size (120x120).
    pub const GRID_MAX: Pixels = px(120.0);
}

/// Constrained image dimensions.
#[derive(Debug, Clone, Copy)]
pub struct ConstrainedSize {
    /// Constrained width in pixels.
    pub width: Pixels,
    /// Constrained height in pixels.
    pub height: Pixels,
}

/// Constrains an image size to fit within the maximum dimensions for a context.
///
/// Maintains aspect ratio while ensuring neither dimension exceeds the maximum.
///
/// # Arguments
///
/// * `width` - Original image width in pixels.
/// * `height` - Original image height in pixels.
/// * `context` - The thumbnail context (List, Preview, or Grid).
///
/// # Returns
///
/// A `ConstrainedSize` with dimensions scaled to fit the constraint.
///
/// # Example
///
/// ```ignore
/// let size = constrain_image_size(128.0, 96.0, ThumbnailContext::List);
/// // Returns ConstrainedSize { width: 64.0, height: 48.0 }
/// ```
#[must_use]
pub fn constrain_image_size(width: f32, height: f32, context: ThumbnailContext) -> ConstrainedSize {
    let max_size = context.max_size();

    if width <= max_size && height <= max_size {
        return ConstrainedSize {
            width: px(width),
            height: px(height),
        };
    }

    let scale = if width > height {
        max_size / width
    } else {
        max_size / height
    };

    ConstrainedSize {
        width: px(width * scale),
        height: px(height * scale),
    }
}

/// Constrains an image size to a specific maximum dimension.
///
/// # Arguments
///
/// * `width` - Original image width in pixels.
/// * `height` - Original image height in pixels.
/// * `max_dimension` - Maximum allowed dimension for either width or height.
///
/// # Returns
///
/// A `ConstrainedSize` with dimensions scaled to fit the constraint.
#[must_use]
pub fn constrain_image_to_max(width: f32, height: f32, max_dimension: f32) -> ConstrainedSize {
    if width <= max_dimension && height <= max_dimension {
        return ConstrainedSize {
            width: px(width),
            height: px(height),
        };
    }

    let scale = if width > height {
        max_dimension / width
    } else {
        max_dimension / height
    };

    ConstrainedSize {
        width: px(width * scale),
        height: px(height * scale),
    }
}

// ============================================================================
// Text Truncation
// ============================================================================

/// Text styling options for truncated text.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextStyle {
    /// Whether to truncate with ellipsis.
    pub truncate: bool,
    /// Maximum number of lines (1 for single-line truncation).
    pub max_lines: u32,
    /// Opacity multiplier (1.0 = full, 0.6 = muted).
    pub opacity: f32,
}

impl TextStyle {
    /// Style for titles: single line, truncated, full opacity.
    #[must_use]
    pub const fn title() -> Self {
        Self {
            truncate: true,
            max_lines: 1,
            opacity: 1.0,
        }
    }

    /// Style for subtitles: single line, truncated, muted (60% opacity).
    #[must_use]
    pub const fn subtitle() -> Self {
        Self {
            truncate: true,
            max_lines: 1,
            opacity: 0.6,
        }
    }

    /// Style for accessories: single line, truncated, muted.
    #[must_use]
    pub const fn accessory() -> Self {
        Self {
            truncate: true,
            max_lines: 1,
            opacity: 0.6,
        }
    }

    /// Style for body text: multi-line, not truncated.
    #[must_use]
    pub const fn body() -> Self {
        Self {
            truncate: false,
            max_lines: 0, // No limit
            opacity: 1.0,
        }
    }
}

/// Maximum character limits for different text contexts.
pub mod text_limits {
    /// Maximum characters for titles before truncation.
    pub const TITLE_MAX_CHARS: usize = 100;
    /// Maximum characters for subtitles before truncation.
    pub const SUBTITLE_MAX_CHARS: usize = 150;
    /// Maximum characters for accessory text before truncation.
    pub const ACCESSORY_MAX_CHARS: usize = 50;
}

/// Truncates a string with ellipsis if it exceeds the maximum length.
///
/// # Arguments
///
/// * `text` - The text to truncate.
/// * `max_chars` - Maximum number of characters.
///
/// # Returns
///
/// The truncated string with "..." appended if it was shortened.
///
/// # Example
///
/// ```ignore
/// let short = truncate_with_ellipsis("Hello", 10);  // "Hello"
/// let long = truncate_with_ellipsis("Hello World!", 8);  // "Hello..."
/// ```
#[must_use]
pub fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
}

/// Truncates a title string according to design system rules.
#[must_use]
pub fn truncate_title(text: &str) -> String {
    truncate_with_ellipsis(text, text_limits::TITLE_MAX_CHARS)
}

/// Truncates a subtitle string according to design system rules.
#[must_use]
pub fn truncate_subtitle(text: &str) -> String {
    truncate_with_ellipsis(text, text_limits::SUBTITLE_MAX_CHARS)
}

/// Truncates an accessory string according to design system rules.
#[must_use]
pub fn truncate_accessory(text: &str) -> String {
    truncate_with_ellipsis(text, text_limits::ACCESSORY_MAX_CHARS)
}

// ============================================================================
// Tag Color Mapping
// ============================================================================

/// Maps a semantic `TagColor` to a gpui-compatible HSLA color.
///
/// Uses the theme colors from `ExtensionViewColors` for consistent theming.
///
/// # Arguments
///
/// * `tag_color` - The semantic tag color.
/// * `colors` - The current theme colors.
///
/// # Returns
///
/// The HSLA color value for the tag.
#[must_use]
pub fn tag_color_to_gpui(tag_color: &TagColor, colors: &ExtensionViewColors) -> Hsla {
    colors.tag_color(tag_color)
}

/// Gets the background color for a tag (faded version).
///
/// # Arguments
///
/// * `tag_color` - The semantic tag color.
/// * `colors` - The current theme colors.
///
/// # Returns
///
/// The HSLA background color for the tag (reduced saturation and alpha).
#[must_use]
pub fn tag_background_to_gpui(tag_color: &TagColor, colors: &ExtensionViewColors) -> Hsla {
    colors.tag_background(tag_color)
}

/// Tag styling configuration.
#[derive(Debug, Clone, Copy)]
pub struct TagStyle {
    /// Text color.
    pub text_color: Hsla,
    /// Background color.
    pub background_color: Hsla,
}

/// Gets complete tag styling for a given tag color.
///
/// # Arguments
///
/// * `tag_color` - The semantic tag color.
/// * `colors` - The current theme colors.
///
/// # Returns
///
/// A `TagStyle` with both text and background colors.
#[must_use]
pub fn get_tag_style(tag_color: &TagColor, colors: &ExtensionViewColors) -> TagStyle {
    TagStyle {
        text_color: colors.tag_color(tag_color),
        background_color: colors.tag_background(tag_color),
    }
}

// ============================================================================
// Typography
// ============================================================================

/// Typography definitions following the host font system (SF Pro Text).
pub mod typography {
    use gpui::{px, FontWeight, Pixels};

    /// Font family for the design system.
    /// Uses SF Pro Text on macOS (system font).
    pub const FONT_FAMILY: &str = ".AppleSystemUIFont";

    /// Title typography: 14pt, Medium weight.
    pub mod title {
        use super::*;
        /// Font size for titles.
        pub const SIZE: Pixels = px(14.0);
        /// Font weight for titles.
        pub const WEIGHT: FontWeight = FontWeight::MEDIUM;
        /// Line height for titles.
        pub const LINE_HEIGHT: Pixels = px(20.0);
    }

    /// Subtitle typography: 12pt, Regular weight, 60% opacity.
    pub mod subtitle {
        use super::*;
        /// Font size for subtitles.
        pub const SIZE: Pixels = px(12.0);
        /// Font weight for subtitles.
        pub const WEIGHT: FontWeight = FontWeight::NORMAL;
        /// Line height for subtitles.
        pub const LINE_HEIGHT: Pixels = px(16.0);
        /// Opacity for subtitle text.
        pub const OPACITY: f32 = 0.6;
    }

    /// Accessory typography: 11pt, Regular weight.
    pub mod accessory {
        use super::*;
        /// Font size for accessories.
        pub const SIZE: Pixels = px(11.0);
        /// Font weight for accessories.
        pub const WEIGHT: FontWeight = FontWeight::NORMAL;
        /// Line height for accessories.
        pub const LINE_HEIGHT: Pixels = px(14.0);
    }

    /// Body typography: 13pt, Regular weight.
    pub mod body {
        use super::*;
        /// Font size for body text.
        pub const SIZE: Pixels = px(13.0);
        /// Font weight for body text.
        pub const WEIGHT: FontWeight = FontWeight::NORMAL;
        /// Line height for body text.
        pub const LINE_HEIGHT: Pixels = px(18.0);
    }

    /// Section header typography: 11pt, Semibold weight, uppercase.
    pub mod section_header {
        use super::*;
        /// Font size for section headers.
        pub const SIZE: Pixels = px(11.0);
        /// Font weight for section headers.
        pub const WEIGHT: FontWeight = FontWeight::SEMIBOLD;
        /// Line height for section headers.
        pub const LINE_HEIGHT: Pixels = px(14.0);
    }

    /// Keyboard shortcut typography: 11pt, Medium weight.
    pub mod shortcut {
        use super::*;
        /// Font size for keyboard shortcuts.
        pub const SIZE: Pixels = px(11.0);
        /// Font weight for keyboard shortcuts.
        pub const WEIGHT: FontWeight = FontWeight::MEDIUM;
    }
}

// ============================================================================
// Animation Timing
// ============================================================================

/// Animation timing constants for consistent motion design.
pub mod animation {
    use std::time::Duration;

    /// Duration for list item hover effects.
    pub const HOVER_DURATION: Duration = Duration::from_millis(100);

    /// Duration for selection highlight animation.
    pub const SELECTION_DURATION: Duration = Duration::from_millis(150);

    /// Duration for view transition animations (slide left/right).
    pub const VIEW_TRANSITION_DURATION: Duration = Duration::from_millis(150);

    /// Duration for fade in/out effects.
    pub const FADE_DURATION: Duration = Duration::from_millis(200);

    /// Duration for loading spinner rotation.
    pub const SPINNER_ROTATION_DURATION: Duration = Duration::from_millis(1000);

    /// Duration for toast notification display.
    pub const TOAST_DURATION: Duration = Duration::from_millis(3000);

    /// Duration for tooltip delay before showing.
    pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

    /// Easing functions (represented as cubic bezier control points).
    pub mod easing {
        /// Standard ease-out for most animations.
        /// Cubic bezier: (0.0, 0.0, 0.2, 1.0)
        pub const EASE_OUT: (f32, f32, f32, f32) = (0.0, 0.0, 0.2, 1.0);

        /// Ease-in-out for view transitions.
        /// Cubic bezier: (0.4, 0.0, 0.2, 1.0)
        pub const EASE_IN_OUT: (f32, f32, f32, f32) = (0.4, 0.0, 0.2, 1.0);

        /// Sharp ease for quick interactions.
        /// Cubic bezier: (0.4, 0.0, 0.6, 1.0)
        pub const SHARP: (f32, f32, f32, f32) = (0.4, 0.0, 0.6, 1.0);
    }
}

/// View transition direction for slide animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDirection {
    /// Slide content to the left (pushing new content from right).
    Left,
    /// Slide content to the right (pushing new content from left).
    Right,
}

// ============================================================================
// Spacing and Layout
// ============================================================================

/// Standard spacing values for consistent layout.
pub mod spacing {
    use gpui::{px, Pixels};

    /// Extra-small spacing (4px).
    pub const XS: Pixels = px(4.0);
    /// Small spacing (8px).
    pub const SM: Pixels = px(8.0);
    /// Medium spacing (12px).
    pub const MD: Pixels = px(12.0);
    /// Large spacing (16px).
    pub const LG: Pixels = px(16.0);
    /// Extra-large spacing (24px).
    pub const XL: Pixels = px(24.0);
    /// 2x extra-large spacing (32px).
    pub const XXL: Pixels = px(32.0);
}

/// Standard border radius values.
pub mod border_radius {
    use gpui::{px, Pixels};

    /// Small border radius (4px) - tags, small buttons.
    pub const SM: Pixels = px(4.0);
    /// Medium border radius (8px) - cards, list items.
    pub const MD: Pixels = px(8.0);
    /// Large border radius (12px) - modals, large cards.
    pub const LG: Pixels = px(12.0);
    /// Full border radius for circular elements.
    pub const FULL: Pixels = px(9999.0);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_icon_small() {
        assert_eq!(scale_icon(10.0), px(16.0));
        assert_eq!(scale_icon(16.0), px(16.0));
        assert_eq!(scale_icon(19.0), px(16.0));
    }

    #[test]
    fn test_scale_icon_medium() {
        assert_eq!(scale_icon(20.0), px(24.0));
        assert_eq!(scale_icon(24.0), px(24.0));
        assert_eq!(scale_icon(27.0), px(24.0));
    }

    #[test]
    fn test_scale_icon_large() {
        assert_eq!(scale_icon(28.0), px(32.0));
        assert_eq!(scale_icon(32.0), px(32.0));
        assert_eq!(scale_icon(64.0), px(32.0));
    }

    #[test]
    fn test_constrain_image_within_bounds() {
        let size = constrain_image_size(32.0, 32.0, ThumbnailContext::List);
        assert_eq!(size.width, px(32.0));
        assert_eq!(size.height, px(32.0));
    }

    #[test]
    fn test_constrain_image_exceeds_width() {
        let size = constrain_image_size(128.0, 64.0, ThumbnailContext::List);
        // Scale factor: 64/128 = 0.5
        assert_eq!(size.width, px(64.0));
        assert_eq!(size.height, px(32.0));
    }

    #[test]
    fn test_constrain_image_exceeds_height() {
        let size = constrain_image_size(64.0, 128.0, ThumbnailContext::List);
        // Scale factor: 64/128 = 0.5
        assert_eq!(size.width, px(32.0));
        assert_eq!(size.height, px(64.0));
    }

    #[test]
    fn test_truncate_with_ellipsis_short() {
        assert_eq!(truncate_with_ellipsis("Hello", 10), "Hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_exact() {
        assert_eq!(truncate_with_ellipsis("Hello", 5), "Hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_long() {
        assert_eq!(truncate_with_ellipsis("Hello World!", 8), "Hello...");
    }

    #[test]
    fn test_icon_size_pixels() {
        assert_eq!(IconSize::Small.pixels(), 16.0);
        assert_eq!(IconSize::Medium.pixels(), 24.0);
        assert_eq!(IconSize::Large.pixels(), 32.0);
    }

    #[test]
    fn test_thumbnail_context_max_size() {
        assert_eq!(ThumbnailContext::List.max_size(), 64.0);
        assert_eq!(ThumbnailContext::Preview.max_size(), 256.0);
        assert_eq!(ThumbnailContext::Grid.max_size(), 120.0);
    }

    #[test]
    fn test_text_style_defaults() {
        let title = TextStyle::title();
        assert!(title.truncate);
        assert_eq!(title.max_lines, 1);
        assert_eq!(title.opacity, 1.0);

        let subtitle = TextStyle::subtitle();
        assert!(subtitle.truncate);
        assert_eq!(subtitle.max_lines, 1);
        assert_eq!(subtitle.opacity, 0.6);
    }
}
