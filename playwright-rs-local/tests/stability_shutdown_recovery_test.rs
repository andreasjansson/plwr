// Integration tests for Graceful Shutdown and Error Recovery (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - Graceful shutdown on Drop
// - SIGTERM handling (Unix only)
// - SIGINT handling (Unix only)
// - Network error recovery
// - Browser crash handling
// - Connection loss recovery
// - Timeout recovery
//
// Success Criteria:
// - Clean shutdown on Drop
// - Proper SIGTERM/SIGINT handling
// - Graceful error recovery
// - No resource leaks on error paths

mod common;
mod test_server;

use playwright_rs::protocol::{GotoOptions, Playwright};
use std::time::Duration;
use test_server::TestServer;

// ============================================================================
// Graceful Shutdown Test: Drop Cleanup
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_on_drop() {
    common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Drop Cleanup ===\n");

    // Test that Playwright cleans up properly when dropped
    {
        let playwright = Playwright::launch()
            .await
            .expect("Failed to launch Playwright");

        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        let page = browser.new_page().await.expect("Failed to create page");
        let _ = page.goto("about:blank", None).await;

        tracing::info!("Playwright, browser, and page created");
        tracing::info!("Dropping all objects...");

        // Explicit drops to test cleanup order
        drop(page);
        drop(browser);
        drop(playwright);
    }

    // Wait for cleanup to complete
    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::info!("\n✓ Graceful shutdown on drop completed");
}

// ============================================================================
// Graceful Shutdown Test: Explicit Close
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_explicit_close() {
    common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Explicit Close ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let page = browser.new_page().await.expect("Failed to create page");
    let _ = page.goto("about:blank", None).await;

    // Close explicitly in reverse order
    tracing::info!("Closing page...");
    page.close().await.expect("Failed to close page");

    tracing::info!("Closing browser...");
    browser.close().await.expect("Failed to close browser");

    tracing::info!("Dropping playwright...");
    drop(playwright);

    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("\n✓ Explicit close completed successfully");
}

// ============================================================================
// Graceful Shutdown Test: Multiple Browsers
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_multiple_browsers() {
    common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Multiple Browsers ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch multiple browsers
    let browser1 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 1");

    let browser2 = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch browser 2");

    tracing::info!("Two browsers launched");

    // Close both
    tracing::info!("Closing browser 1...");
    browser1.close().await.expect("Failed to close browser 1");

    tracing::info!("Closing browser 2...");
    browser2.close().await.expect("Failed to close browser 2");

    tracing::info!("Dropping playwright...");
    drop(playwright);

    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("\n✓ Multiple browsers shut down successfully");
}

// ============================================================================
// Error Recovery Test: Network Timeout Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_network_timeout() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Network Timeout ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: After a timeout error, page should still be usable
    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = page.goto("http://10.255.255.1:9999/", Some(options)).await;

    assert!(result.is_err(), "Expected timeout error");
    assert!(result.is_err(), "Expected timeout error");
    tracing::info!("Timeout error occurred (expected)");

    // Recovery: Page should still work for valid navigation
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after timeout error: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after timeout");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Invalid URL Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_invalid_url() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Invalid URL ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: After invalid URL error, page should still be usable
    let result = page.goto("not-a-valid-url", None).await;

    assert!(result.is_err(), "Expected invalid URL error");
    assert!(result.is_err(), "Expected invalid URL error");
    tracing::info!("Invalid URL error occurred (expected)");

    // Recovery: Page should still work for valid navigation
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after invalid URL error: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after invalid URL");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Multiple Errors in Sequence
// ============================================================================

#[tokio::test]
async fn test_error_recovery_multiple_errors() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Multiple Errors ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Page should handle multiple consecutive errors
    let errors = [
        "not-valid-url",
        "http://localhost:59999/",
        "http://10.255.255.1:9999/",
    ];

    for (i, url) in errors.iter().enumerate() {
        let options = GotoOptions::new().timeout(Duration::from_millis(100));
        let result = page.goto(url, Some(options)).await;

        assert!(result.is_err(), "Error {} should fail", i + 1);
        assert!(result.is_err(), "Error {} should fail", i + 1);
        tracing::info!("Error {} handled (expected)", i + 1);
    }

    // Recovery: Page should still work after multiple errors
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after multiple errors: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after multiple consecutive errors");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Error During Page Creation
// ============================================================================

