use anyhow::Result;
use clap::{Parser, Subcommand};

mod bridge;
mod config;
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
        None => {}
    }

    // Default: CLI Downloader
    run_cli(cli)
}

fn run_cli(cli: Cli) -> Result<()> {
    println!("\x1b[31;1m>> VampYTD v{}\x1b[0m", env!("CARGO_PKG_VERSION"));

    let deps = deps::Dependencies::check();
    if !deps.ytdlp {
        eprintln!("\x1b[31m>> Error: yt-dlp was not found on PATH. Please install yt-dlp.\x1b[0m");
        std::process::exit(1);
    }
    if !deps.ffmpeg {
        println!("\x1b[33m>> Warning: ffmpeg not found. Stream merging may fail.\x1b[0m");
    }

    let url = match cli.url {
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
    let use_cookies = if cli.no_cookies {
        false
    } else {
        true
    };

    let is_playlist = cli.playlist || downloader::is_playlist_url(&url);

    if is_playlist {
        if cli.mode == "interactive" {
            println!("\x1b[36m>> Fetching playlist metadata: {}\x1b[0m", url);
            let meta = downloader::fetch_playlist_metadata(&url, &deps, &config, use_cookies)?;

            println!("\x1b[32m>> Launching playlist preset selector...\x1b[0m");
            let chosen_preset = tui::run_playlist_preset_picker(&meta)?;

            if let Some(preset) = chosen_preset {
                println!("\x1b[32m>> Selected playlist preset: {}\x1b[0m", preset);
                let (fmt, is_audio) = downloader::build_playlist_format(&preset, &cli.codec);
                downloader::execute_playlist_download(
                    &url,
                    &fmt,
                    is_audio,
                    cli.items.as_deref(),
                    &deps,
                    &config,
                    use_cookies,
                )?;
                let title = meta.title.as_deref().unwrap_or("Playlist");
                notify::send_notification("Playlist Download Complete", title);
            } else {
                println!("\x1b[33m>> Playlist download cancelled.\x1b[0m");
            }
        } else {
            let preset = match cli.mode.as_str() {
                "quick-1080p" => "1080p",
                "quick-4k" => "2160p",
                _ => "best",
            };
            println!(
                "\x1b[36m>> Starting quick playlist download ({}, preset: {}, codec: {})...\x1b[0m",
                cli.mode, preset, cli.codec
            );
            let (fmt, is_audio) = downloader::build_playlist_format(preset, &cli.codec);
            downloader::execute_playlist_download(
                &url,
                &fmt,
                is_audio,
                cli.items.as_deref(),
                &deps,
                &config,
                use_cookies,
            )?;
            notify::send_notification("Playlist Download Complete", &url);
        }
    } else {
        if cli.mode == "interactive" && cli.format.is_none() {
            println!("\x1b[36m>> Fetching formats for: {}\x1b[0m", url);
            let meta = downloader::fetch_metadata(&url, &deps, &config, use_cookies)?;

            println!("\x1b[32m>> Launching interactive stream selector...\x1b[0m");
            let chosen_format = tui::run_format_picker(&meta)?;

            if let Some(fmt_str) = chosen_format {
                println!("\x1b[32m>> Selected format: {}\x1b[0m", fmt_str);
                downloader::execute_download(
                    &url,
                    Some(&fmt_str),
                    &cli.mode,
                    &cli.codec,
                    &deps,
                    &config,
                    use_cookies,
                )?;
                notify::send_notification("Download Complete", &meta.title);
            } else {
                println!("\x1b[33m>> Selection cancelled.\x1b[0m");
            }
        } else {
            println!(
                "\x1b[36m>> Starting quick download ({}, codec: {})...\x1b[0m",
                cli.mode, cli.codec
            );
            downloader::execute_download(
                &url,
                cli.format.as_deref(),
                &cli.mode,
                &cli.codec,
                &deps,
                &config,
                use_cookies,
            )?;
            notify::send_notification("Download Complete", &url);
        }
    }

    Ok(())
}
