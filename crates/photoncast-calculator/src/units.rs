//! Unit conversion module.
//!
//! Supports conversion between various unit types:
//! - Length (mm, cm, m, km, in, ft, yd, mi)
//! - Weight/Mass (mg, g, kg, oz, lb, ton)
//! - Volume (ml, l, tsp, tbsp, cup, pt, qt, gal)
//! - Temperature (C, F, K)
//! - Data (B, KB, MB, GB, TB, PB)
//! - Speed (m/s, km/h, mph, knots, ft/s)

use std::collections::HashMap;


use crate::error::{CalculatorError, Result};

/// Map of unit patterns to their canonical form and category.
pub static UNIT_PATTERNS: std::sync::LazyLock<HashMap<&'static str, (&'static str, UnitCategory)>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();

        // Length units
        for alias in &[
            "mm",
            "millimeter",
            "millimeters",
            "millimetre",
            "millimetres",
        ] {
            m.insert(*alias, ("mm", UnitCategory::Length));
        }
        for alias in &[
            "cm",
            "centimeter",
            "centimeters",
            "centimetre",
            "centimetres",
        ] {
            m.insert(*alias, ("cm", UnitCategory::Length));
        }
        for alias in &["m", "meter", "meters", "metre", "metres"] {
            m.insert(*alias, ("m", UnitCategory::Length));
        }
        for alias in &[
            "km",
            "kilometer",
            "kilometers",
            "kilometre",
            "kilometres",
            "kms",
        ] {
            m.insert(*alias, ("km", UnitCategory::Length));
        }
        for alias in &["in", "inch", "inches", "\""] {
            m.insert(*alias, ("in", UnitCategory::Length));
        }
        for alias in &["ft", "foot", "feet", "'"] {
            m.insert(*alias, ("ft", UnitCategory::Length));
        }
        for alias in &["yd", "yard", "yards"] {
            m.insert(*alias, ("yd", UnitCategory::Length));
        }
        for alias in &["mi", "mile", "miles"] {
            m.insert(*alias, ("mi", UnitCategory::Length));
        }
        for alias in &["nm", "nautical mile", "nautical miles", "nmi"] {
            m.insert(*alias, ("nm", UnitCategory::Length));
        }

        // Weight/Mass units
        for alias in &["mg", "milligram", "milligrams"] {
            m.insert(*alias, ("mg", UnitCategory::Weight));
        }
        for alias in &["g", "gram", "grams"] {
            m.insert(*alias, ("g", UnitCategory::Weight));
        }
        for alias in &["kg", "kilogram", "kilograms", "kilo", "kilos"] {
            m.insert(*alias, ("kg", UnitCategory::Weight));
        }
        for alias in &["oz", "ounce", "ounces"] {
            m.insert(*alias, ("oz", UnitCategory::Weight));
        }
        for alias in &["lb", "lbs", "pound", "pounds"] {
            m.insert(*alias, ("lb", UnitCategory::Weight));
        }
        for alias in &["ton", "tons", "tonne", "tonnes", "t"] {
            m.insert(*alias, ("ton", UnitCategory::Weight));
        }
        for alias in &["st", "stone", "stones"] {
            m.insert(*alias, ("st", UnitCategory::Weight));
        }

        // Volume units
        for alias in &[
            "ml",
            "milliliter",
            "milliliters",
            "millilitre",
            "millilitres",
        ] {
            m.insert(*alias, ("ml", UnitCategory::Volume));
        }
        for alias in &["l", "liter", "liters", "litre", "litres"] {
            m.insert(*alias, ("l", UnitCategory::Volume));
        }
        for alias in &["tsp", "teaspoon", "teaspoons"] {
            m.insert(*alias, ("tsp", UnitCategory::Volume));
        }
        for alias in &["tbsp", "tablespoon", "tablespoons"] {
            m.insert(*alias, ("tbsp", UnitCategory::Volume));
        }
        for alias in &["cup", "cups"] {
            m.insert(*alias, ("cup", UnitCategory::Volume));
        }
        for alias in &["pt", "pint", "pints"] {
            m.insert(*alias, ("pt", UnitCategory::Volume));
        }
        for alias in &["qt", "quart", "quarts"] {
            m.insert(*alias, ("qt", UnitCategory::Volume));
        }
        for alias in &["gal", "gallon", "gallons"] {
            m.insert(*alias, ("gal", UnitCategory::Volume));
        }
        for alias in &["floz", "fl oz", "fluid ounce", "fluid ounces"] {
            m.insert(*alias, ("floz", UnitCategory::Volume));
        }

        // Temperature units
        for alias in &["c", "celsius", "°c", "degc"] {
            m.insert(*alias, ("c", UnitCategory::Temperature));
        }
        for alias in &["f", "fahrenheit", "°f", "degf"] {
            m.insert(*alias, ("f", UnitCategory::Temperature));
        }
        for alias in &["k", "kelvin", "°k", "degk"] {
            m.insert(*alias, ("k", UnitCategory::Temperature));
        }

        // Data units (using decimal prefixes for simplicity)
        for alias in &["b", "byte", "bytes"] {
            m.insert(*alias, ("b", UnitCategory::Data));
        }
        for alias in &["kb", "kilobyte", "kilobytes"] {
            m.insert(*alias, ("kb", UnitCategory::Data));
        }
        for alias in &["mb", "megabyte", "megabytes"] {
            m.insert(*alias, ("mb", UnitCategory::Data));
        }
        for alias in &["gb", "gigabyte", "gigabytes"] {
            m.insert(*alias, ("gb", UnitCategory::Data));
        }
        for alias in &["tb", "terabyte", "terabytes"] {
            m.insert(*alias, ("tb", UnitCategory::Data));
        }
        for alias in &["pb", "petabyte", "petabytes"] {
            m.insert(*alias, ("pb", UnitCategory::Data));
        }
        // Binary prefixes
        for alias in &["kib", "kibibyte", "kibibytes"] {
            m.insert(*alias, ("kib", UnitCategory::Data));
        }
        for alias in &["mib", "mebibyte", "mebibytes"] {
            m.insert(*alias, ("mib", UnitCategory::Data));
        }
        for alias in &["gib", "gibibyte", "gibibytes"] {
            m.insert(*alias, ("gib", UnitCategory::Data));
        }
        for alias in &["tib", "tebibyte", "tebibytes"] {
            m.insert(*alias, ("tib", UnitCategory::Data));
        }

        // Speed units
        for alias in &["m/s", "mps", "meters per second", "metres per second"] {
            m.insert(*alias, ("m/s", UnitCategory::Speed));
        }
        for alias in &[
            "km/h",
            "kmh",
            "kph",
            "kilometers per hour",
            "kilometres per hour",
        ] {
            m.insert(*alias, ("km/h", UnitCategory::Speed));
        }
        for alias in &["mph", "miles per hour"] {
            m.insert(*alias, ("mph", UnitCategory::Speed));
        }
        for alias in &["knot", "knots", "kn", "kt"] {
            m.insert(*alias, ("knots", UnitCategory::Speed));
        }
        for alias in &["ft/s", "fps", "feet per second"] {
            m.insert(*alias, ("ft/s", UnitCategory::Speed));
        }

        // Area units
        for alias in &[
            "m²",
            "m2",
            "sqm",
            "square meter",
            "square meters",
            "square metre",
            "square metres",
        ] {
            m.insert(*alias, ("m²", UnitCategory::Area));
        }
        for alias in &[
            "km²",
            "km2",
            "sqkm",
            "square kilometer",
            "square kilometers",
        ] {
            m.insert(*alias, ("km²", UnitCategory::Area));
        }
        for alias in &["ft²", "ft2", "sqft", "square foot", "square feet"] {
            m.insert(*alias, ("ft²", UnitCategory::Area));
        }
        for alias in &["acre", "acres", "ac"] {
            m.insert(*alias, ("acre", UnitCategory::Area));
        }
        for alias in &["hectare", "hectares", "ha"] {
            m.insert(*alias, ("ha", UnitCategory::Area));
        }

        // Time units
        for alias in &["ms", "millisecond", "milliseconds"] {
            m.insert(*alias, ("ms", UnitCategory::Time));
        }
        for alias in &["s", "sec", "second", "seconds"] {
            m.insert(*alias, ("s", UnitCategory::Time));
        }
        for alias in &["min", "minute", "minutes"] {
            m.insert(*alias, ("min", UnitCategory::Time));
        }
        for alias in &["hr", "hour", "hours", "h"] {
            m.insert(*alias, ("hr", UnitCategory::Time));
        }
        for alias in &["day", "days", "d"] {
            m.insert(*alias, ("day", UnitCategory::Time));
        }
        for alias in &["week", "weeks", "wk"] {
            m.insert(*alias, ("week", UnitCategory::Time));
        }

        m
    });

