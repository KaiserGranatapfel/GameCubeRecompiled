// In-game menu UI
use gcrecomp_ui::app::App;
use iced::{Application, Settings};

fn main() -> iced::Result {
    App::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1280.0, 720.0),
            ..Default::default()
        },
        ..Default::default()
    })
}
