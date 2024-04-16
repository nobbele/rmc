use std::{collections::HashMap, hash::Hash};

use sdl2::{keyboard::Keycode, mouse::MouseButton};
use vek::Vec2;

#[derive(Debug, Clone)]
pub struct InputState {
    pub keys: HashMap<Keycode, ButtonState>,
    pub mouse_buttons: HashMap<MouseButton, ButtonState>,
    pub mouse_delta: Vec2<f32>,
    pub scroll_delta: i32,
}

impl InputState {
    pub fn get_key(&self, keycode: Keycode) -> ButtonState {
        self.keys.get(&keycode).cloned().unwrap_or_default()
    }

    pub fn get_mouse_button(&self, button: MouseButton) -> ButtonState {
        self.mouse_buttons.get(&button).cloned().unwrap_or_default()
    }

    pub fn get_movement_vector(&self) -> Vec2<f32> {
        let fwd_bck =
            self.get_key(Keycode::W).pressed() as i8 - self.get_key(Keycode::S).pressed() as i8;
        let rgh_lft =
            self.get_key(Keycode::D).pressed() as i8 - self.get_key(Keycode::A).pressed() as i8;

        Vec2::new(rgh_lft as f32, fwd_bck as f32)
    }
}

impl InputState {
    pub fn update_held_status(&mut self) {
        for keycode in self.keys.keys().cloned().collect::<Vec<_>>() {
            self.keys.insert(
                keycode,
                if self.get_key(keycode).pressed() {
                    ButtonState::KeptPressed
                } else {
                    ButtonState::KeptReleased
                },
            );
        }
        for mouse_button in self.mouse_buttons.keys().cloned().collect::<Vec<_>>() {
            self.mouse_buttons.insert(
                mouse_button,
                if self.get_mouse_button(mouse_button).pressed() {
                    ButtonState::KeptPressed
                } else {
                    ButtonState::KeptReleased
                },
            );
        }
    }

    pub fn push_keyboard_event(&mut self, event: KeyboardEvent) {
        self.keys.insert(
            event.key,
            if event.state == ButtonStateEvent::Press {
                if self.get_key(event.key).pressed() {
                    ButtonState::KeptPressed
                } else {
                    ButtonState::JustPressed
                }
            } else {
                if self.get_key(event.key).released() {
                    ButtonState::KeptReleased
                } else {
                    ButtonState::JustReleased
                }
            },
        );
    }

    pub fn push_mouse_button_event(&mut self, event: MouseButtonEvent) {
        self.mouse_buttons.insert(
            event.button,
            if event.state == ButtonStateEvent::Press {
                if self.get_mouse_button(event.button).pressed() {
                    ButtonState::KeptPressed
                } else {
                    ButtonState::JustPressed
                }
            } else {
                if self.get_mouse_button(event.button).released() {
                    ButtonState::KeptReleased
                } else {
                    ButtonState::JustReleased
                }
            },
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    KeptPressed,
    JustPressed,
    KeptReleased,
    JustReleased,
}

impl ButtonState {
    pub fn just_pressed(self) -> bool {
        self == ButtonState::JustPressed
    }

    pub fn kept_pressed(self) -> bool {
        self == ButtonState::KeptPressed
    }

    pub fn pressed(self) -> bool {
        self.kept_pressed() || self.just_pressed()
    }

    pub fn released(self) -> bool {
        self == ButtonState::KeptReleased || self == ButtonState::JustReleased
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::KeptReleased
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStateEvent {
    Press,
    Release,
}

pub trait ButtonEvent {
    type Key;
    fn key(&self) -> Self::Key;
    fn state(&self) -> ButtonStateEvent;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub key: Keycode,
    pub state: ButtonStateEvent,
}

impl ButtonEvent for KeyboardEvent {
    type Key = Keycode;

    fn key(&self) -> Self::Key {
        self.key
    }

    fn state(&self) -> ButtonStateEvent {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub state: ButtonStateEvent,
}

impl ButtonEvent for MouseButtonEvent {
    type Key = MouseButton;

    fn key(&self) -> Self::Key {
        self.button
    }

    fn state(&self) -> ButtonStateEvent {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    MouseButton(MouseButtonEvent),
    MouseMovement(Vec2<f32>),
}

/// Take-latest buffering.
/// Up Up Down -> Up Down
/// Up Down Up -> Down Up
/// Systems using this object will have to handle the existing state and ignore redundant events themselves to produce the result: Up Down Up -> Up
pub struct ButtonBuffer<E: ButtonEvent>(HashMap<E::Key, (E, Option<E>)>);

impl<E> ButtonBuffer<E>
where
    E: ButtonEvent + Clone,
    E::Key: Clone + Hash + Eq,
{
    pub fn new() -> Self {
        ButtonBuffer(HashMap::new())
    }

    pub fn keys(&self) -> impl Iterator<Item = E::Key> + '_ {
        self.0.keys().cloned()
    }

    pub fn push(&mut self, event: E) {
        let current = self.0.get(&event.key());
        self.0.insert(
            event.key(),
            if let Some(current) = current {
                let latest = if let Some(second) = current.1.clone() {
                    second
                } else {
                    current.0.clone()
                };

                // TODO
                if latest.state() == event.state() {
                    current.clone()
                } else {
                    if let Some(second) = current.1.clone() {
                        (second, Some(event))
                    } else {
                        (current.0.clone(), Some(event))
                    }
                }
            } else {
                (event, None)
            },
        );
    }

    pub fn pull(&mut self, key: E::Key) -> Option<E> {
        let current = self.0.get_mut(&key)?;
        let first = current.0.clone();
        if let Some(second) = current.1.clone() {
            current.0 = second;
            current.1 = None;
        } else {
            self.0.remove(&key);
        };

        Some(first)
    }
}

#[test]
pub fn test_key_buffer() {
    let mut buffer = ButtonBuffer::new();
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });

    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(buffer.pull(Keycode::A), None);

    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });

    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(buffer.pull(Keycode::A), None);

    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Release,
    });

    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Release
        })
    );
    assert_eq!(buffer.pull(Keycode::A), None);

    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Release,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });

    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Release
        })
    );
    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(buffer.pull(Keycode::A), None);

    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::B,
        state: ButtonStateEvent::Press,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Release,
    });
    buffer.push(KeyboardEvent {
        key: Keycode::A,
        state: ButtonStateEvent::Press,
    });

    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Release
        })
    );
    assert_eq!(
        buffer.pull(Keycode::A),
        Some(KeyboardEvent {
            key: Keycode::A,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(
        buffer.pull(Keycode::B),
        Some(KeyboardEvent {
            key: Keycode::B,
            state: ButtonStateEvent::Press
        })
    );
    assert_eq!(buffer.pull(Keycode::A), None);
    assert_eq!(buffer.pull(Keycode::B), None);
}
