<p align="center">
  <img src="assets/og-image-dark.png" alt="VampYTD - Watch. Save. Go." width="100%">
</p>

<p align="center">
  <a href="https://github.com/vampdued/vampytd/releases"><img src="https://img.shields.io/badge/version-2.3.0-FF0A21?style=flat-square" alt="Version 2.3.0"></a>
  <img src="https://img.shields.io/badge/rust-2021_edition-FF0A21?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021">
  <img src="https://img.shields.io/badge/platform-Windows_x64-0E0E12?style=flat-square&logo=windows&logoColor=white" alt="Windows x64">
  <img src="https://img.shields.io/badge/engine-yt--dlp-FF0A21?style=flat-square&logo=youtube&logoColor=white" alt="yt-dlp Engine">
  <img src="https://img.shields.io/badge/license-MIT-0E0E12?style=flat-square" alt="License MIT">
</p>

Unified high-performance YouTube downloader and Chrome Native Messaging Bridge with embedded Ratatui & Nucleo fuzzy terminal UI.

This repository is a **monorepo** containing both the core Rust CLI / native messaging engine and the Manifest V3 browser extension.

---

## 📁 Repository Structure

```
vampytd/
├── src/                  # Core Rust engine & Native Messaging host
│   ├── bridge.rs         # Browser extension communication bridge (stdin/stdout JSON)
│   ├── colorizer.rs      # ANSI real-time terminal download stream colorizer
│   ├── config.rs         # User configuration & cookie management
│   ├── deps.rs           # Dependency validation (yt-dlp, ffmpeg, JS runtimes)
│   ├── downloader.rs     # Stream sorting, HDR/language detection & yt-dlp execution
│   ├── main.rs           # CLI argument parsing & command dispatch
│   ├── notify.rs         # Native Windows toast notifications (WinRT)
│   └── tui.rs            # Interactive Ratatui + Nucleo fuzzy format picker (Vamp theme)
├── extension/            # Chrome / Brave / Edge browser extension (Manifest V3)
│   ├── background.js     # Background service worker & native port manager
│   ├── content.js        # YouTube in-page UI enhancements & context menus
│   ├── manifest.json     # Extension manifest with fixed deterministic ID
│   ├── popup.html/js     # Extension popup dashboard, tool diagnostics & Inter fonts
│   ├── fonts/            # Offline bundled Inter OTF font family
│   └── icons/            # Theme-adaptive SVG vector logos & PNG icons
├── assets/               # Brand assets, multi-res .ico, and social cards
├── build.rs              # Windows resource compiler embedding .ico into vampytd.exe
├── install.ps1           # Automated build, registry registration & deployment script
└── Cargo.toml            # Rust workspace dependencies & release profile
```

---

## 🌟 Key Features

- 🚀 **Unified Single Binary**: Combines both the interactive CLI downloader and the Chrome/Edge Native Messaging Bridge into one standalone `vampytd.exe`.
- 🔍 **Embedded Fuzzy Selector (No external `fzf` required)**: Features a custom YouTube Crimson-themed TUI powered by **Ratatui** and **Nucleo-Matcher** (the multi-threaded fuzzy engine behind the Helix editor).
- 🎨 **Dynamic Stream Inspector**:
  - **YouTube Premium Stream Detection**: Detects format `616` (1080p Enhanced Bitrate) with `[PREM]` gold badging and top-tier ranking.
  - **Filesize Estimation**: Calculates approximate sizes (`~XX.X MB`) for adaptive/HLS streams missing Content-Length headers.
  - **Delivery Protocol Column (`PROTO`)**: Clear tagging for `https`, `m3u8`, and `dash` streams.
  - **Rich Audio Diagnostics**: Detailed sampling rates and audio channels (e.g. `Opus 48k [hi]`, `AAC 48k 5.1 [en]`).
  - **Smart Codec Ranking**: Quality sorting using efficiency multipliers (AV1 2.0x, VP9/HEVC 1.45x, H.264 1.0x).
  - Pure numeric bitrate column (`2540k`, `128k`) indexed for fuzzy searching.
  - Vibrant **`4K 2160p HDR`** badges with automatic dynamic range detection.
  - Multi-track language tagging (e.g. `Opus [en]`, `AAC [hi]`).
  - Real-time auto-pairing status line previewing the exact audio track that pairs with the highlighted video stream.
