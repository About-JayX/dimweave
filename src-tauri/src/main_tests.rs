#[test]
fn main_opens_devtools_without_stale_feature_gate() {
    let source = include_str!("main.rs");
    assert!(source.contains("w.open_devtools();"));
    assert!(!source.contains("#[cfg(feature = \"devtools\")]"));
}
