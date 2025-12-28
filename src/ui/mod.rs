// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Terminal UI for the SEQ sequencer.
//!
//! Provides a ratatui-based terminal interface with transport controls,
//! track status view, and MIDI activity display.

mod transport;
mod tracks;
mod midi_activity;

pub use transport::TransportWidget;
pub use tracks::TracksWidget;
pub use midi_activity::MidiActivityWidget;

use std::io::{self, Stdout};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use crate::sequencer::{SequencerTiming, TrackState};

/// UI state shared between components
#[derive(Debug, Clone)]
pub struct UiState {
    /// Transport state
    pub transport: TransportState,
    /// Track states
    pub tracks: Vec<TrackUiState>,
    /// MIDI activity
    pub midi_activity: MidiActivityState,
    /// Help text visible
    pub show_help: bool,
    /// Status message
    pub status_message: Option<String>,
    /// Status message timestamp
    pub status_time: Option<Instant>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            transport: TransportState::default(),
            tracks: Vec::new(),
            midi_activity: MidiActivityState::default(),
            show_help: false,
            status_message: None,
            status_time: None,
        }
    }
}

impl UiState {
    /// Set a status message that will be displayed temporarily
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
        self.status_time = Some(Instant::now());
    }

    /// Clear expired status message
    pub fn clear_expired_status(&mut self) {
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(3) {
                self.status_message = None;
                self.status_time = None;
            }
        }
    }
}

/// Transport state for UI display
#[derive(Debug, Clone)]
pub struct TransportState {
    /// Whether playing
    pub playing: bool,
    /// Whether recording
    pub recording: bool,
    /// Current tempo in BPM
    pub tempo: f64,
    /// Time signature numerator
    pub time_sig_num: u8,
    /// Time signature denominator
    pub time_sig_denom: u8,
    /// Current bar (1-indexed for display)
    pub bar: u64,
    /// Current beat (1-indexed for display)
    pub beat: u64,
    /// Current tick
    pub tick: u64,
    /// Total ticks elapsed
    pub total_ticks: u64,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            playing: false,
            recording: false,
            tempo: 120.0,
            time_sig_num: 4,
            time_sig_denom: 4,
            bar: 1,
            beat: 1,
            tick: 0,
            total_ticks: 0,
        }
    }
}

impl TransportState {
    /// Update from sequencer timing
    pub fn update_from_timing(&mut self, timing: &SequencerTiming) {
        self.tempo = timing.tempo;
        self.time_sig_num = timing.beats_per_bar;
        self.time_sig_denom = timing.beat_unit;
        self.bar = timing.current_bar() + 1;
        self.beat = timing.current_beat() + 1;
        self.tick = timing.current_tick();
        self.total_ticks = timing.position_ticks;
    }
}

/// Track state for UI display
#[derive(Debug, Clone)]
pub struct TrackUiState {
    /// Track name
    pub name: String,
    /// Track index
    pub index: usize,
    /// MIDI channel (1-16 for display)
    pub channel: u8,
    /// Track state
    pub state: TrackState,
    /// Active clip name (if any)
    pub active_clip: Option<String>,
    /// Generator name (if any)
    pub generator: Option<String>,
    /// Currently playing notes
    pub playing_notes: Vec<u8>,
    /// Velocity meter (0-127)
    pub velocity_meter: u8,
}

impl TrackUiState {
    /// Create a new track UI state
    pub fn new(index: usize, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            index,
            channel: 1,
            state: TrackState::Active,
            active_clip: None,
            generator: None,
            playing_notes: Vec::new(),
            velocity_meter: 0,
        }
    }
}

/// MIDI activity state
#[derive(Debug, Clone, Default)]
pub struct MidiActivityState {
    /// Recent MIDI input messages
    pub input_messages: Vec<MidiActivityMessage>,
    /// Recent MIDI output messages
    pub output_messages: Vec<MidiActivityMessage>,
    /// MIDI learn mode active
    pub learn_mode: bool,
    /// Last learned mapping
    pub last_learned: Option<String>,
    /// Maximum messages to keep
    pub max_messages: usize,
}

impl MidiActivityState {
    /// Create with default capacity
    pub fn new() -> Self {
        Self {
            max_messages: 10,
            ..Default::default()
        }
    }

    /// Add an input message
    pub fn add_input(&mut self, msg: MidiActivityMessage) {
        self.input_messages.push(msg);
        if self.input_messages.len() > self.max_messages {
            self.input_messages.remove(0);
        }
    }

    /// Add an output message
    pub fn add_output(&mut self, msg: MidiActivityMessage) {
        self.output_messages.push(msg);
        if self.output_messages.len() > self.max_messages {
            self.output_messages.remove(0);
        }
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.input_messages.clear();
        self.output_messages.clear();
    }
}

