use crate::app::{
    AppCapability, AppCategory, AppId, AppMetadata, AppPackage, UiComponent, UiComponentKind,
};
use crate::sandbox::{SandboxPermission, SandboxPolicy};
use semver::Version;
use std::time::{Duration, Instant};

/// Return catalog metadata for NovaStream.
pub fn metadata() -> AppMetadata {
    let sandbox = SandboxPolicy::new()
        .allow_permission(SandboxPermission::Camera)
        .allow_permission(SandboxPermission::Microphone)
        .allow_permission(SandboxPermission::ScreenCapture)
        .allow_permission(SandboxPermission::FileWrite)
        .allow_network_host("rtmp://live.novaos.dev")
        .allow_storage_root("~/Videos/NovaStream");

    let package = AppPackage::new(Version::new(1, 0, 0))
        .with_sandbox(sandbox)
        .with_capability(
            AppCapability::new(
                "novastream.capture",
                "Multi-source capture, mixing, and recording pipeline",
            )
            .with_tag("streaming")
            .with_tag("multimedia"),
        )
        .with_ui_component(
            UiComponent::new(
                "novastream-studio",
                UiComponentKind::ShellWidget,
                "Scene composer with preview and program monitors",
            )
            .with_entry_point("novastream::studio"),
        );

    AppMetadata::new(
        AppId::new("dev.nova.stream").expect("valid id"),
        "NovaStream",
        "Stream, record, and mix multiple media sources",
        "NovaStream delivers an OBS-inspired workflow with scene presets, capture device management, and extensible output pipelines.",
        AppCategory::Multimedia,
        "Nova Labs",
        package,
    )
    .with_tag("video")
    .with_tag("streaming")
    .with_screenshot("screenshots/novastream.png")
}

/// High-level controller orchestrating a streaming session.
#[derive(Debug)]
pub struct StreamSession {
    scenes: Vec<Scene>,
    active_scene: Option<usize>,
    events: Vec<StreamEvent>,
    recorder: Recorder,
}

impl StreamSession {
    /// Create a new session with no scenes configured.
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            active_scene: None,
            events: Vec::new(),
            recorder: Recorder::default(),
        }
    }

    /// Register a new scene available for broadcasting.
    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.push(scene);
        if self.active_scene.is_none() {
            self.active_scene = Some(0);
        }
    }

    /// Retrieve all configured scenes.
    pub fn scenes(&self) -> &[Scene] {
        &self.scenes
    }

    /// Switch the active scene by name.
    pub fn switch_scene(&mut self, name: &str) -> Option<()> {
        let index = self
            .scenes
            .iter()
            .position(|scene| scene.name.eq_ignore_ascii_case(name))?;
        self.active_scene = Some(index);
        self.events.push(StreamEvent::SceneSwitched {
            scene: self.scenes[index].name.clone(),
            timestamp: Instant::now(),
        });
        Some(())
    }

    /// Start recording the program output.
    pub fn start_recording(&mut self) {
        if !self.recorder.is_recording {
            self.recorder.start();
            self.events.push(StreamEvent::RecordingStarted(Instant::now()));
        }
    }

    /// Stop recording and return the duration captured.
    pub fn stop_recording(&mut self) -> Option<Duration> {
        if self.recorder.is_recording {
            let duration = self.recorder.stop();
            self.events.push(StreamEvent::RecordingStopped(Instant::now(), duration));
            Some(duration)
        } else {
            None
        }
    }

    /// Add a source to the active scene.
    pub fn add_source_to_active(&mut self, source: Source) -> Option<()> {
        let index = self.active_scene?;
        self.scenes[index].sources.push(source.clone());
        self.events.push(StreamEvent::SourceAdded {
            scene: self.scenes[index].name.clone(),
            source,
            timestamp: Instant::now(),
        });
        Some(())
    }

    /// Generate a lightweight mix graph summarising the active scene.
    pub fn mix_summary(&self) -> Option<SceneMix> {
        let scene = self.active_scene.and_then(|idx| self.scenes.get(idx))?;
        let mut overlays = Vec::new();
        let mut captures = Vec::new();
        for source in &scene.sources {
            match source {
                Source::Camera(camera) => captures.push(format!("Camera:{}", camera.device_id)),
                Source::Screen(screen) => captures.push(format!("Screen:{}", screen.display)),
                Source::Media(media) => overlays.push(format!("Media:{}", media.path)),
                Source::Overlay(overlay) => overlays.push(format!("Overlay:{}", overlay.name)),
            }
        }
        Some(SceneMix {
            scene: scene.name.clone(),
            captures,
            overlays,
        })
    }

    /// Access recorded event history.
    pub fn events(&self) -> &[StreamEvent] {
        &self.events
    }
}

/// Recorder tracks the state of file-based captures.
#[derive(Debug)]
struct Recorder {
    is_recording: bool,
    started_at: Option<Instant>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            is_recording: false,
            started_at: None,
        }
    }
}

impl Recorder {
    fn start(&mut self) {
        self.is_recording = true;
        self.started_at = Some(Instant::now());
    }

    fn stop(&mut self) -> Duration {
        self.is_recording = false;
        let duration = self
            .started_at
            .take()
            .map(|start| start.elapsed())
            .unwrap_or_default();
        duration
    }
}

/// Description of an active scene.
#[derive(Debug, Clone)]
pub struct Scene {
    pub name: String,
    pub sources: Vec<Source>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sources: Vec::new(),
        }
    }
}

/// Mixer summary used by the UI to display pipeline status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneMix {
    pub scene: String,
    pub captures: Vec<String>,
    pub overlays: Vec<String>,
}

/// Capture sources supported by NovaStream.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Camera(CameraSource),
    Screen(ScreenSource),
    Media(MediaSource),
    Overlay(OverlaySource),
}

/// Camera capture device configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraSource {
    pub device_id: String,
    pub resolution: (u32, u32),
    pub fps: u32,
}

/// Screen capture source configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenSource {
    pub display: String,
    pub region: Option<(u32, u32, u32, u32)>,
}

/// Media file source (video/image) configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSource {
    pub path: String,
    pub loop_playback: bool,
}

/// Overlay asset configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlaySource {
    pub name: String,
    pub opacity: f32,
}

/// Events produced during streaming operations.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    SceneSwitched { scene: String, timestamp: Instant },
    SourceAdded { scene: String, source: Source, timestamp: Instant },
    RecordingStarted(Instant),
    RecordingStopped(Instant, Duration),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn manages_scenes_and_sources() {
        let mut session = StreamSession::new();
        session.add_scene(Scene::new("Intro"));
        session.add_scene(Scene::new("Gameplay"));
        session.switch_scene("Gameplay").unwrap();
        session
            .add_source_to_active(Source::Camera(CameraSource {
                device_id: "cam0".into(),
                resolution: (1920, 1080),
                fps: 60,
            }))
            .unwrap();
        session
            .add_source_to_active(Source::Overlay(OverlaySource {
                name: "Logo".into(),
                opacity: 0.8,
            }))
            .unwrap();
        let mix = session.mix_summary().unwrap();
        assert_eq!(mix.scene, "Gameplay");
        assert_eq!(mix.captures.len(), 1);
        assert_eq!(mix.overlays.len(), 1);
    }

    #[test]
    fn records_sessions_with_durations() {
        let mut session = StreamSession::new();
        session.add_scene(Scene::new("Main"));
        session.start_recording();
        thread::sleep(Duration::from_millis(10));
        let duration = session.stop_recording().unwrap();
        assert!(duration >= Duration::from_millis(10));
        assert!(session.events().len() >= 2);
    }
}
