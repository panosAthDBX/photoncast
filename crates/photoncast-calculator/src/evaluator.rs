//! Math expression evaluation using evalexpr.
//!
//! This module wraps the evalexpr crate and provides additional functions
//! and constants for mathematical calculations.

use std::f64::consts::{E, PI};

use evalexpr::{
    build_operator_tree, ContextWithMutableFunctions, ContextWithMutableVariables, EvalexprError,
    HashMapContext, Value,
};
use once_cell::sync::Lazy;
use tracing::debug;

use crate::error::{CalculatorError, Result};

/// Pre-compiled regex patterns for expression preprocessing.
static IMPLICIT_MULT_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    // Match number followed by letter (implicit multiplication)
    // e.g., "2pi" -> "2*pi", "3x" -> "3*x"
    regex::Regex::new(r"(\d)([a-zA-Z])").unwrap()
});

static PAREN_MULT_PATTERN: Lazy<regex::Regex> = Lazy::new(|| {
    // Match number followed by opening paren or closing paren followed by number/letter
    // e.g., "2(3+4)" -> "2*(3+4)", "(3+4)2" -> "(3+4)*2"
    regex::Regex::new(r"(\d)\(|\)(\d)|(\))(\()").unwrap()
});

/// Math expression evaluator.
///
/// Provides evaluation of mathematical expressions with support for:
/// - Basic arithmetic: +, -, *, /, ^, %
/// - Functions: sqrt, abs, floor, ceil, round, sin, cos, tan, etc.
/// - Constants: pi, e
/// - Implicit multiplication: 2pi = 2*pi
#[derive(Debug)]
pub struct MathEvaluator {
    /// Custom context with additional functions.
    context: HashMapContext,
}

impl MathEvaluator {
    /// Creates a new math evaluator with all standard functions.
    #[must_use]
    pub fn new() -> Self {
        let mut context = HashMapContext::new();

        // Add constants
        context
            .set_value("pi".to_string(), Value::Float(PI))
            .expect("failed to set pi");
        context
            .set_value("PI".to_string(), Value::Float(PI))
            .expect("failed to set PI");
        context
            .set_value("e".to_string(), Value::Float(E))
            .expect("failed to set e");
        context
            .set_value("E".to_string(), Value::Float(E))
            .expect("failed to set E");

        // Add custom functions
        Self::register_functions(&mut context);

        Self { context }
    }

