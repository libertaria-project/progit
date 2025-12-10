# ProGit Style System Enhancements 🎨

This document describes the advanced styling features added to ProGit's TUI system.

## Overview

The enhanced style system provides CSS-like capabilities for customizing ProGit's terminal interface. It includes:

1. **Style Inheritance** - Create style hierarchies with reusable base styles
2. **Conditional Styling** - Apply different styles based on context/state
3. **Style Validation** - Catch configuration errors early
4. **Comprehensive Color Support** - Named colors, hex colors, and RGB

## Features Implemented

### 1. Style Inheritance System

**Syntax:**
```kdl
styles {
    base {
        fg "white"
        bg "black"
        bold true
    }
    
    header inherits base {
        fg "blue"
        underlined true
    }
    
    important inherits header {
        fg "red"
        bold true  // Overrides the inherited bold from base
    }
}
```

**How it works:**
- Styles can inherit from other styles using the `inherits` keyword
- Child styles override parent properties while inheriting unspecified ones
- Multi-level inheritance is supported (grandparent → parent → child)
- Circular inheritance is detected and prevented

**Implementation details:**
- Added `inherits: Option<String>` field to `StyleConfig`
- Enhanced `ThemeEngine::new()` to resolve inheritance before parsing
- Added `resolve_inheritance()` function for recursive inheritance resolution
- Added `merge_modifiers()` to handle modifier inheritance properly

### 2. Conditional Styling

**Syntax:**
```kdl
styles {
    issue {
        fg "white"
        bg "black"
    }
    
    issue.urgent {
        fg "red"
        bold true
    }
    
    issue.completed {
        fg "green"
        dim true
    }
}
```

**Usage in code:**
```rust
// Get conditional style
let style = engine.get_conditional("issue", "urgent", colors.normal());
```

**How it works:**
- Conditional styles use dot notation: `base.condition`
- The engine first looks for the specific conditional style
- Falls back to the base style if no condition match is found
- Perfect for styling based on issue status, urgency, etc.

### 3. Style Validation

**Validation checks:**
- **Circular inheritance detection** - Prevents infinite loops
- **Color validation** - Ensures all color names are valid
- **Modifier validation** - Checks for valid modifier names

**Usage:**
```rust
if let Err(e) = engine.validate() {
    eprintln!("Style configuration error: {}", e);
}
```

**Supported modifiers:**
- `bold`, `dim`, `italic`, `underlined`, `reversed`, `hidden`, `crossed_out`

**Supported colors:**
- Named colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `darkgray`, `lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`, `white`
- Hex colors: `#RRGGBB` format

### 4. Enhanced Theme Engine

**New methods:**
- `get_conditional(base_name: &str, condition: &str, fallback: Style) -> Style`
- `get_config(name: &str) -> Option<&StyleConfig>`
- `validate() -> Result<(), String>`

**Backward compatibility:**
- All existing `get()` calls continue to work unchanged
- Existing themes remain as robust fallbacks
- No breaking changes to the API

## Example Configuration

See `example_advanced_styles.kdl` for a comprehensive example showing:
- Base style definitions
- Inheritance hierarchies
- Conditional variants
- Component-specific styling

## Usage Patterns

### Inheritance for Component Families
```kdl
styles {
    button {
        fg "black"
        bg "white"
        bold true
    }
    
    button.primary inherits button {
        bg "blue"
        fg "white"
    }
    
    button.danger inherits button {
        bg "red"
        fg "white"
    }
}
```

### Conditional Issue Styling
```kdl
styles {
    issue {
        fg "white"
        bg "black"
    }
    
    issue.urgent {
        fg "red"
        bold true
    }
    
    issue.blocked {
        fg "yellow"
        bg "darkred"
    }
}
```

### Status Indicators
```kdl
styles {
    status {
        fg "white"
        bg "black"
    }
    
    status.todo inherits status {
        fg "yellow"
    }
    
    status.in_progress inherits status {
        fg "blue"
    }
    
    status.done inherits status {
        fg "green"
    }
}
```

## Testing

A basic test suite is provided in `tests/style_inheritance.rs` that verifies:
- Style inheritance works correctly
- Properties are properly overridden
- Multi-level inheritance functions
- No circular inheritance occurs

## Future Enhancements

Potential areas for future improvement:

1. **Style Variables** - Define reusable color variables
2. **Dynamic Themes** - Time-based or context-based theme switching
3. **Animation Support** - Pulsing, fading, or other animations
4. **Style Preview** - CLI command to preview styles
5. **Export/Import** - Share style presets between users

## Migration Guide

Existing configurations continue to work without changes. To take advantage of new features:

1. **Add inheritance:** Simply add `inherits "parent_style"` to any style
2. **Add conditions:** Create conditional variants using dot notation
3. **Validate:** Call `engine.validate()` to catch configuration errors

## Performance Considerations

- Inheritance resolution happens once at startup
- Style lookups remain O(1) hash map operations
- No runtime performance impact on rendering
- Memory overhead is minimal (storing resolved configurations)

## Summary

These enhancements transform ProGit's styling system from a basic configuration system to a powerful, CSS-like styling framework that enables:

✅ **Reusable style components** through inheritance
✅ **Context-aware styling** through conditions  
✅ **Early error detection** through validation
✅ **Backward compatibility** with existing configurations
✅ **Modern TUI theming** comparable to web frameworks

The system is now on par with modern terminal frameworks while maintaining ProGit's performance and simplicity.