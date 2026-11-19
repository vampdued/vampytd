use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod bridge;
mod colorizer;
mod config;
mod cookies;
mod deps;
mod downloader;
mod notify;
mod tui;

#[derive(Parser, Debug)]
#[command(
    name = "vampytd",
    version,
    about = "Unified high-performance YouTube downloader and Chrome Native Messaging Bridge"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Video or Playlist URL to download
    #[arg(index = 1)]
    url: Option<String>,

    /// Download mode: interactive, quick-max, quick-1080p, quick-4k
    #[arg(short, long, default_value = "interactive")]
    mode: String,

    /// Preferred codec: auto, av1, vp9, hevc, h264
    #[arg(short, long, default_value = "auto")]
    codec: String,

    /// Force enable browser cookies
    #[arg(long)]
    cookies: bool,

    /// Force disable browser cookies
    #[arg(long)]
    no_cookies: bool,

    /// Download full playlist if URL contains a list
    #[arg(long)]
    playlist: bool,

    /// Force single-video download and strip playlist parameters
    #[arg(long)]
    no_playlist: bool,

    /// Playlist item range to download (e.g. 1-10, 5, 8:)
    #[arg(long)]
    items: Option<String>,

    /// Explicit format selector string (e.g. 137+140)
    #[arg(short, long)]
    format: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as Chrome/Edge Native Messaging Host (stdin/stdout JSON)
    Bridge,
    /// Update yt-dlp to latest version (yt-dlp -U)
    Update,
    /// Manage, sanitize, and verify YouTube / Hotstar cookies
    Cookies {
        #[command(subcommand)]
        action: CookieCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum CookieCommands {
    /// Import, sanitize, and save a cookie file (.txt)
    Import {
        /// Path to cookie file (.txt)
        path: PathBuf,
        /// Target platform: youtube (default) or hotstar
        #[arg(short, long, default_value = "youtube")]
        target: String,
    },
    /// Audit and display status of saved cookies
    Status,
    /// Delete / clear saved cookies
    Clear {
        /// Target platform: youtube (default), hotstar, or all
        #[arg(short, long, default_value = "all")]
        target: String,
    },
}

fn main() -> Result<()> {
    // Chrome launches native messaging hosts with `chrome-extension://<id>/` as the first argument
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.iter().any(|a| a.starts_with("chrome-extension://")) {
        return bridge::run_bridge();
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Bridge) => return bridge::run_bridge(),
        Some(Commands::Update) => return downloader::update_ytdlp(),
        Some(Commands::Cookies { action }) => return handle_cookies_cli(action),
        None => {}
    }

    // Default: CLI Downloader
    run_cli(cli)
}

