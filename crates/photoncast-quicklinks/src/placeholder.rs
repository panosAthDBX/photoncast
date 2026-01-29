//! Placeholder parsing and substitution for dynamic quick links.
//!
//! This module provides a comprehensive placeholder system for URLs and text,
//! supporting various placeholder types, modifiers, and date/time features.
//!
//! # Placeholder Types
//!
//! - `{argument}` - Prompts for user input
//! - `{argument name="..."}` - Named argument (reusable across multiple occurrences)
//! - `{argument default="..."}` - Optional argument with default value
//! - `{argument options="a,b,c"}` - Dropdown selection from options
//! - `{clipboard}` - Current clipboard content
//! - `{selection}` - Currently selected text
//! - `{date}`, `{time}`, `{datetime}`, `{day}` - Date/time values
//! - `{uuid}` - Random UUID v4
//!
//! # Modifiers
//!
//! Modifiers are applied using pipe syntax: `{clipboard | trim | uppercase}`
//!
//! - `uppercase` - Convert to uppercase
//! - `lowercase` - Convert to lowercase
//! - `trim` - Remove leading/trailing whitespace
//! - `percent-encode` - URL encode the value
//! - `raw` - Disable auto percent-encoding
//!
//! # Date/Time Features
//!
//! - Offsets: `{date offset="+2d"}`, `{date offset="-1M"}`
//! - Formats: `{date format="yyyy-MM-dd"}`
//! - Combined: `{date format="yyyy-MM-dd" offset="+3d"}`
//!
//! # Example
//!
//! ```rust,ignore
//! use photoncast_quicklinks::placeholder::{parse_placeholders, substitute_placeholders};
//! use std::collections::HashMap;
//!
//! let url = "https://search.example.com?q={argument name=\"query\" | trim}";
//! let mut args = HashMap::new();
//! args.insert("query".to_string(), "  rust programming  ".to_string());
//!
//! let result = substitute_placeholders(url, &args, None, None)?;
//! assert_eq!(result, "https://search.example.com?q=rust%20programming");
//! ```

use chrono::{Datelike, Duration, Local};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

/// Error type for placeholder operations.
#[derive(Debug, Error)]
pub enum PlaceholderError {
    #[error("missing required argument: {name}")]
    MissingArgument { name: String },

    #[error("invalid placeholder syntax: {details}")]
    InvalidSyntax { details: String },

    #[error("invalid date offset: {offset}")]
    InvalidDateOffset { offset: String },

    #[error("invalid modifier: {modifier}")]
    InvalidModifier { modifier: String },

    #[error("clipboard content required but not provided")]
    ClipboardRequired,

    #[error("selection content required but not provided")]
    SelectionRequired,
}

/// Result type for placeholder operations.
pub type Result<T> = std::result::Result<T, PlaceholderError>;

/// Types of placeholders supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceholderKind {
    /// User-provided argument input
    Argument,
    /// Current clipboard content
    Clipboard,
    /// Currently selected text
    Selection,
    /// Current date
    Date,
    /// Current time
    Time,
    /// Current date and time
    DateTime,
    /// Current day of week
    Day,
    /// Random UUID v4
    Uuid,
}

impl PlaceholderKind {
    /// Parse a placeholder kind from a string.
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "argument" | "query" => Some(Self::Argument), // "query" for backward compatibility
            "clipboard" => Some(Self::Clipboard),
            "selection" => Some(Self::Selection),
            "date" => Some(Self::Date),
            "time" => Some(Self::Time),
            "datetime" => Some(Self::DateTime),
            "day" => Some(Self::Day),
            "uuid" => Some(Self::Uuid),
            _ => None,
        }
    }
}

/// Modifiers that can be applied to placeholder values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Convert to uppercase
    Uppercase,
    /// Convert to lowercase
    Lowercase,
    /// Trim whitespace
    Trim,
    /// URL percent-encode
    PercentEncode,
    /// Disable auto percent-encoding
    Raw,
}

