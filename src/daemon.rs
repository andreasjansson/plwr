use crate::protocol::{Command, Request, Response};
use crate::pw_ext;
use anyhow::Result;
use playwright_rs::{
    protocol::click::{KeyboardModifier, MouseButton},
    BrowserContextOptions, CheckOptions, ClickOptions, FillOptions, HoverOptions, LaunchOptions,
    Locator, Page, Playwright, RecordVideo, SelectOption, SelectOptions,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

const READY_SIGNAL: &str = "### ready";
const ERROR_PREFIX: &str = "### error ";
const CHANNEL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

const NETWORK_INTERCEPTOR_JS: &str = r#"
if (!window.__plwr_network) {
    window.__plwr_network = [];
    window.__plwr_network_fetch_queue = {};
    window.__plwr_network_xhr_queue = {};

    // Classify by initiatorType + URL extension
    function classifyType(entry) {
        const url = entry.name || '';
        const init = entry.initiatorType || '';
        if (entry.entryType === 'navigation') return 'doc';
        if (init === 'fetch') return 'fetch';
        if (init === 'xmlhttprequest') return 'xhr';
        if (init === 'script') return 'js';
        if (init === 'link') {
            if (/\.css(\?|$)/i.test(url)) return 'css';
            if (/manifest/i.test(url)) return 'manifest';
            return 'css';
        }
        if (init === 'img') return 'img';
        if (init === 'audio' || init === 'video') return 'media';
        if (init === 'css') {
            if (/\.(woff2?|ttf|otf|eot)(\?|$)/i.test(url)) return 'font';
            return 'img';
        }
        if (/\.wasm(\?|$)/i.test(url)) return 'wasm';
        if (/\.(js|mjs)(\?|$)/i.test(url)) return 'js';
        if (/\.css(\?|$)/i.test(url)) return 'css';
        if (/\.(png|jpe?g|gif|svg|webp|ico|bmp|avif)(\?|$)/i.test(url)) return 'img';
        if (/\.(mp4|webm|ogg|mp3|wav|flac|aac)(\?|$)/i.test(url)) return 'media';
        if (/\.(woff2?|ttf|otf|eot)(\?|$)/i.test(url)) return 'font';
        return 'other';
    }

    function resolveURL(url) {
        try { return new URL(url, location.href).href; }
        catch { return url; }
    }

    // Monkey-patch fetch to capture method
    const origFetch = window.fetch;
    window.fetch = function(input, init) {
        const url = resolveURL(
            (typeof input === 'string') ? input
            : (input instanceof URL) ? input.href
            : (input instanceof Request) ? input.url
            : String(input)
        );
        const method = (init && init.method) ? init.method.toUpperCase()
            : (input instanceof Request) ? input.method.toUpperCase()
            : 'GET';
        (window.__plwr_network_fetch_queue[url] = window.__plwr_network_fetch_queue[url] || []).push(method);
        return origFetch.apply(this, arguments);
    };

    // Monkey-patch XMLHttpRequest to capture method
    const origXHROpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__plwr_method = method.toUpperCase();
        this.__plwr_url = resolveURL(
            (typeof url === 'string') ? url
            : (url instanceof URL) ? url.href : String(url)
        );
        return origXHROpen.apply(this, arguments);
    };
    const origXHRSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function() {
        if (this.__plwr_url) {
            (window.__plwr_network_xhr_queue[this.__plwr_url] = window.__plwr_network_xhr_queue[this.__plwr_url] || []).push(this.__plwr_method);
        }
        return origXHRSend.apply(this, arguments);
    };

    // Monkey-patch WebSocket
    const OrigWS = window.WebSocket;
    window.WebSocket = function(url, protocols) {
        const ws = protocols !== undefined
            ? new OrigWS(url, protocols)
            : new OrigWS(url);
        const entry = {
            type: 'ws',
            url: (typeof url === 'string') ? url : url.href,
            status: null,
            duration: 0,
            ts: Date.now(),
            messages: []
        };
        ws.addEventListener('open', function() {
            entry.status = 101;
        });
        ws.addEventListener('message', function(e) {
            entry.messages.push({ dir: 'recv', data: typeof e.data === 'string' ? e.data : '<binary>', ts: Date.now() });
        });
        ws.addEventListener('close', function() {
            entry.duration = Date.now() - entry.ts;
        });
        ws.addEventListener('error', function() {
            entry.status = 0;
            entry.duration = Date.now() - entry.ts;
        });
        const origSend = ws.send.bind(ws);
        ws.send = function(data) {
            entry.messages.push({ dir: 'send', data: typeof data === 'string' ? data : '<binary>', ts: Date.now() });
            return origSend(data);
        };
        window.__plwr_network.push(entry);
        return ws;
    };
    window.WebSocket.prototype = OrigWS.prototype;
    window.WebSocket.CONNECTING = OrigWS.CONNECTING;
    window.WebSocket.OPEN = OrigWS.OPEN;
    window.WebSocket.CLOSING = OrigWS.CLOSING;
    window.WebSocket.CLOSED = OrigWS.CLOSED;

    function queueShift(queue, url) {
        if (queue[url] && queue[url].length) return queue[url].shift();
        return null;
    }

    // PerformanceObserver for resource and navigation entries
    function processEntry(entry) {
        const type = classifyType(entry);
        const url = entry.name;

        let method = null;
        if (type === 'fetch') {
            method = queueShift(window.__plwr_network_fetch_queue, url) || 'GET';
        } else if (type === 'xhr') {
            method = queueShift(window.__plwr_network_xhr_queue, url) || 'GET';
        } else if (type === 'doc') {
            method = 'GET';
        }

        window.__plwr_network.push({
            type: type,
            url: url,
            status: entry.responseStatus || null,
            method: method,
            size: entry.transferSize || null,
            duration: Math.round(entry.duration),
            ts: Math.round(performance.timeOrigin + entry.startTime)
        });
    }

    const obs = new PerformanceObserver(function(list) {
        list.getEntries().forEach(processEntry);
    });
    obs.observe({ type: 'resource', buffered: true });
    obs.observe({ type: 'navigation', buffered: true });
}
"#;

