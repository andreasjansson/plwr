# plwr

Clean CLI for Playwright browser automation with CSS selectors. Built on [playwright-rs](https://github.com/padamson/playwright-rust).

## Install

```bash
./script/install              # installs to ~/.local/bin
./script/install -d ~/bin     # custom directory
```

Requires Playwright browsers:

```bash
npx playwright@1.56.1 install chromium
```

For video conversion to non-webm formats, install [ffmpeg](https://ffmpeg.org/).

## Usage

The browser starts automatically on first use and stays alive for subsequent
commands. No setup step needed.

```bash
plwr open https://example.com   # auto-starts browser, navigates
plwr text h1                    # Example Domain
plwr stop                       # shut down browser
```

### Environment variables

| Variable | Effect |
|----------|--------|
| `PLAYWRIGHT_HEADED` | Set to any value to run the browser with a visible window |
| `PLWR_SESSION` | Default session name (default: `default`) |
| `PLWR_TIMEOUT` | Default timeout in ms (default: `5000`) |

All commands take `-S`/`--session` and `-T`/`--timeout` as global options,
which override the environment variables.

### Navigation

`open` navigates the current page within the existing browser context. Headers,
cookies, and other state are preserved across navigations. There is no separate
`goto` command — `open` always reuses the same context. If you need a fresh
context, use `plwr stop` followed by `plwr open`.

```bash
plwr open "https://example.com"
plwr reload
plwr url
```

### Waiting

```bash
plwr wait .my-element
plwr wait-not .loading-spinner -T 10000
```

### Interaction

```bash
plwr click '#submit-btn'
plwr fill '#name-input' 'Alice'
plwr press Enter
plwr press Control+c
```

Supported keys for `press`: `a`–`z`, `A`–`Z`, `0`–`9`, `Backspace`, `Tab`,
`Enter`, `Escape`, `Space`, `Delete`, `Insert`, `ArrowUp`, `ArrowDown`,
`ArrowLeft`, `ArrowRight`, `Home`, `End`, `PageUp`, `PageDown`, `F1`–`F12`,
`Control`, `Shift`, `Alt`, `Meta`, and any US keyboard character
(`` !@#$%^&*()_+-=[]{}\\|;:'",./<>?`~ ``). Chords use `+`: `Control+c`,
`Shift+Enter`, `Meta+a`.

### Querying

```bash
plwr text h1                     # print textContent
plwr attr a href                 # print attribute value
plwr count '.list-item'          # print number of matches
plwr exists '.sidebar'           # exit 0 if found, 1 if not
```

### Headers

Set extra HTTP headers sent with every request. Headers persist across
navigations within the same session.

```bash
plwr header CF-Access-Client-Id "$CLIENT_ID"
plwr header CF-Access-Client-Secret "$CLIENT_SECRET"
plwr open "$WORKER_URL"          # headers sent automatically
plwr header --clear              # remove all extra headers
```

### Cookies

```bash
plwr cookie session_id abc123    # set on current page's URL
plwr cookie token xyz --url https://example.com
plwr cookie --list               # list all cookies as JSON
plwr cookie --clear              # remove all cookies
```

### Viewport

```bash
plwr viewport 1280 720          # desktop
plwr viewport 375 667           # iPhone SE
```

### JavaScript

```bash
plwr eval "document.title"
plwr eval "({a: 1, b: [2, 3]})"   # returns pretty-printed JSON
```

### DOM tree

```bash
plwr tree              # full page tree as JSON
plwr tree '.sidebar'   # subtree rooted at selector
```

### Screenshots

```bash
plwr screenshot
plwr screenshot --selector '.chart' --path chart.png
```

### Video

```bash
plwr video-start                 # start recording
# ... do stuff ...
plwr video-stop recording.mp4   # stop and convert to mp4
plwr video-stop recording.webm  # stop, keep as webm (no ffmpeg needed)
```

### Sessions

Run multiple independent browser sessions in parallel:

```bash
plwr -S session-a open https://example.com
plwr -S session-b open https://other.com
plwr -S session-a text h1   # Example Domain
plwr -S session-b text h1   # other.com's h1
plwr -S session-a stop
plwr -S session-b stop
```

## Example: cctr e2e test

Before (with raw `playwright-cli run-code`):

```
===
send a message
===
./pw --session=e2e run-code "async page => {
  const input = await page.waitForSelector('.chat-input', { timeout: 2000 });
  await input.fill('Hello agent');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => {
    const msgs = document.querySelectorAll('[data-role=assistant]');
    return Array.from(msgs).some(m => m.textContent.includes('Hi'));
  }, { timeout: 5000 });
}"
---
```

After (with `plwr`):

```
===
send a message
===
plwr fill '.chat-input' 'Hello agent'
plwr press Enter
---

===
agent responds
===
plwr wait '[data-role=assistant]:has-text("Hi")' -T 10000
---
```
