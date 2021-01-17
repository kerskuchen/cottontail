#[allow(dead_code)]
pub const LAUNCHER_WINDOW_TITLE: &str = "{{project_display_name}}";
#[allow(dead_code)]
pub const LAUNCHER_SAVE_FOLDER_NAME: &str = "{{windows_appdata_dir}}";
#[allow(dead_code)]
pub const LAUNCHER_COMPANY_NAME: &str = "{{project_company_name}}";

mod game;
use game::GameState;

impl GameState {
    /// Helper function for when we need a additional reference of ourselves
    /// IMPORTANT: This can be highly unsafe! So use sparingly!
    #[allow(dead_code)]
    fn get_additional_self(&self) -> &'static GameState {
        unsafe { std::mem::transmute::<&GameState, &'static GameState>(self) }
    }
    /// Helper function for when we need a additional mutable reference of ourselves
    /// IMPORTANT: This can be highly unsafe! So use sparingly!
    #[allow(dead_code)]
    fn get_additional_self_mut(&mut self) -> &'static mut GameState {
        unsafe { std::mem::transmute::<&mut GameState, &'static mut GameState>(self) }
    }
}

fn main() {
    cottontail::game::start_mainloop::<GameState>();
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_main() {
    main();
}
