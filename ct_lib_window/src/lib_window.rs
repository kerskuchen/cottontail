#[cfg(target_arch = "wasm32")]
#[path = "platform_wasm/wasm_app.rs"]
pub mod platform;

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform_sdl2/sdl_app.rs"]
pub mod platform;

pub mod input;
pub mod renderer_opengl;

use input::InputState;
use platform::audio::AudioOutput;
use renderer_opengl::Renderer;

pub struct AppInfo {
    pub window_title: String,
    pub save_folder_name: String,
    pub company_name: String,
}
pub enum AppCommand {
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
pub trait AppContextInterface: Clone {
    fn get_app_info() -> AppInfo;
    fn new(renderer: &mut Renderer, input: &InputState, audio: &mut AudioOutput) -> Self;
    fn reset(&mut self);
    fn run_tick(
        &mut self,
        renderer: &mut Renderer,
        input: &InputState,
        audio: &mut AudioOutput,
        out_systemcommands: &mut Vec<AppCommand>,
    );
}

pub fn run_main<AppContextType: 'static + AppContextInterface>() {
    platform::run_main::<AppContextType>();
}

fn snap_deltatime_to_nearest_common_refresh_rate(deltatime: f32) -> f32 {
    let common_refresh_rates = [30, 60, 72, 75, 85, 90, 120, 144, 240, 360];
    let index_with_smallest_distance = common_refresh_rates
        .iter()
        .map(|refresh_rate| (deltatime - 1.0 / *refresh_rate as f32).abs())
        .enumerate()
        .min_by(|(_index_a, a), (_index_b, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0;
    1.0 / common_refresh_rates[index_with_smallest_distance] as f32
}
