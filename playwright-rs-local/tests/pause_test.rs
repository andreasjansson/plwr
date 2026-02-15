mod test_server; // Needed to compile testing infrastructure if shared modules require it

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

mod common;

#[tokio::test]
async fn test_pause_headless() {
    common::init_tracing();
    let _server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // In headless mode (default), pause() should have no effect and return immediately
    // This verifies that calling it doesn't crash the bindings
    page.pause().await.expect("Failed to pause");

    browser.close().await.expect("Failed to close browser");
}
