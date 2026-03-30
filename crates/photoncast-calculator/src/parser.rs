//! Expression parsing and routing.
//!
//! This module parses user input and determines what type of calculation
//! to perform (math, currency, unit conversion, or date/time).

use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;
use tracing::debug;

use crate::currency::{CryptoCurrency, FIAT_CURRENCIES};
use crate::datetime::TIMEZONE_KEYWORDS;
use crate::error::{CalculatorError, Result};
use crate::units::UNIT_PATTERNS;

/// Regex patterns for expression parsing.
static CURRENCY_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "100 usd in eur", "100 usd to eur", "$100 to euros"
    // Also: "0.5 btc in usd", "100$ in €"
    Regex::new(
        r"(?ix)
        ^
        (?:\$|€|£|¥|₹|₿|Ξ)?  # Optional currency symbol at start
        \s*
        ([\d,]+(?:\.\d+)?)    # Amount (capture group 1)
        \s*
        (?:\$|€|£|¥|₹|₿|Ξ)?  # Optional currency symbol after number
        \s*
        ([a-zA-Z]{2,10})      # From currency (capture group 2)
        \s+
        (?:in|to|as|into|->|→|=>)  # Conversion keyword
        \s+
        (?:\$|€|£|¥|₹|₿|Ξ)?  # Optional currency symbol
        \s*
        ([a-zA-Z]{2,10})      # To currency (capture group 3)
        \s*
        $
    ",
    )
    .unwrap()
});

static SYMBOL_CURRENCY_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "$100 in eur", "€50 to usd"
    Regex::new(
        r"(?ix)
        ^
        ([\$€£¥₹₿Ξ])          # Currency symbol (capture group 1)
        \s*
        ([\d,]+(?:\.\d+)?)    # Amount (capture group 2)
        \s+
        (?:in|to|as|into|->|→|=>)
        \s+
        ([a-zA-Z]{2,10})      # To currency (capture group 3)
        \s*
        $
    ",
    )
    .unwrap()
});

static CODE_ONLY_CURRENCY_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "usd to eur", "btc in usd"
    Regex::new(
        r"(?ix)
        ^
        ([a-zA-Z]{2,10})      # From currency (capture group 1)
        \s+
        (?:in|to|as|into|->|→|=>)
        \s+
        ([a-zA-Z]{2,10})      # To currency (capture group 2)
        \s*
        $
    ",
    )
    .unwrap()
});

static SYMBOL_ONLY_CURRENCY_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "$ to eur", "€ to usd"
    Regex::new(
        r"(?ix)
        ^
        ([\$€£¥₹₿Ξ])          # Currency symbol (capture group 1)
        \s*
        (?:in|to|as|into|->|→|=>)
        \s+
        ([a-zA-Z]{2,10})      # To currency (capture group 2)
        \s*
        $
    ",
    )
    .unwrap()
});

static UNIT_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "5 km to miles", "100 f in c", "32 fahrenheit to celsius"
    Regex::new(
        r"(?ix)
        ^
        (-?[\d,]+(?:\.\d+)?)  # Amount (capture group 1)
        \s*
        ([a-zA-Z°/²³]+)       # From unit (capture group 2)
        \s+
        (?:in|to|as|into|->|→|=>)
        \s+
        ([a-zA-Z°/²³]+)       # To unit (capture group 3)
        \s*
        $
    ",
    )
    .unwrap()
});

static PERCENTAGE_OF_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: "32% of 500", "15 percent of 200"
    Regex::new(
        r"(?ix)
        ^
        ([\d.]+)              # Percentage (capture group 1)
        \s*
        (?:%|percent)
        \s+
        of
        \s+
        ([\d.]+)              # Base value (capture group 2)
        \s*
        $
    ",
    )
    .unwrap()
});

static TIME_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"\d{1,2}(?::\d{2})?\s*(?:am|pm)|^\d{1,2}:\d{2}$").unwrap()
});

