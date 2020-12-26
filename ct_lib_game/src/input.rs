////////////////////////////////////////////////////////////////////////////////////////////////////
// GameInput

use super::math::Vec2;

use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct GameInput {
    pub mouse: MouseState,
    pub touch: TouchState,
    pub keyboard: KeyboardState,
    pub textinput: Textinput,

    pub has_focus_event: bool,
    pub has_focus: bool,
    pub has_foreground_event: bool,

    pub screen_is_fullscreen: bool,
    pub screen_framebuffer_width: u32,
    pub screen_framebuffer_height: u32,
    pub screen_framebuffer_dimensions_changed: bool,

    pub gamepad: GamepadState,

    pub deltatime: f32,
    pub real_world_uptime: f64,

    pub audio_playback_rate_hz: usize,
}

impl GameInput {
    pub fn new() -> GameInput {
        GameInput::default()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Textinput

#[derive(Default, Clone)]
pub struct Textinput {
    pub is_textinput_enabled: bool,

    pub has_new_textinput_event: bool,
    pub inputtext: String,

    pub has_new_composition_event: bool,
    pub composition_text: String,
    pub composition_cursor_pos: i32,
    pub composition_selection_length: i32,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Buttons

/// NOTE: The transition count is useful if we have multiple transitions in a frame. This gives
/// us information about what state the button was before the frame started and how often it
/// switched states (useful for frames that took longer than expected but we still don't want
/// to miss the players input)
#[derive(Default, Copy, Clone, Debug)]
pub struct ButtonState {
    pub is_pressed: bool,
    pub transition_count: u32,
    pub system_repeat_count: u32,
    // NOTE: This can be used to implement soft key-repeats
    pub tick_of_last_transition: u64,
}

impl ButtonState {
    /// Changes state of a button while counting all transitions from pressed -> released and from
    /// released -> pressed
    pub fn process_event(&mut self, is_pressed: bool, is_repeat: bool, tick: u64) {
        if self.is_pressed != is_pressed {
            self.transition_count += 1;
            self.is_pressed = is_pressed;
            self.tick_of_last_transition = tick;
        } else {
            debug_assert!(is_pressed);
            debug_assert!(is_repeat);
            self.system_repeat_count += 1;
        }
    }

    pub fn recently_pressed(&self) -> bool {
        self.is_pressed && (self.transition_count > 0)
    }

    pub fn recently_pressed_or_repeated(&self) -> bool {
        self.is_pressed && ((self.transition_count > 0) || (self.system_repeat_count > 0))
    }

    pub fn recently_released(&self) -> bool {
        !self.is_pressed && (self.transition_count > 0)
    }

    pub fn clear_transitions(&mut self) {
        self.transition_count = 0;
        self.system_repeat_count = 0;
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Mouse

#[derive(Default, Clone)]
pub struct MouseState {
    pub has_moved: bool,
    pub has_press_event: bool,
    pub has_release_event: bool,
    pub has_wheel_event: bool,

    // Pos in [0, screen_width - 1]x[0, screen_height - 1] (left to right and top to bottom)
    pub pos_x: i32,
    pub pos_y: i32,

    pub delta_x: i32,
    pub delta_y: i32,

    pub wheel_delta: i32,

    pub button_left: ButtonState,
    pub button_middle: ButtonState,
    pub button_right: ButtonState,
    pub button_x1: ButtonState,
    pub button_x2: ButtonState,
}

impl MouseState {
    pub fn clear_transitions(&mut self) {
        self.has_moved = false;
        self.has_press_event = false;
        self.has_release_event = false;
        self.has_wheel_event = false;

        self.delta_x = 0;
        self.delta_y = 0;
        self.wheel_delta = 0;

        self.button_left.clear_transitions();
        self.button_middle.clear_transitions();
        self.button_right.clear_transitions();
        self.button_x1.clear_transitions();
        self.button_x2.clear_transitions();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Touch

pub type FingerId = usize;
pub type FingerPlatformId = i64;

#[derive(Clone)]
pub struct TouchFinger {
    pub state: ButtonState,

    // Pos in [0, screen_width - 1]x[0, screen_height - 1] (left to right and top to bottom)
    pub pos_x: i32,
    pub pos_y: i32,

    pub delta_x: i32,
    pub delta_y: i32,

    pub id: FingerId,              // Given by us
    platform_id: FingerPlatformId, // Given by the Implementation
}

impl TouchFinger {
    fn new(id: FingerId, platform_id: FingerPlatformId, pos_x: i32, pos_y: i32) -> TouchFinger {
        TouchFinger {
            state: ButtonState::default(),
            pos_x,
            pos_y,
            delta_x: 0,
            delta_y: 0,
            id,
            platform_id,
        }
    }
}

#[derive(Default, Clone)]
pub struct TouchState {
    pub has_move_event: bool,
    pub has_press_event: bool,
    pub has_release_event: bool,

    pub fingers: HashMap<FingerId, TouchFinger>,
    fingers_previous: HashMap<FingerId, TouchFinger>,
}

impl TouchState {
    pub fn process_finger_down(
        &mut self,
        platform_id: FingerPlatformId,
        pos_x: i32,
        pos_y: i32,
        current_tick: u64,
    ) {
        // NOTE: It can happen that the implementation re-used a finger ID faster
        //       than we could delete our corresponding finger one. If that happens we just delete
        //       our corresponding finger and create a new one with the same ID.
        //       We use retain here instead of just inserting the new finger because we want
        //       `get_next_free_finger_id` to give us the correct id in the case we removed the last
        //       finger in our list
        self.fingers
            .retain(|_id, finger| finger.platform_id != platform_id);
        let id = self.get_next_free_finger_id();

        self.has_press_event = true;
        let mut finger = TouchFinger::new(id, platform_id, pos_x, pos_y);
        finger.state.process_event(true, false, current_tick);
        self.fingers.insert(id, finger);
    }

    pub fn process_finger_up(
        &mut self,
        platform_id: FingerPlatformId,
        pos_x: i32,
        pos_y: i32,
        current_tick: u64,
    ) {
        self.has_release_event |= {
            if let Some(finger) = self.get_finger_by_platform_id_mut(platform_id) {
                finger.pos_x = pos_x;
                finger.pos_y = pos_y;
                finger.state.process_event(false, false, current_tick);
                true
            } else {
                debug_assert!(
                    false,
                    "Got touch up event for non-existing finger {}",
                    platform_id
                );
                false
            }
        };
    }

    pub fn process_finger_move(&mut self, platform_id: FingerPlatformId, pos_x: i32, pos_y: i32) {
        self.has_move_event |= {
            if let Some(finger) = self.get_finger_by_platform_id_mut(platform_id) {
                finger.pos_x = pos_x;
                finger.pos_y = pos_y;
                true
            } else {
                debug_assert!(
                    false,
                    "Got touch up event for non-existing finger {}",
                    platform_id
                );
                false
            }
        };
    }

    pub fn calculate_move_deltas(&mut self) {
        let ids: Vec<FingerId> = self.fingers.keys().cloned().collect();
        for id in ids {
            let previous_pos = {
                self.fingers_previous
                    .get(&id)
                    .map(|previous_finger| (previous_finger.pos_x, previous_finger.pos_y))
            };

            if let Some((previous_pos_x, previous_pos_y)) = previous_pos {
                let mut finger = self.fingers.get_mut(&id).unwrap();
                finger.delta_x = finger.pos_x - previous_pos_x;
                finger.delta_y = finger.pos_y - previous_pos_y;
            }
        }
    }

    pub fn clear_transitions(&mut self) {
        self.has_move_event = false;
        self.has_press_event = false;
        self.has_release_event = false;

        for finger in self.fingers.values_mut() {
            finger.state.transition_count = 0;
            finger.delta_x = 0;
            finger.delta_y = 0;
        }

        // Remove inactive fingers
        self.fingers
            .retain(|_id, finger| finger.state.is_pressed || finger.state.recently_released());

        self.fingers_previous = self.fingers.clone();
    }

    fn get_next_free_finger_id(&self) -> FingerId {
        if self.fingers.is_empty() {
            0
        } else {
            let max_index = self
                .fingers
                .values()
                .max_by(|a, b| FingerId::cmp(&a.id, &b.id))
                .unwrap()
                .id;
            max_index + 1
        }
    }

    fn get_finger_by_platform_id_mut(
        &mut self,
        platform_id: FingerPlatformId,
    ) -> Option<&mut TouchFinger> {
        for finger in self.fingers.values_mut() {
            if finger.platform_id == platform_id {
                return Some(finger);
            }
        }
        None
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Gamepad

#[derive(Default, Clone)]
pub struct GamepadState {
    pub is_connected: bool,

    pub start: ButtonState,
    pub back: ButtonState,
    pub home: ButtonState,

    pub move_up: ButtonState,
    pub move_down: ButtonState,
    pub move_left: ButtonState,
    pub move_right: ButtonState,

    pub action_up: ButtonState,
    pub action_down: ButtonState,
    pub action_left: ButtonState,
    pub action_right: ButtonState,

    pub stick_left: Vec2,
    pub stick_right: Vec2,

    pub stick_button_left: ButtonState,
    pub stick_button_right: ButtonState,

    pub trigger_left: f32,
    pub trigger_right: f32,

    pub trigger_button_left_1: ButtonState,
    pub trigger_button_left_2: ButtonState,

    pub trigger_button_right_1: ButtonState,
    pub trigger_button_right_2: ButtonState,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Keyboard

#[derive(Clone, Default)]
pub struct KeyboardState {
    pub has_press_event: bool,
    pub has_release_event: bool,
    pub has_system_repeat_event: bool,
    pub keys: HashMap<Scancode, KeyState>,
}

#[derive(Clone, Copy)]
pub struct KeyState {
    pub keycode: Keycode,
    pub scancode: Scancode,
    pub button: ButtonState,
}

impl Default for KeyState {
    fn default() -> Self {
        KeyState {
            keycode: Keycode::Unidentified,
            scancode: Scancode::Unidentified,
            button: ButtonState::default(),
        }
    }
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState::default()
    }

    pub fn clear_state_and_transitions(&mut self) {
        self.clear_transitions();
        self.keys.values_mut().for_each(|keystate| {
            keystate.button.is_pressed = false;
        });
    }

    pub fn clear_transitions(&mut self) {
        self.has_press_event = false;
        self.has_release_event = false;
        self.has_system_repeat_event = false;

        self.keys.values_mut().for_each(|keystate| {
            keystate.button.transition_count = 0;
            keystate.button.system_repeat_count = 0;
        });
    }

    /// Changes state of a key while counting all transitions from pressed -> released and from
    /// released -> pressed
    pub fn process_key_event(
        &mut self,
        scancode: Scancode,
        keycode: Keycode,
        is_pressed: bool,
        is_repeat: bool,
        tick: u64,
    ) {
        let mut key = self.keys.entry(scancode).or_insert(KeyState {
            keycode,
            scancode,
            button: ButtonState::default(),
        });
        if key.keycode != keycode {
            // NOTE: Update keycode (keycode may differ for example if the user changed their
            // input language)
            key.keycode = keycode;
        }
        key.button.process_event(is_pressed, is_repeat, tick);
    }

    pub fn is_down(&self, scancode: Scancode) -> bool {
        if let Some(key) = self.keys.get(&scancode) {
            key.button.is_pressed
        } else {
            false
        }
    }

    pub fn recently_pressed(&self, scancode: Scancode) -> bool {
        if let Some(key) = self.keys.get(&scancode) {
            key.button.recently_pressed()
        } else {
            false
        }
    }

    pub fn recently_pressed_or_repeated(&self, scancode: Scancode) -> bool {
        if let Some(key) = self.keys.get(&scancode) {
            key.button.recently_pressed_or_repeated()
        } else {
            false
        }
    }

    pub fn recently_released(&self, scancode: Scancode) -> bool {
        if let Some(key) = self.keys.get(&scancode) {
            key.button.recently_released()
        } else {
            false
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Handling digit-keys

const SCANCODE_DIGITS: [Scancode; 10] = [
    Scancode::Digit0,
    Scancode::Digit1,
    Scancode::Digit2,
    Scancode::Digit3,
    Scancode::Digit4,
    Scancode::Digit5,
    Scancode::Digit6,
    Scancode::Digit7,
    Scancode::Digit8,
    Scancode::Digit9,
];

const SCANCODE_DIGITS_KEYPAD: [Scancode; 10] = [
    Scancode::Numpad0,
    Scancode::Numpad1,
    Scancode::Numpad2,
    Scancode::Numpad3,
    Scancode::Numpad4,
    Scancode::Numpad5,
    Scancode::Numpad6,
    Scancode::Numpad7,
    Scancode::Numpad8,
    Scancode::Numpad9,
];

impl KeyboardState {
    pub fn is_down_digit(&self, digit: usize) -> bool {
        let code = SCANCODE_DIGITS[digit];
        let code_keypad = SCANCODE_DIGITS_KEYPAD[digit];
        self.is_down(code) || self.is_down(code_keypad)
    }

    pub fn recently_pressed_digit(&self, digit: usize) -> bool {
        let code = SCANCODE_DIGITS[digit];
        let code_keypad = SCANCODE_DIGITS_KEYPAD[digit];
        self.recently_pressed(code) || self.recently_pressed(code_keypad)
    }

    pub fn recently_pressed_or_repeated_digit(&self, digit: usize) -> bool {
        let code = SCANCODE_DIGITS[digit];
        let code_keypad = SCANCODE_DIGITS_KEYPAD[digit];
        self.recently_pressed_or_repeated(code) || self.recently_pressed_or_repeated(code_keypad)
    }

    pub fn recently_released_digit(&self, digit: usize) -> bool {
        let code = SCANCODE_DIGITS[digit];
        let code_keypad = SCANCODE_DIGITS_KEYPAD[digit];
        self.recently_released(code) || self.recently_released(code_keypad)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Keycodes, scancodes and keymods
//

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Scancode {
    Unidentified,

    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    AudioVolumeMute,
    AudioVolumeUp,
    AudioVolumeDown,

    Equal,        // =
    Minus,        // -
    BracketRight, // ]
    BracketLeft,  // [
    Slash,        // /
    Backslash,    // \

    Escape,
    Enter,
    Tab,
    Space,
    Backspace,

    Quote,     // '
    Backquote, // `
    Semicolon, // ;
    Comma,     // ,
    Period,    // .

    CapsLock,
    MetaLeft,  // i.e. Windows key
    MetaRight, // i.e. Windows key
    ShiftLeft,
    ShiftRight,
    AltLeft,
    AltRight,
    ControlLeft,
    ControlRight,
    ContextMenu,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    Numlock,
    NumpadMultiply,
    NumpadAdd,
    NumpadDivide,
    NumpadEnter,
    NumpadSubtract,
    NumpadEqual,
    NumpadComma,
    NumpadDecimal,

    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    ScrollLock,
    PrintScreen,
    Pause,

    Home,
    Delete,
    End,
    PageUp,
    PageDown,
    Insert,

    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Keycode {
    Unidentified,

    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    AudioVolumeMute,
    AudioVolumeUp,
    AudioVolumeDown,

    Equal,        // =
    Minus,        // -
    BracketRight, // ]
    BracketLeft,  // [
    Slash,        // /
    Backslash,    // \

    Escape,
    Enter,
    Tab,
    Space,
    Backspace,

    Quote,     // '
    Backquote, // `
    Semicolon, // ;
    Comma,     // ,
    Period,    // .

    CapsLock,
    MetaLeft,  // i.e. Windows key
    MetaRight, // i.e. Windows key
    ShiftLeft,
    ShiftRight,
    AltLeft,
    AltRight,
    ControlLeft,
    ControlRight,
    ContextMenu,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    Numlock,
    NumpadMultiply,
    NumpadAdd,
    NumpadDivide,
    NumpadEnter,
    NumpadSubtract,
    NumpadEqual,
    NumpadComma,
    NumpadDecimal,

    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    ScrollLock,
    PrintScreen,
    Pause,

    Home,
    Delete,
    End,
    PageUp,
    PageDown,
    Insert,

    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,

    // No corresponding scancode
    Ampersand,   // &
    Asterisk,    // *
    At,          // @
    Caret,       // ^
    Colon,       // :
    Dollar,      // $
    Exclaim,     // !
    Greater,     // >
    Hash,        // #
    ParenLeft,   // (
    Less,        // <
    Percent,     // %
    Plus,        // +
    Question,    // ?
    QuoteDouble, // "
    ParenRight,  // )
    Underscore,  // _
}