const CONSOLE_INTERCEPTOR_JS: &str = r#"
if (!window.__plwr_console) {
    window.__plwr_console = [];
    const orig = {};
    for (const level of ['log', 'warn', 'error', 'info', 'debug']) {
        orig[level] = console[level];
        console[level] = (...args) => {
            window.__plwr_console.push({
                level,
                ts: Date.now(),
                args: args.map(a => {
                    try { return typeof a === 'object' ? JSON.stringify(a) : String(a); }
                    catch { return String(a); }
                })
            });
            orig[level].apply(console, args);
        };
    }
}
"#;

enum DialogAction {
    Accept(Option<String>),
    Dismiss,
}

struct State {
    _playwright: Playwright,
    page: Page,
    page_opened: bool,
    headers: HashMap<String, String>,
    video: Option<VideoState>,
    console_initialized: bool,
    network_initialized: bool,
    dialog_action: Arc<Mutex<Option<DialogAction>>>,
    dialog_installed: bool,
    clipboard_granted: bool,
    cdp: bool,
}

struct VideoState {
    output_path: String,
    temp_dir: std::path::PathBuf,
}

pub async fn run(socket_path: &Path, headed: bool, ignore_cert_errors: bool) -> Result<()> {
    // Ignore SIGPIPE — stdout is a pipe from the parent process that
    // closes after reading the ready signal. Any later stdout write
    // (e.g. from Playwright internals) must not kill us.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let playwright = match Playwright::launch().await {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                println!(
                    "{}Playwright not found. Install with: npm install -g playwright && npx playwright install chromium",
                    ERROR_PREFIX
                );
            } else {
                println!("{}{}", ERROR_PREFIX, msg);
            }
            return Err(e.into());
        }
    };

    let cdp_channel = std::env::var("PLWR_CDP").ok();
    let is_cdp = cdp_channel.is_some();

    let (page, video) = if let Some(ref channel) = cdp_channel {
        let ws_url = match resolve_cdp_endpoint(channel) {
            Ok(url) => url,
            Err(e) => {
                println!("{}{}", ERROR_PREFIX, e);
                return Err(e);
            }
        };
        let result = match pw_ext::connect_over_cdp(playwright.chromium(), &ws_url, 30000.0).await {
            Ok(r) => r,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("refused") || msg.contains("403") {
                    println!(
                        "{}Connection refused. Did you click \"Allow\" in Chrome's remote debugging dialog?",
                        ERROR_PREFIX
                    );
                } else {
                    println!("{}{}", ERROR_PREFIX, msg);
                }
                return Err(e.into());
            }
        };
        let page = match &result.default_context {
            Some(ctx) => ctx.new_page().await,
            None => result.browser.new_page().await,
        };
        let page = match page {
            Ok(p) => p,
            Err(e) => {
                println!("{}{}", ERROR_PREFIX, e);
                return Err(e.into());
            }
        };
        (page, None)
    } else {
        let video_output = std::env::var("PLWR_VIDEO").ok();

        let args = if ignore_cert_errors {
            Some(vec!["--ignore-certificate-errors".to_string()])
        } else {
            None
        };

        let browser = match playwright
            .chromium()
            .launch_with_options(LaunchOptions {
                headless: Some(!headed),
                args,
                ..Default::default()
            })
            .await
        {
            Ok(b) => b,
            Err(e) => {
                println!("{}{}", ERROR_PREFIX, e);
                return Err(e.into());
            }
        };

        let video = if let Some(ref output_path) = video_output {
            let cache = dirs::cache_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("plwr")
                .join("video");
            std::fs::create_dir_all(&cache)?;
            let temp_dir = tempfile::tempdir_in(&cache)?.keep();
            Some(VideoState {
                output_path: output_path.clone(),
                temp_dir,
            })
        } else {
            None
        };

        let page = if let Some(ref vs) = video {
            let ctx = match browser
                .new_context_with_options(BrowserContextOptions {
                    record_video: Some(RecordVideo {
                        dir: vs.temp_dir.to_string_lossy().to_string(),
                        size: None,
                    }),
                    ..Default::default()
                })
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    println!("{}{}", ERROR_PREFIX, e);
                    return Err(e.into());
                }
            };
            match ctx.new_page().await {
                Ok(p) => p,
                Err(e) => {
                    println!("{}{}", ERROR_PREFIX, e);
                    return Err(e.into());
                }
            }
        } else {
            match browser.new_page().await {
                Ok(p) => p,
                Err(e) => {
                    println!("{}{}", ERROR_PREFIX, e);
                    return Err(e.into());
                }
            }
        };

        (page, video)
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
        _playwright: playwright,
        page,
        page_opened: false,
        headers: HashMap::new(),
        video,
        console_initialized: false,
        network_initialized: false,
        dialog_action: Arc::new(Mutex::new(None)),
        dialog_installed: false,
        clipboard_granted: false,
        cdp: is_cdp,
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
                handle_command(&mut state, req.command)
                    .await
                    .unwrap_or_else(|e| Response::err(clean_error(e)))
            };

            let mut buf = serde_json::to_vec(&resp)?;
            buf.push(b'\n');
            writer.write_all(&buf).await?;

            Ok::<bool, anyhow::Error>(is_stop)
        }
        .await;

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

