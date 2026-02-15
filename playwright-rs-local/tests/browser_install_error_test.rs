use playwright_rs::Error;

#[test]
fn test_browser_not_installed_error_message() {
    let error = Error::BrowserNotInstalled {
        browser_name: "chromium".to_string(),
        message: "Looks like Playwright Test or Playwright was just installed or updated."
            .to_string(),
    };

    let msg = error.to_string();
    assert!(msg.contains("chromium"));
    assert!(msg.contains("npx playwright install chromium"));
}

#[test]
fn test_browser_not_installed_different_browsers() {
    for browser in ["chromium", "firefox", "webkit"] {
        let error = Error::BrowserNotInstalled {
            browser_name: browser.to_string(),
            message: format!("Browser '{}' is not installed", browser),
        };

        let msg = error.to_string();
        assert!(msg.contains(browser));
        assert!(msg.contains(&format!("npx playwright install {}", browser)));
    }
}
