pub use crate::input::FingerId;
pub use crate::input::KeyModifier;
use ct_lib_window::add_platform_window_command;
pub use ct_lib_window::input::{GamepadButton, Keycode, MouseButton, Scancode};

use crate::*;

//--------------------------------------------------------------------------------------------------
// Global objects

#[inline]
pub fn get_camera() -> &'static mut GameCamera {
    &mut get_globals().camera
}

#[inline]
pub fn get_random_generator() -> &'static mut Random {
    &mut get_globals().random
}

//--------------------------------------------------------------------------------------------------
// CANVAS

#[inline]
pub fn canvas_width() -> f32 {
    get_globals().canvas_width
}

#[inline]
pub fn canvas_height() -> f32 {
    get_globals().canvas_height
}

#[inline]
pub fn canvas_dimensions() -> Vec2 {
    let globals = get_globals();
    Vec2::new(globals.canvas_width, globals.canvas_height)
}

//--------------------------------------------------------------------------------------------------
// TIMING

#[inline]
pub fn time_deltatime() -> f32 {
    get_globals().deltatime
}

#[inline]
pub fn time_deltatime_without_speedup_factor() -> f32 {
    get_globals().deltatime_without_speedup
}

#[inline]
pub fn time_speed_factor_user() -> f32 {
    get_globals().deltatime_speed_factor_user
}

#[inline]
pub fn time_speed_factor_debug() -> f32 {
    get_globals().deltatime_speed_factor_debug
}

#[inline]
pub fn time_deltatime_speed_factor_total() -> f32 {
    get_globals().deltatime_speed_factor_user * get_globals().deltatime_speed_factor_debug
}

#[inline]
pub fn time_since_startup() -> f64 {
    get_globals().time_since_startup
}

//--------------------------------------------------------------------------------------------------
// WINDOW

#[inline]
pub fn window_has_focus() -> bool {
    get_input().has_focus
}

#[inline]
pub fn window_is_fullscreen() -> bool {
    get_input().screen_is_fullscreen
}

#[inline]
pub fn window_screen_width() -> u32 {
    get_input().screen_framebuffer_width
}

#[inline]
pub fn window_screen_height() -> u32 {
    get_input().screen_framebuffer_height
}

#[inline]
pub fn window_screen_dimensions() -> (u32, u32) {
    let input = get_input();
    (
        input.screen_framebuffer_width,
        input.screen_framebuffer_height,
    )
}

#[inline]
pub fn platform_window_toggle_fullscreen() {
    add_platform_window_command(PlatformWindowCommand::FullscreenToggle)
}

#[inline]
pub fn platform_window_start_textinput_mode(
    inputrect_x: i32,
    inputrect_y: i32,
    inputrect_width: u32,
    inputrect_height: u32,
) {
    add_platform_window_command(PlatformWindowCommand::TextinputStart {
        inputrect_x,
        inputrect_y,
        inputrect_width,
        inputrect_height,
    })
}

#[inline]
pub fn platform_window_stop_textinput_mode() {
    add_platform_window_command(PlatformWindowCommand::TextinputStop)
}

#[inline]
pub fn platform_window_set_cursor_grabbing(enable_grab: bool) {
    add_platform_window_command(PlatformWindowCommand::ScreenSetGrabInput(enable_grab))
}

#[inline]
pub fn platform_window_set_allow_windowed_mode(allow: bool) {
    add_platform_window_command(PlatformWindowCommand::WindowedModeAllow(allow))
}

#[inline]
pub fn platform_window_set_allow_window_resizing_by_user(allow: bool) {
    add_platform_window_command(PlatformWindowCommand::WindowedModeAllowResizing(allow))
}

#[inline]
pub fn platform_window_set_window_size(
    width: u32,
    height: u32,
    minimum_width: u32,
    minimum_height: u32,
) {
    add_platform_window_command(PlatformWindowCommand::WindowedModeSetSize {
        width,
        height,
        minimum_width,
        minimum_height,
    })
}

#[inline]
pub fn platform_window_shutdown() {
    add_platform_window_command(PlatformWindowCommand::Shutdown)
}

#[inline]
pub fn platform_window_restart() {
    add_platform_window_command(PlatformWindowCommand::Restart)
}
//--------------------------------------------------------------------------------------------------
// KEYBOARD INPUT

// Keyboard events

#[inline]
pub fn key_press_event_happened() -> bool {
    get_input().keyboard.has_press_event
}

#[inline]
pub fn key_release_event_happened() -> bool {
    get_input().keyboard.has_release_event
}

#[inline]
pub fn key_repeat_event_happened() -> bool {
    get_input().keyboard.has_system_repeat_event
}

// Keyboard regular keys

#[inline]
pub fn key_is_down(scancode: Scancode) -> bool {
    get_input().keyboard.is_down(scancode)
}

