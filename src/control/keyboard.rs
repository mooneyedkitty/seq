// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Keyboard shortcut handling.
//!
//! Provides configurable keyboard bindings for transport, track control,
//! and navigation actions.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use super::ControlAction;

/// A keyboard shortcut definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// Key code
    pub code: KeyCode,
    /// Required modifiers
    pub modifiers: KeyModifiers,
}

impl Shortcut {
    /// Create a new shortcut
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Create a shortcut with no modifiers
    pub fn key(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    /// Create a shortcut with Ctrl modifier
    pub fn ctrl(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::CONTROL)
    }

    /// Create a shortcut with Shift modifier
    pub fn shift(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::SHIFT)
    }

    /// Create a shortcut with Alt modifier
    pub fn alt(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::ALT)
    }

    /// Create a shortcut with Ctrl+Shift modifiers
    pub fn ctrl_shift(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
    }

    /// Check if this shortcut matches a key event
    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }
}

/// A keyboard binding (shortcut to action)
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The shortcut
    pub shortcut: Shortcut,
    /// The action to perform
    pub action: ControlAction,
    /// Description for help display
    pub description: String,
    /// Category for grouping in help
    pub category: String,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(
        shortcut: Shortcut,
        action: ControlAction,
        description: impl Into<String>,
    ) -> Self {
        Self {
            shortcut,
            action,
            description: description.into(),
            category: "General".to_string(),
        }
    }

    /// Set the category
    pub fn category(mut self, cat: impl Into<String>) -> Self {
        self.category = cat.into();
        self
    }
}

/// Keyboard controller with configurable bindings
pub struct KeyboardController {
    bindings: HashMap<Shortcut, KeyBinding>,
    /// Repeat delay for held keys (in frames)
    repeat_delay: u32,
    /// Repeat rate for held keys (in frames)
    repeat_rate: u32,
    /// Currently held keys with repeat counters
    held_keys: HashMap<Shortcut, u32>,
}

