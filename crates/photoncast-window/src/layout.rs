//! Window layout definitions and calculations.

use core_graphics::display::CGRect;
use serde::{Deserialize, Serialize};

/// Window layout presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowLayout {
    // Halves
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,

    // Quarters
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,

    // Thirds
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,

    // Special
    Maximize,
    Center,
    Restore,

    // New layouts
    /// Fill screen with configurable margin on all sides.
    AlmostMaximize,
    /// Center window at 50% screen width, full height.
    CenterHalf,
    /// Center window at 66% screen width, full height.
    CenterTwoThirds,
    /// Reasonable size: 75% width, 80% height, min 800x600.
    ReasonableSize,
    /// Shrink window by 10% from center.
    MakeSmaller,
    /// Grow window by 10% from center.
    MakeLarger,
    /// Toggle fullscreen mode.
    ToggleFullscreen,
}

impl WindowLayout {
    /// Returns the display name for this layout.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::LeftHalf => "Left Half",
            Self::RightHalf => "Right Half",
            Self::TopHalf => "Top Half",
            Self::BottomHalf => "Bottom Half",
            Self::TopLeft => "Top Left Quarter",
            Self::TopRight => "Top Right Quarter",
            Self::BottomLeft => "Bottom Left Quarter",
            Self::BottomRight => "Bottom Right Quarter",
            Self::FirstThird => "First Third",
            Self::CenterThird => "Center Third",
            Self::LastThird => "Last Third",
            Self::FirstTwoThirds => "First Two Thirds",
            Self::LastTwoThirds => "Last Two Thirds",
            Self::Maximize => "Maximize",
            Self::Center => "Center",
            Self::Restore => "Restore",
            Self::AlmostMaximize => "Almost Maximize",
            Self::CenterHalf => "Center Half",
            Self::CenterTwoThirds => "Center Two Thirds",
            Self::ReasonableSize => "Reasonable Size",
            Self::MakeSmaller => "Make Smaller",
            Self::MakeLarger => "Make Larger",
            Self::ToggleFullscreen => "Toggle Fullscreen",
        }
    }

    /// Returns the command ID for this layout.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::LeftHalf => "window_left_half",
            Self::RightHalf => "window_right_half",
            Self::TopHalf => "window_top_half",
            Self::BottomHalf => "window_bottom_half",
            Self::TopLeft => "window_top_left",
            Self::TopRight => "window_top_right",
            Self::BottomLeft => "window_bottom_left",
            Self::BottomRight => "window_bottom_right",
            Self::FirstThird => "window_first_third",
            Self::CenterThird => "window_center_third",
            Self::LastThird => "window_last_third",
            Self::FirstTwoThirds => "window_first_two_thirds",
            Self::LastTwoThirds => "window_last_two_thirds",
            Self::Maximize => "window_maximize",
            Self::Center => "window_center",
            Self::Restore => "window_restore",
            Self::AlmostMaximize => "window_almost_maximize",
            Self::CenterHalf => "window_center_half",
            Self::CenterTwoThirds => "window_center_two_thirds",
            Self::ReasonableSize => "window_reasonable_size",
            Self::MakeSmaller => "window_make_smaller",
            Self::MakeLarger => "window_make_larger",
            Self::ToggleFullscreen => "window_toggle_fullscreen",
        }
    }

    /// Returns all available layouts.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::LeftHalf,
            Self::RightHalf,
            Self::TopHalf,
            Self::BottomHalf,
            Self::TopLeft,
            Self::TopRight,
            Self::BottomLeft,
            Self::BottomRight,
            Self::FirstThird,
            Self::CenterThird,
            Self::LastThird,
            Self::FirstTwoThirds,
            Self::LastTwoThirds,
            Self::Maximize,
            Self::Center,
            Self::Restore,
            Self::AlmostMaximize,
            Self::CenterHalf,
            Self::CenterTwoThirds,
            Self::ReasonableSize,
            Self::MakeSmaller,
            Self::MakeLarger,
            Self::ToggleFullscreen,
        ]
    }

    /// Parses a layout from a command id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "window_left_half" => Some(Self::LeftHalf),
            "window_right_half" => Some(Self::RightHalf),
            "window_top_half" => Some(Self::TopHalf),
            "window_bottom_half" => Some(Self::BottomHalf),
            "window_top_left" => Some(Self::TopLeft),
            "window_top_right" => Some(Self::TopRight),
            "window_bottom_left" => Some(Self::BottomLeft),
            "window_bottom_right" => Some(Self::BottomRight),
            "window_first_third" => Some(Self::FirstThird),
            "window_center_third" => Some(Self::CenterThird),
            "window_last_third" => Some(Self::LastThird),
            "window_first_two_thirds" => Some(Self::FirstTwoThirds),
            "window_last_two_thirds" => Some(Self::LastTwoThirds),
            "window_maximize" => Some(Self::Maximize),
            "window_center" => Some(Self::Center),
            "window_restore" => Some(Self::Restore),
            "window_almost_maximize" => Some(Self::AlmostMaximize),
            "window_center_half" => Some(Self::CenterHalf),
            "window_center_two_thirds" => Some(Self::CenterTwoThirds),
            "window_reasonable_size" => Some(Self::ReasonableSize),
            "window_make_smaller" => Some(Self::MakeSmaller),
            "window_make_larger" => Some(Self::MakeLarger),
            "window_toggle_fullscreen" => Some(Self::ToggleFullscreen),
            _ => None,
        }
    }
}

