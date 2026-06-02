// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! File path → canonical language id.
//!
//! Used by the diff renderer to tell the highlight plugin what language
//! the content is in. Returns lowercase ids matching what the
//! `syntax-highlight` Lua plugin's pattern tables expect.
//!
//! Plugins that want to do their own detection can ignore the hint.

/// Map a path's extension to a canonical language id.
///
/// Returns `None` when the extension is unknown — the plugin will see
/// `language = nil` and may either guess from content or decline.
pub fn from_path(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "rust",
        "py" | "pyi" | "pyw" => "python",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "sh" | "bash" | "zsh" => "bash",
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        "md" | "markdown" => "markdown",
        "lua" => "lua",
        "toml" => "toml",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_extensions() {
        assert_eq!(from_path("src/main.rs"), Some("rust"));
        assert_eq!(from_path("foo.py"), Some("python"));
        assert_eq!(from_path("a/b/c.tsx"), Some("typescript"));
        assert_eq!(from_path("config.YAML"), Some("yaml")); // case insensitive
        assert_eq!(from_path("notes.md"), Some("markdown"));
    }

    #[test]
    fn unknown_or_missing_extension_returns_none() {
        assert_eq!(from_path("Makefile"), None);
        assert_eq!(from_path("README"), None);
        assert_eq!(from_path(".env"), None); // "env" not in the table
        assert_eq!(from_path("/etc/hosts"), None);
    }
}
