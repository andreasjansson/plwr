# plwr

`plwr` (prounounced [_PLUR_](https://en.wikipedia.org/wiki/PLUR)) is a Playwright CLI for browser automation using CSS selectors. Built on [playwright-rs](https://github.com/padamson/playwright-rust).

```bash
plwr start                     # Started session 'default'
plwr open https://example.com
plwr text h1                   # Example Domain
plwr attr a href               # https://iana.org/domains/example
plwr stop                      # Stopped session 'default'
```

## Install

### Homebrew

```bash
brew install andreasjansson/tap/plwr
```

### Crates.io

```bash
cargo install plwr
```

### Dependencies

Requires Playwright:

```bash
npm install -g playwright && npx playwright install chromium
```

For video conversion to non-webm formats, install [ffmpeg](https://ffmpeg.org/).

## AI Agent Skill

plwr includes a skill file that teaches AI coding agents (like Claude Code) how to automate browsers with plwr. The skill covers the full command set, selector syntax, and common patterns.

Copy the skill to your skills directory:

```bash
# Claude Code - personal (available across all your projects):
cp -r skills/plwr ~/.claude/skills/

# Claude Code - project-specific (commit to version control):
cp -r skills/plwr .claude/skills/

# OpenCode:
cp -r skills/plwr ~/.config/opencode/skills/
```

## Usage

Start a browser session, navigate, interact, and stop:

```bash
plwr start                      # start headless browser
plwr open https://example.com   # navigate to URL
plwr text h1                    # Example Domain
plwr stop                       # shut down browser
```

### Environment variables

| Variable | Effect |
|----------|--------|
| `PLAYWRIGHT_HEADED` | Set to any value to run the browser with a visible window |
| `PLWR_SESSION` | Default session name (default: `default`) |
| `PLWR_TIMEOUT` | Default timeout in ms (default: `5000`) |
| `PLWR_IGNORE_CERT_ERRORS` | Set to any value to ignore TLS/SSL certificate errors |
| `PLWR_CDP` | CDP connection target: channel name, user-data-dir path, or `ws://` URL |

All commands take `-S`/`--session` and `-T`/`--timeout` as global options,
which override the environment variables.

### Starting and stopping

`start` launches the browser. All other commands require a running session.
Use `--headed` (or the `PLAYWRIGHT_HEADED` env var) to show the browser window.

```bash
plwr start                             # headless
plwr start --headed                     # visible browser window
plwr start --video recording.mp4       # record video of session
plwr start --ignore-cert-errors        # ignore TLS certificate errors
plwr stop                              # shut down (saves video if recording)
```

Commands that interact with page content (`text`, `click`, `wait`, `eval`,
etc.) require a page to be open first via `plwr open`. Commands that configure
the session (`header`, `viewport`) work before any page is opened.

### Remote debugging (CDP)

Connect to your running Chrome instead of launching a new browser. New tabs
share your existing cookies and login state.

1. In Chrome, go to `chrome://inspect/#remote-debugging` and enable it
2. Connect plwr:

```bash
plwr start --cdp                     # stable Chrome (default)
plwr start --cdp beta                # Chrome Beta
plwr start --cdp ~/my-profile        # custom --user-data-dir path
plwr start --cdp ws://host:9222/...  # direct WebSocket URL
plwr open https://example.com        # opens in a new tab in your Chrome
plwr text h1
plwr stop                            # closes the tab, Chrome keeps running
```

Chrome shows a permission dialog on each connection — click **Allow**.
`--cdp` is mutually exclusive with `--headed` and `--video`.

**Example: post to X/Twitter** (using your logged-in session):

```bash
plwr start --cdp
plwr open https://x.com/compose/post
plwr click '[data-testid="tweetTextarea_0"] >> nth=0'
plwr type 'hello world'
plwr click '[data-testid="tweetButton"]'
plwr stop
```

### Navigation

`open` navigates the current page within the existing browser context. Headers,
cookies, and other state are preserved across navigations. There is no separate
`goto` command — `open` always reuses the same context. If you need a fresh
context, use `plwr stop` followed by `plwr start` and `plwr open`.

```bash
plwr open "https://example.com"
plwr reload
plwr url
```

### Waiting

```bash
plwr wait .my-element
plwr wait-not .loading-spinner -T 10000
plwr wait-any '.success' '.error' '.timeout'   # prints first match
plwr wait-all '.header' '.sidebar' '.content'
```

### Interaction

All interaction commands (`click`, `fill`, `hover`, `check`, etc.) auto-wait
for the element to appear and become actionable before performing the action,
up to the timeout (`-T`, default 5000ms). You rarely need an explicit
`plwr wait` before an interaction — just use the interaction directly:

```bash
plwr click '#submit-btn'                 # waits for button, then clicks
plwr fill '#name-input' 'Alice' -T 10000 # waits up to 10s, then fills
```

```bash
plwr click '#submit-btn'
plwr fill '#name-input' 'Alice'
plwr press Enter
plwr press Control+c
plwr dblclick '.editable-cell'   # double-click
plwr hover '.dropdown-trigger'   # hover (for tooltips, menus)
plwr focus '#search'             # focus an element
plwr blur '#email'               # unfocus an element
plwr scroll '.footer'            # scroll element into view
```

`click` and `dblclick` support modifier keys and mouse button flags:

```bash
plwr click '#item' --shift           # shift-click (e.g. range select)
plwr click '#item' --alt             # alt-click
plwr click '#item' --meta            # meta-click (Cmd on macOS)
plwr click '#item' --control         # control-click (--ctrl also works)
plwr click '#item' --alt --shift     # multiple modifiers
plwr click '#item' --right           # right-click (context menu)
plwr click '#item' --middle          # middle-click
plwr dblclick '#item' --shift        # shift-double-click
```

Supported keys for `press`: `a`–`z`, `A`–`Z`, `0`–`9`, `Backspace`, `Tab`,
`Enter`, `Escape`, `Space`, `Delete`, `Insert`, `ArrowUp`, `ArrowDown`,
`ArrowLeft`, `ArrowRight`, `Home`, `End`, `PageUp`, `PageDown`, `F1`–`F12`,
`Control`, `Shift`, `Alt`, `Meta`, and any US keyboard character
(`` !@#$%^&*()_+-=[]{}\\|;:'",./<>?`~ ``). Chords use `+`: `Control+c`,
`Shift+Enter`, `Meta+a`.

### Clipboard

Copy content from an element to the browser clipboard and paste it at the
currently focused element. Works with text and images (`<img>`, `<canvas>`).

```bash
plwr clipboard-copy '#source'        # copy text or image to clipboard
plwr focus '#target'
plwr clipboard-paste                 # paste at focused element
```

### Checkboxes and radios

```bash
plwr check '#agree-terms'        # check a checkbox or radio
plwr uncheck '#newsletter'       # uncheck a checkbox
```

### Select dropdowns

```bash
plwr select '#country' us               # select by value
plwr select '#country' --label 'Canada' # select by visible text
plwr select '#colors' red green blue    # multi-select
```

### Querying

Like interaction commands, `text`, `attr`, `inner-html`, and `input-value`
auto-wait for the element to appear before reading its value.

```bash
plwr text h1                     # print textContent
plwr inner-html '.content'       # print innerHTML (preserves tags)
plwr attr a href                 # print attribute value
plwr input-value '#email'        # print value of input/textarea/select
plwr computed-style '.box' display width  # print computed CSS properties
plwr count '.list-item'          # print number of matches
plwr exists '.sidebar'           # exit 0 if found, 1 if not
```

### Headers

Set extra HTTP headers sent with every request. Headers persist across
navigations within the same session. Can be set before or after `open`.

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

### File uploads

```bash
plwr input-files 'input[type=file]' photo.png
plwr input-files '#upload' a.txt b.txt c.txt   # multiple files
plwr input-files '#upload'                      # clear selection
```

### Dialogs (alert, confirm, prompt)

Handle native browser dialogs. The `next-dialog` command registers a one-shot
handler for the **next** dialog — call it **before** the action that triggers
the dialog, because dialogs block page execution until handled.

```bash
plwr next-dialog accept                # click OK on the next alert or confirm
plwr next-dialog dismiss               # click Cancel on the next confirm or prompt
plwr next-dialog accept 'Alice'        # type 'Alice' into a prompt, then click OK
```

Typical flow:

```bash
plwr next-dialog accept           # arm the handler
plwr click '#delete-btn'          # triggers confirm() → auto-accepted
plwr text '#result'               # page updated after dialog closed
```

For `prompt()` dialogs, the text argument to `accept` is entered into the
prompt's input field. If omitted, the prompt is accepted with an empty string.
`dismiss` clicks Cancel, returning `null` to the page.

### Console logs

Capture browser console output (log, warn, error, info, debug). Messages
are automatically captured from page load onward, including messages logged
before your code runs.

```bash
plwr console                     # print all captured messages as JSON
plwr console --clear             # clear the log buffer
```

Each entry includes `level`, `ts` (timestamp in ms), and `args` (array of
stringified arguments).

### Network requests

Capture all network requests made by the page. Requests are automatically
captured from page load onward, including document, CSS, JS, images, fonts,
fetch, XHR, and WebSocket connections. Status codes are available for all
resource types.

```bash
plwr network                     # print all captured requests as JSON
plwr network --type fetch        # filter by type
plwr network --type css,js,img   # multiple types, comma-separated
plwr network --url '\.json$'     # filter by URL (regex)
plwr network --type fetch --url '/api/'  # combine type and URL filters
plwr network --type ws --include-ws-messages  # include WS send/recv log
plwr network --clear             # clear the buffer
```

Available types: `doc`, `css`, `js`, `img`, `font`, `media`, `fetch`, `xhr`,
`ws`, `wasm`, `manifest`, `other`.

Each entry includes `type`, `url`, `status` (HTTP status code), `method`
(for fetch/XHR/doc), `size` (transfer size in bytes), `duration` (ms), and
`ts` (timestamp in ms).

WebSocket entries include `type`, `url`, `status` (101 on success), `duration`,
and `ts`. With `--include-ws-messages`, a `messages` array is added containing
each frame's `dir` (`send`/`recv`), `data`, and `ts`.

### Computed styles

```bash
plwr computed-style h1                          # all computed styles as JSON
plwr computed-style '.box' display width color  # specific properties
```

### JavaScript

Simple expressions are evaluated directly:

```bash
plwr eval "document.title"
plwr eval "({a: 1, b: [2, 3]})"   # returns pretty-printed JSON
```

For multi-statement logic, use an IIFE (immediately invoked function expression):

```bash
plwr eval "(() => {
  const rows = document.querySelectorAll('table tr');
  return Array.from(rows).map(r => r.cells[0]?.textContent);
})()"
```

Walk the DOM, gather computed styles, inspect layout:

```bash
plwr eval "(() => {
  const el = document.querySelector('.content');
  let node = el;
  const chain = [];
  while (node && chain.length < 6) {
    const cs = getComputedStyle(node);
    chain.push({
      tag: node.tagName,
      class: node.className,
      display: cs.display,
      width: cs.width,
    });
    node = node.parentElement;
  }
  return chain;
})()"
```

Objects and arrays are returned as pretty-printed JSON. Primitives (strings,
numbers, booleans) are printed as plain text.

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

Record a session by passing `--video` to `start`. The video is saved when
`stop` is called. Non-webm formats (e.g. `.mp4`) require [ffmpeg](https://ffmpeg.org/).

```bash
plwr start --video recording.mp4   # start with video recording
plwr open https://example.com
# ... do stuff ...
plwr stop                          # saves recording.mp4
```

### Sessions

Run multiple independent browser sessions in parallel:

```bash
plwr -S session-a start
plwr -S session-b start
plwr -S session-a open https://example.com
plwr -S session-b open https://other.com
plwr -S session-a text h1   # Example Domain
plwr -S session-b text h1   # other.com's h1
plwr -S session-a stop
plwr -S session-b stop
```

## Selectors

Playwright uses its own selector engine that extends CSS. Most standard CSS
selectors work directly, but some advanced pseudo-classes need a `css=` prefix
to bypass Playwright's parser.

### Basics

```bash
plwr click '#submit-btn'                   # by id
plwr click '.btn.primary'                  # compound class
plwr click 'button'                        # by tag
plwr count 'input[type=email]'             # attribute match
plwr count 'input[type=text]'              # no quotes needed
```

### Combinators

```bash
plwr count '#list > li'                    # child
plwr count 'h1 + p'                        # adjacent sibling
plwr count 'h1 ~ p'                        # general sibling
plwr text '.card p'                        # descendant
```

### Attribute selectors

Unquoted attribute values work directly. For quoted values, use the `css=`
prefix (see [css= prefix](#css-prefix) below).

```bash
plwr count 'a[data-external]'             # has attribute
plwr count 'a[href^=/]'                   # starts with
plwr count 'a[href$=.pdf]'                # ends with
plwr count 'a[href*=example]'             # contains
plwr count '[data-testid=login-form]'      # exact match (no quotes)
```

### Pseudo-classes that work without prefix

```bash
plwr click 'li:first-child'
plwr click 'li:last-child'
plwr text '#list li:nth-child(2)'          # second item
plwr count '#list li:nth-child(odd)'       # 1st, 3rd, ...
plwr count 'li:not(.done)'
plwr count '.card:has(img)'
plwr count 'div:empty'
plwr count 'input:checked'
plwr count 'input:disabled'
plwr count 'input:enabled'
plwr count 'input:required'
```

### Playwright extensions

These are Playwright-specific and don't exist in standard CSS:

```bash
plwr click ':has-text("Sign in")'          # contains text
plwr click 'text=Sign in'                  # text shorthand
plwr click 'li.item >> nth=0'             # first match (0-based)
plwr click 'li.item >> nth=-1'            # last match
plwr text ':nth-match(li.item, 2)'         # alternative to nth=
plwr count 'button:visible'               # only visible elements
plwr text 'tr:has-text("Bob") >> td.name'  # chain with >>
```

The `>>` operator chains selectors — each segment is scoped to the previous
match. You can mix CSS and Playwright engines:

```bash
plwr text '#data-table >> tr:has-text("Alice") >> td.status'
```

### css= prefix

Playwright's selector parser auto-detects whether a string is CSS, XPath, or a
Playwright selector. Some valid CSS pseudo-classes confuse the auto-detection
because Playwright tries to interpret parenthesized arguments or quoted strings
as its own syntax. Prefixing with `css=` forces native CSS evaluation.

**Need `css=` prefix:**

| Selector | Example |
|----------|---------|
| `:last-of-type` | `css=.list span:last-of-type` |
| `:first-of-type` | `css=.list p:first-of-type` |
| `:nth-of-type()` | `css=span:nth-of-type(2)` |
| `:nth-last-child()` | `css=li:nth-last-child(1)` |
| `:is()` | `css=:is(.card, .sidebar)` |
| `:where()` | `css=:where(.card, .sidebar) > p` |
| Quoted `[attr="val"]` | `css=[data-testid="login-form"]` |

```bash
plwr text 'css=.mixed span:last-of-type'
plwr text 'css=li:nth-of-type(2)'
plwr count 'css=:is(.card, .sidebar)'
plwr text 'css=[data-testid="login-form"] button'
```

**Work without prefix** (Playwright recognizes these natively):

`:nth-child()`, `:first-child`, `:last-child`, `:not()`, `:has()`, `:empty`,
`:checked`, `:disabled`, `:enabled`, `:required`, `:visible`, `:has-text()`,
`text=`, `>> nth=N`.

### Strict mode

Playwright locators are strict by default — if a selector matches multiple
elements, commands like `text`, `click`, and `attr` will fail. Use `>> nth=N`
or `:nth-match()` to pick one:

```bash
plwr text 'li.item'                        # fails if >1 match
plwr text 'li.item >> nth=0'              # first match
plwr text ':nth-match(li.item, 2)'         # second match (1-based)
plwr count 'li.item'                       # count always works
plwr exists 'li.item'                      # exists always works
```

### Shell quoting

Watch out for shell metacharacters in selectors. The `$` in `$=` will be
interpreted by bash if not single-quoted:

```bash
plwr count "a[href$=.pdf]"                # ✗ bash eats the $
plwr count 'a[href$=.pdf]'                # ✓ single quotes
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