/// Unit category for grouping compatible units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitCategory {
    /// Length/distance units.
    Length,
    /// Weight/mass units.
    Weight,
    /// Volume units.
    Volume,
    /// Temperature units.
    Temperature,
    /// Digital data units.
    Data,
    /// Speed units.
    Speed,
    /// Area units.
    Area,
    /// Time duration units.
    Time,
}

impl UnitCategory {
    /// Returns the display name for this category.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Weight => "Weight",
            Self::Volume => "Volume",
            Self::Temperature => "Temperature",
            Self::Data => "Data",
            Self::Speed => "Speed",
            Self::Area => "Area",
            Self::Time => "Time",
        }
    }
}

/// Unit converter.
#[derive(Debug)]
pub struct UnitConverter {
    /// Conversion factors to base unit for each unit.
    /// For temperature, this is handled specially.
    factors: HashMap<&'static str, f64>,
    /// Base unit for each category.
    base_units: HashMap<UnitCategory, &'static str>,
}

impl UnitConverter {
    /// Creates a new unit converter with all conversion factors.
    #[must_use]
    pub fn new() -> Self {
        let mut factors = HashMap::new();
        let mut base_units = HashMap::new();

        // Length (base: meters)
        base_units.insert(UnitCategory::Length, "m");
        factors.insert("mm", 0.001);
        factors.insert("cm", 0.01);
        factors.insert("m", 1.0);
        factors.insert("km", 1_000.0);
        factors.insert("in", 0.025_4);
        factors.insert("ft", 0.304_8);
        factors.insert("yd", 0.914_4);
        factors.insert("mi", 1_609.344);
        factors.insert("nm", 1_852.0); // nautical mile

        // Weight (base: grams)
        base_units.insert(UnitCategory::Weight, "g");
        factors.insert("mg", 0.001);
        factors.insert("g", 1.0);
        factors.insert("kg", 1_000.0);
        factors.insert("oz", 28.349_523_125);
        factors.insert("lb", 453.592_37);
        factors.insert("ton", 1_000_000.0); // metric ton
        factors.insert("st", 6_350.293_18); // stone

        // Volume (base: milliliters)
        base_units.insert(UnitCategory::Volume, "ml");
        factors.insert("ml", 1.0);
        factors.insert("l", 1_000.0);
        factors.insert("tsp", 4.928_92); // US teaspoon
        factors.insert("tbsp", 14.786_8); // US tablespoon
        factors.insert("cup", 236.588); // US cup
        factors.insert("pt", 473.176); // US pint
        factors.insert("qt", 946.353); // US quart
        factors.insert("gal", 3_785.41); // US gallon
        factors.insert("floz", 29.573_5); // US fluid ounce

        // Data (base: bytes) - using decimal prefixes
        base_units.insert(UnitCategory::Data, "b");
        factors.insert("b", 1.0);
        factors.insert("kb", 1_000.0);
        factors.insert("mb", 1_000_000.0);
        factors.insert("gb", 1_000_000_000.0);
        factors.insert("tb", 1_000_000_000_000.0);
        factors.insert("pb", 1_000_000_000_000_000.0);
        // Binary prefixes
        factors.insert("kib", 1_024.0);
        factors.insert("mib", 1_048_576.0);
        factors.insert("gib", 1_073_741_824.0);
        factors.insert("tib", 1_099_511_627_776.0);

        // Speed (base: meters per second)
        base_units.insert(UnitCategory::Speed, "m/s");
        factors.insert("m/s", 1.0);
        factors.insert("km/h", 0.277_778);
        factors.insert("mph", 0.447_04);
        factors.insert("knots", 0.514_444);
        factors.insert("ft/s", 0.304_8);

        // Area (base: square meters)
        base_units.insert(UnitCategory::Area, "m²");
        factors.insert("m²", 1.0);
        factors.insert("km²", 1_000_000.0);
        factors.insert("ft²", 0.092_903);
        factors.insert("acre", 4_046.86);
        factors.insert("ha", 10_000.0);

        // Time (base: seconds)
        base_units.insert(UnitCategory::Time, "s");
        factors.insert("ms", 0.001);
        factors.insert("s", 1.0);
        factors.insert("min", 60.0);
        factors.insert("hr", 3_600.0);
        factors.insert("day", 86_400.0);
        factors.insert("week", 604_800.0);

        // Temperature is handled specially (not a linear conversion)
        base_units.insert(UnitCategory::Temperature, "c");

        Self {
            factors,
            base_units,
        }
    }

