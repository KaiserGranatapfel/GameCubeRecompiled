// Cemu-like controller mapping UI
use iced::{
    Element, Length, Renderer, Theme,
    widget::{Column, Container, Text, Button, Row, Space, Slider, Checkbox},
};
use crate::app::Message;
use crate::config::GameConfig;

pub struct ControllerConfigUI {
    selected_controller: Option<usize>,
    mapping_mode: bool,
    test_mode: bool,
}

impl ControllerConfigUI {
    pub fn new() -> Self {
        Self {
            selected_controller: None,
            mapping_mode: false,
            test_mode: false,
        }
    }
    
    pub fn view(config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Controller Configuration").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Select Controller:"))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Button::new(Text::new("Controller 1")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Controller 2")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Controller 3")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Controller 4")).width(Length::Fixed(150.0)))
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Visual Controller Display"))
            .push(
                Container::new(Text::new("Controller visualization would appear here"))
                    .width(Length::Fixed(400.0))
                    .height(Length::Fixed(300.0))
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Button Mapping:"))
            .push(
                Column::new()
                    .spacing(10)
                    .push(create_mapping_row("A Button", "Click to map"))
                    .push(create_mapping_row("B Button", "Click to map"))
                    .push(create_mapping_row("X Button", "Click to map"))
                    .push(create_mapping_row("Y Button", "Click to map"))
                    .push(create_mapping_row("Start", "Click to map"))
                    .push(create_mapping_row("L Trigger", "Click to map"))
                    .push(create_mapping_row("R Trigger", "Click to map"))
                    .push(create_mapping_row("Z Button", "Click to map"))
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Advanced Settings:"))
            .push(
                Column::new()
                    .spacing(10)
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(Text::new("Left Stick Dead Zone:"))
                            .push(Slider::new(0.0..=1.0, 0.15, |_| {}).width(Length::Fixed(200.0)))
                    )
                    .push(
                        Row::new()
                            .spacing(10)
                            .push(Text::new("Right Stick Dead Zone:"))
                            .push(Slider::new(0.0..=1.0, 0.15, |_| {}).width(Length::Fixed(200.0)))
                    )
                    .push(Checkbox::new("Invert Y Axis", false))
                    .push(Checkbox::new("Enable Vibration", true))
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Button::new(Text::new("Test Mode")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Save Profile")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Load Profile")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Reset to Default")).width(Length::Fixed(150.0)))
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Button::new(Text::new("Back"))
                    .on_press(Message::CloseMenu)
                    .width(Length::Fixed(200.0))
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

fn create_mapping_row(label: &str, button_text: &str) -> Row<'static, Message> {
    Row::new()
        .spacing(10)
        .push(Text::new(label).width(Length::Fixed(100.0)))
        .push(Button::new(Text::new(button_text)).width(Length::Fixed(200.0)))
}

