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

pub struct PlaylistEntry {
    pub id: Option<String>,

    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PlaylistMeta {
    pub id: Option<String>,

    pub title: Option<String>,

    pub uploader: Option<String>,

    pub playlist_count: Option<usize>,

    pub entries: Option<Vec<PlaylistEntry>>,
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

    pub vbr: Option<f64>,

    pub abr: Option<f64>,

    pub format_note: Option<String>,

    pub dynamic_range: Option<String>,

    pub language: Option<String>,
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

    pub fn bitrate_clean(&self) -> String {
        let rate_kbps = if self.is_audio_only() {
            self.abr.or(self.tbr)
        } else {
            self.vbr.or(self.tbr)
        };

        if let Some(kbps) = rate_kbps {
            let rounded = kbps.round() as u64;

            if rounded > 0 {
                format!("{}k", rounded)
            } else {
                "-".to_string()
            }
        } else {
            "-".to_string()
        }
    }

    pub fn is_storyboard(&self) -> bool {
        if self.format_id.starts_with("sb") || self.ext == "mhtml" {
            return true;
        }

        let v = self.vcodec.as_deref().unwrap_or("none");

        let a = self.acodec.as_deref().unwrap_or("none");

        if v == "none" && a == "none" {
            return true;
        }

        if let Some(ref note) = self.format_note {
            let lower = note.to_lowercase();

            if lower.contains("storyboard") {
                return true;
            }
        }

        false
    }

    pub fn is_muxed(&self) -> bool {
        self.is_video() && self.acodec.as_deref().unwrap_or("none") != "none"
    }

    pub fn stream_type_badge(&self) -> &'static str {
        if self.is_muxed() {
            "MUX"
        } else if self.is_video() {
            "VID"
        } else {
            "AUD"
        }
    }

    pub fn vcodec_clean(&self) -> &'static str {
        let v = self.vcodec.as_deref().unwrap_or("none").to_lowercase();

        if v == "none" {
            "-"
        } else if v.starts_with("av01") {
            "AV1"
        } else if v.starts_with("vp9") || v.starts_with("vp09") {
            "VP9"
        } else if v.starts_with("hvc1")
            || v.starts_with("hev1")
            || v.starts_with("hevc")
            || v.starts_with("h265")
        {
            "HEVC"
        } else if v.starts_with("avc1") || v.starts_with("h264") {
            "H.264"
        } else {
            "Other"
        }
    }

    pub fn acodec_clean(&self) -> &'static str {
        let a = self.acodec.as_deref().unwrap_or("none").to_lowercase();

        if a == "none" {
            "-"
        } else if a.contains("opus") {
            "Opus"
        } else if a.contains("mp4a") || a.contains("aac") {
            "AAC"
        } else if a.contains("mp3") {
            "MP3"
        } else if a.contains("vorbis") {
            "Vorbis"
        } else {
            "Other"
        }
    }

    pub fn fps_clean(&self) -> String {
        if let Some(f) = self.fps {
            let rounded = f.round();

            if rounded > 0.0 {
                return format!("{}", rounded as u32);
            }
        }

        "-".to_string()
    }

    pub fn is_hdr(&self) -> bool {
        if let Some(ref dr) = self.dynamic_range {
            if dr.to_uppercase().contains("HDR") {
                return true;
            }
        }

        if let Some(ref note) = self.format_note {
            if note.to_uppercase().contains("HDR") {
                return true;
            }
        }

        false
    }

    pub fn res_clean(&self) -> String {
        let base = if let Some(h) = self.height {
            match h {
                4320.. => format!("8K {}p", h),

                2160.. => format!("4K {}p", h),

                1440.. => format!("2K {}p", h),

                1080 => "1080p FHD".to_string(),

                720 => "720p HD".to_string(),

                _ => format!("{}p", h),
            }
        } else if self.is_audio_only() {
            "Audio Only".to_string()
        } else if let Some(ref r) = self.resolution {
            r.clone()
        } else {
            "?".to_string()
        };

        if self.is_hdr() && !self.is_audio_only() {
            format!("{} HDR", base)
        } else {
            base
        }
    }

    pub fn acodec_with_lang(&self) -> String {
        let codec = self.acodec_clean();

        if codec == "-" {
            return "-".to_string();
        }

        if let Some(ref lang) = self.language {
            let l = lang.trim();

            if !l.is_empty() && l != "und" {
                return format!("{} [{}]", codec, l);
            }
        }

        codec.to_string()
    }

    pub fn vcodec_rank(&self) -> u8 {
        let v = self.vcodec.as_deref().unwrap_or("none").to_lowercase();

        if v.starts_with("av01") {
            1
        } else if v.starts_with("vp9") || v.starts_with("vp09") {
            2
        } else if v.starts_with("hvc1")
            || v.starts_with("hev1")
            || v.starts_with("hevc")
            || v.starts_with("h265")
        {
            3
        } else if v.starts_with("avc1") || v.starts_with("h264") {
            4
        } else {
            99
        }
    }

    pub fn acodec_rank(&self) -> u8 {
        let a = self.acodec.as_deref().unwrap_or("none").to_lowercase();

        if a.contains("opus") {
            1
        } else if a.contains("aac") || a.contains("mp4a") {
            2
        } else if a.contains("mp3") {
            3
        } else {
            99
        }
    }
}