    /// Converts a value from one unit to another.
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Result<f64> {
        let from_lower = from.to_lowercase();
        let to_lower = to.to_lowercase();

        // Look up canonical forms
        let (from_canonical, from_category) = UNIT_PATTERNS
            .get(from_lower.as_str())
            .ok_or_else(|| CalculatorError::UnsupportedUnit(from.to_string()))?;

        let (to_canonical, to_category) = UNIT_PATTERNS
            .get(to_lower.as_str())
            .ok_or_else(|| CalculatorError::UnsupportedUnit(to.to_string()))?;

        // Check compatibility
        if from_category != to_category {
            return Err(CalculatorError::IncompatibleUnits {
                from: from.to_string(),
                to: to.to_string(),
            });
        }

        // Handle temperature specially
        if *from_category == UnitCategory::Temperature {
            return Self::convert_temperature(value, from_canonical, to_canonical);
        }

        // Standard conversion via base unit
        let from_factor = self.factors.get(from_canonical).unwrap_or(&1.0);
        let to_factor = self.factors.get(to_canonical).unwrap_or(&1.0);

        // Convert: from -> base -> to
        let base_value = value * from_factor;
        let result = base_value / to_factor;

        Ok(result)
    }

    /// Converts temperature between Celsius, Fahrenheit, and Kelvin.
    fn convert_temperature(value: f64, from: &str, to: &str) -> Result<f64> {
        // First convert to Celsius
        let celsius = match from {
            "c" => value,
            "f" => (value - 32.0) * 5.0 / 9.0,
            "k" => value - 273.15,
            _ => return Err(CalculatorError::UnsupportedUnit(from.to_string())),
        };

        // Then convert from Celsius to target
        let result = match to {
            "c" => celsius,
            "f" => celsius * 9.0 / 5.0 + 32.0,
            "k" => celsius + 273.15,
            _ => return Err(CalculatorError::UnsupportedUnit(to.to_string())),
        };

        Ok(result)
    }

