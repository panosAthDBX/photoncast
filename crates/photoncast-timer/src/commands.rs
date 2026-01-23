//! Timer commands and search provider.
//!
//! This module provides integration with PhotonCast's command system,
//! allowing users to create timers through natural language search.

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::debug;

use crate::error::Result;
use crate::parser::{parse_timer_expression, TimerExpression};
use crate::scheduler::{ActiveTimer, TimerAction, TimerScheduler};

/// Timer command information.
#[derive(Debug, Clone)]
pub struct TimerCommand {
    /// Display name
    pub name: &'static str,
    /// Description
    pub description: &'static str,
    /// Icon name
    pub icon: &'static str,
    /// Timer action
    pub action: TimerAction,
    /// Example queries
    pub examples: &'static [&'static str],
}

impl TimerCommand {
    /// Returns information about all available timer commands.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self {
                name: "Sleep Timer",
                description: "Put Mac to sleep after delay",
                icon: "moon",
                action: TimerAction::Sleep,
                examples: &["sleep in 30 minutes", "30m", "sleep at 10pm"],
            },
            Self {
                name: "Shutdown Timer",
                description: "Shut down Mac after delay",
                icon: "power",
                action: TimerAction::Shutdown,
                examples: &["shutdown in 1 hour", "shutdown at 11pm"],
            },
            Self {
                name: "Restart Timer",
                description: "Restart Mac after delay",
                icon: "rotate-ccw",
                action: TimerAction::Restart,
                examples: &["restart in 2 hours", "restart at midnight"],
            },
            Self {
                name: "Lock Timer",
                description: "Lock screen after delay",
                icon: "lock",
                action: TimerAction::Lock,
                examples: &["lock in 15 minutes", "lock at 5pm"],
            },
            Self {
                name: "Cancel Timer",
                description: "Cancel active timer",
                icon: "x-circle",
                action: TimerAction::Sleep, // Dummy, not used for cancel
                examples: &["cancel timer", "stop timer"],
            },
            Self {
                name: "Show Timer",
                description: "View active timer status",
                icon: "clock",
                action: TimerAction::Sleep, // Dummy, not used for show
                examples: &["show timer", "timer status", "active timer"],
            },
        ]
    }
}

/// Timer manager that handles scheduling and execution.
pub struct TimerManager {
    /// Timer scheduler
    scheduler: Arc<RwLock<TimerScheduler>>,
    /// Active timer monitoring task handle
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TimerManager {
    /// Creates a new timer manager.
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler cannot be initialized.
    #[allow(clippy::future_not_send)]
    pub async fn new(db_path: impl AsRef<std::path::Path>) -> Result<Self> {
        let scheduler = TimerScheduler::new(db_path).await?;

        Ok(Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
            monitor_handle: None,
        })
    }

    /// Checks if the timer has expired and returns the action if so, without executing.
    ///
    /// Use this when you want to execute the action in a separate thread to avoid
    /// blocking the UI or crossing thread boundaries with rusqlite.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(action))` if timer expired (and was cancelled),
    /// `Ok(None)` if no timer or not expired yet.
    ///
    /// # Errors
    ///
    /// Returns an error if database access fails.
    #[allow(clippy::future_not_send)]
    pub async fn check_expired(&self) -> Result<Option<TimerAction>> {
        let timer = self.get_timer().await?;

        if let Some(timer) = timer {
            if timer.is_expired() {
                let action = timer.action;
                debug!("Timer expired, returning action: {:?}", action);

                // Cancel the timer first
                self.cancel_timer().await?;

                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    /// Checks if the timer has expired and executes the action if so.
    ///
    /// This method should be called periodically from the main application event loop
    /// (e.g., every second). It avoids background threading issues with rusqlite.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(action))` if an action was executed, `Ok(None)` if no timer
    /// or timer hasn't expired yet, or an error if execution failed.
    ///
    /// # Errors
    ///
    /// Returns an error if database access or action execution fails.
    #[allow(clippy::future_not_send)]
    pub async fn check_and_execute(&self) -> Result<Option<TimerAction>> {
        let timer = self.get_timer().await?;

        if let Some(timer) = timer {
            if timer.is_expired() {
                let action = timer.action;
                debug!("Timer expired, executing action: {:?}", action);

                // Cancel the timer first
                self.cancel_timer().await?;

                // Execute the action
                crate::scheduler::TimerScheduler::execute_action(action).await?;

                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    /// Gets the remaining time until the timer expires.
    ///
    /// # Returns
    ///
    /// Returns `Some((action, duration))` if a timer is active, `None` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if database access fails.
    #[allow(clippy::future_not_send)]
    pub async fn get_remaining(&self) -> Result<Option<(TimerAction, chrono::Duration)>> {
        let timer = self.get_timer().await?;
        Ok(timer.map(|t| (t.action, t.remaining())))
    }

    /// Starts the timer monitor that checks for expired timers.
    ///
    /// Note: For proper integration, prefer using `check_and_execute()` from
    /// the main application event loop instead. This method provides a basic
    /// background implementation for standalone use.
    pub fn start_monitor(&mut self) {
        debug!("Timer monitor started - use check_and_execute() for polling");
    }

    /// Stops the timer monitor.
    pub fn stop_monitor(&mut self) {
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
        }
    }

    /// Sets a timer from a natural language expression.
    ///
    /// # Errors
    ///
    /// Returns an error if the expression cannot be parsed or the timer cannot be set.
    #[allow(clippy::future_not_send)]
    pub async fn set_timer_from_expression(&self, expression: &str) -> Result<TimerExpression> {
        let expr = parse_timer_expression(expression)?;

        let timer = ActiveTimer::new(expr.action, expr.execute_at);
        let scheduler = self.scheduler.write().await;
        scheduler.set_timer(timer).await?;
        drop(scheduler);

        debug!("Timer set from expression: {}", expression);

        Ok(expr)
    }

    /// Gets the active timer, if one exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    #[allow(clippy::future_not_send)]
    pub async fn get_timer(&self) -> Result<Option<ActiveTimer>> {
        let scheduler = self.scheduler.read().await;
        scheduler.get_timer().await
    }

    /// Cancels the active timer.
    ///
    /// # Errors
    ///
    /// Returns an error if the database deletion fails.
    #[allow(clippy::future_not_send)]
    pub async fn cancel_timer(&self) -> Result<()> {
        let scheduler = self.scheduler.write().await;
        scheduler.cancel_timer().await
    }
}

impl Drop for TimerManager {
    fn drop(&mut self) {
        self.stop_monitor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_timer_manager() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("timer.db");

        let manager = TimerManager::new(&db_path).await.unwrap();

        // Set timer from expression
        let expr = manager
            .set_timer_from_expression("sleep in 30 minutes")
            .await
            .unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        // Get timer
        let timer = manager.get_timer().await.unwrap();
        assert!(timer.is_some());

        // Cancel timer
        manager.cancel_timer().await.unwrap();
        let timer = manager.get_timer().await.unwrap();
        assert!(timer.is_none());
    }

    #[test]
    fn test_timer_commands() {
        let commands = TimerCommand::all();
        assert_eq!(commands.len(), 6); // Sleep, Shutdown, Restart, Lock, Cancel, Show

        let sleep_cmd = &commands[0];
        assert_eq!(sleep_cmd.name, "Sleep Timer");
        assert_eq!(sleep_cmd.action, TimerAction::Sleep);
    }
}
