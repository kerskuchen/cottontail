pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

pub type FingerId = usize;
pub type FingerPlatformId = i64;

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