impl Modifier {
    /// Parse a modifier from a string.
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "uppercase" => Some(Self::Uppercase),
            "lowercase" => Some(Self::Lowercase),
            "trim" => Some(Self::Trim),
            "percent-encode" | "percentencode" | "urlencode" => Some(Self::PercentEncode),
            "raw" => Some(Self::Raw),
            _ => None,
        }
    }

    /// Apply this modifier to a value.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn apply(&self, value: &str) -> String {
        match self {
            Self::Uppercase => value.to_uppercase(),
            Self::Lowercase => value.to_lowercase(),
            Self::Trim => value.trim().to_string(),
            Self::PercentEncode => utf8_percent_encode(value, NON_ALPHANUMERIC).to_string(),
            Self::Raw => value.to_string(), // Raw doesn't transform, just marks for no auto-encoding
        }
    }
}

/// Represents a date/time offset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DateOffset {
    /// Offset in minutes
    pub minutes: i32,
    /// Offset in hours
    pub hours: i32,
    /// Offset in days
    pub days: i32,
    /// Offset in months
    pub months: i32,
    /// Offset in years
    pub years: i32,
}

impl DateOffset {
    /// Parse a date offset string like "+2d", "-1M", "+3h30m".
    pub fn parse(s: &str) -> Result<Self> {
        static OFFSET_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"([+-]?\d+)([mhdMy])").expect("Invalid offset regex")
        });

        let s = s.trim();
        if s.is_empty() {
            return Ok(Self::default());
        }

        let mut offset = Self::default();
        let mut found_any = false;

        for cap in OFFSET_RE.captures_iter(s) {
            found_any = true;
            let value: i32 = cap[1]
                .parse()
                .map_err(|_| PlaceholderError::InvalidDateOffset {
                    offset: s.to_string(),
                })?;
            let unit = &cap[2];

            match unit {
                "m" => offset.minutes += value,
                "h" => offset.hours += value,
                "d" => offset.days += value,
                "M" => offset.months += value,
                "y" => offset.years += value,
                _ => {
                    return Err(PlaceholderError::InvalidDateOffset {
                        offset: s.to_string(),
                    })
                },
            }
        }

        if !found_any {
            return Err(PlaceholderError::InvalidDateOffset {
                offset: s.to_string(),
            });
        }

        Ok(offset)
    }

    /// Check if this offset is zero (no offset).
    pub fn is_zero(&self) -> bool {
        self.minutes == 0
            && self.hours == 0
            && self.days == 0
            && self.months == 0
            && self.years == 0
    }
}

/// A parsed placeholder with all its attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPlaceholder {
    /// The type of placeholder
    pub kind: PlaceholderKind,
    /// Name for named arguments
    pub name: Option<String>,
    /// Default value for optional arguments
    pub default: Option<String>,
    /// Options for dropdown selection
    pub options: Vec<String>,
    /// Modifiers to apply
    pub modifiers: Vec<Modifier>,
    /// Custom date/time format
    pub format: Option<String>,
    /// Date/time offset
    pub offset: Option<DateOffset>,
    /// Original matched text (e.g., "{argument name=\"query\"}")
    pub raw_text: String,
}

impl ParsedPlaceholder {
    /// Check if this placeholder requires user input.
    pub fn requires_input(&self) -> bool {
        self.kind == PlaceholderKind::Argument && self.default.is_none()
    }

    /// Check if the raw modifier is present.
    pub fn has_raw_modifier(&self) -> bool {
        self.modifiers.contains(&Modifier::Raw)
    }
}

/// Information about an argument that needs user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentInfo {
    /// Name of the argument (if named)
    pub name: Option<String>,
    /// Default value (if provided)
    pub default: Option<String>,
    /// Options for selection (if provided)
    pub options: Vec<String>,
}

// Regex patterns for parsing
static PLACEHOLDER_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    // Matches: {kind ...} or {kind | modifier | ...}
    Regex::new(r"\{([a-zA-Z]+)([^}]*)\}").expect("Invalid placeholder regex")
});

static ATTR_NAME_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"name\s*=\s*"([^"]*)""#).expect("Invalid name attr regex")
});

