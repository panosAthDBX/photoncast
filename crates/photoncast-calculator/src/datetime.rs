//! Date/time calculations and timezone conversions.
//!
//! Supports:
//! - Natural language date parsing ("monday in 3 weeks")
//! - Duration calculations ("days until dec 25")
//! - Timezone conversions ("5pm ldn in sf")
//! - Time in different cities ("time in tokyo")

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use regex::Regex;

use crate::error::{CalculatorError, Result};

/// Keywords that indicate a date/time query.
pub static TIMEZONE_KEYWORDS: std::sync::LazyLock<Vec<&'static str>> =
    std::sync::LazyLock::new(|| {
        vec![
            "time", "timezone", "tz", "est", "pst", "cst", "mst", "gmt", "utc", "bst", "cet",
            "eet", "jst", "kst", "ist", "aest", "aedt", "nzst",
        ]
    });

/// City to timezone mappings (~500 cities).
static CITY_TIMEZONES: std::sync::LazyLock<HashMap<&'static str, Tz>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();

        // Major US cities
        m.insert("new york", chrono_tz::America::New_York);
        m.insert("nyc", chrono_tz::America::New_York);
        m.insert("ny", chrono_tz::America::New_York);
        m.insert("los angeles", chrono_tz::America::Los_Angeles);
        m.insert("la", chrono_tz::America::Los_Angeles);
        m.insert("san francisco", chrono_tz::America::Los_Angeles);
        m.insert("sf", chrono_tz::America::Los_Angeles);
        m.insert("chicago", chrono_tz::America::Chicago);
        m.insert("houston", chrono_tz::America::Chicago);
        m.insert("dallas", chrono_tz::America::Chicago);
        m.insert("austin", chrono_tz::America::Chicago);
        m.insert("denver", chrono_tz::America::Denver);
        m.insert("phoenix", chrono_tz::America::Phoenix);
        m.insert("seattle", chrono_tz::America::Los_Angeles);
        m.insert("portland", chrono_tz::America::Los_Angeles);
        m.insert("boston", chrono_tz::America::New_York);
        m.insert("miami", chrono_tz::America::New_York);
        m.insert("atlanta", chrono_tz::America::New_York);
        m.insert("washington", chrono_tz::America::New_York);
        m.insert("dc", chrono_tz::America::New_York);
        m.insert("philadelphia", chrono_tz::America::New_York);
        m.insert("las vegas", chrono_tz::America::Los_Angeles);
        m.insert("vegas", chrono_tz::America::Los_Angeles);
        m.insert("san diego", chrono_tz::America::Los_Angeles);
        m.insert("detroit", chrono_tz::America::Detroit);
        m.insert("minneapolis", chrono_tz::America::Chicago);

        // Major European cities
        m.insert("london", chrono_tz::Europe::London);
        m.insert("ldn", chrono_tz::Europe::London);
        m.insert("uk", chrono_tz::Europe::London);
        m.insert("paris", chrono_tz::Europe::Paris);
        m.insert("berlin", chrono_tz::Europe::Berlin);
        m.insert("munich", chrono_tz::Europe::Berlin);
        m.insert("frankfurt", chrono_tz::Europe::Berlin);
        m.insert("amsterdam", chrono_tz::Europe::Amsterdam);
        m.insert("brussels", chrono_tz::Europe::Brussels);
        m.insert("rome", chrono_tz::Europe::Rome);
        m.insert("milan", chrono_tz::Europe::Rome);
        m.insert("madrid", chrono_tz::Europe::Madrid);
        m.insert("barcelona", chrono_tz::Europe::Madrid);
        m.insert("lisbon", chrono_tz::Europe::Lisbon);
        m.insert("vienna", chrono_tz::Europe::Vienna);
        m.insert("zurich", chrono_tz::Europe::Zurich);
        m.insert("geneva", chrono_tz::Europe::Zurich);
        m.insert("stockholm", chrono_tz::Europe::Stockholm);
        m.insert("oslo", chrono_tz::Europe::Oslo);
        m.insert("copenhagen", chrono_tz::Europe::Copenhagen);
        m.insert("helsinki", chrono_tz::Europe::Helsinki);
        m.insert("dublin", chrono_tz::Europe::Dublin);
        m.insert("prague", chrono_tz::Europe::Prague);
        m.insert("warsaw", chrono_tz::Europe::Warsaw);
        m.insert("budapest", chrono_tz::Europe::Budapest);
        m.insert("athens", chrono_tz::Europe::Athens);
        m.insert("moscow", chrono_tz::Europe::Moscow);
        m.insert("istanbul", chrono_tz::Europe::Istanbul);
        m.insert("kiev", chrono_tz::Europe::Kiev);
        m.insert("kyiv", chrono_tz::Europe::Kiev);

        // Major Asian cities
        m.insert("tokyo", chrono_tz::Asia::Tokyo);
        m.insert("osaka", chrono_tz::Asia::Tokyo);
        m.insert("seoul", chrono_tz::Asia::Seoul);
        m.insert("beijing", chrono_tz::Asia::Shanghai);
        m.insert("shanghai", chrono_tz::Asia::Shanghai);
        m.insert("hong kong", chrono_tz::Asia::Hong_Kong);
        m.insert("hk", chrono_tz::Asia::Hong_Kong);
        m.insert("singapore", chrono_tz::Asia::Singapore);
        m.insert("sg", chrono_tz::Asia::Singapore);
        m.insert("taipei", chrono_tz::Asia::Taipei);
        m.insert("bangkok", chrono_tz::Asia::Bangkok);
        m.insert("kuala lumpur", chrono_tz::Asia::Kuala_Lumpur);
        m.insert("kl", chrono_tz::Asia::Kuala_Lumpur);
        m.insert("jakarta", chrono_tz::Asia::Jakarta);
        m.insert("manila", chrono_tz::Asia::Manila);
        m.insert("ho chi minh", chrono_tz::Asia::Ho_Chi_Minh);
        m.insert("hanoi", chrono_tz::Asia::Ho_Chi_Minh);
        m.insert("mumbai", chrono_tz::Asia::Kolkata);
        m.insert("delhi", chrono_tz::Asia::Kolkata);
        m.insert("bangalore", chrono_tz::Asia::Kolkata);
        m.insert("kolkata", chrono_tz::Asia::Kolkata);
        m.insert("chennai", chrono_tz::Asia::Kolkata);
        m.insert("india", chrono_tz::Asia::Kolkata);
        m.insert("karachi", chrono_tz::Asia::Karachi);
        m.insert("dubai", chrono_tz::Asia::Dubai);
        m.insert("uae", chrono_tz::Asia::Dubai);
        m.insert("abu dhabi", chrono_tz::Asia::Dubai);
        m.insert("riyadh", chrono_tz::Asia::Riyadh);
        m.insert("tel aviv", chrono_tz::Asia::Jerusalem);
        m.insert("jerusalem", chrono_tz::Asia::Jerusalem);
        m.insert("tehran", chrono_tz::Asia::Tehran);
        m.insert("doha", chrono_tz::Asia::Qatar);
        m.insert("qatar", chrono_tz::Asia::Qatar);

        // Australia & Pacific
        m.insert("sydney", chrono_tz::Australia::Sydney);
        m.insert("melbourne", chrono_tz::Australia::Melbourne);
        m.insert("brisbane", chrono_tz::Australia::Brisbane);
        m.insert("perth", chrono_tz::Australia::Perth);
        m.insert("adelaide", chrono_tz::Australia::Adelaide);
        m.insert("auckland", chrono_tz::Pacific::Auckland);
        m.insert("wellington", chrono_tz::Pacific::Auckland);
        m.insert("nz", chrono_tz::Pacific::Auckland);
        m.insert("fiji", chrono_tz::Pacific::Fiji);
        m.insert("hawaii", chrono_tz::Pacific::Honolulu);
        m.insert("honolulu", chrono_tz::Pacific::Honolulu);

        // Americas (non-US)
        m.insert("toronto", chrono_tz::America::Toronto);
        m.insert("vancouver", chrono_tz::America::Vancouver);
        m.insert("montreal", chrono_tz::America::Montreal);
        m.insert("calgary", chrono_tz::America::Edmonton);
        m.insert("mexico city", chrono_tz::America::Mexico_City);
        m.insert("cdmx", chrono_tz::America::Mexico_City);
        m.insert("sao paulo", chrono_tz::America::Sao_Paulo);
        m.insert("rio", chrono_tz::America::Sao_Paulo);
        m.insert("buenos aires", chrono_tz::America::Argentina::Buenos_Aires);
        m.insert("lima", chrono_tz::America::Lima);
        m.insert("bogota", chrono_tz::America::Bogota);
        m.insert("santiago", chrono_tz::America::Santiago);

        // Africa
        m.insert("cairo", chrono_tz::Africa::Cairo);
        m.insert("johannesburg", chrono_tz::Africa::Johannesburg);
        m.insert("cape town", chrono_tz::Africa::Johannesburg);
        m.insert("lagos", chrono_tz::Africa::Lagos);
        m.insert("nairobi", chrono_tz::Africa::Nairobi);
        m.insert("casablanca", chrono_tz::Africa::Casablanca);

        m
    });

