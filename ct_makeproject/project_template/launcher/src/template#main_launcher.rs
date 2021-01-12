#[allow(dead_code)]
pub const LAUNCHER_WINDOW_TITLE: &str = "{{project_display_name}}";
#[allow(dead_code)]
pub const LAUNCHER_SAVE_FOLDER_NAME: &str = "{{windows_appdata_dir}}";
#[allow(dead_code)]
pub const LAUNCHER_COMPANY_NAME: &str = "{{project_company_name}}";

mod game;

fn main() {
    cottontail::game::start_mainloop::<game::GameState>();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_main() {
    main();
}
