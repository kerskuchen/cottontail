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

////////////////////////////////////////////////////////////////////////////////////////////////
// Drawing

//----------------------------------------------------------------------------------------------
// Environment

pub fn draw_set_letterbox_color(color: Color) {
    get_draw().set_letterbox_color(color)
}

pub fn draw_set_clear_color_and_depth(color: Color, depth: Depth) {
    get_draw().set_clear_color_and_depth(color, depth)
}

//----------------------------------------------------------------------------------------------
// Quad drawing

#[inline]
pub fn draw_quad(
    quad: &Quad,
    uvs: AAQuad,
    uv_region_contains_translucency: bool,
    texture_index: TextureIndex,
    drawparams: Drawparams,
) {
    get_draw().draw_quad(
        quad,
        uvs,
        uv_region_contains_translucency,
        texture_index,
        drawparams,
    );
}

//----------------------------------------------------------------------------------------------
// Sprite drawing

/// NOTE: Rotation is performed around the sprites pivot point
#[inline]
pub fn draw_sprite(
    sprite: &Sprite,
    xform: Transform,
    flip_horizontally: bool,
    flip_vertically: bool,
    drawparams: Drawparams,
) {
    get_draw().draw_sprite(
        sprite,
        xform,
        flip_horizontally,
        flip_vertically,
        drawparams,
    )
}

#[inline]
pub fn draw_sprite_clipped(
    sprite: &Sprite,
    pos: Vec2,
    scale: Vec2,
    clipping_rect: Rect,
    drawparams: Drawparams,
) {
    get_draw().draw_sprite_clipped(sprite, pos, scale, clipping_rect, drawparams)
}

#[inline]
pub fn draw_sprite_3d(sprite: &Sprite3D, xform: Transform, drawparams: Drawparams) {
    get_draw().draw_sprite_3d(sprite, xform, drawparams)
}

//----------------------------------------------------------------------------------------------
// Primitive drawing

/// This fills the following pixels:
/// [left, right[ x [top, bottom[
#[inline]
pub fn draw_rect(rect: Rect, filled: bool, drawparams: Drawparams) {
    get_draw().draw_rect(rect, filled, drawparams)
}

/// Draws a rotated rectangle where `rotation_dir` = (1,0) corresponds to angle zero.
/// IMPORTANT: `rotation_dir` is assumed to be normalized
/// IMPORTANT: The `pivot` is the rotation pivot and position pivot
/// This fills the following pixels when given `rotation_dir` = (1,0), `rotation_pivot` = (0,0):
/// [left, right[ x [top, bottom[
#[inline]
pub fn draw_rect_transformed(
    rect_dim: Vec2,
    filled: bool,
    centered: bool,
    pivot: Vec2,
    xform: Transform,
    drawparams: Drawparams,
) {
    get_draw().draw_rect_transformed(rect_dim, filled, centered, pivot, xform, drawparams)
}

/// Expects vertices in the form [v_a0, v_a1, v_a2, v_b0, v_b1, v_b2, ...]
#[inline]
pub fn draw_polygon(vertices: &[Vec2], pivot: Vec2, xform: Transform, drawparams: Drawparams) {
    get_draw().draw_polygon(vertices, pivot, xform, drawparams)
}

#[inline]
pub fn draw_circle_filled(center: Vec2, radius: f32, drawparams: Drawparams) {
    get_draw().draw_circle_filled(center, radius, drawparams)
}

#[inline]
pub fn draw_circle_bresenham(center: Vec2, radius: f32, drawparams: Drawparams) {
    get_draw().draw_circle_bresenham(center, radius, drawparams)
}

#[inline]
pub fn draw_ring(center: Vec2, radius: f32, thickness: f32, drawparams: Drawparams) {
    get_draw().draw_ring(center, radius, thickness, drawparams)
}

/// WARNING: This can be slow if used often
#[inline]
pub fn draw_pixel(pos: Vec2, drawparams: Drawparams) {
    get_draw().draw_pixel(pos, drawparams)
}

/// WARNING: This can be slow if used often
/// NOTE: Skipping the last pixel is useful i.e. for drawing translucent line loops which start
///       and end on the same pixel and pixels must not overlap
#[inline]
pub fn draw_linestrip_bresenham(points: &[Vec2], skip_last_pixel: bool, drawparams: Drawparams) {
    get_draw().draw_linestrip_bresenham(points, skip_last_pixel, drawparams)
}

/// WARNING: This can be slow if used often
/// NOTE: Skipping the last pixel is useful i.e. for drawing translucent linestrips where pixels
///       must not overlap
#[inline]
pub fn draw_line_bresenham(start: Vec2, end: Vec2, skip_last_pixel: bool, drawparams: Drawparams) {
    get_draw().draw_line_bresenham(start, end, skip_last_pixel, drawparams)
}

#[inline]
pub fn draw_line_with_thickness(
    start: Vec2,
    end: Vec2,
    thickness: f32,
    smooth_edges: bool,
    drawparams: Drawparams,
) {
    get_draw().draw_line_with_thickness(start, end, thickness, smooth_edges, drawparams)
}

//--------------------------------------------------------------------------------------------------
// Text drawing

