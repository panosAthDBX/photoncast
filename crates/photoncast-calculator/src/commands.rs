//! Calculator commands for the launcher.
//!
//! This module provides calculator commands that integrate with the
//! PhotonCast launcher interface.

use std::sync::Arc;
use std::time::Duration;

#[allow(unused_imports)]
use gpui::{AppContext, Context, Model, SharedString};
use parking_lot::RwLock;
#[allow(unused_imports)]
use tokio::sync::mpsc;
use tracing::info;

use crate::currency::CurrencyCode;
use crate::units::UNIT_PATTERNS;
use crate::{Calculator, CalculatorResult, Result};

/// Calculator command state.
pub struct CalculatorCommand {
    /// The calculator instance.
    calculator: Arc<RwLock<Option<Arc<Calculator>>>>,
    /// Current result (if any).
    current_result: Option<CalculatorResult>,
    /// Whether the calculator is loading.
    is_loading: bool,
    /// Error message (if any).
    error: Option<String>,
    /// Input debounce duration.
    debounce_duration: Duration,
}

impl CalculatorCommand {
    /// Creates a new calculator command.
    #[must_use]
    pub fn new() -> Self {
        Self {
            calculator: Arc::new(RwLock::new(None)),
            current_result: None,
            is_loading: false,
            error: None,
            debounce_duration: Duration::from_millis(100),
        }
    }

    /// Initializes the calculator (async).
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing calculator...");

        let calc = Arc::new(Calculator::new().await?);

        {
            let mut guard = self.calculator.write();
            *guard = Some(calc);
        }

        info!("Calculator initialized");
        Ok(())
    }

    /// Returns whether the calculator is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.calculator.read().is_some()
    }

    /// Evaluates an expression.
    pub async fn evaluate(&mut self, expression: &str) -> Result<CalculatorResult> {
        let calc = {
            let calculator = self.calculator.read();
            calculator.as_ref().cloned().ok_or_else(|| {
                crate::error::CalculatorError::InternalError(
                    "calculator not initialized".to_string(),
                )
            })?
        };

        let result = calc.evaluate(expression).await?;
        self.current_result = Some(result.clone());
        self.error = None;

        Ok(result)
    }

    /// Returns the current result.
    #[must_use]
    pub const fn current_result(&self) -> Option<&CalculatorResult> {
        self.current_result.as_ref()
    }

    /// Returns the current error.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Clears the current result and error.
    pub fn clear(&mut self) {
        self.current_result = None;
        self.error = None;
    }

    /// Copies the formatted result to clipboard.
    #[must_use]
    pub fn formatted_value(&self) -> Option<&str> {
        self.current_result
            .as_ref()
            .map(|r| r.formatted_value.as_str())
    }

    /// Copies the raw result to clipboard.
    #[must_use]
    pub fn raw_value(&self) -> Option<f64> {
        self.current_result.as_ref().map(|r| r.raw_value)
    }

    /// Refreshes currency rates.
    pub async fn refresh_rates(&self) -> Result<()> {
        let calc = { self.calculator.read().as_ref().cloned() };
        if let Some(calc) = calc {
            calc.refresh_rates().await?;
        }
        Ok(())
    }

    /// Returns the calculation history.
    #[must_use]
    pub fn history(&self) -> Vec<CalculatorResult> {
        let calculator = self.calculator.read();
        calculator
            .as_ref()
            .map(|calc| calc.history())
            .unwrap_or_default()
    }

    /// Clears the calculation history.
    pub fn clear_history(&self) {
        let calculator = self.calculator.read();
        if let Some(calc) = calculator.as_ref() {
            calc.clear_history();
        }
    }
}

impl Default for CalculatorCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Checks if an input looks like a calculator expression.
///
/// This is used to determine whether to auto-activate the calculator
/// when the user types in the search bar.
#[must_use]
pub fn is_calculator_expression(input: &str) -> bool {
    let input = input.trim();

    if input.is_empty() {
        return false;
    }

    let first_char = input.chars().next().unwrap();
    let lower = input.to_lowercase();

    // Check for date/time keywords
    for keyword in &[
        "time in",
        "days until",
        "days ago",
        "weeks ago",
        "months ago",
        "in days",
        "in weeks",
        "next monday",
        "next tuesday",
        "next wednesday",
        "next thursday",
        "next friday",
        "next saturday",
        "next sunday",
    ] {
        if lower.contains(keyword) {
            return true;
        }
    }

    if contains_conversion_keyword(&lower) {
        let has_digit = input.chars().any(|c| c.is_ascii_digit());
        let symbol_count = currency_symbol_count(input);
        let currency_count = count_currency_tokens(input);
        let unit_count = count_unit_tokens(input);
        let looks_like_conversion =
            has_digit || symbol_count > 0 || currency_count > 0 || unit_count > 0;

        if looks_like_conversion {
            let currency_markers = currency_count + symbol_count;
            return currency_markers >= 2 || unit_count >= 2;
        }
    }

    // Check if it starts with a digit or mathematical symbol
    if first_char.is_ascii_digit() || first_char == '-' || first_char == '.' || first_char == '(' {
        // Check for mathematical operators
        if input.contains('+')
            || input.contains('-')
            || input.contains('*')
            || input.contains('/')
            || input.contains('^')
            || input.contains('%')
            || input.contains('(')
        {
            return true;
        }

        // Check for percentage
        if lower.contains("% of") || lower.contains("percent of") {
            return true;
        }
    }

    // Check for function calls
    for func in &[
        "sqrt",
        "sin",
        "cos",
        "tan",
        "log",
        "ln",
        "exp",
        "abs",
        "floor",
        "ceil",
        "round",
        "factorial",
        "pow",
        "min",
        "max",
    ] {
        if lower.starts_with(func) && lower.contains('(') {
            return true;
        }
    }

    false
}

