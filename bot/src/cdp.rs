//! A minimal Chrome DevTools Protocol client: starts Chrome, talks to its page over one
//! blocking websocket, one command at a time. Events are ignored.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

/// A running Chrome and a connection to its page.
pub struct Cdp {
    /// The page's DevTools socket.
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    /// The id of the next command.
    next_id: u64,
    /// The Chrome process.
    chrome: Child,
    /// A throwaway profile directory, removed on drop.
    profile: PathBuf,
}

/// Chrome to use: `$BOT_CHROME`, else the first of the usual names on `PATH`.
fn chrome_binary() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("BOT_CHROME") {
        return Ok(p.into());
    }
    let names = [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ];
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
        .find(|p| p.is_file())
        .ok_or_else(|| anyhow!("no Chrome found; set BOT_CHROME to its path"))
}

impl Cdp {
    /// Starts Chrome with `args` (besides its own) and connects to its first page.
    pub fn launch(args: &[String]) -> Result<Self> {
        let profile = std::env::temp_dir().join(format!("rubyrushers-bot-{}", std::process::id()));
        let mut chrome = Command::new(chrome_binary()?)
            .arg("--remote-debugging-port=0")
            .arg(format!("--user-data-dir={}", profile.display()))
            .args(["--no-first-run", "--no-default-browser-check"])
            .args(args)
            .arg("about:blank")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("start Chrome")?;
        // Chrome announces its DevTools port on stderr.
        let mut lines = BufReader::new(chrome.stderr.take().context("Chrome stderr")?).lines();
        let port = loop {
            let line = lines
                .next()
                .context("Chrome exited before it was ready")??;
            if let Some(rest) = line.strip_prefix("DevTools listening on ws://127.0.0.1:") {
                break rest.split('/').next().unwrap_or_default().parse::<u16>()?;
            }
        };
        // Keep draining stderr so Chrome never blocks on a full pipe.
        std::thread::spawn(move || lines.for_each(drop));

        let list = http_get(port, "/json/list")?;
        let targets: Value = serde_json::from_str(&list)?;
        let page = targets
            .as_array()
            .and_then(|t| t.iter().find(|t| t["type"] == "page"))
            .and_then(|t| t["webSocketDebuggerUrl"].as_str())
            .context("Chrome has no page")?;
        let (socket, _) = tungstenite::connect(page).context("connect to the page")?;
        Ok(Self {
            socket,
            next_id: 1,
            chrome,
            profile,
        })
    }

    /// Sends a command and waits for its result.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({ "id": id, "method": method, "params": params });
        self.socket.send(Message::text(request.to_string()))?;
        loop {
            let Message::Text(text) = self.socket.read()? else {
                continue;
            };
            let reply: Value = serde_json::from_str(&text)?;
            if reply["id"] != id {
                continue; // An event, or an older reply.
            }
            if let Some(err) = reply.get("error") {
                bail!("{method}: {err}");
            }
            return Ok(reply["result"].clone());
        }
    }
}

impl Drop for Cdp {
    fn drop(&mut self) {
        let _ = self.chrome.kill();
        let _ = self.chrome.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

/// The body of `GET path` from Chrome's local HTTP endpoint (which keeps the connection
/// open, so the body is read by its length).
fn http_get(port: u16, path: &str) -> Result<String> {
    let mut stream = BufReader::new(TcpStream::connect(("127.0.0.1", port))?);
    write!(
        stream.get_mut(),
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )?;
    let mut length = None;
    loop {
        let mut line = String::new();
        stream.read_line(&mut line)?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = Some(value.trim().parse::<usize>()?);
        }
    }
    let mut body = vec![0; length.context("no Content-Length from Chrome")?];
    stream.read_exact(&mut body)?;
    Ok(String::from_utf8(body)?)
}