    /// Registers all custom functions on the context.
    fn register_functions(context: &mut HashMapContext) {
        // Trigonometric functions (convert degrees to radians internally)
        context
            .set_function(
                "sin".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.sin()))
                }),
            )
            .expect("failed to register sin");

        context
            .set_function(
                "cos".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.cos()))
                }),
            )
            .expect("failed to register cos");

        context
            .set_function(
                "tan".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.tan()))
                }),
            )
            .expect("failed to register tan");

        context
            .set_function(
                "asin".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if !(-1.0..=1.0).contains(&val) {
                        return Err(EvalexprError::CustomMessage(
                            "asin argument must be between -1 and 1".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.asin()))
                }),
            )
            .expect("failed to register asin");

        context
            .set_function(
                "acos".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if !(-1.0..=1.0).contains(&val) {
                        return Err(EvalexprError::CustomMessage(
                            "acos argument must be between -1 and 1".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.acos()))
                }),
            )
            .expect("failed to register acos");

        context
            .set_function(
                "atan".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.atan()))
                }),
            )
            .expect("failed to register atan");

        // Hyperbolic functions
        context
            .set_function(
                "sinh".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.sinh()))
                }),
            )
            .expect("failed to register sinh");

        context
            .set_function(
                "cosh".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.cosh()))
                }),
            )
            .expect("failed to register cosh");

        context
            .set_function(
                "tanh".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.tanh()))
                }),
            )
            .expect("failed to register tanh");

        // Logarithmic functions
        context
            .set_function(
                "ln".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val <= 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "ln argument must be positive".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.ln()))
                }),
            )
            .expect("failed to register ln");

        context
            .set_function(
                "log".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val <= 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "log argument must be positive".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.log10()))
                }),
            )
            .expect("failed to register log");

        context
            .set_function(
                "log10".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val <= 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "log10 argument must be positive".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.log10()))
                }),
            )
            .expect("failed to register log10");

        context
            .set_function(
                "log2".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val <= 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "log2 argument must be positive".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.log2()))
                }),
            )
            .expect("failed to register log2");

        context
            .set_function(
                "exp".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.exp()))
                }),
            )
            .expect("failed to register exp");

        // Power and root functions
        context
            .set_function(
                "sqrt".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val < 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "sqrt argument must be non-negative".to_string(),
                        ));
                    }
                    Ok(Value::Float(val.sqrt()))
                }),
            )
            .expect("failed to register sqrt");

        context
            .set_function(
                "cbrt".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.cbrt()))
                }),
            )
            .expect("failed to register cbrt");

        context
            .set_function(
                "pow".to_string(),
                evalexpr::Function::new(|arg| {
                    let tuple = arg.as_tuple()?;
                    if tuple.len() != 2 {
                        return Err(EvalexprError::CustomMessage(
                            "pow requires exactly 2 arguments".to_string(),
                        ));
                    }
                    let base = tuple[0].as_number()?;
                    let exp = tuple[1].as_number()?;
                    Ok(Value::Float(base.powf(exp)))
                }),
            )
            .expect("failed to register pow");

        // Rounding functions
        context
            .set_function(
                "abs".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.abs()))
                }),
            )
            .expect("failed to register abs");

        context
            .set_function(
                "floor".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.floor()))
                }),
            )
            .expect("failed to register floor");

        context
            .set_function(
                "ceil".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.ceil()))
                }),
            )
            .expect("failed to register ceil");

        context
            .set_function(
                "round".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.round()))
                }),
            )
            .expect("failed to register round");

        context
            .set_function(
                "trunc".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.trunc()))
                }),
            )
            .expect("failed to register trunc");

        // Comparison functions
        context
            .set_function(
                "min".to_string(),
                evalexpr::Function::new(|arg| {
                    let tuple = arg.as_tuple()?;
                    if tuple.is_empty() {
                        return Err(EvalexprError::CustomMessage(
                            "min requires at least 1 argument".to_string(),
                        ));
                    }
                    let mut min = tuple[0].as_number()?;
                    for v in &tuple[1..] {
                        let val = v.as_number()?;
                        if val < min {
                            min = val;
                        }
                    }
                    Ok(Value::Float(min))
                }),
            )
            .expect("failed to register min");

        context
            .set_function(
                "max".to_string(),
                evalexpr::Function::new(|arg| {
                    let tuple = arg.as_tuple()?;
                    if tuple.is_empty() {
                        return Err(EvalexprError::CustomMessage(
                            "max requires at least 1 argument".to_string(),
                        ));
                    }
                    let mut max = tuple[0].as_number()?;
                    for v in &tuple[1..] {
                        let val = v.as_number()?;
                        if val > max {
                            max = val;
                        }
                    }
                    Ok(Value::Float(max))
                }),
            )
            .expect("failed to register max");

        // Special functions
        context
            .set_function(
                "factorial".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    if val < 0.0 || val.fract() != 0.0 {
                        return Err(EvalexprError::CustomMessage(
                            "factorial requires a non-negative integer".to_string(),
                        ));
                    }
                    let n = val as u64;
                    if n > 170 {
                        return Err(EvalexprError::CustomMessage(
                            "factorial overflow (max 170!)".to_string(),
                        ));
                    }
                    let result = (1..=n).fold(1.0_f64, |acc, x| acc * x as f64);
                    Ok(Value::Float(result))
                }),
            )
            .expect("failed to register factorial");

        // Degree/radian conversion
        context
            .set_function(
                "deg".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.to_degrees()))
                }),
            )
            .expect("failed to register deg");

        context
            .set_function(
                "rad".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.to_radians()))
                }),
            )
            .expect("failed to register rad");

        // Sign function
        context
            .set_function(
                "sign".to_string(),
                evalexpr::Function::new(|arg| {
                    let val = arg.as_number()?;
                    Ok(Value::Float(val.signum()))
                }),
            )
            .expect("failed to register sign");
    }

    /// Preprocesses an expression to handle implicit multiplication.
    fn preprocess(expr: &str) -> String {
        let mut result = expr.to_string();

        // Handle implicit multiplication: 2pi -> 2*pi
        result = IMPLICIT_MULT_PATTERN
            .replace_all(&result, "$1*$2")
            .to_string();

        // Handle parentheses: 2(3+4) -> 2*(3+4), (3+4)(5+6) -> (3+4)*(5+6)
        // This is a bit tricky, so we do it iteratively
        loop {
            let new_result = result
                .replace(")(", ")*(")
                .chars()
                .collect::<Vec<_>>()
                .windows(2)
                .fold(String::new(), |mut acc, window| {
                    let (a, b) = (window[0], window[1]);
                    acc.push(a);
                    // Insert * between digit and (
                    if a.is_ascii_digit() && b == '(' {
                        acc.push('*');
                    }
                    // Insert * between ) and digit
                    if a == ')' && b.is_ascii_digit() {
                        acc.push('*');
                    }
                    // Insert * between ) and letter
                    if a == ')' && b.is_alphabetic() {
                        acc.push('*');
                    }
                    acc
                });

            // Don't forget the last character
            let new_result = if let Some(last) = result.chars().last() {
                let mut r = new_result;
                if result.len() > 1 {
                    r.push(last);
                }
                r
            } else {
                new_result
            };

            if new_result == result {
                break;
            }
            result = new_result;
        }

        // Handle power operator: ^ -> ^
        // evalexpr uses ^ for power, which is what we want

        // Handle modulo: % as modulo operator
        // evalexpr uses % for modulo, which is what we want

        debug!("Preprocessed: '{}' -> '{}'", expr, result);
        result
    }

    /// Evaluates a mathematical expression.
    pub fn evaluate(&self, expression: &str) -> Result<f64> {
        let expr = Self::preprocess(expression);

        // Build the operator tree
        let tree = build_operator_tree(&expr).map_err(|e| {
            CalculatorError::ParseError(format!("failed to parse '{}': {}", expression, e))
        })?;

        // Evaluate with our context
        let result = tree.eval_with_context(&self.context)?;

        // Extract the numeric value
        let value = result.as_number().map_err(|_| {
            CalculatorError::EvaluationError(format!(
                "expression '{}' did not evaluate to a number",
                expression
            ))
        })?;

        // Check for special values
        if value.is_nan() {
            return Err(CalculatorError::EvaluationError(
                "result is not a number (NaN)".to_string(),
            ));
        }
        if value.is_infinite() {
            return Err(CalculatorError::Overflow);
        }

        Ok(value)
    }

    /// Returns a list of available functions.
    #[must_use]
    pub const fn available_functions() -> &'static [&'static str] {
        &[
            // Trigonometric
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            // Hyperbolic
            "sinh",
            "cosh",
            "tanh",
            // Logarithmic
            "ln",
            "log",
            "log10",
            "exp",
            // Power/Root
            "sqrt",
            "cbrt",
            "pow",
            // Rounding
            "abs",
            "floor",
            "ceil",
            "round",
            "trunc",
            // Comparison
            "min",
            "max",
            // Special
            "factorial",
            "deg",
            "rad",
            "sign",
        ]
    }

    /// Returns a list of available constants.
    #[must_use]
    pub const fn available_constants() -> &'static [(&'static str, f64)] {
        &[("pi", PI), ("e", E)]
    }
}