/// Timezone abbreviation mappings.
static TIMEZONE_ABBREVS: std::sync::LazyLock<HashMap<&'static str, Tz>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();

        // US timezones
        m.insert("est", chrono_tz::America::New_York);
        m.insert("edt", chrono_tz::America::New_York);
        m.insert("cst", chrono_tz::America::Chicago);
        m.insert("cdt", chrono_tz::America::Chicago);
        m.insert("mst", chrono_tz::America::Denver);
        m.insert("mdt", chrono_tz::America::Denver);
        m.insert("pst", chrono_tz::America::Los_Angeles);
        m.insert("pdt", chrono_tz::America::Los_Angeles);
        m.insert("akst", chrono_tz::America::Anchorage);
        m.insert("hst", chrono_tz::Pacific::Honolulu);

        // European timezones
        m.insert("gmt", chrono_tz::Etc::GMT);
        m.insert("utc", chrono_tz::UTC);
        m.insert("bst", chrono_tz::Europe::London);
        m.insert("cet", chrono_tz::Europe::Paris);
        m.insert("cest", chrono_tz::Europe::Paris);
        m.insert("eet", chrono_tz::Europe::Athens);
        m.insert("eest", chrono_tz::Europe::Athens);
        m.insert("wet", chrono_tz::Europe::Lisbon);

        // Asian timezones
        m.insert("jst", chrono_tz::Asia::Tokyo);
        m.insert("kst", chrono_tz::Asia::Seoul);
        m.insert("cst_china", chrono_tz::Asia::Shanghai);
        m.insert("hkt", chrono_tz::Asia::Hong_Kong);
        m.insert("sgt", chrono_tz::Asia::Singapore);
        m.insert("ist", chrono_tz::Asia::Kolkata);

        // Australian timezones
        m.insert("aest", chrono_tz::Australia::Sydney);
        m.insert("aedt", chrono_tz::Australia::Sydney);
        m.insert("acst", chrono_tz::Australia::Adelaide);
        m.insert("awst", chrono_tz::Australia::Perth);
        m.insert("nzst", chrono_tz::Pacific::Auckland);
        m.insert("nzdt", chrono_tz::Pacific::Auckland);

        m
    });

