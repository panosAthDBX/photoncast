#![recursion_limit = "512"]
//! PhotonCast Calculator Module
//!
//! This crate provides calculator functionality for PhotonCast, including:
//!
//! - Mathematical expression evaluation using `evalexpr`
//! - Currency conversion (fiat via frankfurter.app, crypto via CoinGecko)
//! - Unit conversion (length, weight, volume, temperature, data, speed)
//! - Date/time calculations with natural language parsing
//!
//! # Architecture
//!
//! The calculator is organized as follows:
//!
//! - [`Calculator`] - Main struct orchestrating all calculation types
//! - [`evaluator`] - Math expression evaluation with evalexpr
//! - [`currency`] - Currency and cryptocurrency conversion
//! - [`units`] - Unit conversion engine
//! - [`datetime`] - Date/time and timezone calculations
//! - [`parser`] - Expression parsing and routing
//! - [`cache`] - SQLite caching for currency rates
//!
//! # Example
//!
//! ```no_run
//! use photoncast_calculator::{Calculator, CalculatorResult};
//!
//! #[tokio::main]
//! async fn main() {
//!     let calculator = Calculator::new().await.expect("failed to create calculator");
//!     
//!     // Math expression
//!     let result = calculator.evaluate("2 + 3 * 4").await;
//!     
//!     // Currency conversion
//!     let result = calculator.evaluate("100 usd in eur").await;
//!     
//!     // Unit conversion
//!     let result = calculator.evaluate("5 km to miles").await;
//!     
//!     // Date calculation
//!     let result = calculator.evaluate("days until dec 25").await;
//! }
//! ```

#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
// Several fields/constants are retained for future use (schema versioning, timezone support, etc.)
#![allow(dead_code)]

pub mod cache;
pub mod currency;
pub mod datetime;
pub mod error;
pub mod evaluator;
pub mod parser;
pub mod units;

#[cfg(feature = "ui")]
pub mod commands;
#[cfg(feature = "ui")]
pub mod ui;

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use tokio::sync::Mutex as TokioMutex;
use tracing::{debug, info, warn};

pub use cache::RateCache;
pub use currency::{CryptoCurrency, CurrencyCode, CurrencyConverter};
pub use datetime::{DateTimeCalculator, TimezoneDatabase};
pub use error::{CalculatorError, Result};
pub use evaluator::MathEvaluator;
pub use parser::{ExpressionKind, ExpressionParser};
pub use units::{UnitCategory, UnitConverter};

/// Result of a calculation.
#[derive(Debug, Clone)]
pub struct CalculatorResult {
    /// The original expression that was evaluated.
    pub expression: String,
    /// The type of calculation performed.
    pub kind: CalculatorResultKind,
    /// The raw numeric value (for copying).
    pub raw_value: f64,
    /// Formatted display value.
    pub formatted_value: String,
    /// Additional details/context.
    pub details: Option<String>,
    /// When the data was last updated (for currency rates).
    pub last_updated: Option<DateTime<Utc>>,
    /// Time taken to evaluate.
    pub evaluation_time: Duration,
}

impl CalculatorResult {
    /// Creates a new math result.
    #[must_use]
    pub const fn math(expression: String, value: f64, formatted: String, time: Duration) -> Self {
        Self {
            expression,
            kind: CalculatorResultKind::Math,
            raw_value: value,
            formatted_value: formatted,
            details: None,
            last_updated: None,
            evaluation_time: time,
        }
    }

