---
name: plwr
description: >
  Browser automation CLI using CSS selectors, built on Playwright. Use when the
  user needs to navigate websites, interact with web pages, fill forms, take
  screenshots, test web applications, or extract information from web pages.
---

# Browser Automation with plwr

plwr uses CSS selectors (not element refs) and a persistent daemon session.
Every session must be explicitly started and stopped.

## Quick Start

```bash
plwr start
plwr open https://example.com
plwr text h1                    # Example Domain
plwr click 'a'
plwr stop
```

## Core Workflow

1. `plwr start` — launch browser (headless by default)
2. `plwr open URL` — navigate
3. Interact/query using CSS selectors
4. `plwr stop` — shut down

## ⚠️ Key Concepts

- **Always start/stop**: every session needs `plwr start` before and `plwr stop` after.
- **CSS selectors, not refs**: all commands take standard CSS selectors (e.g. `#id`, `.class`, `tag`, `[attr=val]`).
- **Auto-wait**: interaction and query commands auto-wait for elements up to the timeout. You rarely need `plwr wait`.
- **Strict mode**: if a selector matches multiple elements, commands like `text`, `click`, `attr` fail. Use `>> nth=N` to pick one, or `count`/`exists` which handle multiple matches.
- **Single-quote selectors** in shell to avoid bash metacharacter issues (e.g. `$` in `[href$=.pdf]`).

## Commands

### Session Lifecycle

```bash
plwr start                         # headless
plwr start --headed                # visible browser window
plwr start --video recording.mp4   # record video
plwr start --ignore-cert-errors    # ignore TLS certificate errors
plwr stop                          # shut down (saves video if recording)
```

### Navigation

```bash
plwr open 'https://example.com'
plwr reload
plwr url
```

### Waiting

```bash
plwr wait '.my-element'
plwr wait-not '.loading-spinner' -T 10000
plwr wait-any '.success' '.error' '.timeout'
plwr wait-all '.header' '.sidebar' '.content'
```

### Interaction

```bash
plwr click '#submit-btn'
plwr fill '#name-input' 'Alice'
plwr press Enter
plwr press Control+c
plwr dblclick '.editable-cell'
plwr hover '.dropdown-trigger'
plwr focus '#search'
plwr blur '#email'
plwr scroll '.footer'
```

Click/dblclick modifiers:

```bash
plwr click '#item' --shift
plwr click '#item' --alt
plwr click '#item' --meta
plwr click '#item' --control
plwr click '#item' --right           # right-click
plwr click '#item' --middle          # middle-click
```

### Checkboxes, Radios, Selects

```bash
plwr check '#agree-terms'
plwr uncheck '#newsletter'
plwr select '#country' us
plwr select '#country' --label 'Canada'
plwr select '#colors' red green blue     # multi-select
```

### Querying

```bash
plwr text h1                     # textContent
plwr inner-html '.content'       # innerHTML (preserves tags)
plwr attr a href                 # attribute value
plwr input-value '#email'        # value of input/textarea/select
plwr count '.list-item'          # number of matches
plwr exists '.sidebar'           # exit 0 if found, 1 if not
plwr computed-style '.box' display width
```

### Clipboard

```bash
plwr clipboard-copy '#source'
plwr focus '#target'
plwr clipboard-paste
```

### Headers and Cookies

```bash
plwr header Authorization 'Bearer tok123'
plwr header --clear
plwr cookie session_id abc123
plwr cookie token xyz --url https://example.com
plwr cookie --list
plwr cookie --clear
```

### Viewport

```bash
plwr viewport 1280 720
plwr viewport 375 667
```

### File Uploads

```bash
plwr input-files 'input[type=file]' photo.png
plwr input-files '#upload' a.txt b.txt c.txt
```

### Dialogs

Register a handler **before** the action that triggers the dialog:

```bash
plwr next-dialog accept
plwr click '#delete-btn'

plwr next-dialog dismiss
plwr click '#cancel-btn'

plwr next-dialog accept 'Alice'   # prompt with text
plwr click '#rename-btn'
```

### Console Logs

```bash
plwr console                     # all captured messages as JSON
plwr console --clear
```

### Network Requests

Capture all HTTP requests (doc, CSS, JS, images, fonts, fetch, XHR, WebSocket)
with status codes for every resource type.