static DATE_KEYWORDS: std::sync::LazyLock<Vec<&'static str>> = std::sync::LazyLock::new(|| {
    vec![
        "today",
        "tomorrow",
        "yesterday",
        "now",
        "time",
        "date",
        "day",
        "days",
        "week",
        "weeks",
        "month",
        "months",
        "year",
        "years",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "ago",
        "from now",
        "until",
        "since",
        "before",
        "after",
        "next",
        "last",
        "this",
        "in ",
    ]
});

/// Parsed expression with its type and components.
#[derive(Debug, Clone)]
pub struct ParsedExpression {
    /// The original expression.
    pub original: String,
    /// The type of expression.
    pub kind: ExpressionKind,
}

/// The type of expression and its components.
#[derive(Debug, Clone)]
pub enum ExpressionKind {
    /// Mathematical expression.
    Math {
        /// The preprocessed expression for evaluation.
        expr: String,
    },
    /// Currency conversion.
    Currency {
        /// Amount to convert.
        amount: Decimal,
        /// Source currency code.
        from: String,
        /// Target currency code.
        to: String,
    },
    /// Unit conversion.
    Unit {
        /// Amount to convert.
        amount: f64,
        /// Source unit.
        from_unit: String,
        /// Target unit.
        to_unit: String,
    },
    /// Date/time calculation.
    DateTime {
        /// The date/time query.
        query: String,
    },
    /// Percentage calculation.
    Percentage {
        /// Base value.
        base: f64,
        /// Percentage to calculate.
        percentage: f64,
    },
}

/// Expression parser that routes input to the appropriate evaluator.
#[derive(Debug)]
pub struct ExpressionParser {
    /// Whether to enable currency parsing.
    enable_currency: bool,
    /// Whether to enable unit parsing.
    enable_units: bool,
    /// Whether to enable date/time parsing.
    enable_datetime: bool,
}

