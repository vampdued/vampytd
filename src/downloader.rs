use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

use crate::config::Config;
use crate::deps::Dependencies;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMeta {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub formats: Option<Vec<Format>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Format {
    pub format_id: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub tbr: Option<f64>,
    pub format_note: Option<String>,
}

impl Format {
    pub fn is_video(&self) -> bool {
        self.vcodec.as_deref().unwrap_or("none") != "none"
    }

    pub fn is_audio_only(&self) -> bool {
        !self.is_video() && self.acodec.as_deref().unwrap_or("none") != "none"
    }

    pub fn size_string(&self) -> String {
        let bytes = self.filesize.or(self.filesize_approx).unwrap_or(0);
        if bytes == 0 {
            return "?".to_string();
        }
        let mb = (bytes as f64) / (1024.0 * 1024.0);
        if mb >= 1024.0 {
            format!("{:.1} GB", mb / 1024.0)
        } else {
            format!("{:.1} MB", mb)
        }
    }

    pub fn res_string(&self) -> String {
        if let Some(h) = self.height {
            format!("{}p", h)
        } else if let Some(ref r) = self.resolution {
            r.clone()
        } else if self.is_audio_only() {
            "audio".to_string()
        } else {
            "?".to_string()
        }
    }
}

pub fn fetch_metadata(url: &str, deps: &Dependencies, config: &Config) -> Result<VideoMeta> {
    let mut cmd = Command::new("yt-dlp");
    cmd.arg("--dump-json").arg("--no-playlist");
    cmd.args(deps.client_args());

    if let Some(cookie_path) = config.get_cookie_for_url(url) {
        cmd.arg("--cookies").arg(cookie_path);
    }

    cmd.arg(url);

    let output = cmd
        .output()
        .context("Failed to invoke yt-dlp to extract video metadata")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("yt-dlp metadata dump failed: {}", err);
    }

    let meta: VideoMeta =
        serde_json::from_slice(&output.stdout).context("Failed to parse yt-dlp JSON output")?;

    Ok(meta)
}

pub fn execute_download(
    url: &str,
    format_spec: Option<&str>,
    mode: &str,
    deps: &Dependencies,
    config: &Config,
) -> Result<()> {
    let mut cmd = Command::new("yt-dlp");
    cmd.args(deps.client_args());

    if let Some(cookie_path) = config.get_cookie_for_url(url) {
        cmd.arg("--cookies").arg(cookie_path);
    }

    cmd.arg("-P").arg(&config.download_dir);
    cmd.arg("-o").arg(&config.template);
    cmd.arg("--embed-chapters");
    cmd.arg("--embed-thumbnail");

    match mode {
        "quick-max" => {
            cmd.arg("-f").arg("bv*+ba/b");
        }
        "quick-1080p" => {
            cmd.arg("-f").arg("bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b");
        }
        "quick-4k" => {
            cmd.arg("-f").arg("bv*[height<=2160]+ba/b[height<=2160]/bv*+ba/b");
        }
        _ => {
            if let Some(f) = format_spec {
                cmd.arg("-f").arg(f);
            }
        }
    }

    cmd.arg(url);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to run yt-dlp downloader process")?;
    if !status.success() {
        anyhow::bail!("yt-dlp download failed with status {}", status);
    }

    Ok(())
}