/// A MIDI activity message for display
#[derive(Debug, Clone)]
pub struct MidiActivityMessage {
    /// Message type description
    pub message_type: String,
    /// Channel (1-16)
    pub channel: u8,
    /// Data (note number, CC, etc.)
    pub data: String,
    /// Timestamp
    pub time: Instant,
}

impl MidiActivityMessage {
    /// Create a note on message
    pub fn note_on(channel: u8, note: u8, velocity: u8) -> Self {
        Self {
            message_type: "Note On".to_string(),
            channel,
            data: format!("{} vel:{}", note_name(note), velocity),
            time: Instant::now(),
        }
    }

    /// Create a note off message
    pub fn note_off(channel: u8, note: u8) -> Self {
        Self {
            message_type: "Note Off".to_string(),
            channel,
            data: note_name(note),
            time: Instant::now(),
        }
    }

    /// Create a CC message
    pub fn control_change(channel: u8, cc: u8, value: u8) -> Self {
        Self {
            message_type: "CC".to_string(),
            channel,
            data: format!("{}={}", cc, value),
            time: Instant::now(),
        }
    }
}

/// Convert MIDI note number to name
fn note_name(note: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (note / 12) as i8 - 1;
    let name = NAMES[(note % 12) as usize];
    format!("{}{}", name, octave)
}

/// Key event result
#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    /// No action needed
    None,
    /// Quit the application
    Quit,
    /// Toggle play/pause
    TogglePlay,
    /// Stop playback
    Stop,
    /// Toggle record
    ToggleRecord,
    /// Increase tempo
    TempoUp,
    /// Decrease tempo
    TempoDown,
    /// Nudge tempo up
    NudgeUp,
    /// Nudge tempo down
    NudgeDown,
    /// Toggle track mute
    ToggleMute(usize),
    /// Toggle track solo
    ToggleSolo(usize),
    /// Trigger scene
    TriggerScene(usize),
    /// Toggle help
    ToggleHelp,
    /// Toggle MIDI learn
    ToggleLearn,
}

/// Terminal UI application
pub struct App {
    /// Shared UI state
    state: Arc<Mutex<UiState>>,
    /// Terminal handle
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Target frame rate
    frame_rate: u32,
    /// Whether to continue running
    running: bool,
}

impl App {
    /// Create a new app with shared state
    pub fn new(state: Arc<Mutex<UiState>>) -> io::Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            state,
            terminal,
            frame_rate: 60,
            running: true,
        })
    }

    /// Create app with default state
    pub fn with_default_state() -> io::Result<Self> {
        Self::new(Arc::new(Mutex::new(UiState::default())))
    }

    /// Get shared state handle
    pub fn state(&self) -> Arc<Mutex<UiState>> {
        Arc::clone(&self.state)
    }

    /// Set frame rate
    pub fn set_frame_rate(&mut self, fps: u32) {
        self.frame_rate = fps.clamp(1, 120);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the app
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Handle a key event
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> KeyAction {
        match (code, modifiers) {
            // Quit
            (KeyCode::Char('q'), KeyModifiers::NONE)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.quit();
                KeyAction::Quit
            }

            // Transport
            (KeyCode::Char(' '), KeyModifiers::NONE) => KeyAction::TogglePlay,
            (KeyCode::Esc, KeyModifiers::NONE) => KeyAction::Stop,
            (KeyCode::Char('r'), KeyModifiers::NONE) => KeyAction::ToggleRecord,

            // Tempo
            (KeyCode::Up, KeyModifiers::NONE) => KeyAction::TempoUp,
            (KeyCode::Down, KeyModifiers::NONE) => KeyAction::TempoDown,
            (KeyCode::Up, KeyModifiers::SHIFT) => KeyAction::NudgeUp,
            (KeyCode::Down, KeyModifiers::SHIFT) => KeyAction::NudgeDown,

            // Track mute (1-8)
            (KeyCode::Char(c @ '1'..='8'), KeyModifiers::NONE) => {
                let index = (c as usize) - ('1' as usize);
                KeyAction::ToggleMute(index)
            }

            // Track solo (Shift + 1-8)
            (KeyCode::Char(c @ '!'..='*'), KeyModifiers::SHIFT) => {
                // Shift+1-8 produces !@#$%^&*
                let index = match c {
                    '!' => 0,
                    '@' => 1,
                    '#' => 2,
                    '$' => 3,
                    '%' => 4,
                    '^' => 5,
                    '&' => 6,
                    '*' => 7,
                    _ => return KeyAction::None,
                };
                KeyAction::ToggleSolo(index)
            }

            // Scene triggers (F1-F8)
            (KeyCode::F(n @ 1..=8), KeyModifiers::NONE) => {
                KeyAction::TriggerScene((n - 1) as usize)
            }

            // Help
            (KeyCode::Char('?'), _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                if let Ok(mut state) = self.state.lock() {
                    state.show_help = !state.show_help;
                }
                KeyAction::ToggleHelp
            }

            // MIDI learn
            (KeyCode::Char('l'), KeyModifiers::NONE) => KeyAction::ToggleLearn,

            _ => KeyAction::None,
        }
    }

    /// Poll for events with timeout
    pub fn poll_event(&self) -> io::Result<Option<Event>> {
        let timeout = Duration::from_millis(1000 / self.frame_rate as u64);
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }

    /// Draw the UI
    pub fn draw(&mut self) -> io::Result<()> {
        let state = self.state.lock().unwrap().clone();

        self.terminal.draw(|frame| {
            let area = frame.area();

            // Main layout: header, content, footer
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Transport
                    Constraint::Min(10),    // Tracks
                    Constraint::Length(6),  // MIDI Activity
                    Constraint::Length(1),  // Status bar
                ])
                .split(area);

            // Transport
            render_transport(frame, chunks[0], &state.transport);

            // Tracks
            render_tracks(frame, chunks[1], &state.tracks);

            // MIDI Activity
            render_midi_activity(frame, chunks[2], &state.midi_activity);

            // Status bar
            render_status_bar(frame, chunks[3], &state);

            // Help overlay
            if state.show_help {
                render_help_overlay(frame, area);
            }
        })?;

        Ok(())
    }

    /// Cleanup terminal on drop
    fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Render transport section
