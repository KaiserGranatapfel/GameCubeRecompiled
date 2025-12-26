// In-game menu UI
use iced::{Application, Settings};
use gcrecomp_ui::app::App;

fn main() -> iced::Result {
    App::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1280.0, 720.0),
            ..Default::default()
        },
        ..Default::default()
    })
}

