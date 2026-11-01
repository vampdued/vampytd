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

use crate::downloader::{Format, VideoMeta};

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
                                return Ok(Some(formats[orig_idx].format_id.clone()));
                            }
                        }
                    } else {
                        // Join tagged streams (e.g. video_id+audio_id)
                        let mut sorted_tags: Vec<usize> = selected_indices.into_iter().collect();
                        sorted_tags.sort();
                        let spec = sorted_tags
                            .into_iter()
                            .map(|i| formats[i].format_id.as_str())
                            .collect::<Vec<_>>()
                            .join("+");
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
