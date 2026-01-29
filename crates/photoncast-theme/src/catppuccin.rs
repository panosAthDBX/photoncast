//! Catppuccin color palette definitions.

/// Catppuccin flavor (theme variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatppuccinFlavor {
    /// Light theme.
    Latte,
    /// Dark theme (low contrast).
    Frappe,
    /// Dark theme (medium contrast).
    Macchiato,
    /// Dark theme (high contrast).
    #[default]
    Mocha,
}

impl CatppuccinFlavor {
    /// Returns true if this is a dark flavor.
    #[must_use]
    pub const fn is_dark(&self) -> bool {
        !matches!(self, Self::Latte)
    }

    /// Returns the display name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Latte => "Latte",
            Self::Frappe => "Frappé",
            Self::Macchiato => "Macchiato",
            Self::Mocha => "Mocha",
        }
    }
}

/// Catppuccin accent colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentColor {
    /// Rosewater accent.
    Rosewater,
    /// Flamingo accent.
    Flamingo,
    /// Pink accent.
    Pink,
    /// Mauve accent (default).
    #[default]
    Mauve,
    /// Red accent.
    Red,
    /// Maroon accent.
    Maroon,
    /// Peach accent.
    Peach,
    /// Yellow accent.
    Yellow,
    /// Green accent.
    Green,
    /// Teal accent.
    Teal,
    /// Sky accent.
    Sky,
    /// Sapphire accent.
    Sapphire,
    /// Blue accent.
    Blue,
    /// Lavender accent.
    Lavender,
}

impl AccentColor {
    /// Returns the display name.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Rosewater => "Rosewater",
            Self::Flamingo => "Flamingo",
            Self::Pink => "Pink",
            Self::Mauve => "Mauve",
            Self::Red => "Red",
            Self::Maroon => "Maroon",
            Self::Peach => "Peach",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Teal => "Teal",
            Self::Sky => "Sky",
            Self::Sapphire => "Sapphire",
            Self::Blue => "Blue",
            Self::Lavender => "Lavender",
        }
    }
}

impl std::str::FromStr for AccentColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rosewater" => Ok(Self::Rosewater),
            "flamingo" => Ok(Self::Flamingo),
            "pink" => Ok(Self::Pink),
            "mauve" => Ok(Self::Mauve),
            "red" => Ok(Self::Red),
            "maroon" => Ok(Self::Maroon),
            "peach" => Ok(Self::Peach),
            "yellow" => Ok(Self::Yellow),
            "green" => Ok(Self::Green),
            "teal" => Ok(Self::Teal),
            "sky" => Ok(Self::Sky),
            "sapphire" => Ok(Self::Sapphire),
            "blue" => Ok(Self::Blue),
            "lavender" => Ok(Self::Lavender),
            _ => Err(()),
        }
    }
}

/// HSLA color representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsla {
    /// Hue (0.0 to 1.0).
    pub h: f32,
    /// Saturation (0.0 to 1.0).
    pub s: f32,
    /// Lightness (0.0 to 1.0).
    pub l: f32,
    /// Alpha (0.0 to 1.0).
    pub a: f32,
}

impl Hsla {
    /// Creates a new HSLA color.
    #[must_use]
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Returns this color with a different alpha value.
    #[must_use]
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// Converts to `gpui::Hsla`.
    #[must_use]
    pub fn to_gpui(self) -> gpui::Hsla {
        gpui::hsla(self.h, self.s, self.l, self.a)
    }
}

impl From<Hsla> for gpui::Hsla {
    fn from(color: Hsla) -> Self {
        gpui::hsla(color.h, color.s, color.l, color.a)
    }
}

/// Helper to create HSLA from degree-based hue.
#[must_use]
pub const fn hsla(h_degrees: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla::new(h_degrees / 360.0, s, l, a)
}

/// The full Catppuccin color palette.
#[derive(Debug, Clone)]
pub struct CatppuccinPalette {
    // Accent colors
    pub rosewater: Hsla,
    pub flamingo: Hsla,
    pub pink: Hsla,
    pub mauve: Hsla,
    pub red: Hsla,
    pub maroon: Hsla,
    pub peach: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub teal: Hsla,
    pub sky: Hsla,
    pub sapphire: Hsla,
    pub blue: Hsla,
    pub lavender: Hsla,