impl KeyboardController {
    /// Create an empty keyboard controller
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            repeat_delay: 30, // ~500ms at 60fps
            repeat_rate: 6,   // ~100ms at 60fps
            held_keys: HashMap::new(),
        }
    }

    /// Create a keyboard controller with default bindings
    pub fn with_defaults() -> Self {
        let mut controller = Self::new();
        controller.add_default_bindings();
        controller
    }

    /// Add default keyboard bindings
    fn add_default_bindings(&mut self) {
        // Transport
        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char(' ')),
            ControlAction::TogglePlay,
            "Play/Pause",
        ).category("Transport"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Esc),
            ControlAction::Stop,
            "Stop",
        ).category("Transport"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('r')),
            ControlAction::ToggleRecord,
            "Toggle Record",
        ).category("Transport"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Enter),
            ControlAction::Play,
            "Play",
        ).category("Transport"));

        // Tempo
        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Up),
            ControlAction::AdjustTempo(1.0),
            "Tempo +1 BPM",
        ).category("Tempo"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Down),
            ControlAction::AdjustTempo(-1.0),
            "Tempo -1 BPM",
        ).category("Tempo"));

        self.add(KeyBinding::new(
            Shortcut::shift(KeyCode::Up),
            ControlAction::AdjustTempo(10.0),
            "Tempo +10 BPM",
        ).category("Tempo"));

        self.add(KeyBinding::new(
            Shortcut::shift(KeyCode::Down),
            ControlAction::AdjustTempo(-10.0),
            "Tempo -10 BPM",
        ).category("Tempo"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('t')),
            ControlAction::TapTempo,
            "Tap Tempo",
        ).category("Tempo"));

        // Track mute (1-8)
        for i in 0..8 {
            let c = char::from_digit(i + 1, 10).unwrap();
            self.add(KeyBinding::new(
                Shortcut::key(KeyCode::Char(c)),
                ControlAction::ToggleMute(i as usize),
                format!("Toggle Mute Track {}", i + 1),
            ).category("Tracks"));
        }

        // Track solo (Shift + 1-8 produces !@#$%^&*)
        let shift_chars = ['!', '@', '#', '$', '%', '^', '&', '*'];
        for (i, &c) in shift_chars.iter().enumerate() {
            self.add(KeyBinding::new(
                Shortcut::key(KeyCode::Char(c)),
                ControlAction::ToggleSolo(i),
                format!("Toggle Solo Track {}", i + 1),
            ).category("Tracks"));
        }

        // Scene triggers (F1-F8)
        for i in 1..=8 {
            self.add(KeyBinding::new(
                Shortcut::key(KeyCode::F(i)),
                ControlAction::TriggerScene((i - 1) as usize),
                format!("Trigger Scene {}", i),
            ).category("Scenes"));
        }

        // Navigation
        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Left),
            ControlAction::NavigateLeft,
            "Navigate Left",
        ).category("Navigation"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Right),
            ControlAction::NavigateRight,
            "Navigate Right",
        ).category("Navigation"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Tab),
            ControlAction::NavigateRight,
            "Next",
        ).category("Navigation"));

        self.add(KeyBinding::new(
            Shortcut::shift(KeyCode::BackTab),
            ControlAction::NavigateLeft,
            "Previous",
        ).category("Navigation"));

        // UI
        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('?')),
            ControlAction::ToggleHelp,
            "Toggle Help",
        ).category("UI"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('h')),
            ControlAction::ToggleHelp,
            "Toggle Help",
        ).category("UI"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('l')),
            ControlAction::ToggleLearn,
            "Toggle MIDI Learn",
        ).category("UI"));

        self.add(KeyBinding::new(
            Shortcut::key(KeyCode::Char('q')),
            ControlAction::Quit,
            "Quit",
        ).category("UI"));

        self.add(KeyBinding::new(
            Shortcut::ctrl(KeyCode::Char('c')),
            ControlAction::Quit,
            "Quit",
        ).category("UI"));

        // Stop all
        self.add(KeyBinding::new(
            Shortcut::shift(KeyCode::Esc),
            ControlAction::StopAllClips,
            "Stop All Clips",
        ).category("Transport"));
    }

    /// Add a key binding
    pub fn add(&mut self, binding: KeyBinding) {
        self.bindings.insert(binding.shortcut.clone(), binding);
    }

    /// Remove a key binding
    pub fn remove(&mut self, shortcut: &Shortcut) -> Option<KeyBinding> {
        self.bindings.remove(shortcut)
    }

    /// Get action for a key event
    pub fn get_action(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<&ControlAction> {
        let shortcut = Shortcut::new(code, modifiers);
        self.bindings.get(&shortcut).map(|b| &b.action)
    }

    /// Process a key event and return the action
    pub fn process_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<ControlAction> {
        let shortcut = Shortcut::new(code, modifiers);

        if let Some(binding) = self.bindings.get(&shortcut) {
            Some(binding.action.clone())
        } else {
            None
        }
    }

    /// Handle key press (for repeat handling)
    pub fn key_down(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let shortcut = Shortcut::new(code, modifiers);
        self.held_keys.insert(shortcut, 0);
    }

    /// Handle key release
    pub fn key_up(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let shortcut = Shortcut::new(code, modifiers);
        self.held_keys.remove(&shortcut);
    }

    /// Update repeat timers and return actions for repeated keys
    pub fn update_repeats(&mut self) -> Vec<ControlAction> {
        let mut actions = Vec::new();

        for (shortcut, count) in self.held_keys.iter_mut() {
            *count += 1;

            if *count >= self.repeat_delay {
                let repeat_frame = (*count - self.repeat_delay) % self.repeat_rate;
                if repeat_frame == 0 {
                    if let Some(binding) = self.bindings.get(shortcut) {
                        // Only repeat certain action types
                        match &binding.action {
                            ControlAction::AdjustTempo(_)
                            | ControlAction::NavigateUp
                            | ControlAction::NavigateDown
                            | ControlAction::NavigateLeft
                            | ControlAction::NavigateRight => {
                                actions.push(binding.action.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        actions
    }

    /// Get all bindings for help display
    pub fn bindings(&self) -> impl Iterator<Item = &KeyBinding> {
        self.bindings.values()
    }

    /// Get bindings grouped by category
    pub fn bindings_by_category(&self) -> HashMap<String, Vec<&KeyBinding>> {
        let mut grouped: HashMap<String, Vec<&KeyBinding>> = HashMap::new();

        for binding in self.bindings.values() {
            grouped
                .entry(binding.category.clone())
                .or_default()
                .push(binding);
        }

        grouped
    }

    /// Get binding for a shortcut
    pub fn get_binding(&self, shortcut: &Shortcut) -> Option<&KeyBinding> {
        self.bindings.get(shortcut)
    }

    /// Set repeat parameters
    pub fn set_repeat(&mut self, delay_frames: u32, rate_frames: u32) {
        self.repeat_delay = delay_frames;
        self.repeat_rate = rate_frames.max(1);
    }
}

impl Default for KeyboardController {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Format a shortcut for display
pub fn format_shortcut(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();

    if shortcut.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if shortcut.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    if shortcut.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }

    let key = match shortcut.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        _ => "?".to_string(),
    };

    parts.push(&key);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_creation() {
        let s = Shortcut::key(KeyCode::Char('a'));
        assert_eq!(s.code, KeyCode::Char('a'));
        assert_eq!(s.modifiers, KeyModifiers::NONE);

        let s = Shortcut::ctrl(KeyCode::Char('c'));
        assert_eq!(s.modifiers, KeyModifiers::CONTROL);

        let s = Shortcut::shift(KeyCode::Up);
        assert_eq!(s.modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn test_shortcut_matches() {
        let s = Shortcut::ctrl(KeyCode::Char('c'));
        assert!(s.matches(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(!s.matches(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(!s.matches(KeyCode::Char('x'), KeyModifiers::CONTROL));
    }

    #[test]
    fn test_keyboard_controller_defaults() {
        let controller = KeyboardController::with_defaults();

        // Space should be play/pause
        let action = controller.get_action(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(action, Some(&ControlAction::TogglePlay));

        // Esc should be stop
        let action = controller.get_action(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(action, Some(&ControlAction::Stop));

        // Up should adjust tempo
        let action = controller.get_action(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(action, Some(&ControlAction::AdjustTempo(1.0)));
    }

    #[test]
    fn test_keyboard_controller_mute() {
        let controller = KeyboardController::with_defaults();

        // 1-8 should toggle mute
        for i in 1..=8 {
            let c = char::from_digit(i, 10).unwrap();
            let action = controller.get_action(KeyCode::Char(c), KeyModifiers::NONE);
            assert_eq!(action, Some(&ControlAction::ToggleMute((i - 1) as usize)));
        }
    }

    #[test]
    fn test_keyboard_controller_solo() {
        let controller = KeyboardController::with_defaults();

        // Shift+1 = ! should toggle solo track 1
        let action = controller.get_action(KeyCode::Char('!'), KeyModifiers::NONE);
        assert_eq!(action, Some(&ControlAction::ToggleSolo(0)));
    }

    #[test]
    fn test_add_remove_binding() {
        let mut controller = KeyboardController::new();

        let binding = KeyBinding::new(
            Shortcut::key(KeyCode::Char('x')),
            ControlAction::Stop,
            "Custom Stop",
        );

        controller.add(binding);
        assert!(controller.get_action(KeyCode::Char('x'), KeyModifiers::NONE).is_some());

        controller.remove(&Shortcut::key(KeyCode::Char('x')));
        assert!(controller.get_action(KeyCode::Char('x'), KeyModifiers::NONE).is_none());
    }

    #[test]
    fn test_format_shortcut() {
        let s = Shortcut::key(KeyCode::Char(' '));
        assert_eq!(format_shortcut(&s), "Space");

        let s = Shortcut::ctrl(KeyCode::Char('c'));
        assert_eq!(format_shortcut(&s), "Ctrl+C");

        let s = Shortcut::shift(KeyCode::Up);
        assert_eq!(format_shortcut(&s), "Shift+↑");

        let s = Shortcut::key(KeyCode::F(5));
        assert_eq!(format_shortcut(&s), "F5");
    }

    #[test]
    fn test_bindings_by_category() {
        let controller = KeyboardController::with_defaults();
        let grouped = controller.bindings_by_category();

        assert!(grouped.contains_key("Transport"));
        assert!(grouped.contains_key("Tempo"));
        assert!(grouped.contains_key("Tracks"));
    }

    #[test]
    fn test_process_key() {
        let mut controller = KeyboardController::with_defaults();

        let action = controller.process_key(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(action, Some(ControlAction::TogglePlay));

        let action = controller.process_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(action, None);
    }
}
