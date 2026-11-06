use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::process::Command;

use crate::deps::Dependencies;

const MAX_BODY_SIZE: u32 = 64 * 1024; // 64 KB limit

#[derive(Debug, Deserialize)]
pub struct BridgeRequest {
    pub action: Option<String>,
    pub url: Option<String>,
    pub mode: Option<String>,
    pub codec: Option<String>,
    pub cookies: Option<bool>,
    pub playlist: Option<bool>,
    pub items: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticResponse {
    pub ok: bool,
    pub platform: String,
    pub download_dir: String,
    pub downloader: bool,
    pub yt_dlp: bool,
    pub ffmpeg: bool,
    pub js_runtime: Option<String>,
    pub aria2: bool,
    pub fzf: bool, // Always reported true because Nucleo/Ratatui is embedded!
}

pub fn run_bridge() -> Result<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let req = match read_message(&mut stdin)? {
        Some(msg) => msg,
        None => return Ok(()),
    };

    if req.action.as_deref() == Some("ping") {
        send_response(&mut stdout, &serde_json::json!({ "ok": true }))?;
        return Ok(());
    }

    if req.action.as_deref() == Some("diagnostics") {
        let deps = Dependencies::check();
        let download_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.download_dir().map(|d| d.join("VampYTD")))
            .unwrap_or_else(|| std::path::PathBuf::from("Downloads").join("VampYTD"));

        let diag = DiagnosticResponse {
            ok: true,
            platform: std::env::consts::OS.to_string(),
            download_dir: download_dir.to_string_lossy().to_string(),
            downloader: true,
            yt_dlp: deps.ytdlp,
            ffmpeg: deps.ffmpeg,
            js_runtime: deps.js_runtime,
            aria2: deps.aria2,
            fzf: true, // Nucleo/Ratatui TUI is built-in!
        };
        send_response(&mut stdout, &diag)?;
        return Ok(());
    }

    if req.action.as_deref() == Some("update") {
        launch_terminal_updater()?;
        send_response(&mut stdout, &serde_json::json!({ "ok": true }))?;
        return Ok(());
    }

    // Download action
    if req.url.is_some() {
        launch_terminal_downloader(req)?;
        send_response(&mut stdout, &serde_json::json!({ "ok": true }))?;
    } else {
        send_response(
            &mut stdout,
            &serde_json::json!({ "ok": false, "error": "Missing URL" }),
        )?;
    }

    Ok(())
}

fn read_message<R: Read>(reader: &mut R) -> Result<Option<BridgeRequest>> {
    let mut len_bytes = [0u8; 4];
    match reader.read_exact(&mut len_bytes) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e).context("Failed reading length header from browser stdin"),
    }

    let len = u32::from_ne_bytes(len_bytes);
    if len > MAX_BODY_SIZE {
        anyhow::bail!("Request length {} exceeds max 64KB", len);
    }

    let mut body = vec![0u8; len as usize];
    reader
        .read_exact(&mut body)
        .context("Failed reading body bytes from browser stdin")?;

    let req: BridgeRequest =
        serde_json::from_slice(&body).context("Failed deserializing JSON from browser")?;
    Ok(Some(req))
}

fn send_response<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    let len = bytes.len() as u32;
    writer.write_all(&len.to_ne_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

fn launch_terminal_downloader(req: BridgeRequest) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let mut args = vec![];

    if let Some(url) = req.url {
        args.push(url);
    }
    if let Some(mode) = req.mode {
        args.push("--mode".to_string());
        args.push(mode);
    }
    if let Some(codec) = req.codec {
        args.push("--codec".to_string());
        args.push(codec);
    }
    if let Some(cookies) = req.cookies {
        if cookies {
            args.push("--cookies".to_string());
        } else {
            args.push("--no-cookies".to_string());
        }
    }
    if req.playlist.unwrap_or(false) {
        args.push("--playlist".to_string());
    }
    if let Some(items) = req.items {
        if !items.trim().is_empty() {
            args.push("--items".to_string());
            args.push(items.trim().to_string());
        }
    }

    spawn_terminal_process(&current_exe, &args, "VampYTD Downloader")
}

fn launch_terminal_updater() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let args = vec!["update".to_string()];
    spawn_terminal_process(&current_exe, &args, "VampYTD Updater")
}

fn spawn_terminal_process(exe: &std::path::Path, args: &[String], title: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Try Windows Terminal (wt.exe) first
        let wt_found = which::which("wt").is_ok();
        if wt_found {
            let mut cmd = Command::new("wt.exe");
            cmd.arg("new-tab");
            cmd.arg("--title").arg(title);
            cmd.arg(exe);
            for a in args {
                cmd.arg(a);
            }
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }

        // Fallback to start cmd.exe
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/c", "start", title, exe.to_str().unwrap()]);
        for a in args {
            cmd.arg(a);
        }
        cmd.spawn()?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(exe).args(args).spawn()?;
    }

    Ok(())
}