/// Draws a given utf8 text with a given font
/// Returns the starting_offset for the next `draw_text`
#[inline]
pub fn draw_text(
    text: &str,
    font: &SpriteFont,
    font_scale: f32,
    starting_origin: Vec2,
    starting_offset: Vec2,
    alignment: Option<TextAlignment>,
    color_background: Option<Color>,
    drawparams: Drawparams,
) -> Vec2 {
    get_draw().draw_text(
        text,
        font,
        font_scale,
        starting_origin,
        starting_offset,
        alignment,
        color_background,
        drawparams,
    )
}

/// Draws a given utf8 text in a given font using a clipping rectangle
/// NOTE: This does not do any word wrapping - the given text should be already pre-wrapped
///       for a good result
#[inline]
pub fn draw_text_clipped(
    text: &str,
    font: &SpriteFont,
    font_scale: f32,
    starting_origin: Vec2,
    starting_offset: Vec2,
    origin_is_baseline: bool,
    clipping_rect: Rect,
    drawparams: Drawparams,
) {
    get_draw().draw_text_clipped(
        text,
        font,
        font_scale,
        starting_origin,
        starting_offset,
        origin_is_baseline,
        clipping_rect,
        drawparams,
    )
}

////////////////////////////////////////////////////////////////////////////////////////////////
// Debug Drawing

#[inline]
pub fn draw_debug_checkerboard(
    origin: Vec2,
    cells_per_side: usize,
    cell_size: f32,
    drawparams: Drawparams,
) {
    get_draw().debug_draw_checkerboard(origin, cells_per_side, cell_size, drawparams)
}

#[inline]
pub fn debug_arrow(start: Vec2, dir: Vec2, drawparams: Drawparams) {
    get_draw().debug_draw_arrow(start, dir, drawparams)
}

#[inline]
pub fn draw_debug_arrow_line(start: Vec2, end: Vec2, drawparams: Drawparams) {
    get_draw().debug_draw_arrow_line(start, end, drawparams)
}

#[inline]
pub fn draw_debug_triangle(point_a: Vec2, point_b: Vec2, point_c: Vec2, drawparams: Drawparams) {
    get_draw().debug_draw_triangle(point_a, point_b, point_c, drawparams)
}

#[inline]
pub fn draw_debug_log(text: impl Into<String>) {
    get_draw().debug_log(text)
}

#[inline]
pub fn draw_debug_log_color(text: impl Into<String>, color: Color) {
    get_draw().debug_log_color(text, color)
}

#[inline]
pub fn draw_debug_log_visualize_value_percent(
    label: impl Into<String>,
    color: Color,
    value_percent: f32,
) {
    get_draw().debug_log_visualize_value_percent(label, color, value_percent)
}

#[inline]
pub fn draw_debug_log_visualize_value(
    label: impl Into<String>,
    color: Color,
    value: f32,
    value_min: f32,
    value_max: f32,
) {
    get_draw().debug_log_visualize_value(label, color, value, value_min, value_max)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Multiresource

#[inline]
pub fn draw_debug_grid(world_grid_size: f32, line_thickness: i32, color: Color, depth: f32) {
    debug_draw_grid(
        get_draw(),
        &get_camera().cam,
        world_grid_size,
        window_screen_width() as f32,
        window_screen_height() as f32,
        line_thickness,
        color,
        depth,
    )
}

pub fn draw_debug_crosshair(pos_world: Vec2, line_thickness: f32, color: Color, depth: f32) {
    debug_draw_crosshair(
        get_draw(),
        &get_camera().cam,
        pos_world,
        window_screen_width() as f32,
        window_screen_height() as f32,
        line_thickness,
        color,
        depth,
    )
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// GUI

#[inline]
pub fn gui_begin_frame() {
    get_gui().begin_frame()
}

#[inline]
pub fn gui_end_frame() {
    get_gui().end_frame()
}

#[inline]
#[must_use = "It returns whether the button was pressed or clicked or not"]
pub fn gui_button(
    id: GuiElemId,
    button_rect: Rect,
    label: &str,
    label_font: &SpriteFont,
    color_label: Color,
    color_background: Color,
    drawparams: Drawparams,
) -> (bool, bool) {
    get_gui().button(
        id,
        button_rect,
        label,
        label_font,
        color_label,
        color_background,
        drawparams,
    )
}

#[inline]
#[must_use = "It returns a new percentage value if the slider was mutated"]
pub fn gui_horizontal_slider(
    id: GuiElemId,
    slider_rect: Rect,
    cur_value: f32,
    depth: f32,
) -> Option<f32> {
    get_gui().horizontal_slider(id, slider_rect, cur_value, depth)
}

pub fn gui_text_scroller(
    id: GuiElemId,
    dt: f32,
    rect: Rect,
    font: &SpriteFont,
    font_scale: f32,
    text_color: Color,
    text: &str,
    linecount: usize,
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    inout_acc: &mut f32,
    depth: f32,
) {
    get_gui().text_scroller(
        id, dt, rect, font, font_scale, text_color, text, linecount, inout_pos, inout_vel,
        inout_acc, depth,
    )
}
