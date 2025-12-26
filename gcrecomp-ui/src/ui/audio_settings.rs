// Audio settings
use iced::{
    Element, Length, Renderer, Theme,
    widget::{Column, Container, Text, Button, Space},
};
use crate::app::Message;
use crate::config::GameConfig;

pub struct AudioSettings;

impl AudioSettings {
    pub fn view(config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Audio Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new(&format!("Master Volume: {:.0}%", config.master_volume * 100.0)))
            .push(Text::new(&format!("Music Volume: {:.0}%", config.music_volume * 100.0)))
            .push(Text::new(&format!("SFX Volume: {:.0}%", config.sfx_volume * 100.0)))
            .push(Text::new(&format!("Audio Backend: {}", config.audio_backend)))
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

