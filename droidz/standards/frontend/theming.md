# PhotonCast Theming

> Theme system with Catppuccin color palettes

## Overview

PhotonCast uses a comprehensive theme system based on **Catppuccin**, a soothing pastel theme with four flavors ranging from light to dark. The theme system is designed for consistency, accessibility, and ease of customization.

---

## Catppuccin Flavors

| Flavor | Mode | Description |
|--------|------|-------------|
| **Latte** | Light | Warm, creamy light theme |
| **Frappé** | Dark | Muted, low-contrast dark |
| **Macchiato** | Dark | Medium contrast dark |
| **Mocha** | Dark | High contrast, deep dark |

---

## Color Palette

### Latte (Light Mode)

```rust
pub struct CatppuccinLatte;

impl Theme for CatppuccinLatte {
    // Base colors
    const ROSEWATER: Hsla = hsla(10.0 / 360.0, 0.58, 0.74, 1.0);   // #dc8a78
    const FLAMINGO: Hsla = hsla(0.0 / 360.0, 0.60, 0.67, 1.0);     // #dd7878
    const PINK: Hsla = hsla(316.0 / 360.0, 0.73, 0.69, 1.0);       // #ea76cb
    const MAUVE: Hsla = hsla(266.0 / 360.0, 0.85, 0.58, 1.0);      // #8839ef
    const RED: Hsla = hsla(347.0 / 360.0, 0.87, 0.44, 1.0);        // #d20f39
    const MAROON: Hsla = hsla(355.0 / 360.0, 0.76, 0.48, 1.0);     // #e64553
    const PEACH: Hsla = hsla(22.0 / 360.0, 0.99, 0.52, 1.0);       // #fe640b
    const YELLOW: Hsla = hsla(35.0 / 360.0, 0.77, 0.49, 1.0);      // #df8e1d
    const GREEN: Hsla = hsla(109.0 / 360.0, 0.58, 0.40, 1.0);      // #40a02b
    const TEAL: Hsla = hsla(183.0 / 360.0, 0.74, 0.35, 1.0);       // #179299
    const SKY: Hsla = hsla(197.0 / 360.0, 0.97, 0.46, 1.0);        // #04a5e5
    const SAPPHIRE: Hsla = hsla(189.0 / 360.0, 0.70, 0.42, 1.0);   // #209fb5
    const BLUE: Hsla = hsla(220.0 / 360.0, 0.91, 0.54, 1.0);       // #1e66f5
    const LAVENDER: Hsla = hsla(231.0 / 360.0, 0.97, 0.72, 1.0);   // #7287fd
    
    // Surface colors
    const TEXT: Hsla = hsla(234.0 / 360.0, 0.16, 0.35, 1.0);       // #4c4f69
    const SUBTEXT1: Hsla = hsla(233.0 / 360.0, 0.13, 0.41, 1.0);   // #5c5f77
    const SUBTEXT0: Hsla = hsla(233.0 / 360.0, 0.10, 0.47, 1.0);   // #6c6f85
    const OVERLAY2: Hsla = hsla(232.0 / 360.0, 0.10, 0.53, 1.0);   // #7c7f93
    const OVERLAY1: Hsla = hsla(231.0 / 360.0, 0.10, 0.59, 1.0);   // #8c8fa1
    const OVERLAY0: Hsla = hsla(228.0 / 360.0, 0.11, 0.65, 1.0);   // #9ca0b0
    const SURFACE2: Hsla = hsla(227.0 / 360.0, 0.12, 0.71, 1.0);   // #acb0be
    const SURFACE1: Hsla = hsla(225.0 / 360.0, 0.14, 0.77, 1.0);   // #bcc0cc
    const SURFACE0: Hsla = hsla(223.0 / 360.0, 0.16, 0.83, 1.0);   // #ccd0da
    const BASE: Hsla = hsla(220.0 / 360.0, 0.23, 0.95, 1.0);       // #eff1f5
    const MANTLE: Hsla = hsla(220.0 / 360.0, 0.22, 0.92, 1.0);     // #e6e9ef
    const CRUST: Hsla = hsla(220.0 / 360.0, 0.21, 0.89, 1.0);      // #dce0e8
}
```