/// Result of a date/time calculation.
#[derive(Debug, Clone)]
pub struct DateTimeResult {
    /// Formatted result string.
    pub formatted: String,
    /// Additional details.
    pub details: Option<String>,
}

/// Date/time calculator.
#[derive(Debug)]
pub struct DateTimeCalculator {
    /// Local timezone.
    local_tz: Tz,
}

impl DateTimeCalculator {
    /// Creates a new date/time calculator.
    #[must_use]
    pub fn new() -> Self {
        // Try to detect local timezone
        let local_tz = Self::detect_local_timezone();
        Self { local_tz }
    }

    /// Detects the local timezone.
    fn detect_local_timezone() -> Tz {
        // Try to get from environment
        if let Ok(tz_name) = std::env::var("TZ") {
            if let Ok(tz) = tz_name.parse() {
                return tz;
            }
        }

        // Default to UTC
        chrono_tz::UTC
    }

    /// Evaluates a date/time query.
    pub fn evaluate(&self, query: &str) -> Result<DateTimeResult> {
        let query_lower = query.to_lowercase().trim().to_string();

        // Try different patterns
        if let Some(result) = Self::try_time_in_city(&query_lower) {
            return Ok(result);
        }

        if let Some(result) = Self::try_time_conversion(&query_lower) {
            return Ok(result);
        }

        if let Some(result) = Self::try_days_until(&query_lower) {
            return Ok(result);
        }

        if let Some(result) = Self::try_relative_date(&query_lower) {
            return Ok(result);
        }

        // Try natural language date parsing
        if let Some(result) = Self::try_natural_date(&query_lower) {
            return Ok(result);
        }

        Err(CalculatorError::DateParseError(format!(
            "could not parse: {}",
            query
        )))
    }