    // Surface colors
    pub text: Hsla,
    pub subtext1: Hsla,
    pub subtext0: Hsla,
    pub overlay2: Hsla,
    pub overlay1: Hsla,
    pub overlay0: Hsla,
    pub surface2: Hsla,
    pub surface1: Hsla,
    pub surface0: Hsla,
    pub base: Hsla,
    pub mantle: Hsla,
    pub crust: Hsla,
}

impl CatppuccinPalette {
    /// Creates the Mocha (dark, high contrast) palette.
    #[must_use]
    pub const fn mocha() -> Self {
        Self {
            rosewater: hsla(10.0, 0.56, 0.91, 1.0),
            flamingo: hsla(0.0, 0.59, 0.88, 1.0),
            pink: hsla(316.0, 0.72, 0.86, 1.0),
            mauve: hsla(267.0, 0.84, 0.81, 1.0),
            red: hsla(343.0, 0.81, 0.75, 1.0),
            maroon: hsla(350.0, 0.65, 0.77, 1.0),
            peach: hsla(23.0, 0.92, 0.75, 1.0),
            yellow: hsla(41.0, 0.86, 0.83, 1.0),
            green: hsla(115.0, 0.54, 0.76, 1.0),
            teal: hsla(170.0, 0.57, 0.73, 1.0),
            sky: hsla(189.0, 0.71, 0.73, 1.0),
            sapphire: hsla(199.0, 0.76, 0.69, 1.0),
            blue: hsla(217.0, 0.92, 0.76, 1.0),
            lavender: hsla(232.0, 0.97, 0.85, 1.0),

            text: hsla(226.0, 0.64, 0.88, 1.0),
            subtext1: hsla(227.0, 0.35, 0.80, 1.0),
            subtext0: hsla(228.0, 0.24, 0.72, 1.0),
            overlay2: hsla(228.0, 0.17, 0.64, 1.0),
            overlay1: hsla(227.0, 0.13, 0.55, 1.0),
            overlay0: hsla(229.0, 0.11, 0.47, 1.0),
            surface2: hsla(228.0, 0.13, 0.40, 1.0),
            surface1: hsla(227.0, 0.15, 0.32, 1.0),
            surface0: hsla(230.0, 0.19, 0.23, 1.0),
            base: hsla(240.0, 0.21, 0.15, 1.0),
            mantle: hsla(240.0, 0.21, 0.12, 1.0),
            crust: hsla(240.0, 0.23, 0.09, 1.0),
        }
    }

    /// Creates the Latte (light) palette.
    #[must_use]
    pub const fn latte() -> Self {
        Self {
            rosewater: hsla(10.0, 0.58, 0.74, 1.0),
            flamingo: hsla(0.0, 0.60, 0.67, 1.0),
            pink: hsla(316.0, 0.73, 0.69, 1.0),
            mauve: hsla(266.0, 0.85, 0.58, 1.0),
            red: hsla(347.0, 0.87, 0.44, 1.0),
            maroon: hsla(355.0, 0.76, 0.48, 1.0),
            peach: hsla(22.0, 0.99, 0.52, 1.0),
            yellow: hsla(35.0, 0.77, 0.49, 1.0),
            green: hsla(109.0, 0.58, 0.40, 1.0),
            teal: hsla(183.0, 0.74, 0.35, 1.0),
            sky: hsla(197.0, 0.97, 0.46, 1.0),
            sapphire: hsla(189.0, 0.70, 0.42, 1.0),
            blue: hsla(220.0, 0.91, 0.54, 1.0),
            lavender: hsla(231.0, 0.97, 0.72, 1.0),

            text: hsla(234.0, 0.16, 0.35, 1.0),
            subtext1: hsla(233.0, 0.13, 0.41, 1.0),
            subtext0: hsla(233.0, 0.10, 0.47, 1.0),
            overlay2: hsla(232.0, 0.10, 0.53, 1.0),
            overlay1: hsla(231.0, 0.10, 0.59, 1.0),
            overlay0: hsla(228.0, 0.11, 0.65, 1.0),
            surface2: hsla(227.0, 0.12, 0.71, 1.0),
            surface1: hsla(225.0, 0.14, 0.77, 1.0),
            surface0: hsla(223.0, 0.16, 0.83, 1.0),
            base: hsla(220.0, 0.23, 0.95, 1.0),
            mantle: hsla(220.0, 0.22, 0.92, 1.0),
            crust: hsla(220.0, 0.21, 0.89, 1.0),
        }
    }

