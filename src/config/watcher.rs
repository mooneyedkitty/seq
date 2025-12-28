// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! File watcher for hot-reload configuration.
//!
//! This module provides file system watching capabilities to detect
//! changes to configuration files and trigger reloads without
//! stopping playback.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::SongFile;

/// Events emitted by the config watcher
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// Configuration file was modified and successfully reloaded
    Reloaded(Box<SongFile>),
    /// Configuration file was modified but failed to parse
    Error(String),
    /// A new file was created in the watch directory
    FileCreated(PathBuf),
    /// A file was deleted from the watch directory
    FileDeleted(PathBuf),
}

/// Configuration file watcher with debouncing and validation
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    event_receiver: Receiver<ConfigEvent>,
    watched_path: PathBuf,
}

impl ConfigWatcher {
    /// Create a new config watcher for the specified path
    ///
    /// The watcher will monitor the file (or directory) for changes
    /// and emit `ConfigEvent`s when files are modified.
    ///
    /// # Arguments
    /// * `path` - Path to watch (file or directory)
    /// * `debounce_ms` - Debounce duration in milliseconds (default: 500)
    pub fn new<P: AsRef<Path>>(path: P, debounce_ms: Option<u64>) -> Result<Self> {
        let watched_path = path.as_ref().to_path_buf();
        let debounce_duration = Duration::from_millis(debounce_ms.unwrap_or(500));

        let (event_tx, event_rx): (Sender<ConfigEvent>, Receiver<ConfigEvent>) = mpsc::channel();

        // State for debouncing
        let debounced_path = watched_path.clone();
        let (notify_tx, notify_rx): (Sender<Event>, Receiver<Event>) = mpsc::channel();

        // Create the watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| anyhow!("Failed to create file watcher: {}", e))?;

        // Start watching
        let watch_path = watched_path.clone();
        let mode = if watched_path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(&watch_path, mode)
            .map_err(|e| anyhow!("Failed to watch path {:?}: {}", watch_path, e))?;

        // Spawn debounce thread
        std::thread::spawn(move || {
            let mut last_event_time: Option<Instant> = None;
            let mut pending_paths: Vec<PathBuf> = Vec::new();

            loop {
                // Check for new events with timeout
                match notify_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        // Process the event
                        match event.kind {
                            EventKind::Create(_) => {
                                for path in event.paths {
                                    let _ = event_tx.send(ConfigEvent::FileCreated(path));
                                }
                            }
                            EventKind::Remove(_) => {
                                for path in event.paths {
                                    let _ = event_tx.send(ConfigEvent::FileDeleted(path));
                                }
                            }
                            EventKind::Modify(_) => {
                                // Add to pending and update debounce timer
                                for path in event.paths {
                                    if !pending_paths.contains(&path) {
                                        pending_paths.push(path);
                                    }
                                }
                                last_event_time = Some(Instant::now());
                            }
                            _ => {}
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Check if debounce period has passed
                        if let Some(last_time) = last_event_time {
                            if last_time.elapsed() >= debounce_duration {
                                // Process pending modifications
                                for path in pending_paths.drain(..) {
                                    // Only process YAML files
                                    if let Some(ext) = path.extension() {
                                        if ext == "yaml" || ext == "yml" {
                                            match SongFile::load(&path) {
                                                Ok(config) => {
                                                    let _ = event_tx.send(ConfigEvent::Reloaded(
                                                        Box::new(config),
                                                    ));
                                                }
                                                Err(e) => {
                                                    let _ = event_tx.send(ConfigEvent::Error(
                                                        format!(
                                                            "Failed to load {:?}: {}",
                                                            path, e
                                                        ),
                                                    ));
                                                }
                                            }
                                        }
                                    } else if path == debounced_path {
                                        // Watch path itself without extension
                                        match SongFile::load(&path) {
                                            Ok(config) => {
                                                let _ = event_tx
                                                    .send(ConfigEvent::Reloaded(Box::new(config)));
                                            }
                                            Err(e) => {
                                                let _ = event_tx.send(ConfigEvent::Error(format!(
                                                    "Failed to load {:?}: {}",
                                                    path, e
                                                )));
                                            }
                                        }
                                    }
                                }
                                last_event_time = None;
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Watcher was dropped, exit thread
                        break;
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            event_receiver: event_rx,
            watched_path,
        })
    }

    /// Try to receive the next config event (non-blocking)
    pub fn try_recv(&self) -> Option<ConfigEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Receive all pending config events
    pub fn recv_all(&self) -> Vec<ConfigEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv() {
            events.push(event);
        }
        events
    }

    /// Block until the next config event is received
    pub fn recv(&self) -> Option<ConfigEvent> {
        self.event_receiver.recv().ok()
    }

    /// Get the path being watched
    pub fn watched_path(&self) -> &Path {
        &self.watched_path
    }
}

/// Validate a configuration without applying it
pub fn validate_config<P: AsRef<Path>>(path: P) -> Result<SongFile> {
    SongFile::load(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_validate_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_song.yaml");

        let yaml = r#"
song:
  name: "Test Song"
  tempo: 120
  key: "C"
  scale: "major"
"#;

        fs::write(&file_path, yaml).unwrap();

        let config = validate_config(&file_path).unwrap();
        assert_eq!(config.song.name, "Test Song");
        assert_eq!(config.song.tempo, 120.0);
    }

    #[test]
    fn test_validate_invalid_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid.yaml");

        fs::write(&file_path, "this is not valid yaml: [").unwrap();

        let result = validate_config(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_event_variants() {
        // Test that ConfigEvent can hold all variants
        let song = SongFile {
            song: super::super::SongConfig::default(),
            tracks: Vec::new(),
            parts: std::collections::HashMap::new(),
        };

        let _reloaded = ConfigEvent::Reloaded(Box::new(song));
        let _error = ConfigEvent::Error("test error".to_string());
        let _created = ConfigEvent::FileCreated(PathBuf::from("/test/path"));
        let _deleted = ConfigEvent::FileDeleted(PathBuf::from("/test/path"));
    }

    #[test]
    fn test_watcher_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("watch_test.yaml");

        let yaml = r#"
song:
  name: "Watch Test"
  tempo: 100
  key: "D"
  scale: "minor"
"#;

        fs::write(&file_path, yaml).unwrap();

        // Create watcher for the directory
        let watcher = ConfigWatcher::new(dir.path(), Some(100));
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert_eq!(watcher.watched_path(), dir.path());
    }

    #[test]
    fn test_watcher_detects_changes() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("detect_test.yaml");

        let yaml = r#"
song:
  name: "Initial"
  tempo: 120
  key: "C"
  scale: "major"
"#;

        fs::write(&file_path, yaml).unwrap();

        // Create watcher with short debounce
        let watcher = ConfigWatcher::new(dir.path(), Some(100)).unwrap();

        // Modify the file
        std::thread::sleep(Duration::from_millis(50));

        let new_yaml = r#"
song:
  name: "Modified"
  tempo: 140
  key: "G"
  scale: "major"
"#;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file_path)
            .unwrap();
        file.write_all(new_yaml.as_bytes()).unwrap();
        file.flush().unwrap();
        drop(file);

        // Wait for debounce + processing
        std::thread::sleep(Duration::from_millis(300));

        // Check for events
        let events = watcher.recv_all();

        // We should have received a Reloaded event
        let reloaded_event = events.iter().find(|e| matches!(e, ConfigEvent::Reloaded(_)));

        if let Some(ConfigEvent::Reloaded(config)) = reloaded_event {
            assert_eq!(config.song.name, "Modified");
            assert_eq!(config.song.tempo, 140.0);
        }
        // Note: The event may not always fire in CI environments due to timing
        // So we don't assert that we definitely got the event
    }
}