pub fn sort_and_filter_formats(formats: &[Format]) -> Vec<Format> {
    let mut valid: Vec<Format> = formats
        .iter()
        .filter(|f| !f.is_storyboard())
        .cloned()
        .collect();

    valid.sort_by(|a, b| {
        // Audio-only at the bottom

        if a.is_audio_only() != b.is_audio_only() {
            return a.is_audio_only().cmp(&b.is_audio_only());
        }

        if a.is_audio_only() && b.is_audio_only() {
            // Audio quality ranking (Opus > AAC > MP3)

            if a.acodec_rank() != b.acodec_rank() {
                return a.acodec_rank().cmp(&b.acodec_rank());
            }

            let a_size = a.filesize.or(a.filesize_approx).unwrap_or(0);

            let b_size = b.filesize.or(b.filesize_approx).unwrap_or(0);

            return b_size.cmp(&a_size);
        }

        // Video streams: Height descending (8K -> 4K -> 1440p -> 1080p -> ...)

        let a_h = a.height.unwrap_or(0);

        let b_h = b.height.unwrap_or(0);

        if a_h != b_h {
            return b_h.cmp(&a_h);
        }

        // FPS descending (60fps > 30fps)

        let a_fps = a.fps.map(|f| f.round() as u32).unwrap_or(0);

        let b_fps = b.fps.map(|f| f.round() as u32).unwrap_or(0);

        if a_fps != b_fps {
            return b_fps.cmp(&a_fps);
        }

        // Codec rank (AV1 > VP9 > HEVC > H264)

        if a.vcodec_rank() != b.vcodec_rank() {
            return a.vcodec_rank().cmp(&b.vcodec_rank());
        }

        // Filesize / bitrate descending

        let a_size = a.filesize.or(a.filesize_approx).unwrap_or(0);

        let b_size = b.filesize.or(b.filesize_approx).unwrap_or(0);

        b_size.cmp(&a_size)
    });

    valid
}

pub fn is_playlist_url(url: &str) -> bool {
    let lower = url.to_lowercase();

    lower.contains("list=") || lower.contains("/playlist")
}