    /// Creates a new currency conversion result.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn currency(
        expression: String,
        value: Decimal,
        from: &str,
        to: &str,
        rate: Decimal,
        source: &str,
        last_updated: DateTime<Utc>,
        time: Duration,
    ) -> Self {
        let raw = value.to_string().parse::<f64>().unwrap_or(0.0);
        Self {
            expression,
            kind: CalculatorResultKind::Currency {
                from_currency: from.to_uppercase(),
                to_currency: to.to_uppercase(),
                rate,
                source: source.to_string(),
            },
            raw_value: raw,
            formatted_value: format_currency(value, to),
            details: Some(format!(
                "1 {} = {} {}",
                from.to_uppercase(),
                rate,
                to.to_uppercase()
            )),
            last_updated: Some(last_updated),
            evaluation_time: time,
        }
    }

    /// Creates a new unit conversion result.
    #[must_use]
    pub fn unit(
        expression: String,
        value: f64,
        from_unit: &str,
        to_unit: &str,
        time: Duration,
    ) -> Self {
        Self {
            expression,
            kind: CalculatorResultKind::Unit {
                from_unit: from_unit.to_string(),
                to_unit: to_unit.to_string(),
            },
            raw_value: value,
            formatted_value: format_unit_value(value, to_unit),
            details: None,
            last_updated: None,
            evaluation_time: time,
        }
    }

    /// Creates a new date/time result.
    #[must_use]
    pub const fn datetime(
        expression: String,
        formatted: String,
        details: Option<String>,
        time: Duration,
    ) -> Self {
        Self {
            expression,
            kind: CalculatorResultKind::DateTime,
            raw_value: 0.0,
            formatted_value: formatted,
            details,
            last_updated: None,
            evaluation_time: time,
        }
    }
}

/// The type of calculation performed.
#[derive(Debug, Clone)]
pub enum CalculatorResultKind {
    /// Mathematical expression.
    Math,
    /// Currency conversion.
    Currency {
        /// Source currency code.
        from_currency: String,
        /// Target currency code.
        to_currency: String,
        /// Exchange rate used.
        rate: Decimal,
        /// Data source (e.g., "frankfurter.app").
        source: String,
    },
    /// Unit conversion.
    Unit {
        /// Source unit.
        from_unit: String,
        /// Target unit.
        to_unit: String,
    },
    /// Date/time calculation.
    DateTime,
}

/// Main calculator struct that orchestrates all calculation types.
///
/// The calculator provides a unified interface for evaluating various
/// types of expressions including math, currency conversions, unit
/// conversions, and date/time calculations.
pub struct Calculator {
    /// Math expression evaluator.
    math_evaluator: MathEvaluator,
    /// Currency converter with rate caching.
    currency_converter: Arc<RwLock<CurrencyConverter>>,
    /// Unit converter.
    unit_converter: UnitConverter,
    /// Date/time calculator.
    datetime: DateTimeCalculator,
    /// Expression parser.
    parser: ExpressionParser,
    /// Rate cache for persistence.
    rate_cache: Arc<TokioMutex<RateCache>>,
    /// Last time rates were updated.
    last_rate_update: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Update interval for rates (default: 6 hours).
    rate_update_interval: Duration,
    /// Calculation history.
    history: Arc<RwLock<Vec<CalculatorResult>>>,
    /// Maximum history size.
    max_history_size: usize,
}

impl Calculator {
    /// Creates a new calculator with default configuration.
    ///
    /// This initializes all sub-systems and loads cached rates from SQLite.
    pub async fn new() -> Result<Self> {
        Self::with_config(CalculatorConfig::default()).await
    }

    /// Creates a new calculator with custom configuration.
    pub async fn with_config(config: CalculatorConfig) -> Result<Self> {
        info!("Initializing calculator...");

        let rate_cache = RateCache::new(&config.cache_path).await?;
        let rate_cache = Arc::new(TokioMutex::new(rate_cache));

        let mut currency_converter = CurrencyConverter::new();

        // Load cached rates
        let rates = rate_cache.lock().await.load_all_rates()?;
        currency_converter.load_rates(rates);
        info!("Loaded {} cached rates", currency_converter.rate_count());

        let calculator = Self {
            math_evaluator: MathEvaluator::new(),
            currency_converter: Arc::new(RwLock::new(currency_converter)),
            unit_converter: UnitConverter::new(),
            datetime: DateTimeCalculator::new(),
            parser: ExpressionParser::new(),
            rate_cache,
            last_rate_update: Arc::new(RwLock::new(None)),
            rate_update_interval: Duration::from_secs(config.rate_update_hours * 3_600),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: config.max_history_size,
        };

        Ok(calculator)
    }