### Frappé (Dark - Low Contrast)

```rust
pub struct CatppuccinFrappe;

impl Theme for CatppuccinFrappe {
    // Base colors
    const ROSEWATER: Hsla = hsla(10.0 / 360.0, 0.57, 0.88, 1.0);   // #f2d5cf
    const FLAMINGO: Hsla = hsla(0.0 / 360.0, 0.59, 0.84, 1.0);     // #eebebe
    const PINK: Hsla = hsla(316.0 / 360.0, 0.73, 0.84, 1.0);       // #f4b8e4
    const MAUVE: Hsla = hsla(277.0 / 360.0, 0.59, 0.76, 1.0);      // #ca9ee6
    const RED: Hsla = hsla(359.0 / 360.0, 0.68, 0.71, 1.0);        // #e78284
    const MAROON: Hsla = hsla(358.0 / 360.0, 0.66, 0.76, 1.0);     // #ea999c
    const PEACH: Hsla = hsla(20.0 / 360.0, 0.79, 0.70, 1.0);       // #ef9f76
    const YELLOW: Hsla = hsla(40.0 / 360.0, 0.62, 0.73, 1.0);      // #e5c890
    const GREEN: Hsla = hsla(96.0 / 360.0, 0.44, 0.68, 1.0);       // #a6d189
    const TEAL: Hsla = hsla(172.0 / 360.0, 0.39, 0.65, 1.0);       // #81c8be
    const SKY: Hsla = hsla(189.0 / 360.0, 0.48, 0.73, 1.0);        // #99d1db
    const SAPPHIRE: Hsla = hsla(199.0 / 360.0, 0.55, 0.69, 1.0);   // #85c1dc
    const BLUE: Hsla = hsla(222.0 / 360.0, 0.74, 0.74, 1.0);       // #8caaee
    const LAVENDER: Hsla = hsla(239.0 / 360.0, 0.66, 0.84, 1.0);   // #babbf1
    
    // Surface colors
    const TEXT: Hsla = hsla(227.0 / 360.0, 0.70, 0.87, 1.0);       // #c6d0f5
    const SUBTEXT1: Hsla = hsla(227.0 / 360.0, 0.44, 0.80, 1.0);   // #b5bfe2
    const SUBTEXT0: Hsla = hsla(228.0 / 360.0, 0.29, 0.70, 1.0);   // #a5adce
    const OVERLAY2: Hsla = hsla(228.0 / 360.0, 0.22, 0.63, 1.0);   // #949cbb
    const OVERLAY1: Hsla = hsla(227.0 / 360.0, 0.17, 0.54, 1.0);   // #838ba7
    const OVERLAY0: Hsla = hsla(229.0 / 360.0, 0.13, 0.45, 1.0);   // #737994
    const SURFACE2: Hsla = hsla(228.0 / 360.0, 0.12, 0.39, 1.0);   // #626880
    const SURFACE1: Hsla = hsla(227.0 / 360.0, 0.12, 0.32, 1.0);   // #51576d
    const SURFACE0: Hsla = hsla(230.0 / 360.0, 0.12, 0.26, 1.0);   // #414559
    const BASE: Hsla = hsla(229.0 / 360.0, 0.19, 0.23, 1.0);       // #303446
    const MANTLE: Hsla = hsla(231.0 / 360.0, 0.19, 0.20, 1.0);     // #292c3c
    const CRUST: Hsla = hsla(229.0 / 360.0, 0.20, 0.17, 1.0);      // #232634
}
```

### Macchiato (Dark - Medium Contrast)

