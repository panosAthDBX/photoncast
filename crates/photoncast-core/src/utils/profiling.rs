//! Performance profiling utilities.
//!
//! This module provides utilities for measuring and reporting
//! performance metrics for PhotonCast.

use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Performance targets for PhotonCast.
pub mod targets {
    use std::time::Duration;

    /// Target cold start time.
    pub const COLD_START: Duration = Duration::from_millis(100);

    /// Target hotkey response time.
    pub const HOTKEY_RESPONSE: Duration = Duration::from_millis(50);

    /// Target search latency.
    pub const SEARCH_LATENCY: Duration = Duration::from_millis(30);

    /// Target memory usage (idle) in bytes.
    pub const MEMORY_IDLE_BYTES: usize = 50 * 1024 * 1024; // 50MB
}

/// Result of a performance measurement.
#[derive(Debug, Clone)]
pub struct ProfileResult {
    /// Name of the operation.
    pub name: String,
    /// Duration of the operation.
    pub duration: Duration,
    /// Target duration.
    pub target: Duration,
    /// Whether the target was met.
    pub met_target: bool,
}

impl ProfileResult {
    /// Creates a new profile result.
    #[must_use]
    pub fn new(name: impl Into<String>, duration: Duration, target: Duration) -> Self {
        Self {
            name: name.into(),
            met_target: duration <= target,
            duration,
            target,
        }
    }

    /// Returns the overshoot as a percentage (0 if target met).
    #[must_use]
    pub fn overshoot_percent(&self) -> f64 {
        if self.met_target {
            0.0
        } else {
            let overshoot = self.duration.as_secs_f64() - self.target.as_secs_f64();
            (overshoot / self.target.as_secs_f64()) * 100.0
        }
    }

    /// Returns the margin as a percentage (0 if target not met).
    #[must_use]
    pub fn margin_percent(&self) -> f64 {
        if self.met_target {
            let margin = self.target.as_secs_f64() - self.duration.as_secs_f64();
            (margin / self.target.as_secs_f64()) * 100.0
        } else {
            0.0
        }
    }

    /// Logs the result.
    pub fn log(&self) {
        if self.met_target {
            info!(
                name = %self.name,
                duration_ms = self.duration.as_millis(),
                target_ms = self.target.as_millis(),
                margin = format!("{:.1}%", self.margin_percent()),
                "✓ Performance target met"
            );
        } else {
            warn!(
                name = %self.name,
                duration_ms = self.duration.as_millis(),
                target_ms = self.target.as_millis(),
                overshoot = format!("{:.1}%", self.overshoot_percent()),
                "✗ Performance target missed"
            );
        }
    }
}

/// Scoped profiler that measures time on drop.
pub struct ScopedProfiler {
    name: String,
    start: Instant,
    target: Duration,
}

impl ScopedProfiler {
    /// Creates a new scoped profiler.
    #[must_use]
    pub fn new(name: impl Into<String>, target: Duration) -> Self {
        let name = name.into();
        debug!(name = %name, "Starting profiler");
        Self {
            name,
            start: Instant::now(),
            target,
        }
    }

    /// Creates a profiler for cold start measurement.
    #[must_use]
    pub fn cold_start() -> Self {
        Self::new("Cold Start", targets::COLD_START)
    }

    /// Creates a profiler for hotkey response measurement.
    #[must_use]
    pub fn hotkey_response() -> Self {
        Self::new("Hotkey Response", targets::HOTKEY_RESPONSE)
    }

    /// Creates a profiler for search latency measurement.
    #[must_use]
    pub fn search_latency() -> Self {
        Self::new("Search Latency", targets::SEARCH_LATENCY)
    }

    /// Ends the profiler and returns the result without logging.
    #[must_use]
    pub fn finish(self) -> ProfileResult {
        let duration = self.start.elapsed();
        ProfileResult::new(&self.name, duration, self.target)
    }

    /// Ends the profiler, logs the result, and returns it.
    pub fn finish_and_log(self) -> ProfileResult {
        let result = self.finish();
        result.log();
        result
    }
}

/// Profiles a closure and returns its result along with the profile.
pub fn profile<T, F: FnOnce() -> T>(
    name: impl Into<String>,
    target: Duration,
    f: F,
) -> (T, ProfileResult) {
    let profiler = ScopedProfiler::new(name, target);
    let result = f();
    let profile = profiler.finish();
    (result, profile)
}

/// Profiles a closure, logs the result, and returns the value.
pub fn profile_and_log<T, F: FnOnce() -> T>(name: impl Into<String>, target: Duration, f: F) -> T {
    let profiler = ScopedProfiler::new(name, target);
    let result = f();
    profiler.finish_and_log();
    result
}

