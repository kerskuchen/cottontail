use crate::{Keycode, Scancode};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Platform specific input

/// Given a deadzone_threshold in [0.0, 1.0[
/// Outputs [-1.0, 1.0] if axisValue in [-1.0, -deadzone_threshold] u [deadzone_threshold, 1.0]
/// or 0.0 if axisValue in ]-deadzone_threshold, deadzone_threshold[
pub fn _process_gamepad_axis(
    controller: &sdl2::controller::GameController,
    axis: sdl2::controller::Axis,
    deadzone_threshold: f32,
) -> f32 {
    debug_assert!(0.0 <= deadzone_threshold && deadzone_threshold < 1.0);

    let mut axis_value = controller.axis(axis) as f32;

    const CONTROLLER_AXIS_ABSMAX_POSITIVE: f32 = 32767.0;
    const CONTROLLER_AXIS_ABSMAX_NEGATIVE: f32 = 32768.0;

    if axis_value >= 0.0 {
        axis_value /= CONTROLLER_AXIS_ABSMAX_POSITIVE;
        axis_value = if axis_value >= deadzone_threshold {
            (axis_value - deadzone_threshold) / (1.0 - deadzone_threshold)
        } else {
            0.0
        }
    } else {
        axis_value /= CONTROLLER_AXIS_ABSMAX_NEGATIVE;
        axis_value = if axis_value <= -deadzone_threshold {
            (axis_value + deadzone_threshold) / (1.0 - deadzone_threshold)
        } else {
            0.0
        }
    }

    axis_value
}