fn render_transport(frame: &mut Frame, area: Rect, state: &TransportState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Transport ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Transport info layout
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // Play/Stop
            Constraint::Length(15), // Position
            Constraint::Length(12), // Tempo
            Constraint::Length(10), // Time Sig
            Constraint::Min(0),     // Padding
        ])
        .split(inner);

    // Play/Stop indicator
    let play_text = if state.playing {
        if state.recording {
            Span::styled("● REC", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("▶ PLAY", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        }
    } else {
        Span::styled("■ STOP", Style::default().fg(Color::Yellow))
    };
    frame.render_widget(Paragraph::new(play_text), chunks[0]);

    // Position
    let position = format!("{:03}:{:02}:{:02}", state.bar, state.beat, state.tick);
    let pos_widget = Paragraph::new(position)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(pos_widget, chunks[1]);

    // Tempo
    let tempo = format!("{:.1} BPM", state.tempo);
    let tempo_widget = Paragraph::new(tempo)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(tempo_widget, chunks[2]);

    // Time signature
    let time_sig = format!("{}/{}", state.time_sig_num, state.time_sig_denom);
    let sig_widget = Paragraph::new(time_sig)
        .style(Style::default().fg(Color::White));
    frame.render_widget(sig_widget, chunks[3]);
}

/// Render tracks section
fn render_tracks(frame: &mut Frame, area: Rect, tracks: &[TrackUiState]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Tracks ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tracks.is_empty() {
        let empty = Paragraph::new("No tracks configured")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner);
        return;
    }

    // Calculate track row height
    let track_height = 2;
    let constraints: Vec<Constraint> = tracks
        .iter()
        .map(|_| Constraint::Length(track_height))
        .collect();

    let track_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, track) in tracks.iter().enumerate() {
        if i >= track_chunks.len() {
            break;
        }
        render_track_row(frame, track_chunks[i], track);
    }
}

/// Render a single track row
fn render_track_row(frame: &mut Frame, area: Rect, track: &TrackUiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),  // Index
            Constraint::Length(12), // Name
            Constraint::Length(4),  // Channel
            Constraint::Length(6),  // State (M/S)
            Constraint::Length(15), // Clip/Generator
            Constraint::Min(10),    // Notes/Meter
        ])
        .split(area);

    // Index
    let idx = Paragraph::new(format!("{}", track.index + 1))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(idx, chunks[0]);

    // Name
    let name_style = match track.state {
        TrackState::Muted => Style::default().fg(Color::DarkGray),
        TrackState::Soloed => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        TrackState::Active => Style::default().fg(Color::White),
    };
    let name = Paragraph::new(track.name.clone()).style(name_style);
    frame.render_widget(name, chunks[1]);

    // Channel
    let ch = Paragraph::new(format!("Ch{}", track.channel))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(ch, chunks[2]);

    // State
    let state_text = match track.state {
        TrackState::Muted => Span::styled("M", Style::default().fg(Color::Red)),
        TrackState::Soloed => Span::styled("S", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        TrackState::Active => Span::styled("-", Style::default().fg(Color::DarkGray)),
    };
    frame.render_widget(Paragraph::new(state_text), chunks[3]);

    // Clip/Generator
    let source = track
        .active_clip
        .as_ref()
        .or(track.generator.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("-");
    let source_widget = Paragraph::new(source)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(source_widget, chunks[4]);

    // Velocity meter
    let meter_width = chunks[5].width.saturating_sub(2) as usize;
    let filled = (track.velocity_meter as usize * meter_width) / 127;
    let meter: String = "█".repeat(filled) + &"░".repeat(meter_width - filled);
    let meter_widget = Paragraph::new(meter)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(meter_widget, chunks[5]);
}

/// Render MIDI activity section
fn render_midi_activity(frame: &mut Frame, area: Rect, state: &MidiActivityState) {
    let title = if state.learn_mode {
        " MIDI Activity [LEARN MODE] "
    } else {
        " MIDI Activity "
    };

    let title_style = if state.learn_mode {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, title_style));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into input and output columns
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // Input messages
    render_midi_messages(frame, chunks[0], "Input", &state.input_messages);

    // Output messages
    render_midi_messages(frame, chunks[1], "Output", &state.output_messages);
}

