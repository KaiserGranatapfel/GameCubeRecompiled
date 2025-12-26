// Nintendo Switch Pro Controller support
use anyhow::Result;
use hidapi::HidApi;
use std::time::Duration;

pub struct SwitchProController {
    device: Option<hidapi::HidDevice>,
    connected: bool,
}

impl SwitchProController {
    pub fn new() -> Result<Self> {
        let api = HidApi::new()?;

        // Switch Pro Controller USB vendor/product IDs
        const NINTENDO_VENDOR_ID: u16 = 0x057e;
        const PRO_CONTROLLER_PRODUCT_ID: u16 = 0x2009;

        let device = api.open(NINTENDO_VENDOR_ID, PRO_CONTROLLER_PRODUCT_ID).ok();

        Ok(Self {
            device,
            connected: device.is_some(),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn read_input(&mut self) -> Result<SwitchProInput> {
        if let Some(ref device) = self.device {
            let mut buf = [0u8; 64];
            let len = device.read_timeout(&mut buf, Duration::from_millis(16))?;

            if len > 0 {
                return Self::parse_input(&buf[..len]);
            }
        }

        Ok(SwitchProInput::default())
    }

    fn parse_input(data: &[u8]) -> Result<SwitchProInput> {
        if data.len() < 10 {
            return Ok(SwitchProInput::default());
        }

        let buttons = u16::from_le_bytes([data[3], data[4]]);

        Ok(SwitchProInput {
            a: (buttons & 0x0001) != 0,
            b: (buttons & 0x0002) != 0,
            x: (buttons & 0x0004) != 0,
            y: (buttons & 0x0008) != 0,
            minus: (buttons & 0x0010) != 0,
            plus: (buttons & 0x0020) != 0,
            l: (buttons & 0x0040) != 0,
            r: (buttons & 0x0080) != 0,
            zl: (buttons & 0x0100) != 0,
            zr: (buttons & 0x0200) != 0,
            left_stick_x: data[6] as f32 / 128.0 - 1.0,
            left_stick_y: data[7] as f32 / 128.0 - 1.0,
            right_stick_x: data[8] as f32 / 128.0 - 1.0,
            right_stick_y: data[9] as f32 / 128.0 - 1.0,
        })
    }

    pub fn set_rumble(&mut self, low_freq: u8, high_freq: u8) -> Result<()> {
        if let Some(ref device) = self.device {
            let mut buf = [0u8; 10];
            buf[0] = 0x10; // Rumble command
            buf[1] = 0x80;
            buf[3] = low_freq;
            buf[4] = high_freq;
            device.write(&buf)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwitchProInput {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub minus: bool,
    pub plus: bool,
    pub l: bool,
    pub r: bool,
    pub zl: bool,
    pub zr: bool,
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,
}