    /// Returns the category for a unit, if supported.
    pub fn get_category(&self, unit: &str) -> Option<UnitCategory> {
        let lower = unit.to_lowercase();
        UNIT_PATTERNS.get(lower.as_str()).map(|(_, cat)| *cat)
    }

    /// Returns all supported units for a category.
    #[must_use]
    pub fn units_for_category(category: UnitCategory) -> Vec<&'static str> {
        UNIT_PATTERNS
            .iter()
            .filter(|(_, (_, cat))| *cat == category)
            .map(|(alias, _)| *alias)
            .collect()
    }
}

impl Default for UnitConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(from: f64, from_unit: &str, to_unit: &str) -> f64 {
        let converter = UnitConverter::new();
        converter
            .convert(from, from_unit, to_unit)
            .expect("conversion failed")
    }

    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_length_conversions() {
        // Basic conversions
        assert!(approx_eq(convert(1.0, "km", "m"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "m", "cm"), 100.0, 0.001));
        assert!(approx_eq(convert(1.0, "ft", "in"), 12.0, 0.001));
        assert!(approx_eq(convert(1.0, "mi", "km"), 1.609_344, 0.001));
        assert!(approx_eq(convert(1.0, "mi", "ft"), 5_280.0, 0.1));

        // Aliases
        assert!(approx_eq(
            convert(5.0, "kilometers", "miles"),
            3.106_86,
            0.001
        ));
        assert!(approx_eq(convert(1.0, "foot", "inches"), 12.0, 0.001));
    }

    #[test]
    fn test_weight_conversions() {
        assert!(approx_eq(convert(1.0, "kg", "g"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "lb", "oz"), 16.0, 0.001));
        assert!(approx_eq(convert(1.0, "kg", "lb"), 2.204_62, 0.001));
        assert!(approx_eq(convert(1.0, "ton", "kg"), 1_000.0, 0.001));

        // Aliases
        assert!(approx_eq(
            convert(1.0, "kilogram", "pounds"),
            2.204_62,
            0.001
        ));
    }

    #[test]
    fn test_volume_conversions() {
        assert!(approx_eq(convert(1.0, "l", "ml"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "gal", "l"), 3.785_41, 0.001));
        assert!(approx_eq(convert(1.0, "cup", "ml"), 236.588, 0.01));
        assert!(approx_eq(convert(1.0, "tbsp", "tsp"), 3.0, 0.01));
    }

    #[test]
    fn test_temperature_conversions() {
        // Celsius to Fahrenheit
        assert!(approx_eq(convert(0.0, "c", "f"), 32.0, 0.001));
        assert!(approx_eq(convert(100.0, "c", "f"), 212.0, 0.001));
        assert!(approx_eq(convert(-40.0, "c", "f"), -40.0, 0.001));

        // Fahrenheit to Celsius
        assert!(approx_eq(convert(32.0, "f", "c"), 0.0, 0.001));
        assert!(approx_eq(convert(212.0, "f", "c"), 100.0, 0.001));

        // Kelvin
        assert!(approx_eq(convert(0.0, "c", "k"), 273.15, 0.001));
        assert!(approx_eq(convert(273.15, "k", "c"), 0.0, 0.001));

        // Aliases
        assert!(approx_eq(
            convert(100.0, "celsius", "fahrenheit"),
            212.0,
            0.001
        ));
    }

    #[test]
    fn test_data_conversions() {
        assert!(approx_eq(convert(1.0, "kb", "b"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "mb", "kb"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "gb", "mb"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "tb", "gb"), 1_000.0, 0.001));

        // Binary prefixes
        assert!(approx_eq(convert(1.0, "kib", "b"), 1_024.0, 0.001));
        assert!(approx_eq(convert(1.0, "gib", "mib"), 1_024.0, 0.001));
    }

    #[test]
    fn test_speed_conversions() {
        assert!(approx_eq(convert(1.0, "km/h", "m/s"), 0.277_778, 0.001));
        assert!(approx_eq(convert(1.0, "mph", "km/h"), 1.609_34, 0.001));
        assert!(approx_eq(convert(1.0, "knots", "km/h"), 1.852, 0.001));

        // Aliases
        assert!(approx_eq(
            convert(60.0, "miles per hour", "km/h"),
            96.560_6,
            0.01
        ));
    }

    #[test]
    fn test_time_conversions() {
        assert!(approx_eq(convert(1.0, "hr", "min"), 60.0, 0.001));
        assert!(approx_eq(convert(1.0, "day", "hr"), 24.0, 0.001));
        assert!(approx_eq(convert(1.0, "week", "day"), 7.0, 0.001));
    }

    #[test]
    fn test_incompatible_units() {
        let converter = UnitConverter::new();
        let result = converter.convert(1.0, "km", "kg");
        assert!(matches!(
            result,
            Err(CalculatorError::IncompatibleUnits { .. })
        ));
    }

    #[test]
    fn test_unsupported_unit() {
        let converter = UnitConverter::new();
        let result = converter.convert(1.0, "xyz", "m");
        assert!(matches!(result, Err(CalculatorError::UnsupportedUnit(_))));
    }

    #[test]
    fn test_case_insensitivity() {
        assert!(approx_eq(convert(1.0, "KM", "M"), 1_000.0, 0.001));
        assert!(approx_eq(convert(1.0, "Km", "m"), 1_000.0, 0.001));
        assert!(approx_eq(convert(100.0, "F", "C"), 37.777_8, 0.01));
    }
}
