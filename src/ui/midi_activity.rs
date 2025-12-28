// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! MIDI activity display widget.

use std::time::{Duration, Instant};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::{MidiActivityMessage, MidiActivityState};

/// Widget for displaying MIDI activity
pub struct MidiActivityWidget<'a> {
    state: &'a MidiActivityState,
    block: Option<Block<'a>>,
    max_messages: usize,
}

impl<'a> MidiActivityWidget<'a> {
    /// Create a new MIDI activity widget
    pub fn new(state: &'a MidiActivityState) -> Self {
        Self {
            state,
            block: None,
            max_messages: 4,
        }
    }

    /// Set the block wrapper
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set maximum messages to display
    pub fn max_messages(mut self, max: usize) -> Self {
        self.max_messages = max;
        self
    }
}

impl Widget for MidiActivityWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Determine title based on learn mode
        let title = if self.state.learn_mode {
            Span::styled(
                " MIDI Activity [LEARN] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" MIDI Activity ")
        };

        let block = self.block.unwrap_or_else(|| {
            Block::default()
                .borders(Borders::ALL)
                .title(title)
        });

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into input and output columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        // Render input messages
        render_message_column(
            chunks[0],
            buf,
            "Input",
            &self.state.input_messages,
            self.max_messages,
        );

        // Render output messages
        render_message_column(
            chunks[1],
            buf,
            "Output",
            &self.state.output_messages,
            self.max_messages,
        );
    }
}

/// Render a column of MIDI messages
fn render_message_column(
    area: Rect,
    buf: &mut Buffer,
    label: &str,
    messages: &[MidiActivityMessage],
    max: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::once(Constraint::Length(1))
                .chain((0..max).map(|_| Constraint::Length(1)))
                .collect::<Vec<_>>(),
        )
        .split(area);

    // Header
    Paragraph::new(Span::styled(
        format!(" {} ", label),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
    .render(chunks[0], buf);

    // Messages (most recent first)
    for (i, msg) in messages.iter().rev().take(max).enumerate() {
        if i + 1 >= chunks.len() {
            break;
        }

        let age = msg.time.elapsed();
        let alpha = message_color(age);

        let line = Line::from(vec![
            Span::styled(format!("{:2} ", msg.channel), Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:8} ", msg.message_type), Style::default().fg(alpha)),
            Span::styled(&msg.data, Style::default().fg(alpha)),
        ]);

        Paragraph::new(line).render(chunks[i + 1], buf);
    }
}

/// Get color based on message age
fn message_color(age: Duration) -> Color {
    if age < Duration::from_millis(200) {
        Color::White
    } else if age < Duration::from_millis(500) {
        Color::Gray
    } else if age < Duration::from_secs(2) {
        Color::DarkGray
    } else {
        Color::Black
    }
}

/// Widget for displaying controller mappings
pub struct MappingsWidget<'a> {
    mappings: &'a [ControllerMapping],
    block: Option<Block<'a>>,
}

/// A controller mapping for display
#[derive(Debug, Clone)]
pub struct ControllerMapping {
    /// MIDI source (e.g., "CC 1", "Note C4")
    pub source: String,
    /// Target parameter
    pub target: String,
    /// Current value (0.0 - 1.0)
    pub value: f64,
}

impl ControllerMapping {
    /// Create a new mapping
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value: 0.0,
        }
    }

    /// Set current value
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }
}

impl<'a> MappingsWidget<'a> {
    /// Create a new mappings widget
    pub fn new(mappings: &'a [ControllerMapping]) -> Self {
        Self {
            mappings,
            block: None,
        }
    }

