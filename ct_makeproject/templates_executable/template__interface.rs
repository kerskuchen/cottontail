mod game;
use game::Gamestate;

use ct_lib::audio::*;
use ct_lib::draw::*;
use ct_lib::game::*;
use ct_lib::math::*;

pub const GAME_WINDOW_TITLE: &str = "{{project_display_name}}";
pub const GAME_SAVE_FOLDER_NAME: &str = "{{windows_appdata_dir}}";
pub const GAME_COMPANY_NAME: &str = "{{project_company_name}}";

const SPLASHSCREEN_FADEIN_TIME: f32 = 0.5;
const SPLASHSCREEN_SUSTAIN_TIME: f32 = 0.5;
const SPLASHSCREEN_FADEOUT_TIME: f32 = 0.5;

#[derive(Default, Clone)]
pub struct GameMemory {
    game: Option<game::Gamestate>,
    pub draw: Option<Drawstate>,
    pub audio: Option<Audiostate>,
    assets: Option<GameAssets>,

    splashscreen: Option<SplashScreen>,
}

#[no_mangle]
pub fn game_update_and_draw(
    memory: &mut GameMemory,
    input: &GameInput,
    current_audio_frame_index: AudioFrameIndex,
    out_systemcommands: &mut Vec<SystemCommand>,
) {
    if memory.draw.is_none() {
        let atlas = game_load_atlas("assets_baked");
        let mut draw = Drawstate::new(atlas, "ProggyTiny_bordered");
        game_setup_window(
            &mut draw,
            &game::WINDOW_CONFIG,
            input.screen_framebuffer_width,
            input.screen_framebuffer_height,
            out_systemcommands,
        );
        draw.set_shaderparams_simple(
            Color::white(),
            Mat4::ortho_origin_left_top(
                game::WINDOW_CONFIG.canvas_width as f32,
                game::WINDOW_CONFIG.canvas_height as f32,
                DEFAULT_WORLD_ZNEAR,
                DEFAULT_WORLD_ZFAR,
            ),
        );
        memory.draw = Some(draw);
    }
    if memory.assets.is_none() {
        let animations = game_load_animations("assets_baked");
        memory.assets = Some(GameAssets::new(animations));
    }
    if memory.audio.is_none() {
        memory.audio = Some(Audiostate::new());
    }

    let draw = memory.draw.as_mut().unwrap();
    let audio = memory.audio.as_mut().unwrap();
    let assets = memory.assets.as_mut().unwrap();

    audio.update_frame_index(current_audio_frame_index);
    draw.begin_frame();

    if memory.splashscreen.is_none() {
        let splash_sprite = draw.get_sprite_by_name("splash").clone();
        memory.splashscreen = Some(SplashScreen::new(
            splash_sprite,
            SPLASHSCREEN_FADEIN_TIME,
            SPLASHSCREEN_FADEOUT_TIME,
            SPLASHSCREEN_SUSTAIN_TIME,
        ));
    }

    let splashscreen = memory
        .splashscreen
        .as_mut()
        .expect("No Splashscreen initialized");

    if input.keyboard.recently_pressed(Scancode::Escape) {
        splashscreen.force_fast_forward();
    }
    let (canvas_width, canvas_height) = draw.get_canvas_dimensions().unwrap_or((
        input.screen_framebuffer_width,
        input.screen_framebuffer_height,
    ));
    match splashscreen.update_and_draw(draw, input.target_deltatime, canvas_width, canvas_height) {
        SplashscreenState::StartedFadingIn => {}
        SplashscreenState::IsFadingIn => {}
        SplashscreenState::FinishedFadingIn => {
            let audiorecordings_mono = ct_lib::game::game_load_audiorecordings_mono("assets_baked");
            for (recording_name, buffer) in audiorecordings_mono.into_iter() {
                audio.add_recording_mono(&recording_name, buffer);
            }

            assert!(memory.game.is_none());
            memory.game = Some(Gamestate::new(draw, audio, assets, &input));
        }
        SplashscreenState::Sustain => {}
        SplashscreenState::StartedFadingOut => {}
        SplashscreenState::IsFadingOut => {}
        SplashscreenState::FinishedFadingOut => {}
        SplashscreenState::IsDone => {}
    }

    if let Some(game) = memory.game.as_mut() {
        game::update_and_draw(game, draw, audio, assets, input);
        game_handle_system_keys(&input.keyboard, out_systemcommands);
    }

    draw.finish_frame(
        input.screen_framebuffer_width,
        input.screen_framebuffer_height,
    );
}