    /// Creates the Frappé (dark, low contrast) palette.
    #[must_use]
    pub const fn frappe() -> Self {
        Self {
            rosewater: hsla(10.0, 0.57, 0.88, 1.0),
            flamingo: hsla(0.0, 0.59, 0.84, 1.0),
            pink: hsla(316.0, 0.73, 0.84, 1.0),
            mauve: hsla(277.0, 0.59, 0.76, 1.0),
            red: hsla(359.0, 0.68, 0.71, 1.0),
            maroon: hsla(358.0, 0.66, 0.76, 1.0),
            peach: hsla(20.0, 0.79, 0.70, 1.0),
            yellow: hsla(40.0, 0.62, 0.73, 1.0),
            green: hsla(96.0, 0.44, 0.68, 1.0),
            teal: hsla(172.0, 0.39, 0.65, 1.0),
            sky: hsla(189.0, 0.48, 0.73, 1.0),
            sapphire: hsla(199.0, 0.55, 0.69, 1.0),
            blue: hsla(222.0, 0.74, 0.74, 1.0),
            lavender: hsla(239.0, 0.66, 0.84, 1.0),

            text: hsla(227.0, 0.70, 0.87, 1.0),
            subtext1: hsla(227.0, 0.44, 0.80, 1.0),
            subtext0: hsla(228.0, 0.29, 0.70, 1.0),
            overlay2: hsla(228.0, 0.22, 0.63, 1.0),
            overlay1: hsla(227.0, 0.17, 0.54, 1.0),
            overlay0: hsla(229.0, 0.13, 0.45, 1.0),
            surface2: hsla(228.0, 0.12, 0.39, 1.0),
            surface1: hsla(227.0, 0.12, 0.32, 1.0),
            surface0: hsla(230.0, 0.12, 0.26, 1.0),
            base: hsla(229.0, 0.19, 0.23, 1.0),
            mantle: hsla(231.0, 0.19, 0.20, 1.0),
            crust: hsla(229.0, 0.20, 0.17, 1.0),
        }
    }

    /// Creates the Macchiato (dark, medium contrast) palette.
    #[must_use]
    pub const fn macchiato() -> Self {
        Self {
            rosewater: hsla(10.0, 0.58, 0.90, 1.0),
            flamingo: hsla(0.0, 0.58, 0.86, 1.0),
            pink: hsla(316.0, 0.74, 0.85, 1.0),
            mauve: hsla(267.0, 0.83, 0.80, 1.0),
            red: hsla(351.0, 0.74, 0.73, 1.0),
            maroon: hsla(355.0, 0.71, 0.77, 1.0),
            peach: hsla(21.0, 0.86, 0.73, 1.0),
            yellow: hsla(41.0, 0.86, 0.83, 1.0),
            green: hsla(105.0, 0.48, 0.72, 1.0),
            teal: hsla(171.0, 0.47, 0.69, 1.0),
            sky: hsla(189.0, 0.59, 0.73, 1.0),
            sapphire: hsla(199.0, 0.66, 0.69, 1.0),
            blue: hsla(220.0, 0.83, 0.75, 1.0),
            lavender: hsla(234.0, 0.82, 0.85, 1.0),

            text: hsla(227.0, 0.68, 0.88, 1.0),
            subtext1: hsla(228.0, 0.39, 0.80, 1.0),
            subtext0: hsla(227.0, 0.27, 0.72, 1.0),
            overlay2: hsla(228.0, 0.20, 0.63, 1.0),
            overlay1: hsla(228.0, 0.15, 0.55, 1.0),
            overlay0: hsla(230.0, 0.12, 0.47, 1.0),
            surface2: hsla(230.0, 0.14, 0.41, 1.0),
            surface1: hsla(231.0, 0.16, 0.34, 1.0),
            surface0: hsla(230.0, 0.19, 0.26, 1.0),
            base: hsla(232.0, 0.23, 0.18, 1.0),
            mantle: hsla(233.0, 0.23, 0.15, 1.0),
            crust: hsla(236.0, 0.23, 0.12, 1.0),
        }
    }

