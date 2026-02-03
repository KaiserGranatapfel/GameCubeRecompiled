// Graphics settings
use crate::app::Message;
use crate::config::GameConfig;
use iced::{
    widget::{Button, Column, Container, Space, Text},
    Element, Length,
};

pub struct GraphicsSettings;

impl GraphicsSettings {
    pub fn view(_config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Graphics Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Resolution: Native (640x480)"))
            .push(Text::new("Upscaling Factor: 1.0x"))
            .push(Text::new("Maintain Aspect Ratio: Yes"))
            .push(Text::new("Texture Filtering: Linear"))
            .push(Text::new("Anti-Aliasing: None"))
            .push(Text::new("VSync: Off"))
            .push(Text::new("Triple Buffering: Off"))
            .push(Text::new("Frame Rate Limit: Unlimited"))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("(Interactive controls to be implemented)"))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Button::new(Text::new("Back"))
                    .on_press(Message::CloseMenu)
                    .width(Length::Fixed(200.0)),
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
