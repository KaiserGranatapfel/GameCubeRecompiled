// Menu application state — renders Lua-defined screens via Iced
use crate::config::GameConfig;
use gcrecomp_lua::bindings::ui::{LuaScreenDef, LuaWidget, LUA_SCREENS, NAV_STACK};
use iced::{
    widget::{Button, Checkbox, Column, Container, PickList, Row, Slider, Space, Text, TextInput},
    Application, Command, Element, Length, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    CloseMenu,
    ConfigChanged(GameConfig),
    NavigateTo(String),
    GoBack,
    LuaWidgetClicked(String, String),
    LuaSliderChanged(String, String, f64),
    LuaCheckboxToggled(String, String, bool),
    LuaPickListSelected(String, String, String),
    LuaTextInputChanged(String, String, String),
}

pub struct App {
    menu_visible: bool,
    config: GameConfig,
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
            Message::CloseMenu => {
                self.menu_visible = false;
                if let Ok(mut stack) = NAV_STACK.lock() {
                    stack.clear();
                }
            }
            Message::ConfigChanged(config) => {
                self.config = config;
                if let Err(e) = self.config.save() {
                    eprintln!("Failed to save config: {}", e);
                }
            }
            Message::NavigateTo(screen_id) => {
                if let Ok(mut stack) = NAV_STACK.lock() {
                    stack.push(screen_id);
                }
            }
            Message::GoBack => {
                if let Ok(mut stack) = NAV_STACK.lock() {
                    stack.pop();
                }
            }
            Message::LuaWidgetClicked(_screen_id, _widget_id) => {
                // Callback invocation handled by the game loop
            }
            Message::LuaSliderChanged(screen_id, widget_id, value) => {
                if let Ok(mut screens) = LUA_SCREENS.lock() {
                    if let Some(screen) = screens.iter_mut().find(|s| s.id == screen_id) {
                        if let Some(widget) = screen.widgets.iter_mut().find(|w| w.id == widget_id)
                        {
                            widget.value = Some(serde_json::json!(value));
                        }
                    }
                }
            }
            Message::LuaCheckboxToggled(screen_id, widget_id, value) => {
                if let Ok(mut screens) = LUA_SCREENS.lock() {
                    if let Some(screen) = screens.iter_mut().find(|s| s.id == screen_id) {
                        if let Some(widget) = screen.widgets.iter_mut().find(|w| w.id == widget_id)
                        {
                            widget.value = Some(serde_json::Value::Bool(value));
                        }
                    }
                }
            }
            Message::LuaPickListSelected(screen_id, widget_id, value) => {
                if let Ok(mut screens) = LUA_SCREENS.lock() {
                    if let Some(screen) = screens.iter_mut().find(|s| s.id == screen_id) {
                        if let Some(widget) = screen.widgets.iter_mut().find(|w| w.id == widget_id)
                        {
                            widget.value = Some(serde_json::Value::String(value));
                        }
                    }
                }
            }
            Message::LuaTextInputChanged(screen_id, widget_id, value) => {
                if let Ok(mut screens) = LUA_SCREENS.lock() {
                    if let Some(screen) = screens.iter_mut().find(|s| s.id == screen_id) {
                        if let Some(widget) = screen.widgets.iter_mut().find(|w| w.id == widget_id)
                        {
                            widget.value = Some(serde_json::Value::String(value));
                        }
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if !self.menu_visible {
            return Container::new(Text::new("Press ESC to open menu"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into();
        }

        // Determine which screen to show from the nav stack
        let current_screen_id = NAV_STACK
            .lock()
            .ok()
            .and_then(|stack| stack.last().cloned());

        let content: Element<Message> = if let Some(screen_id) = current_screen_id {
            // Render a Lua-defined screen
            let screens = LUA_SCREENS.lock().ok();
            if let Some(ref screens) = screens {
                if let Some(screen) = screens.iter().find(|s| s.id == screen_id) {
                    render_lua_screen(screen)
                } else {
                    Column::new()
                        .push(Text::new(format!("Screen '{}' not found", screen_id)))
                        .push(Button::new(Text::new("Back")).on_press(Message::GoBack))
                        .into()
                }
            } else {
                Text::new("Error loading screens").into()
            }
        } else {
            // Show main menu: list all registered Lua screens
            render_main_menu()
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

/// Render the main menu with navigation to all registered Lua screens.
fn render_main_menu() -> Element<'static, Message> {
    let mut menu = Column::new()
        .spacing(20)
        .push(Text::new("Game Settings").size(32))
        .push(Space::with_height(Length::Fixed(20.0)));

    if let Ok(screens) = LUA_SCREENS.lock() {
        for screen in screens.iter() {
            let id = screen.id.clone();
            menu = menu.push(
                Button::new(Text::new(screen.title.clone()))
                    .on_press(Message::NavigateTo(id))
                    .width(Length::Fixed(250.0)),
            );
        }
    }

    menu = menu.push(Space::with_height(Length::Fixed(20.0))).push(
        Button::new(Text::new("Close Menu (ESC)"))
            .on_press(Message::CloseMenu)
            .width(Length::Fixed(250.0)),
    );

    Container::new(menu)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
}

/// Render a Lua-defined screen by mapping LuaWidgets to Iced widgets.
fn render_lua_screen(screen: &LuaScreenDef) -> Element<'static, Message> {
    let screen_id = screen.id.clone();
    let mut col = Column::new().spacing(15);

    // Title
    col = col.push(Text::new(screen.title.clone()).size(28));
    col = col.push(Space::with_height(Length::Fixed(10.0)));

    for widget in &screen.widgets {
        col = col.push(render_lua_widget(&screen_id, widget));
    }

    // Back button
    col = col.push(Space::with_height(Length::Fixed(20.0)));
    col = col.push(Button::new(Text::new("Back")).on_press(Message::GoBack));

    col.into()
}

/// Map a single LuaWidget to an Iced Element.
fn render_lua_widget(screen_id: &str, widget: &LuaWidget) -> Element<'static, Message> {
    let sid = screen_id.to_string();
    let wid = widget.id.clone();

    match widget.widget_type.as_str() {
        "label" | "text" => {
            let text = widget.text.as_deref().unwrap_or("");
            let size = widget
                .style
                .as_ref()
                .and_then(|s| s.font_size)
                .unwrap_or(16.0);
            Text::new(text.to_string()).size(size as u16).into()
        }
        "button" => {
            let text = widget.text.as_deref().unwrap_or("Button");
            let sid2 = sid.clone();
            let wid2 = wid.clone();
            let mut btn = Button::new(Text::new(text.to_string()));
            let enabled = widget.enabled.unwrap_or(true);
            if enabled {
                if let Some(ref on_click) = widget.on_click {
                    let _ = on_click; // Callback name stored for the Lua callback system
                    btn = btn.on_press(Message::LuaWidgetClicked(sid2, wid2));
                } else {
                    btn = btn.on_press(Message::LuaWidgetClicked(sid2, wid2));
                }
            }
            if let Some(ref style) = widget.style {
                if let Some(w) = style.width {
                    btn = btn.width(Length::Fixed(w));
                }
            }
            btn.into()
        }
        "slider" => {
            let label = widget.label.as_deref().unwrap_or("");
            let min = widget.min.unwrap_or(0.0) as f32;
            let max = widget.max.unwrap_or(100.0) as f32;
            let current = widget
                .value
                .as_ref()
                .and_then(|v| v.as_f64())
                .unwrap_or(min as f64) as f32;
            let sid2 = sid.clone();
            let wid2 = wid.clone();

            Row::new()
                .spacing(10)
                .push(Text::new(label.to_string()).width(Length::Fixed(150.0)))
                .push(
                    Slider::new(min..=max, current, move |v| {
                        Message::LuaSliderChanged(sid2.clone(), wid2.clone(), v as f64)
                    })
                    .width(Length::Fixed(200.0)),
                )
                .push(Text::new(format!("{:.0}", current)))
                .into()
        }
        "checkbox" | "toggle" => {
            let label = widget.label.as_deref().unwrap_or("");
            let checked = widget
                .value
                .as_ref()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sid2 = sid.clone();
            let wid2 = wid.clone();

            Checkbox::new(label.to_string(), checked)
                .on_toggle(move |v| Message::LuaCheckboxToggled(sid2.clone(), wid2.clone(), v))
                .into()
        }
        "dropdown" | "picklist" => {
            let label = widget.label.as_deref().unwrap_or("");
            let options = widget.options.clone().unwrap_or_default();
            let selected = widget
                .value
                .as_ref()
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let sid2 = sid.clone();
            let wid2 = wid.clone();

            Row::new()
                .spacing(10)
                .push(Text::new(label.to_string()).width(Length::Fixed(150.0)))
                .push(
                    PickList::new(options, selected, move |v| {
                        Message::LuaPickListSelected(sid2.clone(), wid2.clone(), v)
                    })
                    .width(Length::Fixed(200.0)),
                )
                .into()
        }
        "text_input" => {
            let label = widget.label.as_deref().unwrap_or("");
            let current = widget
                .value
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let sid2 = sid.clone();
            let wid2 = wid.clone();

            Row::new()
                .spacing(10)
                .push(Text::new(label.to_string()).width(Length::Fixed(150.0)))
                .push(
                    TextInput::new("", &current)
                        .on_input(move |v| {
                            Message::LuaTextInputChanged(sid2.clone(), wid2.clone(), v)
                        })
                        .width(Length::Fixed(200.0)),
                )
                .into()
        }
        "spacer" | "separator" => {
            let height = widget.style.as_ref().and_then(|s| s.height).unwrap_or(10.0);
            Space::with_height(Length::Fixed(height)).into()
        }
        "row" => {
            let mut row = Row::new().spacing(
                widget
                    .style
                    .as_ref()
                    .and_then(|s| s.spacing)
                    .unwrap_or(10.0) as u16,
            );
            if let Some(ref children) = widget.children {
                for child in children {
                    row = row.push(render_lua_widget(screen_id, child));
                }
            }
            row.into()
        }
        "column" => {
            let mut col = Column::new().spacing(
                widget
                    .style
                    .as_ref()
                    .and_then(|s| s.spacing)
                    .unwrap_or(10.0) as u16,
            );
            if let Some(ref children) = widget.children {
                for child in children {
                    col = col.push(render_lua_widget(screen_id, child));
                }
            }
            col.into()
        }
        _ => Text::new(format!("[unknown widget type: {}]", widget.widget_type)).into(),
    }
}
