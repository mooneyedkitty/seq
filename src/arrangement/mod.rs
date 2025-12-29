// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Song and arrangement system.
//!
//! This module provides:
//! - Parts: Collections of track clip/generator states
//! - Scenes: Track state snapshots with matrix triggering
//! - Song mode: Ordered arrangement playback

pub mod part;
pub mod scene;
pub mod song;

pub use part::{Part, PartManager, PartTransition, TrackClipState};
pub use scene::{Scene, SceneManager, SceneSlot};
pub use song::{Song, SongMode, SongPosition, SongSection};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_creation() {
        let part = Part::new("Verse");
        assert_eq!(part.name(), "Verse");
        assert!(part.track_states().is_empty());
    }

    #[test]
    fn test_scene_creation() {
        let scene = Scene::new("Scene A");
        assert_eq!(scene.name(), "Scene A");
    }

    #[test]
    fn test_song_creation() {
        let song = Song::new("My Song");
        assert_eq!(song.name(), "My Song");
        assert!(song.sections().is_empty());
    }
}
