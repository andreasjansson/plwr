use crate::protocol::{Command, Request, Response};
use crate::pw_ext;
use anyhow::Result;
use playwright_rs::{
    BrowserContextOptions, ClickOptions, FillOptions, LaunchOptions, Locator, Page, Playwright,
    RecordVideo,
};
use std::collections::HashMap;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

const READY_SIGNAL: &str = "### ready";
const ERROR_PREFIX: &str = "### error ";
const CHANNEL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

struct State {
    playwright: Playwright,
    _browser: playwright_rs::Browser,
    page: Page,
    page_opened: bool,
    headers: HashMap<String, String>,
    video_page: Option<Page>,
    video_dir: Option<String>,
}

impl State {
    fn active_page(&self) -> &Page {
        self.video_page.as_ref().unwrap_or(&self.page)
    }
}

pub async fn run(socket_path: &Path, headed: bool) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            println!("{}{}", ERROR_PREFIX, e);
            return Err(e.into());
        }
    };
    let browser = match playwright.chromium().launch_with_options(LaunchOptions {
        headless: Some(!headed),
        ..Default::default()
    }).await {
        Ok(b) => b,
        Err(e) => {
            println!("{}{}", ERROR_PREFIX, e);
            return Err(e.into());
        }
    };
    let page = match browser.new_page().await {
        Ok(p) => p,
        Err(e) => {
            println!("{}{}", ERROR_PREFIX, e);
            return Err(e.into());
        }
    };
    let listener = match UnixListener::bind(socket_path) {
        Ok(l) => l,
        Err(e) => {
            println!("{}{}", ERROR_PREFIX, e);
            return Err(e.into());
        }
    };

    println!("{}", READY_SIGNAL);

    let mut state = State {
        playwright,
        _browser: browser,
        page,
        page_opened: false,
        headers: HashMap::new(),
        video_page: None,
        video_dir: None,
    };

    loop {
        let (stream, _) = listener.accept().await?;

        let resp = async {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            let req: Request = serde_json::from_str(&line)?;
            let is_stop = matches!(req.command, Command::Stop);
            let resp = if !state.page_opened && req.command.requires_page() {
                Response::err("No page open. Use 'plwr open <url>' first.".to_string())
            } else {
                handle_command(&mut state, req.command, headed)
                    .await
                    .unwrap_or_else(|e| Response::err(clean_error(e)))
            };

            let mut buf = serde_json::to_vec(&resp)?;
            buf.push(b'\n');
            writer.write_all(&buf).await?;

            Ok::<bool, anyhow::Error>(is_stop)
        }.await;

        match resp {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => eprintln!("connection error: {}", e),
        }
    }

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    Ok(())
}

