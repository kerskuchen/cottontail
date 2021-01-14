use std::collections::HashMap;

pub type FingerPlatformId = i64;
pub type GamepadPlatformId = i64;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum GamepadButton {
    Start,
    Back,
    Home,

    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    ActionUp,
    ActionDown,
    ActionLeft,
    ActionRight,

    StickLeft,
    StickRight,

    TriggerLeft1,
    TriggerLeft2,

    TriggerRight1,
    TriggerRight2,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum GamepadAxis {
    StickLeftX,
    StickLeftY,
    StickRightX,
    StickRightY,

    TriggerLeft,
    TriggerRight,
}

#[derive(Clone, PartialEq)]
pub struct GamepadPlatformState {
    pub buttons: HashMap<GamepadButton, bool>,
    pub axes: HashMap<GamepadAxis, f32>,
}

impl Default for GamepadPlatformState {
    fn default() -> Self {
        let buttons_list = [
            GamepadButton::Start,
            GamepadButton::Back,
            GamepadButton::Home,
            GamepadButton::MoveUp,
            GamepadButton::MoveDown,
            GamepadButton::MoveLeft,
            GamepadButton::MoveRight,
            GamepadButton::ActionUp,
            GamepadButton::ActionDown,
            GamepadButton::ActionLeft,
            GamepadButton::ActionRight,
            GamepadButton::StickLeft,
            GamepadButton::StickRight,
            GamepadButton::TriggerLeft1,
            GamepadButton::TriggerLeft2,
            GamepadButton::TriggerRight1,
            GamepadButton::TriggerRight2,
        ];
        let axes_list = [
            GamepadAxis::StickLeftX,
            GamepadAxis::StickLeftY,
            GamepadAxis::StickRightX,
            GamepadAxis::StickRightY,
            GamepadAxis::TriggerLeft,
            GamepadAxis::TriggerRight,
        ];
        let buttons = buttons_list.iter().map(|&button| (button, false)).collect();
        let axes = axes_list.iter().map(|&axis| (axis, 0.0)).collect();
        GamepadPlatformState { buttons, axes }
    }
}
impl GamepadPlatformState {
    pub fn new() -> GamepadPlatformState {
        GamepadPlatformState::default()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

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
