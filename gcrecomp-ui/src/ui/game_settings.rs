// Game settings
use iced::{
    Element, Length, Renderer, Theme,
    widget::{Column, Container, Text, Button, Space},
};
use crate::app::Message;
use crate::config::GameConfig;

pub struct GameSettings;

impl GameSettings {
    pub fn view(_config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Game Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Save/Load game"))
            .push(Text::new("Cheats/Mods"))
            .push(Text::new("(To be implemented)"))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Button::new(Text::new("Back"))
                    .on_press(Message::CloseMenu),
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

