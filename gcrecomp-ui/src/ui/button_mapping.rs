//! Button mapping UI
//!
//! Provides a user-friendly interface for mapping controller buttons to GameCube buttons

use crate::app::Message;
use crate::config::GameConfig;
use iced::{
    widget::{Button, Column, Container, Row, Space, Text},
    Element, Length, Renderer, Theme,
};

pub struct ButtonMappingUI;

#[derive(Debug, Clone)]
pub enum ButtonMappingMessage {
    StartMapping(String), // GameCube button name
    CancelMapping,
    SaveMapping,
    ResetMapping,
}

impl ButtonMappingUI {
    pub fn view(config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Button Mapping").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Click a button below to remap it").size(16))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(create_button_row("A", "B", "X", "Y"))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(create_button_row("Start", "D-Pad Up", "D-Pad Down", "D-Pad Left"))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(create_button_row("D-Pad Right", "L", "R", "Z"))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Sticks & Triggers").size(24))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(create_axis_row("Left Stick", "Right Stick"))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(create_axis_row("Left Trigger", "Right Trigger"))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Button::new(Text::new("Save")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Reset")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Back"))
                        .on_press(Message::CloseMenu)
                        .width(Length::Fixed(150.0))),
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .center_x()
            .center_y()
            .into()
    }
}

fn create_button_row(btn1: &str, btn2: &str, btn3: &str, btn4: &str) -> Element<'static, Message> {
    Row::new()
        .spacing(10)
        .push(create_mappable_button(btn1))
        .push(create_mappable_button(btn2))
        .push(create_mappable_button(btn3))
        .push(create_mappable_button(btn4))
        .into()
}

fn create_axis_row(axis1: &str, axis2: &str) -> Element<'static, Message> {
    Row::new()
        .spacing(10)
        .push(create_mappable_button(axis1))
        .push(create_mappable_button(axis2))
        .into()
}

fn create_mappable_button(label: &str) -> Element<'static, Message> {
    Button::new(Text::new(label))
        .width(Length::Fixed(150.0))
        .into()
}