pub fn build_format_string(mode: &str, codec: &str, custom_format: Option<&str>) -> String {
    if let Some(f) = custom_format {
        return f.to_string();
    }

    let codec_filter = match codec.to_lowercase().as_str() {
        "av1" => "[vcodec^=av01]",

        "vp9" => "[vcodec^=vp09]",

        "hevc" => "[vcodec^=hvc1]",

        "h264" => "[vcodec^=avc1]",

        _ => "",
    };

    match mode {
        "quick-max" => {
            if codec_filter.is_empty() {
                "bv*+ba/b".to_string()
            } else {
                format!("bv*{}+ba/bv*+ba/b", codec_filter)
            }
        }

        "quick-1080p" => {
            if codec_filter.is_empty() {
                "bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b".to_string()
            } else {
                format!(
                    "bv*[height<=1080]{}+ba/bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b",
                    codec_filter
                )
            }
        }

        "quick-4k" => {
            if codec_filter.is_empty() {
                "bv*[height<=2160]+ba/b[height<=2160]/bv*+ba/b".to_string()
            } else {
                format!(
                    "bv*[height<=2160]{}+ba/bv*[height<=2160]+ba/b[height<=2160]/bv*+ba/b",
                    codec_filter
                )
            }
        }

        _ => "bv*+ba/b".to_string(),
    }
}

pub fn build_playlist_format(preset: &str, codec: &str) -> (String, bool) {
    let codec_filter = match codec.to_lowercase().as_str() {
        "av1" => "[vcodec^=av01]",

        "vp9" => "[vcodec^=vp09]",

        "hevc" => "[vcodec^=hvc1]",

        "h264" => "[vcodec^=avc1]",

        _ => "",
    };

    match preset.to_lowercase().as_str() {
        "audio" | "6" => ("ba/b".to_string(), true),

        "720p" | "5" => {
            if codec_filter.is_empty() {
                ("bv*[height<=720]+ba/b[height<=720]/best".to_string(), false)
            } else {
                (
                    format!(
                        "bv*[height<=720]{}+ba/bv*[height<=720]+ba/b[height<=720]/best",
                        codec_filter
                    ),
                    false,
                )
            }
        }

        "1080p" | "4" => {
            if codec_filter.is_empty() {
                (
                    "bv*[height<=1080]+ba/b[height<=1080]/best".to_string(),
                    false,
                )
            } else {
                (
                    format!(
                        "bv*[height<=1080]{}+ba/bv*[height<=1080]+ba/b[height<=1080]/best",
                        codec_filter
                    ),
                    false,
                )
            }
        }

        "1440p" | "2k" | "3" => {
            if codec_filter.is_empty() {
                (
                    "bv*[height<=1440]+ba/b[height<=1440]/best".to_string(),
                    false,
                )
            } else {
                (
                    format!(
                        "bv*[height<=1440]{}+ba/bv*[height<=1440]+ba/b[height<=1440]/best",
                        codec_filter
                    ),
                    false,
                )
            }
        }

        "2160p" | "4k" | "2" => {
            if codec_filter.is_empty() {
                (
                    "bv*[height<=2160]+ba/b[height<=2160]/best".to_string(),
                    false,
                )
            } else {
                (
                    format!(
                        "bv*[height<=2160]{}+ba/bv*[height<=2160]+ba/b[height<=2160]/best",
                        codec_filter
                    ),
                    false,
                )
            }
        }

        _ => {
            if codec_filter.is_empty() {
                ("bv*+ba/b".to_string(), false)
            } else {
                (format!("bv*{}+ba/bv*+ba/b", codec_filter), false)
            }
        }
    }
}

pub fn fetch_metadata(
    url: &str,

    deps: &Dependencies,

    config: &Config,

    use_cookies: bool,
) -> Result<VideoMeta> {
    let mut cmd = Command::new("yt-dlp");

    cmd.arg("--dump-json").arg("--no-playlist");

    cmd.args(deps.client_args());

    if use_cookies {
        if let Some(cookie_path) = config.get_cookie_for_url(url) {
            cmd.arg("--cookies").arg(cookie_path);
        }
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

pub fn fetch_playlist_metadata(
    url: &str,

    deps: &Dependencies,

    config: &Config,

    use_cookies: bool,
) -> Result<PlaylistMeta> {
    let mut cmd = Command::new("yt-dlp");

    cmd.arg("--flat-playlist").arg("--dump-single-json");

    cmd.args(deps.client_args());

    if use_cookies {
        if let Some(cookie_path) = config.get_cookie_for_url(url) {
            cmd.arg("--cookies").arg(cookie_path);
        }
    }

    cmd.arg(url);

    let output = cmd
        .output()
        .context("Failed to invoke yt-dlp to extract playlist metadata")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);

        anyhow::bail!("yt-dlp playlist metadata dump failed: {}", err);
    }

    let meta: PlaylistMeta = serde_json::from_slice(&output.stdout)
        .context("Failed to parse yt-dlp playlist JSON output")?;

    Ok(meta)
}

