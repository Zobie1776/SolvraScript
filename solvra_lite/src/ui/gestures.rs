#![allow(dead_code)]

use std::collections::VecDeque;

use egui::Pos2;
use uuid::Uuid;

/// Different phases of a touch interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// A single touch event coming from the input subsystem.
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub id: Uuid,
    pub position: Pos2,
    pub phase: TouchPhase,
    pub pressure: f32,
}

impl TouchEvent {
    pub fn tap(position: Pos2) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            phase: TouchPhase::End,
            pressure: 0.0,
        }
    }
}

/// Recognized gesture that higher layers can react to.
#[derive(Debug, Clone)]
pub enum Gesture {
    Tap { position: Pos2 },
    LongPress { position: Pos2 },
    Swipe { start: Pos2, end: Pos2 },
}

/// Tracks touch events and emits gestures.
#[derive(Debug, Default)]
pub struct GestureSystem {
    history: VecDeque<TouchEvent>,
    max_history: usize,
}

impl GestureSystem {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(32),
            max_history: 32,
        }
    }

    pub fn record(&mut self, event: TouchEvent) -> Option<Gesture> {
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(event.clone());

        match event.phase {
            TouchPhase::End => self.detect_swipe().or({
                Some(Gesture::Tap {
                    position: event.position,
                })
            }),
            TouchPhase::Cancel => None,
            _ => None,
        }
    }

    fn detect_swipe(&self) -> Option<Gesture> {
        let mut iter = self.history.iter().rev();
        let end = iter.next()?;
        let start = iter.find(|e| matches!(e.phase, TouchPhase::Start))?;
        let delta = end.position - start.position;
        if delta.length() > 120.0 {
            Some(Gesture::Swipe {
                start: start.position,
                end: end.position,
            })
        } else {
            None
        }
    }
}
