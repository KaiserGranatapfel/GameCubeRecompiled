// Main menu screen - Modern card-based design inspired by Dolphin/N64Recomp
use crate::app::Message;
use iced::{
    widget::{Button, Column, Container, Row, Space, Text},
    Element, Length, Renderer, Theme,
};

pub struct MainMenu;

impl MainMenu {
    pub fn view() -> Element<'static, Message> {
        // Main title
        let title = Text::new("GCRecomp")
            .size(48)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(0.6, 0.4, 0.9))); // Purple GameCube color

        // Settings cards in a grid-like layout
        let settings_row = Row::new()
            .spacing(15)
            .push(create_setting_card("Graphics", "Resolution, upscaling, post-processing", Message::OpenGraphicsSettings))
            .push(create_setting_card("Audio", "Volume, backend settings", Message::OpenAudioSettings))
            .push(create_setting_card("Input", "Controller configuration", Message::OpenInputSettings));

        let settings_row2 = Row::new()
            .spacing(15)
            .push(create_setting_card("FPS", "Frame rate limits", Message::OpenFpsSettings))
            .push(create_setting_card("Game", "Game-specific settings", Message::OpenGameSettings))
            .push(create_setting_card("Controller", "Controller mapping", Message::OpenControllerConfig));

        let menu = Column::new()
            .spacing(30)
            .push(title)
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(settings_row)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(settings_row2)
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(
                Button::new(Text::new("Close Menu (ESC)"))
                    .on_press(Message::CloseMenu)
                    .width(Length::Fixed(250.0))
                    .style(iced::theme::Button::Secondary),
            );

        Container::new(menu)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .center_x()
            .center_y()
            .into()
    }
}

fn create_setting_card(title: &str, description: &str, message: Message) -> Element<'static, Message> {
    Container::new(
        Column::new()
            .spacing(10)
            .push(Text::new(title).size(24))
            .push(Text::new(description).size(12).style(iced::theme::Text::Color(iced::Color::from_rgb(0.7, 0.7, 0.7))))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(
                Button::new(Text::new("Configure"))
                    .on_press(message)
                    .width(Length::Fill),
            )
    )
    .padding(20)
    .width(Length::Fixed(250.0))
    .style(iced::theme::Container::Box)
    .into()
}
