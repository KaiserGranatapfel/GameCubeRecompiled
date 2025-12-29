//! Performance stats overlay
//!
//! Displays real-time FPS, frame time graph, and memory usage

use crate::app::Message;
use iced::{
    widget::{Canvas, Column, Container, Row, Text},
    Element, Length, Renderer, Theme,
};

pub struct PerformanceStats;

impl PerformanceStats {
    pub fn view(fps: f32, frame_time: f32, ram_usage_mb: f32, vram_usage_mb: f32) -> Element<'static, Message> {
        let stats = Column::new()
            .spacing(5)
            .push(Text::new(format!("FPS: {:.1}", fps)).size(16))
            .push(Text::new(format!("Frame Time: {:.2} ms", frame_time)).size(14))
            .push(Text::new(format!("RAM: {:.1} MB", ram_usage_mb)).size(14))
            .push(Text::new(format!("VRAM: {:.1} MB", vram_usage_mb)).size(14));

        Container::new(stats)
            .padding(10)
            .style(iced::theme::Container::Box)
            .into()
    }
}

