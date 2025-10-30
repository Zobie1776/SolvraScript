//=============================================
// solvra_shell_launcher/src/main.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Solvra GUI desktop surface
// Objective: Provide multi-window desktop, task bar, and panels using iced
//=============================================

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::pane_grid::{self, Axis, Content, Pane, State as PaneGridState, TitleBar};
use iced::widget::{button, column, container, row, text, Space};
use iced::{executor, Alignment, Application, Color, Command, Element, Length, Renderer, Settings};
use theme_engine::{ThemeDocument, ThemeTokens};

//=============================================
// SECTION 1: Desktop Application State
//=============================================

/// Top-level iced application driving the Solvra desktop UI.
struct DesktopApp {
    tokens: ThemeTokens,
    layout: LayoutInfo,
    style: StyleCatalog,
    windows: WindowsState,
    taskbar: TaskBarState,
}

/// Messages emitted by UI interactions.
#[derive(Debug, Clone)]
enum Message {
    TaskInvoked(WindowKind),
    PaneClosed(Pane),
    PaneFocused(Pane),
    /// Placeholder for timed refreshes (clock, animations).
    Tick,
}

impl Application for DesktopApp {
    type Executor = executor::Default;
    type Flags = ThemeTokens;
    type Message = Message;
    type Theme = iced::Theme;

    fn new(tokens: Self::Flags) -> (Self, Command<Message>) {
        utils::logging::init("desktop");

        let style = StyleCatalog::new(tokens.clone());
        let windows = WindowsState::new();
        let taskbar = TaskBarState::new();

        (
            Self {
                tokens,
                layout: LayoutInfo::default(),
                style,
                windows,
                taskbar,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Solvra Desktop".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TaskInvoked(kind) => {
                if self.windows.toggle(kind) {
                    self.taskbar.set_active(Some(kind));
                } else {
                    self.taskbar.set_active(None);
                }
            }
            Message::PaneClosed(pane) => {
                if let Some(closed_kind) = self.windows.close(pane) {
                    if self.taskbar.active == Some(closed_kind) {
                        self.taskbar.set_active(None);
                    }
                }
            }
            Message::PaneFocused(pane) => {
                if let Some(kind) = self.windows.kind_for(pane) {
                    self.taskbar.set_active(Some(kind));
                    self.windows.focus(pane);
                }
            }
            Message::Tick => {
                // Reserved for clock updates / animations.
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let pane_grid = self.windows.view(&self.style);
        let taskbar = self.taskbar.view(&self.style);

        let desktop_surface = column![pane_grid, taskbar]
            .spacing(self.style.spacing_large)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_items(Alignment::Start);

        container(desktop_surface)
            .padding(self.style.spacing_large)
            .style(iced::theme::Container::Custom(Box::new(
                self.style.desktop_container(),
            )))
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(30)).map(|_| Message::Tick)
    }
}

//=============================================
// SECTION 2: Layout, Styling, and Theme Helpers
//=============================================

/// Tracks responsive layout breakpoints.
#[derive(Debug, Clone)]
struct LayoutInfo {
    width: u32,
    height: u32,
    compact: bool,
}

impl Default for LayoutInfo {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            compact: false,
        }
    }
}

/// Provides computed styles derived from theme tokens.
#[derive(Debug, Clone)]
struct StyleCatalog {
    tokens: ThemeTokens,
    accent_primary: Color,
    accent_secondary: Color,
    text_primary: Color,
    text_subtle: Color,
    background: Color,
    panel_background: Color,
    spacing_small: u16,
    spacing_medium: u16,
    spacing_large: u16,
    corner_radius: f32,
}

#[derive(Clone)]
struct ContainerStyle(iced::widget::container::Appearance);

impl iced::widget::container::StyleSheet for ContainerStyle {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        self.0.clone()
    }
}

impl StyleCatalog {
    fn new(tokens: ThemeTokens) -> Self {
        let accent_primary = parse_hex(&tokens.colors.accent);
        let accent_secondary = parse_hex(&tokens.colors.primary);
        let text_primary = Color::WHITE;
        let text_subtle = Color {
            a: 0.75,
            ..Color::WHITE
        };
        let background = parse_hex(&tokens.colors.surface);
        let panel_background = parse_hex(&tokens.colors.overlay);

        Self {
            spacing_small: 6,
            spacing_medium: 12,
            spacing_large: 20,
            corner_radius: tokens.effects.corner_radius as f32,
            tokens,
            accent_primary,
            accent_secondary,
            text_primary,
            text_subtle,
            background,
            panel_background,
        }
    }

    fn desktop_container(&self) -> ContainerStyle {
        let mut appearance = iced::widget::container::Appearance::default();
        appearance.background = Some(self.background.into());
        appearance.border_radius = self.corner_radius;
        ContainerStyle(appearance)
    }

    fn pane_container(&self) -> ContainerStyle {
        let mut appearance = iced::widget::container::Appearance::default();
        appearance.background = Some(self.panel_background.into());
        appearance.border_radius = self.corner_radius;
        appearance.border_width = 1.0;
        appearance.border_color = self.accent_secondary;
        ContainerStyle(appearance)
    }