impl ExpressionParser {
    /// Creates a new expression parser with all features enabled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enable_currency: true,
            enable_units: true,
            enable_datetime: true,
        }
    }

    /// Parses an expression and determines its type.
    pub fn parse(&self, input: &str) -> Result<ParsedExpression> {
        let input = input.trim();

        if input.is_empty() {
            return Err(CalculatorError::EmptyExpression);
        }

        let lower = input.to_lowercase();

        // Try percentage pattern first (most specific)
        if let Some(parsed) = Self::try_parse_percentage(input) {
            return Ok(parsed);
        }

        // Try currency conversion
        if self.enable_currency {
            if let Some(parsed) = Self::try_parse_currency(input) {
                return Ok(parsed);
            }
        }

        // Try unit conversion
        if self.enable_units {
            if let Some(parsed) = Self::try_parse_unit(input) {
                return Ok(parsed);
            }
        }

        // Try date/time patterns
        if self.enable_datetime && Self::looks_like_datetime(&lower) {
            return Ok(ParsedExpression {
                original: input.to_string(),
                kind: ExpressionKind::DateTime {
                    query: input.to_string(),
                },
            });
        }

        // Default to math expression
        Ok(ParsedExpression {
            original: input.to_string(),
            kind: ExpressionKind::Math {
                expr: input.to_string(),
            },
        })
    }

    /// Tries to parse a percentage expression.
    fn try_parse_percentage(input: &str) -> Option<ParsedExpression> {
        if let Some(caps) = PERCENTAGE_OF_PATTERN.captures(input) {
            let percentage: f64 = caps.get(1)?.as_str().parse().ok()?;
            let base: f64 = caps.get(2)?.as_str().parse().ok()?;

            debug!("Parsed percentage: {}% of {}", percentage, base);

            return Some(ParsedExpression {
                original: input.to_string(),
                kind: ExpressionKind::Percentage { base, percentage },
            });
        }
        None
    }

    /// Tries to parse a currency conversion expression.
    fn try_parse_currency(input: &str) -> Option<ParsedExpression> {
        // Try the main currency pattern
        if let Some(caps) = CURRENCY_PATTERN.captures(input) {
            let amount_str = caps.get(1)?.as_str().replace(',', "");
            let amount = Decimal::from_str(&amount_str).ok()?;
            let from = caps.get(2)?.as_str().to_uppercase();
            let to = caps.get(3)?.as_str().to_uppercase();

            // Validate currencies
            if Self::is_valid_currency(&from) && Self::is_valid_currency(&to) {
                debug!("Parsed currency: {} {} to {}", amount, from, to);
                return Some(ParsedExpression {
                    original: input.to_string(),
                    kind: ExpressionKind::Currency { amount, from, to },
                });
            }
        }

        // Try symbol-based pattern
        if let Some(caps) = SYMBOL_CURRENCY_PATTERN.captures(input) {
            let symbol = caps.get(1)?.as_str();
            let amount_str = caps.get(2)?.as_str().replace(',', "");
            let amount = Decimal::from_str(&amount_str).ok()?;
            let to = caps.get(3)?.as_str().to_uppercase();

            let from = symbol_to_currency(symbol)?;

            if Self::is_valid_currency(&to) {
                debug!("Parsed currency (symbol): {} {} to {}", amount, from, to);
                return Some(ParsedExpression {
                    original: input.to_string(),
                    kind: ExpressionKind::Currency { amount, from, to },
                });
            }
        }

        None
    }

    /// Tries to parse a unit conversion expression.
    fn try_parse_unit(input: &str) -> Option<ParsedExpression> {
        if let Some(caps) = UNIT_PATTERN.captures(input) {
            let amount_str = caps.get(1)?.as_str().replace(',', "");
            let amount: f64 = amount_str.parse().ok()?;
            let from_unit = caps.get(2)?.as_str().to_lowercase();
            let to_unit = caps.get(3)?.as_str().to_lowercase();

            // Check if these look like valid units (not currencies)
            if Self::is_valid_unit(&from_unit) && Self::is_valid_unit(&to_unit) {
                // Make sure they're not currencies being misidentified
                if !Self::is_valid_currency(&from_unit.to_uppercase()) {
                    debug!("Parsed unit: {} {} to {}", amount, from_unit, to_unit);
                    return Some(ParsedExpression {
                        original: input.to_string(),
                        kind: ExpressionKind::Unit {
                            amount,
                            from_unit,
                            to_unit,
                        },
                    });
                }
            }
        }

        // Try code-only pattern with implicit amount of 1
        if let Some(caps) = CODE_ONLY_CURRENCY_PATTERN.captures(input) {
            let from = caps.get(1)?.as_str().to_uppercase();
            let to = caps.get(2)?.as_str().to_uppercase();

            if Self::is_valid_currency(&from) && Self::is_valid_currency(&to) {
                debug!("Parsed currency (implicit): {} to {}", from, to);
                return Some(ParsedExpression {
                    original: input.to_string(),
                    kind: ExpressionKind::Currency {
                        amount: Decimal::ONE,
                        from,
                        to,
                    },
                });
            }
        }

        // Try symbol-only pattern with implicit amount of 1
        if let Some(caps) = SYMBOL_ONLY_CURRENCY_PATTERN.captures(input) {
            let symbol = caps.get(1)?.as_str();
            let to = caps.get(2)?.as_str().to_uppercase();
            let from = symbol_to_currency(symbol)?;

            if Self::is_valid_currency(&to) {
                debug!("Parsed currency (symbol implicit): {} to {}", from, to);
                return Some(ParsedExpression {
                    original: input.to_string(),
                    kind: ExpressionKind::Currency {
                        amount: Decimal::ONE,
                        from,
                        to,
                    },
                });
            }
        }
        None
    }

    /// Checks if the input looks like a date/time expression.
    fn looks_like_datetime(input: &str) -> bool {
        let lower = input.to_lowercase();

        // Check for date keywords
        for keyword in DATE_KEYWORDS.iter() {
            if lower.contains(keyword) {
                return true;
            }
        }

        // Check for timezone keywords
        for keyword in TIMEZONE_KEYWORDS.iter() {
            if lower.contains(keyword) {
                return true;
            }
        }

        // Check for time patterns like "5pm", "14:00"
        if TIME_PATTERN.is_match(&lower) {
            return true;
        }

        false
    }

    /// Checks if a string is a valid currency code.
    fn is_valid_currency(code: &str) -> bool {
        let upper = code.to_uppercase();
        FIAT_CURRENCIES.contains(&upper.as_str()) || CryptoCurrency::from_code(&upper).is_some()
    }

    /// Checks if a string is a valid unit.
    fn is_valid_unit(unit: &str) -> bool {
        let lower = unit.to_lowercase();
        UNIT_PATTERNS.contains_key(lower.as_str())
    }
}

