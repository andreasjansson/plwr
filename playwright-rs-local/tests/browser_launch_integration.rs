// Integration tests for BrowserType::launch()
//
// These tests verify that we can launch real browsers using the Playwright server.

use playwright_rs::api::LaunchOptions;
use playwright_rs::protocol::Playwright;

mod common;

#[tokio::test]
async fn test_launch_chromium() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_chromium: Starting");

    // Launch Playwright
    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    // Get chromium browser type
    let chromium = playwright.chromium();

    // Launch browser with default options
    tracing::debug!("[TEST] Launching Chromium browser...");
    let browser = chromium.launch().await.expect("Failed to launch Chromium");
    tracing::debug!("[TEST] Chromium browser launched successfully");

    // Verify browser was created
    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    tracing::info!("Launched Chromium version: {}", browser.version());

    // Cleanup
    tracing::debug!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    tracing::debug!("[TEST] Browser closed successfully");
    tracing::debug!("[TEST] test_launch_chromium: Complete");
}

#[tokio::test]
async fn test_launch_with_headless_option() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_with_headless_option: Starting");

    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    let chromium = playwright.chromium();

    // Launch with explicit headless option
    let options = LaunchOptions::default().headless(true);

    tracing::debug!("[TEST] Launching Chromium browser with headless option...");
    let browser = chromium
        .launch_with_options(options)
        .await
        .expect("Failed to launch Chromium with options");
    tracing::debug!("[TEST] Chromium browser launched successfully");

    assert_eq!(browser.name(), "chromium");
    assert!(!browser.version().is_empty());

    // Cleanup
    tracing::debug!("[TEST] Closing browser...");
    browser.close().await.expect("Failed to close browser");
    tracing::debug!("[TEST] Browser closed successfully");
    tracing::debug!("[TEST] test_launch_with_headless_option: Complete");
}

#[tokio::test]
async fn test_launch_all_three_browsers() {
    common::init_tracing();
    tracing::debug!("[TEST] test_launch_all_three_browsers: Starting");

    tracing::debug!("[TEST] Launching Playwright server...");
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    tracing::debug!("[TEST] Playwright server launched successfully");

    // Test Chromium
    tracing::debug!("[TEST] === Testing Chromium ===");
    let chromium = playwright.chromium();
    tracing::debug!("[TEST] Launching Chromium browser...");
    let chromium_browser = chromium.launch().await.expect("Failed to launch Chromium");
    assert_eq!(chromium_browser.name(), "chromium");
    tracing::info!("✓ Chromium: {}", chromium_browser.version());
    tracing::debug!("[TEST] Closing Chromium...");
    chromium_browser
        .close()
        .await
        .expect("Failed to close Chromium");
    tracing::debug!("[TEST] Chromium closed successfully");

    // Test Firefox
    tracing::debug!("[TEST] === Testing Firefox ===");
    let firefox = playwright.firefox();
    tracing::debug!("[TEST] Launching Firefox browser...");
    let firefox_browser = firefox.launch().await.expect("Failed to launch Firefox");
    assert_eq!(firefox_browser.name(), "firefox");
    tracing::info!("✓ Firefox: {}", firefox_browser.version());
    tracing::debug!("[TEST] Closing Firefox...");
    firefox_browser
        .close()
        .await
        .expect("Failed to close Firefox");
    tracing::debug!("[TEST] Firefox closed successfully");

    // Test WebKit
    tracing::debug!("[TEST] === Testing WebKit ===");
    let webkit = playwright.webkit();
    tracing::debug!("[TEST] Launching WebKit browser...");
    let webkit_browser = webkit.launch().await.expect("Failed to launch WebKit");
    assert_eq!(webkit_browser.name(), "webkit");
    tracing::info!("✓ WebKit: {}", webkit_browser.version());
    tracing::debug!("[TEST] Closing WebKit...");
    webkit_browser
        .close()
        .await
        .expect("Failed to close WebKit");
    tracing::debug!("[TEST] WebKit closed successfully");

    tracing::debug!("[TEST] test_launch_all_three_browsers: Complete");
}

#[tokio::test]
async fn test_browser_close() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch Chromium");

    // Verify browser is open
    assert_eq!(browser.name(), "chromium");

    // Close browser
    browser.close().await.expect("Failed to close browser");

    tracing::info!("✓ Browser closed successfully");
}

#[tokio::test]
async fn test_close_multiple_browsers() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch multiple browsers
    let chromium = playwright.chromium();
    let browser1 = chromium
        .launch()
        .await
        .expect("Failed to launch Chromium 1");
    let browser2 = chromium
        .launch()
        .await
        .expect("Failed to launch Chromium 2");

    tracing::info!("Launched 2 browsers");

    // Close both browsers
    browser1.close().await.expect("Failed to close browser 1");
    tracing::info!("✓ Browser 1 closed");

    browser2.close().await.expect("Failed to close browser 2");
    tracing::info!("✓ Browser 2 closed");
}

#[tokio::test]
async fn test_browser_is_connected() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let chromium = playwright.chromium();

    // Launch browser
    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Should be connected initially
    assert!(
        browser.is_connected(),
        "Browser should be connected after launch"
    );

    // Close browser
    browser.close().await.expect("Failed to close browser");

    // Should be disconnected after close
    // Note: close() waits for the server to process the close command,
    // which should trigger the "disconnected" event before returning or shortly after.

    // Check immediately first.
    if browser.is_connected() {
        // Give it a moment for the event to arrive
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    assert!(
        !browser.is_connected(),
        "Browser should be disconnected after close"
    );
}