async fn handle_command(state: &mut State, command: Command) -> Result<Response> {
    // Handle commands that mutate state before borrowing the page
    match command {
        Command::Open { url, timeout } => {
            if !state.cdp && !state.console_initialized {
                state.page.add_init_script(CONSOLE_INTERCEPTOR_JS).await?;
                state.console_initialized = true;
            }
            if !state.cdp && !state.network_initialized {
                state.page.add_init_script(NETWORK_INTERCEPTOR_JS).await?;
                state.network_initialized = true;
            }
            state
                .page
                .goto(
                    &url,
                    Some(playwright_rs::GotoOptions {
                        timeout: Some(std::time::Duration::from_millis(timeout)),
                        wait_until: None,
                    }),
                )
                .await?;
            if state.cdp {
                pw_ext::page_evaluate_value(&state.page, CONSOLE_INTERCEPTOR_JS).await?;
                pw_ext::page_evaluate_value(&state.page, NETWORK_INTERCEPTOR_JS).await?;
            }
            state.page_opened = true;
            return Ok(Response::ok_empty());
        }
        Command::Header { name, value } => {
            state.headers.insert(name, value);
            let ctx = &state.page.context()?;
            pw_ext::set_extra_http_headers(ctx, state.headers.clone()).await?;
            return Ok(Response::ok_empty());
        }
        Command::HeaderClear => {
            state.headers.clear();
            let ctx = &state.page.context()?;
            pw_ext::set_extra_http_headers(ctx, HashMap::new()).await?;
            return Ok(Response::ok_empty());
        }
        Command::Cookie { name, value, url } => {
            let ctx = state.page.context()?;
            let url = if url.is_empty() {
                state.page.url()
            } else {
                url
            };
            pw_ext::add_cookie(&ctx, name, value, url).await?;
            return Ok(Response::ok_empty());
        }
        Command::CookieList => {
            let ctx = &state.page.context()?;
            let cookies = pw_ext::get_cookies(ctx).await?;
            let json: Vec<serde_json::Value> = cookies
                .iter()
                .map(|c| {
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
                })
                .collect();
            return Ok(Response::ok_value(serde_json::Value::Array(json)));
        }
        Command::CookieClear => {
            let ctx = &state.page.context()?;
            pw_ext::clear_cookies(ctx).await?;
            return Ok(Response::ok_empty());
        }
        Command::Viewport { width, height } => {
            state
                .page
                .set_viewport_size(playwright_rs::Viewport { width, height })
                .await?;
            return Ok(Response::ok_empty());
        }
        Command::ClipboardCopy { selector, timeout } => {
            ensure_clipboard_permissions(state).await?;
            let loc = state.page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;

            // For <img> and <canvas> elements, copies as image/png.
            // For everything else, copies textContent.
            let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
            let js = format!(
                r#"async () => {{
                    const el = document.querySelector('{}');
                    if (!el) throw new Error('No element found');
                    const tag = el.tagName.toLowerCase();
                    if (tag === 'img') {{
                        const resp = await fetch(el.src);
                        const blob = await resp.blob();
                        const pngBlob = await createImageBitmap(blob).then(bmp => {{
                            const c = document.createElement('canvas');
                            c.width = bmp.width;
                            c.height = bmp.height;
                            c.getContext('2d').drawImage(bmp, 0, 0);
                            return new Promise(r => c.toBlob(r, 'image/png'));
                        }});
                        await navigator.clipboard.write([new ClipboardItem({{'image/png': pngBlob}})]);
                        return 'image';
                    }} else if (tag === 'canvas') {{
                        const blob = await new Promise(r => el.toBlob(r, 'image/png'));
                        await navigator.clipboard.write([new ClipboardItem({{'image/png': blob}})]);
                        return 'image';
                    }} else {{
                        const text = el.textContent || '';
                        await navigator.clipboard.writeText(text);
                        return 'text';
                    }}
                }}"#,
                escaped
            );
            pw_ext::page_evaluate_value(&state.page, &js).await?;
            return Ok(Response::ok_empty());
        }
        Command::ClipboardPaste => {
            ensure_clipboard_permissions(state).await?;
            let js = r#"async () => {
                const items = await navigator.clipboard.read();
                const active = document.activeElement;
                if (!active) throw new Error('No focused element');
                for (const item of items) {
                    if (item.types.includes('image/png')) {
                        const blob = await item.getType('image/png');
                        const dt = new DataTransfer();
                        dt.items.add(new File([blob], 'paste.png', { type: 'image/png' }));
                        active.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true }));
                        return;
                    }
                }
                const text = await navigator.clipboard.readText();
                const dt = new DataTransfer();
                dt.setData('text/plain', text);
                active.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true }));
                if (active.matches('input,textarea,[contenteditable]')) {
                    document.execCommand('insertText', false, text);
                }
            }"#;
            pw_ext::page_evaluate_value(&state.page, js).await?;
            return Ok(Response::ok_empty());
        }
        _ => {}
    }

    let page = &state.page;

    match command {
        Command::Stop => {
            if state.cdp {
                state.page.close().await.ok();
                return Ok(Response::ok_empty());
            }
            if let Some(vs) = state.video.take() {
                let ctx = state.page.context()?;
                state.page.close().await?;
                ctx.close().await?;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let webm = std::fs::read_dir(&vs.temp_dir)?
                    .filter_map(|e| e.ok())
                    .find(|e| e.path().extension().is_some_and(|ext| ext == "webm"))
                    .map(|e| e.path());

                if let Some(webm) = webm {
                    if vs.output_path.ends_with(".webm") {
                        std::fs::copy(&webm, &vs.output_path)?;
                    } else {
                        let status = std::process::Command::new("ffmpeg")
                            .args(["-y", "-i"])
                            .arg(&webm)
                            .arg(&vs.output_path)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status()?;
                        if !status.success() {
                            std::fs::remove_dir_all(&vs.temp_dir).ok();
                            return Ok(Response::err(format!("ffmpeg exited with {}", status)));
                        }
                    }
                }
                std::fs::remove_dir_all(&vs.temp_dir).ok();
            }
            Ok(Response::ok_empty())
        }

        Command::Reload => {
            page.reload(None).await?;
            Ok(Response::ok_empty())
        }

        Command::Url => Ok(Response::ok_value(serde_json::Value::String(page.url()))),

        Command::Wait { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            Ok(Response::ok_empty())
        }

        Command::WaitAny { selectors, timeout } => {
            let start = std::time::Instant::now();
            loop {
                for sel in &selectors {
                    let loc = page.locator(sel).await;
                    let n = match loc.count().await {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    if n > 0 {
                        let visible = loc.first().is_visible().await.unwrap_or(false);
                        if visible {
                            return Ok(Response::ok_value(serde_json::Value::String(sel.clone())));
                        }
                    }
                }
                if start.elapsed().as_millis() as u64 > timeout {
                    let list = selectors.join(", ");
                    anyhow::bail!("Timeout {}ms exceeded. None matched: [{}]", timeout, list);
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }

        Command::WaitAll { selectors, timeout } => {
            let start = std::time::Instant::now();
            loop {
                let mut all_visible = true;
                for sel in &selectors {
                    let loc = page.locator(sel).await;
                    let n = match loc.count().await {
                        Ok(n) => n,
                        Err(_) => {
                            all_visible = false;
                            break;
                        }
                    };
                    if n == 0 || !loc.first().is_visible().await.unwrap_or(false) {
                        all_visible = false;
                        break;
                    }
                }
                if all_visible {
                    return Ok(Response::ok_empty());
                }
                if start.elapsed().as_millis() as u64 > timeout {
                    let mut missing = Vec::new();
                    for sel in &selectors {
                        let loc = page.locator(sel).await;
                        let n = loc.count().await?;
                        if n == 0 || !loc.first().is_visible().await? {
                            missing.push(sel.as_str());
                        }
                    }
                    anyhow::bail!(
                        "Timeout {}ms exceeded. Still missing: [{}]",
                        timeout,
                        missing.join(", ")
                    );
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }

        Command::WaitNot { selector, timeout } => {
            let loc = page.locator(&selector).await;
            let start = std::time::Instant::now();
            loop {
                let n = loc.count().await.unwrap_or(0);
                if n == 0 {
                    return Ok(Response::ok_empty());
                }
                if start.elapsed().as_millis() as u64 > timeout {
                    anyhow::bail!("Timeout waiting for '{}' to disappear", selector);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Command::Click {
            selector,
            timeout,
            modifiers,
            button,
        } => {
            let loc = page.locator(&selector).await;
            loc.click(Some(ClickOptions {
                timeout: Some(timeout as f64),
                modifiers: parse_modifiers(&modifiers),
                button: parse_button(button.as_deref()),
                ..Default::default()
            }))
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Fill {
            selector,
            text,
            timeout,
        } => {
            let loc = page.locator(&selector).await;
            loc.fill(
                &text,
                Some(FillOptions {
                    timeout: Some(timeout as f64),
                    ..Default::default()
                }),
            )
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Press { key } => match page.keyboard().press(&key, None).await {
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
        },

        Command::Exists { selector } => {
            let loc = page.locator(&selector).await;
            let n = tokio::time::timeout(CHANNEL_TIMEOUT, loc.count())
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Timeout waiting for Playwright response. [selector: {}]",
                        selector
                    )
                })??;
            Ok(Response::ok_value(serde_json::Value::Bool(n > 0)))
        }

        Command::Text { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            let text = loc.text_content().await?.unwrap_or_default();
            Ok(Response::ok_value(serde_json::Value::String(text)))
        }

        Command::Attr {
            selector,
            name,
            timeout,
        } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            match loc.get_attribute(&name).await? {
                Some(val) => Ok(Response::ok_value(serde_json::Value::String(val))),
                None => Ok(Response::ok_value(serde_json::Value::Null)),
            }
        }

        Command::Count { selector } => {
            let loc = page.locator(&selector).await;
            let n = tokio::time::timeout(CHANNEL_TIMEOUT, loc.count())
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Timeout waiting for Playwright response. [selector: {}]",
                        selector
                    )
                })??;
            Ok(Response::ok_value(serde_json::json!(n)))
        }

        Command::InputFiles {
            selector, paths, ..
        } => {
            let loc = page.locator(&selector).await;
            if paths.is_empty() {
                loc.set_input_files_multiple(&[], None).await?;
            } else {
                let pathbufs: Vec<std::path::PathBuf> =
                    paths.iter().map(std::path::PathBuf::from).collect();
                let refs: Vec<&std::path::PathBuf> = pathbufs.iter().collect();
                loc.set_input_files_multiple(&refs, None).await?;
            }
            Ok(Response::ok_empty())
        }

        Command::Select {
            selector,
            values,
            by_label,
            timeout,
        } => {
            let loc = page.locator(&selector).await;
            let opts = Some(SelectOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            });
            let select_values: Vec<SelectOption> = values
                .into_iter()
                .map(|v| {
                    if by_label {
                        SelectOption::Label(v)
                    } else {
                        SelectOption::Value(v)
                    }
                })
                .collect();
            if select_values.len() == 1 {
                loc.select_option(select_values.into_iter().next().unwrap(), opts)
                    .await?;
            } else {
                loc.select_option_multiple(&select_values, opts).await?;
            }
            Ok(Response::ok_empty())
        }

        Command::Hover { selector, timeout } => {
            let loc = page.locator(&selector).await;
            loc.hover(Some(HoverOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            }))
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Check { selector, timeout } => {
            let loc = page.locator(&selector).await;
            loc.check(Some(CheckOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            }))
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Uncheck { selector, timeout } => {
            let loc = page.locator(&selector).await;
            loc.uncheck(Some(CheckOptions {
                timeout: Some(timeout as f64),
                ..Default::default()
            }))
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Dblclick {
            selector,
            timeout,
            modifiers,
            button,
        } => {
            let loc = page.locator(&selector).await;
            loc.dblclick(Some(ClickOptions {
                timeout: Some(timeout as f64),
                modifiers: parse_modifiers(&modifiers),
                button: parse_button(button.as_deref()),
                ..Default::default()
            }))
            .await?;
            Ok(Response::ok_empty())
        }

        Command::Focus { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            loc.click(Some(ClickOptions {
                trial: Some(true),
                timeout: Some(timeout as f64),
                ..Default::default()
            }))
            .await?;
            pw_ext::locator_focus(page, &selector).await?;
            Ok(Response::ok_empty())
        }

        Command::Blur { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            pw_ext::locator_blur(page, &selector).await?;
            Ok(Response::ok_empty())
        }

        Command::InnerHtml { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            let html = loc.inner_html().await?;
            Ok(Response::ok_value(serde_json::Value::String(html)))
        }

        Command::InputValue { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            let val = loc.input_value(None).await?;
            Ok(Response::ok_value(serde_json::Value::String(val)))
        }

        Command::ScrollIntoView { selector, timeout } => {
            let loc = page.locator(&selector).await;
            wait_for_visible(&loc, &selector, timeout).await?;
            pw_ext::locator_scroll_into_view(page, &selector).await?;
            Ok(Response::ok_empty())
        }

        Command::ComputedStyle {
            selector,
            properties,
            timeout,
        } => {
            let loc = page.locator(&selector).await;
            let start = std::time::Instant::now();
            loop {
                if loc.count().await.unwrap_or(0) > 0 {
                    break;
                }
                if start.elapsed().as_millis() as u64 > timeout {
                    anyhow::bail!("Timeout {}ms: element not found [{}]", timeout, selector);
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }

            let js = if properties.is_empty() {
                r#"el => {
                    const cs = getComputedStyle(el);
                    const result = {};
                    for (let i = 0; i < cs.length; i++) {
                        const prop = cs[i];
                        result[prop] = cs.getPropertyValue(prop);
                    }
                    return JSON.stringify(result);
                }"#
                .to_string()
            } else {
                let props_js: Vec<String> = properties
                    .iter()
                    .map(|p| format!("'{}'", p.replace('\'', "\\'")))
                    .collect();
                format!(
                    r#"el => {{
                    const cs = getComputedStyle(el);
                    const props = [{}];
                    const result = {{}};
                    for (const p of props) {{ result[p] = cs.getPropertyValue(p); }}
                    return JSON.stringify(result);
                }}"#,
                    props_js.join(", ")
                )
            };

            let val = pw_ext::locator_eval_on_selector(page, &selector, &js).await?;
            let json_str: String = serde_json::from_str(&val).unwrap_or(val);
            let styles: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(Response::ok_value(styles))
        }

        Command::DialogAccept { prompt_text } => {
            install_dialog_handler(state).await?;
            *state.dialog_action.lock().unwrap() = Some(DialogAction::Accept(prompt_text));
            Ok(Response::ok_empty())
        }

        Command::DialogDismiss => {
            install_dialog_handler(state).await?;
            *state.dialog_action.lock().unwrap() = Some(DialogAction::Dismiss);
            Ok(Response::ok_empty())
        }

        Command::Console => {
            let val = pw_ext::page_evaluate_value(
                page,
                "() => JSON.stringify(window.__plwr_console || [])",
            )
            .await?;
            let json_str: String = serde_json::from_str(&val).unwrap_or(val);
            let logs: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(Response::ok_value(logs))
        }

        Command::ConsoleClear => {
            pw_ext::page_evaluate_value(page, "() => { window.__plwr_console = []; }").await?;
            Ok(Response::ok_empty())
        }

        Command::Network {
            types,
            url_pattern,
            include_ws_messages,
        } => {
            let val = pw_ext::page_evaluate_value(
                page,
                "() => JSON.stringify(window.__plwr_network || [])",
            )
            .await?;
            let json_str: String = serde_json::from_str(&val).unwrap_or(val);
            let entries: serde_json::Value = serde_json::from_str(&json_str)?;

            let url_regex = url_pattern
                .as_deref()
                .map(regex::Regex::new)
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid URL regex: {}", e))?;

            let strip_messages = |e: &serde_json::Value| -> serde_json::Value {
                let mut e = e.clone();
                if let Some(obj) = e.as_object_mut() {
                    obj.remove("messages");
                }
                e
            };

            let filtered = entries
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter(|e| {
                            let type_ok = types.is_empty()
                                || e.get("type")
                                    .and_then(|t| t.as_str())
                                    .is_some_and(|t| types.iter().any(|f| f == t));
                            let url_ok = url_regex.as_ref().is_none_or(|re| {
                                e.get("url")
                                    .and_then(|u| u.as_str())
                                    .is_some_and(|u| re.is_match(u))
                            });
                            type_ok && url_ok
                        })
                        .map(|e| {
                            if include_ws_messages {
                                e.clone()
                            } else {
                                strip_messages(e)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(Response::ok_value(serde_json::Value::Array(filtered)))
        }

        Command::NetworkClear => {
            pw_ext::page_evaluate_value(page, "() => { window.__plwr_network = []; }").await?;
            Ok(Response::ok_empty())
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
                        Ok(v @ serde_json::Value::Object(_))
                        | Ok(v @ serde_json::Value::Array(_)) => Ok(Response::ok_value(v)),
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
            Ok(Response::ok_value(serde_json::Value::String(format!(
                "Saved {} bytes to {}",
                bytes.len(),
                path
            ))))
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
        | Command::Header { .. }
        | Command::HeaderClear
        | Command::Cookie { .. }
        | Command::CookieList
        | Command::CookieClear
        | Command::Viewport { .. }
        | Command::ClipboardCopy { .. }
        | Command::ClipboardPaste => unreachable!(),
    }
}

async fn ensure_clipboard_permissions(state: &mut State) -> Result<()> {
    if state.clipboard_granted {
        return Ok(());
    }
    let ctx = state.page.context()?;
    pw_ext::grant_permissions(&ctx, &["clipboard-read", "clipboard-write"]).await?;
    state.clipboard_granted = true;
    Ok(())
}

async fn install_dialog_handler(state: &mut State) -> Result<()> {
    if state.dialog_installed {
        return Ok(());
    }
    let action_ref = Arc::clone(&state.dialog_action);
    state
        .page
        .on_dialog(move |dialog| {
            let action_ref = Arc::clone(&action_ref);
            async move {
                let action = action_ref.lock().unwrap().take();
                match action {
                    Some(DialogAction::Accept(text)) => dialog.accept(text.as_deref()).await,
                    Some(DialogAction::Dismiss) => dialog.dismiss().await,
                    None => dialog.dismiss().await,
                }
            }
        })
        .await?;
    state.dialog_installed = true;
    Ok(())
}

async fn wait_for_visible(loc: &Locator, selector: &str, timeout: u64) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        let n = loc.count().await.unwrap_or_default();
        if n > 0 && loc.first().is_visible().await.unwrap_or(false) {
            return Ok(());
        }
        if start.elapsed().as_millis() as u64 > timeout {
            anyhow::bail!("Timeout {}ms exceeded. [selector: {}]", timeout, selector);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn parse_modifiers(modifiers: &[String]) -> Option<Vec<KeyboardModifier>> {
    if modifiers.is_empty() {
        return None;
    }
    Some(
        modifiers
            .iter()
            .map(|m| match m.as_str() {
                "Alt" => KeyboardModifier::Alt,
                "Control" => KeyboardModifier::Control,
                "Meta" => KeyboardModifier::Meta,
                "Shift" => KeyboardModifier::Shift,
                other => panic!("Unknown modifier: {}", other),
            })
            .collect(),
    )
}

fn parse_button(button: Option<&str>) -> Option<MouseButton> {
    button.map(|b| match b {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        other => panic!("Unknown button: {}", other),
    })
}

fn resolve_cdp_endpoint(arg: &str) -> Result<String> {
    if arg.starts_with("ws://") || arg.starts_with("wss://") {
        return Ok(arg.to_string());
    }
    match arg {
        "stable" | "beta" | "canary" | "dev" | "" => {
            read_devtools_ws_url_from_dir(&chrome_user_data_dir(arg))
        }
        path => {
            let expanded = if let Some(rest) = path.strip_prefix('~') {
                let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
                home.join(rest.strip_prefix('/').unwrap_or(rest))
            } else {
                std::path::PathBuf::from(path)
            };
            read_devtools_ws_url_from_dir(&expanded)
        }
    }
}

fn chrome_user_data_dir(channel: &str) -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("PLWR_CDP_USER_DATA_DIR") {
        return std::path::PathBuf::from(dir);
    }
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
    if cfg!(target_os = "macos") {
        let suffix = match channel {
            "stable" | "" => "Google/Chrome",
            "beta" => "Google/Chrome Beta",
            "canary" => "Google/Chrome Canary",
            "dev" => "Google/Chrome Dev",
            other => other,
        };
        home.join("Library/Application Support").join(suffix)
    } else {
        // Linux
        let suffix = match channel {
            "stable" | "" => "google-chrome",
            "beta" => "google-chrome-beta",
            "canary" | "dev" => "google-chrome-unstable",
            other => other,
        };
        home.join(".config").join(suffix)
    }
}

fn read_devtools_ws_url_from_dir(user_data_dir: &std::path::Path) -> Result<String> {
    let port_file = user_data_dir.join("DevToolsActivePort");
    let content = std::fs::read_to_string(&port_file).map_err(|_| {
        anyhow::anyhow!(
            "Could not find DevToolsActivePort in {}. Enable remote debugging: chrome://inspect/#remote-debugging",
            port_file.display()
        )
    })?;
    let mut lines = content.lines();
    let port = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("DevToolsActivePort is empty: {}", port_file.display()))?;
    let ws_path = lines.next().ok_or_else(|| {
        anyhow::anyhow!(
            "DevToolsActivePort missing WebSocket path: {}",
            port_file.display()
        )
    })?;
    Ok(format!("ws://127.0.0.1:{}{}", port.trim(), ws_path.trim()))
}

fn clean_error(e: anyhow::Error) -> String {
    let msg = e.to_string();

    // Extract [selector: ...] suffix before stripping (it may be at the very end,
    // after stack traces that we're about to remove)
    let selector_suffix = msg
        .rfind("[selector: ")
        .map(|i| &msg[i..])
        .and_then(|s| s.find(']').map(|j| &s[..=j]))
        .unwrap_or("");

    // Strip stack traces: everything after " \n " (Playwright appends " \n stack")
    let msg = msg.split(" \n ").next().unwrap_or(&msg);
    // Also strip lines starting with "    at " (JS stack frames)
    let msg = msg
        .lines()
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
