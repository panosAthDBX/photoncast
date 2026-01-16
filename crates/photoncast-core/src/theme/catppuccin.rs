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

    /// Parses an accent color from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rosewater" => Some(Self::Rosewater),
            "flamingo" => Some(Self::Flamingo),
            "pink" => Some(Self::Pink),
            "mauve" => Some(Self::Mauve),
            "red" => Some(Self::Red),
            "maroon" => Some(Self::Maroon),
            "peach" => Some(Self::Peach),
            "yellow" => Some(Self::Yellow),
            "green" => Some(Self::Green),
            "teal" => Some(Self::Teal),
            "sky" => Some(Self::Sky),
            "sapphire" => Some(Self::Sapphire),
            "blue" => Some(Self::Blue),
            "lavender" => Some(Self::Lavender),
            _ => None,
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

    /// Converts to gpui::Hsla.
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
    pub fn mocha() -> Self {
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
    pub fn latte() -> Self {
        Self {
            rosewater: hsla(10.0, 0.58, 0.74, 1.0), // #dc8a78
            flamingo: hsla(0.0, 0.60, 0.67, 1.0),   // #dd7878
            pink: hsla(316.0, 0.73, 0.69, 1.0),     // #ea76cb
            mauve: hsla(266.0, 0.85, 0.58, 1.0),    // #8839ef
            red: hsla(347.0, 0.87, 0.44, 1.0),      // #d20f39
            maroon: hsla(355.0, 0.76, 0.48, 1.0),   // #e64553
            peach: hsla(22.0, 0.99, 0.52, 1.0),     // #fe640b
            yellow: hsla(35.0, 0.77, 0.49, 1.0),    // #df8e1d
            green: hsla(109.0, 0.58, 0.40, 1.0),    // #40a02b
            teal: hsla(183.0, 0.74, 0.35, 1.0),     // #179299
            sky: hsla(197.0, 0.97, 0.46, 1.0),      // #04a5e5
            sapphire: hsla(189.0, 0.70, 0.42, 1.0), // #209fb5
            blue: hsla(220.0, 0.91, 0.54, 1.0),     // #1e66f5
            lavender: hsla(231.0, 0.97, 0.72, 1.0), // #7287fd

            text: hsla(234.0, 0.16, 0.35, 1.0),     // #4c4f69
            subtext1: hsla(233.0, 0.13, 0.41, 1.0), // #5c5f77
            subtext0: hsla(233.0, 0.10, 0.47, 1.0), // #6c6f85
            overlay2: hsla(232.0, 0.10, 0.53, 1.0), // #7c7f93
            overlay1: hsla(231.0, 0.10, 0.59, 1.0), // #8c8fa1
            overlay0: hsla(228.0, 0.11, 0.65, 1.0), // #9ca0b0
            surface2: hsla(227.0, 0.12, 0.71, 1.0), // #acb0be
            surface1: hsla(225.0, 0.14, 0.77, 1.0), // #bcc0cc
            surface0: hsla(223.0, 0.16, 0.83, 1.0), // #ccd0da
            base: hsla(220.0, 0.23, 0.95, 1.0),     // #eff1f5
            mantle: hsla(220.0, 0.22, 0.92, 1.0),   // #e6e9ef
            crust: hsla(220.0, 0.21, 0.89, 1.0),    // #dce0e8
        }
    }

    /// Creates the Frappé (dark, low contrast) palette.
    #[must_use]
    pub fn frappe() -> Self {
        Self {
            rosewater: hsla(10.0, 0.57, 0.88, 1.0), // #f2d5cf
            flamingo: hsla(0.0, 0.59, 0.84, 1.0),   // #eebebe
            pink: hsla(316.0, 0.73, 0.84, 1.0),     // #f4b8e4
            mauve: hsla(277.0, 0.59, 0.76, 1.0),    // #ca9ee6
            red: hsla(359.0, 0.68, 0.71, 1.0),      // #e78284
            maroon: hsla(358.0, 0.66, 0.76, 1.0),   // #ea999c
            peach: hsla(20.0, 0.79, 0.70, 1.0),     // #ef9f76
            yellow: hsla(40.0, 0.62, 0.73, 1.0),    // #e5c890
            green: hsla(96.0, 0.44, 0.68, 1.0),     // #a6d189
            teal: hsla(172.0, 0.39, 0.65, 1.0),     // #81c8be
            sky: hsla(189.0, 0.48, 0.73, 1.0),      // #99d1db
            sapphire: hsla(199.0, 0.55, 0.69, 1.0), // #85c1dc
            blue: hsla(222.0, 0.74, 0.74, 1.0),     // #8caaee
            lavender: hsla(239.0, 0.66, 0.84, 1.0), // #babbf1

            text: hsla(227.0, 0.70, 0.87, 1.0),     // #c6d0f5
            subtext1: hsla(227.0, 0.44, 0.80, 1.0), // #b5bfe2
            subtext0: hsla(228.0, 0.29, 0.70, 1.0), // #a5adce
            overlay2: hsla(228.0, 0.22, 0.63, 1.0), // #949cbb
            overlay1: hsla(227.0, 0.17, 0.54, 1.0), // #838ba7
            overlay0: hsla(229.0, 0.13, 0.45, 1.0), // #737994
            surface2: hsla(228.0, 0.12, 0.39, 1.0), // #626880
            surface1: hsla(227.0, 0.12, 0.32, 1.0), // #51576d
            surface0: hsla(230.0, 0.12, 0.26, 1.0), // #414559
            base: hsla(229.0, 0.19, 0.23, 1.0),     // #303446
            mantle: hsla(231.0, 0.19, 0.20, 1.0),   // #292c3c
            crust: hsla(229.0, 0.20, 0.17, 1.0),    // #232634
        }
    }

    /// Creates the Macchiato (dark, medium contrast) palette.
    #[must_use]
    pub fn macchiato() -> Self {
        Self {
            rosewater: hsla(10.0, 0.58, 0.90, 1.0), // #f4dbd6
            flamingo: hsla(0.0, 0.58, 0.86, 1.0),   // #f0c6c6
            pink: hsla(316.0, 0.74, 0.85, 1.0),     // #f5bde6
            mauve: hsla(267.0, 0.83, 0.80, 1.0),    // #c6a0f6
            red: hsla(351.0, 0.74, 0.73, 1.0),      // #ed8796
            maroon: hsla(355.0, 0.71, 0.77, 1.0),   // #ee99a0
            peach: hsla(21.0, 0.86, 0.73, 1.0),     // #f5a97f
            yellow: hsla(41.0, 0.86, 0.83, 1.0),    // #eed49f
            green: hsla(105.0, 0.48, 0.72, 1.0),    // #a6da95
            teal: hsla(171.0, 0.47, 0.69, 1.0),     // #8bd5ca
            sky: hsla(189.0, 0.59, 0.73, 1.0),      // #91d7e3
            sapphire: hsla(199.0, 0.66, 0.69, 1.0), // #7dc4e4
            blue: hsla(220.0, 0.83, 0.75, 1.0),     // #8aadf4
            lavender: hsla(234.0, 0.82, 0.85, 1.0), // #b7bdf8

            text: hsla(227.0, 0.68, 0.88, 1.0),     // #cad3f5
            subtext1: hsla(228.0, 0.39, 0.80, 1.0), // #b8c0e0
            subtext0: hsla(227.0, 0.27, 0.72, 1.0), // #a5adcb
            overlay2: hsla(228.0, 0.20, 0.63, 1.0), // #939ab7
            overlay1: hsla(228.0, 0.15, 0.55, 1.0), // #8087a2
            overlay0: hsla(230.0, 0.12, 0.47, 1.0), // #6e738d
            surface2: hsla(230.0, 0.14, 0.41, 1.0), // #5b6078
            surface1: hsla(231.0, 0.16, 0.34, 1.0), // #494d64
            surface0: hsla(230.0, 0.19, 0.26, 1.0), // #363a4f
            base: hsla(232.0, 0.23, 0.18, 1.0),     // #24273a
            mantle: hsla(233.0, 0.23, 0.15, 1.0),   // #1e2030
            crust: hsla(236.0, 0.23, 0.12, 1.0),    // #181926
        }
    }

    /// Creates a palette for the given flavor.
    #[must_use]
    pub fn for_flavor(flavor: CatppuccinFlavor) -> Self {
        match flavor {
            CatppuccinFlavor::Latte => Self::latte(),
            CatppuccinFlavor::Frappe => Self::frappe(),
            CatppuccinFlavor::Macchiato => Self::macchiato(),
            CatppuccinFlavor::Mocha => Self::mocha(),
        }
    }

    /// Gets the accent color for the given variant.
    #[must_use]
    pub fn get_accent(&self, accent: AccentColor) -> Hsla {
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

    // Helper to assert color values are within valid HSLA ranges
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

    // Helper to validate all colors in a palette
    fn validate_palette(palette: &CatppuccinPalette) {
        // Validate all accent colors
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

        // Validate all surface colors
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
    fn test_latte_palette_loads_correctly() {
        let palette = CatppuccinPalette::latte();
        validate_palette(&palette);

        // Verify characteristic Latte colors (light theme)
        // Base should be very light (high lightness)
        assert!(palette.base.l > 0.9, "Latte base should be very light");
        // Text should be dark (low lightness)
        assert!(palette.text.l < 0.5, "Latte text should be dark");
    }

    #[test]
    fn test_frappe_palette_loads_correctly() {
        let palette = CatppuccinPalette::frappe();
        validate_palette(&palette);

        // Verify characteristic Frappé colors (dark theme, low contrast)
        // Base should be dark
        assert!(palette.base.l < 0.3, "Frappé base should be dark");
        // Text should be light
        assert!(palette.text.l > 0.8, "Frappé text should be light");
    }

    #[test]
    fn test_macchiato_palette_loads_correctly() {
        let palette = CatppuccinPalette::macchiato();
        validate_palette(&palette);

        // Verify characteristic Macchiato colors (dark theme, medium contrast)
        // Base should be darker than Frappé
        assert!(palette.base.l < 0.25, "Macchiato base should be quite dark");
        // Text should be light
        assert!(palette.text.l > 0.8, "Macchiato text should be light");
    }

    #[test]
    fn test_mocha_palette_loads_correctly() {
        let palette = CatppuccinPalette::mocha();
        validate_palette(&palette);

        // Verify characteristic Mocha colors (dark theme, high contrast)
        // Base should be very dark
        assert!(palette.base.l < 0.2, "Mocha base should be very dark");
        // Text should be light
        assert!(palette.text.l > 0.85, "Mocha text should be light");
    }

    #[test]
    fn test_for_flavor_returns_correct_palette() {
        // Test all flavors return distinct palettes
        let latte = CatppuccinPalette::for_flavor(CatppuccinFlavor::Latte);
        let frappe = CatppuccinPalette::for_flavor(CatppuccinFlavor::Frappe);
        let macchiato = CatppuccinPalette::for_flavor(CatppuccinFlavor::Macchiato);
        let mocha = CatppuccinPalette::for_flavor(CatppuccinFlavor::Mocha);

        // Latte should be light, others dark
        assert!(latte.base.l > 0.9, "Latte should be light");
        assert!(frappe.base.l < 0.3, "Frappé should be dark");
        assert!(macchiato.base.l < 0.3, "Macchiato should be dark");
        assert!(mocha.base.l < 0.2, "Mocha should be darkest");

        // Each dark flavor should have distinct base lightness
        assert!(
            mocha.base.l < macchiato.base.l,
            "Mocha should be darker than Macchiato"
        );
    }

    #[test]
    fn test_flavor_is_dark() {
        assert!(!CatppuccinFlavor::Latte.is_dark(), "Latte should be light");
        assert!(CatppuccinFlavor::Frappe.is_dark(), "Frappé should be dark");
        assert!(
            CatppuccinFlavor::Macchiato.is_dark(),
            "Macchiato should be dark"
        );
        assert!(CatppuccinFlavor::Mocha.is_dark(), "Mocha should be dark");
    }

    #[test]
    fn test_flavor_display_name() {
        assert_eq!(CatppuccinFlavor::Latte.display_name(), "Latte");
        assert_eq!(CatppuccinFlavor::Frappe.display_name(), "Frappé");
        assert_eq!(CatppuccinFlavor::Macchiato.display_name(), "Macchiato");
        assert_eq!(CatppuccinFlavor::Mocha.display_name(), "Mocha");
    }

    #[test]
    fn test_accent_color_from_str() {
        assert_eq!(AccentColor::from_str("mauve"), Some(AccentColor::Mauve));
        assert_eq!(AccentColor::from_str("MAUVE"), Some(AccentColor::Mauve));
        assert_eq!(AccentColor::from_str("Mauve"), Some(AccentColor::Mauve));
        assert_eq!(AccentColor::from_str("blue"), Some(AccentColor::Blue));
        assert_eq!(
            AccentColor::from_str("rosewater"),
            Some(AccentColor::Rosewater)
        );
        assert_eq!(AccentColor::from_str("invalid"), None);
        assert_eq!(AccentColor::from_str(""), None);
    }

    #[test]
    fn test_accent_color_display_name() {
        assert_eq!(AccentColor::Mauve.display_name(), "Mauve");
        assert_eq!(AccentColor::Rosewater.display_name(), "Rosewater");
        assert_eq!(AccentColor::Blue.display_name(), "Blue");
        assert_eq!(AccentColor::Lavender.display_name(), "Lavender");
    }

    #[test]
    fn test_get_accent_returns_correct_color() {
        let palette = CatppuccinPalette::mocha();

        // Verify get_accent returns the right color
        let mauve = palette.get_accent(AccentColor::Mauve);
        assert_eq!(mauve.h, palette.mauve.h);
        assert_eq!(mauve.s, palette.mauve.s);
        assert_eq!(mauve.l, palette.mauve.l);
        assert_eq!(mauve.a, palette.mauve.a);

        let blue = palette.get_accent(AccentColor::Blue);
        assert_eq!(blue.h, palette.blue.h);
    }

    #[test]
    fn test_hsla_with_alpha() {
        let color = hsla(180.0, 0.5, 0.5, 1.0);
        let translucent = color.with_alpha(0.5);

        assert_eq!(translucent.h, color.h);
        assert_eq!(translucent.s, color.s);
        assert_eq!(translucent.l, color.l);
        assert_eq!(translucent.a, 0.5);
    }

    #[test]
    fn test_default_flavor_is_mocha() {
        assert_eq!(CatppuccinFlavor::default(), CatppuccinFlavor::Mocha);
    }

    #[test]
    fn test_default_accent_is_mauve() {
        assert_eq!(AccentColor::default(), AccentColor::Mauve);
    }

    #[test]
    fn test_all_14_accent_colors_exist() {
        let palette = CatppuccinPalette::mocha();
        let accents = [
            AccentColor::Rosewater,
            AccentColor::Flamingo,
            AccentColor::Pink,
            AccentColor::Mauve,
            AccentColor::Red,
            AccentColor::Maroon,
            AccentColor::Peach,
            AccentColor::Yellow,
            AccentColor::Green,
            AccentColor::Teal,
            AccentColor::Sky,
            AccentColor::Sapphire,
            AccentColor::Blue,
            AccentColor::Lavender,
        ];

        assert_eq!(accents.len(), 14, "Should have exactly 14 accent colors");

        for accent in accents {
            let color = palette.get_accent(accent);
            assert_valid_hsla(&color);
        }
    }
}