/// Performance report for the application.
#[derive(Debug, Default)]
pub struct PerformanceReport {
    /// Individual measurements.
    pub measurements: Vec<ProfileResult>,
}

impl PerformanceReport {
    /// Creates a new empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a measurement to the report.
    pub fn add(&mut self, result: ProfileResult) {
        self.measurements.push(result);
    }

    /// Returns the number of measurements.
    #[must_use]
    pub fn count(&self) -> usize {
        self.measurements.len()
    }

    /// Returns the number of targets met.
    #[must_use]
    pub fn targets_met(&self) -> usize {
        self.measurements.iter().filter(|m| m.met_target).count()
    }

    /// Returns the number of targets missed.
    #[must_use]
    pub fn targets_missed(&self) -> usize {
        self.measurements.iter().filter(|m| !m.met_target).count()
    }

    /// Returns true if all targets were met.
    #[must_use]
    pub fn all_targets_met(&self) -> bool {
        self.measurements.iter().all(|m| m.met_target)
    }

    /// Logs the full report.
    pub fn log(&self) {
        info!("=== Performance Report ===");
        for m in &self.measurements {
            m.log();
        }
        info!(
            met = self.targets_met(),
            missed = self.targets_missed(),
            total = self.count(),
            "Summary"
        );
    }

    /// Returns a summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Performance: {}/{} targets met ({}%)",
            self.targets_met(),
            self.count(),
            if self.count() > 0 {
                (self.targets_met() * 100) / self.count()
            } else {
                100
            }
        )
    }
}

/// Estimates current memory usage.
///
/// Note: This is a rough estimate on macOS.
#[cfg(target_os = "macos")]
#[must_use]
pub fn estimate_memory_usage() -> Option<usize> {
    use std::process::Command;

    let pid = std::process::id();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    let rss = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .ok()?;

    // ps reports in KB, convert to bytes
    Some(rss * 1024)
}

#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn estimate_memory_usage() -> Option<usize> {
    None
}

/// Checks if memory usage is within target.
#[must_use]
pub fn check_memory_target() -> ProfileResult {
    let usage = estimate_memory_usage().unwrap_or(0);
    let met = usage <= targets::MEMORY_IDLE_BYTES;

    ProfileResult {
        name: "Memory Usage".to_string(),
        duration: Duration::from_secs(0), // Not applicable
        target: Duration::from_secs(0),   // Not applicable
        met_target: met,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_result_met_target() {
        let result =
            ProfileResult::new("test", Duration::from_millis(20), Duration::from_millis(30));
        assert!(result.met_target);
        assert!(result.overshoot_percent().abs() < f64::EPSILON);
        assert!(result.margin_percent() > 0.0);
    }

    #[test]
    fn test_profile_result_missed_target() {
        let result =
            ProfileResult::new("test", Duration::from_millis(50), Duration::from_millis(30));
        assert!(!result.met_target);
        assert!(result.overshoot_percent() > 0.0);
        assert!(result.margin_percent().abs() < f64::EPSILON);
    }

    #[test]
    fn test_scoped_profiler() {
        let profiler = ScopedProfiler::new("test", Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(10));
        let result = profiler.finish();

        assert!(result.duration >= Duration::from_millis(10));
        assert!(result.met_target);
    }

    #[test]
    fn test_profile_function() {
        let (value, result) = profile("test", Duration::from_secs(1), || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(value, 42);
        assert!(result.met_target);
    }

    #[test]
    fn test_performance_report() {
        let mut report = PerformanceReport::new();

        report.add(ProfileResult::new(
            "fast",
            Duration::from_millis(10),
            Duration::from_millis(100),
        ));
        report.add(ProfileResult::new(
            "slow",
            Duration::from_millis(200),
            Duration::from_millis(100),
        ));

        assert_eq!(report.count(), 2);
        assert_eq!(report.targets_met(), 1);
        assert_eq!(report.targets_missed(), 1);
        assert!(!report.all_targets_met());
    }

    #[test]
    fn test_performance_report_summary() {
        let mut report = PerformanceReport::new();
        report.add(ProfileResult::new(
            "test",
            Duration::from_millis(10),
            Duration::from_millis(100),
        ));

        let summary = report.summary();
        assert!(summary.contains("1/1"));
        assert!(summary.contains("100%"));
    }

    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn test_targets_constants() {
        assert!(targets::COLD_START <= Duration::from_millis(100));
        assert!(targets::HOTKEY_RESPONSE <= Duration::from_millis(50));
        assert!(targets::SEARCH_LATENCY <= Duration::from_millis(30));
        assert!(targets::MEMORY_IDLE_BYTES <= 50 * 1024 * 1024);
    }
}
