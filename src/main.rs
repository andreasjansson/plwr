mod client;
mod daemon;
mod protocol;
mod pw_ext;

use crate::protocol::Command;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "plwr",
    about = "Playwright CLI for browser automation using CSS selectors",
    after_long_help = EXAMPLES,
    after_help = "Use --help for examples",
    disable_help_subcommand = true,
    version,
)]
struct Cli {
    /// Session name for parallel browser instances
    #[arg(short = 'S', long, global = true, env = "PLWR_SESSION", default_value = "default")]
    session: String,

    /// Timeout in milliseconds for wait/click/fill operations
    #[arg(short = 'T', long, global = true, env = "PLWR_TIMEOUT", default_value_t = 5000)]
    timeout: u64,

    #[command(subcommand)]
    command: Cmd,
}

const EXAMPLES: &str = "\x1b[1;4mExamples:\x1b[0m

  Navigate and extract text:
    plwr open https://example.com
    plwr text h1                         # \"Example Domain\"
    plwr attr a href                     # \"https://www.iana.org/...\"

  Fill a form and submit:
    plwr fill '#email' 'alice@test.com'
    plwr fill '#password' 'hunter2'
    plwr click 'button[type=submit]'
    plwr wait '.dashboard'               # wait for redirect

  When a selector matches multiple elements:
    plwr click 'li.item >> nth=0'        # first match
    plwr click 'li.item >> nth=2'        # third match
    plwr text ':nth-match(li.item, 2)'   # alternative syntax

  Chain with shell conditionals:
    plwr exists '.cookie-banner' && plwr click '.accept-cookies'

  Set headers for authenticated requests:
    plwr header Authorization 'Bearer tok_xxx'
    plwr open https://api.example.com/dashboard

  Manage cookies:
    plwr cookie session_id abc123
    plwr cookie --list                   # show all as JSON
    plwr cookie --clear

  Run JavaScript:
    plwr eval 'document.title'
    plwr eval '({count: document.querySelectorAll(\"li\").length})'

  Inspect the DOM:
    plwr tree '.sidebar'                 # JSON tree of element
    plwr count '.search-result'          # number of matches

  Screenshot and video:
    plwr screenshot --selector '.chart' --path chart.png
    plwr video-start
    plwr click '#run-demo'
    plwr video-stop demo.mp4

  Adjust viewport for responsive testing:
    plwr viewport 375 667               # iPhone SE
    plwr screenshot --path mobile.png
    plwr viewport 1280 720              # desktop

  Keyboard input:
    plwr press Enter
    plwr press Control+a                 # select all
    plwr press Meta+c                    # copy (macOS)

  Sessions — each session is an independent browser with its own
  cookies, headers, and page state. The browser starts automatically
  on first use and persists until stopped:
    plwr -S admin open https://app.com/admin
    plwr -S user open https://app.com/login
    plwr -S user fill '#email' 'user@test.com'
    plwr -S admin text '.active-users'   # check admin view
    plwr -S admin stop
    plwr -S user stop

  Custom timeout:
    plwr wait '.slow-element' -T 30000   # wait up to 30s