async fn handle_command(state: &mut State, command: Command, headed: bool) -> Result<Response> {
    // Handle commands that mutate state before borrowing the page
    match command {
        Command::Open { url } => {
            state.active_page().goto(&url, None).await?;
            state.page_opened = true;
            return Ok(Response::ok_empty());
        }
        Command::Header { name, value } => {
            state.headers.insert(name, value);
            let ctx = state.active_page().context()?;
            pw_ext::set_extra_http_headers(&ctx, state.headers.clone()).await?;
            return Ok(Response::ok_empty());
        }
        Command::HeaderClear => {
            state.headers.clear();
            let ctx = state.active_page().context()?;
            pw_ext::set_extra_http_headers(&ctx, HashMap::new()).await?;
            return Ok(Response::ok_empty());
        }
        Command::Cookie { name, value, url } => {
            let ctx = state.active_page().context()?;
            let url = if url.is_empty() {
                state.active_page().url()
            } else {
                url
            };
            pw_ext::add_cookie(&ctx, name, value, url).await?;
            return Ok(Response::ok_empty());
        }
        Command::CookieList => {
            let ctx = state.active_page().context()?;
            let cookies = pw_ext::get_cookies(&ctx).await?;
            let json: Vec<serde_json::Value> = cookies.iter().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "value": c.value,
                    "domain": c.domain,
                    "path": c.path,
                    "expires": c.expires,
                    "httpOnly": c.http_only,
                    "secure": c.secure,
                    "sameSite": c.same_site,
                })
            }).collect();
            return Ok(Response::ok_value(serde_json::Value::Array(json)));
        }
        Command::CookieClear => {
            let ctx = state.active_page().context()?;
            pw_ext::clear_cookies(&ctx).await?;
            return Ok(Response::ok_empty());
        }
        Command::Viewport { width, height } => {
            state.active_page().set_viewport_size(playwright_rs::Viewport { width, height }).await?;
            return Ok(Response::ok_empty());
        }
        _ => {}
    }

    let page = state.active_page();

    match command {
        Command::Stop => {
            Ok(Response::ok_empty())
        }

        Command::Reload => {
            page.reload(None).await?;
            Ok(Response::ok_empty())
        }

        Command::Url => {
            Ok(Response::ok_value(serde_json::Value::String(page.url())))
        }

        Command::Wait { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            Ok(Response::ok_empty())
        }

        Command::WaitNot { selector, timeout } => {
            let loc = page.locator(&selector).await;
            let start = std::time::Instant::now();
            loop {
                let n = loc.count().await?;
                if n == 0 {
                    return Ok(Response::ok_empty());
                }
                if start.elapsed().as_millis() as u64 > timeout {
                    anyhow::bail!("Timeout waiting for '{}' to disappear", selector);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Command::Click { selector, timeout } => {
            let loc = page.locator(&selector).await;
            loc.click(Some(ClickOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            })).await?;
            Ok(Response::ok_empty())
        }

        Command::Fill { selector, text, timeout } => {
            let loc = page.locator(&selector).await;
            loc.fill(&text, Some(FillOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            })).await?;
            Ok(Response::ok_empty())
        }

        Command::Press { key } => {
            match page.keyboard().press(&key, None).await {
                Ok(()) => Ok(Response::ok_empty()),
                Err(e) => {
                    let msg = clean_error(anyhow::anyhow!(e));
                    if msg.contains("Unknown key") {
                        Ok(Response::err(format!(
                            "{msg}\n\n\
                            Valid keys: a-z A-Z 0-9, \
                            Backspace Tab Enter Escape Space Delete Insert, \
                            ArrowUp ArrowDown ArrowLeft ArrowRight Home End PageUp PageDown, \
                            F1-F12, Control Shift Alt Meta, \
                            any US keyboard character: !@#$%^&*()_+-=[]{{}}\\|;':\",./<>?`~\n\
                            Chords: Control+c, Shift+Enter, Alt+Tab, Meta+a"
                        )))
                    } else {
                        Ok(Response::err(msg))
                    }
                }
            }
        }

        Command::Exists { selector } => {
            let loc = page.locator(&selector).await;
            let n = tokio::time::timeout(
                CHANNEL_TIMEOUT,
                loc.count(),
            ).await
                .map_err(|_| anyhow::anyhow!("Timeout waiting for Playwright response. [selector: {}]", selector))?
                ?;
            Ok(Response::ok_value(serde_json::Value::Bool(n > 0)))
        }

        Command::Text { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            let text = loc.text_content().await?.unwrap_or_default();
            Ok(Response::ok_value(serde_json::Value::String(text)))
        }

        Command::Attr { selector, name, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            match loc.get_attribute(&name).await? {
                Some(val) => Ok(Response::ok_value(serde_json::Value::String(val))),
                None => Ok(Response::ok_value(serde_json::Value::Null)),
            }
        }

        Command::Count { selector } => {
            let loc = page.locator(&selector).await;
            let n = tokio::time::timeout(
                CHANNEL_TIMEOUT,
                loc.count(),
            ).await
                .map_err(|_| anyhow::anyhow!("Timeout waiting for Playwright response. [selector: {}]", selector))?
                ?;
            Ok(Response::ok_value(serde_json::json!(n)))
        }

        Command::Eval { js } => {
            let wrapper = format!(
                "() => {{ const __r = ({}); return typeof __r === 'object' ? JSON.stringify(__r) : __r; }}",
                js
            );
            let val = pw_ext::page_evaluate_value(page, &wrapper).await?;
            match serde_json::from_str::<serde_json::Value>(&val) {
                Ok(serde_json::Value::String(s)) => {
                    match serde_json::from_str::<serde_json::Value>(&s) {
                        Ok(v @ serde_json::Value::Object(_)) | Ok(v @ serde_json::Value::Array(_)) => Ok(Response::ok_value(v)),
                        _ => Ok(Response::ok_value(serde_json::Value::String(s))),
                    }
                }
                Ok(v) => Ok(Response::ok_value(v)),
                Err(_) => Ok(Response::ok_value(serde_json::Value::String(val))),
            }
        }

        Command::Screenshot { selector, path, .. } => {
            let bytes = match &selector {
                Some(sel) => {
                    let loc = page.locator(sel).await;
                    loc.screenshot(None).await?
                }
                None => page.screenshot(None).await?,
            };
            std::fs::write(&path, &bytes)?;
            Ok(Response::ok_value(serde_json::Value::String(
                format!("Saved {} bytes to {}", bytes.len(), path),
            )))
        }

        Command::Tree { selector, .. } => {
            let walk_js = r#"el => {
                function walk(el) {
                    const node = { tag: el.tagName ? el.tagName.toLowerCase() : '#text' };
                    if (el.id) node.id = el.id;
                    if (el.className && typeof el.className === 'string' && el.className.trim())
                        node.class = el.className.trim().split(/\s+/);
                    if (el.attributes) {
                        const attrs = {};
                        for (const a of el.attributes) {
                            if (a.name !== 'id' && a.name !== 'class' && !a.name.startsWith('data-plwr'))
                                attrs[a.name] = a.value;
                        }
                        if (Object.keys(attrs).length > 0) node.attrs = attrs;
                    }
                    const text = Array.from(el.childNodes)
                        .filter(n => n.nodeType === 3)
                        .map(n => n.textContent.trim())
                        .filter(t => t)
                        .join(' ');
                    if (text) node.text = text;
                    const children = Array.from(el.children).map(walk);
                    if (children.length > 0) node.children = children;
                    return node;
                }
                return JSON.stringify(walk(el));
            }"#;
            let sel = selector.as_deref().unwrap_or("html");
            let val = pw_ext::locator_eval_on_selector(page, sel, walk_js).await?;
            let json_str: String = serde_json::from_str(&val).unwrap_or(val);
            let tree: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(Response::ok_value(tree))
        }

        Command::Open { .. }
        | Command::Header { .. } | Command::HeaderClear
        | Command::Cookie { .. } | Command::CookieList | Command::CookieClear
        | Command::Viewport { .. } => unreachable!(),

        Command::VideoStart { dir } => {
            std::fs::create_dir_all(&dir)?;
            let tmp_dir = tempfile::tempdir()?;
            let user_data = tmp_dir.path().to_string_lossy().to_string();

            let ctx = state.playwright.chromium()
                .launch_persistent_context_with_options(
                    user_data,
                    BrowserContextOptions {
                        headless: Some(!headed),
                        record_video: Some(RecordVideo {
                            dir: dir.clone(),
                            size: None,
                        }),
                        ..Default::default()
                    },
                ).await?;
            let vpage = ctx.new_page().await?;

            let url = state.page.url();
            if url != "about:blank" {
                let _ = vpage.goto(&url, None).await;
            }

            state.video_dir = Some(dir);
            state.video_page = Some(vpage);
            Ok(Response::ok_empty())
        }

        Command::VideoStop { output } => {
            if let (Some(vpage), Some(dir)) = (state.video_page.take(), state.video_dir.take()) {
                let ctx = vpage.context()?;
                ctx.close().await?;

                let webm = std::fs::read_dir(&dir)?
                    .filter_map(|e| e.ok())
                    .find(|e| e.path().extension().is_some_and(|ext| ext == "webm"))
                    .map(|e| e.path());

                match webm {
                    Some(webm_path) => {
                        if output.ends_with(".webm") {
                            std::fs::rename(&webm_path, &output)?;
                            Ok(Response::ok_value(serde_json::Value::String(
                                format!("Saved recording to {}", output),
                            )))
                        } else {
                            let status = std::process::Command::new("ffmpeg")
                                .args(["-y", "-i"])
                                .arg(&webm_path)
                                .arg(&output)
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status()?;
                            if status.success() {
                                std::fs::remove_file(&webm_path).ok();
                                Ok(Response::ok_value(serde_json::Value::String(
                                    format!("Saved recording to {}", output),
                                )))
                            } else {
                                Ok(Response::err(format!("ffmpeg exited with {}", status)))
                            }
                        }
                    }
                    None => Ok(Response::err("No video file found".to_string())),
                }
            } else {
                Ok(Response::err("No video recording in progress".to_string()))
            }
        }
    }
}

