// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Parameter system with smoothing and automation.
//!
//! Provides a registry of named parameters with configurable ranges,
//! value smoothing, and optional automation support.

use std::collections::HashMap;

/// Parameter value with optional smoothing
#[derive(Debug, Clone)]
pub struct ParameterValue {
    /// Current value (after smoothing)
    current: f64,
    /// Target value
    target: f64,
    /// Smoothing coefficient (0.0 = instant, 1.0 = never reaches target)
    smoothing: f64,
}

impl ParameterValue {
    /// Create a new parameter value
    pub fn new(value: f64) -> Self {
        Self {
            current: value,
            target: value,
            smoothing: 0.0,
        }
    }

    /// Create with smoothing
    pub fn with_smoothing(value: f64, smoothing: f64) -> Self {
        Self {
            current: value,
            target: value,
            smoothing: smoothing.clamp(0.0, 0.999),
        }
    }

    /// Get current value
    pub fn current(&self) -> f64 {
        self.current
    }

    /// Get target value
    pub fn target(&self) -> f64 {
        self.target
    }

    /// Set target value
    pub fn set(&mut self, value: f64) {
        self.target = value;
        // If no smoothing, update current immediately
        if self.smoothing == 0.0 {
            self.current = value;
        }
    }

    /// Set value immediately (bypassing smoothing)
    pub fn set_immediate(&mut self, value: f64) {
        self.current = value;
        self.target = value;
    }

    /// Set smoothing coefficient
    pub fn set_smoothing(&mut self, smoothing: f64) {
        self.smoothing = smoothing.clamp(0.0, 0.999);
    }

    /// Update value with smoothing (call once per frame)
    pub fn update(&mut self, delta_time: f64) {
        if self.smoothing == 0.0 {
            self.current = self.target;
        } else {
            // Exponential smoothing
            let factor = 1.0 - self.smoothing.powf(delta_time * 60.0);
            self.current += (self.target - self.current) * factor;

            // Snap to target when close enough
            if (self.current - self.target).abs() < 0.0001 {
                self.current = self.target;
            }
        }
    }

    /// Check if value is still transitioning
    pub fn is_transitioning(&self) -> bool {
        (self.current - self.target).abs() > 0.0001
    }
}

impl Default for ParameterValue {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Default value
    pub default: f64,
    /// Current value
    pub value: ParameterValue,
    /// Unit string (e.g., "Hz", "dB", "%")
    pub unit: String,
    /// Number of decimal places for display
    pub precision: u8,
    /// Parameter group/category
    pub group: String,
    /// Whether parameter is exposed for MIDI control
    pub midi_controllable: bool,
}

impl Parameter {
    /// Create a new parameter
    pub fn new(name: impl Into<String>, min: f64, max: f64, default: f64) -> Self {
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            min,
            max,
            default,
            value: ParameterValue::new(default),
            unit: String::new(),
            precision: 2,
            group: "General".to_string(),
            midi_controllable: true,
        }
    }

    /// Set display name
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    /// Set unit
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Set precision
    pub fn precision(mut self, precision: u8) -> Self {
        self.precision = precision;
        self
    }

    /// Set group
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Set MIDI controllable
    pub fn midi_controllable(mut self, controllable: bool) -> Self {
        self.midi_controllable = controllable;
        self
    }

    /// Set smoothing
    pub fn smoothing(mut self, smoothing: f64) -> Self {
        self.value.set_smoothing(smoothing);
        self
    }

    /// Get current value
    pub fn get(&self) -> f64 {
        self.value.current()
    }

    /// Get normalized value (0.0 - 1.0)
    pub fn get_normalized(&self) -> f64 {
        if self.max == self.min {
            0.0
        } else {
            (self.value.current() - self.min) / (self.max - self.min)
        }
    }

    /// Set value (clamped to range)
    pub fn set(&mut self, value: f64) {
        self.value.set(value.clamp(self.min, self.max));
    }

    /// Set normalized value (0.0 - 1.0)
    pub fn set_normalized(&mut self, value: f64) {
        let value = value.clamp(0.0, 1.0);
        self.set(self.min + value * (self.max - self.min));
    }

    /// Adjust value by delta
    pub fn adjust(&mut self, delta: f64) {
        self.set(self.value.target() + delta);
    }

    /// Adjust normalized value by delta
    pub fn adjust_normalized(&mut self, delta: f64) {
        self.set_normalized(self.get_normalized() + delta);
    }

    /// Reset to default
    pub fn reset(&mut self) {
        self.set(self.default);
    }

    /// Format value for display
    pub fn format(&self) -> String {
        let value = self.value.current();
        if self.unit.is_empty() {
            format!("{:.prec$}", value, prec = self.precision as usize)
        } else {
            format!("{:.prec$} {}", value, self.unit, prec = self.precision as usize)
        }
    }

    /// Update smoothing
    pub fn update(&mut self, delta_time: f64) {
        self.value.update(delta_time);
    }
}

