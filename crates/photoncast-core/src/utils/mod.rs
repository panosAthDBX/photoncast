//! Shared utilities.

pub mod paths;
pub mod profiling;

pub use paths::*;
pub use profiling::{
    check_memory_target, estimate_memory_usage, profile, profile_and_log, targets,
    PerformanceReport, ProfileResult, ScopedProfiler,
};
