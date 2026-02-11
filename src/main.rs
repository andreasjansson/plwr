mod client;
mod daemon;
mod protocol;

use crate::protocol::Command;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "plwr", about = "Clean CLI for Playwright browser automation with CSS selectors")]
struct Cli {
    #[arg(short = 'S', long, global = true, env = "PLWR_SESSION", default_value = "default")]
    session: String,

    #[arg(short = 'T', long, global = true, env = "PLWR_TIMEOUT", default_value_t = 5000)]
    timeout: u64,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
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

        cmd => {
            let command = match cmd {
                Cmd::Daemon => unreachable!(),
                Cmd::Stop => Command::Stop,
                Cmd::Open { url } => Command::Open { url },
                Cmd::Reload => Command::Reload,
                Cmd::Url => Command::Url,
                Cmd::Wait { selector } => Command::Wait { selector, timeout: cli.timeout },
                Cmd::WaitNot { selector } => Command::WaitNot { selector, timeout: cli.timeout },
                Cmd::Click { selector } => Command::Click { selector, timeout: cli.timeout },
                Cmd::Fill { selector, text } => Command::Fill { selector, text, timeout: cli.timeout },
                Cmd::Press { key } => Command::Press { key },
                Cmd::Exists { selector } => Command::Exists { selector },
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
