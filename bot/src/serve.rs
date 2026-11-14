//! A tiny static file server, so the bot can host the game's web build itself.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// Serves the files under `root` on a free local port in a background thread. Returns
/// the base URL.
pub fn serve(root: impl Into<PathBuf>) -> Result<String> {
    let root = root.into().canonicalize()?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/", listener.local_addr()?);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let root = root.clone();
            std::thread::spawn(move || {
                let _ = respond(&root, stream);
            });
        }
    });
    Ok(url)
}

/// Answers one GET request with a file under `root` (or 404).
fn respond(root: &Path, mut stream: std::net::TcpStream) -> std::io::Result<()> {
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let rel = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let file = root.join(rel);
    // Never leave the served directory.
    let body = match file.canonicalize() {
        Ok(f) if f.starts_with(root) => std::fs::read(f).ok(),
        _ => None,
    };
    let Some(body) = body else {
        return stream.write_all(
            b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        );
    };
    let mime = match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("png") => "image/png",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    };
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: {mime}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}