    fn text_primary_style(&self) -> iced::theme::Text {
        iced::theme::Text::Color(self.text_primary)
    }

    fn text_subtle_style(&self) -> iced::theme::Text {
        iced::theme::Text::Color(self.text_subtle)
    }

    fn task_button_style(
        &self,
        active: bool,
    ) -> impl Fn(&Renderer, iced::widget::button::Status) -> iced::widget::button::Appearance + '_
    {
        let accent = self.accent_primary;
        let radius = self.corner_radius;
        move |_, status| {
            let mut appearance = iced::widget::button::Appearance {
                background: Some(
                    if active {
                        accent
                    } else {
                        Color { a: 0.15, ..accent }
                    }
                    .into(),
                ),
                border_radius: radius,
                text_color: Color::WHITE,
                ..Default::default()
            };
            if matches!(status, iced::widget::button::Status::Hovered) {
                appearance.background = Some(Color { a: 0.25, ..accent }.into());
            }
            appearance
        }
    }
}

//=============================================
// SECTION 3: Task Bar Implementation
//=============================================

#[derive(Debug, Clone)]
struct TaskBarState {
    buttons: Vec<TaskDefinition>,
    active: Option<WindowKind>,
}

impl TaskBarState {
    fn new() -> Self {
        Self {
            buttons: vec![
                TaskDefinition::new("IDE", "", WindowKind::Ide),
                TaskDefinition::new("CLI", "", WindowKind::Cli),
                TaskDefinition::new("App Store", "", WindowKind::AppStore),
                TaskDefinition::new("Settings", "", WindowKind::Settings),
            ],
            active: None,
        }
    }

    fn set_active(&mut self, active: Option<WindowKind>) {
        self.active = active;
    }

    fn view(&self, style: &StyleCatalog) -> Element<Message> {
        let buttons = self.buttons.iter().fold(row![], |row, button| {
            let active = self.active == Some(button.kind);
            let label = column![
                text(button.icon)
                    .style(style.text_primary_style())
                    .size(26)
                    .horizontal_alignment(Horizontal::Center),
                text(button.label)
                    .style(style.text_subtle_style())
                    .size(12)
                    .horizontal_alignment(Horizontal::Center),
            ]
            .spacing(4)
            .width(Length::Fixed(80.0))
            .align_items(Alignment::Center);

            row.push(
                button::Button::new(label)
                    .style(style.task_button_style(active))
                    .width(Length::Shrink)
                    .on_press(Message::TaskInvoked(button.kind)),
            )
            .spacing(style.spacing_medium)
        });

        container(
            row![
                buttons.align_items(Alignment::Center),
                Space::with_width(Length::Fill),
                text("Solvra OS")
                    .style(style.text_subtle_style())
                    .horizontal_alignment(Horizontal::Right)
            ]
            .align_items(Alignment::Center),
        )
        .padding([style.spacing_small, style.spacing_large])
        .style(iced::theme::Container::Custom(Box::new(
            style.pane_container(),
        )))
        .into()
    }
}

#[derive(Debug, Clone)]
struct TaskDefinition {
    label: &'static str,
    icon: &'static str,
    kind: WindowKind,
}

impl TaskDefinition {
    fn new(label: &'static str, icon: &'static str, kind: WindowKind) -> Self {
        Self { label, icon, kind }
    }
}

//=============================================
// SECTION 4: Window Management State
//=============================================

#[derive(Debug)]
struct WindowsState {
    panes: PaneGridState<WindowEntry>,
    index_by_kind: HashMap<WindowKind, Pane>,
    next_id: u16,
}

impl WindowsState {
    fn new() -> Self {
        let (panes, root) = PaneGridState::new(WindowEntry::new(WindowKind::Overview));

        let mut index_by_kind = HashMap::new();
        index_by_kind.insert(WindowKind::Overview, root);

        Self {
            panes,
            index_by_kind,
            next_id: 1,
        }
    }

