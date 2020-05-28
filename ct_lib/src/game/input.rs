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

    pub screen_framebuffer_width: u32,
    pub screen_framebuffer_height: u32,
    pub screen_framebuffer_dimensions_changed: bool,

    pub gamepad: GamepadState,

    /// Measured time since last frame
    pub deltatime: f32,
    /// Optimal time a frame should take at our current refresh rate
    /// NOTE: target_deltatime = 1 / monitor_refresh_rate_hz
    pub target_deltatime: f32,
    pub real_world_uptime: f64,

    pub audio_dsp_time: f64,
    pub audio_frames_per_second: usize,
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

pub const TOUCH_MAX_FINGER_COUNT: usize = 4;

#[derive(Default, Clone)]
pub struct TouchState {
    pub has_move_event: bool,
    pub has_press_event: bool,
    pub has_release_event: bool,

    pub fingers: [TouchFinger; TOUCH_MAX_FINGER_COUNT],
    pub fingers_previous: [TouchFinger; TOUCH_MAX_FINGER_COUNT],
}

#[derive(Default, Clone)]
pub struct TouchFinger {
    pub state: ButtonState,

    // Pos in [0, screen_width - 1]x[0, screen_height - 1] (left to right and top to bottom)
    pub pos_x: i32,
    pub pos_y: i32,

    pub delta_x: i32,
    pub delta_y: i32,
}

impl TouchState {
    pub fn touchstate_clear_transitions(&mut self) {
        self.has_move_event = false;
        self.has_press_event = false;
        self.has_release_event = false;

        self.fingers.iter_mut().for_each(|finger| {
            finger.state.transition_count = 0;
            finger.delta_x = 0;
            finger.delta_y = 0;

            // Remove inactive fingers from screen
            if !finger.state.is_pressed && !finger.state.recently_released() {
                finger.pos_x = -1;
                finger.pos_y = -1;
            }
        });
        self.fingers_previous = self.fingers.clone();
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
            keycode: Keycode::Unknown,
            scancode: Scancode::Unknown,
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
        let mut key = self
            .keys
            .get_mut(&scancode)
            .expect("Scancode is not in keystates list");
        if key.keycode != keycode {
            // NOTE: Update keycode (keycode may differ for example if the player changed their
            // input language)
            key.keycode = keycode;
        }
        key.button.process_event(is_pressed, is_repeat, tick);
    }

    pub fn is_down(&self, scancode: Scancode) -> bool {
        self.keys[&scancode].button.is_pressed
    }

    pub fn recently_pressed(&self, scancode: Scancode) -> bool {
        self.keys[&scancode].button.recently_pressed()
    }

    pub fn recently_pressed_or_repeated(&self, scancode: Scancode) -> bool {
        self.keys[&scancode].button.recently_pressed_or_repeated()
    }

    pub fn recently_released(&self, scancode: Scancode) -> bool {
        self.keys[&scancode].button.recently_released()
    }
}

//--------------------------------------------------------------------------------------------------
// Handling digit-keys

const SCANCODE_DIGITS: [Scancode; 10] = [
    Scancode::Num0,
    Scancode::Num1,
    Scancode::Num2,
    Scancode::Num3,
    Scancode::Num4,
    Scancode::Num5,
    Scancode::Num6,
    Scancode::Num7,
    Scancode::Num8,
    Scancode::Num9,
];