\x1b[1;4mSelector reference:\x1b[0m

  Playwright extends CSS selectors with extra features.

  CSS selectors (all standard CSS works):
    plwr click '#submit-btn'             # by id
    plwr click '.btn.primary'            # compound class
    plwr count 'input[type=email]'       # attribute match
    plwr count 'input:checked'           # pseudo-class
    plwr count 'input:disabled'          # form state
    plwr count 'input:required'          # form validation
    plwr count 'div:empty'              # empty elements
    plwr click 'li:first-child'          # positional
    plwr click 'li:last-child'           # positional
    plwr count '#list > li'              # child combinator
    plwr count 'h1 + p'                  # adjacent sibling
    plwr count 'h1 ~ p'                  # general sibling
    plwr count 'a[href^=/]'              # starts with
    plwr count 'a[href$=.pdf]'           # ends with
    plwr count 'a[href*=example]'        # contains
    plwr count 'a[download]'             # has attribute
    plwr click 'li:not(.done)'           # negation
    plwr click '.card:has(img)'          # has descendant

  Playwright extensions:
    plwr click ':has-text(\"Sign in\")'    # contains text
    plwr click 'text=Sign in'            # text shorthand
    plwr click 'li.item >> nth=0'        # pick nth match
    plwr click ':visible'                # only visible
    plwr text 'tr:has-text(\"Bob\") >> td.status'
                                         # chain with >>

  Some CSS pseudo-classes need the css= prefix to avoid
  Playwright's selector parser misinterpreting them:
    plwr text 'css=span:last-of-type'         # ✓ works
    plwr text 'span:last-of-type'             # ✗ misinterpreted
    plwr text 'css=li:nth-of-type(2)'         # ✓ works
    plwr text 'css=:is(.card, .sidebar)'      # ✓ works
    plwr text 'css=[data-id=\"login\"]'         # ✓ quoted attrs

  The css= prefix is needed for: :last-of-type, :first-of-type,
  :nth-of-type(), :nth-last-child(), :is(), :where(),
  and quoted attribute values [attr=\"val\"].

  These work without the prefix: :nth-child(), :first-child,
  :last-child, :not(), :has(), :empty, :checked, :disabled,
  :enabled, :required, :visible, :has-text().

\x1b[1;4mEnvironment variables:\x1b[0m

  PLAYWRIGHT_HEADED    Show browser window (set to any value)
  PLWR_SESSION         Default session name (default: \"default\")
  PLWR_TIMEOUT         Default timeout in ms (default: 5000)";

#[derive(Subcommand)]
enum Cmd {
    /// Start the browser session
    Start {
        /// Show the browser window
        #[arg(long)]
        headed: bool,
    },
    /// Stop the browser
    Stop,

    /// Navigate to a URL
    Open { url: String },
    /// Reload the current page
    Reload,
    /// Print the current page URL
    Url,

    /// Wait for a CSS selector to appear
    Wait { selector: String },
    /// Wait for a CSS selector to disappear
    WaitNot { selector: String },

    /// Click an element matching a CSS selector
    Click { selector: String },
    /// Fill text into an input matching a CSS selector
    Fill { selector: String, text: String },

    /// Press a keyboard key or chord (e.g. Enter, Escape, Control+c)
    Press { key: String },

    /// Exit 0 if selector exists, exit 1 if not (for && chaining)
    Exists { selector: String },

    /// Print the textContent of the first matching element
    Text { selector: String },
    /// Print the value of an attribute on the first matching element
    Attr { selector: String, name: String },

    /// Print the number of elements matching a CSS selector
    Count { selector: String },

    /// Set a cookie (use --list to show all, --clear to remove all)
    Cookie {
        /// Cookie name (omit for --list or --clear)
        name: Option<String>,
        /// Cookie value
        value: Option<String>,
        /// URL the cookie applies to (defaults to current page URL)
        #[arg(long)]
        url: Option<String>,
        /// List all cookies as JSON
        #[arg(long)]
        list: bool,
        /// Clear all cookies
        #[arg(long)]
        clear: bool,
    },

    /// Set the browser viewport size
    Viewport {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
    },

    /// Set an extra HTTP header sent with every request (use --clear to remove all)
    Header {
        /// Header name (omit to clear all headers)
        name: Option<String>,
        /// Header value
        value: Option<String>,
        /// Clear all extra headers
        #[arg(long)]
        clear: bool,
    },

    /// Evaluate arbitrary JavaScript in page context, print the result
    Eval { js: String },

    /// Take a screenshot (optionally of a specific element)
    Screenshot {
        #[arg(long)]
        selector: Option<String>,
        #[arg(long, default_value = "screenshot.png")]
        path: String,
    },

    /// Dump the DOM tree as JSON (optionally rooted at a selector)
    Tree {
        /// CSS selector to use as root
        selector: Option<String>,
    },

    /// Start video recording
    VideoStart {
        /// Directory to store raw video files
        #[arg(long, default_value = ".plwr-video")]
        dir: String,
    },
    /// Stop video recording and save the file
    VideoStop {
        /// Output file path (.mp4, .webm, .gif, etc.)
        output: String,
    },

    /// Internal: run the browser daemon (not for direct use)
    #[command(hide = true)]
    Daemon,
}

fn socket_path(session: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("plwr");
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!("{}.sock", session))
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let sock = socket_path(&cli.session);

    match cli.command {
        Cmd::Daemon => {
            let headed = std::env::var("PLAYWRIGHT_HEADED").is_ok_and(|v| !v.is_empty());
            match daemon::run(&sock, headed).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    std::fs::remove_file(&sock).ok();
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }

        Cmd::Start { headed } => {
            let headed = headed || std::env::var("PLAYWRIGHT_HEADED").is_ok_and(|v| !v.is_empty());
            match client::start_and_send(&sock, Command::Open { url: "about:blank".into() }, headed).await {
                Ok(resp) => {
                    if resp.ok {
                        println!("Started session '{}'", cli.session);
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("{}", resp.error.unwrap_or_else(|| "Unknown error".into()));
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }

        Cmd::Stop => {
            match client::send_if_running(&sock, Command::Stop).await {
                Ok(Some(_)) => {
                    println!("Stopped session '{}'", cli.session);
                    ExitCode::SUCCESS
                }
                Ok(None) => {
                    println!("No session '{}' running", cli.session);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }

        cmd => {
            let command = match cmd {
                Cmd::Daemon | Cmd::Stop | Cmd::Start { .. } => unreachable!(),
                Cmd::Open { url } => Command::Open { url },
                Cmd::Reload => Command::Reload,
                Cmd::Url => Command::Url,
                Cmd::Wait { selector } => Command::Wait { selector, timeout: cli.timeout },
                Cmd::WaitNot { selector } => Command::WaitNot { selector, timeout: cli.timeout },
                Cmd::Click { selector } => Command::Click { selector, timeout: cli.timeout },
                Cmd::Fill { selector, text } => Command::Fill { selector, text, timeout: cli.timeout },
                Cmd::Press { key } => Command::Press { key },
                Cmd::Exists { selector } => Command::Exists { selector },
                Cmd::Cookie { list: true, .. } => Command::CookieList,
                Cmd::Cookie { clear: true, .. } => Command::CookieClear,
                Cmd::Cookie { name: Some(name), value: Some(value), url, .. } => {
                    let url = url.unwrap_or_default();
                    Command::Cookie { name, value, url }
                }
                Cmd::Cookie { name: Some(name), value: None, .. } => {
                    eprintln!("Usage: plwr cookie <name> <value> [--url <url>], plwr cookie --list, or plwr cookie --clear");
                    eprintln!("Missing value for cookie '{}'", name);
                    return ExitCode::FAILURE;
                }
                Cmd::Cookie { .. } => {
                    eprintln!("Usage: plwr cookie <name> <value> [--url <url>], plwr cookie --list, or plwr cookie --clear");
                    return ExitCode::FAILURE;
                }
                Cmd::Viewport { width, height } => Command::Viewport { width, height },
                Cmd::Header { clear: true, .. } => Command::HeaderClear,
                Cmd::Header { name: Some(name), value: Some(value), .. } => Command::Header { name, value },
                Cmd::Header { name: Some(name), value: None, .. } => {
                    eprintln!("Usage: plwr header <name> <value> or plwr header --clear");
                    eprintln!("Missing value for header '{}'", name);
                    return ExitCode::FAILURE;
                }
                Cmd::Header { name: None, .. } => {
                    eprintln!("Usage: plwr header <name> <value> or plwr header --clear");
                    return ExitCode::FAILURE;
                }
                Cmd::Text { selector } => Command::Text { selector, timeout: cli.timeout },
                Cmd::Attr { selector, name } => Command::Attr { selector, name, timeout: cli.timeout },
                Cmd::Count { selector } => Command::Count { selector },
                Cmd::Eval { js } => Command::Eval { js },
                Cmd::Screenshot { selector, path } => Command::Screenshot { selector, path, timeout: cli.timeout },
                Cmd::Tree { selector } => Command::Tree { selector, timeout: cli.timeout },
                Cmd::VideoStart { dir } => Command::VideoStart { dir },
                Cmd::VideoStop { output } => Command::VideoStop { output },
            };

            match client::send(&sock, command).await {
                Ok(resp) => {
                    if resp.ok {
                        if let Some(value) = resp.value {
                            match value {
                                serde_json::Value::String(s) => println!("{}", s),
                                serde_json::Value::Bool(b) => {
                                    if !b {
                                        return ExitCode::FAILURE;
                                    }
                                }
                                serde_json::Value::Null => {}
                                other => println!("{}", serde_json::to_string_pretty(&other).unwrap()),
                            }
                        }
                        ExitCode::SUCCESS
                    } else {
                        eprintln!("{}", resp.error.unwrap_or_else(|| "Unknown error".into()));
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::FAILURE
                }
            }
        }
    }
}
