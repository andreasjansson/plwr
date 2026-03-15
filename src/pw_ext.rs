use playwright_rs::server::channel_owner::ChannelOwner;
use playwright_rs::{Browser, BrowserContext, BrowserType, Page};
use serde::Deserialize;
use std::collections::HashMap;

// -- BrowserType extensions --

pub struct ConnectOverCDPResult {
    pub browser: Browser,
    pub default_context: Option<BrowserContext>,
}

pub async fn connect_over_cdp(
    browser_type: &BrowserType,
    ws_endpoint: &str,
    timeout: f64,
) -> playwright_rs::Result<ConnectOverCDPResult> {
    #[derive(Deserialize)]
    struct GuidRef {
        guid: String,
    }
    #[derive(Deserialize)]
    struct Response {
        browser: GuidRef,
        #[serde(rename = "defaultContext")]
        default_context: Option<GuidRef>,
    }

    let params = serde_json::json!({
        "endpointURL": ws_endpoint,
        "timeout": timeout,
    });

    let response: Response = browser_type
        .channel()
        .send("connectOverCDP", params)
        .await?;

    let conn = browser_type.connection();

    let browser_arc = conn.get_object(&response.browser.guid).await?;
    let browser = browser_arc
        .as_any()
        .downcast_ref::<Browser>()
        .ok_or_else(|| playwright_rs::Error::ProtocolError("Expected Browser object".to_string()))?
        .clone();

    let default_context = if let Some(ctx_ref) = response.default_context {
        let ctx_arc = conn.get_object(&ctx_ref.guid).await?;
        let ctx = ctx_arc
            .as_any()
            .downcast_ref::<BrowserContext>()
            .ok_or_else(|| {
                playwright_rs::Error::ProtocolError("Expected BrowserContext object".to_string())
            })?
            .clone();
        Some(ctx)
    } else {
        None
    };

    Ok(ConnectOverCDPResult {
        browser,
        default_context,
    })
}

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

// -- Page extensions --

pub async fn disable_network_interception(page: &Page) -> playwright_rs::Result<()> {
    page.channel()
        .send_no_result(
            "setNetworkInterceptionPatterns",
            serde_json::json!({ "patterns": [] }),
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

pub async fn grant_permissions(
    ctx: &BrowserContext,
    permissions: &[&str],
) -> playwright_rs::Result<()> {
    let perms: Vec<serde_json::Value> = permissions
        .iter()
        .map(|p| serde_json::Value::String(p.to_string()))
        .collect();
    ctx.channel()
        .send_no_result(
            "grantPermissions",
            serde_json::json!({ "permissions": perms }),
        )
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