/// Cycling state for a window layout.
///
/// When a layout is applied repeatedly, it cycles through different sizes.
/// For example, Left Half cycles: 50% → 33% → 66% → 50% ...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleState {
    /// Initial state (50% for halves, 33% for thirds).
    Initial,
    /// First cycle (33% for halves).
    FirstCycle,
    /// Second cycle (66% for halves).
    SecondCycle,
}

impl CycleState {
    /// Advances to the next cycle state.
    #[must_use]
    pub const fn next(&self) -> Self {
        match self {
            Self::Initial => Self::FirstCycle,
            Self::FirstCycle => Self::SecondCycle,
            Self::SecondCycle => Self::Initial,
        }
    }
}

/// Layout calculator for computing window frames.
#[derive(Debug)]
pub struct LayoutCalculator {
    /// Menu bar height (typically 24 points on macOS).
    menu_bar_height: f64,
    /// Dock size and position (cached).
    dock_bounds: Option<CGRect>,
    /// Gap between windows and screen edges.
    window_gap: f64,
    /// Whether to account for menu bar.
    respect_menu_bar: bool,
    /// Whether to account for dock.
    respect_dock: bool,
    /// Margin for almost maximize layout.
    almost_maximize_margin: f64,
}

impl LayoutCalculator {
    /// Creates a new layout calculator with default values.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            menu_bar_height: 24.0, // Standard macOS menu bar
            dock_bounds: None,
            window_gap: 0.0,
            respect_menu_bar: true,
            respect_dock: true,
            almost_maximize_margin: 20.0,
        }
    }

    /// Creates a new layout calculator with the given configuration.
    #[must_use]
    pub fn with_config(
        window_gap: u32,
        respect_menu_bar: bool,
        respect_dock: bool,
        almost_maximize_margin: u32,
    ) -> Self {
        Self {
            menu_bar_height: 24.0,
            dock_bounds: None,
            window_gap: f64::from(window_gap),
            respect_menu_bar,
            respect_dock,
            almost_maximize_margin: f64::from(almost_maximize_margin),
        }
    }

    /// Updates configuration values.
    pub fn update_config(
        &mut self,
        window_gap: u32,
        respect_menu_bar: bool,
        respect_dock: bool,
        almost_maximize_margin: u32,
    ) {
        self.window_gap = f64::from(window_gap);
        self.respect_menu_bar = respect_menu_bar;
        self.respect_dock = respect_dock;
        self.almost_maximize_margin = f64::from(almost_maximize_margin);
    }

    /// Updates the dock bounds cache.
    pub fn update_dock_bounds(&mut self, bounds: Option<CGRect>) {
        self.dock_bounds = bounds;
    }

    // Helper: Create frame aligned to the left edge
    fn frame_left(x: f64, y: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { x, y },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Create frame aligned to the right edge  
    fn frame_right(x: f64, y: f64, total_width: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { x: x + total_width - w, y },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Create frame aligned to the bottom edge
    fn frame_bottom(x: f64, y: f64, total_height: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { x, y: y + total_height - h },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Create frame at bottom-right corner
    fn frame_bottom_right(x: f64, y: f64, total_width: f64, total_height: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { 
                x: x + total_width - w, 
                y: y + total_height - h 
            },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Create centered frame
    fn frame_centered(x: f64, y: f64, total_width: f64, total_height: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { 
                x: x + (total_width - w) / 2.0, 
                y: y + (total_height - h) / 2.0 
            },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Create horizontally centered frame (full height)
    fn frame_centered_horizontal(x: f64, y: f64, total_width: f64, w: f64, h: f64) -> CGRect {
        CGRect::new(
            &core_graphics::geometry::CGPoint { 
                x: x + (total_width - w) / 2.0, 
                y 
            },
            &core_graphics::geometry::CGSize { width: w, height: h },
        )
    }

    // Helper: Get width based on cycle state (half -> third -> two-thirds)
    fn cycle_width(total_width: f64, cycle_state: CycleState) -> f64 {
        match cycle_state {
            CycleState::Initial => total_width / 2.0,
            CycleState::FirstCycle => total_width / 3.0,
            CycleState::SecondCycle => total_width * 2.0 / 3.0,
        }
    }

    // Helper: Get height based on cycle state (half -> third -> two-thirds)
    fn cycle_height(total_height: f64, cycle_state: CycleState) -> f64 {
        match cycle_state {
            CycleState::Initial => total_height / 2.0,
            CycleState::FirstCycle => total_height / 3.0,
            CycleState::SecondCycle => total_height * 2.0 / 3.0,
        }
    }

    // Helper: Get center layout width based on cycle state (80% -> 50% -> 66%)
    fn center_cycle_width(total_width: f64, cycle_state: CycleState) -> f64 {
        match cycle_state {
            CycleState::Initial => total_width * 0.8,
            CycleState::FirstCycle => total_width * 0.5,
            CycleState::SecondCycle => total_width * 2.0 / 3.0,
        }
    }

    /// Calculates the target frame for a given layout.
    ///
    /// # Arguments
    /// * `layout` - The layout to apply
    /// * `screen_frame` - The screen's full frame
    /// * `cycle_state` - The current cycle state (for cycling layouts)
    ///
    /// # Returns
    /// The calculated window frame (x, y, width, height)
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn calculate_frame(
        &self,
        layout: WindowLayout,
        screen_frame: CGRect,
        cycle_state: CycleState,
    ) -> CGRect {
        // Get usable screen area (excluding menu bar and dock)
        let usable_frame = self.get_usable_frame(screen_frame);
        let x = usable_frame.origin.x;
        let y = usable_frame.origin.y;
        let width = usable_frame.size.width;
        let height = usable_frame.size.height;

        match layout {
            // Halves with cycling
            WindowLayout::LeftHalf => {
                let w = Self::cycle_width(width, cycle_state);
                Self::frame_left(x, y, w, height)
            },
            WindowLayout::RightHalf => {
                let w = Self::cycle_width(width, cycle_state);
                Self::frame_right(x, y, width, w, height)
            },
            WindowLayout::TopHalf => {
                let h = Self::cycle_height(height, cycle_state);
                Self::frame_left(x, y, width, h)
            },
            WindowLayout::BottomHalf => {
                let h = Self::cycle_height(height, cycle_state);
                Self::frame_bottom(x, y, height, width, h)
            },

            // Quarters
            WindowLayout::TopLeft => {
                Self::frame_left(x, y, width / 2.0, height / 2.0)
            },
            WindowLayout::TopRight => {
                Self::frame_right(x, y, width, width / 2.0, height / 2.0)
            },
            WindowLayout::BottomLeft => {
                Self::frame_bottom(x, y, height, width / 2.0, height / 2.0)
            },
            WindowLayout::BottomRight => {
                Self::frame_bottom_right(x, y, width, height, width / 2.0, height / 2.0)
            },

            // Thirds
            WindowLayout::FirstThird => {
                Self::frame_left(x, y, width / 3.0, height)
            },
            WindowLayout::CenterThird => {
                Self::frame_centered_horizontal(x, y, width, width / 3.0, height)
            },
            WindowLayout::LastThird => {
                Self::frame_right(x, y, width, width / 3.0, height)
            },
            WindowLayout::FirstTwoThirds => {
                Self::frame_left(x, y, width * 2.0 / 3.0, height)
            },
            WindowLayout::LastTwoThirds => {
                Self::frame_right(x, y, width, width * 2.0 / 3.0, height)
            },

            // Special
            WindowLayout::Maximize => usable_frame,
            WindowLayout::Center => {
                // Center with cycling width (80% -> 50% -> 66%), always 80% height
                let w = Self::center_cycle_width(width, cycle_state);
                let h = height * 0.8;
                Self::frame_centered(x, y, width, height, w, h)
            },
            WindowLayout::Restore => {
                // Restore is handled separately by restoring the saved frame
                usable_frame
            },

            // New layouts
            WindowLayout::AlmostMaximize => {
                // Fill screen with configurable margin
                let margin = self.almost_maximize_margin;
                Self::frame_left(
                    x + margin, y + margin,
                    width - 2.0 * margin, height - 2.0 * margin
                )
            },
            WindowLayout::CenterHalf => {
                // Center window at 50% screen width, full height
                Self::frame_centered_horizontal(x, y, width, width * 0.5, height)
            },
            WindowLayout::CenterTwoThirds => {
                // Center window at 66% screen width, full height
                Self::frame_centered_horizontal(x, y, width, width * 2.0 / 3.0, height)
            },
            WindowLayout::ReasonableSize => {
                // 75% width, 80% height, centered, with min 800x600
                let w = (width * 0.75).max(800.0).min(width);
                let h = (height * 0.8).max(600.0).min(height);
                Self::frame_centered(x, y, width, height, w, h)
            },
            WindowLayout::MakeSmaller | WindowLayout::MakeLarger => {
                // These require current window frame - return usable_frame as placeholder
                // Actual implementation should be handled in WindowManager
                usable_frame
            },
            WindowLayout::ToggleFullscreen => {
                // Handled specially in WindowManager - return usable_frame as placeholder
                usable_frame
            },
        }
    }

    /// Gets the usable screen frame (excluding menu bar and dock based on config).
    fn get_usable_frame(&self, screen_frame: CGRect) -> CGRect {
        let mut usable = screen_frame;

        // Account for menu bar at the top if configured
        if self.respect_menu_bar {
            usable.origin.y += self.menu_bar_height;
            usable.size.height -= self.menu_bar_height;
        }

        // Account for dock if present and configured
        if self.respect_dock {
            if let Some(dock) = self.dock_bounds {
                // Determine dock position and adjust usable frame
                // This is simplified - in reality, dock position detection is complex
                // For now, assume dock is at bottom (most common)
                usable.size.height -= dock.size.height;
            }
        }

        // Apply window gap to all edges
        if self.window_gap > 0.0 {
            usable.origin.x += self.window_gap;
            usable.origin.y += self.window_gap;
            usable.size.width -= 2.0 * self.window_gap;
            usable.size.height -= 2.0 * self.window_gap;
        }

        usable
    }

    /// Resizes a frame by a percentage from its center.
    ///
    /// # Arguments
    /// * `current_frame` - The current window frame
    /// * `screen_frame` - The screen's full frame (for bounds checking)
    /// * `percent` - The percentage to resize by (e.g., 0.1 for 10%)
    /// * `grow` - True to grow, false to shrink
    ///
    /// # Returns
    /// The resized frame, respecting min 400x300 and max screen bounds
    #[must_use]
    pub fn resize_frame(
        &self,
        current_frame: CGRect,
        screen_frame: CGRect,
        percent: f64,
        grow: bool,
    ) -> CGRect {
        let usable_frame = self.get_usable_frame(screen_frame);

        // Calculate new dimensions
        let factor = if grow { 1.0 + percent } else { 1.0 - percent };
        let new_width = current_frame.size.width * factor;
        let new_height = current_frame.size.height * factor;

        // Apply min/max constraints
        let min_width = 400.0;
        let min_height = 300.0;
        let max_width = usable_frame.size.width;
        let max_height = usable_frame.size.height;

        let final_width = new_width.max(min_width).min(max_width);
        let final_height = new_height.max(min_height).min(max_height);

        // Calculate new position to keep window centered
        let center_x = current_frame.origin.x + current_frame.size.width / 2.0;
        let center_y = current_frame.origin.y + current_frame.size.height / 2.0;

        let mut new_x = center_x - final_width / 2.0;
        let mut new_y = center_y - final_height / 2.0;

        // Ensure window stays within usable bounds
        if new_x < usable_frame.origin.x {
            new_x = usable_frame.origin.x;
        }
        if new_y < usable_frame.origin.y {
            new_y = usable_frame.origin.y;
        }
        if new_x + final_width > usable_frame.origin.x + usable_frame.size.width {
            new_x = usable_frame.origin.x + usable_frame.size.width - final_width;
        }
        if new_y + final_height > usable_frame.origin.y + usable_frame.size.height {
            new_y = usable_frame.origin.y + usable_frame.size.height - final_height;
        }

        CGRect::new(
            &core_graphics::geometry::CGPoint { x: new_x, y: new_y },
            &core_graphics::geometry::CGSize {
                width: final_width,
                height: final_height,
            },
        )
    }
}

impl Default for LayoutCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_names() {
        assert_eq!(WindowLayout::LeftHalf.name(), "Left Half");
        assert_eq!(WindowLayout::TopLeft.name(), "Top Left Quarter");
        assert_eq!(WindowLayout::FirstThird.name(), "First Third");
    }

    #[test]
    fn test_layout_ids() {
        assert_eq!(WindowLayout::LeftHalf.id(), "window_left_half");
        assert_eq!(WindowLayout::TopLeft.id(), "window_top_left");
        assert_eq!(WindowLayout::Maximize.id(), "window_maximize");
    }

    #[test]
    fn test_cycle_state() {
        let state = CycleState::Initial;
        assert_eq!(state.next(), CycleState::FirstCycle);
        assert_eq!(state.next().next(), CycleState::SecondCycle);
        assert_eq!(state.next().next().next(), CycleState::Initial);
    }

    #[test]
    fn test_left_half_layout() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        // Initial state: 50%
        let frame = calc.calculate_frame(WindowLayout::LeftHalf, screen, CycleState::Initial);
        assert!((frame.origin.x - 0.0).abs() < f64::EPSILON);
        assert!((frame.size.width - 960.0).abs() < 0.1);

        // First cycle: 33%
        let frame = calc.calculate_frame(WindowLayout::LeftHalf, screen, CycleState::FirstCycle);
        assert!((frame.size.width - 640.0).abs() < 0.1);

        // Second cycle: 66%
        let frame = calc.calculate_frame(WindowLayout::LeftHalf, screen, CycleState::SecondCycle);
        assert!((frame.size.width - 1280.0).abs() < 0.1);
    }

    #[test]
    fn test_maximize_layout() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        let frame = calc.calculate_frame(WindowLayout::Maximize, screen, CycleState::Initial);
        // Should account for menu bar
        assert!((frame.origin.y - 24.0).abs() < f64::EPSILON);
        assert!((frame.size.height - (1080.0 - 24.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_almost_maximize_layout() {
        let calc = LayoutCalculator::with_config(0, true, true, 20);
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        let frame = calc.calculate_frame(WindowLayout::AlmostMaximize, screen, CycleState::Initial);
        // Should have 20px margin on all sides (plus menu bar adjustment)
        assert!((frame.origin.x - 20.0).abs() < f64::EPSILON);
        assert!((frame.size.width - (1920.0 - 40.0)).abs() < 0.1);
    }

    #[test]
    fn test_center_half_layout() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        let frame = calc.calculate_frame(WindowLayout::CenterHalf, screen, CycleState::Initial);
        // Should be 50% width, centered
        assert!((frame.size.width - 960.0).abs() < 0.1);
        // Origin should be at 25% of screen width (accounting for menu bar adjustment)
        assert!((frame.origin.x - 480.0).abs() < 0.1);
    }

    #[test]
    fn test_center_two_thirds_layout() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        let frame = calc.calculate_frame(WindowLayout::CenterTwoThirds, screen, CycleState::Initial);
        // Should be 66% width, centered
        assert!((frame.size.width - 1280.0).abs() < 0.1);
    }

    #[test]
    fn test_reasonable_size_layout() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );

        let frame = calc.calculate_frame(WindowLayout::ReasonableSize, screen, CycleState::Initial);
        // Should be 75% width, 80% height (accounting for menu bar)
        assert!((frame.size.width - 1440.0).abs() < 0.1); // 1920 * 0.75
    }

    #[test]
    fn test_resize_frame_shrink() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );
        let current = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 100.0, y: 100.0 },
            &core_graphics::geometry::CGSize {
                width: 800.0,
                height: 600.0,
            },
        );

        let frame = calc.resize_frame(current, screen, 0.1, false);
        // Should shrink by 10%
        assert!((frame.size.width - 720.0).abs() < 0.1);
        assert!((frame.size.height - 540.0).abs() < 0.1);
    }

    #[test]
    fn test_resize_frame_grow() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );
        let current = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 100.0, y: 100.0 },
            &core_graphics::geometry::CGSize {
                width: 800.0,
                height: 600.0,
            },
        );

        let frame = calc.resize_frame(current, screen, 0.1, true);
        // Should grow by 10%
        assert!((frame.size.width - 880.0).abs() < 0.1);
        assert!((frame.size.height - 660.0).abs() < 0.1);
    }

    #[test]
    fn test_resize_frame_respects_min() {
        let calc = LayoutCalculator::new();
        let screen = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 1920.0,
                height: 1080.0,
            },
        );
        let current = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 100.0, y: 100.0 },
            &core_graphics::geometry::CGSize {
                width: 410.0,
                height: 310.0,
            },
        );

        // Shrink by 10% - should hit minimum
        let frame = calc.resize_frame(current, screen, 0.1, false);
        assert!(frame.size.width >= 400.0);
        assert!(frame.size.height >= 300.0);
    }

    #[test]
    fn test_new_layout_ids() {
        assert_eq!(WindowLayout::AlmostMaximize.id(), "window_almost_maximize");
        assert_eq!(WindowLayout::CenterHalf.id(), "window_center_half");
        assert_eq!(WindowLayout::CenterTwoThirds.id(), "window_center_two_thirds");
        assert_eq!(WindowLayout::ReasonableSize.id(), "window_reasonable_size");
        assert_eq!(WindowLayout::MakeSmaller.id(), "window_make_smaller");
        assert_eq!(WindowLayout::MakeLarger.id(), "window_make_larger");
        assert_eq!(WindowLayout::ToggleFullscreen.id(), "window_toggle_fullscreen");
    }

    #[test]
    fn test_new_layout_from_id() {
        assert_eq!(WindowLayout::from_id("window_almost_maximize"), Some(WindowLayout::AlmostMaximize));
        assert_eq!(WindowLayout::from_id("window_center_half"), Some(WindowLayout::CenterHalf));
        assert_eq!(WindowLayout::from_id("window_toggle_fullscreen"), Some(WindowLayout::ToggleFullscreen));
        assert_eq!(WindowLayout::from_id("invalid_id"), None);
    }
}
