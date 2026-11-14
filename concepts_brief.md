# Concepts Brief: Format Extraction & TUI Stream Inspection Enhancements

* **Author:** VampDued
* **Date:** 2026-11-14
* **Status:** Brainstorming Synthesis Complete

---

## Overview

VampYTD v2.1.0 provides high-performance YouTube downloading and a terminal user interface (TUI) powered by Ratatui and Nucleo. However, analysis of real-world stream extraction comparing native `yt-dlp -F` against VampYTD revealed critical gaps:
1. **Missing Filesize Headers:** Adaptive DASH and HLS streams frequently omit explicit `filesize` and `filesize_approx` in manifest metadata, displaying `?` in the TUI even when duration and bitrate are known.
2. **Opaque YouTube Premium Streams:** High-bitrate 1080p Premium streams (format 616/356) appear identical to baseline 1080p streams.
3. **Codec-Blind Ranking Inversion:** Strict codec prioritization (AV1 > VP9 > HEVC > H264) causes starved 1.5 Mbps AV1 streams to rank ahead of 4.4 Mbps Premium VP9 or 12 Mbps H.264 streams.
4. **Opaque Delivery Protocols:** Direct HTTPS chunked streams and fragmented HLS (`m3u8`) playlists are not distinguished in the interface.
5. **Collapsed Audio Metadata:** High-fidelity audio parameters (5.1 surround sound, 48 kHz / 44.1 kHz sampling rates) are collapsed into a bare codec string.
6. **HLS Audio Stream Dropping Bug:** `is_storyboard()` drops streams where both `vcodec` and `acodec` are unpopulated in the manifest, erroneously discarding valid HLS audio tracks (such as formats 233 and 234).

---

## Analysis & Multi-Lens Findings

### 1. UX & Visual Presentation (Design Lens)
- **Column Real Estate:** Total table width budgeted at 96 columns. `PROTO` is added as a dedicated 6-char column (`https`, `m3u8`, `dash`), while audio sampling rate and channel counts live directly in the `ACODEC` cell (`Opus 48k`, `AAC 5.1 [hi]`).
- **Premium Badge:** YouTube Premium 1080p streams are badged with a bold gold/amber tag (`1080p [PREM]`) in the resolution column and highlighted in the auto-pairing footer status line.
- **Estimated Sizes:** Hard sizes from Content-Length appear in bright green (`60.5 MB`), while calculated sizes appear with a leading tilde in cyan/yellow (`~103.0 MB`). Live streams with unknown duration display `-`.

### 2. Codebase Mechanics & Math (Codebase Details Lens)
- **Filesize Estimation Formula:**
  $$\text{estimated\_bytes} = (\text{bitrate\_kbps} \times 125.0) \times \text{duration\_sec}$$
  Applied safely in `downloader.rs` by checking `duration > 0.0` and validating finite float ranges.
- **HLS Audio Bug Fix:** In `is_storyboard()`, remove the blanket `v == "none" && a == "none"` check. A stream is only classified as a storyboard if `format_id` starts with `sb`, `ext == "mhtml"`, or `format_note` contains `"storyboard"`.
- **Deserialization Extensions:** Add `asr: Option<u32>`, `audio_channels: Option<u8>`, `protocol: Option<String>`, and `quality_label: Option<String>` to `Format`.

### 3. Architecture & Ecosystem Realities (Zoom-Out Lens)
- **Premium Authentication:** Format 616 requires authenticated cookies; if chosen without cookies, provide actionable guidance rather than a silent failure.
- **Protocol Expectations:** Surfacing `m3u8` alerts users that the stream is chunked HLS, which downloads via native fragments and cannot utilize multi-connection chunking.

### 4. Smart Ranking & Sorting Hierarchy (Structural Lens)
Within each resolution height tier:
1. **Premium tier**: `is_premium()` ranks above standard streams.
2. **Effective Bitrate**: Incorporates codec efficiency multipliers (AV1 ~2.0x, VP9/HEVC ~1.45x, H.264 1.0x baseline).
3. **FPS**: 60fps > 30fps.
4. **Filesize / Bitrate**: Descending fallback.

---

## Goals vs. Non-Goals

### In-Scope Goals
- Enrich `Format` struct with `asr`, `audio_channels`, `protocol`, and `quality_label`.
- Implement safe filesize estimation ($\text{bitrate} \times \text{duration}$) replacing `?` with `~XX.X MB`.
- Highlight YouTube Premium 1080p Enhanced Bitrate streams with `[PREM]` badges.
- Rebalance ranking so high-bitrate / Premium streams sit at the top of their resolution tier.
- Surface `PROTO` (`https` vs `m3u8`) and rich audio metadata (`48k`, `44k`, `5.1`).
- Fix `is_storyboard()` to preserve HLS audio streams (formats 233, 234).

### Out-of-Scope Non-Goals
- Modifying yt-dlp backend extraction logic.
- Building custom HLS fragment download decoders.
- Altering the Chrome Native Messaging protocol schema.

---

## Key Decisions & Staging for Next Steps

1. **Phase 1: Domain Struct & Bug Fix (`src/downloader.rs`)**:
   - Extend `Format` with new metadata fields.
   - Refactor `is_storyboard()` and `is_audio_only()` to preserve all legitimate audio streams.
   - Implement `calculate_estimated_size()` and thread `duration` from `VideoMeta`.
2. **Phase 2: Smart Ranking Model (`src/downloader.rs`)**:
   - Implement `effective_bitrate()` and `is_premium()`.
   - Update `sort_and_filter_formats()` comparator.
3. **Phase 3: TUI Table Presentation (`src/tui.rs`)**:
   - Add `PROTO` column and expand `ACODEC` cell formatting.
   - Style estimated sizes with `~` and Premium streams with gold highlights.
4. **Phase 4: Unit Testing & Verification**:
   - Write unit tests covering filesize estimation, HLS audio preservation, and Premium ranking.

