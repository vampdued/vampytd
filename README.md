# VampYTD

Unified high-performance YouTube downloader and Chrome Native Messaging Bridge with embedded Ratatui & Nucleo fuzzy terminal UI.

## Features

- 🚀 **Unified Single Binary**: Combines both the interactive CLI downloader and the Chrome/Edge Native Messaging Bridge into one standalone `vampytd.exe`.
- 🔍 **Embedded Fuzzy Selector (No `fzf` Required!)**: Features a custom YouTube Crimson-themed TUI powered by **Ratatui** and **Nucleo-Matcher** (the multi-threaded fuzzy engine behind the Helix editor).
- ⚡ **Zero Subprocess Overhead**: Replaces external `fzf.exe` and PowerShell toast processes with native Rust and WinRT notification APIs.
- 🎬 **Multi-Stream Selection**: Easily select and combine video and audio streams with keyboard navigation (Spacebar to toggle, Enter to confirm).
- 🌐 **Seamless Browser Extension Bridge**: Handles length-prefixed JSON messaging on stdin/stdout to integrate directly with the VampYTD Chrome extension.

## Building

```powershell
cargo build --release
```

The optimized, stripped release binary will be placed at `target/release/vampytd.exe`.

## CLI Usage

### Interactive Mode (Default)
```powershell
vampytd https://www.youtube.com/watch?v=VIDEO_ID
```
Opens the Ratatui interactive format picker with live fuzzy search.

### Quick Modes
```powershell
# Best quality
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-max

# Full HD (1080p)
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-1080p

# Ultra HD (4K)
vampytd https://www.youtube.com/watch?v=VIDEO_ID --mode quick-4k
```

### Chrome Native Messaging Bridge Mode
```powershell
vampytd bridge
```
Launched automatically by Chrome/Edge when triggered from the browser extension.

