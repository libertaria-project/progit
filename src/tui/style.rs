//! Style System
//!
//! dynamic styling engine for ProGit TUI.
//! Handles configurable styles with hardcoded theme fallback.

use crate::storage::config::StyleConfig;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
// ThemeColors unused

/// Manages application styles with configuration overrides
pub struct ThemeEngine {
    /// Custom styles from configuration
    styles: HashMap<String, Style>,
    /// Raw config for inheritance resolution
    config_styles: HashMap<String, StyleConfig>,
}

impl ThemeEngine {
    pub fn new(config_styles: &HashMap<String, StyleConfig>) -> Self {
        let mut styles = HashMap::new();
        let mut resolved_configs = HashMap::new();

        // First pass: resolve inheritance
        for (name, config) in config_styles {
            let mut visited = Vec::new();
            let resolved_config = resolve_inheritance(name, config, config_styles, &mut visited);
            resolved_configs.insert(name.clone(), resolved_config);
        }

        // Second pass: parse resolved configs to styles
        for (name, config) in &resolved_configs {
            let style = parse_style_config(config);
            styles.insert(name.clone(), style);
        }

        Self {
            styles,
            config_styles: resolved_configs,
        }
    }

    /// Get a style by name, falling back to a default style if not configured
    pub fn get(&self, name: &str, fallback: Style) -> Style {
        self.styles.get(name).cloned().unwrap_or(fallback)
    }

    /// Get the raw style configuration (for conditional styling)
    pub fn get_config(&self, name: &str) -> Option<&StyleConfig> {
        self.config_styles.get(name)
    }

    /// Get a style with conditional overrides
    pub fn get_conditional(&self, base_name: &str, condition: &str, fallback: Style) -> Style {
        // Try to get the conditional style first (e.g., "issue.urgent")
        let conditional_name = format!("{}.{}", base_name, condition);
        if let Some(style) = self.styles.get(&conditional_name) {
            return style.clone();
        }

        // Fall back to the base style
        self.get(base_name, fallback)
    }

    /// Validate style configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check for circular inheritance
        for (name, config) in &self.config_styles {
            if let Some(parent) = &config.inherits {
                if self.has_circular_inheritance(name, parent) {
                    return Err(format!(
                        "Circular inheritance detected: {} -> {}",
                        name, parent
                    ));
                }
            }
        }

        // Check for invalid color names
        for (name, config) in &self.config_styles {
            if let Some(fg) = &config.fg {
                if parse_color(fg).is_none() {
                    return Err(format!(
                        "Invalid foreground color '{}' in style '{}'",
                        fg, name
                    ));
                }
            }
            if let Some(bg) = &config.bg {
                if parse_color(bg).is_none() {
                    return Err(format!(
                        "Invalid background color '{}' in style '{}'",
                        bg, name
                    ));
                }
            }
        }

