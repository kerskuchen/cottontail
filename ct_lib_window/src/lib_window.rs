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
