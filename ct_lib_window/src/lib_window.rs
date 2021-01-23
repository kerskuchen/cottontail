#[cfg(target_arch = "wasm32")]
#[path = "platform_wasm/wasm_app.rs"]
mod platform;

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform_sdl2/sdl_app.rs"]
mod platform;

pub mod input;
pub mod renderer_opengl;

use input::*;
pub use platform::add_platform_window_command;
pub use platform::audio::AudioOutput;
pub use renderer_opengl::Renderer;

pub struct AppInfo {
    pub window_title: String,
    pub save_folder_name: String,
    pub company_name: String,
}
pub trait AppEventHandler {
    fn reset(&mut self);

    fn handle_window_resize(&mut self, new_width: u32, new_height: u32, is_fullscreen: bool);
    fn handle_window_focus_gained(&mut self);
    fn handle_window_focus_lost(&mut self);

    fn handle_key_press(&mut self, scancode: Scancode, keycode: Keycode, is_repeat: bool);
    fn handle_key_release(&mut self, scancode: Scancode, keycode: Keycode);

    fn handle_mouse_press(&mut self, button: MouseButton, pos_x: i32, pos_y: i32);
    fn handle_mouse_release(&mut self, button: MouseButton, pos_x: i32, pos_y: i32);
    fn handle_mouse_move(&mut self, pos_x: i32, pos_y: i32);
    fn handle_mouse_wheel_scroll(&mut self, scroll_delta: i32);

    fn handle_touch_press(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32);
    fn handle_touch_release(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32);
    fn handle_touch_move(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32);
    fn handle_touch_cancelled(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32);

    fn handle_gamepad_connected(&mut self, gamepad_id: GamepadPlatformId);
    fn handle_gamepad_disconnected(&mut self, gamepad_id: GamepadPlatformId);
    fn handle_gamepad_new_state(
        &mut self,
        gamepad_id: GamepadPlatformId,
        state: &GamepadPlatformState,
    );

    fn run_tick(
        &mut self,
        frametime: f32,
        real_world_uptime: f64,
        renderer: &mut Renderer,
        audio: &mut AudioOutput,
    );
}

pub fn run_main<AppEventHandlerType: AppEventHandler + 'static>(
    app_context: AppEventHandlerType,
    app_info: AppInfo,
) {
    platform::run_main(app_context, app_info).ok();
}

pub enum PlatformWindowCommand {
    FullscreenToggle,
    TextinputStart {
        inputrect_x: i32,
        inputrect_y: i32,
        inputrect_width: u32,
        inputrect_height: u32,
    },
    TextinputStop,
    ScreenSetGrabInput(bool),
    WindowedModeAllowResizing(bool),
    WindowedModeAllow(bool),
    WindowedModeSetSize {
        width: u32,
        height: u32,
        minimum_width: u32,
        minimum_height: u32,
    },
    Shutdown,
    Restart,
}