- ⚡ **Zero Subprocess Overhead**: Replaces external `fzf.exe` and PowerShell toast processes with native Rust and WinRT notification APIs.
- 🍪 **Smart Cookie Management & Sanitizer**:
  - Automatically strips third-party ad/tracking telemetry (DoubleClick, Analytics) and retains essential authentication tokens (`LOGIN_INFO`, `SID`, `HSID`, `SSID`).
  - 1-click file import & drag-and-drop right inside the Chrome/Edge extension popup.
  - Built-in `vampytd cookies status` CLI audit tool reporting session validity, token checklists, and days remaining.
- 🎬 **Multi-Stream Selection**: Easily select and combine video and audio streams with keyboard navigation (`Spacebar` to tag, `Enter` to confirm).
- 🌐 **Seamless Browser Extension Bridge**: One-click download trigger directly from YouTube pages in Chrome, Brave, and Edge via Chrome Native Messaging.

---

## 📋 Prerequisites

VampYTD leverages dedicated media engines for streaming and multiplexing. You can install the recommended tools on Windows with `winget`:

```powershell
# Required: Downloader and media multiplexer
winget install yt-dlp Gyan.FFmpeg

# Optional: Multi-connection download accelerator
winget install aria2

# Optional: JavaScript runtime for player extraction
winget install OpenJS.NodeJS.LTS
```

---

## 🚀 Quick Install (Windows PowerShell)

Because VampYTD is hosted on a public GitHub repository, you can download, extract, and install the latest pre-compiled release binary and browser extension with a single PowerShell command—no GitHub CLI, Git, or Rust installation required:

```powershell
$zip = "$env:TEMP\VampYTD.zip"; $dir = "$env:TEMP\VampYTD-Install"; $url = ((Invoke-RestMethod "https://api.github.com/repos/vampdued/vampytd/releases/latest").assets | Where-Object { $_.name -like "*.zip" } | Select-Object -First 1).browser_download_url; Invoke-WebRequest -Uri $url -OutFile $zip; Expand-Archive -Path $zip -DestinationPath $dir -Force; & "$dir\install.ps1" -SkipBuild; Remove-Item -Path $zip, $dir -Recurse -Force
```

> **Note**: This automatically deploys `vampytd.exe` and the extension to `%LOCALAPPDATA%\VampYTD`, sets up the Native Messaging host registry keys, registers Windows notification handlers, and adds VampYTD to your user PATH.

<details>
<summary><b>Alternative: Manual Release Download</b></summary>