static ATTR_DEFAULT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"default\s*=\s*"([^"]*)""#).expect("Invalid default attr regex")
});

static ATTR_OPTIONS_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"options\s*=\s*"([^"]*)""#).expect("Invalid options attr regex")
});

static ATTR_FORMAT_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"format\s*=\s*"([^"]*)""#).expect("Invalid format attr regex")
});

static ATTR_OFFSET_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"offset\s*=\s*"([^"]*)""#).expect("Invalid offset attr regex")
});

/// Parse all placeholders from a link string.
///
/// # Arguments
///
/// * `link` - The URL or text containing placeholders
///
/// # Returns
///
/// A vector of parsed placeholders found in the string.
///
/// # Example
///
/// ```rust,ignore
/// let placeholders = parse_placeholders("https://example.com?q={argument}&date={date format=\"%Y-%m-%d\"}");
/// assert_eq!(placeholders.len(), 2);
/// ```
pub fn parse_placeholders(link: &str) -> Vec<ParsedPlaceholder> {
    let mut placeholders = Vec::new();

    for cap in PLACEHOLDER_RE.captures_iter(link) {
        let raw_text = cap.get(0).map_or("", |m| m.as_str()).to_string();
        let kind_str = &cap[1];
        let rest = cap.get(2).map_or("", |m| m.as_str());

        // Parse the kind
        let Some(kind) = PlaceholderKind::from_str(kind_str) else {
            // Unknown placeholder type, skip it
            continue;
        };

        // Parse attributes from the rest of the placeholder
        let name = ATTR_NAME_RE
            .captures(rest)
            .map(|c| c[1].to_string())
            .filter(|s| !s.is_empty());

        let default = ATTR_DEFAULT_RE.captures(rest).map(|c| c[1].to_string());

        let options: Vec<String> = ATTR_OPTIONS_RE
            .captures(rest)
            .map(|c| c[1].split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let format = ATTR_FORMAT_RE
            .captures(rest)
            .map(|c| c[1].to_string())
            .filter(|s| !s.is_empty());

        let offset = ATTR_OFFSET_RE
            .captures(rest)
            .and_then(|c| DateOffset::parse(&c[1]).ok());

        // Parse modifiers (after |)
        let modifiers: Vec<Modifier> = if rest.contains('|') {
            rest.split('|')
                .skip(1) // Skip the part before first |
                .filter_map(|s| {
                    // Extract just the modifier name, ignoring any attributes
                    let s = s.trim();
                    // If there's an attribute, take only the first word
                    let modifier_name = s.split_whitespace().next().unwrap_or(s);
                    Modifier::from_str(modifier_name)
                })
                .collect()
        } else {
            Vec::new()
        };

        placeholders.push(ParsedPlaceholder {
            kind,
            name,
            default,
            options,
            modifiers,
            format,
            offset,
            raw_text,
        });
    }

    placeholders
}

/// Extract unique argument names that need user input.
///
/// This function returns information about arguments that require user input,
/// deduplicating by name.
///
/// # Arguments
///
/// * `link` - The URL or text containing placeholders
///
/// # Returns
///
/// A vector of `ArgumentInfo` structs for arguments needing input.
pub fn extract_required_arguments(link: &str) -> Vec<ArgumentInfo> {
    let placeholders = parse_placeholders(link);
    let mut seen_names: HashMap<Option<String>, usize> = HashMap::new();
    let mut arguments: Vec<ArgumentInfo> = Vec::new();

    for p in placeholders {
        if p.kind != PlaceholderKind::Argument {
            continue;
        }

        // If this name was already seen, skip it (for named arguments)
        // Unnamed arguments are always added
        if p.name.is_some() {
            if seen_names.contains_key(&p.name) {
                continue;
            }
            seen_names.insert(p.name.clone(), arguments.len());
        }

        arguments.push(ArgumentInfo {
            name: p.name,
            default: p.default,
            options: p.options,
        });
    }

    arguments
}

/// Substitute placeholders with values.
///
/// # Arguments
///
/// * `link` - The URL or text containing placeholders
/// * `arguments` - Map of argument names to values (use empty string key for unnamed args)
/// * `clipboard` - Current clipboard content (if available)
/// * `selection` - Currently selected text (if available)
///
/// # Returns
///
/// The string with all placeholders substituted, or an error if required values are missing.
///
/// # Substitution Rules
///
/// - All substitutions are auto percent-encoded by default
/// - The `raw` modifier disables encoding
/// - Modifiers are applied in order: trim first, then case change, then encode
#[allow(clippy::implicit_hasher)]
pub fn substitute_placeholders(
    link: &str,
    arguments: &HashMap<String, String>,
    clipboard: Option<&str>,
    selection: Option<&str>,
) -> Result<String> {
    let placeholders = parse_placeholders(link);

    if placeholders.is_empty() {
        return Ok(link.to_string());
    }

    let mut result = link.to_string();
    let mut unnamed_arg_index = 0;

    for placeholder in &placeholders {
        let value = resolve_placeholder_value(
            placeholder,
            arguments,
            clipboard,
            selection,
            &mut unnamed_arg_index,
        )?;

        // Apply modifiers
        let mut processed = value;
        let mut has_explicit_encode = false;
        let mut has_raw = false;

        for modifier in &placeholder.modifiers {
            match modifier {
                Modifier::PercentEncode => {
                    has_explicit_encode = true;
                    processed = modifier.apply(&processed);
                },
                Modifier::Raw => {
                    has_raw = true;
                },
                _ => {
                    processed = modifier.apply(&processed);
                },
            }
        }

        // Auto percent-encode if neither explicit encode nor raw was specified
        if !has_explicit_encode && !has_raw {
            processed = utf8_percent_encode(&processed, NON_ALPHANUMERIC).to_string();
        }

        result = result.replace(&placeholder.raw_text, &processed);
    }

    Ok(result)
}

/// Resolve the value for a single placeholder.
fn resolve_placeholder_value(
    placeholder: &ParsedPlaceholder,
    arguments: &HashMap<String, String>,
    clipboard: Option<&str>,
    selection: Option<&str>,
    unnamed_arg_index: &mut usize,
) -> Result<String> {
    match &placeholder.kind {
        PlaceholderKind::Argument => {
            // Look up by name or by index
            let key = placeholder
                .name
                .clone()
                .unwrap_or_else(|| unnamed_arg_index.to_string());

            if placeholder.name.is_none() {
                *unnamed_arg_index += 1;
            }

            if let Some(value) = arguments.get(&key) {
                Ok(value.clone())
            } else if let Some(default) = &placeholder.default {
                Ok(default.clone())
            } else {
                Err(PlaceholderError::MissingArgument {
                    name: placeholder
                        .name
                        .clone()
                        .unwrap_or_else(|| "unnamed".to_string()),
                })
            }
        },

        PlaceholderKind::Clipboard => clipboard
            .map(std::string::ToString::to_string)
            .ok_or(PlaceholderError::ClipboardRequired),

        PlaceholderKind::Selection => selection
            .map(std::string::ToString::to_string)
            .ok_or(PlaceholderError::SelectionRequired),

        PlaceholderKind::Date => Ok(format_date(
            placeholder.format.as_deref(),
            placeholder.offset.as_ref(),
        )),

        PlaceholderKind::Time => Ok(format_time(
            placeholder.format.as_deref(),
            placeholder.offset.as_ref(),
        )),

        PlaceholderKind::DateTime => Ok(format_datetime(
            placeholder.format.as_deref(),
            placeholder.offset.as_ref(),
        )),

        PlaceholderKind::Day => Ok(format_day(placeholder.offset.as_ref())),

        PlaceholderKind::Uuid => Ok(uuid::Uuid::new_v4().to_string()),
    }
}

/// Format the current date with optional format and offset.
fn format_date(format: Option<&str>, offset: Option<&DateOffset>) -> String {
    let now = Local::now();
    let dt = apply_offset(now, offset);
    let format_str = format.unwrap_or("%Y-%m-%d");
    dt.format(format_str).to_string()
}

/// Format the current time with optional format and offset.
fn format_time(format: Option<&str>, offset: Option<&DateOffset>) -> String {
    let now = Local::now();
    let dt = apply_offset(now, offset);
    let format_str = format.unwrap_or("%H:%M:%S");
    dt.format(format_str).to_string()
}

/// Format the current date and time with optional format and offset.
fn format_datetime(format: Option<&str>, offset: Option<&DateOffset>) -> String {
    let now = Local::now();
    let dt = apply_offset(now, offset);
    let format_str = format.unwrap_or("%Y-%m-%d %H:%M:%S");
    dt.format(format_str).to_string()
}

/// Format the current day of week with optional offset.
fn format_day(offset: Option<&DateOffset>) -> String {
    let now = Local::now();
    let dt = apply_offset(now, offset);
    dt.format("%A").to_string() // Full weekday name
}

/// Apply a date offset to a datetime.
fn apply_offset(
    dt: chrono::DateTime<Local>,
    offset: Option<&DateOffset>,
) -> chrono::DateTime<Local> {
    let Some(offset) = offset else {
        return dt;
    };

    let mut result = dt;

    // Apply time-based offsets using Duration
    if offset.minutes != 0 {
        result += Duration::minutes(i64::from(offset.minutes));
    }
    if offset.hours != 0 {
        result += Duration::hours(i64::from(offset.hours));
    }
    if offset.days != 0 {
        result += Duration::days(i64::from(offset.days));
    }

    // Apply month offset (more complex due to varying month lengths)
    if offset.months != 0 {
        #[allow(clippy::cast_possible_wrap)]
        let total_months = result.month0() as i32 + offset.months;
        let years_adjust = total_months.div_euclid(12);
        let new_month = total_months.rem_euclid(12) as u32 + 1;
        let new_year = result.year() + years_adjust;

        // Clamp day to valid range for new month
        let max_day = days_in_month(new_year, new_month);
        let new_day = result.day().min(max_day);

        result = result
            .with_year(new_year)
            .and_then(|d| d.with_month(new_month))
            .and_then(|d| d.with_day(new_day))
            .unwrap_or(result);
    }

    // Apply year offset
    if offset.years != 0 {
        let new_year = result.year() + offset.years;
        // Handle Feb 29 in non-leap years
        if result.month() == 2 && result.day() == 29 && !is_leap_year(new_year) {
            result = result
                .with_day(28)
                .and_then(|d| d.with_year(new_year))
                .unwrap_or(result);
        } else {
            result = result.with_year(new_year).unwrap_or(result);
        }
    }

    result
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        },
        _ => 30,
    }
}

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Check if a link contains any placeholders.
pub fn has_placeholders(link: &str) -> bool {
    PLACEHOLDER_RE.is_match(link)
}