const SCANCODE_DIGITS_KEYPAD: [Scancode; 10] = [
    Scancode::Kp0,
    Scancode::Kp1,
    Scancode::Kp2,
    Scancode::Kp3,
    Scancode::Kp4,
    Scancode::Kp5,
    Scancode::Kp6,
    Scancode::Kp7,
    Scancode::Kp8,
    Scancode::Kp9,
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
    Unknown,
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
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    Return,
    Escape,
    Backspace,
    Tab,
    Space,
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Backslash,
    NonUsHash,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Period,
    Slash,
    CapsLock,
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
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLockClear,
    KpDivide,
    KpMultiply,
    KpMinus,
    KpPlus,
    KpEnter,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    Kp0,
    KpPeriod,
    NonUsBackslash,
    Application,
    Power,
    KpEquals,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Execute,
    Help,
    Menu,
    Select,
    Stop,
    Again,
    Undo,
    Cut,
    Copy,
    Paste,
    Find,
    Mute,
    VolumeUp,
    VolumeDown,
    KpComma,
    KpEqualsAS400,
    International1,
    International2,
    International3,
    International4,
    International5,
    International6,
    International7,
    International8,
    International9,
    Lang1,
    Lang2,
    Lang3,
    Lang4,
    Lang5,
    Lang6,
    Lang7,
    Lang8,
    Lang9,
    AltErase,
    SysReq,
    Cancel,
    Clear,
    Prior,
    Return2,
    Separator,
    Out,
    Oper,
    ClearAgain,
    CrSel,
    ExSel,
    Kp00,
    Kp000,
    ThousandsSeparator,
    DecimalSeparator,
    CurrencyUnit,
    CurrencySubUnit,
    KpLeftParen,
    KpRightParen,
    KpLeftBrace,
    KpRightBrace,
    KpTab,
    KpBackspace,
    KpA,
    KpB,
    KpC,
    KpD,
    KpE,
    KpF,
    KpXor,
    KpPower,
    KpPercent,
    KpLess,
    KpGreater,
    KpAmpersand,
    KpDblAmpersand,
    KpVerticalBar,
    KpDblVerticalBar,
    KpColon,
    KpHash,
    KpSpace,
    KpAt,
    KpExclam,
    KpMemStore,
    KpMemRecall,
    KpMemClear,
    KpMemAdd,
    KpMemSubtract,
    KpMemMultiply,
    KpMemDivide,
    KpPlusMinus,
    KpClear,
    KpClearEntry,
    KpBinary,
    KpOctal,
    KpDecimal,
    KpHexadecimal,
    LCtrl,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui,
    Mode,
    AudioNext,
    AudioPrev,
    AudioStop,
    AudioPlay,
    AudioMute,
    MediaSelect,
    Www,
    Mail,
    Calculator,
    Computer,
    AcSearch,
    AcHome,
    AcBack,
    AcForward,
    AcStop,
    AcRefresh,
    AcBookmarks,
    BrightnessDown,
    BrightnessUp,
    DisplaySwitch,
    KbdIllumToggle,
    KbdIllumDown,
    KbdIllumUp,
    Eject,
    Sleep,
    App1,
    App2,
    Num,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Keycode {
    Unknown,
    Backspace,
    Tab,
    Return,
    Escape,
    Space,
    Exclaim,
    Quotedbl,
    Hash,
    Dollar,
    Percent,
    Ampersand,
    Quote,
    LeftParen,
    RightParen,
    Asterisk,
    Plus,
    Comma,
    Minus,
    Period,
    Slash,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Colon,
    Semicolon,
    Less,
    Equals,
    Greater,
    Question,
    At,
    LeftBracket,
    Backslash,
    RightBracket,
    Caret,
    Underscore,
    Backquote,
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
    Delete,
    CapsLock,
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
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLockClear,
    KpDivide,
    KpMultiply,
    KpMinus,
    KpPlus,
    KpEnter,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    Kp0,
    KpPeriod,
    Application,
    Power,
    KpEquals,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Execute,
    Help,
    Menu,
    Select,
    Stop,
    Again,
    Undo,
    Cut,
    Copy,
    Paste,
    Find,
    Mute,
    VolumeUp,
    VolumeDown,
    KpComma,
    KpEqualsAS400,
    AltErase,
    Sysreq,
    Cancel,
    Clear,
    Prior,
    Return2,
    Separator,
    Out,
    Oper,
    ClearAgain,
    CrSel,
    ExSel,
    Kp00,
    Kp000,
    ThousandsSeparator,
    DecimalSeparator,
    CurrencyUnit,
    CurrencySubUnit,
    KpLeftParen,
    KpRightParen,
    KpLeftBrace,
    KpRightBrace,
    KpTab,
    KpBackspace,
    KpA,
    KpB,
    KpC,
    KpD,
    KpE,
    KpF,
    KpXor,
    KpPower,
    KpPercent,
    KpLess,
    KpGreater,
    KpAmpersand,
    KpDblAmpersand,
    KpVerticalBar,
    KpDblVerticalBar,
    KpColon,
    KpHash,
    KpSpace,
    KpAt,
    KpExclam,
    KpMemStore,
    KpMemRecall,
    KpMemClear,
    KpMemAdd,
    KpMemSubtract,
    KpMemMultiply,
    KpMemDivide,
    KpPlusMinus,
    KpClear,
    KpClearEntry,
    KpBinary,
    KpOctal,
    KpDecimal,
    KpHexadecimal,
    LCtrl,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui,
    Mode,
    AudioNext,
    AudioPrev,
    AudioStop,
    AudioPlay,
    AudioMute,
    MediaSelect,
    Www,
    Mail,
    Calculator,
    Computer,
    AcSearch,
    AcHome,
    AcBack,
    AcForward,
    AcStop,
    AcRefresh,
    AcBookmarks,
    BrightnessDown,
    BrightnessUp,
    DisplaySwitch,
    KbdIllumToggle,
    KbdIllumDown,
    KbdIllumUp,
    Eject,
    Sleep,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Keymod {
    None,
    LShift,
    Rshift,
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,
    LGui,
    RGui,
    Num,
    Caps,
    Mode,
    Reserved,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum KeymodSimple {
    Ctrl,
    Shift,
    Alt,
    Gui,
}