    /// Set the block wrapper
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for MappingsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if self.mappings.is_empty() {
            Paragraph::new("No mappings")
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let constraints: Vec<Constraint> = self
            .mappings
            .iter()
            .map(|_| Constraint::Length(1))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, mapping) in self.mappings.iter().enumerate() {
            if i >= chunks.len() {
                break;
            }

            let row_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(12), // Source
                    Constraint::Length(3),  // Arrow
                    Constraint::Length(15), // Target
                    Constraint::Min(10),    // Value bar
                ])
                .split(chunks[i]);

            // Source
            Paragraph::new(&*mapping.source)
                .style(Style::default().fg(Color::Cyan))
                .render(row_chunks[0], buf);

            // Arrow
            Paragraph::new("→")
                .style(Style::default().fg(Color::DarkGray))
                .render(row_chunks[1], buf);

            // Target
            Paragraph::new(&*mapping.target)
                .style(Style::default().fg(Color::Green))
                .render(row_chunks[2], buf);

            // Value bar
            let bar_width = row_chunks[3].width.saturating_sub(2) as usize;
            let filled = (mapping.value * bar_width as f64) as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
            Paragraph::new(bar)
                .style(Style::default().fg(Color::Magenta))
                .render(row_chunks[3], buf);
        }
    }
}

/// Widget for MIDI learn indicator
pub struct LearnIndicatorWidget {
    active: bool,
    last_message: Option<String>,
}

impl LearnIndicatorWidget {
    /// Create a new learn indicator
    pub fn new(active: bool) -> Self {
        Self {
            active,
            last_message: None,
        }
    }

    /// Set last received message
    pub fn last_message(mut self, msg: impl Into<String>) -> Self {
        self.last_message = Some(msg.into());
        self
    }
}

impl Widget for LearnIndicatorWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.active {
            return;
        }

        let style = Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK);

        let text = if let Some(ref msg) = self.last_message {
            format!("LEARN: {} - Press key to assign", msg)
        } else {
            "LEARN: Move controller to assign...".to_string()
        };

        Paragraph::new(text).style(style).render(area, buf);
    }
}

/// Activity indicator (flashes on MIDI activity)
pub struct ActivityIndicator {
    last_activity: Option<Instant>,
    label: String,
}

impl ActivityIndicator {
    /// Create a new activity indicator
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            last_activity: None,
            label: label.into(),
        }
    }

    /// Trigger activity
    pub fn trigger(&mut self) {
        self.last_activity = Some(Instant::now());
    }

    /// Check if currently active (within flash duration)
    pub fn is_active(&self) -> bool {
        self.last_activity
            .map(|t| t.elapsed() < Duration::from_millis(100))
            .unwrap_or(false)
    }
}

impl Widget for ActivityIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = if self.is_active() {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let indicator = if self.is_active() { "●" } else { "○" };
        let text = format!("{} {}", indicator, self.label);

        Paragraph::new(text).style(style).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_activity_widget() {
        let state = MidiActivityState::new();
        let widget = MidiActivityWidget::new(&state);
        assert_eq!(widget.max_messages, 4);

        let widget = widget.max_messages(8);
        assert_eq!(widget.max_messages, 8);
    }

    #[test]
    fn test_controller_mapping() {
        let mapping = ControllerMapping::new("CC 1", "Filter Cutoff")
            .with_value(0.75);

        assert_eq!(mapping.source, "CC 1");
        assert_eq!(mapping.target, "Filter Cutoff");
        assert!((mapping.value - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_mappings_widget() {
        let mappings = vec![
            ControllerMapping::new("CC 1", "Cutoff"),
            ControllerMapping::new("CC 2", "Resonance"),
        ];
        let widget = MappingsWidget::new(&mappings);
        assert_eq!(widget.mappings.len(), 2);
    }

    #[test]
    fn test_learn_indicator() {
        let widget = LearnIndicatorWidget::new(true);
        assert!(widget.active);
        assert!(widget.last_message.is_none());

        let widget = widget.last_message("CC 1");
        assert_eq!(widget.last_message, Some("CC 1".to_string()));
    }

    #[test]
    fn test_activity_indicator() {
        let mut indicator = ActivityIndicator::new("IN");
        assert!(!indicator.is_active());

        indicator.trigger();
        assert!(indicator.is_active());
    }

    #[test]
    fn test_message_color() {
        assert_eq!(message_color(Duration::from_millis(100)), Color::White);
        assert_eq!(message_color(Duration::from_millis(300)), Color::Gray);
        assert_eq!(message_color(Duration::from_secs(1)), Color::DarkGray);
        assert_eq!(message_color(Duration::from_secs(3)), Color::Black);
    }
}
