//=============================================
// solvra_shell_settings/src/main.rs
//=============================================
// Author: Solvra Shell Team
// License: MIT
// Goal: Settings panel bootstrap for Solvra Shell
// Objective: Render profile/theme controls via iced while talking to compositor IPC
//=============================================

use anyhow::Result;
use iced::widget::{column, pick_list, text, Column};
use iced::{executor, Application, Command, Element, Settings, Theme};
use theme_engine::{ThemeDocument, ThemeTokens};

//=============================================
// SECTION: Settings Application
//=============================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileOption {
    /// Full SolvraOS profile.
    Full,
    /// Lite profile.
    Lite,
    /// Tablet profile.
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
    /// Profile selection changed.
    ProfileChanged(ProfileOption),
}

struct SettingsApp {
    tokens: ThemeTokens,
    profile: ProfileOption,
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
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Solvra Shell Settings".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ProfileChanged(option) => self.profile = option,
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let picker = pick_list(
            ProfileOption::all(),
            Some(self.profile),
            Message::ProfileChanged,
        );
        let layout: Column<Message> = column![
            text("Solvra Shell Settings"),
            text(format!(
                "Current theme shadow blur: {}",
                self.tokens.effects.shadow_blur
            )),
            picker,
        ]
        .spacing(12);
        layout.into()
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
