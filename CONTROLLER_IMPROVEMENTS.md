# Controller Support Improvements

## Overview

Enhanced controller support with comprehensive button mapping, gyro support, and improved integration using existing Rust crates.

## Features Implemented

### 1. Enhanced Button Mapping System

**File:** `gcrecomp-runtime/src/input/button_mapper.rs`

- **Custom Button Mappings**: Map any controller button/axis/trigger to any GameCube button
- **Axis Mapping Configuration**: Configure stick axes with inversion, dead zones, and sensitivity
- **Trigger Mapping**: Configure trigger thresholds and dead zones
- **Input Detection**: Helper functions to detect which button/axis is currently active
- **Serialization**: Full JSON serialization support for saving/loading mappings

**Key Features:**
- Map buttons to buttons, axes, or triggers
- Configure dead zones per input
- Adjust sensitivity per stick
- Invert axes individually
- Save/load custom mappings

### 2. Gyro Support

**Files:**
- `gcrecomp-runtime/src/input/gyro.rs` (existing, enhanced)
- `gcrecomp-runtime/src/input/gyro_sensor.rs` (new)

**Gyro Features:**
- **Auto-calibration**: Automatically calibrates gyro on startup
- **Multiple Mapping Modes**:
  - Right Stick (aiming)
  - Left Stick (movement)
  - Both Sticks
  - Disabled
- **Sensitivity Control**: Adjustable from 0.0 to 2.0
- **Dead Zone**: Configurable dead zone to filter out small movements
- **Controller-Specific Support**: Framework for Switch Pro, DualSense, etc.

**Gyro Sensor Implementation:**
- Uses `hidapi` crate (already in dependencies) for direct HID access
- Framework for controller-specific implementations:
  - Nintendo Switch Pro Controller (VID: 0x057e, PID: 0x2009)
  - Sony DualSense (VID: 0x054c, PID: 0x0ce6)
- Extensible for additional controllers

### 3. Enhanced Controller Manager

**File:** `gcrecomp-runtime/src/input/controller.rs`

**New Features:**
- Button mapper integration
- Per-controller button mappers
- Auto-apply button mappers to mappings
- Enhanced gyro controller management
- Better profile integration

**API Additions:**
```rust
// Get/set button mapper
controller_manager.get_button_mapper(controller_id)
controller_manager.set_button_mapper(controller_id, mapper)

// Apply mapper to mapping
controller_manager.apply_button_mapper(controller_id)

// Gyro control
controller_manager.set_gyro_mapping_mode(controller_id, mode)
controller_manager.get_gyro_controller(controller_id)
```

### 4. Enhanced Profiles

**File:** `gcrecomp-runtime/src/input/profiles.rs`

**Improvements:**
- Button mapper integration in profiles
- Gyro settings in profiles (enabled, sensitivity, dead zone, mapping mode)
- Complete serialization support
- Save/load profiles with all settings

### 5. UI Components

**Files:**
- `gcrecomp-ui/src/ui/button_mapping.rs` (new)
- `gcrecomp-ui/src/ui/controller_config.rs` (enhanced)

**UI Features:**
- **Button Mapping Screen**: Dedicated screen for mapping buttons
- **Gyro Settings**: UI controls for gyro configuration
  - Enable/disable gyro
  - Sensitivity slider
  - Dead zone slider
  - Mapping mode selection (Right Stick, Left Stick, Both, Disabled)
- **Controller Config Integration**: Link to button mapping from controller config
- **Visual Feedback**: Clear indication of which buttons can be mapped

### 6. Backend Enhancements

**Files:**
- `gcrecomp-runtime/src/input/backends/sdl2.rs`
- `gcrecomp-runtime/src/input/backends/gilrs.rs`

**Improvements:**
- Better gyro data handling
- Framework for SDL2 sensor API integration
- Comments for future hidapi integration in gilrs backend

## Rust Crates Used

1. **gilrs** (v0.10) - Cross-platform gamepad support
2. **sdl2** (v0.35) - Cross-platform controller and sensor support
3. **hidapi** (v2.0) - Direct HID device access for gyro sensors
4. **serde** + **serde_json** - Profile and mapping serialization

## Usage Examples

### Button Mapping

```rust
use gcrecomp_runtime::input::{ButtonMapper, InputDetector};

// Create a button mapper
let mut mapper = ButtonMapper::new();

// Map GameCube A button to controller button 0
mapper.map_button("a", ButtonMapping::Button(0));

// Map left stick X axis with custom settings
mapper.map_axis("left_stick_x", AxisMappingConfig {
    axis_index: 0,
    invert: false,
    dead_zone: 0.15,
    sensitivity: 1.0,
});

// Apply to controller
controller_manager.set_button_mapper(controller_id, mapper);
```

### Gyro Configuration

```rust
// Enable gyro for right stick (aiming)
controller_manager.set_gyro_mapping_mode(controller_id, GyroMappingMode::RightStick);

// Configure gyro sensitivity
if let Some(gyro) = controller_manager.get_gyro_controller_mut(controller_id) {
    gyro.set_sensitivity(1.5); // 1.5x sensitivity
    gyro.set_dead_zone(0.01); // Small dead zone
    gyro.set_enabled(true);
}
```

### Profile Management

```rust
// Save a profile with button mapper and gyro settings
let profile = ControllerProfile::from_mapping_with_mapper(
    "My Profile".to_string(),
    mapping,
    button_mapper,
);
profile.gyro_enabled = true;
profile.gyro_sensitivity = 1.2;
profile.save_to_file(&path)?;

// Load profile
let profile = ControllerProfile::load_from_file(&path)?;
```

## Architecture

```
ControllerManager
├── Backends (SDL2, gilrs, XInput)
│   └── RawInput (buttons, axes, triggers, gyro)
├── ButtonMapper
│   └── Custom mappings → GameCubeMapping
├── GyroController
│   ├── Calibration
│   ├── Sensitivity/Dead Zone
│   └── Mapping Mode
└── Profiles
    ├── Button Mapper
    ├── Gyro Settings
    └── Serialization
```

## Future Enhancements

1. **Full HID Report Parsing**: Complete implementation of gyro sensor reading for Switch Pro and DualSense
2. **SDL2 Sensor API**: Full integration with SDL2's sensor subsystem
3. **Visual Controller Display**: Show controller visualization in UI
4. **Test Mode**: Real-time input testing in UI
5. **Vibration Support**: Haptic feedback support
6. **Multiple Profile Support**: Per-game profiles

## Testing

To test button mapping:
1. Open controller config in UI
2. Click "Button Mapping"
3. Click a button to remap
4. Press the desired controller button
5. Save the mapping

To test gyro:
1. Connect a gyro-enabled controller (Switch Pro, DualSense)
2. Open controller config
3. Enable gyro
4. Adjust sensitivity and dead zone
5. Select mapping mode (Right Stick recommended for aiming)

