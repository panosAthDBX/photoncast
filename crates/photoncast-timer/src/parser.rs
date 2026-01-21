//! Natural language parser for timer expressions.
//!
//! Parses expressions like:
//! - "sleep in 30 minutes", "30 min", "30m"
//! - "shutdown in 1 hour", "1h", "1.5 hours"
//! - "at 10pm", "at 22:00"

use chrono::{DateTime, Duration, Local, NaiveTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;

use crate::error::{Result, TimerError};
use crate::scheduler::TimerAction;

lazy_static! {
    // Pattern: "sleep in 30 minutes", "shutdown in 1 hour"
    static ref RELATIVE_PATTERN: Regex = Regex::new(
        r"(?i)^(?P<action>sleep|shutdown|restart|lock)\s+in\s+(?P<amount>[\d.]+)\s*(?P<unit>s|sec|second|seconds|m|min|minute|minutes|h|hr|hour|hours)$"
    ).unwrap();

    // Pattern: "30 minutes", "1 hour", "2.5h"
    static ref DURATION_ONLY_PATTERN: Regex = Regex::new(
        r"(?i)^(?P<amount>[\d.]+)\s*(?P<unit>s|sec|second|seconds|m|min|minute|minutes|h|hr|hour|hours)$"
    ).unwrap();

    // Pattern: "at 10pm", "at 22:00", "at 10:30 pm"
    static ref TIME_PATTERN: Regex = Regex::new(
        r"(?i)^(?:at\s+)?(?P<hour>\d{1,2})(?::(?P<minute>\d{2}))?\s*(?P<period>am|pm)?$"
    ).unwrap();

    // Pattern with time: "sleep at 10pm", "shutdown at 22:00"
    static ref ACTION_TIME_PATTERN: Regex = Regex::new(
        r"(?i)^(?P<action>sleep|shutdown|restart|lock)\s+(?:at\s+)?(?P<hour>\d{1,2})(?::(?P<minute>\d{2}))?\s*(?P<period>am|pm)?$"
    ).unwrap();
}

/// Parsed timer expression.
#[derive(Debug, Clone)]
pub struct TimerExpression {
    /// Action to perform
    pub action: TimerAction,
    /// When to execute
    pub execute_at: DateTime<Utc>,
}

/// Parses a timer expression and returns the action and execution time.
///
/// # Examples
///
/// ```
/// use photoncast_timer::parser::parse_timer_expression;
///
/// // Relative time
/// let expr = parse_timer_expression("sleep in 30 minutes").unwrap();
/// let expr = parse_timer_expression("30m").unwrap();
///
/// // Absolute time
/// let expr = parse_timer_expression("at 10pm").unwrap();
/// let expr = parse_timer_expression("shutdown at 22:00").unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if the expression cannot be parsed.
pub fn parse_timer_expression(input: &str) -> Result<TimerExpression> {
    let input = input.trim();

    // Try relative pattern with action: "sleep in 30 minutes"
    if let Some(caps) = RELATIVE_PATTERN.captures(input) {
        let action = parse_action(&caps["action"])?;
        let duration = parse_duration(&caps["amount"], &caps["unit"])?;
        let execute_at = Utc::now() + duration;

        return Ok(TimerExpression { action, execute_at });
    }

    // Try action with time: "sleep at 10pm"
    if let Some(caps) = ACTION_TIME_PATTERN.captures(input) {
        let action = parse_action(&caps["action"])?;
        let execute_at = parse_time(&caps)?;

        return Ok(TimerExpression { action, execute_at });
    }

    // Try duration only: "30 minutes" (default to Sleep)
    if let Some(caps) = DURATION_ONLY_PATTERN.captures(input) {
        let duration = parse_duration(&caps["amount"], &caps["unit"])?;
        let execute_at = Utc::now() + duration;

        return Ok(TimerExpression {
            action: TimerAction::Sleep,
            execute_at,
        });
    }

    // Try time only: "at 10pm" (default to Sleep)
    if let Some(caps) = TIME_PATTERN.captures(input) {
        let execute_at = parse_time(&caps)?;

        return Ok(TimerExpression {
            action: TimerAction::Sleep,
            execute_at,
        });
    }

    Err(TimerError::Parse(format!(
        "Could not parse timer expression: {input}"
    )))
}

/// Parses an action string into a TimerAction.
fn parse_action(s: &str) -> Result<TimerAction> {
    match s.to_lowercase().as_str() {
        "sleep" => Ok(TimerAction::Sleep),
        "shutdown" | "shut down" => Ok(TimerAction::Shutdown),
        "restart" | "reboot" => Ok(TimerAction::Restart),
        "lock" => Ok(TimerAction::Lock),
        _ => Err(TimerError::Parse(format!("Unknown action: {s}"))),
    }
}

/// Parses a duration from amount and unit strings.
#[allow(clippy::cast_possible_truncation)]
fn parse_duration(amount_str: &str, unit_str: &str) -> Result<Duration> {
    let amount: f64 = amount_str
        .parse()
        .map_err(|e| TimerError::Parse(format!("Invalid duration amount: {e}")))?;

    let unit = unit_str.to_lowercase();

    let seconds = match unit.as_str() {
        "s" | "sec" | "second" | "seconds" => amount,
        "m" | "min" | "minute" | "minutes" => amount * 60.0,
        "h" | "hr" | "hour" | "hours" => amount * 3600.0,
        _ => return Err(TimerError::Parse(format!("Unknown time unit: {unit_str}"))),
    };

    Duration::try_seconds(seconds as i64)
        .ok_or_else(|| TimerError::Parse("Duration out of range".to_string()))
}

/// Parses a time string into a DateTime.
fn parse_time(caps: &regex::Captures) -> Result<DateTime<Utc>> {
    let hour_str = &caps["hour"];
    let minute_str = caps.name("minute").map_or("0", |m| m.as_str());
    let period = caps.name("period").map(|m| m.as_str().to_lowercase());

    let mut hour: u32 = hour_str
        .parse()
        .map_err(|e| TimerError::Parse(format!("Invalid hour: {e}")))?;
    let minute: u32 = minute_str
        .parse()
        .map_err(|e| TimerError::Parse(format!("Invalid minute: {e}")))?;

    // Validate ranges
    if hour > 23 {
        return Err(TimerError::Parse(format!("Invalid hour: {hour}")));
    }
    if minute > 59 {
        return Err(TimerError::Parse(format!("Invalid minute: {minute}")));
    }

    // Handle AM/PM
    if let Some(period) = period {
        match period.as_str() {
            "am" => {
                if hour == 12 {
                    hour = 0;
                }
            },
            "pm" => {
                if hour != 12 {
                    hour += 12;
                }
            },
            _ => {},
        }
    }

    // Create time for today
    let now = Local::now();
    let target_time = NaiveTime::from_hms_opt(hour, minute, 0)
        .ok_or_else(|| TimerError::Parse("Invalid time".to_string()))?;

    let mut target_datetime = now
        .date_naive()
        .and_time(target_time)
        .and_local_timezone(Local)
        .single()
        .ok_or_else(|| TimerError::Parse("Ambiguous local time".to_string()))?;

    // If the time has already passed today, schedule for tomorrow
    if target_datetime <= now {
        target_datetime += Duration::days(1);
    }

    Ok(target_datetime.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_with_action() {
        let expr = parse_timer_expression("sleep in 30 minutes").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("shutdown in 1 hour").unwrap();
        assert_eq!(expr.action, TimerAction::Shutdown);

        let expr = parse_timer_expression("restart in 2 hours").unwrap();
        assert_eq!(expr.action, TimerAction::Restart);

        let expr = parse_timer_expression("lock in 5 minutes").unwrap();
        assert_eq!(expr.action, TimerAction::Lock);
    }

    #[test]
    fn test_parse_duration_only() {
        let expr = parse_timer_expression("30 minutes").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep); // Default action

        let expr = parse_timer_expression("1h").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("30m").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("1.5 hours").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);
    }

    #[test]
    fn test_parse_duration_units() {
        // Seconds
        let expr = parse_timer_expression("30 seconds").unwrap();
        let remaining = expr.execute_at - Utc::now();
        assert!(remaining.num_seconds() >= 29 && remaining.num_seconds() <= 31);

        // Minutes
        let expr = parse_timer_expression("5 min").unwrap();
        let remaining = expr.execute_at - Utc::now();
        assert!(remaining.num_minutes() >= 4 && remaining.num_minutes() <= 5);

        // Hours
        let expr = parse_timer_expression("2 hr").unwrap();
        let remaining = expr.execute_at - Utc::now();
        assert!(remaining.num_hours() >= 1 && remaining.num_hours() <= 2);
    }

    #[test]
    fn test_parse_time_12h() {
        // This test might fail if run at the exact specified time,
        // but should pass in general cases
        let expr = parse_timer_expression("10pm").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("at 2:30 pm").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("sleep at 11pm").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);
    }

    #[test]
    fn test_parse_time_24h() {
        let expr = parse_timer_expression("22:00").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("shutdown at 23:30").unwrap();
        assert_eq!(expr.action, TimerAction::Shutdown);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_timer_expression("invalid").is_err());
        assert!(parse_timer_expression("sleep in 30 invalid").is_err());
        assert!(parse_timer_expression("25:00").is_err()); // Invalid hour
        assert!(parse_timer_expression("10:60").is_err()); // Invalid minute
    }

    #[test]
    fn test_case_insensitive() {
        let expr = parse_timer_expression("SLEEP IN 30 MINUTES").unwrap();
        assert_eq!(expr.action, TimerAction::Sleep);

        let expr = parse_timer_expression("ShUtDoWn In 1 HoUr").unwrap();
        assert_eq!(expr.action, TimerAction::Shutdown);
    }
}
