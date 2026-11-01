use std::path::{Path, PathBuf};

pub struct Config {
    pub download_dir: PathBuf,
    pub cookies_yt: Option<PathBuf>,
    pub cookies_jhs: Option<PathBuf>,
    pub video_template: String,
    pub audio_template: String,
    pub playlist_video_template: String,
    pub playlist_audio_template: String,
}

impl Config {
    pub fn default_config() -> Self {
        let download_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.download_dir().map(|d| d.join("VampYTD")))
            .unwrap_or_else(|| PathBuf::from("Downloads").join("VampYTD"));

        let video_template = "%(uploader)s - %(title)s [%(id)s] [%(height)sp] [%(vcodec)s] [%(format_id)s].%(ext)s".to_string();
        let audio_template = "%(uploader)s - %(title)s [%(id)s] [%(acodec)s] [%(format_id)s].%(ext)s".to_string();
        let playlist_video_template = "%(playlist_title|playlist)s/%(playlist_index)02d - %(title)s [%(id)s] [%(height)sp] [%(vcodec)s].%(ext)s".to_string();
        let playlist_audio_template = "%(playlist_title|playlist)s/%(playlist_index)02d - %(title)s [%(id)s] [%(acodec)s].%(ext)s".to_string();

        let mut config = Self {
            download_dir,
            cookies_yt: None,
            cookies_jhs: None,
            video_template,
            audio_template,
            playlist_video_template,
            playlist_audio_template,
        };

        config.discover_cookies();
        config
    }

    fn discover_cookies(&mut self) {
        // 1. Environment Variable overrides
        if let Ok(path) = std::env::var("VAMPYTD_COOKIES_YT") {
            let p = PathBuf::from(path);
            if p.exists() {
                self.cookies_yt = Some(p);
            }
        }
        if let Ok(path) = std::env::var("VAMPYTD_COOKIES_JHS") {
            let p = PathBuf::from(path);
            if p.exists() {
                self.cookies_jhs = Some(p);
            }
        }

        // 2. User AppData / LocalConfig directory
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let local_app_data = base_dirs.data_local_dir();
            for folder_name in ["VampYTD", "vampytd"] {
                let candidate_dir = local_app_data.join(folder_name);
                if self.cookies_yt.is_none() {
                    let yt_path = candidate_dir.join("cookies-yt.txt");
                    if yt_path.exists() {
                        self.cookies_yt = Some(yt_path);
                    }
                }
                if self.cookies_jhs.is_none() {
                    let jhs_path = candidate_dir.join("cookies-jhs.txt");
                    if jhs_path.exists() {
                        self.cookies_jhs = Some(jhs_path);
                    }
                }
            }
        }

        // 3. Executable parent directory
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                if self.cookies_yt.is_none() {
                    let yt_path = dir.join("cookies-yt.txt");
                    if yt_path.exists() {
                        self.cookies_yt = Some(yt_path);
                    }
                }
                if self.cookies_jhs.is_none() {
                    let jhs_path = dir.join("cookies-jhs.txt");
                    if jhs_path.exists() {
                        self.cookies_jhs = Some(jhs_path);
                    }
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