#[inline]
pub fn key_recently_pressed(scancode: Scancode) -> bool {
    get_input().keyboard.recently_pressed(scancode)
}

#[inline]
pub fn key_recently_pressed_ignore_repeat(scancode: Scancode) -> bool {
    get_input()
        .keyboard
        .recently_pressed_ignore_repeat(scancode)
}

#[inline]
pub fn key_recently_released(scancode: Scancode) -> bool {
    get_input().keyboard.recently_released(scancode)
}

// Keyboard digit keys

#[inline]
pub fn key_is_down_digit(digit: usize) -> bool {
    get_input().keyboard.is_down_digit(digit)
}

#[inline]
pub fn key_recently_pressed_digit(digit: usize) -> bool {
    get_input().keyboard.recently_pressed_digit(digit)
}

#[inline]
pub fn key_recently_pressed_ignore_repeat_digit(digit: usize) -> bool {
    get_input()
        .keyboard
        .recently_pressed_ignore_repeat_digit(digit)
}

#[inline]
pub fn key_recently_released_digit(digit: usize) -> bool {
    get_input().keyboard.recently_released_digit(digit)
}

// Keyboard modifier keys

#[inline]
pub fn key_is_down_modifier(modifier: KeyModifier) -> bool {
    get_input().keyboard.is_down_modifier(modifier)
}

#[inline]
pub fn key_recently_pressed_modifier(modifier: KeyModifier) -> bool {
    get_input().keyboard.recently_pressed_modifier(modifier)
}

#[inline]
pub fn key_recently_pressed_ignore_repeat_modifier(modifier: KeyModifier) -> bool {
    get_input()
        .keyboard
        .recently_pressed_ignore_repeat_modifier(modifier)
}

#[inline]
pub fn key_recently_released_modifier(modifier: KeyModifier) -> bool {
    get_input().keyboard.recently_released_modifier(modifier)
}

//--------------------------------------------------------------------------------------------------
// MOUSE INPUT

// Mouse events

#[inline]
pub fn mouse_press_event_happened() -> bool {
    get_input().mouse.has_press_event
}

#[inline]
pub fn mouse_release_event_happened() -> bool {
    get_input().mouse.has_release_event
}

#[inline]
pub fn mouse_move_event_happened() -> bool {
    get_input().mouse.has_move_event
}

#[inline]
pub fn mouse_wheel_event_happened() -> bool {
    get_input().mouse.has_wheel_event
}

// Mouse position / delta

#[inline]
pub fn mouse_pos_screen() -> Vec2 {
    get_globals().cursors.mouse.pos_screen
}

#[inline]
pub fn mouse_pos_canvas() -> Vec2 {
    get_globals().cursors.mouse.pos_canvas
}

#[inline]
pub fn mouse_pos_world() -> Vec2 {
    get_globals().cursors.mouse.pos_world
}

#[inline]
pub fn mouse_delta_screen() -> Vec2 {
    get_globals().cursors.mouse.delta_screen
}

#[inline]
pub fn mouse_delta_canvas() -> Vec2 {
    get_globals().cursors.mouse.delta_canvas
}

#[inline]
pub fn mouse_delta_world() -> Vec2 {
    get_globals().cursors.mouse.delta_world
}

// Mouse wheel

pub fn mouse_wheel_delta() -> i32 {
    get_input().mouse.wheel_delta
}

// Mouse button left

#[inline]
pub fn mouse_is_down_left() -> bool {
    get_input().mouse.button_left.is_pressed
}

#[inline]
pub fn mouse_recently_pressed_left() -> bool {
    get_input().mouse.button_left.recently_pressed()
}

#[inline]
pub fn mouse_recently_released_left() -> bool {
    get_input().mouse.button_left.recently_released()
}

// Mouse button right

#[inline]
pub fn mouse_is_down_right() -> bool {
    get_input().mouse.button_right.is_pressed
}

#[inline]
pub fn mouse_recently_pressed_right() -> bool {
    get_input().mouse.button_right.recently_pressed()
}

#[inline]
pub fn mouse_recently_released_right() -> bool {
    get_input().mouse.button_right.recently_released()
}

// Mouse button middle

#[inline]
pub fn mouse_is_down_middle() -> bool {
    get_input().mouse.button_middle.is_pressed
}

#[inline]
pub fn mouse_recently_pressed_middle() -> bool {
    get_input().mouse.button_middle.recently_pressed()
}

#[inline]
pub fn mouse_recently_released_middle() -> bool {
    get_input().mouse.button_middle.recently_released()
}

// Mouse button x1

#[inline]
pub fn mouse_is_down_x1() -> bool {
    get_input().mouse.button_x1.is_pressed
}

#[inline]
pub fn mouse_recently_pressed_x1() -> bool {
    get_input().mouse.button_x1.recently_pressed()
}