```rust
pub struct CatppuccinMacchiato;

impl Theme for CatppuccinMacchiato {
    // Base colors
    const ROSEWATER: Hsla = hsla(10.0 / 360.0, 0.58, 0.90, 1.0);   // #f4dbd6
    const FLAMINGO: Hsla = hsla(0.0 / 360.0, 0.58, 0.86, 1.0);     // #f0c6c6
    const PINK: Hsla = hsla(316.0 / 360.0, 0.74, 0.85, 1.0);       // #f5bde6
    const MAUVE: Hsla = hsla(267.0 / 360.0, 0.83, 0.80, 1.0);      // #c6a0f6
    const RED: Hsla = hsla(351.0 / 360.0, 0.74, 0.73, 1.0);        // #ed8796
    const MAROON: Hsla = hsla(355.0 / 360.0, 0.71, 0.77, 1.0);     // #ee99a0
    const PEACH: Hsla = hsla(21.0 / 360.0, 0.86, 0.73, 1.0);       // #f5a97f
    const YELLOW: Hsla = hsla(41.0 / 360.0, 0.86, 0.83, 1.0);      // #eed49f
    const GREEN: Hsla = hsla(105.0 / 360.0, 0.48, 0.72, 1.0);      // #a6da95
    const TEAL: Hsla = hsla(171.0 / 360.0, 0.47, 0.69, 1.0);       // #8bd5ca
    const SKY: Hsla = hsla(189.0 / 360.0, 0.59, 0.73, 1.0);        // #91d7e3
    const SAPPHIRE: Hsla = hsla(199.0 / 360.0, 0.66, 0.69, 1.0);   // #7dc4e4
    const BLUE: Hsla = hsla(220.0 / 360.0, 0.83, 0.75, 1.0);       // #8aadf4
    const LAVENDER: Hsla = hsla(234.0 / 360.0, 0.82, 0.85, 1.0);   // #b7bdf8
    
    // Surface colors
    const TEXT: Hsla = hsla(227.0 / 360.0, 0.68, 0.88, 1.0);       // #cad3f5
    const SUBTEXT1: Hsla = hsla(228.0 / 360.0, 0.39, 0.80, 1.0);   // #b8c0e0
    const SUBTEXT0: Hsla = hsla(227.0 / 360.0, 0.27, 0.72, 1.0);   // #a5adcb
    const OVERLAY2: Hsla = hsla(228.0 / 360.0, 0.20, 0.63, 1.0);   // #939ab7
    const OVERLAY1: Hsla = hsla(228.0 / 360.0, 0.15, 0.55, 1.0);   // #8087a2
    const OVERLAY0: Hsla = hsla(230.0 / 360.0, 0.12, 0.47, 1.0);   // #6e738d
    const SURFACE2: Hsla = hsla(230.0 / 360.0, 0.14, 0.41, 1.0);   // #5b6078
    const SURFACE1: Hsla = hsla(231.0 / 360.0, 0.16, 0.34, 1.0);   // #494d64
    const SURFACE0: Hsla = hsla(230.0 / 360.0, 0.19, 0.26, 1.0);   // #363a4f
    const BASE: Hsla = hsla(232.0 / 360.0, 0.23, 0.18, 1.0);       // #24273a
    const MANTLE: Hsla = hsla(233.0 / 360.0, 0.23, 0.15, 1.0);     // #1e2030
    const CRUST: Hsla = hsla(236.0 / 360.0, 0.23, 0.12, 1.0);      // #181926
}
```

### Mocha (Dark - High Contrast)

