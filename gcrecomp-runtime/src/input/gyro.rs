//! Gyro control implementation
//! 
//! Heavily influenced by and adapted from N64Recomp:
//! https://github.com/N64Recomp/N64Recomp
//! 
//! Translated to Rust with GameCube-specific adaptations.
//! 
//! This module provides gyroscope and accelerometer support for motion controls,
//! including calibration, sensitivity adjustment, and motion-to-stick mapping.

use anyhow::Result;
use std::time::{Duration, Instant};

/// Gyro sensor data from controller
#[derive(Debug, Clone, Copy)]
pub struct GyroData {
    /// Angular velocity in radians per second (X, Y, Z)
    pub angular_velocity: (f32, f32, f32),
    /// Acceleration in m/s² (X, Y, Z)
    pub acceleration: (f32, f32, f32),
    /// Timestamp of the reading
    pub timestamp: Instant,
}

impl Default for GyroData {
    fn default() -> Self {
        Self {
            angular_velocity: (0.0, 0.0, 0.0),
            acceleration: (0.0, 0.0, 0.0),
            timestamp: Instant::now(),
        }
    }
}

/// Gyro calibration state
#[derive(Debug, Clone)]
pub struct GyroCalibration {
    /// Offset values for angular velocity (X, Y, Z)
    pub offset: (f32, f32, f32),
    /// Number of samples collected
    pub sample_count: usize,
    /// Whether calibration is complete
    pub calibrated: bool,
}

impl Default for GyroCalibration {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0, 0.0),
            sample_count: 0,
            calibrated: false,
        }
    }
}

/// Gyro controller with calibration and motion-to-stick mapping
pub struct GyroController {
    /// Current gyro data
    current_data: GyroData,
    /// Previous gyro data for delta calculation
    previous_data: GyroData,
    /// Calibration state
    calibration: GyroCalibration,
    /// Sensitivity multiplier (0.0 to 1.0+)
    sensitivity: f32,
    /// Dead zone threshold (radians per second)
    dead_zone: f32,
    /// Whether gyro is enabled
    enabled: bool,
    /// Motion-to-stick mapping mode
    mapping_mode: GyroMappingMode,
}

/// Gyro motion-to-stick mapping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GyroMappingMode {
    /// Map gyro directly to right stick (aiming)
    RightStick,
    /// Map gyro to left stick (movement)
    LeftStick,
    /// Map gyro to both sticks
    BothSticks,
    /// Disabled
    Disabled,
}

impl Default for GyroController {
    fn default() -> Self {
        Self {
            current_data: GyroData::default(),
            previous_data: GyroData::default(),
            calibration: GyroCalibration::default(),
            sensitivity: 1.0,
            dead_zone: 0.01, // ~0.57 degrees per second
            enabled: true,
            mapping_mode: GyroMappingMode::RightStick,
        }
    }
}

impl GyroController {
    /// Create a new gyro controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Update gyro data from sensor reading
    pub fn update(&mut self, data: GyroData) {
        self.previous_data = self.current_data;
        self.current_data = data;

        // Auto-calibrate if not calibrated
        if !self.calibration.calibrated {
            self.add_calibration_sample(data);
        }
    }

    /// Add a sample for calibration
    fn add_calibration_sample(&mut self, data: GyroData) {
        const CALIBRATION_SAMPLES: usize = 100;
        
        // Accumulate offset
        self.calibration.offset.0 += data.angular_velocity.0;
        self.calibration.offset.1 += data.angular_velocity.1;
        self.calibration.offset.2 += data.angular_velocity.2;
        self.calibration.sample_count += 1;

        if self.calibration.sample_count >= CALIBRATION_SAMPLES {
            // Calculate average offset
            let count = self.calibration.sample_count as f32;
            self.calibration.offset.0 /= count;
            self.calibration.offset.1 /= count;
            self.calibration.offset.2 /= count;
            self.calibration.calibrated = true;
        }
    }

    /// Start calibration (reset and begin collecting samples)
    pub fn start_calibration(&mut self) {
        self.calibration = GyroCalibration::default();
    }

