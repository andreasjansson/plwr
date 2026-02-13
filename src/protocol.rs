use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub command: Command,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Open {
        url: String,
    },
    Reload,
    Url,
    Wait {
        selector: String,
        timeout: u64,
    },
    WaitNot {
        selector: String,
        timeout: u64,
    },
    Click {
        selector: String,
        timeout: u64,
    },
    Fill {
        selector: String,
        text: String,
        timeout: u64,
    },
    Press {
        key: String,
    },
    Exists {
        selector: String,
    },
    Text {
        selector: String,
        timeout: u64,
    },
    Attr {
        selector: String,
        name: String,
        timeout: u64,
    },
    Count {
        selector: String,
    },
    Eval {
        js: String,
    },
    Screenshot {
        selector: Option<String>,
        path: String,
        timeout: u64,
    },
    Tree {
        selector: Option<String>,
        timeout: u64,
    },
    Header {
        name: String,
        value: String,
    },
    HeaderClear,
    Cookie {
        name: String,
        value: String,
        url: String,
    },
    CookieList,
    CookieClear,
    Viewport {
        width: u32,
        height: u32,
    },
    InputFiles {
        selector: String,
        paths: Vec<String>,
        timeout: u64,
    },
    Select {
        selector: String,
        values: Vec<String>,
        by_label: bool,
        timeout: u64,
    },
    Hover {
        selector: String,
        timeout: u64,
    },
    Check {
        selector: String,
        timeout: u64,
    },
    Uncheck {
        selector: String,
        timeout: u64,
    },
    Dblclick {
        selector: String,
        timeout: u64,
    },
    Focus {
        selector: String,
        timeout: u64,
    },
    Blur {
        selector: String,
        timeout: u64,
    },
    InnerHtml {
        selector: String,
        timeout: u64,
    },
    InputValue {
        selector: String,
        timeout: u64,
    },
    ScrollIntoView {
        selector: String,
        timeout: u64,
    },
    VideoStart {
        dir: String,
    },
    VideoStop {
        output: String,
    },
    Stop,
}

impl Command {
    pub fn requires_page(&self) -> bool {
        !matches!(
            self,
            Command::Open { .. }
                | Command::Stop
                | Command::Header { .. }
                | Command::HeaderClear
                | Command::Viewport { .. }
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok_empty() -> Self {
        Self {
            ok: true,
            value: None,
            error: None,
        }
    }

    pub fn ok_value(value: serde_json::Value) -> Self {
        Self {
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn err(msg: String) -> Self {
        Self {
            ok: false,
            value: None,
            error: Some(msg),
        }
    }
}