pub fn execute_download(
    url: &str,

    format_spec: Option<&str>,

    mode: &str,

    codec: &str,

    deps: &Dependencies,

    config: &Config,

    use_cookies: bool,
    use_aria2: bool,
) -> Result<()> {
    let mut cmd = Command::new("yt-dlp");

    cmd.args(deps.client_args());

    if use_aria2 {
        cmd.args(Dependencies::aria2_args());
    }

    if use_cookies {
        if let Some(cookie_path) = config.get_cookie_for_url(url) {
            cmd.arg("--cookies").arg(cookie_path);
        }
    }

    cmd.arg("-P").arg(&config.download_dir);

    let final_format = build_format_string(mode, codec, format_spec);

    let is_audio =
        final_format == "ba" || final_format == "ba/b" || final_format.starts_with("ba[");

    let template = if is_audio {
        &config.audio_template
    } else {
        &config.video_template
    };

    cmd.arg("-o").arg(template);

    // Rich media embedding

    cmd.arg("--embed-chapters");

    cmd.arg("--embed-thumbnail");

    cmd.arg("--convert-thumbnails").arg("png");

    if !is_audio {
        cmd.arg("--merge-output-format").arg("mkv");

        cmd.arg("--remux-video").arg("mkv");

        cmd.arg("--embed-subs");

        cmd.arg("--sub-langs").arg("en.*,en");
    }

    cmd.arg("-f").arg(&final_format);

    cmd.arg(url);

    crate::colorizer::execute_with_colorizer(cmd)
}

pub fn execute_playlist_download(
    url: &str,

    format_spec: &str,

    is_audio: bool,

    items: Option<&str>,

    deps: &Dependencies,

    config: &Config,

    use_cookies: bool,
    use_aria2: bool,
) -> Result<()> {
    let mut cmd = Command::new("yt-dlp");

    cmd.args(deps.client_args());

    if use_aria2 {
        cmd.args(Dependencies::aria2_args());
    }

    if use_cookies {
        if let Some(cookie_path) = config.get_cookie_for_url(url) {
            cmd.arg("--cookies").arg(cookie_path);
        }
    }

    cmd.arg("-P").arg(&config.download_dir);

    cmd.arg("--yes-playlist");

    cmd.arg("--ignore-errors");

    let template = if is_audio {
        &config.playlist_audio_template
    } else {
        &config.playlist_video_template
    };

    cmd.arg("-o").arg(template);

    // Rich media embedding

    cmd.arg("--embed-chapters");

    cmd.arg("--embed-thumbnail");

    cmd.arg("--convert-thumbnails").arg("png");

    if !is_audio {
        cmd.arg("--merge-output-format").arg("mkv");

        cmd.arg("--remux-video").arg("mkv");

        cmd.arg("--embed-subs");

        cmd.arg("--sub-langs").arg("en.*,en");
    }

    cmd.arg("-f").arg(format_spec);

    if let Some(item_range) = items {
        let trimmed = item_range.trim();

        if !trimmed.is_empty() {
            cmd.arg("--playlist-items").arg(trimmed);
        }
    }

    cmd.arg(url);

    crate::colorizer::execute_with_colorizer(cmd)
}

