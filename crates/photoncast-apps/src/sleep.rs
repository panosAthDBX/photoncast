//! App sleep feature - automatically stop idle applications.

use crate::config::AppSleepConfig;
use crate::error::Result;
use crate::process::quit_app;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Tracks activity for applications.
#[derive(Debug)]
pub struct AppSleepManager {
    /// Configuration.
    config: AppSleepConfig,
    /// Last activity time for each app (by PID).
    last_activity: RwLock<HashMap<u32, SystemTime>>,
}

impl AppSleepManager {
    /// Creates a new app sleep manager.
    #[must_use]
    pub fn new(config: AppSleepConfig) -> Self {
        Self {
            config,
            last_activity: RwLock::new(HashMap::new()),
        }
    }

    /// Updates the last activity time for an app.
    pub async fn record_activity(&self, pid: u32) {
        let mut activity = self.last_activity.write().await;
        activity.insert(pid, SystemTime::now());
    }

    /// Checks if an app should be put to sleep based on idle time.
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID
    /// * `bundle_id` - Optional bundle identifier for checking overrides
    pub async fn should_sleep(&self, pid: u32, bundle_id: Option<&str>) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check if app has a "never sleep" override
        if bundle_id
            .and_then(|id| self.config.app_overrides.get(id))
            .is_some_and(|override_config| override_config.never_sleep)
        {
            return false;
        }

        // Get idle timeout for this app
        let timeout_minutes = bundle_id.map_or(self.config.default_idle_minutes, |bundle_id| {
            self.config
                .app_overrides
                .get(bundle_id)
                .and_then(|c| c.idle_minutes)
                .unwrap_or(self.config.default_idle_minutes)
        });

        // Check last activity time
        let activity = self.last_activity.read().await;
        if let Some(last_time) = activity.get(&pid) {
            if let Ok(elapsed) = SystemTime::now().duration_since(*last_time) {
                let idle_duration = Duration::from_secs(u64::from(timeout_minutes) * 60);
                return elapsed >= idle_duration;
            }
        }
        drop(activity);
        false
    }

    /// Puts an app to sleep (gracefully quits it).
    ///
    /// # Errors
    ///
    /// Returns an error if the quit operation fails.
    pub async fn sleep_app(&self, pid: u32) -> Result<()> {
        tracing::info!("Sleeping app with PID {}", pid);

        // Remove from activity tracking
        let mut activity = self.last_activity.write().await;
        activity.remove(&pid);
        drop(activity);

        // Gracefully quit the app
        quit_app(pid)?;

        Ok(())
    }

    /// Monitors all running apps and sleeps idle ones.
    ///
    /// This should be called periodically (e.g., every minute) by the main app.
    ///
    /// # Errors
    ///
    /// Returns an error if monitoring fails.
    #[allow(clippy::unused_async)]
    pub async fn monitor_and_sleep(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // TODO: Get list of running apps and check each one
        // For now, this is a placeholder

        tracing::debug!("Checking for idle apps to sleep");

        Ok(())
    }

    /// Cleans up tracking for apps that are no longer running.
    pub async fn cleanup_stale_entries(&self, running_pids: &[u32]) {
        let mut activity = self.last_activity.write().await;
        activity.retain(|pid, _| running_pids.contains(pid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_sleep_disabled() {
        let config = AppSleepConfig {
            enabled: false,
            default_idle_minutes: 30,
            app_overrides: HashMap::new(),
        };

        let manager = AppSleepManager::new(config);

        // Should never sleep when disabled
        assert!(!manager.should_sleep(123, None).await);
    }

    #[tokio::test]
    async fn test_never_sleep_override() {
        use crate::config::AppSleepOverride;

        let mut overrides = HashMap::new();
        overrides.insert(
            "com.example.app".to_string(),
            AppSleepOverride {
                idle_minutes: None,
                never_sleep: true,
            },
        );

        let config = AppSleepConfig {
            enabled: true,
            default_idle_minutes: 30,
            app_overrides: overrides,
        };

        let manager = AppSleepManager::new(config);

        // Should never sleep this app
        assert!(!manager.should_sleep(123, Some("com.example.app")).await);
    }

    #[tokio::test]
    async fn test_record_activity() {
        let config = AppSleepConfig::default();
        let manager = AppSleepManager::new(config);

        // Record activity
        manager.record_activity(123).await;

        // Should have entry in tracking
        let activity = manager.last_activity.read().await;
        let has_entry = activity.contains_key(&123);
        drop(activity);
        assert!(has_entry);
    }
}
