//! GPUI views and components.
//!
//! This module contains all the UI components for the PhotonCast launcher.
//!
//! # Components
//!
//! - [`LauncherWindow`] - Main launcher window
//! - [`SearchBar`] - Search input component
//! - [`ResultsList`] - Results display with virtual scrolling
//! - [`ResultItem`] - Individual result row
//! - [`ResultGroup`] - Section grouping component
//!
//! # State Components
//!
//! - [`EmptyState`] - No query or no results display
//! - [`LoadingState`] - Indexing/loading progress display
//! - [`ErrorState`] - Error display with recovery actions
//! - [`LauncherState`] - Central state enum for the launcher
//!
//! # Animation System
//!
//! - [`animations`] - Animation constants, helpers, and reduce motion support

pub mod animations;
pub mod empty_state;
pub mod launcher;
pub mod permission_dialog;
pub mod result_group;
pub mod result_item;
pub mod results_list;
pub mod search_bar;

// Core components
pub use launcher::LauncherWindow;
pub use permission_dialog::PermissionDialog;
pub use result_group::{
    ResultGroup, ResultGroupWithItems, GROUP_HEADER_HEIGHT, GROUP_HEADER_PADDING_X,
};
pub use result_item::{
    ResultItem, RESULT_ICON_SIZE, RESULT_ITEM_HEIGHT, RESULT_PADDING_X, RESULT_PADDING_Y,
};
pub use results_list::{ResultsList, RESULTS_MAX_HEIGHT};
pub use search_bar::{
    SearchBar, DEBOUNCE_DURATION, SEARCH_BAR_HEIGHT, SEARCH_BAR_PADDING_X, SEARCH_ICON_SIZE,
    SEARCH_INPUT_FONT_SIZE,
};

// State components
pub use empty_state::{
    AppError, EmptyState, ErrorAction, ErrorActionType, ErrorCode, ErrorState, KeyboardHint,
    LauncherState, LoadingState,
};

// Animation system
pub use animations::{
    animation_duration, ease_in, ease_in_out, ease_out, get_reduce_motion_override,
    hover_transition_duration, lerp, lerp_color, linear, reduce_motion_enabled,
    selection_change_duration, set_reduce_motion_override, window_appear_duration,
    window_dismiss_duration, ItemAnimationState, WindowAnimationState, HOVER_TRANSITION_MS,
    SELECTION_CHANGE_MS, WINDOW_APPEAR_MS, WINDOW_APPEAR_OPACITY_END, WINDOW_APPEAR_OPACITY_START,
    WINDOW_APPEAR_SCALE_END, WINDOW_APPEAR_SCALE_START, WINDOW_DISMISS_MS,
    WINDOW_DISMISS_SCALE_END,
};
