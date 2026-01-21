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
}

impl LayoutCalculator {
    /// Creates a new layout calculator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            menu_bar_height: 24.0, // Standard macOS menu bar
            dock_bounds: None,
        }
    }

    /// Updates the dock bounds cache.
    pub fn update_dock_bounds(&mut self, bounds: Option<CGRect>) {
        self.dock_bounds = bounds;
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
                let w = match cycle_state {
                    CycleState::Initial => width / 2.0,
                    CycleState::FirstCycle => width / 3.0,
                    CycleState::SecondCycle => width * 2.0 / 3.0,
                };
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::RightHalf => {
                let w = match cycle_state {
                    CycleState::Initial => width / 2.0,
                    CycleState::FirstCycle => width / 3.0,
                    CycleState::SecondCycle => width * 2.0 / 3.0,
                };
                let offset_x = width - w;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x: x + offset_x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::TopHalf => {
                let h = height / 2.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y },
                    &core_graphics::geometry::CGSize { width, height: h },
                )
            },
            WindowLayout::BottomHalf => {
                let h = height / 2.0;
                let offset_y = height - h;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y: y + offset_y },
                    &core_graphics::geometry::CGSize { width, height: h },
                )
            },

            // Quarters
            WindowLayout::TopLeft => {
                let w = width / 2.0;
                let h = height / 2.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y },
                    &core_graphics::geometry::CGSize {
                        width: w,
                        height: h,
                    },
                )
            },
            WindowLayout::TopRight => {
                let w = width / 2.0;
                let h = height / 2.0;
                let offset_x = width - w;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x: x + offset_x, y },
                    &core_graphics::geometry::CGSize {
                        width: w,
                        height: h,
                    },
                )
            },
            WindowLayout::BottomLeft => {
                let w = width / 2.0;
                let h = height / 2.0;
                let offset_y = height - h;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y: y + offset_y },
                    &core_graphics::geometry::CGSize {
                        width: w,
                        height: h,
                    },
                )
            },
            WindowLayout::BottomRight => {
                let w = width / 2.0;
                let h = height / 2.0;
                let offset_x = width - w;
                let offset_y = height - h;
                CGRect::new(
                    &core_graphics::geometry::CGPoint {
                        x: x + offset_x,
                        y: y + offset_y,
                    },
                    &core_graphics::geometry::CGSize {
                        width: w,
                        height: h,
                    },
                )
            },

            // Thirds
            WindowLayout::FirstThird => {
                let w = width / 3.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::CenterThird => {
                let w = width / 3.0;
                let offset_x = width / 3.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x: x + offset_x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::LastThird => {
                let w = width / 3.0;
                let offset_x = width * 2.0 / 3.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x: x + offset_x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::FirstTwoThirds => {
                let w = width * 2.0 / 3.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },
            WindowLayout::LastTwoThirds => {
                let w = width * 2.0 / 3.0;
                let offset_x = width / 3.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint { x: x + offset_x, y },
                    &core_graphics::geometry::CGSize { width: w, height },
                )
            },

            // Special
            WindowLayout::Maximize => usable_frame,
            WindowLayout::Center => {
                // Center with 80% width and height
                let w = width * 0.8;
                let h = height * 0.8;
                let offset_x = (width - w) / 2.0;
                let offset_y = (height - h) / 2.0;
                CGRect::new(
                    &core_graphics::geometry::CGPoint {
                        x: x + offset_x,
                        y: y + offset_y,
                    },
                    &core_graphics::geometry::CGSize {
                        width: w,
                        height: h,
                    },
                )
            },
            WindowLayout::Restore => {
                // Restore is handled separately by restoring the saved frame
                usable_frame
            },
        }
    }

    /// Gets the usable screen frame (excluding menu bar and dock).
    fn get_usable_frame(&self, screen_frame: CGRect) -> CGRect {
        let mut usable = screen_frame;

        // Account for menu bar at the top
        usable.origin.y += self.menu_bar_height;
        usable.size.height -= self.menu_bar_height;

        // Account for dock if present
        if let Some(dock) = self.dock_bounds {
            // Determine dock position and adjust usable frame
            // This is simplified - in reality, dock position detection is complex
            // For now, assume dock is at bottom (most common)
            usable.size.height -= dock.size.height;
        }

        usable
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
}
