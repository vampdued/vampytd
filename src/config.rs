use std::path::{Path, PathBuf};

pub struct Config {
    pub download_dir: PathBuf,
    pub cookies_yt: Option<PathBuf>,
    pub cookies_jhs: Option<PathBuf>,
    pub template: String,
}

impl Config {
    pub fn default_config() -> Self {
        let download_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.download_dir().map(|d| d.join("VampYTD")))
            .unwrap_or_else(|| PathBuf::from("Downloads").join("VampYTD"));

        let template = "%(uploader)s - %(title)s [%(id)s] [%(height)sp] [%(vcodec)s] [%(format_id)s].%(ext)s".to_string();

        let mut config = Self {
            download_dir,
            cookies_yt: None,
            cookies_jhs: None,
            template,
        };

        config.discover_cookies();
        config
    }

    fn discover_cookies(&mut self) {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let c_yt = dir.join("cookies-yt.txt");
                if c_yt.exists() {
                    self.cookies_yt = Some(c_yt);
                }
                let c_jhs = dir.join("cookies-jhs.txt");
                if c_jhs.exists() {
                    self.cookies_jhs = Some(c_jhs);
                }
            }
        }
    }

    pub fn get_cookie_for_url(&self, url: &str) -> Option<&Path> {
        let lower = url.to_lowercase();
        if lower.contains("hotstar.com") || lower.contains("jiohotstar.com") {
            self.cookies_jhs.as_deref()
        } else if lower.contains("youtube.com") || lower.contains("youtu.be") {
            self.cookies_yt.as_deref()
        } else {
            None
        }
    }
}