    /// Get calibrated angular velocity
    fn get_calibrated_velocity(&self) -> (f32, f32, f32) {
        if !self.calibration.calibrated {
            return (0.0, 0.0, 0.0);
        }

        (
            self.current_data.angular_velocity.0 - self.calibration.offset.0,
            self.current_data.angular_velocity.1 - self.calibration.offset.1,
            self.current_data.angular_velocity.2 - self.calibration.offset.2,
        )
    }

    /// Apply dead zone to a value
    fn apply_dead_zone(&self, value: f32) -> f32 {
        if value.abs() < self.dead_zone {
            0.0
        } else {
            // Smooth transition from dead zone
            let sign = if value >= 0.0 { 1.0 } else { -1.0 };
            let magnitude = (value.abs() - self.dead_zone) / (1.0 - self.dead_zone);
            sign * magnitude
        }
    }

    /// Convert gyro motion to stick input
    /// Returns (x, y) stick values in range [-1.0, 1.0]
    pub fn get_stick_input(&self) -> (f32, f32) {
        if !self.enabled || self.mapping_mode == GyroMappingMode::Disabled {
            return (0.0, 0.0);
        }

        let (vx, vy, _vz) = self.get_calibrated_velocity();

        // Apply dead zone
        let vx = self.apply_dead_zone(vx);
        let vy = self.apply_dead_zone(vy);

        // Apply sensitivity
        let vx = vx * self.sensitivity;
        let vy = vy * self.sensitivity;

        // Convert angular velocity to stick input
        // Scale factor: radians/sec to stick value
        // Typical gyro range: ±4.0 rad/s, map to full stick range
        const SCALE_FACTOR: f32 = 0.25; // 1.0 rad/s = 0.25 stick value
        
        let x = (vx * SCALE_FACTOR).clamp(-1.0, 1.0);
        let y = (vy * SCALE_FACTOR).clamp(-1.0, 1.0);

        // Invert Y axis for typical camera controls
        (x, -y)
    }

    /// Get gyro input for right stick (aiming)
    pub fn get_right_stick_input(&self) -> (f32, f32) {
        match self.mapping_mode {
            GyroMappingMode::RightStick | GyroMappingMode::BothSticks => {
                self.get_stick_input()
            }
            _ => (0.0, 0.0),
        }
    }

    /// Get gyro input for left stick (movement)
    pub fn get_left_stick_input(&self) -> (f32, f32) {
        match self.mapping_mode {
            GyroMappingMode::LeftStick | GyroMappingMode::BothSticks => {
                self.get_stick_input()
            }
            _ => (0.0, 0.0),
        }
    }

    /// Set sensitivity (0.0 to 2.0, default 1.0)
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.clamp(0.0, 2.0);
    }

    /// Get current sensitivity
    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    /// Set dead zone threshold in radians per second
    pub fn set_dead_zone(&mut self, dead_zone: f32) {
        self.dead_zone = dead_zone.max(0.0);
    }

    /// Get current dead zone
    pub fn dead_zone(&self) -> f32 {
        self.dead_zone
    }

    /// Enable or disable gyro
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if gyro is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set mapping mode
    pub fn set_mapping_mode(&mut self, mode: GyroMappingMode) {
        self.mapping_mode = mode;
    }

    /// Get current mapping mode
    pub fn mapping_mode(&self) -> GyroMappingMode {
        self.mapping_mode
    }

    /// Check if calibration is complete
    pub fn is_calibrated(&self) -> bool {
        self.calibration.calibrated
    }

    /// Get calibration progress (0.0 to 1.0)
    pub fn calibration_progress(&self) -> f32 {
        if self.calibration.calibrated {
            1.0
        } else {
            (self.calibration.sample_count as f32 / 100.0).min(1.0)
        }
    }
}

/// Helper to convert raw sensor data to GyroData
pub fn raw_to_gyro_data(
    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
) -> GyroData {
    GyroData {
        angular_velocity: (gyro_x, gyro_y, gyro_z),
        acceleration: (accel_x, accel_y, accel_z),
        timestamp: Instant::now(),
    }
}

/// Helper to convert degrees per second to radians per second
pub fn dps_to_radps(dps: f32) -> f32 {
    dps * std::f32::consts::PI / 180.0
}

/// Helper to convert G-force to m/s²
pub fn g_to_ms2(g: f32) -> f32 {
    g * 9.80665
}

