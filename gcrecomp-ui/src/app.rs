// Menu application state
use crate::config::GameConfig;
use crate::ui::main_menu::MainMenu;
use iced::{
    widget::{Button, Column, Container, Row, Text},
    Application, Command, Element, Length, Renderer, Settings, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    OpenFpsSettings,
    OpenGraphicsSettings,
    OpenAudioSettings,
    OpenInputSettings,
    OpenControllerConfig,
    OpenGameSettings,
    CloseMenu,
    ConfigChanged(GameConfig),
}

pub struct App {
    menu_visible: bool,
    current_screen: Screen,
    config: GameConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    MainMenu,
    FpsSettings,
    GraphicsSettings,
    AudioSettings,
    InputSettings,
    GameSettings,
    ControllerConfig,
}

impl Application for App {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let config = GameConfig::load().unwrap_or_default();
        (
            Self {
                menu_visible: false,
                current_screen: Screen::MainMenu,
                config,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "GCRecomp - Game Menu".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleMenu => {
                self.menu_visible = !self.menu_visible;
            }
            Message::OpenFpsSettings => {
                self.current_screen = Screen::FpsSettings;
            }
            Message::OpenGraphicsSettings => {
                self.current_screen = Screen::GraphicsSettings;
            }
            Message::OpenAudioSettings => {
                self.current_screen = Screen::AudioSettings;
            }
            Message::OpenInputSettings => {
                self.current_screen = Screen::InputSettings;
            }
            Message::OpenGameSettings => {
                self.current_screen = Screen::GameSettings;
            }
            Message::CloseMenu => {
                self.menu_visible = false;
                self.current_screen = Screen::MainMenu;
            }
            Message::ConfigChanged(config) => {
                self.config = config;
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        if !self.menu_visible {
            return Container::new(Text::new("Press ESC to open menu"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into();
        }

        let content = match self.current_screen {
            Screen::MainMenu => MainMenu::view(),
            Screen::FpsSettings => crate::ui::fps_settings::FpsSettings::view(&self.config),
            Screen::GraphicsSettings => {
                crate::ui::graphics_settings::GraphicsSettings::view(&self.config)
            }
            Screen::AudioSettings => crate::ui::audio_settings::AudioSettings::view(&self.config),
            Screen::InputSettings => crate::ui::input_settings::InputSettings::view(&self.config),
            Screen::ControllerConfig => {
                crate::ui::controller_config::ControllerConfigUI::view(&self.config)
            }
            Screen::GameSettings => crate::ui::game_settings::GameSettings::view(&self.config),
        };

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
