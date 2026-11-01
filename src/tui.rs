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

use crate::downloader::{Format, PlaylistMeta, VideoMeta};

const YT_RED: Color = Color::Rgb(255, 0, 0);
const YT_SURFACE: Color = Color::Rgb(24, 24, 24);

pub fn run_format_picker(meta: &VideoMeta) -> Result<Option<String>> {
    let formats = match &meta.formats {
        Some(f) if !f.is_empty() => f.clone(),
        _ => anyhow::bail!("No formats available for this video"),
    };

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
    let mut matcher = Matcher::new(MatcherConfig::DEFAULT);

    loop {
        // Filter formats using nucleo fuzzy matcher
        let filtered_indices: Vec<usize> = if query.trim().is_empty() {
            (0..formats.len()).collect()
        } else {
            let mut matches: Vec<(usize, u32)> = Vec::new();
            let pattern = Pattern::new(
                &query,
                CaseMatching::Ignore,
                Normalization::Smart,
                nucleo_matcher::pattern::AtomKind::Fuzzy,
            );

            for (idx, fmt) in formats.iter().enumerate() {
                let hay = format!(
                    "{} {} {} {} {}",
                    fmt.format_id,
                    fmt.res_string(),
                    fmt.vcodec.as_deref().unwrap_or(""),
                    fmt.acodec.as_deref().unwrap_or(""),
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
                    Constraint::Length(3), // Search bar
                    Constraint::Min(5),    // Formats Table
                    Constraint::Length(2), // Status / Help
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

            // 2. Search Box
            let search_style = if is_searching {
                Style::default().fg(YT_RED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let search_p = Paragraph::new(format!(" > {}", query)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search / Fuzzy Filter (Press / to type) ")
                    .border_style(search_style),
            );
            f.render_widget(search_p, chunks[1]);

            // 3. Table Rows
            let header_cells = ["SEL", "ID", "RES", "VCODEC", "ACODEC", "FPS", "SIZE", "EXT"]
                .iter()
                .map(|h| cell_style(*h));
            let header_row = Row::new(header_cells)
                .style(Style::default().fg(YT_RED).add_modifier(Modifier::BOLD))
                .bottom_margin(1);

            let rows: Vec<Row> = filtered_indices
                .iter()
                .map(|&orig_idx| {
                    let fmt = &formats[orig_idx];
                    let is_tagged = selected_indices.contains(&orig_idx);
                    let tag_cell = if is_tagged { "[x]" } else { "[ ]" };
                    let tag_color = if is_tagged { YT_RED } else { Color::DarkGray };

                    let cells = vec![
                        Span::styled(tag_cell, Style::default().fg(tag_color).add_modifier(Modifier::BOLD)),
                        Span::raw(&fmt.format_id),
                        Span::styled(fmt.res_string(), Style::default().fg(Color::Cyan)),
                        Span::raw(fmt.vcodec.as_deref().unwrap_or("-")),
                        Span::raw(fmt.acodec.as_deref().unwrap_or("-")),
                        Span::raw(fmt.fps.map(|v| v.to_string()).unwrap_or_else(|| "-".into())),
                        Span::styled(fmt.size_string(), Style::default().fg(Color::Green)),
                        Span::raw(&fmt.ext),
                    ];
                    Row::new(cells)
                })
                .collect();

            let table = Table::new(
                rows,
                [
                    Constraint::Length(5),  // Tag
                    Constraint::Length(8),  // ID
                    Constraint::Length(10), // RES
                    Constraint::Length(12), // VCODEC
                    Constraint::Length(12), // ACODEC
                    Constraint::Length(6),  // FPS
                    Constraint::Length(10), // SIZE
                    Constraint::Length(6),  // EXT
                ],
            )
            .header(header_row)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Available Streams (Space: Toggle Multi-Select | Enter: Download) ")
                    .border_style(Style::default().fg(Color::Gray)),
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
            let summary = format!(
                " Selected Streams: {} | [↑/↓/j/k]: Navigate | [Space]: Toggle Stream | [Enter]: Confirm | [Esc]: Back/Exit",
                tagged_count
            );
            let footer_p = Paragraph::new(summary).style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer_p, chunks[3]);
        })?;

        // Handle Keyboard Events
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if is_searching {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
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

fn cell_style<'a>(s: &'a str) -> Span<'a> {
    Span::raw(s)
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
