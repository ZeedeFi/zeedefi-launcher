//! `src/input_handler.rs` — abstraction unifiée des entrées (manette + clavier).

use std::collections::HashSet;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};

use egui::{Context, Event, Key};
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use tracing::{info, warn};

pub const DEADZONE: f32 = 0.25;
pub const REPEAT_DELAY: Duration = Duration::from_millis(400);
pub const REPEAT_INTERVAL: Duration = Duration::from_millis(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Navigate(Direction),
    Confirm,
    Back,
    Details,
    QuickSettings,
    Profile,
    FastScroll(f32),
    PageUp,
    PageDown,
    GamepadConnected(String),
    GamepadDisconnected,
}

/// Instantané de l'état d'une manette, calculé dans le thread dédié.
#[derive(Debug, Clone, Default)]
pub struct PadState {
    pub name: String,
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub buttons: HashSet<Button>,
    pub trigger_left: f32,
    pub trigger_right: f32,
}

#[derive(Debug)]
enum GamepadEvent {
    Connected(String),
    Disconnected,
    State(PadState),
}

struct GamepadSource {
    receiver: std_mpsc::Receiver<GamepadEvent>,
    _thread: std::thread::JoinHandle<()>,
}

impl GamepadSource {
    fn spawn() -> Option<Self> {
        let (sender, receiver) = std_mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("zeedefi-gamepad".to_string())
            .spawn(move || {
                let mut gilrs = match Gilrs::new() {
                    Ok(instance) => instance,
                    Err(error) => {
                        warn!(error = %error, "backend manette indisponible");
                        return;
                    }
                };
                let mut active_id: Option<GamepadId> = None;
                loop {
                    if let Some(event) = gilrs.next_event_blocking(Some(Duration::from_millis(50))) { match event.event {
                        EventType::Connected => {
                            active_id = Some(event.id);
                            let pad = gilrs.gamepad(event.id);
                            let name = pad.name().to_string();
                            info!(name = %name, "manette connectée");
                            let _ = sender.send(GamepadEvent::Connected(name));
                        }
                        EventType::Disconnected => {
                            if active_id == Some(event.id) {
                                active_id = None;
                            }
                            let _ = sender.send(GamepadEvent::Disconnected);
                        }
                        _ => {}
                    } }

                    if let Some(id) = active_id {
                        let pad = gilrs.gamepad(id);
                        let mut buttons = HashSet::new();
                        for button in [
                            Button::South,
                            Button::East,
                            Button::West,
                            Button::North,
                            Button::Select,
                            Button::Start,
                            Button::DPadUp,
                            Button::DPadDown,
                            Button::DPadLeft,
                            Button::DPadRight,
                        ] {
                            if pad.is_pressed(button) {
                                buttons.insert(button);
                            }
                        }
                        let state = PadState {
                            name: pad.name().to_string(),
                            left_x: pad
                                .axis_data(Axis::LeftStickX)
                                .map(|data| data.value())
                                .unwrap_or(0.0),
                            left_y: pad
                                .axis_data(Axis::LeftStickY)
                                .map(|data| data.value())
                                .unwrap_or(0.0),
                            right_x: pad
                                .axis_data(Axis::RightStickX)
                                .map(|data| data.value())
                                .unwrap_or(0.0),
                            buttons,
                            trigger_left: pad
                                .button_data(Button::LeftTrigger2)
                                .map(|data| data.value())
                                .unwrap_or(0.0),
                            trigger_right: pad
                                .button_data(Button::RightTrigger2)
                                .map(|data| data.value())
                                .unwrap_or(0.0),
                        };
                        let _ = sender.send(GamepadEvent::State(state));
                    }
                }
            })
            .ok()?;
        Some(Self {
            receiver,
            _thread: handle,
        })
    }
}

pub struct InputManager {
    source: Option<GamepadSource>,
    pad_state: Option<PadState>,
    gamepad_connected: bool,
    held_direction: Option<Direction>,
    held_since: Instant,
    last_repeat: Instant,
    previous_triggers: [f32; 2],
    previous_buttons: HashSet<Button>,
}