/// Registry of parameters
pub struct ParameterRegistry {
    /// Parameters by name
    params: HashMap<String, Parameter>,
    /// Order of parameters for iteration
    order: Vec<String>,
}

impl ParameterRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Register a parameter
    pub fn register(&mut self, param: Parameter) {
        let name = param.name.clone();
        self.params.insert(name.clone(), param);
        if !self.order.contains(&name) {
            self.order.push(name);
        }
    }

    /// Get a parameter by name
    pub fn get(&self, name: &str) -> Option<&Parameter> {
        self.params.get(name)
    }

    /// Get a mutable parameter by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Parameter> {
        self.params.get_mut(name)
    }

    /// Set parameter value by name
    pub fn set(&mut self, name: &str, value: f64) -> bool {
        if let Some(param) = self.params.get_mut(name) {
            param.set(value);
            true
        } else {
            false
        }
    }

    /// Set parameter normalized value by name
    pub fn set_normalized(&mut self, name: &str, value: f64) -> bool {
        if let Some(param) = self.params.get_mut(name) {
            param.set_normalized(value);
            true
        } else {
            false
        }
    }

    /// Adjust parameter by name
    pub fn adjust(&mut self, name: &str, delta: f64) -> bool {
        if let Some(param) = self.params.get_mut(name) {
            param.adjust(delta);
            true
        } else {
            false
        }
    }

    /// Get parameter value by name
    pub fn value(&self, name: &str) -> Option<f64> {
        self.params.get(name).map(|p| p.get())
    }

    /// Get parameter normalized value by name
    pub fn value_normalized(&self, name: &str) -> Option<f64> {
        self.params.get(name).map(|p| p.get_normalized())
    }

    /// Update all parameter smoothing
    pub fn update(&mut self, delta_time: f64) {
        for param in self.params.values_mut() {
            param.update(delta_time);
        }
    }

    /// Iterate over all parameters in order
    pub fn iter(&self) -> impl Iterator<Item = &Parameter> {
        self.order.iter().filter_map(|name| self.params.get(name))
    }

    /// Iterate over parameters in a group
    pub fn iter_group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = &'a Parameter> {
        self.params.values().filter(move |p| p.group == group)
    }

    /// Get all group names
    pub fn groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = self
            .params
            .values()
            .map(|p| p.group.clone())
            .collect();
        groups.sort();
        groups.dedup();
        groups
    }

    /// Get MIDI-controllable parameters
    pub fn midi_controllable(&self) -> impl Iterator<Item = &Parameter> {
        self.params.values().filter(|p| p.midi_controllable)
    }

    /// Number of parameters
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Clear all parameters
    pub fn clear(&mut self) {
        self.params.clear();
        self.order.clear();
    }

    /// Reset all parameters to defaults
    pub fn reset_all(&mut self) {
        for param in self.params.values_mut() {
            param.reset();
        }
    }
}

impl Default for ParameterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Common parameter presets
pub mod presets {
    use super::Parameter;

    /// Create a tempo parameter
    pub fn tempo() -> Parameter {
        Parameter::new("tempo", 20.0, 300.0, 120.0)
            .display_name("Tempo")
            .unit("BPM")
            .precision(1)
            .group("Transport")
            .smoothing(0.5)
    }

    /// Create a volume parameter (0-1)
    pub fn volume(name: impl Into<String>) -> Parameter {
        let name = name.into();
        Parameter::new(&name, 0.0, 1.0, 0.8)
            .display_name(format!("{} Volume", name))
            .precision(2)
            .group("Mixer")
            .smoothing(0.8)
    }

    /// Create a pan parameter (-1 to 1)
    pub fn pan(name: impl Into<String>) -> Parameter {
        let name = name.into();
        Parameter::new(&name, -1.0, 1.0, 0.0)
            .display_name(format!("{} Pan", name))
            .precision(2)
            .group("Mixer")
            .smoothing(0.8)
    }

    /// Create a filter cutoff parameter (Hz)
    pub fn filter_cutoff() -> Parameter {
        Parameter::new("filter_cutoff", 20.0, 20000.0, 1000.0)
            .display_name("Filter Cutoff")
            .unit("Hz")
            .precision(0)
            .group("Filter")
            .smoothing(0.9)
    }

    /// Create a filter resonance parameter
    pub fn filter_resonance() -> Parameter {
        Parameter::new("filter_resonance", 0.0, 1.0, 0.0)
            .display_name("Resonance")
            .precision(2)
            .group("Filter")
            .smoothing(0.9)
    }