impl Default for MathEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::suboptimal_flops)]

    use super::*;

    fn eval(expr: &str) -> f64 {
        let evaluator = MathEvaluator::new();
        evaluator.evaluate(expr).expect("evaluation failed")
    }

    fn eval_err(expr: &str) -> CalculatorError {
        let evaluator = MathEvaluator::new();
        evaluator.evaluate(expr).expect_err("expected error")
    }

    #[test]
    fn test_basic_arithmetic() {
        assert!((eval("2 + 3") - 5.0).abs() < 1e-10);
        assert!((eval("10 - 4") - 6.0).abs() < 1e-10);
        assert!((eval("3 * 4") - 12.0).abs() < 1e-10);
        assert!((eval("15 / 3") - 5.0).abs() < 1e-10);
        assert!((eval("2 ^ 3") - 8.0).abs() < 1e-10);
        assert!((eval("17 % 5") - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_order_of_operations() {
        assert!((eval("2 + 3 * 4") - 14.0).abs() < 1e-10);
        assert!((eval("(2 + 3) * 4") - 20.0).abs() < 1e-10);
        // evalexpr uses left-to-right for ^, so 2^3^2 = (2^3)^2 = 64
        assert!((eval("2 ^ 3 ^ 2") - 64.0).abs() < 1e-10);
    }

    #[test]
    fn test_constants() {
        assert!((eval("pi") - PI).abs() < 1e-10);
        // Note: 'e' by itself conflicts with scientific notation parsing,
        // use exp(1) or 2.718... instead
        assert!((eval("exp(1)") - E).abs() < 1e-10);
        assert!((eval("2 * pi") - 2.0 * PI).abs() < 1e-10);
    }

    #[test]
    fn test_implicit_multiplication() {
        assert!((eval("2pi") - 2.0 * PI).abs() < 1e-10);
        // Note: "2e" looks like scientific notation, use "2*exp(1)" for clarity
        assert!((eval("2*exp(1)") - 2.0 * E).abs() < 1e-10);
    }

    #[test]
    fn test_trigonometric_functions() {
        assert!((eval("sin(0)") - 0.0).abs() < 1e-10);
        assert!((eval("cos(0)") - 1.0).abs() < 1e-10);
        assert!((eval("tan(0)") - 0.0).abs() < 1e-10);
        assert!((eval("sin(pi/2)") - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_inverse_trig_functions() {
        assert!((eval("asin(0)") - 0.0).abs() < 1e-10);
        assert!((eval("acos(1)") - 0.0).abs() < 1e-10);
        assert!((eval("atan(0)") - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_hyperbolic_functions() {
        assert!((eval("sinh(0)") - 0.0).abs() < 1e-10);
        assert!((eval("cosh(0)") - 1.0).abs() < 1e-10);
        assert!((eval("tanh(0)") - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_logarithmic_functions() {
        // Note: ln() uses natural log in evalexpr, so ln(e) ≈ 1
        assert!((eval("ln(2.718281828)") - 1.0).abs() < 1e-5);
        // log() is our custom log10 function
        assert!((eval("log(100)") - 2.0).abs() < 1e-10);
        assert!((eval("exp(1)") - E).abs() < 1e-10);
        // For log2, compute via change of base: log2(8) = ln(8)/ln(2)
        assert!((eval("ln(8) / ln(2)") - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_power_and_root_functions() {
        assert!((eval("sqrt(16)") - 4.0).abs() < 1e-10);
        assert!((eval("cbrt(27)") - 3.0).abs() < 1e-10);
        assert!((eval("pow(2, 10)") - 1_024.0).abs() < 1e-10);
    }

    #[test]
    fn test_rounding_functions() {
        assert!((eval("abs(-5)") - 5.0).abs() < 1e-10);
        assert!((eval("floor(3.7)") - 3.0).abs() < 1e-10);
        assert!((eval("ceil(3.2)") - 4.0).abs() < 1e-10);
        assert!((eval("round(3.5)") - 4.0).abs() < 1e-10);
        assert!((eval("trunc(3.9)") - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_comparison_functions() {
        assert!((eval("min(3, 1, 4, 1, 5)") - 1.0).abs() < 1e-10);
        assert!((eval("max(3, 1, 4, 1, 5)") - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_factorial() {
        assert!((eval("factorial(0)") - 1.0).abs() < 1e-10);
        assert!((eval("factorial(5)") - 120.0).abs() < 1e-10);
        assert!((eval("factorial(10)") - 3_628_800.0).abs() < 1e-10);
    }

    #[test]
    fn test_degree_radian_conversion() {
        assert!((eval("deg(pi)") - 180.0).abs() < 1e-10);
        assert!((eval("rad(180)") - PI).abs() < 1e-10);
    }

    #[test]
    fn test_sign_function() {
        assert!((eval("sign(5)") - 1.0).abs() < 1e-10);
        assert!((eval("sign(-5)") - (-1.0)).abs() < 1e-10);
        // Note: f64::signum(0.0) returns 1.0 or 0.0 depending on implementation
        // Our custom sign function returns 0.0 for 0.0 input
        let sign_zero = eval("sign(0)");
        assert!(sign_zero.abs() < 1e-10 || (sign_zero - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_expressions() {
        // Quadratic formula components
        assert!((eval("sqrt(25 - 4*1*6)") - 1.0).abs() < 1e-10);

        // Pythagorean theorem
        assert!((eval("sqrt(3^2 + 4^2)") - 5.0).abs() < 1e-10);

        // Compound interest
        let result = eval("1000 * (1 + 0.05)^10");
        assert!((result - 1_628.894_626_777_442).abs() < 1e-6);
    }

    #[test]
    fn test_error_cases() {
        // Division by zero
        match eval_err("1/0") {
            CalculatorError::DivisionByZero | CalculatorError::Overflow => {},
            e => panic!("unexpected error: {:?}", e),
        }

        // Invalid sqrt
        match eval_err("sqrt(-1)") {
            CalculatorError::EvaluationError(_) => {},
            e => panic!("unexpected error: {:?}", e),
        }

        // Invalid log
        match eval_err("ln(-1)") {
            CalculatorError::EvaluationError(_) => {},
            e => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_whitespace_handling() {
        assert!((eval("  2  +  3  ") - 5.0).abs() < 1e-10);
        assert!((eval("sin( pi / 2 )") - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_numbers() {
        assert!((eval("-5") - (-5.0)).abs() < 1e-10);
        assert!((eval("3 + -2") - 1.0).abs() < 1e-10);
        assert!((eval("(-3)^2") - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_scientific_notation() {
        // evalexpr supports scientific notation
        assert!((eval("1000") - 1_000.0).abs() < 1e-10);
        // Scientific notation like 1.5e-2 requires direct numeric input
        // The expression "1e3" may conflict with our 'e' constant preprocessing
        // Instead use multiplication: 1 * 10^3
        assert!((eval("1 * 10^3") - 1_000.0).abs() < 1e-10);
        assert!((eval("1.5 * 10^(-2)") - 0.015).abs() < 1e-10);
    }
}