fn handle_cookies_cli(action: CookieCommands) -> Result<()> {
    match action {
        CookieCommands::Import { path, target } => {
            println!("\x1b[38;2;255;10;33;1m>> VampYTD Cookie Sanitizer\x1b[0m");
            if !path.exists() {
                anyhow::bail!("Specified cookie file not found: {}", path.display());
            }

            let content = std::fs::read_to_string(&path)?;
            let status = cookies::import_and_save(&content, &target)?;

            println!("\x1b[32;1m[✓] Saved Destination:\x1b[0m  {}", status.path.unwrap_or_default());
            println!(
                "\x1b[32;1m[✓] Sanitization:\x1b[0m       Kept {} valid cookies (stripped {} ad/trackers)",
                status.sanitized_cookies,
                status.total_cookies.saturating_sub(status.sanitized_cookies)
            );
            println!("\x1b[32;1m[✓] Session Status:\x1b[0m     {}", status.status_label);
            if status.has_login_info && status.has_sid {
                println!("\x1b[36m[✓] Core Auth:\x1b[0m         LOGIN_INFO (✓), SID (✓)");
            }
        }
        CookieCommands::Status => {
            println!("\x1b[38;2;255;10;33;1m================================================================================\x1b[0m");
            println!("\x1b[1;37m>> VampYTD Cookie Health Audit\x1b[0m");
            println!("\x1b[38;2;164;164;176m--------------------------------------------------------------------------------\x1b[0m");

            for target in &["youtube", "hotstar"] {
                let status = cookies::get_cookie_status(target)?;
                let display_name = if *target == "youtube" { "YouTube" } else { "JioHotstar" };

                let badge = if status.valid {
                    "\x1b[32;1m[✓] ACTIVE\x1b[0m"
                } else if status.exists {
                    "\x1b[33;1m[!] EXPIRED/INVALID\x1b[0m"
                } else {
                    "\x1b[38;2;164;164;176m[-] MISSING\x1b[0m"
                };

                println!("\x1b[1;37m[{}]\x1b[0m {}", display_name, badge);
                if status.exists {
                    println!("  Path:        {}", status.path.unwrap_or_default());
                    println!("  Status:      {}", status.status_label);
                    println!("  Cookies:     {} active cookies", status.total_cookies);
                    if *target == "youtube" {
                        let auth_info = match (status.has_login_info, status.has_sid) {
                            (true, true) => "\x1b[32mLOGIN_INFO (✓), SID (✓)\x1b[0m",
                            (true, false) => "\x1b[33mLOGIN_INFO (✓), SID (missing)\x1b[0m",
                            (false, true) => "\x1b[33mLOGIN_INFO (missing), SID (✓)\x1b[0m",
                            (false, false) => "\x1b[31mMissing Core Auth Cookies\x1b[0m",
                        };
                        println!("  Auth Keys:   {}", auth_info);
                    }
                } else {
                    println!("  Path:        {}", status.path.unwrap_or_default());
                    println!("  Note:        File not found (Import via 'vampytd cookies import <path>' or extension)");
                }
                println!();
            }
            println!("\x1b[38;2;255;10;33;1m================================================================================\x1b[0m");
        }
        CookieCommands::Clear { target } => {
            println!("\x1b[38;2;255;10;33;1m>> VampYTD Cookie Clear\x1b[0m");
            if target == "all" {
                let y = cookies::clear_cookie("youtube")?;
                let j = cookies::clear_cookie("hotstar")?;
                if y || j {
                    println!("\x1b[32;1m[✓] All cookies cleared.\x1b[0m");
                } else {
                    println!("\x1b[38;2;164;164;176m[-] No saved cookies found to delete.\x1b[0m");
                }
            } else {
                let deleted = cookies::clear_cookie(&target)?;
                if deleted {
                    println!("\x1b[32;1m[✓] Cleared {} cookies.\x1b[0m", target);
                } else {
                    println!("\x1b[38;2;164;164;176m[-] No cookie file found for {}.\x1b[0m", target);
                }
            }
        }
    }
    Ok(())
}

fn set_terminal_title(title: &str) {
    use std::io::Write;
    print!("\x1b]0;VampYTD: {}\x07", title);
    let _ = std::io::stdout().flush();
}

fn print_download_banner(
    title: &str,
    uploader: Option<&str>,
    format_str: &str,
    dest_dir: &std::path::Path,
    is_audio: bool,
) {
    let uploader_str = uploader.unwrap_or("YouTube");
    let container = if is_audio { "Audio (Native/Opus/M4A)" } else { "MKV (Matroska Universal)" };
    println!();
    println!("\x1b[38;2;255;10;33;1m================================================================================\x1b[0m");
    println!("\x1b[1;37m>> Title:       \x1b[0m\x1b[33;1m{}\x1b[0m", title);
    println!("\x1b[1;37m>> Channel:     \x1b[0m\x1b[36m{}\x1b[0m", uploader_str);
    println!("\x1b[1;37m>> Stream:      \x1b[0m\x1b[38;2;255;10;33;1m{}\x1b[0m", format_str);
    println!("\x1b[1;37m>> Container:   \x1b[0m\x1b[35;1m{}\x1b[0m", container);
    println!("\x1b[1;37m>> Destination: \x1b[0m\x1b[38;2;164;164;176m{}\x1b[0m", dest_dir.display());
    println!("\x1b[38;2;255;10;33;1m================================================================================\x1b[0m");
    println!();
}