/// Render MIDI messages list
fn render_midi_messages(frame: &mut Frame, area: Rect, label: &str, messages: &[MidiActivityMessage]) {
    let header = Line::from(Span::styled(
        format!(" {} ", label),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));

    let mut lines = vec![header];

    for msg in messages.iter().rev().take(4) {
        let age = msg.time.elapsed();
        let alpha = if age < Duration::from_millis(500) {
            Color::White
        } else if age < Duration::from_secs(2) {
            Color::Gray
        } else {
            Color::DarkGray
        };

        lines.push(Line::from(vec![
            Span::styled(format!("Ch{:02} ", msg.channel), Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:8} ", msg.message_type), Style::default().fg(alpha)),
            Span::styled(&msg.data, Style::default().fg(alpha)),
        ]));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}

/// Render status bar
fn render_status_bar(frame: &mut Frame, area: Rect, state: &UiState) {
    let text = if let Some(ref msg) = state.status_message {
        Span::styled(msg, Style::default().fg(Color::Yellow))
    } else {
        Span::styled(
            " Space: Play/Pause | Esc: Stop | 1-8: Mute | Shift+1-8: Solo | h: Help | q: Quit",
            Style::default().fg(Color::DarkGray),
        )
    };

    frame.render_widget(Paragraph::new(text), area);
}

/// Render help overlay
fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Calculate centered area
    let width = 50.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let help_area = Rect::new(x, y, width, height);

    // Clear background
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        help_area,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let help_text = vec![
        Line::from(Span::styled("Transport", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  Space       Play/Pause"),
        Line::from("  Esc         Stop"),
        Line::from("  r           Toggle Record"),
        Line::from("  Up/Down     Tempo +/- 1 BPM"),
        Line::from("  Shift+Up/Dn Nudge tempo"),
        Line::from(""),
        Line::from(Span::styled("Tracks", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  1-8         Toggle mute"),
        Line::from("  Shift+1-8   Toggle solo"),
        Line::from("  F1-F8       Trigger scene"),
        Line::from(""),
        Line::from(Span::styled("Other", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  l           MIDI learn"),
        Line::from("  h/?         Toggle help"),
        Line::from("  q/Ctrl+c    Quit"),
    ];

    frame.render_widget(Paragraph::new(help_text), inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_name() {
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(69), "A4");
        assert_eq!(note_name(0), "C-1");
        assert_eq!(note_name(127), "G9");
    }

    #[test]
    fn test_transport_state_default() {
        let state = TransportState::default();
        assert!(!state.playing);
        assert_eq!(state.tempo, 120.0);
        assert_eq!(state.bar, 1);
    }

    #[test]
    fn test_ui_state_status() {
        let mut state = UiState::default();
        assert!(state.status_message.is_none());

        state.set_status("Test message");
        assert_eq!(state.status_message, Some("Test message".to_string()));
    }

    #[test]
    fn test_midi_activity_message() {
        let msg = MidiActivityMessage::note_on(1, 60, 100);
        assert_eq!(msg.message_type, "Note On");
        assert_eq!(msg.channel, 1);
        assert!(msg.data.contains("C4"));
    }

    #[test]
    fn test_midi_activity_state() {
        let mut state = MidiActivityState::new();
        assert!(state.input_messages.is_empty());

        state.add_input(MidiActivityMessage::note_on(1, 60, 100));
        assert_eq!(state.input_messages.len(), 1);

        state.clear();
        assert!(state.input_messages.is_empty());
    }

    #[test]
    fn test_track_ui_state() {
        let track = TrackUiState::new(0, "Bass");
        assert_eq!(track.name, "Bass");
        assert_eq!(track.index, 0);
        assert_eq!(track.state, TrackState::Active);
    }
}
