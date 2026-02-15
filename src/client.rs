use crate::protocol::{Command, Request, Response};
use anyhow::{bail, Result};
use std::io::BufRead;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn send_if_running(socket_path: &Path, command: Command) -> Result<Option<Response>> {
    let stream = match UnixStream::connect(socket_path).await {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    send_on_stream(stream, command).await.map(Some)
}

pub async fn send(socket_path: &Path, command: Command) -> Result<Response> {
    let stream = UnixStream::connect(socket_path)
        .await
        .map_err(|_| anyhow::anyhow!("No session running. Use 'plwr start' first."))?;
    send_on_stream(stream, command).await
}

pub async fn ensure_started(socket_path: &Path, headed: bool, video: Option<&str>) -> Result<()> {
    if socket_path.exists() {
        if UnixStream::connect(socket_path).await.is_ok() {
            return Ok(());
        }
        std::fs::remove_file(socket_path).ok();
    }
    start_daemon(socket_path, headed, video)
}

async fn send_on_stream(stream: UnixStream, command: Command) -> Result<Response> {
    let (reader, mut writer) = stream.into_split();

    let req = Request { command };
    let mut buf = serde_json::to_vec(&req)?;
    buf.push(b'\n');
    writer.write_all(&buf).await?;

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let resp: Response = serde_json::from_str(&line)?;
    Ok(resp)
}

fn start_daemon(socket_path: &Path, headed: bool, video: Option<&str>) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path).ok();
    }

    let exe = std::env::current_exe()?;

    let session = socket_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default");

    let mut cmd = StdCommand::new(&exe);
    cmd.args(["--session", session, "daemon"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // Safety: setsid() is async-signal-safe and has no preconditions
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    if headed {
        cmd.env("PLAYWRIGHT_HEADED", "1");
    }
    if let Some(path) = video {
        cmd.env("PLWR_VIDEO", path);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let reader = std::io::BufReader::new(stdout);

    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;

    for line in reader.lines() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            bail!("Daemon did not start within {}s", STARTUP_TIMEOUT.as_secs());
        }

        let line = line?;

        if line == "### ready" {
            drop(child);
            return Ok(());
        }

        if let Some(err) = line.strip_prefix("### error ") {
            let _ = child.wait();
            bail!("{}", err);
        }
    }

    let _ = child.wait();
    bail!("Daemon exited unexpectedly");
}
