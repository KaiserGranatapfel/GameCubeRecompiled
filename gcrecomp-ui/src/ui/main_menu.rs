// Main menu screen
use crate::app::Message;
use iced::{
    widget::{Button, Column, Container, Row, Space, Text},
    Element, Length, Renderer, Theme,
};

pub struct MainMenu;

impl MainMenu {
    pub fn view() -> Element<'static, Message> {
        let menu = Column::new()
            .spacing(20)
            .push(Text::new("Game Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Button::new(Text::new("FPS Settings"))
                    .on_press(Message::OpenFpsSettings)
                    .width(Length::Fixed(200.0)),
            )
            .push(
                Button::new(Text::new("Graphics Settings"))
                    .on_press(Message::OpenGraphicsSettings)
                    .width(Length::Fixed(200.0)),
            )
            .push(
                Button::new(Text::new("Audio Settings"))
                    .on_press(Message::OpenAudioSettings)
                    .width(Length::Fixed(200.0)),
            )
            .push(
                Button::new(Text::new("Input Settings"))
                    .on_press(Message::OpenInputSettings)
                    .width(Length::Fixed(200.0)),
            )
            .push(
                Button::new(Text::new("Game Settings"))
                    .on_press(Message::OpenGameSettings)
                    .width(Length::Fixed(200.0)),
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Button::new(Text::new("Close Menu (ESC)"))
                    .on_press(Message::CloseMenu)
                    .width(Length::Fixed(200.0)),
            );

        Container::new(menu)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