/// Check if a link contains placeholders that require user input.
pub fn requires_user_input(link: &str) -> bool {
    let placeholders = parse_placeholders(link);
    placeholders
        .iter()
        .any(|p| p.kind == PlaceholderKind::Argument && p.default.is_none())
}

/// Substitutes `{argument}` placeholders with a given value (for preview).
///
/// This is a simplified substitution for search result previews.
/// For full substitution, use `substitute_placeholders`.
pub fn substitute_argument(link: &str, argument: &str) -> String {
    // Use regex to replace {argument}, {query}, and {argument name="..."} patterns
    // {query} is supported for backward compatibility
    static ARG_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\{(?:argument|query)(?:\s+[^}]*)?\}").expect("Invalid argument regex")
    });

    let encoded = utf8_percent_encode(argument, NON_ALPHANUMERIC).to_string();
    ARG_RE.replace_all(link, encoded.as_str()).to_string()
}

/// Migrate legacy {query} placeholders to {argument}.
///
/// This function converts old-style `{query}` placeholders to the new
/// `{argument}` syntax for backward compatibility.
pub fn migrate_legacy_placeholders(link: &str) -> String {
    // Simple replacement - {query} becomes {argument}
    // This is handled automatically in PlaceholderKind::from_str,
    // but this function can be used for explicit migration/display
    link.replace("{query}", "{argument}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_argument() {
        let placeholders = parse_placeholders("https://example.com?q={argument}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Argument);
        assert!(placeholders[0].name.is_none());
        assert!(placeholders[0].default.is_none());
    }

    #[test]
    fn test_parse_named_argument() {
        let placeholders = parse_placeholders(r#"https://example.com?q={argument name="query"}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Argument);
        assert_eq!(placeholders[0].name, Some("query".to_string()));
    }

    #[test]
    fn test_parse_argument_with_default() {
        let placeholders =
            parse_placeholders(r#"https://example.com?q={argument default="hello"}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].default, Some("hello".to_string()));
    }

    #[test]
    fn test_parse_argument_with_options() {
        let placeholders =
            parse_placeholders(r#"https://example.com?lang={argument options="en,es,fr"}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].options, vec!["en", "es", "fr"]);
    }

    #[test]
    fn test_parse_clipboard() {
        let placeholders = parse_placeholders("https://example.com?text={clipboard}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Clipboard);
    }

    #[test]
    fn test_parse_selection() {
        let placeholders = parse_placeholders("https://example.com?text={selection}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Selection);
    }

    #[test]
    fn test_parse_date() {
        let placeholders = parse_placeholders("https://example.com?date={date}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Date);
    }

    #[test]
    fn test_parse_date_with_format() {
        let placeholders =
            parse_placeholders(r#"https://example.com?date={date format="%d/%m/%Y"}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].format, Some("%d/%m/%Y".to_string()));
    }

    #[test]
    fn test_parse_date_with_offset() {
        let placeholders = parse_placeholders(r#"https://example.com?date={date offset="+2d"}"#);
        assert_eq!(placeholders.len(), 1);
        let offset = placeholders[0].offset.as_ref().unwrap();
        assert_eq!(offset.days, 2);
    }

    #[test]
    fn test_parse_date_with_format_and_offset() {
        let placeholders =
            parse_placeholders(r#"https://example.com?date={date format="%Y-%m-%d" offset="+3d"}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].format, Some("%Y-%m-%d".to_string()));
        let offset = placeholders[0].offset.as_ref().unwrap();
        assert_eq!(offset.days, 3);
    }

    #[test]
    fn test_parse_time() {
        let placeholders = parse_placeholders("https://example.com?time={time}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Time);
    }

    #[test]
    fn test_parse_datetime() {
        let placeholders = parse_placeholders("https://example.com?dt={datetime}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::DateTime);
    }

    #[test]
    fn test_parse_day() {
        let placeholders = parse_placeholders("https://example.com?day={day}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Day);
    }

    #[test]
    fn test_parse_uuid() {
        let placeholders = parse_placeholders("https://example.com?id={uuid}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Uuid);
    }

    #[test]
    fn test_parse_modifiers() {
        let placeholders =
            parse_placeholders("https://example.com?q={clipboard | trim | uppercase}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(
            placeholders[0].modifiers,
            vec![Modifier::Trim, Modifier::Uppercase]
        );
    }

    #[test]
    fn test_parse_raw_modifier() {
        let placeholders = parse_placeholders("https://example.com?q={argument | raw}");
        assert_eq!(placeholders.len(), 1);
        assert!(placeholders[0].has_raw_modifier());
    }

    #[test]
    fn test_parse_multiple_placeholders() {
        let placeholders =
            parse_placeholders("https://example.com?q={argument}&date={date}&id={uuid}");
        assert_eq!(placeholders.len(), 3);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Argument);
        assert_eq!(placeholders[1].kind, PlaceholderKind::Date);
        assert_eq!(placeholders[2].kind, PlaceholderKind::Uuid);
    }

    #[test]
    fn test_parse_legacy_query() {
        // {query} should be treated as {argument} for backward compatibility
        let placeholders = parse_placeholders("https://example.com?q={query}");
        assert_eq!(placeholders.len(), 1);
        assert_eq!(placeholders[0].kind, PlaceholderKind::Argument);
    }

    #[test]
    fn test_extract_required_arguments() {
        let args = extract_required_arguments(
            r#"https://example.com?q={argument name="query"}&lang={argument name="lang" default="en"}"#,
        );
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, Some("query".to_string()));
        assert!(args[0].default.is_none());
        assert_eq!(args[1].name, Some("lang".to_string()));
        assert_eq!(args[1].default, Some("en".to_string()));
    }

    #[test]
    fn test_extract_deduplicates_named_arguments() {
        let args = extract_required_arguments(
            r#"https://example.com?q={argument name="query"}&alt={argument name="query"}"#,
        );
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, Some("query".to_string()));
    }

    #[test]
    fn test_substitute_simple_argument() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "test".to_string());

        let result = substitute_placeholders("https://example.com?q={argument}", &args, None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=test");
    }

    #[test]
    fn test_substitute_named_argument() {
        let mut args = HashMap::new();
        args.insert("query".to_string(), "rust programming".to_string());

        let result = substitute_placeholders(
            r#"https://example.com?q={argument name="query"}"#,
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        // Should be percent-encoded
        assert_eq!(result.unwrap(), "https://example.com?q=rust%20programming");
    }

    #[test]
    fn test_substitute_with_default() {
        let args = HashMap::new();

        let result = substitute_placeholders(
            r#"https://example.com?q={argument default="hello"}"#,
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=hello");
    }

    #[test]
    fn test_substitute_missing_required_argument() {
        let args = HashMap::new();

        let result = substitute_placeholders("https://example.com?q={argument}", &args, None, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlaceholderError::MissingArgument { .. }
        ));
    }

    #[test]
    fn test_substitute_clipboard() {
        let args = HashMap::new();

        let result = substitute_placeholders(
            "https://example.com?text={clipboard}",
            &args,
            Some("copied text"),
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?text=copied%20text");
    }

    #[test]
    fn test_substitute_clipboard_missing() {
        let args = HashMap::new();

        let result =
            substitute_placeholders("https://example.com?text={clipboard}", &args, None, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlaceholderError::ClipboardRequired
        ));
    }

    #[test]
    fn test_substitute_selection() {
        let args = HashMap::new();

        let result = substitute_placeholders(
            "https://example.com?text={selection}",
            &args,
            None,
            Some("selected text"),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?text=selected%20text");
    }

    #[test]
    fn test_substitute_uuid() {
        let args = HashMap::new();

        let result =
            substitute_placeholders("https://example.com?id={uuid}", &args, None, None).unwrap();

        // UUID should be 36 characters (with hyphens) + URL prefix
        assert!(result.starts_with("https://example.com?id="));
        // The UUID part should have the right length after percent-encoding of hyphens
        // 8-4-4-4-12 = 32 hex + 4 hyphens, hyphens become %2D (3 chars each)
        // So: 32 + 4*3 = 44 chars
        let uuid_part = &result["https://example.com?id=".len()..];
        // UUID without encoding would be 36 chars, with %2D for hyphens it's 32 + 4*3 = 44
        assert_eq!(uuid_part.len(), 44);
    }

    #[test]
    fn test_substitute_with_trim_modifier() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "  hello world  ".to_string());

        let result =
            substitute_placeholders("https://example.com?q={argument | trim}", &args, None, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=hello%20world");
    }

    #[test]
    fn test_substitute_with_uppercase_modifier() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "hello".to_string());

        let result = substitute_placeholders(
            "https://example.com?q={argument | uppercase}",
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=HELLO");
    }

    #[test]
    fn test_substitute_with_lowercase_modifier() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "HELLO".to_string());

        let result = substitute_placeholders(
            "https://example.com?q={argument | lowercase}",
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=hello");
    }

    #[test]
    fn test_substitute_with_raw_modifier() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "hello world".to_string());

        let result =
            substitute_placeholders("https://example.com?q={argument | raw}", &args, None, None);
        assert!(result.is_ok());
        // With raw modifier, should NOT be percent-encoded
        assert_eq!(result.unwrap(), "https://example.com?q=hello world");
    }

    #[test]
    fn test_substitute_multiple_modifiers() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "  Hello World  ".to_string());

        let result = substitute_placeholders(
            "https://example.com?q={argument | trim | lowercase}",
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=hello%20world");
    }

    #[test]
    fn test_date_offset_parse() {
        let offset = DateOffset::parse("+2d").unwrap();
        assert_eq!(offset.days, 2);

        let offset = DateOffset::parse("-1M").unwrap();
        assert_eq!(offset.months, -1);

        let offset = DateOffset::parse("+3h30m").unwrap();
        assert_eq!(offset.hours, 3);
        assert_eq!(offset.minutes, 30);

        let offset = DateOffset::parse("-1y").unwrap();
        assert_eq!(offset.years, -1);
    }

    #[test]
    fn test_date_offset_parse_invalid() {
        let result = DateOffset::parse("invalid");
        assert!(result.is_err());

        let result = DateOffset::parse("+2x");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_placeholders() {
        assert!(has_placeholders("https://example.com?q={argument}"));
        assert!(has_placeholders("https://example.com?date={date}"));
        assert!(!has_placeholders("https://example.com"));
        assert!(!has_placeholders("https://example.com?q=test"));
    }

    #[test]
    fn test_requires_user_input() {
        assert!(requires_user_input("https://example.com?q={argument}"));
        assert!(!requires_user_input(
            r#"https://example.com?q={argument default="test"}"#
        ));
        assert!(!requires_user_input("https://example.com?date={date}"));
        assert!(!requires_user_input("https://example.com?text={clipboard}"));
    }

    #[test]
    fn test_migrate_legacy_placeholders() {
        let migrated = migrate_legacy_placeholders("https://example.com?q={query}");
        assert_eq!(migrated, "https://example.com?q={argument}");
    }

    #[test]
    fn test_complex_url_with_multiple_placeholders() {
        let mut args = HashMap::new();
        args.insert("search".to_string(), "rust".to_string());
        args.insert("lang".to_string(), "en".to_string());

        let result = substitute_placeholders(
            r#"https://example.com/search?q={argument name="search" | trim}&lang={argument name="lang"}&date={date}&id={uuid}"#,
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("q=rust"));
        assert!(url.contains("lang=en"));
        // Date and UUID will be substituted with actual values
    }

    #[test]
    fn test_reusing_named_argument() {
        let mut args = HashMap::new();
        args.insert("query".to_string(), "test".to_string());

        let result = substitute_placeholders(
            r#"https://example.com?q={argument name="query"}&alt={argument name="query"}"#,
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=test&alt=test");
    }

    #[test]
    fn test_modifier_ordering() {
        // Modifiers should be applied in order: trim, then case change
        let mut args = HashMap::new();
        args.insert("0".to_string(), "  HELLO WORLD  ".to_string());

        let result = substitute_placeholders(
            "https://example.com?q={argument | trim | lowercase}",
            &args,
            None,
            None,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com?q=hello%20world");
    }

    #[test]
    fn test_no_placeholders_returns_original() {
        let args = HashMap::new();
        let result =
            substitute_placeholders("https://example.com/page", &args, None, None).unwrap();
        assert_eq!(result, "https://example.com/page");
    }

    #[test]
    fn test_special_characters_percent_encoded() {
        let mut args = HashMap::new();
        args.insert("0".to_string(), "hello&world=test".to_string());

        let result = substitute_placeholders("https://example.com?q={argument}", &args, None, None);
        assert!(result.is_ok());
        // & and = should be percent-encoded
        assert_eq!(
            result.unwrap(),
            "https://example.com?q=hello%26world%3Dtest"
        );
    }

    #[test]
    fn test_raw_text_preserved() {
        let placeholders = parse_placeholders(r#"test {argument name="query" | trim}"#);
        assert_eq!(placeholders.len(), 1);
        assert_eq!(
            placeholders[0].raw_text,
            r#"{argument name="query" | trim}"#
        );
    }
}
