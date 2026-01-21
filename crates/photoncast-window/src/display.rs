//! Multi-monitor display management.

use core_graphics::display::{CGDirectDisplayID, CGDisplay, CGRect};

/// Information about a display.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    /// The display ID.
    pub id: CGDirectDisplayID,
    /// The display frame (position and size).
    pub frame: CGRect,
    /// Whether this is the main display.
    pub is_main: bool,
    /// Display index in the arrangement order.
    pub index: usize,
}

/// Manages multiple displays.
#[derive(Debug)]
pub struct DisplayManager {
    /// Cached list of displays.
    displays: Vec<DisplayInfo>,
}

impl DisplayManager {
    /// Creates a new display manager.
    #[must_use]
    pub fn new() -> Self {
        let mut manager = Self {
            displays: Vec::new(),
        };
        manager.refresh_displays();
        manager
    }

    /// Refreshes the list of connected displays.
    #[allow(clippy::cast_possible_truncation)]
    pub fn refresh_displays(&mut self) {
        #[cfg(target_os = "macos")]
        {
            self.displays.clear();

            // Get all active displays
            let max_displays = 32u32;
            let mut display_ids = vec![0u32; max_displays as usize];
            let mut display_count = 0u32;

            unsafe {
                core_graphics::display::CGGetActiveDisplayList(
                    max_displays,
                    display_ids.as_mut_ptr(),
                    &mut display_count,
                );
            }

            // Get main display ID
            let main_display_id = unsafe { core_graphics::display::CGMainDisplayID() };

            // Collect display info
            for (index, &display_id) in display_ids.iter().take(display_count as usize).enumerate()
            {
                let display = CGDisplay::new(display_id);
                let bounds = display.bounds();

                self.displays.push(DisplayInfo {
                    id: display_id,
                    frame: bounds,
                    is_main: display_id == main_display_id,
                    index,
                });
            }

            // Sort by index (macOS arrangement order)
            self.displays.sort_by_key(|d| d.index);

            tracing::debug!("Found {} displays", self.displays.len());
        }
    }

    /// Gets all connected displays.
    #[must_use]
    pub fn displays(&self) -> &[DisplayInfo] {
        &self.displays
    }

    /// Gets the main display.
    #[must_use]
    pub fn main_display(&self) -> Option<&DisplayInfo> {
        self.displays.iter().find(|d| d.is_main)
    }

    /// Gets the display at the specified index.
    #[must_use]
    pub fn display_at_index(&self, index: usize) -> Option<&DisplayInfo> {
        self.displays.get(index)
    }

    /// Gets the display containing the given point.
    #[must_use]
    pub fn display_containing_point(&self, point: (f64, f64)) -> Option<&DisplayInfo> {
        self.displays.iter().find(|display| {
            let frame = &display.frame;
            point.0 >= frame.origin.x
                && point.0 <= frame.origin.x + frame.size.width
                && point.1 >= frame.origin.y
                && point.1 <= frame.origin.y + frame.size.height
        })
    }

    /// Gets the display containing the given window frame.
    ///
    /// Returns the display that contains the majority of the window.
    #[must_use]
    pub fn display_containing_frame(&self, frame: &CGRect) -> Option<&DisplayInfo> {
        // Simple implementation: use window center point
        let center_x = frame.origin.x + frame.size.width / 2.0;
        let center_y = frame.origin.y + frame.size.height / 2.0;
        self.display_containing_point((center_x, center_y))
    }

    /// Gets the next display in the arrangement order.
    #[must_use]
    pub fn next_display(&self, current: &DisplayInfo) -> Option<&DisplayInfo> {
        let next_index = (current.index + 1) % self.displays.len();
        self.display_at_index(next_index)
    }

    /// Gets the previous display in the arrangement order.
    #[must_use]
    pub fn previous_display(&self, current: &DisplayInfo) -> Option<&DisplayInfo> {
        let prev_index = if current.index == 0 {
            self.displays.len().saturating_sub(1)
        } else {
            current.index - 1
        };
        self.display_at_index(prev_index)
    }

    /// Translates a frame from one display to another, preserving relative position.
    ///
    /// For example, a window at the left half of display 1 will be moved to the
    /// left half of display 2.
    #[must_use]
    pub fn translate_frame(
        &self,
        frame: &CGRect,
        from_display: &DisplayInfo,
        to_display: &DisplayInfo,
    ) -> CGRect {
        // Calculate relative position in source display
        let rel_x = (frame.origin.x - from_display.frame.origin.x) / from_display.frame.size.width;
        let rel_y = (frame.origin.y - from_display.frame.origin.y) / from_display.frame.size.height;
        let rel_width = frame.size.width / from_display.frame.size.width;
        let rel_height = frame.size.height / from_display.frame.size.height;

        // Apply to target display
        let new_x = rel_x.mul_add(to_display.frame.size.width, to_display.frame.origin.x);
        let new_y = rel_y.mul_add(to_display.frame.size.height, to_display.frame.origin.y);
        let new_width = rel_width * to_display.frame.size.width;
        let new_height = rel_height * to_display.frame.size.height;

        CGRect::new(
            &core_graphics::geometry::CGPoint { x: new_x, y: new_y },
            &core_graphics::geometry::CGSize {
                width: new_width,
                height: new_height,
            },
        )
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Display movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDirection {
    /// Move to the next display.
    Next,
    /// Move to the previous display.
    Previous,
    /// Move to a specific display index.
    Index(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_manager_creation() {
        let manager = DisplayManager::new();
        // At least one display should be present
        assert!(!manager.displays().is_empty());
    }

    #[test]
    fn test_main_display() {
        let manager = DisplayManager::new();
        let main = manager.main_display();
        assert!(main.is_some());
        if let Some(main) = main {
            assert!(main.is_main);
        }
    }

    #[test]
    fn test_translate_frame() {
        let manager = DisplayManager::new();

        // Create mock displays
        let display1 = DisplayInfo {
            id: 1,
            frame: CGRect::new(
                &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
                &core_graphics::geometry::CGSize {
                    width: 1920.0,
                    height: 1080.0,
                },
            ),
            is_main: true,
            index: 0,
        };

        let display2 = DisplayInfo {
            id: 2,
            frame: CGRect::new(
                &core_graphics::geometry::CGPoint { x: 1920.0, y: 0.0 },
                &core_graphics::geometry::CGSize {
                    width: 2560.0,
                    height: 1440.0,
                },
            ),
            is_main: false,
            index: 1,
        };

        // Window at left half of display 1
        let window = CGRect::new(
            &core_graphics::geometry::CGPoint { x: 0.0, y: 0.0 },
            &core_graphics::geometry::CGSize {
                width: 960.0,
                height: 1080.0,
            },
        );

        // Translate to display 2
        let translated = manager.translate_frame(&window, &display1, &display2);

        // Should be at left half of display 2
        assert!((translated.origin.x - 1920.0).abs() < f64::EPSILON);
        assert!((translated.size.width - 1280.0).abs() < 0.1);
        assert!((translated.size.height - 1440.0).abs() < 0.1);
    }
}