pub fn update_ytdlp() -> Result<()> {
    println!("\x1b[36m>> Checking for yt-dlp updates (yt-dlp -U)...\x1b[0m");

    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-U");

    cmd.stdin(Stdio::inherit());

    cmd.stdout(Stdio::inherit());

    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to run yt-dlp -U")?;

    if !status.success() {
        anyhow::bail!("yt-dlp update exited with status {}", status);
    }

    println!("\x1b[32m>> yt-dlp update completed successfully.\x1b[0m");

    Ok(())
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn test_bitrate_clean_numeric() {
        let mut fmt = Format {
            format_id: "137".to_string(),

            ext: "mp4".to_string(),

            resolution: Some("1920x1080".to_string()),

            width: Some(1920),

            height: Some(1080),

            fps: Some(60.0),

            vcodec: Some("avc1.640028".to_string()),

            acodec: None,

            filesize: Some(100_000_000),

            filesize_approx: None,

            tbr: Some(2540.0),

            vbr: Some(2539.6),

            abr: None,

            format_note: None,

            dynamic_range: None,

            language: None,
        };

        // Video stream prefers vbr -> "2540k"

        assert_eq!(fmt.bitrate_clean(), "2540k");

        // Audio stream prefers abr -> "128k"

        let audio_fmt = Format {
            format_id: "251".to_string(),

            ext: "webm".to_string(),

            resolution: None,

            width: None,

            height: None,

            fps: None,

            vcodec: None,

            acodec: Some("opus".to_string()),

            filesize: Some(5_000_000),

            filesize_approx: None,

            tbr: Some(135.0),

            vbr: None,

            abr: Some(128.0),

            format_note: None,

            dynamic_range: None,

            language: None,
        };

        assert_eq!(audio_fmt.bitrate_clean(), "128k");

        // Missing bitrate defaults to "-"

        fmt.vbr = None;

        fmt.tbr = None;

        assert_eq!(fmt.bitrate_clean(), "-");
    }

    #[test]

    fn test_hdr_and_language_detection() {
        let hdr_fmt = Format {
            format_id: "313".to_string(),

            ext: "webm".to_string(),

            resolution: Some("3840x2160".to_string()),

            width: Some(3840),

            height: Some(2160),

            fps: Some(60.0),

            vcodec: Some("vp9.2".to_string()),

            acodec: None,

            filesize: None,

            filesize_approx: None,

            tbr: Some(18000.0),

            vbr: Some(18000.0),

            abr: None,

            format_note: Some("2160p60 HDR".to_string()),

            dynamic_range: Some("HDR10".to_string()),

            language: None,
        };

        assert!(hdr_fmt.is_hdr());

        assert_eq!(hdr_fmt.res_clean(), "4K 2160p HDR");

        let audio_lang_fmt = Format {
            format_id: "251".to_string(),

            ext: "webm".to_string(),

            resolution: None,

            width: None,

            height: None,

            fps: None,

            vcodec: None,

            acodec: Some("opus".to_string()),

            filesize: None,

            filesize_approx: None,

            tbr: Some(128.0),

            vbr: None,

            abr: Some(128.0),

            format_note: Some("original".to_string()),

            dynamic_range: None,

            language: Some("en".to_string()),
        };

        assert!(!audio_lang_fmt.is_hdr());

        assert_eq!(audio_lang_fmt.acodec_with_lang(), "Opus [en]");
    }

    #[test]

    fn test_serde_format_json_with_bitrates() {
        let json_data = "{\"format_id\":\"137\",\"ext\":\"mp4\",\"resolution\":\"1920x1080\",\"width\":1920,\"height\":1080,\"fps\":60,\"vcodec\":\"avc1.640028\",\"acodec\":\"none\",\"vbr\":4200.5,\"abr\":null,\"tbr\":4200.5}";

        let fmt: Format =
            serde_json::from_str(json_data).expect("Failed to deserialize Format JSON");

        assert_eq!(fmt.vbr, Some(4200.5));

        assert_eq!(fmt.abr, None);

        assert_eq!(fmt.bitrate_clean(), "4201k");
    }
}