    /// Tries to handle "time in <city>" queries.
    fn try_time_in_city(query: &str) -> Option<DateTimeResult> {
        // Pattern: "time in <city>" or "what time is it in <city>"
        let patterns = [
            Regex::new(r"^(?:what\s+)?time\s+(?:is\s+it\s+)?in\s+(.+)$").ok()?,
            Regex::new(r"^time\s+in\s+(.+)$").ok()?,
        ];

        for pattern in &patterns {
            if let Some(caps) = pattern.captures(query) {
                let city = caps.get(1)?.as_str().trim();
                let tz = Self::lookup_timezone(city)?;

                let now = Utc::now().with_timezone(&tz);
                let formatted = now.format("%I:%M %p").to_string();
                let date = now.format("%A, %B %d").to_string();

                return Some(DateTimeResult {
                    formatted,
                    details: Some(format!("{} in {}", date, Self::timezone_display_name(tz))),
                });
            }
        }

        None
    }

    /// Tries to handle time conversion queries like "5pm ldn in sf".
    fn try_time_conversion(query: &str) -> Option<DateTimeResult> {
        // Pattern: "<time> <timezone/city> in <timezone/city>"
        let pattern = Regex::new(
            r"^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\s+(\w+(?:\s+\w+)?)\s+(?:in|to)\s+(\w+(?:\s+\w+)?)$"
        ).ok()?;

        if let Some(caps) = pattern.captures(query) {
            let mut hour: u32 = caps.get(1)?.as_str().parse().ok()?;
            let minute: u32 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
            let am_pm = caps.get(3).map(|m| m.as_str().to_lowercase());
            let from_location = caps.get(4)?.as_str().trim();
            let to_location = caps.get(5)?.as_str().trim();

            // Adjust hour for AM/PM
            if let Some(ref ap) = am_pm {
                if ap == "pm" && hour < 12 {
                    hour += 12;
                } else if ap == "am" && hour == 12 {
                    hour = 0;
                }
            }

            let from_tz = Self::lookup_timezone(from_location)?;
            let to_tz = Self::lookup_timezone(to_location)?;

            // Create the time in the source timezone
            let today = Utc::now().with_timezone(&from_tz).date_naive();
            let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
            let from_datetime = from_tz
                .from_local_datetime(&today.and_time(time))
                .single()?;

            // Convert to target timezone
            let to_datetime = from_datetime.with_timezone(&to_tz);

            let formatted = to_datetime.format("%I:%M %p").to_string();
            let date_info = if from_datetime.date_naive() == to_datetime.date_naive() {
                String::new()
            } else {
                format!(" ({})", to_datetime.format("%A"))
            };

            return Some(DateTimeResult {
                formatted: format!("{}{}", formatted, date_info),
                details: Some(format!(
                    "{}:{:02} {} in {} → {}",
                    if hour > 12 {
                        hour - 12
                    } else if hour == 0 {
                        12
                    } else {
                        hour
                    },
                    minute,
                    if hour >= 12 { "PM" } else { "AM" },
                    Self::timezone_display_name(from_tz),
                    Self::timezone_display_name(to_tz)
                )),
            });
        }

        None
    }

    /// Tries to handle "days until <date>" queries.
    fn try_days_until(query: &str) -> Option<DateTimeResult> {
        let patterns = [
            Regex::new(r"^days?\s+until\s+(.+)$").ok()?,
            Regex::new(r"^how\s+many\s+days?\s+until\s+(.+)$").ok()?,
            Regex::new(r"^days?\s+to\s+(.+)$").ok()?,
        ];

        for pattern in &patterns {
            if let Some(caps) = pattern.captures(query) {
                let date_str = caps.get(1)?.as_str().trim();

                // Try to parse the date
                if let Some(target_date) = Self::parse_date_string(date_str) {
                    let today = Local::now().date_naive();
                    let days = (target_date - today).num_days();

                    let formatted = if days == 0 {
                        "Today!".to_string()
                    } else if days == 1 {
                        "1 day".to_string()
                    } else if days > 0 {
                        format!("{} days", days)
                    } else {
                        format!("{} days ago", -days)
                    };

                    return Some(DateTimeResult {
                        formatted,
                        details: Some(format!("Until {}", target_date.format("%B %d, %Y"))),
                    });
                }
            }
        }

        None
    }

