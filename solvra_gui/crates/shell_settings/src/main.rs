//=============================================
// solvra_shell_settings/src/main.rs
//=============================================
// Author: Solvra GUI Team
// License: MIT
// Goal: Solvra GUI settings application
// Objective: Provide controls for profiles, themes, and plugin toggles via iced
//=============================================

use anyhow::Result;
use iced::widget::{column, pick_list, text, toggler};
use iced::{executor, Application, Command, Element, Settings, Theme};
use theme_engine::{ThemeDocument, ThemeTokens};

//=============================================
// SECTION: Settings State
//=============================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileOption {
    Full,
    Lite,
    Tablet,
}

impl ProfileOption {
    fn all() -> [Self; 3] {
        [Self::Full, Self::Lite, Self::Tablet]
    }
}

impl std::fmt::Display for ProfileOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileOption::Full => write!(f, "Full"),
            ProfileOption::Lite => write!(f, "Lite"),
            ProfileOption::Tablet => write!(f, "Tablet"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ProfileChanged(ProfileOption),
    PluginsToggled(bool),
}

struct SettingsApp {
    tokens: ThemeTokens,
    profile: ProfileOption,
    plugins: bool,
}

impl Application for SettingsApp {
    type Executor = executor::Default;
    type Flags = ThemeTokens;
    type Message = Message;
    type Theme = Theme;

    fn new(tokens: Self::Flags) -> (Self, Command<Message>) {
        utils::logging::init("settings");
        (
            Self {
                tokens,
                profile: ProfileOption::Lite,
                plugins: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Solvra Settings".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ProfileChanged(option) => self.profile = option,
            Message::PluginsToggled(state) => self.plugins = state,
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        column![
            text("Solvra Settings"),
            pick_list(
                ProfileOption::all(),
                Some(self.profile),
                Message::ProfileChanged
            ),
            toggler("Enable plugins", self.plugins, Message::PluginsToggled),
            text(format!("Shadow blur: {}", self.tokens.effects.shadow_blur)),
        ]
        .spacing(16)
        .into()
    }
}

//=============================================
// SECTION: Entry Point
//=============================================

fn main() -> Result<()> {
    let theme_doc = ThemeDocument::load("./themes/CyberGrid/theme.toml")?;
    let tokens: ThemeTokens = theme_doc.into();
    SettingsApp::run(Settings::with_flags(tokens))?;
    Ok(())
}