```rust
pub struct CatppuccinMocha;

impl Theme for CatppuccinMocha {
    // Base colors
    const ROSEWATER: Hsla = hsla(10.0 / 360.0, 0.56, 0.91, 1.0);   // #f5e0dc
    const FLAMINGO: Hsla = hsla(0.0 / 360.0, 0.59, 0.88, 1.0);     // #f2cdcd
    const PINK: Hsla = hsla(316.0 / 360.0, 0.72, 0.86, 1.0);       // #f5c2e7
    const MAUVE: Hsla = hsla(267.0 / 360.0, 0.84, 0.81, 1.0);      // #cba6f7
    const RED: Hsla = hsla(343.0 / 360.0, 0.81, 0.75, 1.0);        // #f38ba8
    const MAROON: Hsla = hsla(350.0 / 360.0, 0.65, 0.77, 1.0);     // #eba0ac
    const PEACH: Hsla = hsla(23.0 / 360.0, 0.92, 0.75, 1.0);       // #fab387
    const YELLOW: Hsla = hsla(41.0 / 360.0, 0.86, 0.83, 1.0);      // #f9e2af
    const GREEN: Hsla = hsla(115.0 / 360.0, 0.54, 0.76, 1.0);      // #a6e3a1
    const TEAL: Hsla = hsla(170.0 / 360.0, 0.57, 0.73, 1.0);       // #94e2d5
    const SKY: Hsla = hsla(189.0 / 360.0, 0.71, 0.73, 1.0);        // #89dceb
    const SAPPHIRE: Hsla = hsla(199.0 / 360.0, 0.76, 0.69, 1.0);   // #74c7ec
    const BLUE: Hsla = hsla(217.0 / 360.0, 0.92, 0.76, 1.0);       // #89b4fa
    const LAVENDER: Hsla = hsla(232.0 / 360.0, 0.97, 0.85, 1.0);   // #b4befe
    
    // Surface colors
    const TEXT: Hsla = hsla(226.0 / 360.0, 0.64, 0.88, 1.0);       // #cdd6f4
    const SUBTEXT1: Hsla = hsla(227.0 / 360.0, 0.35, 0.80, 1.0);   // #bac2de
    const SUBTEXT0: Hsla = hsla(228.0 / 360.0, 0.24, 0.72, 1.0);   // #a6adc8
    const OVERLAY2: Hsla = hsla(228.0 / 360.0, 0.17, 0.64, 1.0);   // #9399b2
    const OVERLAY1: Hsla = hsla(227.0 / 360.0, 0.13, 0.55, 1.0);   // #7f849c
    const OVERLAY0: Hsla = hsla(229.0 / 360.0, 0.11, 0.47, 1.0);   // #6c7086
    const SURFACE2: Hsla = hsla(228.0 / 360.0, 0.13, 0.40, 1.0);   // #585b70
    const SURFACE1: Hsla = hsla(227.0 / 360.0, 0.15, 0.32, 1.0);   // #45475a
    const SURFACE0: Hsla = hsla(230.0 / 360.0, 0.19, 0.23, 1.0);   // #313244
    const BASE: Hsla = hsla(240.0 / 360.0, 0.21, 0.15, 1.0);       // #1e1e2e
    const MANTLE: Hsla = hsla(240.0 / 360.0, 0.21, 0.12, 1.0);     // #181825
    const CRUST: Hsla = hsla(240.0 / 360.0, 0.23, 0.09, 1.0);      // #11111b
}
```

---

## Semantic Color Mapping

Map Catppuccin colors to semantic UI roles:

```rust
pub trait ThemeColors {
    // Backgrounds
    fn background() -> Hsla;           // BASE
    fn background_elevated() -> Hsla;  // SURFACE0
    fn background_modal() -> Hsla;     // MANTLE
    
    // Surfaces
    fn surface() -> Hsla;              // SURFACE0
    fn surface_hover() -> Hsla;        // SURFACE1
    fn surface_active() -> Hsla;       // SURFACE2
    fn surface_selected() -> Hsla;     // Use accent with alpha
    
    // Text
    fn text() -> Hsla;                 // TEXT
    fn text_secondary() -> Hsla;       // SUBTEXT1
    fn text_muted() -> Hsla;           // SUBTEXT0
    fn text_placeholder() -> Hsla;     // OVERLAY1
    
    // Borders
    fn border() -> Hsla;               // SURFACE1
    fn border_focused() -> Hsla;       // ACCENT
    fn border_error() -> Hsla;         // RED
    
    // Accent (primary action color)
    fn accent() -> Hsla;               // MAUVE (default), customizable
    fn accent_hover() -> Hsla;         // Lighter variant
    fn accent_text() -> Hsla;          // BASE (for text on accent bg)
    
    // Status colors
    fn success() -> Hsla;              // GREEN
    fn warning() -> Hsla;              // YELLOW
    fn error() -> Hsla;                // RED
    fn info() -> Hsla;                 // BLUE
    
    // Interactive states
    fn selection() -> Hsla;            // ACCENT with 20% alpha
    fn hover() -> Hsla;                // SURFACE1
    fn focus_ring() -> Hsla;           // ACCENT with 50% alpha
    
    // Icon colors
    fn icon() -> Hsla;                 // SUBTEXT0
    fn icon_accent() -> Hsla;          // ACCENT
    fn icon_muted() -> Hsla;           // OVERLAY1
    
    // Shadows (for dark themes)
    fn shadow() -> Hsla;               // CRUST with alpha
}

impl ThemeColors for CatppuccinMocha {
    fn background() -> Hsla { Self::BASE }
    fn background_elevated() -> Hsla { Self::SURFACE0 }
    fn background_modal() -> Hsla { Self::MANTLE }
    
    fn surface() -> Hsla { Self::SURFACE0 }
    fn surface_hover() -> Hsla { Self::SURFACE1 }
    fn surface_active() -> Hsla { Self::SURFACE2 }
    fn surface_selected() -> Hsla { 
        Self::MAUVE.with_alpha(0.2) 
    }
    
    fn text() -> Hsla { Self::TEXT }
    fn text_secondary() -> Hsla { Self::SUBTEXT1 }
    fn text_muted() -> Hsla { Self::SUBTEXT0 }
    fn text_placeholder() -> Hsla { Self::OVERLAY1 }
    
    fn border() -> Hsla { Self::SURFACE1 }
    fn border_focused() -> Hsla { Self::MAUVE }
    fn border_error() -> Hsla { Self::RED }
    
    fn accent() -> Hsla { Self::MAUVE }
    fn accent_hover() -> Hsla { Self::LAVENDER }
    fn accent_text() -> Hsla { Self::BASE }
    
    fn success() -> Hsla { Self::GREEN }
    fn warning() -> Hsla { Self::YELLOW }
    fn error() -> Hsla { Self::RED }
    fn info() -> Hsla { Self::BLUE }
    
    fn selection() -> Hsla { Self::MAUVE.with_alpha(0.2) }
    fn hover() -> Hsla { Self::SURFACE1 }
    fn focus_ring() -> Hsla { Self::MAUVE.with_alpha(0.5) }
    
    fn icon() -> Hsla { Self::SUBTEXT0 }
    fn icon_accent() -> Hsla { Self::MAUVE }
    fn icon_muted() -> Hsla { Self::OVERLAY1 }
    
    fn shadow() -> Hsla { Self::CRUST.with_alpha(0.5) }
}
```

---

## Accent Color Customization

Users can override the default accent color (Mauve) with any Catppuccin color:

```rust
pub enum AccentColor {
    Rosewater,
    Flamingo,
    Pink,
    Mauve,      // Default
    Red,
    Maroon,
    Peach,
    Yellow,
    Green,
    Teal,
    Sky,
    Sapphire,
    Blue,
    Lavender,
}

impl Theme {
    pub fn with_accent(mut self, accent: AccentColor) -> Self {
        self.accent_color = accent;
        self
    }
}
```

---

## GPUI Integration

### Theme Provider

```rust
use gpui::{App, Global, Hsla};

#[derive(Clone)]
pub struct PhotonTheme {
    pub flavor: CatppuccinFlavor,
    pub accent: AccentColor,
    pub colors: ThemeColorSet,
}

#[derive(Clone)]
pub struct ThemeColorSet {
    // All semantic colors pre-calculated
    pub background: Hsla,
    pub background_elevated: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_active: Hsla,
    pub surface_selected: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub border: Hsla,
    pub border_focused: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub error: Hsla,
    pub info: Hsla,
}

impl Global for PhotonTheme {}

pub fn init_theme(cx: &mut App, flavor: CatppuccinFlavor, accent: AccentColor) {
    let theme = PhotonTheme::new(flavor, accent);
    cx.set_global(theme);
}

// Access in components
pub fn theme(cx: &App) -> &PhotonTheme {
    cx.global::<PhotonTheme>()
}
```