impl Default for ExpressionParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a currency symbol to its code.
fn symbol_to_currency(symbol: &str) -> Option<String> {
    match symbol {
        "$" => Some("USD".to_string()),
        "€" => Some("EUR".to_string()),
        "£" => Some("GBP".to_string()),
        "¥" => Some("JPY".to_string()), // Could also be CNY, default to JPY
        "₹" => Some("INR".to_string()),
        "₿" => Some("BTC".to_string()),
        "Ξ" => Some("ETH".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ExpressionKind {
        let parser = ExpressionParser::new();
        parser.parse(input).expect("parse failed").kind
    }

    #[test]
    fn test_parse_math() {
        match parse("2 + 3") {
            ExpressionKind::Math { expr } => assert_eq!(expr, "2 + 3"),
            other => panic!("expected Math, got {:?}", other),
        }

        match parse("sin(pi/2)") {
            ExpressionKind::Math { expr } => assert_eq!(expr, "sin(pi/2)"),
            other => panic!("expected Math, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_currency() {
        match parse("100 usd in eur") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::from(100));
                assert_eq!(from, "USD");
                assert_eq!(to, "EUR");
            },
            other => panic!("expected Currency, got {:?}", other),
        }

        match parse("0.5 btc to usd") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::from_str("0.5").unwrap());
                assert_eq!(from, "BTC");
                assert_eq!(to, "USD");
            },
            other => panic!("expected Currency, got {:?}", other),
        }

        match parse("$100 to eur") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::from(100));
                assert_eq!(from, "USD");
                assert_eq!(to, "EUR");
            },
            other => panic!("expected Currency, got {:?}", other),
        }

        match parse("usd to gbp") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::ONE);
                assert_eq!(from, "USD");
                assert_eq!(to, "GBP");
            },
            other => panic!("expected Currency, got {:?}", other),
        }

        match parse("€ to usd") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::ONE);
                assert_eq!(from, "EUR");
                assert_eq!(to, "USD");
            },
            other => panic!("expected Currency, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_unit() {
        match parse("5 km to miles") {
            ExpressionKind::Unit {
                amount,
                from_unit,
                to_unit,
            } => {
                assert!((amount - 5.0).abs() < 1e-10);
                assert_eq!(from_unit, "km");
                assert_eq!(to_unit, "miles");
            },
            other => panic!("expected Unit, got {:?}", other),
        }

        match parse("100 f to c") {
            ExpressionKind::Unit {
                amount,
                from_unit,
                to_unit,
            } => {
                assert!((amount - 100.0).abs() < 1e-10);
                assert_eq!(from_unit, "f");
                assert_eq!(to_unit, "c");
            },
            other => panic!("expected Unit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_percentage() {
        match parse("32% of 500") {
            ExpressionKind::Percentage { base, percentage } => {
                assert!((base - 500.0).abs() < 1e-10);
                assert!((percentage - 32.0).abs() < 1e-10);
            },
            other => panic!("expected Percentage, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_datetime() {
        match parse("days until dec 25") {
            ExpressionKind::DateTime { query } => {
                assert_eq!(query, "days until dec 25");
            },
            other => panic!("expected DateTime, got {:?}", other),
        }

        match parse("time in tokyo") {
            ExpressionKind::DateTime { query } => {
                assert_eq!(query, "time in tokyo");
            },
            other => panic!("expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_currency_with_commas() {
        match parse("1,000,000 usd to eur") {
            ExpressionKind::Currency { amount, from, to } => {
                assert_eq!(amount, Decimal::from(1_000_000));
                assert_eq!(from, "USD");
                assert_eq!(to, "EUR");
            },
            other => panic!("expected Currency, got {:?}", other),
        }
    }
}
