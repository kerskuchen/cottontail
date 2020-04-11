use ct_lib::audio::*;
use ct_lib::draw::*;
use ct_lib::game::*;
use ct_lib::math::*;
use ct_lib::random::*;

const CANVAS_WIDTH: f32 = 480.0;
const CANVAS_HEIGHT: f32 = 270.0;

pub const WINDOW_CONFIG: WindowConfig = WindowConfig {
    has_canvas: true,
    canvas_width: CANVAS_WIDTH as u32,
    canvas_height: CANVAS_HEIGHT as u32,
    canvas_color_letterbox: Color::black(),

    windowed_mode_allow: true,
    windowed_mode_allow_resizing: true,

    grab_input: false,

    color_clear: Color::black(),
};

#[derive(Clone)]
pub struct Gamestate {
    globals: Globals,

    debug_deltatime_factor: f32,
    scene_debug: SceneDebug,
}

impl Gamestate {
    pub fn new(
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &mut GameAssets,
        input: &GameInput,
    ) -> Gamestate {
        let random = Random::new_from_seed((input.deltatime * 1000000.0) as u64);

        let camera = GameCamera::new(Vec2::zero(), CANVAS_WIDTH, CANVAS_HEIGHT);

        let cursors = Cursors::new(
            &camera.cam,
            &input.mouse,
            &input.touch,
            input.screen_framebuffer_width,
            input.screen_framebuffer_height,
            CANVAS_WIDTH as u32,
            CANVAS_HEIGHT as u32,
        );

        let font_default = draw.get_font("ProggyTiny_bordered");
        let font_default_no_border = draw.get_font("ProggyTiny");

        let mut globals = Globals {
            random,
            camera,
            cursors,

            deltatime_speed_factor: 1.0,
            deltatime: input.deltatime,
            is_paused: false,

            canvas_width: CANVAS_WIDTH,
            canvas_height: CANVAS_HEIGHT,

            font_default,
            font_default_no_border,
        };

        let scene_debug = SceneDebug::new(draw, audio, assets, input);

        Gamestate {
            globals,

            debug_deltatime_factor: 1.0,
            scene_debug,
        }
    }
}

pub fn update_and_draw(
    game: &mut Gamestate,
    draw: &mut Drawstate,
    audio: &mut Audiostate,
    assets: &mut GameAssets,
    input: &GameInput,
) {
    if input.keyboard.recently_pressed(Scancode::F5) {
        *game = Gamestate::new(draw, audio, assets, input);
    }

    game.globals.cursors = Cursors::new(
        &game.globals.camera.cam,
        &input.mouse,
        &input.touch,
        input.screen_framebuffer_width,
        input.screen_framebuffer_height,
        CANVAS_WIDTH as u32,
        CANVAS_HEIGHT as u32,
    );

    // DEBUG GAMESPEED MANIPULATION
    //
    if !is_effectively_zero(game.debug_deltatime_factor - 1.0) {
        draw.debug_log(format!("timefactor: {:.1}", game.debug_deltatime_factor));
    }
    if input.keyboard.recently_pressed(Scancode::KpPlus) {
        game.debug_deltatime_factor += 0.1;
    }
    if input.keyboard.recently_pressed(Scancode::KpMinus) {
        game.debug_deltatime_factor -= 0.1;
        if game.debug_deltatime_factor < 0.1 {
            game.debug_deltatime_factor = 0.1;
        }
    }
    if input.keyboard.recently_pressed(Scancode::Space) {
        game.globals.is_paused = !game.globals.is_paused;
    }
    let mut deltatime = input.target_deltatime * game.debug_deltatime_factor;
    if game.globals.is_paused {
        if input.keyboard.recently_pressed_or_repeated(Scancode::N) {
            deltatime = input.target_deltatime * game.debug_deltatime_factor;
        } else {
            deltatime = 0.0;
        }
    }
    game.globals.deltatime = deltatime * game.globals.deltatime_speed_factor;

    let mouse_coords = game.globals.cursors.mouse_coords;
    game_handle_mouse_camera_zooming_panning(&mut game.globals.camera, &input.mouse, &mouse_coords);

    game.scene_debug
        .update_and_draw(draw, audio, assets, input, &mut game.globals);

    let deltatime = game.globals.deltatime;
    game.globals.camera.update(deltatime);
    draw.set_shaderparams_simple(Color::white(), game.globals.camera.proj_view_matrix());
}
