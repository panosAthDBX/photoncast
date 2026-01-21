//! Window layout cycling state management.

use crate::layout::{CycleState, WindowLayout};
use std::collections::HashMap;

/// Tracks cycling state for each window.
#[derive(Debug)]
pub struct CyclingManager {
    /// Maps window ID to the last applied layout and cycle state.
    state: HashMap<usize, (WindowLayout, CycleState)>,
}

impl CyclingManager {
    /// Creates a new cycling manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
        }
    }

    /// Gets the cycle state for applying a layout to a window.
    ///
    /// If the same layout is applied repeatedly, the cycle state advances.
    /// If a different layout is applied, the cycle state resets.
    #[must_use]
    pub fn get_cycle_state(&mut self, window_id: usize, layout: WindowLayout) -> CycleState {
        // Check if this layout supports cycling
        if !Self::supports_cycling(layout) {
            return CycleState::Initial;
        }

        // Check previous state
        if let Some((prev_layout, prev_state)) = self.state.get(&window_id) {
            if *prev_layout == layout {
                // Same layout repeated - advance cycle
                let new_state = prev_state.next();
                self.state.insert(window_id, (layout, new_state));
                new_state
            } else {
                // Different layout - reset cycle
                self.state.insert(window_id, (layout, CycleState::Initial));
                CycleState::Initial
            }
        } else {
            // First time for this window - start at initial
            self.state.insert(window_id, (layout, CycleState::Initial));
            CycleState::Initial
        }
    }

    /// Resets the cycling state for a window.
    pub fn reset(&mut self, window_id: usize) {
        self.state.remove(&window_id);
    }

    /// Clears all cycling state.
    pub fn clear(&mut self) {
        self.state.clear();
    }

    /// Checks if a layout supports cycling.
    #[must_use]
    const fn supports_cycling(layout: WindowLayout) -> bool {
        matches!(layout, WindowLayout::LeftHalf | WindowLayout::RightHalf)
    }
}

impl Default for CyclingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycling_same_layout() {
        let mut manager = CyclingManager::new();
        let window_id = 123;

        // First application
        let state1 = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state1, CycleState::Initial);

        // Second application
        let state2 = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state2, CycleState::FirstCycle);

        // Third application
        let state3 = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state3, CycleState::SecondCycle);

        // Fourth application - cycle back
        let state4 = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state4, CycleState::Initial);
    }

    #[test]
    fn test_cycling_different_layout() {
        let mut manager = CyclingManager::new();
        let window_id = 123;

        // Apply left half twice
        let _ = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        let state = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state, CycleState::FirstCycle);

        // Switch to right half - should reset
        let state = manager.get_cycle_state(window_id, WindowLayout::RightHalf);
        assert_eq!(state, CycleState::Initial);
    }

    #[test]
    fn test_non_cycling_layout() {
        let mut manager = CyclingManager::new();
        let window_id = 123;

        // Maximize doesn't support cycling
        let state1 = manager.get_cycle_state(window_id, WindowLayout::Maximize);
        assert_eq!(state1, CycleState::Initial);

        let state2 = manager.get_cycle_state(window_id, WindowLayout::Maximize);
        assert_eq!(state2, CycleState::Initial); // Should not advance
    }

    #[test]
    fn test_reset() {
        let mut manager = CyclingManager::new();
        let window_id = 123;

        // Build up some state
        let _ = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        let _ = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);

        // Reset
        manager.reset(window_id);

        // Should start fresh
        let state = manager.get_cycle_state(window_id, WindowLayout::LeftHalf);
        assert_eq!(state, CycleState::Initial);
    }

    #[test]
    fn test_clear() {
        let mut manager = CyclingManager::new();

        let _ = manager.get_cycle_state(123, WindowLayout::LeftHalf);
        let _ = manager.get_cycle_state(456, WindowLayout::RightHalf);

        manager.clear();

        // All state should be cleared
        let state1 = manager.get_cycle_state(123, WindowLayout::LeftHalf);
        let state2 = manager.get_cycle_state(456, WindowLayout::RightHalf);
        assert_eq!(state1, CycleState::Initial);
        assert_eq!(state2, CycleState::Initial);
    }
}