    /// Evaluates an expression and returns the result.
    ///
    /// This is the main entry point for all calculations. The parser
    /// automatically detects the type of expression and routes it to
    /// the appropriate evaluator.
    pub async fn evaluate(&self, expression: &str) -> Result<CalculatorResult> {
        let start = Instant::now();
        let expression = expression.trim();

        if expression.is_empty() {
            return Err(CalculatorError::EmptyExpression);
        }

        debug!("Evaluating expression: {}", expression);

        // Parse the expression to determine its type
        let parsed = self.parser.parse(expression)?;

        let result = match parsed.kind {
            ExpressionKind::Math { expr } => {
                let value = self.math_evaluator.evaluate(&expr)?;
                let formatted = format_number(value);
                Ok(CalculatorResult::math(
                    expression.to_string(),
                    value,
                    formatted,
                    start.elapsed(),
                ))
            },
            ExpressionKind::Currency { amount, from, to } => {
                self.evaluate_currency(expression, amount, &from, &to, start)
                    .await
            },
            ExpressionKind::Unit {
                amount,
                from_unit,
                to_unit,
            } => {
                let result = self.unit_converter.convert(amount, &from_unit, &to_unit)?;
                Ok(CalculatorResult::unit(
                    expression.to_string(),
                    result,
                    &from_unit,
                    &to_unit,
                    start.elapsed(),
                ))
            },
            ExpressionKind::DateTime { query } => {
                let result = self.datetime.evaluate(&query)?;
                Ok(CalculatorResult::datetime(
                    expression.to_string(),
                    result.formatted,
                    result.details,
                    start.elapsed(),
                ))
            },
            ExpressionKind::Percentage { base, percentage } => {
                let value = base * (percentage / 100.0);
                let formatted = format_number(value);
                Ok(CalculatorResult::math(
                    expression.to_string(),
                    value,
                    formatted,
                    start.elapsed(),
                ))
            },
        };

        // Add to history if successful
        if let Ok(ref r) = result {
            self.add_to_history(r.clone());
        }

        result
    }

    /// Evaluates a currency conversion.
    async fn evaluate_currency(
        &self,
        expression: &str,
        amount: Decimal,
        from: &str,
        to: &str,
        start: Instant,
    ) -> Result<CalculatorResult> {
        // Check if we need to update rates
        self.maybe_update_rates(from, to).await?;

        let (result, rate, source, updated) =
            self.currency_converter.read().convert(amount, from, to)?;

        Ok(CalculatorResult::currency(
            expression.to_string(),
            result,
            from,
            to,
            rate,
            &source,
            updated,
            start.elapsed(),
        ))
    }

