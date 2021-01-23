pub const LAUNCHER_WINDOW_TITLE: &str = "{{project_display_name}}";
pub const LAUNCHER_SAVE_FOLDER_NAME: &str = "{{windows_appdata_dir}}";
pub const LAUNCHER_COMPANY_NAME: &str = "{{project_company_name}}";

mod game;

fn main() {
    cottontail::game::start_mainloop::<game::GameState>(cottontail::window::AppInfo {
        window_title: LAUNCHER_WINDOW_TITLE.to_owned(),
        save_folder_name: LAUNCHER_SAVE_FOLDER_NAME.to_owned(),
        company_name: LAUNCHER_COMPANY_NAME.to_owned(),
    });
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_main() {
    main();
}