pub fn scancode_to_our_scancode(scancode: sdl2::keyboard::Scancode) -> Scancode {
    match scancode {
        sdl2::keyboard::Scancode::Num1 => Scancode::Digit1,
        sdl2::keyboard::Scancode::Num2 => Scancode::Digit2,
        sdl2::keyboard::Scancode::Num3 => Scancode::Digit3,
        sdl2::keyboard::Scancode::Num4 => Scancode::Digit4,
        sdl2::keyboard::Scancode::Num5 => Scancode::Digit5,
        sdl2::keyboard::Scancode::Num6 => Scancode::Digit6,
        sdl2::keyboard::Scancode::Num7 => Scancode::Digit7,
        sdl2::keyboard::Scancode::Num8 => Scancode::Digit8,
        sdl2::keyboard::Scancode::Num9 => Scancode::Digit9,
        sdl2::keyboard::Scancode::Num0 => Scancode::Digit0,

        sdl2::keyboard::Scancode::A => Scancode::A,
        sdl2::keyboard::Scancode::B => Scancode::B,
        sdl2::keyboard::Scancode::C => Scancode::C,
        sdl2::keyboard::Scancode::D => Scancode::D,
        sdl2::keyboard::Scancode::E => Scancode::E,
        sdl2::keyboard::Scancode::F => Scancode::F,
        sdl2::keyboard::Scancode::G => Scancode::G,
        sdl2::keyboard::Scancode::H => Scancode::H,
        sdl2::keyboard::Scancode::I => Scancode::I,
        sdl2::keyboard::Scancode::J => Scancode::J,
        sdl2::keyboard::Scancode::K => Scancode::K,
        sdl2::keyboard::Scancode::L => Scancode::L,
        sdl2::keyboard::Scancode::M => Scancode::M,
        sdl2::keyboard::Scancode::N => Scancode::N,
        sdl2::keyboard::Scancode::O => Scancode::O,
        sdl2::keyboard::Scancode::P => Scancode::P,
        sdl2::keyboard::Scancode::Q => Scancode::Q,
        sdl2::keyboard::Scancode::R => Scancode::R,
        sdl2::keyboard::Scancode::S => Scancode::S,
        sdl2::keyboard::Scancode::T => Scancode::T,
        sdl2::keyboard::Scancode::U => Scancode::U,
        sdl2::keyboard::Scancode::V => Scancode::V,
        sdl2::keyboard::Scancode::W => Scancode::W,
        sdl2::keyboard::Scancode::X => Scancode::X,
        sdl2::keyboard::Scancode::Y => Scancode::Y,
        sdl2::keyboard::Scancode::Z => Scancode::Z,

        sdl2::keyboard::Scancode::AudioMute => Scancode::AudioVolumeMute,
        sdl2::keyboard::Scancode::VolumeUp => Scancode::AudioVolumeUp,
        sdl2::keyboard::Scancode::VolumeDown => Scancode::AudioVolumeDown,

        sdl2::keyboard::Scancode::Equals => Scancode::Equal,
        sdl2::keyboard::Scancode::Minus => Scancode::Minus,
        sdl2::keyboard::Scancode::LeftBracket => Scancode::BracketRight,
        sdl2::keyboard::Scancode::RightBracket => Scancode::BracketLeft,
        sdl2::keyboard::Scancode::Slash => Scancode::Slash,
        sdl2::keyboard::Scancode::Backslash => Scancode::Backslash,

        sdl2::keyboard::Scancode::Escape => Scancode::Escape,
        sdl2::keyboard::Scancode::Return => Scancode::Enter,
        sdl2::keyboard::Scancode::Tab => Scancode::Tab,
        sdl2::keyboard::Scancode::Space => Scancode::Space,
        sdl2::keyboard::Scancode::Backspace => Scancode::Backspace,

        sdl2::keyboard::Scancode::Apostrophe => Scancode::Quote,
        sdl2::keyboard::Scancode::Grave => Scancode::Backquote,
        sdl2::keyboard::Scancode::Semicolon => Scancode::Semicolon,
        sdl2::keyboard::Scancode::Comma => Scancode::Comma,
        sdl2::keyboard::Scancode::Period => Scancode::Period,

        sdl2::keyboard::Scancode::CapsLock => Scancode::CapsLock,
        sdl2::keyboard::Scancode::LGui => Scancode::MetaLeft, // i.e. Windows key
        sdl2::keyboard::Scancode::RGui => Scancode::MetaRight, // i.e. Windows key
        sdl2::keyboard::Scancode::LShift => Scancode::ShiftLeft,
        sdl2::keyboard::Scancode::RShift => Scancode::ShiftRight,
        sdl2::keyboard::Scancode::LAlt => Scancode::AltLeft,
        sdl2::keyboard::Scancode::RAlt => Scancode::AltRight,
        sdl2::keyboard::Scancode::LCtrl => Scancode::ControlLeft,
        sdl2::keyboard::Scancode::RCtrl => Scancode::ControlRight,
        sdl2::keyboard::Scancode::Application => Scancode::ContextMenu,

        sdl2::keyboard::Scancode::F1 => Scancode::F1,
        sdl2::keyboard::Scancode::F2 => Scancode::F2,
        sdl2::keyboard::Scancode::F3 => Scancode::F3,
        sdl2::keyboard::Scancode::F4 => Scancode::F4,
        sdl2::keyboard::Scancode::F5 => Scancode::F5,
        sdl2::keyboard::Scancode::F6 => Scancode::F6,
        sdl2::keyboard::Scancode::F7 => Scancode::F7,
        sdl2::keyboard::Scancode::F8 => Scancode::F8,
        sdl2::keyboard::Scancode::F9 => Scancode::F9,
        sdl2::keyboard::Scancode::F10 => Scancode::F10,
        sdl2::keyboard::Scancode::F11 => Scancode::F11,
        sdl2::keyboard::Scancode::F12 => Scancode::F12,

        sdl2::keyboard::Scancode::NumLockClear => Scancode::Numlock,
        sdl2::keyboard::Scancode::KpMultiply => Scancode::NumpadMultiply,
        sdl2::keyboard::Scancode::KpPlus => Scancode::NumpadAdd,
        sdl2::keyboard::Scancode::KpDivide => Scancode::NumpadDivide,
        sdl2::keyboard::Scancode::KpEnter => Scancode::NumpadEnter,
        sdl2::keyboard::Scancode::KpMinus => Scancode::NumpadSubtract,
        sdl2::keyboard::Scancode::KpEquals => Scancode::NumpadEqual,
        sdl2::keyboard::Scancode::KpComma => Scancode::NumpadComma,
        sdl2::keyboard::Scancode::KpDecimal => Scancode::NumpadDecimal,

        sdl2::keyboard::Scancode::Kp0 => Scancode::Numpad0,
        sdl2::keyboard::Scancode::Kp1 => Scancode::Numpad1,
        sdl2::keyboard::Scancode::Kp2 => Scancode::Numpad2,
        sdl2::keyboard::Scancode::Kp3 => Scancode::Numpad3,
        sdl2::keyboard::Scancode::Kp4 => Scancode::Numpad4,
        sdl2::keyboard::Scancode::Kp5 => Scancode::Numpad5,
        sdl2::keyboard::Scancode::Kp6 => Scancode::Numpad6,
        sdl2::keyboard::Scancode::Kp7 => Scancode::Numpad7,
        sdl2::keyboard::Scancode::Kp8 => Scancode::Numpad8,
        sdl2::keyboard::Scancode::Kp9 => Scancode::Numpad9,

        sdl2::keyboard::Scancode::ScrollLock => Scancode::ScrollLock,
        sdl2::keyboard::Scancode::PrintScreen => Scancode::PrintScreen,
        sdl2::keyboard::Scancode::Pause => Scancode::Pause,

        sdl2::keyboard::Scancode::Home => Scancode::Home,
        sdl2::keyboard::Scancode::Delete => Scancode::Delete,
        sdl2::keyboard::Scancode::End => Scancode::End,
        sdl2::keyboard::Scancode::PageUp => Scancode::PageUp,
        sdl2::keyboard::Scancode::PageDown => Scancode::PageDown,
        sdl2::keyboard::Scancode::Insert => Scancode::Insert,

        sdl2::keyboard::Scancode::Left => Scancode::ArrowLeft,
        sdl2::keyboard::Scancode::Up => Scancode::ArrowUp,
        sdl2::keyboard::Scancode::Right => Scancode::ArrowRight,
        sdl2::keyboard::Scancode::Down => Scancode::ArrowDown,

        _ => Scancode::Unidentified,
    }
}
pub fn keycode_to_our_keycode(keycode: sdl2::keyboard::Keycode) -> Keycode {
    match keycode {
        sdl2::keyboard::Keycode::Num1 => Keycode::Digit1,
        sdl2::keyboard::Keycode::Num2 => Keycode::Digit2,
        sdl2::keyboard::Keycode::Num3 => Keycode::Digit3,
        sdl2::keyboard::Keycode::Num4 => Keycode::Digit4,
        sdl2::keyboard::Keycode::Num5 => Keycode::Digit5,
        sdl2::keyboard::Keycode::Num6 => Keycode::Digit6,
        sdl2::keyboard::Keycode::Num7 => Keycode::Digit7,
        sdl2::keyboard::Keycode::Num8 => Keycode::Digit8,
        sdl2::keyboard::Keycode::Num9 => Keycode::Digit9,
        sdl2::keyboard::Keycode::Num0 => Keycode::Digit0,

        sdl2::keyboard::Keycode::A => Keycode::A,
        sdl2::keyboard::Keycode::B => Keycode::B,
        sdl2::keyboard::Keycode::C => Keycode::C,
        sdl2::keyboard::Keycode::D => Keycode::D,
        sdl2::keyboard::Keycode::E => Keycode::E,
        sdl2::keyboard::Keycode::F => Keycode::F,
        sdl2::keyboard::Keycode::G => Keycode::G,
        sdl2::keyboard::Keycode::H => Keycode::H,
        sdl2::keyboard::Keycode::I => Keycode::I,
        sdl2::keyboard::Keycode::J => Keycode::J,
        sdl2::keyboard::Keycode::K => Keycode::K,
        sdl2::keyboard::Keycode::L => Keycode::L,
        sdl2::keyboard::Keycode::M => Keycode::M,
        sdl2::keyboard::Keycode::N => Keycode::N,
        sdl2::keyboard::Keycode::O => Keycode::O,
        sdl2::keyboard::Keycode::P => Keycode::P,
        sdl2::keyboard::Keycode::Q => Keycode::Q,
        sdl2::keyboard::Keycode::R => Keycode::R,
        sdl2::keyboard::Keycode::S => Keycode::S,
        sdl2::keyboard::Keycode::T => Keycode::T,
        sdl2::keyboard::Keycode::U => Keycode::U,
        sdl2::keyboard::Keycode::V => Keycode::V,
        sdl2::keyboard::Keycode::W => Keycode::W,
        sdl2::keyboard::Keycode::X => Keycode::X,
        sdl2::keyboard::Keycode::Y => Keycode::Y,
        sdl2::keyboard::Keycode::Z => Keycode::Z,

        sdl2::keyboard::Keycode::AudioMute => Keycode::AudioVolumeMute,
        sdl2::keyboard::Keycode::VolumeUp => Keycode::AudioVolumeUp,
        sdl2::keyboard::Keycode::VolumeDown => Keycode::AudioVolumeDown,

        sdl2::keyboard::Keycode::Equals => Keycode::Equal,
        sdl2::keyboard::Keycode::Minus => Keycode::Minus,
        sdl2::keyboard::Keycode::LeftBracket => Keycode::BracketRight,
        sdl2::keyboard::Keycode::RightBracket => Keycode::BracketLeft,
        sdl2::keyboard::Keycode::Slash => Keycode::Slash,
        sdl2::keyboard::Keycode::Backslash => Keycode::Backslash,

        sdl2::keyboard::Keycode::Escape => Keycode::Escape,
        sdl2::keyboard::Keycode::Return => Keycode::Enter,
        sdl2::keyboard::Keycode::Tab => Keycode::Tab,
        sdl2::keyboard::Keycode::Space => Keycode::Space,
        sdl2::keyboard::Keycode::Backspace => Keycode::Backspace,

        sdl2::keyboard::Keycode::Quote => Keycode::Quote,
        sdl2::keyboard::Keycode::Backquote => Keycode::Backquote,
        sdl2::keyboard::Keycode::Semicolon => Keycode::Semicolon,
        sdl2::keyboard::Keycode::Comma => Keycode::Comma,
        sdl2::keyboard::Keycode::Period => Keycode::Period,

        sdl2::keyboard::Keycode::CapsLock => Keycode::CapsLock,
        sdl2::keyboard::Keycode::LGui => Keycode::MetaLeft,
        sdl2::keyboard::Keycode::RGui => Keycode::MetaRight,
        sdl2::keyboard::Keycode::LShift => Keycode::ShiftLeft,
        sdl2::keyboard::Keycode::RShift => Keycode::ShiftRight,
        sdl2::keyboard::Keycode::LAlt => Keycode::AltLeft,
        sdl2::keyboard::Keycode::RAlt => Keycode::AltRight,
        sdl2::keyboard::Keycode::LCtrl => Keycode::ControlLeft,
        sdl2::keyboard::Keycode::RCtrl => Keycode::ControlRight,
        sdl2::keyboard::Keycode::Application => Keycode::ContextMenu,

        sdl2::keyboard::Keycode::F1 => Keycode::F1,
        sdl2::keyboard::Keycode::F2 => Keycode::F2,
        sdl2::keyboard::Keycode::F3 => Keycode::F3,
        sdl2::keyboard::Keycode::F4 => Keycode::F4,
        sdl2::keyboard::Keycode::F5 => Keycode::F5,
        sdl2::keyboard::Keycode::F6 => Keycode::F6,
        sdl2::keyboard::Keycode::F7 => Keycode::F7,
        sdl2::keyboard::Keycode::F8 => Keycode::F8,
        sdl2::keyboard::Keycode::F9 => Keycode::F9,
        sdl2::keyboard::Keycode::F10 => Keycode::F10,
        sdl2::keyboard::Keycode::F11 => Keycode::F11,
        sdl2::keyboard::Keycode::F12 => Keycode::F12,

        sdl2::keyboard::Keycode::NumLockClear => Keycode::Numlock,
        sdl2::keyboard::Keycode::KpMultiply => Keycode::NumpadMultiply,
        sdl2::keyboard::Keycode::KpPlus => Keycode::NumpadAdd,
        sdl2::keyboard::Keycode::KpDivide => Keycode::NumpadDivide,
        sdl2::keyboard::Keycode::KpEnter => Keycode::NumpadEnter,
        sdl2::keyboard::Keycode::KpMinus => Keycode::NumpadSubtract,
        sdl2::keyboard::Keycode::KpEquals => Keycode::NumpadEqual,
        sdl2::keyboard::Keycode::KpComma => Keycode::NumpadComma,
        sdl2::keyboard::Keycode::KpDecimal => Keycode::NumpadDecimal,

        sdl2::keyboard::Keycode::Kp0 => Keycode::Numpad0,
        sdl2::keyboard::Keycode::Kp1 => Keycode::Numpad1,
        sdl2::keyboard::Keycode::Kp2 => Keycode::Numpad2,
        sdl2::keyboard::Keycode::Kp3 => Keycode::Numpad3,
        sdl2::keyboard::Keycode::Kp4 => Keycode::Numpad4,
        sdl2::keyboard::Keycode::Kp5 => Keycode::Numpad5,
        sdl2::keyboard::Keycode::Kp6 => Keycode::Numpad6,
        sdl2::keyboard::Keycode::Kp7 => Keycode::Numpad7,
        sdl2::keyboard::Keycode::Kp8 => Keycode::Numpad8,
        sdl2::keyboard::Keycode::Kp9 => Keycode::Numpad9,

        sdl2::keyboard::Keycode::ScrollLock => Keycode::ScrollLock,
        sdl2::keyboard::Keycode::PrintScreen => Keycode::PrintScreen,
        sdl2::keyboard::Keycode::Pause => Keycode::Pause,

        sdl2::keyboard::Keycode::Home => Keycode::Home,
        sdl2::keyboard::Keycode::Delete => Keycode::Delete,
        sdl2::keyboard::Keycode::End => Keycode::End,
        sdl2::keyboard::Keycode::PageUp => Keycode::PageUp,
        sdl2::keyboard::Keycode::PageDown => Keycode::PageDown,
        sdl2::keyboard::Keycode::Insert => Keycode::Insert,

        sdl2::keyboard::Keycode::Left => Keycode::ArrowLeft,
        sdl2::keyboard::Keycode::Up => Keycode::ArrowUp,
        sdl2::keyboard::Keycode::Right => Keycode::ArrowRight,
        sdl2::keyboard::Keycode::Down => Keycode::ArrowDown,

        sdl2::keyboard::Keycode::Ampersand => Keycode::Ampersand,
        sdl2::keyboard::Keycode::Asterisk => Keycode::Asterisk,
        sdl2::keyboard::Keycode::At => Keycode::At,
        sdl2::keyboard::Keycode::Caret => Keycode::Caret,
        sdl2::keyboard::Keycode::Colon => Keycode::Colon,
        sdl2::keyboard::Keycode::Dollar => Keycode::Dollar,
        sdl2::keyboard::Keycode::Exclaim => Keycode::Exclaim,
        sdl2::keyboard::Keycode::Greater => Keycode::Greater,
        sdl2::keyboard::Keycode::Hash => Keycode::Hash,
        sdl2::keyboard::Keycode::LeftParen => Keycode::ParenLeft,
        sdl2::keyboard::Keycode::Less => Keycode::Less,
        sdl2::keyboard::Keycode::Percent => Keycode::Percent,
        sdl2::keyboard::Keycode::Plus => Keycode::Plus,
        sdl2::keyboard::Keycode::Question => Keycode::Question,
        sdl2::keyboard::Keycode::Quotedbl => Keycode::QuoteDouble,
        sdl2::keyboard::Keycode::RightParen => Keycode::ParenRight,
        sdl2::keyboard::Keycode::Underscore => Keycode::Underscore,

        _ => Keycode::Unidentified,
    }
}
