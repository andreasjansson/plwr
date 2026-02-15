use playwright_rs::PLAYWRIGHT_VERSION;

#[test]
fn test_playwright_version_exists() {
    assert!(!PLAYWRIGHT_VERSION.is_empty());
}

#[test]
fn test_playwright_version_starts_with_semver() {
    assert!(
        PLAYWRIGHT_VERSION.starts_with("1."),
        "Expected version starting with '1.', got: {}",
        PLAYWRIGHT_VERSION
    );
}

#[test]
fn test_playwright_version_is_const() {
    const VERSION_AT_COMPILE_TIME: &str = PLAYWRIGHT_VERSION;
    assert_eq!(VERSION_AT_COMPILE_TIME, PLAYWRIGHT_VERSION);
}
