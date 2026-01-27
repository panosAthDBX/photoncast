//! Extension View Rendering for GPUI.
//!
//! This module renders `ExtensionView` types from the extension API in GPUI.
//! It provides views for:
//! - `ListView` - Lists with sections, search, and preview
//! - `DetailView` - Markdown content with metadata
//! - `FormView` - Forms with validation
//! - `GridView` - Grid layouts for images and icons
//!
//! Each view type implements GPUI's `Render` trait and can be used as a
//! standalone window or embedded within the launcher.
//!
//! Navigation support is provided via `NavigationContainer` which manages
//! a stack of views with push/pop/replace operations and animations.

// Many design system components are intentionally exported for future use
#![allow(dead_code)]
#![allow(unused_imports)]

mod actions;
mod colors;
mod design_system;
pub mod detail_view;
pub mod form_view;
pub mod grid_view;
pub mod list_view;
mod navigation;
mod preview;

pub use actions::CLOSE_VIEW_ACTION;

pub use colors::ExtensionViewColors;
pub use design_system::{
    animation, border_radius, constrain_image_size, constrain_image_to_max, get_icon_size,
    get_tag_style, icon_sizes, scale_icon, spacing, tag_background_to_gpui, tag_color_to_gpui,
    text_limits, thumbnail_sizes, truncate_accessory, truncate_subtitle, truncate_title,
    truncate_with_ellipsis, typography, ConstrainedSize, IconSize, TagStyle, TextStyle,
    ThumbnailContext, TransitionDirection,
};
pub use detail_view::ExtensionDetailView;
pub use form_view::ExtensionFormView;
pub use grid_view::ExtensionGridView;
pub use list_view::ExtensionListView;
pub use navigation::{
    Navigation, NavigationContainer, NavigationController, NavigationStack,
    register_key_bindings as register_navigation_key_bindings,
};
pub use preview::ExtensionPreviewPane;

use std::sync::Arc;

use abi_stable::std_types::RVec;
use gpui::*;
use photoncast_extension_api::{ExtensionView, ListItem};

/// Callback type for action handlers.
/// Uses Arc to allow cloning for multiple views/navigation.
pub type ActionCallback = Arc<dyn Fn(&str, &mut WindowContext) + Send + Sync + 'static>;

/// Renders an extension view as a GPUI element.
///
/// This is the main entry point for rendering extension views. It takes an
/// `ExtensionView` and returns an appropriate GPUI view.
pub fn render_extension_view(
    view: ExtensionView,
    action_callback: Option<ActionCallback>,
    cx: &mut WindowContext,
) -> AnyView {
    match view {
        ExtensionView::List(list_view) => {
            let view = cx.new_view(|cx| ExtensionListView::new(list_view, action_callback, cx));
            view.into()
        },
        ExtensionView::Detail(detail_view) => {
            let view = cx.new_view(|cx| ExtensionDetailView::new(detail_view, action_callback, cx));
            view.into()
        },
        ExtensionView::Form(form_view) => {
            let view = cx.new_view(|cx| ExtensionFormView::new(form_view, action_callback, cx));
            view.into()
        },
        ExtensionView::Grid(grid_view) => {
            let view = cx.new_view(|cx| ExtensionGridView::new(grid_view, action_callback, cx));
            view.into()
        },
    }
}

/// Updates an existing view with new data.
pub fn update_view_items(view: &AnyView, items: RVec<ListItem>, cx: &mut WindowContext) {
    // Try to downcast to ExtensionListView and update items
    if let Ok(list_view) = view.clone().downcast::<ExtensionListView>() {
        list_view.update(cx, |view, cx| {
            view.update_items(items, cx);
        });
    }
}

/// Common dimensions for extension views.
pub mod dimensions {
    use gpui::Pixels;

    // Re-export shared constant from the canonical location.
    pub use crate::constants::SECTION_HEADER_HEIGHT;

    /// Width of the extension view window.
    pub const VIEW_WIDTH: Pixels = gpui::px(600.0);
    /// Maximum height of the extension view window.
    pub const VIEW_MAX_HEIGHT: Pixels = gpui::px(500.0);
    /// Height of a single list item.
    pub const ITEM_HEIGHT: Pixels = gpui::px(48.0);
    /// Height of a grid item.
    pub const GRID_ITEM_HEIGHT: Pixels = gpui::px(120.0);
    /// Width of the preview pane.
    pub const PREVIEW_WIDTH: Pixels = gpui::px(300.0);
    /// Search bar height (extension views use a compact 44px bar).
    pub const SEARCH_BAR_HEIGHT: Pixels = gpui::px(44.0);
    /// Standard padding.
    pub const PADDING: Pixels = gpui::px(12.0);
    /// Standard border radius.
    pub const BORDER_RADIUS: Pixels = gpui::px(8.0);
}
