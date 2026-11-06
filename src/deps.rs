use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependencies {
    pub ytdlp: bool,
    pub ffmpeg: bool,
    pub aria2: bool,
    pub js_runtime: Option<String>,
    pub ytdlp_path: Option<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,
    pub aria2_path: Option<PathBuf>,
}

impl Dependencies {
    pub fn check() -> Self {
        let ytdlp_path = which::which("yt-dlp").ok();
        let ffmpeg_path = which::which("ffmpeg").ok();
        let aria2_path = which::which("aria2c").ok();
        let js_runtime = detect_js_runtime();

        Self {
            ytdlp: ytdlp_path.is_some(),
            ffmpeg: ffmpeg_path.is_some(),
            aria2: aria2_path.is_some(),
            js_runtime,
            ytdlp_path,
            ffmpeg_path,
            aria2_path,
        }
    }

    pub fn client_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(ref runtime) = self.js_runtime {
            args.push("--js-runtime".to_string());
            args.push(runtime.clone());
        }
        args
    }

    pub fn aria2_args() -> Vec<String> {
        vec![
            "--downloader".to_string(),
            "aria2c".to_string(),
            "--downloader-args".to_string(),
            "aria2c:-c -j 16 -x 16 -s 16 -k 1M".to_string(),
        ]
    }
}

fn detect_js_runtime() -> Option<String> {
    for rt in &["node", "bun"] {
        if which::which(rt).is_ok() {
            return Some((*rt).to_string());
        }
    }
    None
}
