// Audio settings
use crate::app::Message;
use crate::config::GameConfig;
use iced::{
    widget::{Button, Column, Container, Space, Text},
    Element, Length,
};

pub struct AudioSettings;

impl AudioSettings {
    pub fn view(config: &GameConfig) -> Element<'static, Message> {
        let master_vol = format!("Master Volume: {:.0}%", config.master_volume * 100.0);
        let music_vol = format!("Music Volume: {:.0}%", config.music_volume * 100.0);
        let sfx_vol = format!("SFX Volume: {:.0}%", config.sfx_volume * 100.0);
        let audio_backend = format!("Audio Backend: {}", config.audio_backend);

        let content = Column::new()
            .spacing(20)
            .push(Text::new("Audio Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new(master_vol))
            .push(Text::new(music_vol))
            .push(Text::new(sfx_vol))
            .push(Text::new(audio_backend))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Button::new(Text::new("Back")).on_press(Message::CloseMenu));

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