### Using Theme in Components

```rust
impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme(cx);
        
        div()
            .flex()
            .items_center()
            .px_4()
            .py_3()
            .bg(theme.colors.surface)
            .border_1()
            .border_color(
                if self.focused {
                    theme.colors.border_focused
                } else {
                    theme.colors.border
                }
            )
            .rounded_lg()
            .child(
                Icon::new(IconName::Search)
                    .color(theme.colors.text_muted)
                    .size_4()
            )
            .child(
                input()
                    .flex_1()
                    .ml_3()
                    .bg(theme.colors.surface)
                    .text_color(theme.colors.text)
                    .placeholder_color(theme.colors.text_muted)
                    .on_focus(cx.listener(|this, _, cx| {
                        this.focused = true;
                        cx.notify();
                    }))
                    .on_blur(cx.listener(|this, _, cx| {
                        this.focused = false;
                        cx.notify();
                    }))
            )
    }
}
```

### List Item Example

```rust
impl RenderOnce for SearchResultItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = theme(cx);
        
        h_flex()
            .gap_3()
            .px_3()
            .py_2()
            .w_full()
            .rounded_md()
            .when(self.selected, |el| {
                el.bg(theme.colors.surface_selected)
            })
            .hover(|el| el.bg(theme.colors.surface_hover))
            .child(
                div()
                    .w_8()
                    .h_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(theme.colors.surface)
                    .child(
                        self.icon
                            .color(theme.colors.icon)
                            .size_5()
                    )
            )
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(theme.colors.text)
                            .truncate()
                            .child(self.title)
                    )
                    .when_some(self.subtitle, |el, subtitle| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(theme.colors.text_secondary)
                                .truncate()
                                .child(subtitle)
                        )
                    })
            )
            .when_some(self.accessory, |el, accessory| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(theme.colors.text_muted)
                        .child(accessory)
                )
            })
    }
}
```

---

## Color Constants (Hex Reference)

### Latte (Light)

| Name | Hex | RGB |
|------|-----|-----|
| Rosewater | `#dc8a78` | `220, 138, 120` |
| Flamingo | `#dd7878` | `221, 120, 120` |
| Pink | `#ea76cb` | `234, 118, 203` |
| Mauve | `#8839ef` | `136, 57, 239` |
| Red | `#d20f39` | `210, 15, 57` |
| Maroon | `#e64553` | `230, 69, 83` |
| Peach | `#fe640b` | `254, 100, 11` |
| Yellow | `#df8e1d` | `223, 142, 29` |
| Green | `#40a02b` | `64, 160, 43` |
| Teal | `#179299` | `23, 146, 153` |
| Sky | `#04a5e5` | `4, 165, 229` |
| Sapphire | `#209fb5` | `32, 159, 181` |
| Blue | `#1e66f5` | `30, 102, 245` |
| Lavender | `#7287fd` | `114, 135, 253` |
| Text | `#4c4f69` | `76, 79, 105` |
| Subtext1 | `#5c5f77` | `92, 95, 119` |
| Subtext0 | `#6c6f85` | `108, 111, 133` |
| Overlay2 | `#7c7f93` | `124, 127, 147` |
| Overlay1 | `#8c8fa1` | `140, 143, 161` |
| Overlay0 | `#9ca0b0` | `156, 160, 176` |
| Surface2 | `#acb0be` | `172, 176, 190` |
| Surface1 | `#bcc0cc` | `188, 192, 204` |
| Surface0 | `#ccd0da` | `204, 208, 218` |
| Base | `#eff1f5` | `239, 241, 245` |
| Mantle | `#e6e9ef` | `230, 233, 239` |
| Crust | `#dce0e8` | `220, 224, 232` |

### Mocha (Dark)

