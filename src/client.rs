use crate::protocol::{Command, Request, Response};
use anyhow::{bail, Result};
use std::io::BufRead;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const STARTUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn send(socket_path: &Path, command: Command) -> Result<Response> {
    // Try connecting; if it fails, auto-start the daemon and retry.
    let stream = match UnixStream::connect(socket_path).await {
        Ok(s) => s,
        Err(_) => {
            start_daemon(socket_path)?;
            UnixStream::connect(socket_path).await
                .map_err(|e| anyhow::anyhow!("Daemon started but cannot connect: {}", e))?
        }
    };

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

/// Spawn `plwr daemon` as a detached child, wait for the ready signal on
/// its stdout. Env vars (PLAYWRIGHT_HEADED, etc.) are inherited.
fn start_daemon(socket_path: &Path) -> Result<()> {
    // Clean stale socket
    if socket_path.exists() {
        std::fs::remove_file(socket_path).ok();
    }

    let exe = std::env::current_exe()?;

    // Reconstruct the session name from the socket filename
    let session = socket_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default");

    let mut child = StdCommand::new(&exe)
        .args(["--session", session, "daemon"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let reader = std::io::BufReader::new(stdout);

    // Read lines until we see the ready signal, an error signal, or EOF.
    // This blocks the current thread, which is fine — we're waiting for
    // the daemon to be ready before sending it commands.
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;

    for line in reader.lines() {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            bail!("Daemon did not start within {}s", STARTUP_TIMEOUT.as_secs());
        }

        let line = line?;

        if line == "### ready" {
            // Detach: drop our handle so the daemon outlives us
            drop(child);
            return Ok(());
        }

        if let Some(err) = line.strip_prefix("### error ") {
            // Wait for the process to finish so we don't leave zombies
            let _ = child.wait();
            bail!("{}", err);
        }
    }

    // stdout closed without a signal — daemon crashed
    let output = child.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("Daemon exited unexpectedly: {}", stderr.trim());
}
