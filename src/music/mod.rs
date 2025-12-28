// Copyright (c) 2026 Robert L. Snyder, Sierra Vista, AZ
// Licensed under the MIT License. See LICENSE file in the project root for details.

//! Music theory utilities for SEQ.
//!
//! This module provides scale definitions, key management, and note
//! manipulation utilities for algorithmic composition.

pub mod scale;

pub use scale::{Key, Note, Scale, ScaleType};
