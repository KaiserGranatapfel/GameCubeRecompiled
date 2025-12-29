//! Performance monitoring and metrics
//!
//! Tracks FPS, frame time, memory usage, and render statistics

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Performance monitor that tracks various metrics
pub struct PerformanceMonitor {
    frame_times: VecDeque<f32>,
    max_samples: usize,
    last_frame_time: Option<Instant>,
    frame_count: u64,
    start_time: Instant,
    
    // Memory tracking
    ram_usage: usize,
    vram_usage: usize,
    
    // Render stats
    draw_calls: u32,
    triangles: u32,
    textures_loaded: u32,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(max_samples),
            max_samples,
            last_frame_time: None,
            frame_count: 0,
            start_time: Instant::now(),
            ram_usage: 0,
            vram_usage: 0,
            draw_calls: 0,
            triangles: 0,
            textures_loaded: 0,
        }
    }

    /// Record a frame
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        
        if let Some(last) = self.last_frame_time {
            let frame_time = (now - last).as_secs_f32() * 1000.0; // Convert to milliseconds
            self.frame_times.push_back(frame_time);
            
            if self.frame_times.len() > self.max_samples {
                self.frame_times.pop_front();
            }
        }
        
        self.last_frame_time = Some(now);
        self.frame_count += 1;
    }

    /// Get current FPS
    pub fn fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Get average frame time in milliseconds
    pub fn avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }

    /// Get minimum frame time in milliseconds
    pub fn min_frame_time(&self) -> f32 {
        self.frame_times.iter().copied().fold(f32::INFINITY, f32::min)
    }

    /// Get maximum frame time in milliseconds
    pub fn max_frame_time(&self) -> f32 {
        self.frame_times.iter().copied().fold(0.0, f32::max)
    }

    /// Get frame time history (for graphing)
    pub fn frame_time_history(&self) -> Vec<f32> {
        self.frame_times.iter().copied().collect()
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get uptime in seconds
    pub fn uptime(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    /// Set RAM usage in bytes
    pub fn set_ram_usage(&mut self, bytes: usize) {
        self.ram_usage = bytes;
    }

    /// Get RAM usage in bytes
    pub fn ram_usage(&self) -> usize {
        self.ram_usage
    }

    /// Set VRAM usage in bytes
    pub fn set_vram_usage(&mut self, bytes: usize) {
        self.vram_usage = bytes;
    }

    /// Get VRAM usage in bytes
    pub fn vram_usage(&self) -> usize {
        self.vram_usage
    }

    /// Set render statistics
    pub fn set_render_stats(&mut self, draw_calls: u32, triangles: u32, textures: u32) {
        self.draw_calls = draw_calls;
        self.triangles = triangles;
        self.textures_loaded = textures;
    }

    /// Get draw calls
    pub fn draw_calls(&self) -> u32 {
        self.draw_calls
    }

    /// Get triangle count
    pub fn triangles(&self) -> u32 {
        self.triangles
    }

    /// Get texture count
    pub fn textures(&self) -> u32 {
        self.textures_loaded
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.frame_times.clear();
        self.last_frame_time = None;
        self.frame_count = 0;
        self.start_time = Instant::now();
        self.ram_usage = 0;
        self.vram_usage = 0;
        self.draw_calls = 0;
        self.triangles = 0;
        self.textures_loaded = 0;
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(300) // 5 seconds at 60 FPS
    }
}