async fn wait_for_visible(loc: &Locator, selector: &str, timeout: u64) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        let n = loc.count().await?;
        if n > 0 && loc.first().is_visible().await? {
            return Ok(());
        }
        if start.elapsed().as_millis() as u64 > timeout {
            anyhow::bail!("Timeout {}ms exceeded. [selector: {}]", timeout, selector);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn clean_error(e: anyhow::Error) -> String {
    let msg = e.to_string();

    // Extract [selector: ...] suffix before stripping (it may be at the very end,
    // after stack traces that we're about to remove)
    let selector_suffix = msg.rfind("[selector: ").map(|i| &msg[i..])
        .and_then(|s| s.find(']').map(|j| &s[..=j]))
        .unwrap_or("");

    // Strip stack traces: everything after " \n " (Playwright appends " \n stack")
    let msg = msg.split(" \n ").next().unwrap_or(&msg);
    // Also strip lines starting with "    at " (JS stack frames)
    let msg = msg.lines()
        .take_while(|l| !l.starts_with("    at "))
        .collect::<Vec<_>>()
        .join("\n");

    // Strip nested prefixes layered by playwright-rs and the Playwright server
    let msg = msg.strip_prefix("Protocol error: ").unwrap_or(&msg);
    let msg = msg.strip_prefix("Protocol error ").unwrap_or(msg);
    let msg = if msg.starts_with('(') {
        msg.find(": ").map(|i| &msg[i + 2..]).unwrap_or(msg)
    } else {
        msg
    };
    let msg = msg.strip_prefix("Error: ").unwrap_or(msg);
    let msg = msg.strip_prefix("strict mode violation: ").unwrap_or(msg);

    let first_line = msg.lines().next().unwrap_or(msg).trim_end();

    let cleaned = if selector_suffix.is_empty() || first_line.ends_with(']') {
        first_line.to_string()
    } else {
        format!("{} {}", first_line, selector_suffix)
    };

    if cleaned.contains("resolved to") && cleaned.contains("elements") {
        let sel = selector_suffix
            .strip_prefix("[selector: ")
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or("SELECTOR");
        format!(
            "{cleaned}\n\nHint: use '>> nth=0' to select the first match, e.g.:\n  \
            plwr <command> \"{sel} >> nth=0\""
        )
    } else {
        cleaned
    }
}


