use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color as RColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::cast::{CastEvent, CastHeader, read_cast_file};
use crate::vterm::VirtualTerminal;

/// Index of a frame: which events constitute it and its timestamp.
struct FrameIndex {
    /// Index of the last event in this frame (inclusive).
    end_event: usize,
    /// Timestamp of this frame.
    timestamp: f64,
}

/// Playback state.
enum PlayState {
    Playing,
    Paused,
}

/// Replay a cast file with TUI playback controls.
pub fn replay(path: &Path, initial_speed: f64) -> Result<()> {
    let (header, events) = read_cast_file(path)?;

    // Build frame index by finding sync-end boundaries.
    let frame_indices = build_frame_index(&header, &events);
    if frame_indices.is_empty() {
        // No sync frames found; treat each event as a frame.
        return replay_simple(header, events, initial_speed);
    }

    // Set up terminal.
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let total_duration = events.last().map(|e| e.time).unwrap_or(0.0);
    let total_frames = frame_indices.len();
    let mut current_frame: usize = 0;
    let mut speed = initial_speed;
    let mut state = PlayState::Playing;
    let mut vt = VirtualTerminal::new(header.width as usize, header.height as usize);
    let mut last_render = Instant::now();
    let mut playback_time: f64 = 0.0;

    // Feed all events up to and including the first frame.
    feed_events_to_frame(&mut vt, &events, &frame_indices, 0);

    loop {
        // Handle input.
        let timeout = Duration::from_millis(16); // ~60fps
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match handle_key(key, &mut state, &mut speed, &mut current_frame, total_frames) {
                    KeyAction::Quit => break,
                    KeyAction::SeekTo(frame) => {
                        current_frame = frame;
                        // Rebuild vterm from scratch up to this frame.
                        vt = VirtualTerminal::new(header.width as usize, header.height as usize);
                        feed_events_to_frame(&mut vt, &events, &frame_indices, current_frame);
                        playback_time = frame_indices[current_frame].timestamp;
                    }
                    KeyAction::JumpSeconds(delta) => {
                        let target_time = (playback_time + delta).max(0.0).min(total_duration);
                        // Find the frame closest to target_time.
                        let target_frame = frame_indices
                            .iter()
                            .position(|f| f.timestamp >= target_time)
                            .unwrap_or(total_frames.saturating_sub(1));
                        current_frame = target_frame;
                        vt = VirtualTerminal::new(header.width as usize, header.height as usize);
                        feed_events_to_frame(&mut vt, &events, &frame_indices, current_frame);
                        playback_time = frame_indices[current_frame].timestamp;
                    }
                    KeyAction::None => {}
                }
            }
        }

        // Advance playback if playing.
        if matches!(state, PlayState::Playing) {
            let real_elapsed = last_render.elapsed().as_secs_f64();
            playback_time += real_elapsed * speed;

            // Advance frames.
            while current_frame + 1 < total_frames
                && frame_indices[current_frame + 1].timestamp <= playback_time
            {
                current_frame += 1;
                // Feed new events.
                let prev_end = if current_frame > 0 {
                    frame_indices[current_frame - 1].end_event + 1
                } else {
                    0
                };
                let cur_end = frame_indices[current_frame].end_event;
                for i in prev_end..=cur_end {
                    if events[i].event_type == "o" {
                        vt.sync_end_seen = false;
                        vt.feed(events[i].data.as_bytes());
                    }
                }
            }
        }
        last_render = Instant::now();

        // Render.
        terminal.draw(|f| {
            let area = f.area();
            // Determine the region for the vterm (leave 1 row for status bar).
            let vterm_height = area.height.saturating_sub(1);
            let vterm_area = Rect::new(area.x, area.y, area.width, vterm_height);
            let status_area = Rect::new(area.x, vterm_height, area.width, 1);

            // Render vterm cells to buffer.
            render_vterm(f, &vt, vterm_area);

            // Render status bar.
            let state_str = match state {
                PlayState::Playing => "PLAYING",
                PlayState::Paused => "PAUSED ",
            };
            let elapsed_str = format_time(playback_time);
            let total_str = format_time(total_duration);
            let status_text = format!(
                " [{}] {}/{}  Frame: {}/{}  Speed: {:.1}x  [Space]=pause [←/→]=step [+/-]=speed [q]=quit",
                state_str,
                elapsed_str,
                total_str,
                current_frame + 1,
                total_frames,
                speed
            );
            let status = Paragraph::new(Line::from(Span::styled(
                status_text,
                Style::default()
                    .fg(RColor::Black)
                    .bg(RColor::White),
            )));
            f.render_widget(status, status_area);
        })?;
    }

    // Restore terminal.
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    Ok(())
}

