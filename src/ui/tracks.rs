// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Track status display widgets.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::sequencer::TrackState;
use super::TrackUiState;

/// Widget for displaying all tracks
pub struct TracksWidget<'a> {
    tracks: &'a [TrackUiState],
    selected: Option<usize>,
    block: Option<Block<'a>>,
}

impl<'a> TracksWidget<'a> {
    /// Create a new tracks widget
    pub fn new(tracks: &'a [TrackUiState]) -> Self {
        Self {
            tracks,
            selected: None,
            block: None,
        }
    }

    /// Set selected track index
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Set the block wrapper
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for TracksWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if self.tracks.is_empty() {
            Paragraph::new("No tracks configured")
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        // Header
        let header_height = 1;
        let track_height = 1;
        let total_height = header_height + self.tracks.len() as u16 * track_height;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                std::iter::once(Constraint::Length(header_height))
                    .chain(self.tracks.iter().map(|_| Constraint::Length(track_height)))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        // Render header
        render_track_header(chunks[0], buf);

        // Render each track
        for (i, track) in self.tracks.iter().enumerate() {
            let is_selected = self.selected == Some(i);
            render_track_row(chunks[i + 1], buf, track, is_selected);
        }
    }
}

/// Render track header row
fn render_track_header(area: Rect, buf: &mut Buffer) {
    let style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),  // #
            Constraint::Length(12), // Name
            Constraint::Length(4),  // Ch
            Constraint::Length(4),  // M
            Constraint::Length(4),  // S
            Constraint::Length(15), // Source
            Constraint::Min(10),    // Meter
        ])
        .split(area);

    Paragraph::new("#").style(style).render(chunks[0], buf);
    Paragraph::new("Name").style(style).render(chunks[1], buf);
    Paragraph::new("Ch").style(style).render(chunks[2], buf);
    Paragraph::new("M").style(style).render(chunks[3], buf);
    Paragraph::new("S").style(style).render(chunks[4], buf);
    Paragraph::new("Source").style(style).render(chunks[5], buf);
    Paragraph::new("Level").style(style).render(chunks[6], buf);
}

/// Render a single track row
fn render_track_row(area: Rect, buf: &mut Buffer, track: &TrackUiState, selected: bool) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),  // Index
            Constraint::Length(12), // Name
            Constraint::Length(4),  // Channel
            Constraint::Length(4),  // Mute
            Constraint::Length(4),  // Solo
            Constraint::Length(15), // Source
            Constraint::Min(10),    // Meter
        ])
        .split(area);

    // Selection indicator / index
    let idx_style = if selected {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let idx_text = if selected {
        format!(">{}", track.index + 1)
    } else {
        format!(" {}", track.index + 1)
    };
    Paragraph::new(idx_text).style(idx_style).render(chunks[0], buf);

    // Name
    let name_style = match track.state {
        TrackState::Muted => Style::default().fg(Color::DarkGray),
        TrackState::Soloed => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        TrackState::Active => Style::default().fg(Color::White),
    };
    Paragraph::new(track.name.clone())
        .style(name_style)
        .render(chunks[1], buf);

    // Channel
    Paragraph::new(format!("{:2}", track.channel))
        .style(Style::default().fg(Color::Cyan))
        .render(chunks[2], buf);

    // Mute indicator
    let mute_style = if track.state == TrackState::Muted {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let mute_text = if track.state == TrackState::Muted { "M" } else { "·" };
    Paragraph::new(mute_text).style(mute_style).render(chunks[3], buf);

    // Solo indicator
    let solo_style = if track.state == TrackState::Soloed {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let solo_text = if track.state == TrackState::Soloed { "S" } else { "·" };
    Paragraph::new(solo_text).style(solo_style).render(chunks[4], buf);

    // Source (clip or generator)
    let source = track
        .active_clip
        .as_ref()
        .or(track.generator.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("-");
    let source_style = if track.state == TrackState::Muted {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Green)
    };
    Paragraph::new(source).style(source_style).render(chunks[5], buf);

    // Level meter
    render_level_meter(chunks[6], buf, track.velocity_meter, track.state);
}

/// Render a level meter
fn render_level_meter(area: Rect, buf: &mut Buffer, level: u8, state: TrackState) {
    let width = area.width.saturating_sub(1) as usize;
    if width == 0 {
        return;
    }

    let filled = (level as usize * width) / 127;
    let color = match state {
        TrackState::Muted => Color::DarkGray,
        _ => {
            if level > 110 {
                Color::Red
            } else if level > 90 {
                Color::Yellow
            } else {
                Color::Green
            }
        }
    };

    let meter: String = "█".repeat(filled) + &"░".repeat(width - filled);
    Paragraph::new(meter)
        .style(Style::default().fg(color))
        .render(area, buf);
}

/// Widget for displaying a single track in detail
pub struct TrackDetailWidget<'a> {
    track: &'a TrackUiState,
    block: Option<Block<'a>>,
}

impl<'a> TrackDetailWidget<'a> {
    /// Create a new track detail widget
    pub fn new(track: &'a TrackUiState) -> Self {
        Self { track, block: None }
    }

    /// Set the block wrapper
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for TrackDetailWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Name + state
                Constraint::Length(1), // Channel + source
                Constraint::Length(1), // Notes
                Constraint::Min(0),    // Remaining
            ])
            .split(area);

        // Name and state
        let state_indicator = match self.track.state {
            TrackState::Muted => Span::styled(" [MUTED]", Style::default().fg(Color::Red)),
            TrackState::Soloed => Span::styled(" [SOLO]", Style::default().fg(Color::Yellow)),
            TrackState::Active => Span::raw(""),
        };
        let name_line = Line::from(vec![
            Span::styled(&self.track.name, Style::default().add_modifier(Modifier::BOLD)),
            state_indicator,
        ]);
        Paragraph::new(name_line).render(chunks[0], buf);

        // Channel and source
        let source = self.track
            .active_clip
            .as_ref()
            .or(self.track.generator.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("None");
        let info_line = Line::from(vec![
            Span::styled("Ch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", self.track.channel), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(source, Style::default().fg(Color::Green)),
        ]);
        Paragraph::new(info_line).render(chunks[1], buf);

        // Playing notes
        if !self.track.playing_notes.is_empty() {
            let notes: Vec<String> = self.track.playing_notes
                .iter()
                .map(|n| super::note_name(*n))
                .collect();
            let notes_text = format!("Notes: {}", notes.join(" "));
            Paragraph::new(notes_text)
                .style(Style::default().fg(Color::Magenta))
                .render(chunks[2], buf);
        }
    }
}