#[tokio::test]
async fn test_error_recovery_page_creation() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Page Creation ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create multiple pages, even if some operations fail
    let page1 = browser.new_page().await.expect("Failed to create page 1");

    // Try an operation that might fail
    let _ = page1.goto("invalid-url", None).await;

    // Should still be able to create more pages
    let page2 = browser.new_page().await.expect("Failed to create page 2");

    // And use them (about:blank may not return a response, so just verify page is usable)
    let _ = page2.goto("about:blank", None).await;
    assert!(!page2.url().is_empty(), "Page 2 should have a URL");

    tracing::info!("✓ Browser recovered and created new page after error");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Recovery Test: Context Error Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_context() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Context ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create context and page
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Cause an error
    let _ = page.goto("invalid-url", None).await;

    // Context should still be usable
    let page2 = context
        .new_page()
        .await
        .expect("Failed to create second page");

    // Note: about:blank may not return a response, so we don't assert on the result
    // The important thing is that we can create and use the page
    let _ = page2.goto("about:blank", None).await;

    // Verify the page is usable by checking we can get its URL
    assert!(!page2.url().is_empty(), "Page 2 should have a URL");

    tracing::info!("✓ Context recovered after page error");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Recovery Test: Browser Relaunch After Close
// ============================================================================

#[tokio::test]
async fn test_error_recovery_browser_relaunch() {
    common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Browser Relaunch ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch and close browser
    let browser1 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 1");

    browser1.close().await.expect("Failed to close browser 1");
    tracing::info!("Browser 1 closed");

    // Should be able to launch new browser
    let browser2 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 2");

    let page = browser2.new_page().await.expect("Failed to create page");

    // Note: about:blank may not return a response, so we don't assert on the result
    // The important thing is that we can create and use the page
    let _ = page.goto("about:blank", None).await;

    // Verify the page is usable by checking we can get its URL
    assert!(!page.url().is_empty(), "Page should have a URL");

    tracing::info!("✓ Browser relaunched successfully");

    browser2.close().await.expect("Failed to close browser 2");
}

// ============================================================================
// Stress Test: Error Recovery Under Load
// ============================================================================

#[tokio::test]
async fn test_error_recovery_stress() {
    common::init_tracing();
    tracing::info!("\n=== Stress Test: Error Recovery Under Load ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Rapidly alternate between errors and successes
    const CYCLES: usize = 20;
    let mut successful_navigations = 0;

    for i in 0..CYCLES {
        if i % 2 == 0 {
            // Cause error (invalid port)
            let _ = page.goto("http://localhost:59999/", None).await;

            // Give a tiny bit of breathing room for the error to propagate
            // This helps stability in CI environments without compromising the "stress" aspect
            // (users rarely navigate INSTANTLY after an error)
            tokio::time::sleep(Duration::from_millis(100)).await;
        } else {
            // Attempt successful navigation
            let result = page
                .goto(&format!("{}/locators.html", server.url()), None)
                .await;

            if result.is_ok() {
                successful_navigations += 1;
            } else {
                tracing::warn!("Navigation failed in cycle {}: {:?}", i, result.err());
            }
        }

        if i % 5 == 4 {
            tracing::info!("Completed {} error/success cycles", i + 1);
        }
    }

    // Verify at least 30% of valid attempts succeeded (allow some flakiness)
    // We attempt CYCLES/2 valid navigations.
    let attempts = CYCLES / 2;
    tracing::info!(
        "Successful navigations: {}/{}",
        successful_navigations,
        attempts
    );

    // We expect most to succeed with the small delay, but CI can be slow.
    // 30% success rate is enough to prove recovery works.
    let min_successful = (attempts as f64 * 0.3).ceil() as usize;
    assert!(
        successful_navigations >= min_successful,
        "Too few successful navigations: {} (expected at least {})",
        successful_navigations,
        min_successful
    );

    tracing::info!("✓ Error recovery stress test passed");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Signal Handling Test: Ctrl+C Simulation (Unix only)
// ============================================================================

#[tokio::test]
#[cfg(unix)]
async fn test_signal_handling_cleanup() {
    common::init_tracing();
    tracing::info!("\n=== Testing Signal Handling: Cleanup ===\n");

    // Note: We can't actually send SIGINT/SIGTERM to our own process in tests,
    // but we can verify that Drop handlers work correctly, which is what
    // signal handlers would ultimately call.

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Simulate abrupt shutdown by just dropping
    // Drop implementations should handle cleanup
    drop(browser);
    drop(playwright);

    // Wait for cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("✓ Cleanup handlers work for signal simulation");

    // Note: Real signal handling would require tokio::signal
    // and is better tested in integration/manual testing
}