impl std::fmt::Debug for InputManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InputManager")
            .field("gamepad_connected", &self.gamepad_connected)
            .finish()
    }
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            source: GamepadSource::spawn(),
            pad_state: None,
            gamepad_connected: false,
            held_direction: None,
            held_since: Instant::now(),
            last_repeat: Instant::now(),
            previous_triggers: [0.0, 0.0],
            previous_buttons: HashSet::new(),
        }
    }

    pub fn poll(&mut self, context: &Context) -> Vec<Action> {
        let mut actions = Vec::new();
        self.drain_gamepad_events(&mut actions);
        if self.pad_state.is_some() {
            actions.extend(self.poll_gamepad());
        }
        actions.extend(self.poll_keyboard(context));
        actions
    }

    fn drain_gamepad_events(&mut self, actions: &mut Vec<Action>) {
        let Some(source) = &self.source else {
            return;
        };
        while let Ok(event) = source.receiver.try_recv() {
            match event {
                GamepadEvent::Connected(name) => {
                    self.gamepad_connected = true;
                    actions.push(Action::GamepadConnected(name));
                }
                GamepadEvent::Disconnected => {
                    self.pad_state = None;
                    self.gamepad_connected = false;
                    actions.push(Action::GamepadDisconnected);
                }
                GamepadEvent::State(state) => {
                    self.pad_state = Some(state);
                }
            }
        }
    }

    fn poll_gamepad(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        let Some(pad) = &self.pad_state else {
            return actions;
        };

        let left_x = apply_deadzone(pad.left_x);
        let left_y = apply_deadzone(pad.left_y);

        let direction = if left_x.abs() > left_y.abs() && left_x.abs() > 0.5 {
            if left_x > 0.0 {
                Some(Direction::Right)
            } else {
                Some(Direction::Left)
            }
        } else if left_y.abs() > 0.5 {
            if left_y > 0.0 {
                Some(Direction::Up)
            } else {
                Some(Direction::Down)
            }
        } else if pad.buttons.contains(&Button::DPadLeft) {
            Some(Direction::Left)
        } else if pad.buttons.contains(&Button::DPadRight) {
            Some(Direction::Right)
        } else if pad.buttons.contains(&Button::DPadUp) {
            Some(Direction::Up)
        } else if pad.buttons.contains(&Button::DPadDown) {
            Some(Direction::Down)
        } else {
            None
        };

        let now = Instant::now();
        match (self.held_direction, direction) {
            (Some(previous), Some(current)) if previous == current => {
                let held_for = now.duration_since(self.held_since);
                if held_for >= REPEAT_DELAY
                    && now.duration_since(self.last_repeat) >= REPEAT_INTERVAL
                {
                    self.last_repeat = now;
                    actions.push(Action::Navigate(current));
                }
            }
            (_, Some(current)) => {
                self.held_direction = Some(current);
                self.held_since = now;
                self.last_repeat = now;
                actions.push(Action::Navigate(current));
            }
            _ => {
                self.held_direction = None;
            }
        }

        // Boutons d'action (front montant).
        let pressed_now: HashSet<Button> = pad.buttons.clone();
        for button in pressed_now.difference(&self.previous_buttons).copied() {
            match button {
                Button::South => actions.push(Action::Confirm),
                Button::East => actions.push(Action::Back),
                Button::West => actions.push(Action::Details),
                Button::North => actions.push(Action::QuickSettings),
                Button::Start => actions.push(Action::Profile),
                _ => {}
            }
        }
        self.previous_buttons = pressed_now;

        // Stick droit : défilement rapide.
        let right_x = apply_deadzone(pad.right_x);
        if right_x.abs() > 0.1 {
            actions.push(Action::FastScroll(right_x));
        }

        // Gâchettes : page précédente/suivante (front montant).
        let trigger_left = pad.trigger_left;
        let trigger_right = pad.trigger_right;
        if trigger_left > 0.6 && self.previous_triggers[0] <= 0.6 {
            actions.push(Action::PageUp);
        }
        if trigger_right > 0.6 && self.previous_triggers[1] <= 0.6 {
            actions.push(Action::PageDown);
        }
        self.previous_triggers = [trigger_left, trigger_right];

        actions
    }

    fn poll_keyboard(&self, context: &Context) -> Vec<Action> {
        let mut actions = Vec::new();
        context.input(|input| {
            for event in &input.events {
                if let Event::Key {
                    key,
                    pressed: true,
                    ..
                } = event
                {
                    match key {
                        Key::ArrowLeft => actions.push(Action::Navigate(Direction::Left)),
                        Key::ArrowRight => actions.push(Action::Navigate(Direction::Right)),
                        Key::ArrowUp => actions.push(Action::Navigate(Direction::Up)),
                        Key::ArrowDown => actions.push(Action::Navigate(Direction::Down)),
                        Key::Enter | Key::Space => actions.push(Action::Confirm),
                        Key::Escape | Key::Backspace => actions.push(Action::Back),
                        Key::X => actions.push(Action::Details),
                        Key::Y | Key::S => actions.push(Action::QuickSettings),
                        Key::Tab | Key::P => actions.push(Action::Profile),
                        Key::PageUp => actions.push(Action::PageUp),
                        Key::PageDown => actions.push(Action::PageDown),
                        _ => {}
                    }
                }
            }
        });
        actions
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_deadzone(value: f32) -> f32 {
    if value.abs() < DEADZONE {
        0.0
    } else {
        let sign = if value >= 0.0 { 1.0 } else { -1.0 };
        sign * (value.abs() - DEADZONE) / (1.0 - DEADZONE)
    }
}