    /// Creates a palette for the given flavor.
    #[must_use]
    pub const fn for_flavor(flavor: CatppuccinFlavor) -> Self {
        match flavor {
            CatppuccinFlavor::Latte => Self::latte(),
            CatppuccinFlavor::Frappe => Self::frappe(),
            CatppuccinFlavor::Macchiato => Self::macchiato(),
            CatppuccinFlavor::Mocha => Self::mocha(),
        }
    }

    /// Gets the accent color for the given variant.
    #[must_use]
    pub const fn get_accent(&self, accent: AccentColor) -> Hsla {
        match accent {
            AccentColor::Rosewater => self.rosewater,
            AccentColor::Flamingo => self.flamingo,
            AccentColor::Pink => self.pink,
            AccentColor::Mauve => self.mauve,
            AccentColor::Red => self.red,
            AccentColor::Maroon => self.maroon,
            AccentColor::Peach => self.peach,
            AccentColor::Yellow => self.yellow,
            AccentColor::Green => self.green,
            AccentColor::Teal => self.teal,
            AccentColor::Sky => self.sky,
            AccentColor::Sapphire => self.sapphire,
            AccentColor::Blue => self.blue,
            AccentColor::Lavender => self.lavender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_hsla(color: &Hsla) {
        assert!(
            (0.0..=1.0).contains(&color.h),
            "Hue {} should be 0-1",
            color.h
        );
        assert!(
            (0.0..=1.0).contains(&color.s),
            "Saturation {} should be 0-1",
            color.s
        );
        assert!(
            (0.0..=1.0).contains(&color.l),
            "Lightness {} should be 0-1",
            color.l
        );
        assert!(
            (0.0..=1.0).contains(&color.a),
            "Alpha {} should be 0-1",
            color.a
        );
    }

    fn validate_palette(palette: &CatppuccinPalette) {
        assert_valid_hsla(&palette.rosewater);
        assert_valid_hsla(&palette.flamingo);
        assert_valid_hsla(&palette.pink);
        assert_valid_hsla(&palette.mauve);
        assert_valid_hsla(&palette.red);
        assert_valid_hsla(&palette.maroon);
        assert_valid_hsla(&palette.peach);
        assert_valid_hsla(&palette.yellow);
        assert_valid_hsla(&palette.green);
        assert_valid_hsla(&palette.teal);
        assert_valid_hsla(&palette.sky);
        assert_valid_hsla(&palette.sapphire);
        assert_valid_hsla(&palette.blue);
        assert_valid_hsla(&palette.lavender);

        assert_valid_hsla(&palette.text);
        assert_valid_hsla(&palette.subtext1);
        assert_valid_hsla(&palette.subtext0);
        assert_valid_hsla(&palette.overlay2);
        assert_valid_hsla(&palette.overlay1);
        assert_valid_hsla(&palette.overlay0);
        assert_valid_hsla(&palette.surface2);
        assert_valid_hsla(&palette.surface1);
        assert_valid_hsla(&palette.surface0);
        assert_valid_hsla(&palette.base);
        assert_valid_hsla(&palette.mantle);
        assert_valid_hsla(&palette.crust);
    }

    #[test]
    fn test_all_palettes_valid() {
        validate_palette(&CatppuccinPalette::latte());
        validate_palette(&CatppuccinPalette::frappe());
        validate_palette(&CatppuccinPalette::macchiato());
        validate_palette(&CatppuccinPalette::mocha());
    }

    #[test]
    fn test_flavor_is_dark() {
        assert!(!CatppuccinFlavor::Latte.is_dark());
        assert!(CatppuccinFlavor::Frappe.is_dark());
        assert!(CatppuccinFlavor::Macchiato.is_dark());
        assert!(CatppuccinFlavor::Mocha.is_dark());
    }

    #[test]
    fn test_accent_color_from_str() {
        assert_eq!("mauve".parse::<AccentColor>(), Ok(AccentColor::Mauve));
        assert_eq!("MAUVE".parse::<AccentColor>(), Ok(AccentColor::Mauve));
        assert!("invalid".parse::<AccentColor>().is_err());
    }

    #[test]
    fn test_hsla_with_alpha() {
        let color = hsla(180.0, 0.5, 0.5, 1.0);
        let translucent = color.with_alpha(0.5);
        assert!((translucent.a - 0.5).abs() < f32::EPSILON);
    }
}
