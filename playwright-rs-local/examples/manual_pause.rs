// Manual verification script for page.pause()
//
// Usage: cargo run --example manual_pause
//
// Expected behavior:
// 1. Browser opens (headless: false)
// 2. Navigates to example.com
// 3. Playwright Inspector opens
// 4. Execution pauses until you click "Resume" in the Inspector

use playwright_rs::{LaunchOptions, Playwright};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("Starting Playwright...");
    let playwright = Playwright::launch().await?;

    // Launch browser with headless: false so we can see the window
    println!("Launching browser (headless: false)...");
    let options = LaunchOptions::default().headless(false);

    let browser = playwright.chromium().launch_with_options(options).await?;
    let context = browser.new_context().await?;
    let page = context.new_page().await?;

    println!("Navigating to example.com...");
    page.goto("https://example.com", None).await?;

    println!("Pausing execution. Look for the Playwright Inspector window!");
    println!("Press 'Resume' in the Inspector to close the browser and exit.");

    // This should block until resumed in the Inspector
    page.pause().await?;

    println!("Resumed! Closing browser...");
    browser.close().await?;

    Ok(())
}