    /// Checks if rates need updating and fetches them if necessary.
    async fn maybe_update_rates(&self, from: &str, to: &str) -> Result<()> {
        let should_update = {
            let last_update = self.last_rate_update.read();
            last_update.map_or(true, |last| {
                let elapsed = Utc::now().signed_duration_since(last);
                let max_seconds =
                    i64::try_from(self.rate_update_interval.as_secs()).unwrap_or(i64::MAX);
                elapsed.num_seconds() > max_seconds
            })
        };

        if should_update {
            if let Err(err) = self.refresh_rates().await {
                let converter = self.currency_converter.read();
                let has_from = converter.has_rate(from);
                let has_to = converter.has_rate(to);
                drop(converter);
                if has_from && has_to {
                    tracing::warn!("Currency refresh failed, using cached rates: {}", err);
                } else {
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    /// Forces a refresh of all currency rates.
    pub async fn refresh_rates(&self) -> Result<()> {
        info!("Refreshing currency rates...");

        // Fetch fiat rates
        let fiat_rates = currency::fetch_fiat_rates().await?;
        let crypto_rates = match currency::fetch_crypto_rates().await {
            Ok(rates) => Some(rates),
            Err(err) => {
                tracing::warn!("Failed to fetch crypto rates: {}", err);
                None
            },
        };

        // Update converter
        {
            let mut converter = self.currency_converter.write();
            converter.update_fiat_rates(fiat_rates.clone());
            if let Some(rates) = &crypto_rates {
                converter.update_crypto_rates(rates.clone());
            }
        }

        // Persist to cache
        {
            let cache = self.rate_cache.lock().await;
            for (code, rate) in &fiat_rates {
                cache.store_rate(code, *rate, "fiat", "frankfurter.app")?;
            }
            if let Some(rates) = &crypto_rates {
                for (code, rate) in rates {
                    cache.store_rate(code, *rate, "crypto", "coingecko")?;
                }
            }
        }

        // Update timestamp
        {
            let mut last_update = self.last_rate_update.write();
            *last_update = Some(Utc::now());
        }

        let crypto_count = crypto_rates
            .as_ref()
            .map_or(0, std::collections::HashMap::len);
        info!(
            "Rates refreshed: {} fiat, {} crypto",
            fiat_rates.len(),
            crypto_count
        );

        Ok(())
    }

    /// Adds a result to the history.
    fn add_to_history(&self, result: CalculatorResult) {
        let mut history = self.history.write();
        history.insert(0, result);
        if history.len() > self.max_history_size {
            history.truncate(self.max_history_size);
        }
    }

    /// Returns the calculation history.
    pub fn history(&self) -> Vec<CalculatorResult> {
        self.history.read().clone()
    }

    /// Clears the calculation history.
    pub fn clear_history(&self) {
        self.history.write().clear();
    }

    /// Returns whether rates are available (from cache).
    pub fn has_cached_rates(&self) -> bool {
        self.currency_converter.read().has_rates()
    }

    /// Returns the last time rates were updated.
    pub fn last_rate_update(&self) -> Option<DateTime<Utc>> {
        *self.last_rate_update.read()
    }
}

/// Configuration for the calculator.
#[derive(Debug, Clone)]
pub struct CalculatorConfig {
    /// Path to the SQLite cache database.
    pub cache_path: std::path::PathBuf,
    /// How often to update currency rates (in hours).
    pub rate_update_hours: u64,
    /// Maximum number of history items to keep.
    pub max_history_size: usize,
}

impl Default for CalculatorConfig {
    fn default() -> Self {
        Self {
            cache_path: default_cache_path(),
            rate_update_hours: 6,
            max_history_size: 100,
        }
    }
}

/// Returns the default cache database path.
fn default_cache_path() -> std::path::PathBuf {
    directories::ProjectDirs::from("", "", "PhotonCast").map_or_else(
        || {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("Library/Application Support/PhotonCast/calculator_cache.db")
        },
        |dirs| dirs.data_dir().join("calculator_cache.db"),
    )
}

/// Formats a number for display.
fn format_number(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("{:.2e}", value)
    } else if value.fract().abs() < 1e-10 {
        // Integer value
        format!("{:.0}", value)
    } else if value.abs() < 0.0001 && value.abs() > 0.0 {
        // Small numbers - format and trim trailing zeros
        let s = format!("{:.10}", value);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    } else {
        // Remove trailing zeros
        let s = format!("{:.10}", value);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}

/// Formats a currency value for display.
fn format_currency(value: Decimal, currency: &str) -> String {
    let symbol = match currency.to_uppercase().as_str() {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" | "CNY" => "¥",
        "CHF" => "CHF ",
        "CAD" => "C$",
        "AUD" => "A$",
        "INR" => "₹",
        "KRW" => "₩",
        "BTC" => "₿",
        "ETH" => "Ξ",
        _ => "",
    };

    let formatted = if symbol.is_empty() {
        format!("{:.2} {}", value, currency.to_uppercase())
    } else if currency.to_uppercase() == "JPY" || currency.to_uppercase() == "KRW" {
        // No decimal places for these currencies
        format!("{}{:.0}", symbol, value)
    } else {
        format!("{}{:.2}", symbol, value)
    };

    formatted
}

/// Formats a unit value for display.
fn format_unit_value(value: f64, unit: &str) -> String {
    let formatted_value = format_number(value);
    format!("{} {}", formatted_value, unit)
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{Calculator, CalculatorConfig, CalculatorResult, CalculatorResultKind, Result};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(14.0), "14");
        assert_eq!(format_number(std::f64::consts::PI), "3.1415926536");
        assert_eq!(format_number(1_000_000_000.0), "1.00e9");
        // Small numbers get trailing zeros trimmed
        let small = format_number(0.00001);
        assert!(small == "0.00001" || small == "0.000010", "Got: {}", small);
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(Decimal::new(9_247, 2), "EUR"), "€92.47");
        assert_eq!(format_currency(Decimal::new(10_000, 2), "USD"), "$100.00");
        assert_eq!(format_currency(Decimal::new(1_000, 0), "JPY"), "¥1000");
    }
}