1. Download the latest `VampYTD-v*-windows-x64.zip` from [GitHub Releases](https://github.com/vampdued/vampytd/releases/latest).
2. Extract the ZIP archive to a folder.
3. Open PowerShell inside the extracted folder and run:
   ```powershell
   .\install.ps1 -SkipBuild
   ```
</details>

<details>
<summary><b>Alternative: Install via GitHub CLI (gh)</b></summary>

If you have the GitHub CLI (`gh`) installed, you can download and install directly:
```powershell
gh release download -R vampdued/vampytd --pattern "*.zip" --dir "$env:TEMP\VampYTD-Install" --clobber; Expand-Archive "$env:TEMP\VampYTD-Install\*.zip" -DestinationPath "$env:TEMP\VampYTD-Install\extracted" -Force; & "$env:TEMP\VampYTD-Install\extracted\install.ps1" -SkipBuild; Remove-Item "$env:TEMP\VampYTD-Install" -Recurse -Force
```
</details>

---

## 🛠️ Build & Install from Source

To build from source using the public Git repository:

```powershell
# 1. Clone the public repository
git clone https://github.com/vampdued/vampytd.git
cd vampytd

# 2. Build the release binary and register Native Messaging
.\install.ps1
```

*(Requires the [Rust toolchain](https://rustup.rs)).*

To skip rebuilding the binary when re-deploying configuration or extension changes:
```powershell
.\install.ps1 -SkipBuild
```

### Loading the Browser Extension:
1. Open **`chrome://extensions`** (or **`edge://extensions`** / **`brave://extensions`** / **`vivaldi://extensions`**).
2. Enable **Developer mode** (toggle in the top-right corner).
3. Click **Load unpacked** and select the extension directory:
   - **Prebuilt / Quick Install**: `%LOCALAPPDATA%\VampYTD\extension`
   - **Local Source Repo**: `vampytd\extension` (or `%LOCALAPPDATA%\VampYTD\extension`)

---

## ⌨️ TUI Keyboard Controls

When running in interactive mode, the terminal opens the stream selector:

| Key | Action |
| :--- | :--- |
| `↑` / `k`, `↓` / `j` | Move selection up / down |
| `/` | Focus search bar (type filter text) |
| `v` | Filter to Video-only streams |
| `a` | Filter to Audio-only streams |
| `h` | Filter to HD+ streams (1080p and above) |
| `c` | Reset active filter |
| `Space` | Tag / untag highlighted stream for custom multi-stream merge |
| `Enter` | Download highlighted stream (or combined tagged streams) |
| `Esc` / `q` | Cancel / Exit |

---

## 💻 CLI Usage

### Interactive Mode (Default)
```powershell
vampytd https://www.youtube.com/watch?v=VIDEO_ID
```

### Quick Modes
```powershell
# Best quality available
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-max

# Full HD (1080p)
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-1080p

# Ultra HD (4K)
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-4k
```

### Playlists
```powershell
# Single video from a playlist URL (default — strips playlist params)
vampytd "https://www.youtube.com/watch?v=VIDEO_ID&list=PLAYLIST_ID"

# Full playlist
vampytd "https://www.youtube.com/playlist?list=PLAYLIST_ID" --playlist

# Specific playlist items (e.g. items 1 to 5)
vampytd https://www.youtube.com/playlist?list=PLAYLIST_ID --playlist --items 1-5
```

### Update yt-dlp
```powershell
vampytd update
```

### Cookie Management (Sanitized Auth)
```powershell
# Audit health and validity of saved cookies
vampytd cookies status

# Sanitize and import any Netscape cookies.txt file
vampytd cookies import "C:\path\to\downloaded_cookies.txt"

# Clear saved cookies
vampytd cookies clear
```

### Native Messaging Host Mode (Automated)
```powershell
vampytd bridge
```
*Invoked automatically by the browser extension via standard I/O length-prefixed JSON.*

---

## 🔒 Security & Privacy

- **Local Execution**: All media operations and downloads execute directly on your local system.
- **Zero Telemetry**: VampYTD does not collect, log, or send any telemetry, usage patterns, or analytics.
- **Sanitized Authentication**: The cookie sanitizer actively deletes third-party tracking, ad-targeting, and analytics identifiers before storing YouTube session cookies.
- **Direct Native Messaging**: Extension communication operates over standard OS input/output pipes with no local web server or open listening ports.

---

## ❓ Troubleshooting & FAQ

<details>
<summary><b>Extension displays "Offline" or fails to connect to bridge</b></summary>

1. Confirm that `install.ps1` was executed to register the host in the Windows Registry.
2. Fully restart your browser (close all browser windows) so native messaging host registrations take effect.
3. Check `chrome://extensions` to ensure the unpacked extension is loaded from `%LOCALAPPDATA%\VampYTD\extension`.
</details>

<details>
<summary><b>Video downloads succeed but lack audio or fail during merge</b></summary>

High-resolution streams require `ffmpeg` to merge separate video and audio tracks. Verify that `ffmpeg` is present in your system PATH by running `ffmpeg -version`. To install:
```powershell
winget install Gyan.FFmpeg
```
</details>

<details>
<summary><b>How do I keep yt-dlp up to date?</b></summary>

Run the included update command in your terminal:
```powershell
vampytd update
```
</details>

---

## 📄 License

MIT
