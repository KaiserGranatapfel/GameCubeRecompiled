// FPS settings
use iced::{
    Element, Length, Renderer, Theme,
    widget::{Column, Container, Text, Button, Row, Space, Slider, Row as IcedRow},
};
use crate::app::Message;
use crate::config::GameConfig;

pub struct FpsSettings;

impl FpsSettings {
    pub fn view(config: &GameConfig) -> Element<'static, Message> {
        let fps_limit_text = config.fps_limit
            .map(|f| format!("{} FPS", f))
            .unwrap_or_else(|| "Unlimited".to_string());

        let content = Column::new()
            .spacing(20)
            .push(Text::new("FPS Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                IcedRow::new()
                    .spacing(10)
                    .push(Text::new("FPS Limit:"))
                    .push(Text::new(&fps_limit_text)),
            )
            .push(
                Button::new(Text::new("30 FPS"))
                    .on_press(Message::ConfigChanged({
                        let mut c = config.clone();
                        c.fps_limit = Some(30);
                        c
                    })),
            )
            .push(
                Button::new(Text::new("60 FPS"))
                    .on_press(Message::ConfigChanged({
                        let mut c = config.clone();
                        c.fps_limit = Some(60);
                        c
                    })),
            )
            .push(
                Button::new(Text::new("Unlimited"))
                    .on_press(Message::ConfigChanged({
                        let mut c = config.clone();
                        c.fps_limit = None;
                        c
                    })),
            )
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

