//! Error types for the calculator module.

use thiserror::Error;

/// Result type alias for calculator operations.
pub type Result<T> = std::result::Result<T, CalculatorError>;

/// Errors that can occur during calculator operations.
#[derive(Error, Debug)]
pub enum CalculatorError {
    /// Expression is empty.
    #[error("expression is empty")]
    EmptyExpression,

    /// Expression could not be parsed.
    #[error("could not parse expression: {0}")]
    ParseError(String),

    /// Math evaluation failed.
    #[error("evaluation error: {0}")]
    EvaluationError(String),

    /// Division by zero.
    #[error("division by zero")]
    DivisionByZero,

    /// Number overflow.
    #[error("number overflow")]
    Overflow,

    /// Unknown function in expression.
    #[error("unknown function: {0}")]
    UnknownFunction(String),

    /// Currency not supported.
    #[error("unsupported currency: {0}")]
    UnsupportedCurrency(String),

    /// Currency rate not available.
    #[error("rate not available for {from} to {to}")]
    RateNotAvailable { from: String, to: String },

    /// Currency rates are stale.
    #[error("currency rates are stale (last updated: {0})")]
    StaleRates(String),

    /// Unit not supported.
    #[error("unsupported unit: {0}")]
    UnsupportedUnit(String),

    /// Incompatible units for conversion.
    #[error("cannot convert {from} to {to} (incompatible unit types)")]
    IncompatibleUnits { from: String, to: String },

    /// Date parsing failed.
    #[error("could not parse date: {0}")]
    DateParseError(String),

    /// Timezone not found.
    #[error("unknown timezone or city: {0}")]
    UnknownTimezone(String),

    /// Invalid time expression.
    #[error("invalid time expression: {0}")]
    InvalidTimeExpression(String),

    /// Network error when fetching rates.
    #[error("network error: {0}")]
    NetworkError(String),

    /// API error from external service.
    #[error("API error from {service}: {message}")]
    ApiError { service: String, message: String },

    /// Database error.
    #[error("database error: {0}")]
    DatabaseError(String),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Internal error.
    #[error("internal error: {0}")]
    InternalError(String),
}

impl From<evalexpr::EvalexprError> for CalculatorError {
    fn from(err: evalexpr::EvalexprError) -> Self {
        match &err {
            evalexpr::EvalexprError::DivisionError { .. } => Self::DivisionByZero,
            evalexpr::EvalexprError::FunctionIdentifierNotFound(name) => {
                Self::UnknownFunction(name.clone())
            },
            _ => Self::EvaluationError(err.to_string()),
        }
    }
}

impl From<rusqlite::Error> for CalculatorError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for CalculatorError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for CalculatorError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(format!("JSON parse error: {}", err))
    }
}

impl From<rust_decimal::Error> for CalculatorError {
    fn from(err: rust_decimal::Error) -> Self {
        Self::EvaluationError(format!("decimal error: {}", err))
    }
}

impl From<chrono::ParseError> for CalculatorError {
    fn from(err: chrono::ParseError) -> Self {
        Self::DateParseError(err.to_string())
    }
}

impl From<anyhow::Error> for CalculatorError {
    fn from(err: anyhow::Error) -> Self {
        Self::InternalError(err.to_string())
    }
}

impl From<tokio::task::JoinError> for CalculatorError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::InternalError(format!("task join error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CalculatorError::EmptyExpression;
        assert_eq!(err.to_string(), "expression is empty");

        let err = CalculatorError::UnsupportedCurrency("XYZ".to_string());
        assert_eq!(err.to_string(), "unsupported currency: XYZ");

        let err = CalculatorError::IncompatibleUnits {
            from: "km".to_string(),
            to: "kg".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "cannot convert km to kg (incompatible unit types)"
        );
    }
}