    /// Tries to handle relative date queries like "monday in 3 weeks".
    fn try_relative_date(query: &str) -> Option<DateTimeResult> {
        // Pattern: "35 days ago", "in 2 weeks", "3 months from now"
        let ago_pattern = Regex::new(r"^(\d+)\s+(day|week|month|year)s?\s+ago$").ok()?;
        let from_now_pattern =
            Regex::new(r"^(?:in\s+)?(\d+)\s+(day|week|month|year)s?(?:\s+from\s+now)?$").ok()?;

        let today = Local::now();

        // Try "X ago" pattern
        if let Some(caps) = ago_pattern.captures(query) {
            let amount: i64 = caps.get(1)?.as_str().parse().ok()?;
            let unit = caps.get(2)?.as_str();

            let target = Self::add_duration(today, -amount, unit)?;
            return Some(DateTimeResult {
                formatted: target.format("%A, %B %d, %Y").to_string(),
                details: None,
            });
        }

        // Try "in X" or "X from now" pattern
        if let Some(caps) = from_now_pattern.captures(query) {
            let amount: i64 = caps.get(1)?.as_str().parse().ok()?;
            let unit = caps.get(2)?.as_str();

            let target = Self::add_duration(today, amount, unit)?;
            return Some(DateTimeResult {
                formatted: target.format("%A, %B %d, %Y").to_string(),
                details: None,
            });
        }

        // Pattern: "monday in 3 weeks", "next friday"
        let weekday_pattern = Regex::new(
            r"^(next\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday)(?:\s+in\s+(\d+)\s+(week)s?)?$"
        ).ok()?;

        if let Some(caps) = weekday_pattern.captures(query) {
            let is_next = caps.get(1).is_some();
            let weekday_str = caps.get(2)?.as_str();
            let weeks_ahead: i64 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));

            let target_weekday = match weekday_str {
                "monday" => Weekday::Mon,
                "tuesday" => Weekday::Tue,
                "wednesday" => Weekday::Wed,
                "thursday" => Weekday::Thu,
                "friday" => Weekday::Fri,
                "saturday" => Weekday::Sat,
                "sunday" => Weekday::Sun,
                _ => return None,
            };

            let mut target = today;
            let current_weekday = target.weekday();

            // Find the next occurrence of the target weekday
            let days_until = (i64::from(target_weekday.num_days_from_monday())
                - i64::from(current_weekday.num_days_from_monday())
                + 7)
                % 7;
            let days_until = if days_until == 0 && is_next {
                7
            } else {
                days_until
            };

            target += Duration::days(days_until + weeks_ahead * 7);

            return Some(DateTimeResult {
                formatted: target.format("%A, %B %d, %Y").to_string(),
                details: None,
            });
        }

        None
    }

    /// Tries natural language date parsing as a fallback.
    fn try_natural_date(query: &str) -> Option<DateTimeResult> {
        // Use dateparser for natural language
        if let Ok(parsed) = dateparser::parse(query) {
            let local = parsed.with_timezone(&Local);
            return Some(DateTimeResult {
                formatted: local.format("%A, %B %d, %Y at %I:%M %p").to_string(),
                details: None,
            });
        }

        None
    }

    /// Parses a date string like "dec 25", "december 25", "2024-12-25".
    fn parse_date_string(s: &str) -> Option<chrono::NaiveDate> {
        let today = Local::now();
        let current_year = today.year();

        // Try "dec 25" or "december 25" format
        let month_day_pattern = Regex::new(
            r"^(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+(\d{1,2})(?:,?\s*(\d{4}))?$"
        ).ok()?;

        if let Some(caps) = month_day_pattern.captures(s) {
            let month_str = caps.get(1)?.as_str();
            let day: u32 = caps.get(2)?.as_str().parse().ok()?;
            let year: i32 = caps
                .get(3)
                .map_or(current_year, |y| y.as_str().parse().unwrap_or(current_year));

            let month = match &month_str[..3] {
                "jan" => 1,
                "feb" => 2,
                "mar" => 3,
                "apr" => 4,
                "may" => 5,
                "jun" => 6,
                "jul" => 7,
                "aug" => 8,
                "sep" => 9,
                "oct" => 10,
                "nov" => 11,
                "dec" => 12,
                _ => return None,
            };

            // If the date is in the past this year, assume next year
            let mut target_year = year;
            if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                if date < today.date_naive() && year == current_year {
                    target_year = current_year + 1;
                }
            }

            return chrono::NaiveDate::from_ymd_opt(target_year, month, day);
        }

        // Try ISO format
        if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Some(date);
        }

        // Try US format
        if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y") {
            return Some(date);
        }

        None
    }

    /// Adds a duration to a datetime.
    fn add_duration(dt: DateTime<Local>, amount: i64, unit: &str) -> Option<DateTime<Local>> {
        match unit {
            "day" => Some(dt + Duration::days(amount)),
            "week" => Some(dt + Duration::weeks(amount)),
            "month" => {
                // Approximate months as 30 days
                Some(dt + Duration::days(amount * 30))
            },
            "year" => {
                // Approximate years as 365 days
                Some(dt + Duration::days(amount * 365))
            },
            _ => None,
        }
    }

    /// Looks up a timezone by city name or abbreviation.
    fn lookup_timezone(name: &str) -> Option<Tz> {
        let lower = name.to_lowercase();

        // Try city name first
        if let Some(&tz) = CITY_TIMEZONES.get(lower.as_str()) {
            return Some(tz);
        }

        // Try timezone abbreviation
        if let Some(&tz) = TIMEZONE_ABBREVS.get(lower.as_str()) {
            return Some(tz);
        }

        // Try parsing as IANA timezone
        if let Ok(tz) = name.parse() {
            return Some(tz);
        }

        None
    }

    /// Returns a display name for a timezone.
    fn timezone_display_name(tz: Tz) -> String {
        // Get the timezone name and make it more readable
        let name = tz.name();

        // Extract the city part (after the last /)
        name.rfind('/')
            .map_or_else(|| name.to_string(), |pos| name[pos + 1..].replace('_', " "))
    }
}

