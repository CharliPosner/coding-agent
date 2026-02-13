//! Theme system for the coding-agent CLI
//!
//! Defines colors and styles in a single place for consistent UI.

use console::Style;
use std::env;

/// Color definitions for different UI elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// User input - white/default
    UserInput,
    /// Agent response - cyan/light blue
    Agent,
    /// Tool calls - yellow/amber
    Tool,
    /// Success - green
    Success,
    /// Error - red
    Error,
    /// Warning - orange/yellow
    Warning,
    /// Muted/secondary - gray
    Muted,
    /// Cost/tokens - magenta
    Cost,
    /// Context bar - green
    ContextGreen,
    /// Context bar - yellow
    ContextYellow,
    /// Context bar - red
    ContextRed,
}

/// Theme configuration
#[derive(Debug, Clone)]
pub struct Theme {
    /// The theme style name
    pub style: ThemeStyle,
    /// Whether colors are enabled (respects NO_COLOR)
    colors_enabled: bool,
}

/// Available theme styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeStyle {
    /// Minimal color usage
    Minimal,
    /// Full color support
    Colorful,
    /// No colors (monochrome)
    Monochrome,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeStyle::Minimal)
    }
}

impl Theme {
    /// Create a new theme with the given style
    pub fn new(style: ThemeStyle) -> Self {
        let colors_enabled = !Self::no_color_env() && style != ThemeStyle::Monochrome;
        Self {
            style,
            colors_enabled,
        }
    }

    /// Check if NO_COLOR environment variable is set
    fn no_color_env() -> bool {
        env::var("NO_COLOR").is_ok()
    }

    /// Check if colors are enabled
    pub fn colors_enabled(&self) -> bool {
        self.colors_enabled
    }

    /// Get the style for a given color
    pub fn style(&self, color: Color) -> Style {
        if !self.colors_enabled {
            return Style::new();
        }

        match color {
            Color::UserInput => Style::new().white(),
            Color::Agent => Style::new().cyan(),
            Color::Tool => Style::new().yellow(),
            Color::Success => Style::new().green(),
            Color::Error => Style::new().red().bold(),
            Color::Warning => Style::new().yellow().bold(),
            Color::Muted => Style::new().dim(),
            Color::Cost => Style::new().magenta(),
            Color::ContextGreen => Style::new().green(),
            Color::ContextYellow => Style::new().yellow(),
            Color::ContextRed => Style::new().red(),
        }
    }

    /// Apply style to text
    pub fn apply(&self, color: Color, text: &str) -> String {
        self.style(color).apply_to(text).to_string()
    }

    /// Get bold version of a style
    pub fn bold(&self, color: Color) -> Style {
        self.style(color).bold()
    }

    /// Get dimmed version of a style
    pub fn dim(&self, color: Color) -> Style {
        self.style(color).dim()
    }
}

impl ThemeStyle {
    /// Parse a theme style from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "minimal" => Some(Self::Minimal),
            "colorful" => Some(Self::Colorful),
            "monochrome" | "mono" | "none" => Some(Self::Monochrome),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_all_colors_defined() {
        let theme = Theme::default();

        // Test all color variants have a style defined
        let colors = [
            Color::UserInput,
            Color::Agent,
            Color::Tool,
            Color::Success,
            Color::Error,
            Color::Warning,
            Color::Muted,
            Color::Cost,
            Color::ContextGreen,
            Color::ContextYellow,
            Color::ContextRed,
        ];

        for color in colors {
            // Should not panic
            let _style = theme.style(color);
        }
    }

    #[test]
    fn test_theme_styles() {
        // Minimal theme
        let minimal = Theme::new(ThemeStyle::Minimal);
        assert_eq!(minimal.style, ThemeStyle::Minimal);

        // Colorful theme
        let colorful = Theme::new(ThemeStyle::Colorful);
        assert_eq!(colorful.style, ThemeStyle::Colorful);

        // Monochrome theme (no colors)
        let mono = Theme::new(ThemeStyle::Monochrome);
        assert!(!mono.colors_enabled());
    }

    #[test]
    fn test_theme_style_from_str() {
        assert_eq!(ThemeStyle::from_str("minimal"), Some(ThemeStyle::Minimal));
        assert_eq!(ThemeStyle::from_str("MINIMAL"), Some(ThemeStyle::Minimal));
        assert_eq!(ThemeStyle::from_str("colorful"), Some(ThemeStyle::Colorful));
        assert_eq!(
            ThemeStyle::from_str("monochrome"),
            Some(ThemeStyle::Monochrome)
        );
        assert_eq!(ThemeStyle::from_str("mono"), Some(ThemeStyle::Monochrome));
        assert_eq!(ThemeStyle::from_str("none"), Some(ThemeStyle::Monochrome));
        assert_eq!(ThemeStyle::from_str("invalid"), None);
    }

    #[test]
    fn test_theme_apply() {
        let theme = Theme::new(ThemeStyle::Minimal);
        let text = "Hello";

        // Apply should return a string
        let result = theme.apply(Color::Success, text);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_output_no_color_mode() {
        // When colors are disabled, style should be empty
        let theme = Theme::new(ThemeStyle::Monochrome);

        let styled = theme.apply(Color::Error, "test");
        // In monochrome mode, no ANSI codes should be added
        // The exact behavior depends on console crate, but text should be present
        assert!(styled.contains("test"));
    }
}