/// Simple replay for files without sync markers (treat each event as a frame).
fn replay_simple(header: CastHeader, events: Vec<CastEvent>, initial_speed: f64) -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let total_duration = events.last().map(|e| e.time).unwrap_or(0.0);
    let total_events = events.len();
    let mut current_event: usize = 0;
    let mut speed = initial_speed;
    let mut state = PlayState::Playing;
    let mut vt = VirtualTerminal::new(header.width as usize, header.height as usize);
    let mut last_render = Instant::now();
    let mut playback_time: f64 = 0.0;

    if !events.is_empty() && events[0].event_type == "o" {
        vt.feed(events[0].data.as_bytes());
    }

    loop {
        let timeout = Duration::from_millis(16);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => {
                        state = match state {
                            PlayState::Playing => PlayState::Paused,
                            PlayState::Paused => PlayState::Playing,
                        };
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        speed = (speed * 2.0).min(16.0);
                    }
                    KeyCode::Char('-') => {
                        speed = (speed / 2.0).max(0.125);
                    }
                    _ => {}
                }
            }
        }

        if matches!(state, PlayState::Playing) {
            let real_elapsed = last_render.elapsed().as_secs_f64();
            playback_time += real_elapsed * speed;

            while current_event + 1 < total_events
                && events[current_event + 1].time <= playback_time
            {
                current_event += 1;
                if events[current_event].event_type == "o" {
                    vt.feed(events[current_event].data.as_bytes());
                }
            }
        }
        last_render = Instant::now();

        terminal.draw(|f| {
            let area = f.area();
            let vterm_height = area.height.saturating_sub(1);
            let vterm_area = Rect::new(area.x, area.y, area.width, vterm_height);
            let status_area = Rect::new(area.x, vterm_height, area.width, 1);

            render_vterm(f, &vt, vterm_area);

            let state_str = match state {
                PlayState::Playing => "PLAYING",
                PlayState::Paused => "PAUSED ",
            };
            let status_text = format!(
                " [{}] {}/{}  Speed: {:.1}x",
                state_str,
                format_time(playback_time),
                format_time(total_duration),
                speed
            );
            let status = Paragraph::new(Line::from(Span::styled(
                status_text,
                Style::default().fg(RColor::Black).bg(RColor::White),
            )));
            f.render_widget(status, status_area);
        })?;
    }

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    Ok(())
}

/// Build frame index by scanning for sync-end markers.
fn build_frame_index(header: &CastHeader, events: &[CastEvent]) -> Vec<FrameIndex> {
    let mut vt = VirtualTerminal::new(header.width as usize, header.height as usize);
    let mut frames = Vec::new();

    for (i, event) in events.iter().enumerate() {
        if event.event_type != "o" {
            continue;
        }
        vt.sync_end_seen = false;
        vt.feed(event.data.as_bytes());
        if vt.sync_end_seen {
            frames.push(FrameIndex {
                end_event: i,
                timestamp: event.time,
            });
        }
    }

    frames
}

/// Feed all events from the beginning up to and including the given frame.
fn feed_events_to_frame(
    vt: &mut VirtualTerminal,
    events: &[CastEvent],
    frame_indices: &[FrameIndex],
    frame: usize,
) {
    let end = frame_indices[frame].end_event;
    for i in 0..=end {
        if events[i].event_type == "o" {
            vt.feed(events[i].data.as_bytes());
        }
    }
}

/// Render VirtualTerminal cells into a ratatui frame.
fn render_vterm(f: &mut ratatui::Frame, vt: &VirtualTerminal, area: Rect) {
    let buf = f.buffer_mut();
    let rows = (area.height as usize).min(vt.height);
    let cols = (area.width as usize).min(vt.width);

    for r in 0..rows {
        for c in 0..cols {
            let cell = &vt.cells[r][c];
            let x = area.x + c as u16;
            let y = area.y + r as u16;
            if x < buf.area.width && y < buf.area.height {
                let buf_cell = &mut buf[(x, y)];
                buf_cell.set_char(cell.ch);
                buf_cell.set_fg(RColor::Rgb(cell.fg.r, cell.fg.g, cell.fg.b));
                buf_cell.set_bg(RColor::Rgb(cell.bg.r, cell.bg.g, cell.bg.b));
                let mut modifier = Modifier::empty();
                if cell.bold {
                    modifier |= Modifier::BOLD;
                }
                if cell.dim {
                    modifier |= Modifier::DIM;
                }
                if cell.italic {
                    modifier |= Modifier::ITALIC;
                }
                if cell.underline {
                    modifier |= Modifier::UNDERLINED;
                }
                buf_cell.set_style(Style::default().add_modifier(modifier));
            }
        }
    }
}

enum KeyAction {
    Quit,
    SeekTo(usize),
    JumpSeconds(f64),
    None,
}

fn handle_key(
    key: KeyEvent,
    state: &mut PlayState,
    speed: &mut f64,
    current_frame: &mut usize,
    total_frames: usize,
) -> KeyAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Char(' ') => {
            *state = match state {
                PlayState::Playing => PlayState::Paused,
                PlayState::Paused => PlayState::Playing,
            };
            KeyAction::None
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            *speed = (*speed * 2.0).min(16.0);
            KeyAction::None
        }
        KeyCode::Char('-') => {
            *speed = (*speed / 2.0).max(0.125);
            KeyAction::None
        }
        KeyCode::Left => {
            // Step back one frame.
            if *current_frame > 0 {
                KeyAction::SeekTo(*current_frame - 1)
            } else {
                KeyAction::None
            }
        }
        KeyCode::Right => {
            // Step forward one frame.
            if *current_frame + 1 < total_frames {
                KeyAction::SeekTo(*current_frame + 1)
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('[') => KeyAction::JumpSeconds(-10.0),
        KeyCode::Char(']') => KeyAction::JumpSeconds(10.0),
        KeyCode::Home => KeyAction::SeekTo(0),
        KeyCode::End => KeyAction::SeekTo(total_frames.saturating_sub(1)),
        _ => KeyAction::None,
    }
}

fn format_time(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}
