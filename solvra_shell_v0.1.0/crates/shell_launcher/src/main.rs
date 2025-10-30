use anyhow::Result;
use iced::application;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text_input::Id as TextInputId;
use iced::widget::{button, column, container, image, row, scrollable, svg, text, text_input};
use iced::{Element, Settings, Task, Theme};

#[derive(Clone, Debug)]
struct AppEntry {
    name: String,
    exec: String,
    icon: Option<String>,
}

#[derive(Debug)]
struct LauncherApp {
    query: String,
    apps: Vec<AppEntry>,
    filtered: Vec<AppEntry>,
    cursor: usize,
    search_id: TextInputId,
}

#[derive(Debug, Clone)]
enum Message {
    QueryChanged(String),
    MoveUp,
    MoveDown,
    Activate,
}

fn hardcoded_apps() -> Vec<AppEntry> {
    vec![
        AppEntry {
            name: "Firefox".into(),
            exec: "firefox".into(),
            icon: Some("/usr/share/icons/hicolor/48x48/apps/firefox.png".into()),
        },
        AppEntry {
            name: "GNOME Terminal".into(),
            exec: "gnome-terminal".into(),
            icon: Some("/usr/share/pixmaps/org.gnome.Terminal.svg".into()),
        },
        AppEntry {
            name: "Files".into(),
            exec: "nautilus".into(),
            icon: None,
        },
    ]
}

fn filter_now(all: &[AppEntry], q: &str) -> Vec<AppEntry> {
    if q.is_empty() {
        return all.to_vec();
    }

    let needle = q.to_lowercase();
    all.iter()
        .filter(|a| a.name.to_lowercase().contains(&needle))
        .cloned()
        .collect()
}

fn icon_el(a: &AppEntry) -> Element<'static, Message> {
    if let Some(path) = &a.icon {
        let p = std::path::Path::new(path);
        if p.extension().and_then(|s| s.to_str()) == Some("svg") {
            svg(SvgHandle::from_path(p)).width(24).height(24).into()
        } else if p.exists() {
            image(ImageHandle::from_path(p)).width(24).height(24).into()
        } else {
            text("·").into()
        }
    } else {
        text("·").into()
    }
}

fn view(app: &LauncherApp) -> Element<'_, Message> {
    let input: iced::widget::text_input::TextInput<'_, Message, Theme, iced::Renderer> =
        text_input("Type to search…", &app.query)
            .id(app.search_id.clone())
            .on_input(Message::QueryChanged);

    let mut list = column![];
    for (i, a) in app.filtered.iter().enumerate().take(30) {
        let line = row![icon_el(a), text(&a.name)].spacing(8);
        let line = if i == app.cursor {
            container(line).padding(6)
        } else {
            container(line).padding(6)
        };
        list = list.push(line);
    }

    let controls = row![
        button("Up").on_press(Message::MoveUp),
        button("Down").on_press(Message::MoveDown),
        button("Launch").on_press(Message::Activate),
    ]
    .spacing(8);

    column![
        text("Solvra Shell Launcher"),
        input,
        scrollable(list),
        controls
    ]
    .spacing(12)
    .padding(12)
    .into()
}

fn update(app: &mut LauncherApp, message: Message) -> Task<Message> {
    match message {
        Message::QueryChanged(q) => {
            app.query = q;
            app.filtered = filter_now(&app.apps, &app.query);
            app.cursor = 0;
        }
        Message::MoveUp => {
            if app.cursor > 0 {
                app.cursor -= 1;
            }
        }
        Message::MoveDown => {
            if app.cursor + 1 < app.filtered.len() {
                app.cursor += 1;
            }
        }
        Message::Activate => {
            if let Some(a) = app.filtered.get(app.cursor) {
                let _ = std::process::Command::new(&a.exec).spawn();
            }
        }
    }
    Task::none()
}

fn initialize() -> (LauncherApp, Task<Message>) {
    if std::env::var_os("WGPU_BACKEND").is_none() {
        std::env::set_var("WGPU_BACKEND", "gl");
    }
    if std::env::var_os("WINIT_UNIX_BACKEND").is_none() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    let search_id = TextInputId::unique();
    let apps = hardcoded_apps();
    let filtered = filter_now(&apps, "");
    let app = LauncherApp {
        query: String::new(),
        apps,
        filtered,
        cursor: 0,
        search_id: search_id.clone(),
    };

    (app, iced::widget::text_input::focus(search_id))
}

fn main() -> Result<()> {
    application("Solvra Shell Launcher", update, view)
        .theme(|_| Theme::Dark)
        .settings(Settings::default())
        .run_with(initialize)?;
    Ok(())
}