    fn view(&self, style: &StyleCatalog) -> pane_grid::PaneGrid<'_, Message> {
        pane_grid::PaneGrid::new(&self.panes, move |pane, entry| {
            let title = entry.title();
            let controls = row![
                button::Button::new(text("—"))
                    .style(style.task_button_style(false))
                    .padding([2, 6])
                    .on_press(Message::PaneFocused(pane)),
                button::Button::new(text("×"))
                    .style(style.task_button_style(false))
                    .padding([2, 6])
                    .on_press(Message::PaneClosed(pane)),
            ]
            .spacing(style.spacing_small);

            let title_bar = TitleBar::new(text(title).size(18).style(style.text_primary_style()))
                .controls(controls);

            Content::new(entry.view(style)).title_bar(title_bar)
        })
        .height(Length::Fill)
        .width(Length::Fill)
        .spacing(style.spacing_medium)
        .on_click(Message::PaneFocused)
        .on_close(Message::PaneClosed)
    }

    fn toggle(&mut self, kind: WindowKind) -> bool {
        if let Some(&pane) = self.index_by_kind.get(&kind) {
            self.focus(pane);
            true
        } else {
            let pane = self.open(kind);
            self.focus(pane);
            true
        }
    }

    fn open(&mut self, kind: WindowKind) -> Pane {
        let entry = WindowEntry::with_id(self.next_id, kind);
        self.next_id += 1;

        let anchor = self.index_by_kind[&WindowKind::Overview];
        let (_, new_pane) = self.panes.split(Axis::Horizontal, anchor, entry);
        self.index_by_kind.insert(kind, new_pane);
        new_pane
    }

    fn close(&mut self, pane: Pane) -> Option<WindowKind> {
        let closed_kind = self
            .index_by_kind
            .iter()
            .find_map(|(kind, &stored)| (stored == pane).then_some(*kind));

        if self.panes.close(pane) {
            if let Some(kind) = closed_kind {
                self.index_by_kind.remove(&kind);
                return Some(kind);
            }
        }
        None
    }

    fn focus(&mut self, pane: Pane) {
        self.panes.focus(pane);
    }

    fn kind_for(&self, pane: Pane) -> Option<WindowKind> {
        self.index_by_kind
            .iter()
            .find_map(|(kind, &stored)| (stored == pane).then_some(*kind))
    }
}

//=============================================
// SECTION 5: Window Entries & Content
//=============================================

#[derive(Debug, Clone)]
struct WindowEntry {
    id: u16,
    kind: WindowKind,
    title: String,
}

impl WindowEntry {
    fn new(kind: WindowKind) -> Self {
        Self {
            id: 0,
            kind,
            title: kind.title().to_string(),
        }
    }

    fn with_id(id: u16, kind: WindowKind) -> Self {
        Self {
            id,
            kind,
            title: kind.title().to_string(),
        }
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn view(&self, style: &StyleCatalog) -> Element<Message> {
        let contents = match self.kind {
            WindowKind::Overview => overview_panel(style),
            WindowKind::Ide => app_panel("Solvra IDE", "Craft code with AI assistance.", style),
            WindowKind::Cli => app_panel("Solvra CLI", "Automate your workflows quickly.", style),
            WindowKind::AppStore => app_panel(
                "Solvra App Store",
                "Discover tools, themes, and plugins.",
                style,
            ),
            WindowKind::Settings => app_panel(
                "System Settings",
                "Adjust profiles, themes, and drivers.",
                style,
            ),
        };

        container(contents)
            .padding(style.spacing_medium)
            .style(iced::theme::Container::Custom(Box::new(
                style.pane_container(),
            )))
            .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WindowKind {
    Overview,
    Ide,
    Cli,
    AppStore,
    Settings,
}

impl WindowKind {
    fn title(&self) -> &'static str {
        match self {
            WindowKind::Overview => "Solvra Overview",
            WindowKind::Ide => "Solvra IDE",
            WindowKind::Cli => "Command Line",
            WindowKind::AppStore => "Solvra App Store",
            WindowKind::Settings => "Control Center",
        }
    }
}

//=============================================
// SECTION 6: Panel Builders
//=============================================

fn overview_panel(style: &StyleCatalog) -> Element<'static, Message> {
    let headline = text("Welcome to Solvra OS")
        .size(32)
        .style(style.text_primary_style())
        .horizontal_alignment(Horizontal::Left);

    let blurb =
        text("Launch the IDE, jump into the CLI, or explore new experiences in the App Store.")
            .style(style.text_subtle_style())
            .size(16);

    column![headline, blurb]
        .spacing(style.spacing_medium)
        .width(Length::Fill)
        .into()
}

fn app_panel(title: &str, description: &str, style: &StyleCatalog) -> Element<'static, Message> {
    column![
        text(title)
            .size(24)
            .style(style.text_primary_style())
            .horizontal_alignment(Horizontal::Left),
        text(description).style(style.text_subtle_style()).size(15),
    ]
    .spacing(style.spacing_medium)
    .width(Length::Fill)
    .into()
}

//=============================================
// SECTION 7: Helpers
//=============================================

fn parse_hex(value: &str) -> Color {
    let value = value.trim_start_matches('#');
    let (r, g, b) = if value.len() == 6 {
        (
            u8::from_str_radix(&value[0..2], 16).unwrap_or(32),
            u8::from_str_radix(&value[2..4], 16).unwrap_or(32),
            u8::from_str_radix(&value[4..6], 16).unwrap_or(32),
        )
    } else {
        (32, 32, 32)
    };
    Color::from_rgb8(r, g, b)
}

//=============================================
// SECTION 8: Entry Point
//=============================================

fn main() -> Result<()> {
    let theme_doc = ThemeDocument::load("./themes/Minimal/theme.toml")?;
    let tokens: ThemeTokens = theme_doc.into();
    DesktopApp::run(Settings::with_flags(tokens))?;
    Ok(())
}