fn run_cli(cli: Cli) -> Result<()> {
    println!("\x1b[38;2;255;10;33;1m>> VampYTD v{}\x1b[0m", env!("CARGO_PKG_VERSION"));

    let deps = deps::Dependencies::check();
    if !deps.ytdlp {
        eprintln!("\x1b[31m>> Error: yt-dlp was not found on PATH. Please install yt-dlp.\x1b[0m");
        std::process::exit(1);
    }
    if !deps.ffmpeg {
        println!("\x1b[33m>> Warning: ffmpeg not found. Stream merging may fail.\x1b[0m");
    }

    let raw_url = match cli.url {
        Some(u) => u,
        None => {
            println!("Enter video or playlist URL: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let trimmed = input.trim().to_string();
            if trimmed.is_empty() {
                println!("No URL provided. Exiting.");
                return Ok(());
            }
            trimmed
        }
    };

    let config = config::Config::default_config();

    // Dual Mode Discovery: CLI auto-loads discovered cookies unless --no-cookies was passed
    let use_cookies = !cli.no_cookies;

    // Default /watch?v=...&list=... URLs to single-video unless --playlist is passed;
    // dedicated /playlist URLs still trigger playlist mode unless --no-playlist is passed.
    let is_playlist = if cli.no_playlist {
        false
    } else {
        cli.playlist || downloader::is_playlist_url(&raw_url)
    };

    let url = if is_playlist {
        raw_url
    } else {
        downloader::strip_playlist_params(&raw_url)
    };

    if is_playlist {
        if cli.mode == "interactive" {
            println!("\x1b[36m>> Fetching playlist metadata: {}\x1b[0m", url);
            let meta = downloader::fetch_playlist_metadata(&url, &deps, &config, use_cookies)?;

            println!("\x1b[32m>> Launching playlist preset selector...\x1b[0m");
            let chosen_preset = tui::run_playlist_preset_picker(&meta)?;

            if let Some(preset) = chosen_preset {
                let title = meta.title.as_deref().unwrap_or("Playlist");
                set_terminal_title(&format!("Playlist: {}", title));
                let (fmt, is_audio) = downloader::build_playlist_format(&preset, &cli.codec);
                print_download_banner(title, meta.uploader.as_deref(), &fmt, &config.download_dir, is_audio);
                downloader::execute_playlist_download(
                    &url,
                    &fmt,
                    is_audio,
                    cli.items.as_deref(),
                    &deps,
                    &config,
                    use_cookies,
                )?;
                notify::send_notification("Playlist download complete", title);
            } else {
                println!("\x1b[33m>> Playlist download cancelled.\x1b[0m");
            }
        } else {
            let preset = match cli.mode.as_str() {
                "quick-1080p" => "1080p",
                "quick-4k" => "2160p",
                _ => "best",
            };
            set_terminal_title("Downloading playlist...");
            let (fmt, is_audio) = downloader::build_playlist_format(preset, &cli.codec);
            print_download_banner(&url, None, &fmt, &config.download_dir, is_audio);
            downloader::execute_playlist_download(
                &url,
                &fmt,
                is_audio,
                cli.items.as_deref(),
                &deps,
                &config,
                use_cookies,
            )?;
            notify::send_notification(
                "Playlist download complete",
                &format!("Saved to {}", config.download_dir.display()),
            );
        }
    } else {
        if cli.mode == "interactive" && cli.format.is_none() {
            println!("\x1b[36m>> Fetching formats for: {}\x1b[0m", url);
            let meta = downloader::fetch_metadata(&url, &deps, &config, use_cookies)?;

            println!("\x1b[32m>> Launching interactive stream selector...\x1b[0m");
            let chosen_format = tui::run_format_picker(&meta)?;

            if let Some(fmt_str) = chosen_format {
                set_terminal_title(&meta.title);
                let is_audio = fmt_str == "ba" || fmt_str == "ba/b" || fmt_str.starts_with("ba[");
                print_download_banner(&meta.title, meta.uploader.as_deref(), &fmt_str, &config.download_dir, is_audio);
                downloader::execute_download(
                    &url,
                    Some(&fmt_str),
                    &cli.mode,
                    &cli.codec,
                    &deps,
                    &config,
                    use_cookies,
                )?;
                notify::send_notification("Download complete", &meta.title);
            } else {
                println!("\x1b[33m>> Selection cancelled.\x1b[0m");
            }
        } else {
            set_terminal_title("Downloading video...");
            let final_fmt = downloader::build_format_string(&cli.mode, &cli.codec, cli.format.as_deref());
            let is_audio = final_fmt == "ba" || final_fmt == "ba/b" || final_fmt.starts_with("ba[");
            print_download_banner(&url, None, &final_fmt, &config.download_dir, is_audio);
            downloader::execute_download(
                &url,
                cli.format.as_deref(),
                &cli.mode,
                &cli.codec,
                &deps,
                &config,
                use_cookies,
            )?;
            notify::send_notification(
                "Download complete",
                &format!("Saved to {}", config.download_dir.display()),
            );
        }
    }

    Ok(())
}
