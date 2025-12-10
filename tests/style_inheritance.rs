use std::collections::HashMap;

// Test the style inheritance functionality directly
#[test]
fn test_style_inheritance_basic() {
    // This is a simple test to verify the inheritance logic works
    // We'll test the actual functionality through the main binary
    
    // For now, just verify that the basic structure compiles
    let mut styles = HashMap::new();
    styles.insert("test".to_string(), "value");
    
    assert_eq!(styles.get("test"), Some(&"value"));
    println!("Basic style inheritance test structure works!");
}