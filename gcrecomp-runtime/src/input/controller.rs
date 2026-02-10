// Controller detection and management
use crate::input::backends::{Backend, ControllerInfo};
use crate::input::gamecube_mapping::GameCubeMapping;
use crate::input::profiles::ControllerProfile;
use anyhow::Result;
use std::collections::HashMap;

pub struct ControllerManager {
    backends: Vec<Box<dyn Backend>>,
    controllers: HashMap<usize, ControllerState>,
    gamecube_mappings: HashMap<usize, GameCubeMapping>,
    profiles: HashMap<String, ControllerProfile>,
    _next_id: usize,
}

#[derive(Debug, Clone)]
pub struct ControllerState {
    pub id: usize,
    pub info: ControllerInfo,
    pub connected: bool,
    pub last_update: std::time::Instant,
}

impl ControllerManager {
    pub fn new() -> Result<Self> {
        let mut backends: Vec<Box<dyn Backend>> = Vec::new();

        // Try to initialize SDL2 backend
        if let Ok(sdl2_backend) = crate::input::backends::sdl2::SDL2Backend::new() {
            backends.push(Box::new(sdl2_backend));
        }

        // Try to initialize gilrs backend (cross-platform gamepad)
        if let Ok(gilrs_backend) = crate::input::backends::gilrs::GilrsBackend::new() {
            backends.push(Box::new(gilrs_backend));
        }

        #[cfg(target_os = "windows")]
        {
            // Try XInput backend on Windows
            if let Ok(xinput_backend) = crate::input::backends::xinput::XInputBackend::new() {
                backends.push(Box::new(xinput_backend));
            }
        }

        Ok(Self {
            backends,
            controllers: HashMap::new(),
            gamecube_mappings: HashMap::new(),
            profiles: HashMap::new(),
            _next_id: 0,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        // Update all backends and detect new/removed controllers
        // Collect controller IDs first to avoid borrow issues
        let mut all_controller_infos: Vec<ControllerInfo> = Vec::new();

        for backend in &mut self.backends {
            backend.update()?;
            all_controller_infos.extend(backend.enumerate_controllers()?);
        }

        // Check for new controllers
        for controller in &all_controller_infos {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.controllers.entry(controller.id)
            {
                let state = ControllerState {
                    id: controller.id,
                    info: controller.clone(),
                    connected: true,
                    last_update: std::time::Instant::now(),
                };
                entry.insert(state);

                // Load default profile or create new mapping
                self.load_default_mapping(controller.id)?;
            }
        }

        // Check for disconnected controllers
        let connected_ids: Vec<usize> = all_controller_infos.iter().map(|c| c.id).collect();

        self.controllers.retain(|id, state| {
            if !connected_ids.contains(id) {
                state.connected = false;
                false
            } else {
                true
            }
        });

        Ok(())
    }

    pub fn get_controller_count(&self) -> usize {
        self.controllers.values().filter(|c| c.connected).count()
    }

    pub fn get_controller_state(&self, id: usize) -> Option<&ControllerState> {
        self.controllers.get(&id)
    }

    pub fn get_gamecube_input(&self, controller_id: usize) -> Option<GameCubeInput> {
        let mapping = self.gamecube_mappings.get(&controller_id)?;

        // Get raw input from backend
        for backend in &self.backends {
            if let Ok(input) = backend.get_input(controller_id) {
                return Some(mapping.map_to_gamecube(&input));
            }
        }

        None
    }

    pub fn set_mapping(&mut self, controller_id: usize, mapping: GameCubeMapping) {
        self.gamecube_mappings.insert(controller_id, mapping);
    }

    pub fn load_profile(&mut self, controller_id: usize, profile_name: &str) -> Result<()> {
        if let Some(profile) = self.profiles.get(profile_name) {
            let mapping = profile.to_gamecube_mapping()?;
            self.set_mapping(controller_id, mapping);
        }
        Ok(())
    }

    pub fn save_profile(&mut self, name: String, controller_id: usize) -> Result<()> {
        if let Some(mapping) = self.gamecube_mappings.get(&controller_id) {
            let profile = ControllerProfile::from_mapping(name, mapping.clone());
            self.profiles.insert(profile.name.clone(), profile);
        }
        Ok(())
    }

    fn load_default_mapping(&mut self, controller_id: usize) -> Result<()> {
        // Try to detect controller type and load appropriate default
        if let Some(state) = self.controllers.get(&controller_id) {
            let default_mapping = GameCubeMapping::default_for_controller(&state.info)?;
            self.set_mapping(controller_id, default_mapping);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GameCubeInput {
    pub buttons: GameCubeButtons,
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Debug, Clone, Default)]
pub struct GameCubeButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub start: bool,
    pub d_up: bool,
    pub d_down: bool,
    pub d_left: bool,
    pub d_right: bool,
    pub l: bool,
    pub r: bool,
    pub z: bool,
}
