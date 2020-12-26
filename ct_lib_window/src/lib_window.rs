#[cfg(target_arch = "wasm32")]
#[path = "platform_wasm/wasm_app.rs"]
pub mod platform;

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform_sdl2/sdl_app.rs"]
pub mod platform;

pub mod renderer_opengl;

use ct_lib_audio as audio;
use ct_lib_core as core;
use ct_lib_draw as draw;
use ct_lib_game as game;
use ct_lib_image as image;
use ct_lib_math as math;
use game::GameStateInterface;

pub fn run_main<GameStateType: 'static + GameStateInterface + Clone>() {
    platform::run_main::<GameStateType>();
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