#[inline]
pub fn mouse_recently_released_x1() -> bool {
    get_input().mouse.button_x1.recently_released()
}

// Mouse button x2

#[inline]
pub fn mouse_is_down_x2() -> bool {
    get_input().mouse.button_x2.is_pressed
}

#[inline]
pub fn mouse_recently_pressed_x2() -> bool {
    get_input().mouse.button_x2.recently_pressed()
}

#[inline]
pub fn mouse_recently_released_x2() -> bool {
    get_input().mouse.button_x2.recently_released()
}

// Mouse button any

#[inline]
pub fn mouse_is_down(button: MouseButton) -> bool {
    match button {
        MouseButton::Left => mouse_is_down_left(),
        MouseButton::Right => mouse_is_down_right(),
        MouseButton::Middle => mouse_is_down_middle(),
        MouseButton::X1 => mouse_is_down_x1(),
        MouseButton::X2 => mouse_is_down_x2(),
    }
}

#[inline]
pub fn mouse_recently_pressed(button: MouseButton) -> bool {
    match button {
        MouseButton::Left => mouse_recently_pressed_left(),
        MouseButton::Right => mouse_recently_pressed_right(),
        MouseButton::Middle => mouse_recently_pressed_middle(),
        MouseButton::X1 => mouse_recently_pressed_x1(),
        MouseButton::X2 => mouse_recently_pressed_x2(),
    }
}

#[inline]
pub fn mouse_recently_released(button: MouseButton) -> bool {
    match button {
        MouseButton::Left => mouse_recently_released_left(),
        MouseButton::Right => mouse_recently_released_right(),
        MouseButton::Middle => mouse_recently_released_middle(),
        MouseButton::X1 => mouse_recently_released_x1(),
        MouseButton::X2 => mouse_recently_released_x2(),
    }
}

//--------------------------------------------------------------------------------------------------
// TOUCH INPUT

// Touch events

#[inline]
pub fn touch_press_event_happened() -> bool {
    get_input().touch.has_press_event
}

#[inline]
pub fn touch_release_event_happened() -> bool {
    get_input().touch.has_release_event
}

#[inline]
pub fn touch_move_event_happened() -> bool {
    get_input().touch.has_move_event
}

// Touch position

#[inline]
pub fn touch_pos_screen(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.pos_screen)
}

#[inline]
pub fn touch_pos_canvas(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.pos_canvas)
}

#[inline]
pub fn touch_pos_world(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.pos_world)
}

#[inline]
pub fn touch_delta_screen(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.delta_screen)
}

#[inline]
pub fn touch_delta_canvas(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.delta_canvas)
}

#[inline]
pub fn touch_delta_world(finger: FingerId) -> Option<Vec2> {
    get_globals()
        .cursors
        .fingers
        .get(&finger)
        .map(|cursor_coord| cursor_coord.delta_world)
}

// Touch state

#[inline]
pub fn touch_is_down(finger: FingerId) -> bool {
    get_input().touch.is_down(finger)
}

#[inline]
pub fn touch_recently_pressed(finger: FingerId) -> bool {
    get_input().touch.recently_pressed(finger)
}

#[inline]
pub fn touch_recently_released(finger: FingerId) -> bool {
    get_input().touch.recently_released(finger)
}

//--------------------------------------------------------------------------------------------------
// GAMEPAD

// Gamepad events

#[inline]
pub fn gamepad_press_event_happened() -> bool {
    get_input().gamepad.has_press_event
}

#[inline]
pub fn gamepad_release_event_happened() -> bool {
    get_input().gamepad.has_release_event
}

#[inline]
pub fn gamepad_stick_event_happened() -> bool {
    get_input().gamepad.has_stick_event
}

#[inline]
pub fn gamepad_trigger_event_happened() -> bool {
    get_input().gamepad.has_trigger_event
}

// Gamepad status

#[inline]
pub fn gamepad_is_connected() -> bool {
    get_input().gamepad.is_connected
}

// Gamepad sticks and triggers

#[inline]
pub fn gamepad_stick_left() -> Vec2 {
    get_input().gamepad.stick_left
}

#[inline]
pub fn gamepad_stick_right() -> Vec2 {
    get_input().gamepad.stick_right
}

#[inline]
pub fn gamepad_trigger_left() -> f32 {
    get_input().gamepad.trigger_left
}

#[inline]
pub fn gamepad_trigger_right() -> f32 {
    get_input().gamepad.trigger_right
}

// Gamepad button state

#[inline]
pub fn gamepad_is_down(button: GamepadButton) -> bool {
    get_input().gamepad.is_down(button)
}

#[inline]
pub fn gamepad_recently_pressed(button: GamepadButton) -> bool {
    get_input().gamepad.recently_pressed(button)
}

#[inline]
pub fn gamepad_recently_released(button: GamepadButton) -> bool {
    get_input().gamepad.recently_released(button)
}
