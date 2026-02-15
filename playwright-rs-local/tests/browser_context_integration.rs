// Integration tests for BrowserContext
//
// These tests verify that we can create browser contexts and manage them.

use playwright_rs::protocol::Playwright;

mod common;

#[tokio::test]
async fn test_new_context() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");

    // Create a new context
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Verify context was created
    tracing::info!("✓ Context created");

    // Cleanup
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_multiple_contexts() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create multiple contexts
    let context1 = browser
        .new_context()
        .await
        .expect("Failed to create context 1");
    let context2 = browser
        .new_context()
        .await
        .expect("Failed to create context 2");

    tracing::info!("✓ Created 2 contexts");

    // Cleanup
    context1.close().await.expect("Failed to close context 1");
    context2.close().await.expect("Failed to close context 2");
    browser.close().await.expect("Failed to close browser");
}