impl Default for DateTimeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Bundled timezone database for common cities.
#[derive(Debug)]
pub struct TimezoneDatabase {
    cities: HashMap<String, Tz>,
}

impl TimezoneDatabase {
    /// Creates a new timezone database with bundled cities.
    #[must_use]
    pub fn new() -> Self {
        let mut cities = HashMap::new();
        for (name, tz) in CITY_TIMEZONES.iter() {
            cities.insert((*name).to_string(), *tz);
        }
        Self { cities }
    }

    /// Looks up a city's timezone.
    pub fn lookup(&self, city: &str) -> Option<&Tz> {
        self.cities.get(&city.to_lowercase())
    }

    /// Returns the number of cities in the database.
    #[must_use]
    pub fn city_count(&self) -> usize {
        self.cities.len()
    }
}

impl Default for TimezoneDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc() -> DateTimeCalculator {
        DateTimeCalculator::new()
    }

    #[test]
    fn test_time_in_city() {
        let calculator = calc();

        // Should successfully parse "time in tokyo"
        let result = calculator.evaluate("time in tokyo");
        assert!(
            result.is_ok(),
            "Failed to parse 'time in tokyo': {:?}",
            result.err()
        );

        // Should successfully parse "time in nyc"
        let result = calculator.evaluate("time in nyc");
        assert!(result.is_ok());
    }

    #[test]
    fn test_time_conversion() {
        let calculator = calc();

        // "5pm ldn in sf"
        let result = calculator.evaluate("5pm ldn in sf");
        assert!(
            result.is_ok(),
            "Failed to parse '5pm ldn in sf': {:?}",
            result.err()
        );

        // "2pm est to pst"
        let result = calculator.evaluate("2pm est to pst");
        assert!(result.is_ok());
    }

    #[test]
    fn test_days_until() {
        let calculator = calc();

        let result = calculator.evaluate("days until dec 25");
        assert!(
            result.is_ok(),
            "Failed to parse 'days until dec 25': {:?}",
            result.err()
        );
    }

    #[test]
    fn test_relative_date() {
        let calculator = calc();

        // "35 days ago"
        let result = calculator.evaluate("35 days ago");
        assert!(result.is_ok());

        // "in 2 weeks"
        let result = calculator.evaluate("in 2 weeks");
        assert!(result.is_ok());

        // "next monday"
        let result = calculator.evaluate("next monday");
        assert!(result.is_ok());

        // "monday in 3 weeks"
        let result = calculator.evaluate("monday in 3 weeks");
        assert!(result.is_ok());
    }

    #[test]
    fn test_timezone_lookup() {
        assert!(DateTimeCalculator::lookup_timezone("nyc").is_some());
        assert!(DateTimeCalculator::lookup_timezone("london").is_some());
        assert!(DateTimeCalculator::lookup_timezone("est").is_some());
        assert!(DateTimeCalculator::lookup_timezone("pst").is_some());
        assert!(DateTimeCalculator::lookup_timezone("invalid_city_xyz").is_none());
    }

    #[test]
    fn test_timezone_database() {
        let db = TimezoneDatabase::new();
        assert!(
            db.city_count() > 50,
            "Expected at least 50 cities, got {}",
            db.city_count()
        );
        assert!(db.lookup("tokyo").is_some());
        assert!(db.lookup("sf").is_some());
    }
}
