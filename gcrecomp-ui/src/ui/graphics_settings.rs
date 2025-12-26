// Graphics settings
use crate::app::Message;
use crate::config::GameConfig;
use iced::{
    widget::{Button, Checkbox, Column, Container, PickList, Row, Slider, Space, Text},
    Element, Length, Renderer, Theme,
};

pub struct GraphicsSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Native,
    Custom,
    Upscale2x,
    Upscale3x,
    Upscale4x,
}

impl Resolution {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Native,
            Self::Upscale2x,
            Self::Upscale3x,
            Self::Upscale4x,
            Self::Custom,
        ]
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "Native (640x480)"),
            Self::Upscale2x => write!(f, "2x (1280x960)"),
            Self::Upscale3x => write!(f, "3x (1920x1440)"),
            Self::Upscale4x => write!(f, "4x (2560x1920)"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

impl GraphicsSettings {
    pub fn view(_config: &GameConfig) -> Element<'static, Message> {
        let content = Column::new()
            .spacing(20)
            .push(Text::new("Graphics Settings").size(32))
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(Text::new("Resolution:"))
            .push(
                PickList::new(Resolution::all(), Some(Resolution::Native), |_| {})
                    .width(Length::Fixed(300.0)),
            )
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(Text::new("Upscaling:"))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Factor:"))
                    .push(Slider::new(1.0..=4.0, 1.0, |_| {}).width(Length::Fixed(200.0)))
                    .push(Text::new("1.0x")),
            )
            .push(Checkbox::new("Maintain Aspect Ratio", true))
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(Text::new("Texture Filtering:"))
            .push(
                PickList::new(
                    vec!["Nearest", "Linear", "Anisotropic"],
                    Some("Linear"),
                    |_| {},
                )
                .width(Length::Fixed(300.0)),
            )
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(Text::new("Anti-Aliasing:"))
            .push(
                PickList::new(
                    vec!["None", "MSAA 2x", "MSAA 4x", "FXAA"],
                    Some("None"),
                    |_| {},
                )
                .width(Length::Fixed(300.0)),
            )
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(Text::new("Performance:"))
            .push(Checkbox::new("VSync", false))
            .push(Checkbox::new("Triple Buffering", false))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Text::new("Frame Rate Limit:"))
                    .push(
                        PickList::new(
                            vec!["Unlimited", "60 FPS", "30 FPS"],
                            Some("Unlimited"),
                            |_| {},
                        )
                        .width(Length::Fixed(200.0)),
                    ),
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                Row::new()
                    .spacing(10)
                    .push(Button::new(Text::new("Apply")).width(Length::Fixed(150.0)))
                    .push(Button::new(Text::new("Reset to Default")).width(Length::Fixed(150.0))),
            )
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
