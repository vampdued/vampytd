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

const YT_RED: Color = Color::Rgb(255, 0, 0);
const YT_SURFACE: Color = Color::Rgb(24, 24, 24);

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
                    "{} {} {} {} {} {}",
                    fmt.format_id,
                    fmt.res_clean(),
                    fmt.vcodec_clean(),
                    fmt.acodec_clean(),
                    fmt.fps_clean(),
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

        if let Some(selected) = table_state.selected() {
            if !filtered_indices.is_empty() && selected >= filtered_indices.len() {
                table_state.select(Some(filtered_indices.len() - 1));
            }
        } else if !filtered_indices.is_empty() {
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
                    Constraint::Length(3), // Status / Help
                ])
                .split(f.area());

            // 1. Header
            let title_text = format!(" VampYTD — {} ", meta.title);
            let uploader_text = meta.uploader.as_deref().unwrap_or("YouTube");
            let header_p = Paragraph::new(vec![Line::from(vec![
                Span::styled(&title_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({})", uploader_text), Style::default().fg(Color::DarkGray)),
            ])])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(YT_RED)),
            );
            f.render_widget(header_p, chunks[0]);

            // 2. Search Box with Quick Filter Badges
            let search_border = if is_searching {
                Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let filter_badge = match quick_filter {
                QuickFilter::All => Span::styled(" [All Streams] ", Style::default().fg(Color::White)),
                QuickFilter::VideoOnly => Span::styled(" [Videos Only (v)] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                QuickFilter::AudioOnly => Span::styled(" [Audio Only (a)] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                QuickFilter::HDPlus => Span::styled(" [HD 1080p+ (h)] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            };

            let search_line = Line::from(vec![
                filter_badge,
                Span::styled(" Search: ", Style::default().fg(Color::DarkGray)),
                Span::styled(if query.is_empty() && !is_searching { "Press '/' to type filter... (or 'v' 'a' 'h' 'c')" } else { &query }, 
                    if is_searching { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
            ]);

            let search_p = Paragraph::new(search_line).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Quick Filter & Search ")
                    .border_style(search_border),
            );
            f.render_widget(search_p, chunks[1]);

            // 3. Table Rows
            let header_row = Row::new(vec![
                Span::styled(" SEL", Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)),
                Span::styled("TYPE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("ID", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("RESOLUTION", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("VCODEC", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("ACODEC", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("FPS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("SIZE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("EXT", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
            .bottom_margin(1);

            let rows: Vec<Row> = filtered_indices
                .iter()
                .map(|&orig_idx| {
                    let fmt = &formats[orig_idx];
                    let is_tagged = selected_indices.contains(&orig_idx);
                    let tag_cell = if is_tagged { "[x]" } else { "[ ]" };
                    let tag_color = if is_tagged { YT_RED } else { Color::DarkGray };

                    let type_style = match fmt.stream_type_badge() {
                        "MUX" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        "VID" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        _ => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    };

                    let res_style = if fmt.is_audio_only() {
                        Style::default().fg(Color::Magenta)
                    } else if fmt.height.unwrap_or(0) >= 2160 {
                        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
                    } else if fmt.height.unwrap_or(0) >= 1080 {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let vcodec_clean = fmt.vcodec_clean();
                    let vcodec_style = match vcodec_clean {
                        "AV1" => Style::default().fg(Color::LightGreen),
                        "VP9" => Style::default().fg(Color::LightCyan),
                        "HEVC" => Style::default().fg(Color::Yellow),
                        "H.264" => Style::default().fg(Color::White),
                        _ => Style::default().fg(Color::DarkGray),
                    };

                    let acodec_clean = fmt.acodec_clean();
                    let acodec_style = match acodec_clean {
                        "Opus" => Style::default().fg(Color::LightGreen),
                        "AAC" => Style::default().fg(Color::LightCyan),
                        "MP3" => Style::default().fg(Color::LightYellow),
                        _ => Style::default().fg(Color::DarkGray),
                    };

                    let fps_str = fmt.fps_clean();
                    let fps_style = if fps_str == "60" {
                        Style::default().fg(Color::LightGreen)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };

                    let cells = vec![
                        Span::styled(format!(" {}", tag_cell), Style::default().fg(tag_color).add_modifier(Modifier::BOLD)),
                        Span::styled(fmt.stream_type_badge(), type_style),
                        Span::styled(&fmt.format_id, Style::default().fg(Color::White)),
                        Span::styled(fmt.res_clean(), res_style),
                        Span::styled(vcodec_clean, vcodec_style),
                        Span::styled(acodec_clean, acodec_style),
                        Span::styled(fps_str, fps_style),
                        Span::styled(fmt.size_string(), Style::default().fg(Color::LightGreen)),
                        Span::styled(&fmt.ext, Style::default().fg(Color::DarkGray)),
                    ];
                    Row::new(cells)
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(5),  // SEL
                    Constraint::Length(6),  // TYPE
                    Constraint::Length(8),  // ID
                    Constraint::Length(14), // RESOLUTION
                    Constraint::Length(9),  // VCODEC
                    Constraint::Length(8),  // ACODEC
                    Constraint::Length(6),  // FPS
                    Constraint::Length(10), // SIZE
                    Constraint::Length(6),  // EXT
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Available Streams (Ranked by Quality) ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(YT_SURFACE)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[2], &mut table_state);

            // 4. Footer Bar
            let tagged_count = selected_indices.len();
            let summary = Line::from(vec![
                Span::styled(format!(" Tagged: {} ", tagged_count), Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)),
                Span::styled("| [Space]", Style::default().fg(Color::Yellow)),
                Span::raw(" Tag | "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" Download | "),
                Span::styled("[v]", Style::default().fg(Color::Cyan)),
                Span::raw(" Video | "),
                Span::styled("[a]", Style::default().fg(Color::Magenta)),
                Span::raw(" Audio | "),
                Span::styled("[h]", Style::default().fg(Color::Green)),
                Span::raw(" HD+ | "),
                Span::styled("[c]", Style::default().fg(Color::LightRed)),
                Span::raw(" Reset | "),
                Span::styled("[/]", Style::default().fg(Color::Yellow)),
                Span::raw(" Fuzzy | "),
                Span::styled("[q]", Style::default().fg(Color::DarkGray)),
                Span::raw(" Exit"),
            ]);
            let footer_p = Paragraph::new(summary)
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
            f.render_widget(footer_p, chunks[3]);
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
                        // Fallback to currently highlighted item if none tagged
                        if let Some(selected) = table_state.selected() {
                            if let Some(&orig_idx) = filtered_indices.get(selected) {
                                let fmt = &formats[orig_idx];
                                if fmt.is_video() && fmt.acodec.as_deref().unwrap_or("none") == "none" {
                                    // Smart pairing: video-only stream auto-pairs with bestaudio
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

                        // Smart pairing: if user tagged video but forgot audio, auto-pair with bestaudio
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
        ("1", "Best Available", "Highest quality video + audio", "best"),
        ("2", "2160p (4K UHD)", "Ultra High Definition", "2160p"),
        ("3", "1440p (2K QHD)", "Quad High Definition", "1440p"),
        ("4", "1080p (Full HD)", "Standard High Definition", "1080p"),
        ("5", "720p (HD)", "Efficient High Definition", "720p"),
        ("6", "Audio Only", "Best Audio Stream (Opus/M4A)", "audio"),
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
            let header_text = format!(" VampYTD Playlist Preset Picker | {} ({})", title, count);
            let header = Paragraph::new(Line::from(vec![
                Span::styled(">>", Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)),
                Span::styled(header_text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(YT_RED))
                    .style(Style::default().bg(YT_SURFACE)),
            );
            f.render_widget(header, chunks[0]);

            let rows: Vec<Row> = presets
                .iter()
                .map(|(num, label, desc, _)| {
                    Row::new(vec![
                        Span::styled(format!(" [{}]", num), Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)),
                        Span::styled(*label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(*desc, Style::default().fg(Color::DarkGray)),
                    ])
                    .height(1)
                })
                .collect();

            let header_row = Row::new(vec![
                Span::styled(" Key", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("Preset", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("Description", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
            .bottom_margin(1);

            let table = Table::new(
                rows,
                [
                    Constraint::Length(6),
                    Constraint::Length(22),
                    Constraint::Min(30),
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .title(" Quality Presets ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .row_highlight_style(
                Style::default()
                    .bg(YT_RED)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

            f.render_stateful_widget(table, chunks[1], &mut table_state);

            let footer = Paragraph::new(Line::from(vec![
                Span::styled(" [1-6]", Style::default().fg(YT_RED).add_modifier(Modifier::BOLD)),
                Span::raw(" Quick Select | "),
                Span::styled(" [↑/↓/j/k]", Style::default().fg(Color::Cyan)),
                Span::raw(" Navigate | "),
                Span::styled(" [Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" Confirm | "),
                Span::styled(" [Esc/q]", Style::default().fg(Color::DarkGray)),
                Span::raw(" Cancel"),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
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
