#[cfg(target_arch = "wasm32")]
#[path = "wasm_app.rs"]
pub mod app;

#[cfg(not(target_arch = "wasm32"))]
#[path = "sdl_app.rs"]
pub mod app;
