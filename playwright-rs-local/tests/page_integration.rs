// Integration tests for Page
//
// These tests verify that we can create pages and manage them.

use playwright_rs::protocol::Playwright;

mod common;

#[tokio::test]
async fn test_context_new_page() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();
    let browser = chromium.launch().await.expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Create a new page
    let page = context.new_page().await.expect("Failed to create page");

    // Verify page was created
    tracing::info!("✓ Page created");

    // Page should initially be at about:blank
    assert_eq!(page.url(), "about:blank");

    // Cleanup
    page.close().await.expect("Failed to close page");
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_browser_new_page_convenience() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create page directly from browser (creates default context)
    let page = browser.new_page().await.expect("Failed to create page");

    tracing::info!("✓ Page created via browser.new_page()");

    // Should be at about:blank
    assert_eq!(page.url(), "about:blank");

    // Cleanup
    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_multiple_pages_in_context() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Create multiple pages
    let page1 = context.new_page().await.expect("Failed to create page 1");
    let page2 = context.new_page().await.expect("Failed to create page 2");

    tracing::info!("✓ Created 2 pages in same context");

    // Each should be at about:blank
    assert_eq!(page1.url(), "about:blank");
    assert_eq!(page2.url(), "about:blank");

    // Cleanup
    page1.close().await.expect("Failed to close page 1");
    page2.close().await.expect("Failed to close page 2");
    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_close() {
    common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");

    // Close page
    page.close().await.expect("Failed to close page");

    tracing::info!("✓ Page closed successfully");

    // Cleanup
    browser.close().await.expect("Failed to close browser");
}