fn contains_conversion_keyword(input: &str) -> bool {
    input.contains(" to ")
        || input.contains(" in ")
        || input.contains(" as ")
        || input.contains(" into ")
        || input.contains("->")
        || input.contains("=>")
        || input.contains('→')
}

fn currency_symbol_count(input: &str) -> usize {
    input
        .chars()
        .filter(|c| matches!(c, '$' | '€' | '£' | '¥' | '₹' | '₿' | 'Ξ'))
        .count()
}

fn count_currency_tokens(input: &str) -> usize {
    input
        .split_whitespace()
        .filter(|token| {
            let token = token.trim_matches(|c: char| !c.is_ascii_alphabetic());
            !token.is_empty() && CurrencyCode::parse(token).is_some()
        })
        .count()
}

fn count_unit_tokens(input: &str) -> usize {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let mut count = 0;

    for (i, token) in tokens.iter().enumerate() {
        let cleaned = token.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && !matches!(c, '°' | '²' | '³' | '/')
        });
        let lower = cleaned.to_lowercase();

        if lower.is_empty() || !UNIT_PATTERNS.contains_key(lower.as_str()) {
            continue;
        }

        // Special case: "in" can be the preposition or the unit "inches"
        // Only count "in" as a unit if it's preceded by a number (e.g., "5 in")
        // not when used as a preposition (e.g., "lock in 30 minutes")
        if lower == "in" {
            let prev_is_number = i > 0
                && tokens[i - 1]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit());
            if !prev_is_number {
                continue;
            }
        }

        count += 1;
    }

    count
}

/// Calculator actions that can be performed on a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculatorAction {
    /// Copy the formatted result (e.g., "€92.47").
    CopyFormatted,
    /// Copy the raw value (e.g., "92.47").
    CopyRaw,
    /// Refresh currency rates.
    RefreshRates,
    /// View calculation history.
    ViewHistory,
    /// Clear calculation history.
    ClearHistory,
}

impl CalculatorAction {
    /// Returns the display title for this action.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        match self {
            Self::CopyFormatted => "Copy Formatted",
            Self::CopyRaw => "Copy Raw Value",
            Self::RefreshRates => "Refresh Rates",
            Self::ViewHistory => "Calculator History",
            Self::ClearHistory => "Clear History",
        }
    }

    /// Returns the keyboard shortcut for this action.
    #[must_use]
    pub const fn shortcut(&self) -> Option<&'static str> {
        match self {
            Self::CopyFormatted => Some("⏎"),
            Self::CopyRaw => Some("⌘⏎"),
            Self::RefreshRates => Some("⌘R"),
            Self::ViewHistory | Self::ClearHistory => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_calculator_expression() {
        // Math expressions
        assert!(is_calculator_expression("2 + 3"));
        assert!(is_calculator_expression("2+3"));
        assert!(is_calculator_expression("100 * 5"));
        assert!(is_calculator_expression("(2 + 3) * 4"));
        assert!(is_calculator_expression("-5 + 3"));

        // Function calls
        assert!(is_calculator_expression("sqrt(16)"));
        assert!(is_calculator_expression("sin(pi/2)"));
        assert!(is_calculator_expression("log(100)"));

        // Conversions
        assert!(is_calculator_expression("100 usd to eur"));
        assert!(is_calculator_expression("usd to gbp"));
        assert!(is_calculator_expression("€ to usd"));
        assert!(is_calculator_expression("5 km in miles"));
        assert!(is_calculator_expression("100 f to c"));
        assert!(!is_calculator_expression("12 usd to g"));
        assert!(!is_calculator_expression("0.3 btc to u"));

        // Percentages
        assert!(is_calculator_expression("32% of 500"));

        // Date/time
        assert!(is_calculator_expression("time in tokyo"));
        assert!(is_calculator_expression("days until dec 25"));
        assert!(is_calculator_expression("next monday"));

        // Non-calculator expressions
        assert!(!is_calculator_expression("firefox"));
        assert!(!is_calculator_expression("open calculator"));
        assert!(!is_calculator_expression(""));

        // Timer expressions should NOT be calculator expressions
        assert!(!is_calculator_expression("lock in 30 minutes"));
        assert!(!is_calculator_expression("sleep in 30 minutes"));
        assert!(!is_calculator_expression("shutdown in 1 hour"));
        assert!(!is_calculator_expression("restart in 2 hours"));
    }
}
