//! Enhanced gyro sensor support using hidapi for direct controller access
//!
//! Provides gyro data from controllers that support it (Switch Pro, DualSense, etc.)

use crate::input::gyro::{GyroData, dps_to_radps, g_to_ms2};
use anyhow::Result;
use std::time::Instant;

/// Gyro sensor reader using hidapi for direct HID access
pub struct GyroSensor {
    device: Option<hidapi::HidDevice>,
    vendor_id: u16,
    product_id: u16,
}

impl GyroSensor {
    /// Create a new gyro sensor reader
    pub fn new(vendor_id: u16, product_id: u16) -> Result<Self> {
        let api = hidapi::HidApi::new()?;
        let device = api.open(vendor_id, product_id).ok();

        Ok(Self {
            device,
            vendor_id,
            product_id,
        })
    }

    /// Read gyro data from the sensor
    pub fn read_gyro(&self) -> Option<GyroData> {
        let device = self.device.as_ref()?;

        // Read HID feature report or input report
        // This is controller-specific and would need implementation per controller type
        // For now, return None - requires controller-specific implementations
        
        // Example for Switch Pro Controller:
        // - Vendor ID: 0x057e (Nintendo)
        // - Product ID: 0x2009 (Pro Controller)
        // - Would read specific HID reports containing gyro data
        
        None
    }

    /// Check if sensor is available
    pub fn is_available(&self) -> bool {
        self.device.is_some()
    }
}

/// Controller-specific gyro implementations
pub mod controllers {
    use super::*;

    /// Nintendo Switch Pro Controller gyro support
    pub struct SwitchProGyro {
        sensor: GyroSensor,
    }

    impl SwitchProGyro {
        pub fn new() -> Result<Self> {
            // Switch Pro Controller: VID 0x057e, PID 0x2009
            let sensor = GyroSensor::new(0x057e, 0x2009)?;
            Ok(Self { sensor })
        }

        pub fn read_gyro(&self) -> Option<GyroData> {
            // Switch Pro Controller sends gyro data in HID input reports
            // Would need to parse the specific report format
            // For now, return None - requires full HID report parsing
            None
        }
    }

    /// Sony DualSense controller gyro support
    pub struct DualSenseGyro {
        sensor: GyroSensor,
    }

    impl DualSenseGyro {
        pub fn new() -> Result<Self> {
            // DualSense: VID 0x054c, PID 0x0ce6
            let sensor = GyroSensor::new(0x054c, 0x0ce6)?;
            Ok(Self { sensor })
        }

        pub fn read_gyro(&self) -> Option<GyroData> {
            // DualSense sends gyro data in HID input reports
            // Would need to parse the specific report format
            None
        }
    }
}

/// Auto-detect and create appropriate gyro sensor
pub fn create_gyro_sensor(vendor_id: u16, product_id: u16) -> Option<Box<dyn GyroReader>> {
    match (vendor_id, product_id) {
        (0x057e, 0x2009) => {
            // Switch Pro Controller
            controllers::SwitchProGyro::new()
                .ok()
                .map(|g| Box::new(g) as Box<dyn GyroReader>)
        }
        (0x054c, 0x0ce6) => {
            // DualSense
            controllers::DualSenseGyro::new()
                .ok()
                .map(|g| Box::new(g) as Box<dyn GyroReader>)
        }
        _ => None,
    }
}

/// Trait for reading gyro data
pub trait GyroReader: Send + Sync {
    fn read_gyro(&self) -> Option<GyroData>;
}

impl GyroReader for controllers::SwitchProGyro {
    fn read_gyro(&self) -> Option<GyroData> {
        self.read_gyro()
    }
}

impl GyroReader for controllers::DualSenseGyro {
    fn read_gyro(&self) -> Option<GyroData> {
        self.read_gyro()
    }
}

