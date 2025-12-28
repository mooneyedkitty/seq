// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Transport display widget.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::TransportState;

/// Transport widget for displaying playback state
pub struct TransportWidget<'a> {
    state: &'a TransportState,
    block: Option<Block<'a>>,
}

impl<'a> TransportWidget<'a> {
    /// Create a new transport widget
    pub fn new(state: &'a TransportState) -> Self {
        Self { state, block: None }
    }

    /// Set the block wrapper
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for TransportWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Layout for transport elements
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(12), // Play/Stop indicator
                Constraint::Length(2),  // Spacer
                Constraint::Length(15), // Position
                Constraint::Length(2),  // Spacer
                Constraint::Length(12), // Tempo
                Constraint::Length(2),  // Spacer
                Constraint::Length(8),  // Time signature
                Constraint::Min(0),     // Remaining
            ])
            .split(area);

        // Play/Stop/Record indicator
        let (indicator, style) = if self.state.playing {
            if self.state.recording {
                ("● REC", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            } else {
                ("▶ PLAY", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            }
        } else {
            ("■ STOP", Style::default().fg(Color::Yellow))
        };
        Paragraph::new(indicator).style(style).render(chunks[0], buf);

        // Position: Bar:Beat:Tick
        let position = format!(
            "{:03}:{:02}:{:02}",
            self.state.bar, self.state.beat, self.state.tick
        );
        Paragraph::new(position)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .render(chunks[2], buf);

        // Tempo
        let tempo = format!("{:.1} BPM", self.state.tempo);
        Paragraph::new(tempo)
            .style(Style::default().fg(Color::Magenta))
            .render(chunks[4], buf);

        // Time signature
        let time_sig = format!("{}/{}", self.state.time_sig_num, self.state.time_sig_denom);
        Paragraph::new(time_sig)
            .style(Style::default().fg(Color::White))
            .render(chunks[6], buf);
    }
}

/// Tempo display with visual indicator
pub struct TempoWidget {
    tempo: f64,
    beat_flash: bool,
}

impl TempoWidget {
    /// Create a new tempo widget
    pub fn new(tempo: f64) -> Self {
        Self {
            tempo,
            beat_flash: false,
        }
    }

    /// Set beat flash state (for visual metronome)
    pub fn beat_flash(mut self, flash: bool) -> Self {
        self.beat_flash = flash;
        self
    }
}

impl Widget for TempoWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = if self.beat_flash {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Magenta)
        };

        let tempo = format!("{:.1}", self.tempo);
        Paragraph::new(tempo).style(style).render(area, buf);
    }
}

/// Position display widget
pub struct PositionWidget {
    bar: u64,
    beat: u64,
    tick: u64,
}

impl PositionWidget {
    /// Create a new position widget
    pub fn new(bar: u64, beat: u64, tick: u64) -> Self {
        Self { bar, beat, tick }
    }

    /// Create from total ticks
    pub fn from_ticks(ticks: u64, ppqn: u32, beats_per_bar: u8) -> Self {
        let ticks_per_beat = ppqn as u64;
        let ticks_per_bar = ticks_per_beat * beats_per_bar as u64;

        let bar = ticks / ticks_per_bar + 1;
        let beat = (ticks % ticks_per_bar) / ticks_per_beat + 1;
        let tick = ticks % ticks_per_beat;

        Self { bar, beat, tick }
    }
}

impl Widget for PositionWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let position = format!("{:03}:{:02}:{:02}", self.bar, self.beat, self.tick);
        Paragraph::new(position)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_from_ticks() {
        // First beat of first bar
        let pos = PositionWidget::from_ticks(0, 24, 4);
        assert_eq!(pos.bar, 1);
        assert_eq!(pos.beat, 1);
        assert_eq!(pos.tick, 0);

        // Second beat of first bar
        let pos = PositionWidget::from_ticks(24, 24, 4);
        assert_eq!(pos.bar, 1);
        assert_eq!(pos.beat, 2);
        assert_eq!(pos.tick, 0);

        // First beat of second bar
        let pos = PositionWidget::from_ticks(96, 24, 4);
        assert_eq!(pos.bar, 2);
        assert_eq!(pos.beat, 1);
        assert_eq!(pos.tick, 0);

        // Mid-tick
        let pos = PositionWidget::from_ticks(110, 24, 4);
        assert_eq!(pos.bar, 2);
        assert_eq!(pos.beat, 1);
        assert_eq!(pos.tick, 14);
    }

    #[test]
    fn test_tempo_widget() {
        let widget = TempoWidget::new(120.0);
        assert_eq!(widget.tempo, 120.0);
        assert!(!widget.beat_flash);

        let widget = widget.beat_flash(true);
        assert!(widget.beat_flash);
    }
}
