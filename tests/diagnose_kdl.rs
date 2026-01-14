use anyhow::Result;
use progit::issue::Issue;
use progit::storage::{kdl::parse_kdl, kdl::serialize_kdl};

#[test]
fn test_complex_kdl_parsing() -> Result<()> {
    // Case 1: Property style (our serializer)
    let kdl_prop = r#"issue id="prop-id" {
    title "Property Style"
}"#;
    let (issue1, gen1) = parse_kdl(kdl_prop)?;
    assert_eq!(issue1.id, "prop-id");
    assert!(!gen1);

    // Case 2: Child style (existing files)
    let kdl_child = r#"issue {
    id "child-id"
    title "Child Style"
}"#;
    let (issue2, gen2) = parse_kdl(kdl_child)?;
    assert_eq!(issue2.id, "child-id");
    assert!(!gen2);

    // Case 3: Missing ID
    let kdl_missing = r#"issue {
    title "Ghost"
}"#;
    let (issue3, gen3) = parse_kdl(kdl_missing)?;
    assert!(!issue3.id.is_empty());
    assert!(gen3);

    println!("All cases passed!");
    Ok(())
}