```bash
plwr network                     # all captured requests as JSON
plwr network --type fetch        # filter by type
plwr network --type css,js,img   # multiple types
plwr network --clear             # clear the buffer
```

Types: `doc`, `css`, `js`, `img`, `font`, `media`, `fetch`, `xhr`, `ws`,
`wasm`, `manifest`, `other`.

Each entry: `{type, url, status, method, size, duration, ts}`.

### JavaScript

```bash
plwr eval 'document.title'
plwr eval '({a: 1, b: [2, 3]})'
plwr eval "(() => {
  const rows = document.querySelectorAll('table tr');
  return Array.from(rows).map(r => r.cells[0]?.textContent);
})()"
```

### DOM Tree

```bash
plwr tree                        # full page
plwr tree '.sidebar'             # subtree
```

### Screenshots and Video

```bash
plwr screenshot
plwr screenshot --selector '.chart' --path chart.png

plwr start --video recording.mp4
# ... interact ...
plwr stop                        # saves recording.mp4
```

### Sessions

```bash
plwr -S session-a start
plwr -S session-b start
plwr -S session-a open https://example.com
plwr -S session-b open https://other.com
plwr -S session-a text h1
plwr -S session-b text h1
plwr -S session-a stop
plwr -S session-b stop
```

### Global Options

| Option | Description |
|--------|-------------|
| `-S`, `--session` | Session name (default: `default`, env: `PLWR_SESSION`) |
| `-T`, `--timeout` | Timeout in ms (default: `5000`, env: `PLWR_TIMEOUT`) |

## Selectors

plwr uses Playwright's selector engine which extends CSS.

### Basics

```bash
plwr click '#submit-btn'                    # by id
plwr click '.btn.primary'                   # compound class
plwr click 'button'                         # by tag
plwr count 'input[type=email]'              # attribute (no quotes needed)
plwr count '[data-testid=login-form]'       # exact attribute match
```

### Combinators

```bash
plwr count '#list > li'                     # child
plwr count 'h1 + p'                         # adjacent sibling
plwr text '.card p'                         # descendant
```

### Playwright Extensions

```bash
plwr click ':has-text("Sign in")'           # contains text
plwr click 'text=Sign in'                   # text shorthand
plwr click 'li.item >> nth=0'              # first match (0-based)
plwr click 'li.item >> nth=-1'             # last match
plwr count 'button:visible'                # only visible
plwr text 'tr:has-text("Bob") >> td.name'  # chain with >>
```

### css= Prefix

Some pseudo-classes need `css=` to bypass Playwright's parser:

```bash
plwr text 'css=.list span:last-of-type'
plwr text 'css=li:nth-of-type(2)'
plwr count 'css=:is(.card, .sidebar)'
plwr text 'css=[data-testid="login-form"] button'
```

Need `css=`: `:last-of-type`, `:first-of-type`, `:nth-of-type()`,
`:nth-last-child()`, `:is()`, `:where()`, quoted `[attr="val"]`.

Work without: `:nth-child()`, `:first-child`, `:last-child`, `:not()`,
`:has()`, `:empty`, `:checked`, `:disabled`, `:visible`, `:has-text()`,
`text=`, `>> nth=N`.

### Strict Mode

```bash
plwr text 'li.item'               # fails if >1 match
plwr text 'li.item >> nth=0'     # first match
plwr count 'li.item'              # count always works
plwr exists 'li.item'             # exists always works
```

## Example: Login Flow

```bash
plwr start
plwr open 'https://app.example.com/login'
plwr fill '#email' 'user@example.com'
plwr fill '#password' 'secret'
plwr click '#login-btn'
plwr wait '.dashboard'
plwr text '.welcome-message'
plwr stop
```

## Example: Scraping a Table

```bash
plwr start
plwr open 'https://example.com/data'
plwr count 'table tbody tr'
plwr text 'table tbody tr >> nth=0'
plwr eval "(() => {
  const rows = document.querySelectorAll('table tbody tr');
  return Array.from(rows).map(r => ({
    name: r.cells[0]?.textContent,
    value: r.cells[1]?.textContent,
  }));
})()"
plwr stop
```

## Example: cctr E2E Tests

plwr is designed to work well with [cctr](https://github.com/andreasjansson/cctr) corpus tests:

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