    /// Create a swing parameter
    pub fn swing() -> Parameter {
        Parameter::new("swing", 0.0, 1.0, 0.0)
            .display_name("Swing")
            .unit("%")
            .precision(0)
            .group("Timing")
    }

    /// Create a probability parameter
    pub fn probability(name: impl Into<String>) -> Parameter {
        let name = name.into();
        Parameter::new(&name, 0.0, 1.0, 1.0)
            .display_name(format!("{} Probability", name))
            .unit("%")
            .precision(0)
            .group("Generator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_value_immediate() {
        let mut value = ParameterValue::new(0.5);
        assert_eq!(value.current(), 0.5);

        value.set_immediate(1.0);
        assert_eq!(value.current(), 1.0);
        assert_eq!(value.target(), 1.0);
    }

    #[test]
    fn test_parameter_value_smoothing() {
        let mut value = ParameterValue::with_smoothing(0.0, 0.9);
        value.set(1.0);

        // Should not be at target immediately
        assert_eq!(value.current(), 0.0);
        assert_eq!(value.target(), 1.0);

        // Update should move towards target
        value.update(1.0 / 60.0);
        assert!(value.current() > 0.0);
        assert!(value.current() < 1.0);
        assert!(value.is_transitioning());

        // Many updates should approach target
        for _ in 0..100 {
            value.update(1.0 / 60.0);
        }
        assert!((value.current() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_parameter_range() {
        let mut param = Parameter::new("test", 0.0, 100.0, 50.0);

        param.set(150.0);
        assert_eq!(param.get(), 100.0); // Clamped to max

        param.set(-10.0);
        assert_eq!(param.get(), 0.0); // Clamped to min
    }

    #[test]
    fn test_parameter_normalized() {
        let mut param = Parameter::new("test", 0.0, 100.0, 50.0);

        assert!((param.get_normalized() - 0.5).abs() < 0.01);

        param.set_normalized(0.0);
        assert_eq!(param.get(), 0.0);

        param.set_normalized(1.0);
        assert_eq!(param.get(), 100.0);

        param.set_normalized(0.25);
        assert!((param.get() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_parameter_adjust() {
        let mut param = Parameter::new("test", 0.0, 100.0, 50.0);

        param.adjust(10.0);
        assert_eq!(param.value.target(), 60.0);

        param.adjust(-20.0);
        assert_eq!(param.value.target(), 40.0);

        // Should clamp
        param.adjust(-100.0);
        assert_eq!(param.value.target(), 0.0);
    }

    #[test]
    fn test_parameter_format() {
        let param = Parameter::new("tempo", 20.0, 300.0, 120.0)
            .unit("BPM")
            .precision(1);

        assert_eq!(param.format(), "120.0 BPM");
    }

    #[test]
    fn test_registry() {
        let mut registry = ParameterRegistry::new();

        registry.register(Parameter::new("tempo", 20.0, 300.0, 120.0));
        registry.register(Parameter::new("volume", 0.0, 1.0, 0.8));

        assert_eq!(registry.len(), 2);

        assert!(registry.set("tempo", 140.0));
        assert_eq!(registry.value("tempo"), Some(140.0));

        assert!(!registry.set("unknown", 0.0));
    }

    #[test]
    fn test_registry_groups() {
        let mut registry = ParameterRegistry::new();

        registry.register(Parameter::new("tempo", 20.0, 300.0, 120.0).group("Transport"));
        registry.register(Parameter::new("swing", 0.0, 1.0, 0.0).group("Timing"));
        registry.register(Parameter::new("volume", 0.0, 1.0, 0.8).group("Mixer"));

        let groups = registry.groups();
        assert!(groups.contains(&"Transport".to_string()));
        assert!(groups.contains(&"Timing".to_string()));
        assert!(groups.contains(&"Mixer".to_string()));
    }

    #[test]
    fn test_presets() {
        let tempo = presets::tempo();
        assert_eq!(tempo.name, "tempo");
        assert_eq!(tempo.min, 20.0);
        assert_eq!(tempo.max, 300.0);
        assert_eq!(tempo.default, 120.0);

        let volume = presets::volume("Master");
        assert_eq!(volume.name, "Master");
        assert_eq!(volume.group, "Mixer");
    }

    #[test]
    fn test_registry_reset() {
        let mut registry = ParameterRegistry::new();

        registry.register(Parameter::new("test", 0.0, 100.0, 50.0));
        registry.set("test", 75.0);
        assert_eq!(registry.value("test"), Some(75.0));

        registry.reset_all();
        assert_eq!(registry.value("test"), Some(50.0));
    }
}
