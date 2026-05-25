// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

//! TUI serial monitor using ratatui + crossterm.

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use crate::storage::SqliteStorage;

/// Run the TUI monitor.
///
/// # Errors
/// Returns error on terminal or I/O failure.
pub fn run_tui(
    port_name: &str,
    baud: u32,
    storage: &Arc<std::sync::Mutex<SqliteStorage>>,
    write_port: &Arc<tokio::sync::Mutex<serial2_tokio::SerialPort>>,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run_app(&mut terminal, port_name, baud, storage, write_port, rt);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

struct AppState {
    lines: Vec<(String, String)>, // (timestamp, payload)
    last_id: i64,
    scroll_offset: usize,
    auto_follow: bool,
    input: String,
    port_name: String,
    baud: u32,
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    port_name: &str,
    baud: u32,
    storage: &Arc<std::sync::Mutex<SqliteStorage>>,
    write_port: &Arc<tokio::sync::Mutex<serial2_tokio::SerialPort>>,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = AppState {
        lines: Vec::new(),
        last_id: 0,
        scroll_offset: 0,
        auto_follow: true,
        input: String::new(),
        port_name: port_name.to_string(),
        baud,
    };

    loop {
        // Poll new lines from storage
        {
            let s = storage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Ok(new_lines) = s.read_lines(state.last_id + 1, 500) {
                for line in &new_lines {
                    let ts = chrono::DateTime::from_timestamp_nanos(line.timestamp_ns)
                        .format("%H:%M:%S%.3f")
                        .to_string();
                    state.lines.push((ts, line.payload.clone()));
                    state.last_id = line.id;
                }
            }
        }

        if state.auto_follow && !state.lines.is_empty() {
            let visible_height = terminal.size()?.height.saturating_sub(6) as usize;
            state.scroll_offset = state.lines.len().saturating_sub(visible_height);
        }

        terminal.draw(|frame| render(frame, &state))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    KeyCode::Char('q') if state.input.is_empty() => break,
                    KeyCode::Char(c) => state.input.push(c),
                    KeyCode::Backspace => {
                        state.input.pop();
                    }
                    KeyCode::Enter if !state.input.is_empty() => {
                        let mut data = state.input.as_bytes().to_vec();
                        data.extend_from_slice(b"\r\n");
                        let wp = Arc::clone(write_port);
                        rt.block_on(async {
                            wp.lock().await.write_all(&data).await.ok();
                        });
                        state.input.clear();
                    }
                    KeyCode::Up => {
                        state.auto_follow = false;
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        let visible_height = terminal
                            .size()
                            .map_or(20, |s| s.height.saturating_sub(6) as usize);
                        let max = state.lines.len().saturating_sub(visible_height);
                        state.scroll_offset = (state.scroll_offset + 1).min(max);
                        if state.scroll_offset >= max {
                            state.auto_follow = true;
                        }
                    }
                    KeyCode::PageUp => {
                        state.auto_follow = false;
                        state.scroll_offset = state.scroll_offset.saturating_sub(20);
                    }
                    KeyCode::PageDown => {
                        let visible_height = terminal
                            .size()
                            .map_or(20, |s| s.height.saturating_sub(6) as usize);
                        let max = state.lines.len().saturating_sub(visible_height);
                        state.scroll_offset = (state.scroll_offset + 20).min(max);
                        if state.scroll_offset >= max {
                            state.auto_follow = true;
                        }
                    }
                    KeyCode::End => {
                        state.auto_follow = true;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(5),    // buffer
            Constraint::Length(3), // input
        ])
        .split(area);

    // Status bar
    let follow_indicator = if state.auto_follow {
        "AUTO-FOLLOW"
    } else {
        "PAUSED"
    };
    let status = format!(
        " {} | {} {} | Lines: {} | {}",
        state.port_name,
        state.baud,
        "8N1",
        state.lines.len(),
        follow_indicator
    );
    let status_widget = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status_widget, chunks[0]);

    // Buffer
    let visible_height = chunks[1].height as usize;
    let end = (state.scroll_offset + visible_height).min(state.lines.len());
    let visible_lines: Vec<Line> = state.lines[state.scroll_offset..end]
        .iter()
        .map(|(ts, payload)| {
            let color = if payload.contains("[ERROR]") || payload.contains("[PANIC]") {
                Color::Red
            } else if payload.contains("[WARN]") {
                Color::Yellow
            } else if payload.contains("[DEBUG]") {
                Color::DarkGray
            } else {
                Color::White
            };
            Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(payload.as_str(), Style::default().fg(color)),
            ])
        })
        .collect();

    let buffer_widget =
        Paragraph::new(visible_lines).block(Block::default().borders(Borders::NONE));
    frame.render_widget(buffer_widget, chunks[1]);

    // Scrollbar
    let mut scrollbar_state = ScrollbarState::new(state.lines.len())
        .position(state.scroll_offset)
        .viewport_content_length(visible_height);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        chunks[1],
        &mut scrollbar_state,
    );

    // Input
    let input_widget = Paragraph::new(format!("> {}", state.input)).block(
        Block::default()
            .borders(Borders::TOP)
            .title(" Send (Enter) | Ctrl+C quit "),
    );
    frame.render_widget(input_widget, chunks[2]);
}
