use playwright_rs::server::channel_owner::ChannelOwner;
use playwright_rs::{BrowserContext, Page};
use serde::Deserialize;
use std::collections::HashMap;

// -- BrowserContext extensions --

pub async fn set_extra_http_headers(
    ctx: &BrowserContext,
    headers: HashMap<String, String>,
) -> playwright_rs::Result<()> {
    let header_array: Vec<serde_json::Value> = headers
        .into_iter()
        .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
        .collect();
    ctx.channel()
        .send_no_result(
            "setExtraHTTPHeaders",
            serde_json::json!({ "headers": header_array }),
        )
        .await
}

#[derive(Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    #[serde(rename = "httpOnly")]
    pub http_only: bool,
    pub secure: bool,
    #[serde(rename = "sameSite")]
    pub same_site: Option<String>,
}

pub async fn get_cookies(ctx: &BrowserContext) -> playwright_rs::Result<Vec<Cookie>> {
    #[derive(Deserialize)]
    struct CookiesResponse {
        cookies: Vec<Cookie>,
    }
    let response: CookiesResponse = ctx
        .channel()
        .send("cookies", serde_json::json!({ "urls": [] }))
        .await?;
    Ok(response.cookies)
}

pub async fn clear_cookies(ctx: &BrowserContext) -> playwright_rs::Result<()> {
    ctx.channel()
        .send_no_result("clearCookies", serde_json::json!({}))
        .await
}

pub async fn add_cookie(
    ctx: &BrowserContext,
    name: String,
    value: String,
    url: String,
) -> playwright_rs::Result<()> {
    let cookie = serde_json::json!({
        "name": name,
        "value": value,
        "url": url,
    });
    ctx.channel()
        .send_no_result("addCookies", serde_json::json!({ "cookies": [cookie] }))
        .await
}

// -- Page video extensions --
// playwright-rs doesn't expose videoStart/videoStop, but we can send them
// directly via the page channel, matching what playwright-cli does internally.

#[derive(Deserialize)]
struct VideoStartResponse {
    artifact: ArtifactRef,
}

#[derive(Deserialize)]
struct ArtifactRef {
    guid: String,
}

pub async fn page_video_start(
    page: &Page,
    dir: &str,
) -> playwright_rs::Result<()> {
    let params = serde_json::json!({
        "dir": dir,
    });
    let _resp: VideoStartResponse = page.channel().send("videoStart", params).await?;
    Ok(())
}

pub async fn page_video_stop(page: &Page) -> playwright_rs::Result<()> {
    page.channel()
        .send_no_result("videoStop", serde_json::json!({}))
        .await
}

// -- Page extensions --
// page.evaluate_value exists but the stock signatures take &str where we need
// String-based wrappers. These are thin helpers.

pub async fn page_evaluate_value(page: &Page, js: &str) -> playwright_rs::Result<String> {
    page.evaluate_value(js).await
}

// -- Locator extensions --
// Locator::evaluate runs JS with the matched element as argument (evalOnSelector).
// Locator::evaluate_value runs JS in the page context via the locator's frame.
// Stock playwright-rs doesn't expose these, so we use page.evaluate with querySelector.

pub async fn locator_focus(page: &Page, selector: &str) -> playwright_rs::Result<()> {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        "() => {{ const el = document.querySelector('{}'); if (!el) throw new Error('No element found'); el.focus(); }}",
        escaped
    );
    page.evaluate_value(&js).await?;
    Ok(())
}

pub async fn locator_blur(page: &Page, selector: &str) -> playwright_rs::Result<()> {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        "() => {{ const el = document.querySelector('{}'); if (!el) throw new Error('No element found'); el.blur(); }}",
        escaped
    );
    page.evaluate_value(&js).await?;
    Ok(())
}

pub async fn locator_scroll_into_view(page: &Page, selector: &str) -> playwright_rs::Result<()> {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        "() => {{ const el = document.querySelector('{}'); if (!el) throw new Error('No element found'); el.scrollIntoView({{behavior: 'instant', block: 'center'}}); }}",
        escaped
    );
    page.evaluate_value(&js).await?;
    Ok(())
}

pub async fn locator_eval_on_selector(
    page: &Page,
    selector: &str,
    js: &str,
) -> playwright_rs::Result<String> {
    let escaped_selector = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let wrapper = format!(
        "() => {{ const el = document.querySelector('{}'); if (!el) throw new Error('No element found for selector: {}'); const fn_ = {}; return JSON.stringify(fn_(el)); }}",
        escaped_selector, escaped_selector, js
    );
    page.evaluate_value(&wrapper).await
}
