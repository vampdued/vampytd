use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as MatcherConfig, Matcher, Utf32Str,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Terminal,
};
use std::collections::HashSet;
use std::io::{self, Stdout};

use crate::downloader::{self, Format, PlaylistMeta, VideoMeta};

// VampYTD Brand Kit Palette
const VAMP_RED: Color = Color::Rgb(255, 10, 33);
const VAMP_INK: Color = Color::Rgb(14, 14, 18);
const VAMP_GRAPHITE: Color = Color::Rgb(28, 28, 34);
const VAMP_SLATE: Color = Color::Rgb(98, 98, 110);
const VAMP_MUTED: Color = Color::Rgb(164, 164, 176); // #A4A4B0 — AA readable on dark terminals
const VAMP_MIST: Color = Color::Rgb(244, 244, 246);
const VAMP_GOLD: Color = Color::Rgb(255, 215, 0);
const VAMP_AMBER: Color = Color::Rgb(255, 175, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickFilter {
    All,
    VideoOnly,
    AudioOnly,
    HDPlus,
}

pub fn run_format_picker(meta: &VideoMeta) -> Result<Option<String>> {
    let raw_formats = match &meta.formats {
        Some(f) if !f.is_empty() => f.clone(),
        _ => anyhow::bail!("No formats available for this video"),
    };

    let formats = downloader::sort_and_filter_formats(&raw_formats);
    if formats.is_empty() {
        anyhow::bail!("No usable video or audio streams found (only storyboards).");
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = tui_loop(&mut terminal, meta, &formats);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    meta: &VideoMeta,
    formats: &[Format],
) -> Result<Option<String>> {
    let mut selected_indices: HashSet<usize> = HashSet::new();
    let mut table_state = TableState::default();
    table_state.select(Some(0));

    let mut query = String::new();
    let mut is_searching = false;
    let mut quick_filter = QuickFilter::All;
    let mut matcher = Matcher::new(MatcherConfig::DEFAULT);

    loop {
        // 1. Apply quick filter
        let base_indices: Vec<usize> = formats
            .iter()
            .enumerate()
            .filter(|(_, fmt)| match quick_filter {
                QuickFilter::All => true,
                QuickFilter::VideoOnly => fmt.is_video(),
                QuickFilter::AudioOnly => fmt.is_audio_only(),
                QuickFilter::HDPlus => fmt.height.unwrap_or(0) >= 1080,
            })
            .map(|(idx, _)| idx)
            .collect();

        // 2. Filter using nucleo fuzzy matcher if query present
        let filtered_indices: Vec<usize> = if query.trim().is_empty() {
            base_indices
        } else {
            let mut matches: Vec<(usize, u32)> = Vec::new();
            let pattern = Pattern::new(
                &query,
                CaseMatching::Ignore,
                Normalization::Smart,
                nucleo_matcher::pattern::AtomKind::Fuzzy,
            );

            for &idx in &base_indices {
                let fmt = &formats[idx];
                let hay = format!(
                    "{} {} {} {} {} {} {} {} {}",
                    fmt.format_id,
                    fmt.res_clean(),
                    fmt.vcodec_clean(),
                    fmt.acodec_with_details(),
                    fmt.fps_clean(),
                    fmt.bitrate_clean(),
                    fmt.proto_clean(),
                    fmt.size_string(),
                    fmt.ext
                );
                let mut buf = Vec::new();
                let utf32 = Utf32Str::new(&hay, &mut buf);
                if let Some(score) = pattern.score(utf32, &mut matcher) {
                    matches.push((idx, score));
                }
            }

            matches.sort_by(|a, b| b.1.cmp(&a.1));
            matches.into_iter().map(|(idx, _)| idx).collect()
        };

        if filtered_indices.is_empty() {
            table_state.select(None);
        } else if let Some(selected) = table_state.selected() {
            if selected >= filtered_indices.len() {
                table_state.select(Some(filtered_indices.len() - 1));
            }
        } else {
            table_state.select(Some(0));
        }

        // Render UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header title
                    Constraint::Length(3), // Search bar & quick filter pills
                    Constraint::Min(6),    // Formats Table
                    Constraint::Length(1), // Dynamic selection / auto-pairing status line
                    Constraint::Length(3), // Status / Help
                ])
                .split(f.area());

            // 1. Header
            let title_text = format!(" VampYTD — {} ", meta.title);
            let uploader_text = meta.uploader.as_deref().unwrap_or("YouTube");
            let header_p = Paragraph::new(vec![Line::from(vec![
                Span::styled(&title_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({})", uploader_text), Style::default().fg(VAMP_MUTED)),
            ])])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(VAMP_RED)),
            );
            f.render_widget(header_p, chunks[0]);

            // 2. Search Box with Quick Filter Badge
            let search_border = if is_searching {
                Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(VAMP_SLATE)
            };

            let filter_badge = match quick_filter {
                QuickFilter::All => Span::styled(" [All streams] ", Style::default().fg(Color::White)),
                QuickFilter::VideoOnly => Span::styled(" [Video only] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                QuickFilter::AudioOnly => Span::styled(" [Audio only] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                QuickFilter::HDPlus => Span::styled(" [1080p+] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            };

            let search_placeholder = "Press / to search, or filter: [v] Video  [a] Audio  [h] 1080p+";
            let search_line = Line::from(vec![
                filter_badge,
                Span::styled(" Search: ", Style::default().fg(VAMP_MUTED)),
                Span::styled(
                    if query.is_empty() && !is_searching { search_placeholder } else { &query },
                    if is_searching {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(VAMP_MUTED)
                    },
                ),
            ]);

            let search_p = Paragraph::new(search_line).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Filter & search ")
                    .border_style(search_border),
            );
            f.render_widget(search_p, chunks[1]);

            // 3. Formats Table (or clean empty-search state)
            if filtered_indices.is_empty() {
                let empty_msg = if !query.trim().is_empty() {
                    format!(" No streams match \"{}\" — press [c] to reset filters.", query.trim())
                } else {
                    " No streams match the active filter — press [c] to show all streams.".to_string()
                };
                let empty_p = Paragraph::new(Line::from(vec![
                    Span::styled(empty_msg, Style::default().fg(VAMP_MUTED)),
                ]))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Available streams (0 matches) ")
                        .border_style(Style::default().fg(VAMP_SLATE)),
                );
                f.render_widget(empty_p, chunks[2]);
            } else {
                let col_header_style = Style::default().fg(VAMP_MIST).add_modifier(Modifier::BOLD);
                let header_row = Row::new(vec![
                    Span::styled(" SEL", Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                    Span::styled("TYPE", col_header_style),
                    Span::styled("ID", col_header_style),
                    Span::styled("RESOLUTION", col_header_style),
                    Span::styled("VCODEC", col_header_style),
                    Span::styled("ACODEC", col_header_style),
                    Span::styled("FPS", col_header_style),
                    Span::styled("BITRATE", col_header_style),
                    Span::styled("SIZE", col_header_style),
                    Span::styled("PROTO", col_header_style),
                    Span::styled("EXT", col_header_style),
                ])
                .bottom_margin(1);

                let rows: Vec<Row> = filtered_indices
                    .iter()
                    .map(|&orig_idx| {
                        let fmt = &formats[orig_idx];
                        let is_tagged = selected_indices.contains(&orig_idx);
                        let tag_cell = if is_tagged { "[x]" } else { "[ ]" };
                        let tag_color = if is_tagged { VAMP_RED } else { VAMP_MUTED };

                        let type_style = match fmt.stream_type_badge() {
                            "MUX" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                            "VID" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            _ => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                        };

                        // Reserve high-chroma colors for key quality signals ([PREM], HDR, 4K)
                        let res_style = if fmt.is_premium() {
                            Style::default().fg(VAMP_GOLD).add_modifier(Modifier::BOLD)
                        } else if fmt.is_audio_only() {
                            Style::default().fg(Color::Magenta)
                        } else if fmt.is_hdr() {
                            Style::default().fg(VAMP_AMBER).add_modifier(Modifier::BOLD)
                        } else if fmt.height.unwrap_or(0) >= 2160 {
                            Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)
                        } else if fmt.height.unwrap_or(0) >= 1080 {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(VAMP_MIST)
                        };

                        let vcodec_clean = fmt.vcodec_clean();
                        let vcodec_style = if vcodec_clean == "-" {
                            Style::default().fg(VAMP_MUTED)
                        } else {
                            Style::default().fg(VAMP_MIST)
                        };

                        let acodec_str = fmt.acodec_with_details();
                        let acodec_style = if acodec_str == "-" {
                            Style::default().fg(VAMP_MUTED)
                        } else {
                            Style::default().fg(VAMP_MIST)
                        };

                        let fps_str = fmt.fps_clean();
                        let fps_style = if fps_str == "60" {
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(VAMP_MUTED)
                        };

                        let bitrate_str = fmt.bitrate_clean();
                        let bitrate_style = if bitrate_str == "-" {
                            Style::default().fg(VAMP_MUTED)
                        } else {
                            Style::default().fg(VAMP_MIST)
                        };

                        let size_str = fmt.size_string();
                        let size_style = if fmt.is_size_estimated() {
                            Style::default().fg(Color::Cyan)
                        } else if size_str == "-" {
                            Style::default().fg(VAMP_MUTED)
                        } else {
                            Style::default().fg(Color::Green)
                        };

                        let proto_str = fmt.proto_clean();
                        let proto_style = if proto_str == "m3u8" {
                            Style::default().fg(VAMP_AMBER)
                        } else {
                            Style::default().fg(VAMP_MUTED)
                        };

                        let cells = vec![
                            Span::styled(format!(" {}", tag_cell), Style::default().fg(tag_color).add_modifier(Modifier::BOLD)),
                            Span::styled(fmt.stream_type_badge(), type_style),
                            Span::styled(&fmt.format_id, Style::default().fg(Color::White)),
                            Span::styled(fmt.res_clean(), res_style),
                            Span::styled(vcodec_clean, vcodec_style),
                            Span::styled(acodec_str, acodec_style),
                            Span::styled(fps_str, fps_style),
                            Span::styled(bitrate_str, bitrate_style),
                            Span::styled(size_str, size_style),
                            Span::styled(proto_str, proto_style),
                            Span::styled(&fmt.ext, Style::default().fg(VAMP_MUTED)),
                        ];
                        Row::new(cells)
                    })
                    .collect();

                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(5),  // SEL
                        Constraint::Length(4),  // TYPE
                        Constraint::Length(5),  // ID
                        Constraint::Min(14),    // RESOLUTION (flexible)
                        Constraint::Length(6),  // VCODEC
                        Constraint::Min(12),    // ACODEC (flexible)
                        Constraint::Length(3),  // FPS
                        Constraint::Length(7),  // BITRATE
                        Constraint::Length(9),  // SIZE
                        Constraint::Length(5),  // PROTO
                        Constraint::Length(4),  // EXT
                    ],
                )
                .header(header_row)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Available streams ({} shown, ranked by quality) ", filtered_indices.len()))
                        .border_style(Style::default().fg(VAMP_SLATE)),
                )
                .row_highlight_style(
                    Style::default()
                        .bg(VAMP_GRAPHITE)
                        .fg(VAMP_MIST)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

                f.render_stateful_widget(table, chunks[2], &mut table_state);
            }

            // 4. Dynamic Selection & Auto-Pair Status Line
            let selected_idx = table_state.selected().and_then(|sel| filtered_indices.get(sel).copied());

            let preview_line = if !selected_indices.is_empty() {
                let count = selected_indices.len();
                let mut tag_ids: Vec<&str> = selected_indices.iter().map(|&i| formats[i].format_id.as_str()).collect();
                tag_ids.sort();
                Line::from(vec![
                    Span::styled(" >> Tagged: ", Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} streams [{}]", count, tag_ids.join(" + ")), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(" — ", Style::default().fg(VAMP_MUTED)),
                    Span::styled("Press [Enter] to download combined formats", Style::default().fg(Color::White)),
                ])
            } else if let Some(idx) = selected_idx {
                let fmt = &formats[idx];
                if fmt.is_video() && fmt.acodec.as_deref().unwrap_or("none") == "none" {
                    let best_audio = downloader::find_best_audio(formats);
                    let audio_desc = if let Some(a) = best_audio {
                        format!("#{} {} ({}, {})", a.format_id, a.acodec_with_lang(), a.bitrate_clean(), a.size_string())
                    } else {
                        "Best audio (+ba)".to_string()
                    };

                    let mut spans = vec![
                        Span::styled(" >> Selected: ", Style::default().fg(VAMP_MUTED).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("#{} ({} {})", fmt.format_id, fmt.res_clean(), fmt.vcodec_clean()), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(" — Auto-pairs with: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(audio_desc, Style::default().fg(Color::White)),
                    ];
                    if fmt.is_premium() {
                        spans.push(Span::styled(" ★ Premium", Style::default().fg(VAMP_GOLD).add_modifier(Modifier::BOLD)));
                    }
                    Line::from(spans)
                } else if fmt.is_muxed() {
                    Line::from(vec![
                        Span::styled(" >> Selected: ", Style::default().fg(VAMP_MUTED).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("#{} ({} {} + {})", fmt.format_id, fmt.res_clean(), fmt.vcodec_clean(), fmt.acodec_with_lang()), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(" — ", Style::default().fg(VAMP_MUTED)),
                        Span::styled("Pre-muxed stream (no merge needed)", Style::default().fg(Color::White)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(" >> Selected: ", Style::default().fg(VAMP_MUTED).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("#{} {} ({}, {})", fmt.format_id, fmt.acodec_with_lang(), fmt.bitrate_clean(), fmt.size_string()), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                        Span::styled(" — ", Style::default().fg(VAMP_MUTED)),
                        Span::styled("Direct audio extraction", Style::default().fg(Color::White)),
                    ])
                }
            } else {
                Line::from(vec![
                    Span::styled(" >> No stream selected", Style::default().fg(VAMP_MUTED)),
                ])
            };

            f.render_widget(Paragraph::new(preview_line), chunks[3]);

            // 5. Footer Bar
            let tagged_count = selected_indices.len();
            let summary = Line::from(vec![
                Span::styled(format!(" Tagged: {} ", tagged_count), Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                Span::styled("| [Space]", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(" Tag | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" Download | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[v]", Style::default().fg(Color::Cyan)),
                Span::styled(" Video | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[a]", Style::default().fg(Color::Magenta)),
                Span::styled(" Audio | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[h]", Style::default().fg(Color::Green)),
                Span::styled(" 1080p+ | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[c]", Style::default().fg(Color::White)),
                Span::styled(" Reset | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[/]", Style::default().fg(Color::White)),
                Span::styled(" Search | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[q]", Style::default().fg(VAMP_MUTED)),
                Span::styled(" Cancel", Style::default().fg(VAMP_MUTED)),
            ]);

            let footer_p = Paragraph::new(summary)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(VAMP_SLATE)));
            f.render_widget(footer_p, chunks[4]);
        })?;

        // Handle Keyboard Events
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if is_searching {
                match key.code {
                    KeyCode::Esc => {
                        is_searching = false;
                        query.clear();
                    }
                    KeyCode::Enter => {
                        is_searching = false;
                    }
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Char(c) => {
                        query.push(c);
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                KeyCode::Char('/') => {
                    is_searching = true;
                }
                KeyCode::Char('v') => {
                    quick_filter = if quick_filter == QuickFilter::VideoOnly { QuickFilter::All } else { QuickFilter::VideoOnly };
                }
                KeyCode::Char('a') => {
                    quick_filter = if quick_filter == QuickFilter::AudioOnly { QuickFilter::All } else { QuickFilter::AudioOnly };
                }
                KeyCode::Char('h') => {
                    quick_filter = if quick_filter == QuickFilter::HDPlus { QuickFilter::All } else { QuickFilter::HDPlus };
                }
                KeyCode::Char('c') => {
                    quick_filter = QuickFilter::All;
                    query.clear();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if let Some(selected) = table_state.selected() {
                        if selected > 0 {
                            table_state.select(Some(selected - 1));
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(selected) = table_state.selected() {
                        if !filtered_indices.is_empty() && selected < filtered_indices.len() - 1 {
                            table_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(selected) = table_state.selected() {
                        if let Some(&orig_idx) = filtered_indices.get(selected) {
                            if selected_indices.contains(&orig_idx) {
                                selected_indices.remove(&orig_idx);
                            } else {
                                selected_indices.insert(orig_idx);
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    if selected_indices.is_empty() {
                        if let Some(selected) = table_state.selected() {
                            if let Some(&orig_idx) = filtered_indices.get(selected) {
                                let fmt = &formats[orig_idx];
                                if fmt.is_video() && fmt.acodec.as_deref().unwrap_or("none") == "none" {
                                    return Ok(Some(format!("{}+ba", fmt.format_id)));
                                } else {
                                    return Ok(Some(fmt.format_id.clone()));
                                }
                            }
                        }
                    } else {
                        let mut sorted_tags: Vec<usize> = selected_indices.into_iter().collect();
                        sorted_tags.sort();

                        let has_audio = sorted_tags.iter().any(|&i| {
                            let f = &formats[i];
                            f.is_audio_only() || f.acodec.as_deref().unwrap_or("none") != "none"
                        });
                        let has_video = sorted_tags.iter().any(|&i| formats[i].is_video());

                        let mut spec = sorted_tags
                            .into_iter()
                            .map(|i| formats[i].format_id.as_str())
                            .collect::<Vec<_>>()
                            .join("+");

                        if has_video && !has_audio {
                            spec.push_str("+ba");
                        }

                        return Ok(Some(spec));
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn run_playlist_preset_picker(meta: &PlaylistMeta) -> Result<Option<String>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = playlist_tui_loop(&mut terminal, meta);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn playlist_tui_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    meta: &PlaylistMeta,
) -> Result<Option<String>> {
    let presets = [
        ("1", "Best available", "Highest quality video + audio", "best"),
        ("2", "2160p (4K UHD)", "Ultra High Definition", "2160p"),
        ("3", "1440p (2K QHD)", "Quad High Definition", "1440p"),
        ("4", "1080p (Full HD)", "Recommended balance", "1080p"),
        ("5", "720p (HD)", "Smaller file size", "720p"),
        ("6", "Audio only", "Best audio stream (Opus/M4A)", "audio"),
    ];

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(8),    // Table
                    Constraint::Length(3), // Footer
                ])
                .split(f.area());

            let title = meta.title.as_deref().unwrap_or("YouTube Playlist");
            let count = meta.playlist_count.map(|c| format!("{} videos", c)).unwrap_or_else(|| "Playlist".to_string());
            let header_text = format!(" VampYTD Playlist — {} ({})", title, count);

            let header = Paragraph::new(Line::from(vec![
                Span::styled(">>", Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                Span::styled(header_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(VAMP_RED))
                    .style(Style::default().bg(VAMP_INK)),
            );
            f.render_widget(header, chunks[0]);

            let rows: Vec<Row> = presets
                .iter()
                .map(|(num, label, desc, _)| {
                    Row::new(vec![
                        Span::styled(format!(" [{}]", num), Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                        Span::styled(*label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(*desc, Style::default().fg(VAMP_MUTED)),
                    ])
                    .height(1)
                })
                .collect();

            let col_header_style = Style::default().fg(VAMP_MIST).add_modifier(Modifier::BOLD);
            let header_row = Row::new(vec![
                Span::styled(" Key", col_header_style),
                Span::styled("Preset", col_header_style),
                Span::styled("Description", col_header_style),
            ])
            .bottom_margin(1);

            let table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Length(20),
                    Constraint::Min(28),
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .title(" Quality presets ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(VAMP_SLATE)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(VAMP_GRAPHITE)
                    .fg(VAMP_MIST)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[1], &mut table_state);

            let footer = Paragraph::new(Line::from(vec![
                Span::styled(" [1-6]", Style::default().fg(VAMP_RED).add_modifier(Modifier::BOLD)),
                Span::styled(" Quick select | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[↑/↓/j/k]", Style::default().fg(Color::White)),
                Span::styled(" Navigate | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" Download | ", Style::default().fg(VAMP_MUTED)),
                Span::styled("[Esc/q]", Style::default().fg(VAMP_MUTED)),
                Span::styled(" Cancel", Style::default().fg(VAMP_MUTED)),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(VAMP_SLATE)));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                    KeyCode::Char('1') => return Ok(Some("best".to_string())),
                    KeyCode::Char('2') => return Ok(Some("2160p".to_string())),
                    KeyCode::Char('3') => return Ok(Some("1440p".to_string())),
                    KeyCode::Char('4') => return Ok(Some("1080p".to_string())),
                    KeyCode::Char('5') => return Ok(Some("720p".to_string())),
                    KeyCode::Char('6') => return Ok(Some("audio".to_string())),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(selected) = table_state.selected() {
                            if selected > 0 {
                                table_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(selected) = table_state.selected() {
                            if selected < presets.len() - 1 {
                                table_state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = table_state.selected() {
                            if let Some((_, _, _, id)) = presets.get(selected) {
                                return Ok(Some(id.to_string()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