| Name | Hex | RGB |
|------|-----|-----|
| Rosewater | `#f5e0dc` | `245, 224, 220` |
| Flamingo | `#f2cdcd` | `242, 205, 205` |
| Pink | `#f5c2e7` | `245, 194, 231` |
| Mauve | `#cba6f7` | `203, 166, 247` |
| Red | `#f38ba8` | `243, 139, 168` |
| Maroon | `#eba0ac` | `235, 160, 172` |
| Peach | `#fab387` | `250, 179, 135` |
| Yellow | `#f9e2af` | `249, 226, 175` |
| Green | `#a6e3a1` | `166, 227, 161` |
| Teal | `#94e2d5` | `148, 226, 213` |
| Sky | `#89dceb` | `137, 220, 235` |
| Sapphire | `#74c7ec` | `116, 199, 236` |
| Blue | `#89b4fa` | `137, 180, 250` |
| Lavender | `#b4befe` | `180, 190, 254` |
| Text | `#cdd6f4` | `205, 214, 244` |
| Subtext1 | `#bac2de` | `186, 194, 222` |
| Subtext0 | `#a6adc8` | `166, 173, 200` |
| Overlay2 | `#9399b2` | `147, 153, 178` |
| Overlay1 | `#7f849c` | `127, 132, 156` |
| Overlay0 | `#6c7086` | `108, 112, 134` |
| Surface2 | `#585b70` | `88, 91, 112` |
| Surface1 | `#45475a` | `69, 71, 90` |
| Surface0 | `#313244` | `49, 50, 68` |
| Base | `#1e1e2e` | `30, 30, 46` |
| Mantle | `#181825` | `24, 24, 37` |
| Crust | `#11111b` | `17, 17, 27` |

---

## System Theme Detection

Follow macOS appearance automatically:

```rust
use cocoa::appkit::{NSApp, NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua};

pub fn detect_system_theme() -> CatppuccinFlavor {
    unsafe {
        let appearance = NSApp().effectiveAppearance();
        let name = appearance.bestMatchFromAppearancesWithNames(vec![
            NSAppearanceNameAqua,
            NSAppearanceNameDarkAqua,
        ]);
        
        if name == NSAppearanceNameDarkAqua {
            CatppuccinFlavor::Mocha
        } else {
            CatppuccinFlavor::Latte
        }
    }
}

pub fn observe_system_theme_changes(cx: &mut App) {
    // Listen for appearance changes
    cx.observe_system_appearance(|cx| {
        let flavor = detect_system_theme();
        let current_theme = cx.global::<PhotonTheme>();
        
        if current_theme.flavor != flavor {
            let new_theme = PhotonTheme::new(flavor, current_theme.accent);
            cx.set_global(new_theme);
        }
    });
}
```

---

## Configuration

```toml
# ~/.config/photoncast/config.toml

[theme]
# "latte", "frappe", "macchiato", "mocha", "system"
flavor = "system"

# Accent color override
# "rosewater", "flamingo", "pink", "mauve", "red", "maroon",
# "peach", "yellow", "green", "teal", "sky", "sapphire", "blue", "lavender"
accent = "mauve"

# Override individual colors (advanced)
# [theme.overrides]
# accent = "#ff79c6"
# background = "#1a1a2e"
```

---

## Accessibility

### Contrast Ratios

All text colors meet WCAG 2.1 AA standards:

| Combination | Ratio | WCAG |
|-------------|-------|------|
| Text on Base | ≥7:1 | AAA |
| Subtext1 on Base | ≥4.5:1 | AA |
| Subtext0 on Base | ≥4.5:1 | AA |
| Text on Surface0 | ≥7:1 | AAA |

### Focus States

All interactive elements have visible focus indicators:

```rust
fn focus_ring(theme: &PhotonTheme) -> impl Element {
    div()
        .when_focused(|el| {
            el.outline_2()
              .outline_offset_2()
              .outline_color(theme.colors.focus_ring)
        })
}
```

---

## Resources

- [Catppuccin GitHub](https://github.com/catppuccin/catppuccin)
- [Catppuccin Palette](https://catppuccin.com/palette)
- [WCAG Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [GPUI Theming](https://gpui.rs)