        // Check for invalid modifiers
        for (name, config) in &self.config_styles {
            for modifier in &config.modifiers {
                if !self.is_valid_modifier(modifier) {
                    return Err(format!(
                        "Invalid modifier '{}' in style '{}'",
                        modifier, name
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check for circular inheritance recursively
    fn has_circular_inheritance(&self, original: &str, current: &str) -> bool {
        if original == current {
            return true;
        }

        if let Some(config) = self.config_styles.get(current) {
            if let Some(parent) = &config.inherits {
                return self.has_circular_inheritance(original, parent);
            }
        }

        false
    }

    /// Check if a modifier is valid
    fn is_valid_modifier(&self, modifier: &str) -> bool {
        matches!(
            modifier,
            "bold" | "dim" | "italic" | "underlined" | "reversed" | "hidden" | "crossed_out"
        )
    }
}

/// Resolve style inheritance recursively
fn resolve_inheritance(
    current_name: &str,
    config: &StyleConfig,
    all_styles: &HashMap<String, StyleConfig>,
    visited: &mut Vec<String>,
) -> StyleConfig {
    if visited.contains(&current_name.to_string()) {
        // Cycle detected, stop recursion
        // We return the config as is (without resolving further parent) to break cycle
        return StyleConfig {
            fg: config.fg.clone(),
            bg: config.bg.clone(),
            modifiers: config.modifiers.clone(),
            inherits: None,
        };
    }

    visited.push(current_name.to_string());

    let result = if let Some(parent_name) = &config.inherits {
        if let Some(parent_config) = all_styles.get(parent_name) {
            // Recursively resolve parent first
            let resolved_parent =
                resolve_inheritance(parent_name, parent_config, all_styles, visited);

            // Merge: parent properties first, then override with current config
            StyleConfig {
                fg: config.fg.clone().or(resolved_parent.fg.clone()),
                bg: config.bg.clone().or(resolved_parent.bg.clone()),
                modifiers: merge_modifiers(&resolved_parent.modifiers, &config.modifiers),
                inherits: None, // Inheritance is resolved
            }
        } else {
            // Parent not found, just return current config
            StyleConfig {
                fg: config.fg.clone(),
                bg: config.bg.clone(),
                modifiers: config.modifiers.clone(),
                inherits: None,
            }
        }
    } else {
        // No inheritance, return as-is
        StyleConfig {
            fg: config.fg.clone(),
            bg: config.bg.clone(),
            modifiers: config.modifiers.clone(),
            inherits: None,
        }
    };

    visited.pop();
    result
}

/// Merge modifiers: parent modifiers + child modifiers (no duplicates)
fn merge_modifiers(parent_mods: &[String], child_mods: &[String]) -> Vec<String> {
    let mut result = parent_mods.to_vec();
    for child_mod in child_mods {
        if !result.contains(child_mod) {
            result.push(child_mod.clone());
        }
    }
    result
}

/// Parse StyleConfig into Ratatui Style
fn parse_style_config(config: &StyleConfig) -> Style {
    let mut style = Style::default();

    if let Some(fg) = &config.fg {
        if let Some(color) = parse_color(fg) {
            style = style.fg(color);
        }
    }

    if let Some(bg) = &config.bg {
        if let Some(color) = parse_color(bg) {
            style = style.bg(color);
        }
    }

    for modifier in &config.modifiers {
        match modifier.as_str() {
            "bold" => style = style.add_modifier(Modifier::BOLD),
            "dim" => style = style.add_modifier(Modifier::DIM),
            "italic" => style = style.add_modifier(Modifier::ITALIC),
            "underlined" => style = style.add_modifier(Modifier::UNDERLINED),
            "reversed" => style = style.add_modifier(Modifier::REVERSED),
            "hidden" => style = style.add_modifier(Modifier::HIDDEN),
            "crossed_out" => style = style.add_modifier(Modifier::CROSSED_OUT),
            _ => {}
        }
    }

    style
}

/// Parse color string (name, hex, or rgb)
fn parse_color(s: &str) -> Option<Color> {
    let s = s.to_lowercase();

    // Named colors
    match s.as_str() {
        "black" => return Some(Color::Black),
        "red" => return Some(Color::Red),
        "green" => return Some(Color::Green),
        "yellow" => return Some(Color::Yellow),
        "blue" => return Some(Color::Blue),
        "magenta" => return Some(Color::Magenta),
        "cyan" => return Some(Color::Cyan),
        "gray" | "grey" => return Some(Color::Gray),
        "darkgray" | "darkgrey" => return Some(Color::DarkGray),
        "lightred" => return Some(Color::LightRed),
        "lightgreen" => return Some(Color::LightGreen),
        "lightyellow" => return Some(Color::LightYellow),
        "lightblue" => return Some(Color::LightBlue),
        "lightmagenta" => return Some(Color::LightMagenta),
        "lightcyan" => return Some(Color::LightCyan),
        "white" => return Some(Color::White),
        _ => {}
    }

    // Hex colors (#RRGGBB)
    if s.starts_with('#') && s.len() == 7 {
        if let Ok(r) = u8::from_str_radix(&s[1..3], 16) {
            if let Ok(g) = u8::from_str_radix(&s[3..5], 16) {
                if let Ok(b) = u8::from_str_radix(&s[5..7], 16) {
                    return Some(Color::Rgb(r, g, b));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inheritance_resolution() {
        let mut configs = HashMap::new();

        // Base style
        configs.insert(
            "base".to_string(),
            StyleConfig {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec!["bold".to_string()],
                inherits: None,
            },
        );

        // Child style inheriting from base
        configs.insert(
            "child".to_string(),
            StyleConfig {
                fg: None, // Should inherit red
                bg: Some("blue".to_string()),
                modifiers: vec!["italic".to_string()], // Should have both bold and italic
                inherits: Some("base".to_string()),
            },
        );

        let engine = ThemeEngine::new(&configs);
        let style = engine.get("child", Style::default());

        assert_eq!(style.fg, Some(Color::Red));
        assert_eq!(style.bg, Some(Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_cycle_detection() {
        let mut configs = HashMap::new();

        // Cycle: A -> B -> A
        configs.insert(
            "A".to_string(),
            StyleConfig {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec![],
                inherits: Some("B".to_string()),
            },
        );

        configs.insert(
            "B".to_string(),
            StyleConfig {
                fg: Some("blue".to_string()),
                bg: None,
                modifiers: vec![],
                inherits: Some("A".to_string()),
            },
        );

        // This should not panic or overflow
        let engine = ThemeEngine::new(&configs);

        // Cycles should be broken safely.
        // Logic: A inherits B. B inherits A.
        // Resolve A: visit A. inherits B.
        // Resolve B: visit B. inherits A.
        // Resolve A (again): visited contains A -> return A config (red).
        // B result: merges red (from parent A) + blue (self) -> blue overrides if set?
        // Actually resolve_inheritance logic:
        // StyleConfig { fg: config.fg.or(parent.fg) ... }
        // If config has fg, it takes precedence.

        let style_a = engine.get("A", Style::default());
        let style_b = engine.get("B", Style::default());

        // A has red, B has blue. Both defined specific FG.
        assert_eq!(style_a.fg, Some(Color::Red));
        assert_eq!(style_b.fg, Some(Color::Blue));
    }

    #[test]
    fn test_validate() {
        let mut configs = HashMap::new();

        // Invalid color
        configs.insert(
            "bad_color".to_string(),
            StyleConfig {
                fg: Some("not_a_color".to_string()),
                bg: None,
                modifiers: vec![],
                inherits: None,
            },
        );

        // Valid style
        configs.insert(
            "good".to_string(),
            StyleConfig {
                fg: Some("red".to_string()),
                bg: None,
                modifiers: vec![],
                inherits: None,
            },
        );

        let engine = ThemeEngine::new(&configs);
        let result = engine.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid foreground color"));
    }
}
