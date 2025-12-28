pub mod gilrs;
pub mod sdl2;
#[cfg(target_os = "windows")]
pub mod xinput;

use crate::input::controller::GameCubeInput;
use anyhow::Result;

pub trait Backend: Send + Sync {
    fn update(&mut self) -> Result<()>;
    fn enumerate_controllers(&self) -> Result<Vec<ControllerInfo>>;
    fn get_input(&self, controller_id: usize) -> Result<RawInput>;
}

#[derive(Debug, Clone)]
pub struct ControllerInfo {
    pub id: usize,
    pub name: String,
    pub controller_type: ControllerType,
    pub button_count: usize,
    pub axis_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerType {
    Xbox,
    PlayStation,
    SwitchPro,
    Generic,
    Keyboard,
}

#[derive(Debug, Clone)]
pub struct RawInput {
    pub buttons: Vec<bool>,
    pub axes: Vec<f32>,
    pub triggers: Vec<f32>,
    pub hat: Option<HatState>,
    /// Gyro data (if available)
    pub gyro: Option<crate::input::gyro::GyroData>,
}

#[derive(Debug, Clone)]
pub struct HatState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}
