//! Audio system for GameCube recompiler
//!
//! This module provides audio/DSP emulation for GameCube games.
//! It includes:
//! - DSP (Digital Signal Processor) emulation
//! - AI (Audio Interface) stream processing
//! - Audio output integration

pub mod dsp;
pub mod ai;

pub use dsp::DSP;
pub use ai::AudioInterface;

