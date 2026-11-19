use anyhow::{Context, Result};
use regex::Regex;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

pub fn execute_with_colorizer(mut cmd: Command) -> Result<()> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Force progress output from yt-dlp even when stdout is piped
    cmd.arg("--progress");

    let mut child = cmd.spawn().context("Failed to spawn yt-dlp process")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let out_thread = std::thread::spawn(move || {
        process_stream(stdout, false);
    });

    let err_thread = std::thread::spawn(move || {
        process_stream(stderr, true);
    });

    let _ = out_thread.join();
    let _ = err_thread.join();

    let status = child.wait().context("Failed to wait on yt-dlp child process")?;
    if !status.success() {
        anyhow::bail!("yt-dlp exited with error status: {}", status);
    }

    println!("\n\x1b[32;1m✔ Download complete\x1b[0m\n");
    Ok(())
}

fn process_stream<R: Read>(mut reader: R, is_stderr: bool) {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];

    let progress_re = Regex::new(r"(\d+\.?\d*%)(\s+of\s+[^\s]+)?(\s+at\s+[^\s]+)?(\s+ETA\s+[^\s]+)?").ok();
    let aria2_re = Regex::new(r"\[#([a-f0-9]+)\s+([^\(]+)\((\d+%\))\s+(CN:\d+)\s+(DL:[^\s\]]+)(\s+ETA:[^\s\]]+)?\]").ok();

    loop {
        match reader.read(&mut byte) {
            Ok(0) => {
                if !buf.is_empty() {
                    let line = String::from_utf8_lossy(&buf);
                    print_line(&line, b'\n', is_stderr, &progress_re, &aria2_re);
                }
                break;
            }
            Ok(_) => {
                let b = byte[0];
                if b == b'\r' || b == b'\n' {
                    let line = String::from_utf8_lossy(&buf);
                    print_line(&line, b, is_stderr, &progress_re, &aria2_re);
                    buf.clear();
                } else {
                    buf.push(b);
                }
            }
            Err(_) => break,
        }
    }
}

fn print_line(
    line: &str,
    delimiter: u8,
    _is_stderr: bool,
    progress_re: &Option<Regex>,
    aria2_re: &Option<Regex>,
) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    // Noise filtering: hide internal player API chatter
    if trimmed.contains("samsung player API")
        || trimmed.contains("visionos player API")
        || trimmed.contains("Downloading player ")
        || trimmed.contains("Downloading m3u8 information")
    {
        return;
    }

    // Is it an in-place aria2c live progress bar update?
    if trimmed.starts_with("[#") && (trimmed.contains("CN:") || trimmed.contains("DL:")) {
        let mut colorized = trimmed.to_string();
        if let Some(ref re) = aria2_re {
            if let Some(caps) = re.captures(trimmed) {
                if let Some(gid) = caps.get(1) {
                    let colored_gid = format!("\x1b[35;1m[#aria2:{}]\x1b[0m", gid.as_str());
                    let full_tag = format!("[#{}]", gid.as_str());
                    colorized = colorized.replace(&full_tag, &colored_gid);
                }
                if let Some(pct) = caps.get(3) {
                    let colored_pct = format!("\x1b[36;1m{}\x1b[0m", pct.as_str());
                    colorized = colorized.replace(pct.as_str(), &colored_pct);
                }
                if let Some(cn) = caps.get(4) {
                    let colored_cn = format!("\x1b[33;1m{}\x1b[0m", cn.as_str());
                    colorized = colorized.replace(cn.as_str(), &colored_cn);
                }
                if let Some(dl) = caps.get(5) {
                    let colored_dl = format!("\x1b[92;1m{}\x1b[0m", dl.as_str());
                    colorized = colorized.replace(dl.as_str(), &colored_dl);
                }
                if let Some(eta) = caps.get(6) {
                    let colored_eta = format!("\x1b[33;1m{}\x1b[0m", eta.as_str().trim());
                    colorized = colorized.replace(eta.as_str().trim(), &colored_eta);
                }
            }
        }

        if delimiter == b'\r' {
            print!("\r\x1b[2K{}", colorized);
            let _ = io::stdout().flush();
        } else {
            println!("\r\x1b[2K{}", colorized);
        }
        return;
    }

    // Is it an in-place live progress bar update from yt-dlp native?
    if trimmed.starts_with("[download]") && trimmed.contains('%') {
        let mut colorized = trimmed.to_string();
        colorized = colorized.replacen("[download]", "\x1b[32;1m[download]\x1b[0m", 1);

        if let Some(ref re) = progress_re {
            if let Some(caps) = re.captures(trimmed) {
                if let Some(pct) = caps.get(1) {
                    let colored_pct = format!("\x1b[36;1m{}\x1b[0m", pct.as_str());
                    colorized = colorized.replace(pct.as_str(), &colored_pct);
                }
                if let Some(speed) = caps.get(3) {
                    let colored_speed = format!("\x1b[92;1m{}\x1b[0m", speed.as_str());
                    colorized = colorized.replace(speed.as_str(), &colored_speed);
                }
                if let Some(eta) = caps.get(4) {
                    let colored_eta = format!("\x1b[33;1m{}\x1b[0m", eta.as_str());
                    colorized = colorized.replace(eta.as_str(), &colored_eta);
                }
            }
        }

        if delimiter == b'\r' {
            print!("\r\x1b[2K{}", colorized);
            let _ = io::stdout().flush();
        } else {
            println!("\r\x1b[2K{}", colorized);
        }
        return;
    }

    // Tag and milestone colorization
    let mut out = trimmed.to_string();

    if out.starts_with("[youtube]") {
        out = out.replacen("[youtube]", "\x1b[31;1m[youtube]\x1b[0m", 1);
    } else if out.starts_with("[info]") {
        out = out.replacen("[info]", "\x1b[36;1m[info]\x1b[0m", 1);
    } else if out.starts_with("[download]") {
        out = out.replacen("[download]", "\x1b[32;1m[download]\x1b[0m", 1);
    } else if out.starts_with("[ThumbnailsConvertor]") {
        out = out.replacen("[ThumbnailsConvertor]", "\x1b[35;1m[Thumbnail]\x1b[0m", 1);
    } else if out.starts_with("[Merger]") {
        out = out.replacen("[Merger]", "\x1b[35;1m[Merger]\x1b[0m", 1);
    } else if out.starts_with("[Metadata]") {
        out = out.replacen("[Metadata]", "\x1b[35;1m[Metadata]\x1b[0m", 1);
    } else if out.starts_with("WARNING:") {
        out = out.replacen("WARNING:", "\x1b[33;1mWARNING:\x1b[0m", 1);
    } else if out.starts_with("ERROR:") {
        out = out.replacen("ERROR:", "\x1b[91;1mERROR:\x1b[0m", 1);
    } else if out.starts_with("Deleting original file") {
        out = format!("\x1b[2m{}\x1b[0m", out);
    }

    // Highlight Destination path
    if out.contains("Destination: ") {
        out = out.replace("Destination: ", "Destination: \x1b[34;1m");
        out.push_str("\x1b[0m");
    }

    if delimiter == b'\r' {
        print!("\r\x1b[2K{}", out);
        let _ = io::stdout().flush();
    } else {
        println!("{}", out);
    }
}