/// Widget for displaying playing notes as a piano roll snippet
pub struct NoteDisplayWidget {
    notes: Vec<u8>,
    range: (u8, u8),
}

impl NoteDisplayWidget {
    /// Create a new note display widget
    pub fn new(notes: Vec<u8>) -> Self {
        Self {
            notes,
            range: (36, 96), // C2 to C7
        }
    }

    /// Set the display range
    pub fn range(mut self, low: u8, high: u8) -> Self {
        self.range = (low, high);
        self
    }
}

impl Widget for NoteDisplayWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (low, high) = self.range;
        let range_size = (high - low) as usize;
        let width = area.width as usize;

        if width == 0 || range_size == 0 {
            return;
        }

        // Create display string
        let mut display: Vec<char> = vec!['·'; width.min(range_size)];

        for &note in &self.notes {
            if note >= low && note < high {
                let pos = ((note - low) as usize * display.len()) / range_size;
                if pos < display.len() {
                    display[pos] = '█';
                }
            }
        }

        let text: String = display.into_iter().collect();
        Paragraph::new(text)
            .style(Style::default().fg(Color::Cyan))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracks_widget_empty() {
        let tracks: Vec<TrackUiState> = vec![];
        let widget = TracksWidget::new(&tracks);
        // Just verify it creates without panic
        assert!(widget.tracks.is_empty());
    }

    #[test]
    fn test_tracks_widget_with_tracks() {
        let tracks = vec![
            TrackUiState::new(0, "Bass"),
            TrackUiState::new(1, "Drums"),
        ];
        let widget = TracksWidget::new(&tracks).selected(Some(0));
        assert_eq!(widget.tracks.len(), 2);
        assert_eq!(widget.selected, Some(0));
    }

    #[test]
    fn test_track_detail_widget() {
        let track = TrackUiState::new(0, "Lead");
        let widget = TrackDetailWidget::new(&track);
        assert_eq!(widget.track.name, "Lead");
    }

    #[test]
    fn test_note_display_widget() {
        let notes = vec![60, 64, 67]; // C, E, G
        let widget = NoteDisplayWidget::new(notes);
        assert_eq!(widget.range, (36, 96));

        let widget = widget.range(48, 72);
        assert_eq!(widget.range, (48, 72));
    }
}
