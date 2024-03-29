use std::collections::VecDeque;

use cottontail::audio::*;
use cottontail::core::dformat;
use cottontail::draw::{draw::*, PixelSnapped, Sprite, Sprite3D};
use cottontail::game::*;
use cottontail::image::*;
use cottontail::math::*;
use cottontail::window::{
    input::{InputState, Scancode},
    AppCommand,
};

const CANVAS_WIDTH: f32 = 480.0;
const CANVAS_HEIGHT: f32 = 270.0;

const DEPTH_DRAW: Depth = 20.0;
const DEPTH_GLITTER: Depth = 20.0;

const INTERVAL_MEASURE: MusicalInterval = MusicalInterval::Measure {
    beats_per_minute: 140,
    beats_per_measure: 4,
};
const INTERVAL_HALFBEAT: MusicalInterval = MusicalInterval::HalfBeat {
    beats_per_minute: 140,
};
const INTERVAL_QUARTERBEAT: MusicalInterval = MusicalInterval::QuarterBeat {
    beats_per_minute: 140,
};

#[derive(Debug, Copy, Clone)]
enum SelectedScene {
    Choreographer = 1,
    Sprites,
    Sprites3DSpatialSound,
    Credits,
}

impl SelectedScene {
    fn next(self) -> SelectedScene {
        match self {
            SelectedScene::Choreographer => SelectedScene::Sprites,
            SelectedScene::Sprites => SelectedScene::Sprites3DSpatialSound,
            SelectedScene::Sprites3DSpatialSound => SelectedScene::Credits,
            SelectedScene::Credits => SelectedScene::Choreographer,
        }
    }
}

#[derive(Clone)]
pub struct GameState {
    glitter: ParticleSystem,

    selected_scene: SelectedScene,
    scene_choreographer: SceneChoreographer,
    scene_sprites: SceneSprites,
    scene_sprites3d_spatial: SceneSprites3dSpatialSound,
    scene_credits: SceneCredits,
}

impl GameStateInterface for GameState {
    fn get_game_info() -> GameInfo {
        GameInfo {
            game_window_title: crate::LAUNCHER_WINDOW_TITLE.to_owned(),
            game_save_folder_name: crate::LAUNCHER_SAVE_FOLDER_NAME.to_owned(),
            game_company_name: crate::LAUNCHER_COMPANY_NAME.to_owned(),
        }
    }

    fn get_window_config() -> WindowConfig {
        WindowConfig {
            has_canvas: true,
            canvas_width: CANVAS_WIDTH as u32,
            canvas_height: CANVAS_HEIGHT as u32,
            canvas_color_letterbox: Color::black(),

            windowed_mode_allow: true,
            windowed_mode_allow_resizing: true,

            grab_input: false,

            color_clear: Color::black(),
            color_splash_progressbar: Color::from_hex_rgba(0x6bbbedff),
        }
    }

    fn new(
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
    ) -> GameState {
        // let credits = String::from_utf8_lossy(assets.get_content_filedata("credits.txt"));
        // log::info!("Credits: {}", credits);

        let glitter_params = ParticleSystemParams {
            gravity: Vec2::new(0.0, -15.0),
            vel_start: Vec2::new(1.0, 0.0),
            vel_max: 10000.0,
            scale_start: 1.0,
            scale_end: 1.0,
            spawn_radius: 15.0,
            lifetime: 1.0,
            additivity_start: ADDITIVITY_MAX,
            additivity_end: ADDITIVITY_NONE,
            color_start: Color::white(),
            color_end: 0.0 * Color::white(),
        };

        audio.group_mute(SelectedScene::Sprites as u32);
        audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
        audio.group_mute(SelectedScene::Credits as u32);

        GameState {
            glitter: ParticleSystem::new(glitter_params, 30, Vec2::zero()),
            selected_scene: SelectedScene::Choreographer,
            scene_choreographer: SceneChoreographer::new(),
            scene_sprites: SceneSprites::new(draw, audio, assets, input),
            scene_sprites3d_spatial: SceneSprites3dSpatialSound::new(
                draw, audio, assets, input, globals,
            ),
            scene_credits: SceneCredits::new(draw, audio, assets, input, globals),
        }
    }

    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        out_systemcommands: &mut Vec<AppCommand>,
    ) {
        if input.keyboard.recently_pressed(Scancode::F5) {
            audio.reset();
            *self = GameState::new(draw, audio, assets, input, globals);
        }

        // audio.set_global_volume(0.0);

        // CURSOR VISUALIZATION
        {
            draw.draw_pixel(
                globals.cursors.mouse.pos_world,
                Drawparams::without_additivity(DEPTH_DEBUG, Color::magenta(), Drawspace::World),
            );
            if let Some(pos) = globals
                .cursors
                .finger_primary
                .map(|coords| coords.pos_canvas)
            {
                draw.draw_circle_filled(
                    pos,
                    20.0,
                    Drawparams::without_additivity(DEPTH_DEBUG, Color::red(), Drawspace::Canvas),
                )
            }
            if let Some(pos) = globals
                .cursors
                .finger_secondary
                .map(|coords| coords.pos_canvas)
            {
                draw.draw_circle_filled(
                    pos,
                    20.0,
                    Drawparams::without_additivity(DEPTH_DEBUG, Color::yellow(), Drawspace::Canvas),
                )
            }
            self.glitter.move_to(globals.cursors.mouse.pos_canvas);
            self.glitter.update_and_draw(
                draw,
                &mut globals.random,
                globals.deltatime,
                DEPTH_GLITTER,
                Drawspace::World,
            );

            draw.debug_log(format!(
                "screen: {}x{}",
                input.screen_framebuffer_width, input.screen_framebuffer_height,
            ));
            draw.debug_log(format!(
                "canvas: {}x{}",
                globals.canvas_width, globals.canvas_height
            ));
            draw.debug_log(format!(
                "mworld: {}x{}",
                globals.cursors.mouse.pos_world.x, globals.cursors.mouse.pos_world.y,
            ));
            draw.debug_log(format!(
                "mscreen: {}x{}",
                globals.cursors.mouse.pos_screen.x, globals.cursors.mouse.pos_screen.y,
            ));
            draw.debug_log(format!(
                "mcanvas: {}x{}",
                globals.cursors.mouse.pos_canvas.x, globals.cursors.mouse.pos_canvas.y,
            ));
            draw.debug_log(format!(
                "fpworld: {:?}",
                globals
                    .cursors
                    .finger_primary
                    .map(|coords| coords.pos_world)
            ));
            draw.debug_log(format!(
                "fpscreen: {:?}",
                globals
                    .cursors
                    .finger_primary
                    .map(|coords| coords.pos_screen)
            ));
            draw.debug_log(format!(
                "fpcanvas: {:?}",
                globals
                    .cursors
                    .finger_primary
                    .map(|coords| coords.pos_canvas)
            ));
            draw.debug_log(format!(
                "fsworld: {:?}",
                globals
                    .cursors
                    .finger_secondary
                    .map(|coords| coords.pos_world)
            ));
            draw.debug_log(format!(
                "fsscreen: {:?}",
                globals
                    .cursors
                    .finger_secondary
                    .map(|coords| coords.pos_screen)
            ));
            draw.debug_log(format!(
                "fscanvas: {:?}",
                globals
                    .cursors
                    .finger_secondary
                    .map(|coords| coords.pos_canvas)
            ));
            draw.debug_log(format!("mousedown: {}", input.mouse.button_left.is_pressed));
        }

        // FULLSCREEN BUTTON
        {
            let button_fullscreen_text = "fullscreen";
            let button_fullscreen_color = if input.screen_is_fullscreen {
                Color::red()
            } else {
                Color::green()
            };
            let button_fullscreen_rect =
                Rect::from_xy_width_height(globals.canvas_width as f32 - 70.0, 0.0, 70.0, 30.0);
            let (pressed, _clicked) = gui_button(
                draw,
                GuiElemId::new("toggle_fullscreen"),
                button_fullscreen_rect,
                button_fullscreen_text,
                assets.get_font("Grand9K_Pixel_bordered"),
                Color::white(),
                button_fullscreen_color,
                Drawparams::with_depth_drawspace(DEPTH_MAX, Drawspace::Canvas),
            );
            // IMPORTANT: We care for the 'pressed' event and not 'clicked'. Because on WASM we need
            //            to send our AppCommand before we release our mouse/finger so that WASM can
            //            toggle fullscreen on the release event.
            // NOTE: Querying wheter we have a press event prevents infinite toggling on every frame
            //       when holding down mousebutton/finger
            if pressed && (input.mouse.has_press_event || input.touch.has_press_event) {
                out_systemcommands.push(AppCommand::FullscreenToggle);
            }
        }

        // SWITCH SCENE
        {
            let (button_scene_text, button_scene_color) = match self.selected_scene {
                SelectedScene::Choreographer => ("scene 1", Color::cyan()),
                SelectedScene::Sprites => ("scene 2", Color::magenta()),
                SelectedScene::Sprites3DSpatialSound => ("scene 3", Color::yellow()),
                SelectedScene::Credits => ("scene 4", Color::white()),
            };
            let button_scene_rect =
                Rect::from_xy_width_height(globals.canvas_width as f32 - 140.0, 0.0, 70.0, 30.0);
            let (_pressed, clicked) = gui_button(
                draw,
                GuiElemId::new("switch_scene"),
                button_scene_rect,
                button_scene_text,
                assets.get_font("Grand9K_Pixel_bordered"),
                Color::white(),
                button_scene_color,
                Drawparams::with_depth_drawspace(DEPTH_MAX, Drawspace::Canvas),
            );
            if clicked {
                self.selected_scene = self.selected_scene.next();
                audio.group_mute(SelectedScene::Choreographer as u32);
                audio.group_mute(SelectedScene::Sprites as u32);
                audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
                audio.group_mute(SelectedScene::Credits as u32);
                audio.group_unmute(self.selected_scene as u32);
            }

            if input.keyboard.recently_pressed(Scancode::Digit1) {
                self.selected_scene = SelectedScene::Choreographer;
                audio.group_mute(SelectedScene::Choreographer as u32);
                audio.group_mute(SelectedScene::Sprites as u32);
                audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
                audio.group_mute(SelectedScene::Credits as u32);
                audio.group_unmute(self.selected_scene as u32);
            }
            if input.keyboard.recently_pressed(Scancode::Digit2) {
                self.selected_scene = SelectedScene::Sprites;
                audio.group_mute(SelectedScene::Choreographer as u32);
                audio.group_mute(SelectedScene::Sprites as u32);
                audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
                audio.group_mute(SelectedScene::Credits as u32);
                audio.group_unmute(self.selected_scene as u32);
            }
            if input.keyboard.recently_pressed(Scancode::Digit3) {
                self.selected_scene = SelectedScene::Sprites3DSpatialSound;
                audio.group_mute(SelectedScene::Choreographer as u32);
                audio.group_mute(SelectedScene::Sprites as u32);
                audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
                audio.group_mute(SelectedScene::Credits as u32);
                audio.group_unmute(self.selected_scene as u32);
            }
            if input.keyboard.recently_pressed(Scancode::Digit4) {
                self.selected_scene = SelectedScene::Credits;
                audio.group_mute(SelectedScene::Choreographer as u32);
                audio.group_mute(SelectedScene::Sprites as u32);
                audio.group_mute(SelectedScene::Sprites3DSpatialSound as u32);
                audio.group_mute(SelectedScene::Credits as u32);
                audio.group_unmute(self.selected_scene as u32);
            }
        }

        match self.selected_scene {
            SelectedScene::Choreographer => self.scene_choreographer.update(
                draw,
                audio,
                assets,
                input,
                globals,
                out_systemcommands,
            ),
            SelectedScene::Sprites => {
                self.scene_sprites
                    .update(draw, audio, assets, input, globals, out_systemcommands)
            }
            SelectedScene::Sprites3DSpatialSound => self.scene_sprites3d_spatial.update(
                draw,
                audio,
                assets,
                input,
                globals,
                out_systemcommands,
            ),
            SelectedScene::Credits => {
                self.scene_credits
                    .update(draw, audio, assets, input, globals, out_systemcommands)
            }
        };
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CHOREOGRAPHER
#[derive(Clone)]
struct SceneChoreographer {
    choreographer_randoms: Choreographer,
    choreographer_tween: Choreographer,
    choreographer_conversation: Choreographer,
    choreographer_rectangles: Choreographer,
    choreographer_hp_front: Choreographer,
    choreographer_hp_back: Choreographer,
    choreographer_hp_refill: Choreographer,

    hp: f32,
    hp_previous: f32,

    circle_radius: f32,

    music_stream_id: AudioStreamId,
    current_measure: usize,
    last_measure_completion_ratio: f32,
    drumtimes: VecDeque<f64>,
}

impl SceneChoreographer {
    fn new() -> SceneChoreographer {
        SceneChoreographer {
            choreographer_randoms: Choreographer::new(),
            choreographer_tween: Choreographer::new(),
            choreographer_conversation: Choreographer::new(),
            choreographer_rectangles: Choreographer::new(),
            choreographer_hp_front: Choreographer::new(),
            choreographer_hp_back: Choreographer::new(),
            choreographer_hp_refill: Choreographer::new(),

            hp: 1.0,
            hp_previous: 1.0,
            circle_radius: 50.0,

            music_stream_id: 0,
            current_measure: 0,
            last_measure_completion_ratio: 0.0,
            drumtimes: VecDeque::new(),
        }
    }

    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        _out_systemcommands: &mut Vec<AppCommand>,
    ) {
        const DEPTH_DRAW: Depth = 20.0;

        // MUSIC VISUALIZATION
        {
            // Start metronome
            if self.music_stream_id == 0 {
                self.music_stream_id = audio.play(
                    "loop_bell",
                    SelectedScene::Choreographer as AudioGroupId,
                    music_get_next_point_in_time(audio.current_time_seconds(), INTERVAL_MEASURE),
                    true,
                    0.1,
                    1.0,
                    0.0,
                    None,
                );
            }

            if input.keyboard.recently_pressed(Scancode::M) {
                audio.stream_mute(self.music_stream_id);
            }
            if input.keyboard.recently_pressed(Scancode::U) {
                audio.stream_unmute(self.music_stream_id);
            }

            static mut SPEED: f32 = 1.0;
            unsafe {
                if input.keyboard.is_down(Scancode::PageDown) {
                    SPEED -= 0.01;
                }
                if input.keyboard.is_down(Scancode::PageUp) {
                    SPEED += 0.01;
                }
                if SPEED <= 0.1 {
                    SPEED = 0.1;
                }
                draw.debug_log(dformat!(SPEED));
                audio.stream_set_playback_speed(self.music_stream_id, SPEED);
            }
            static mut PAN: f32 = 0.0;
            unsafe {
                if input.keyboard.is_down(Scancode::ArrowLeft) {
                    PAN -= 0.01;
                }
                if input.keyboard.is_down(Scancode::ArrowRight) {
                    PAN += 0.01;
                }
                if PAN <= -1.0 {
                    PAN = -1.0;
                }
                if PAN >= 1.0 {
                    PAN = 1.0;
                }
                draw.debug_log(dformat!(PAN));
                audio.stream_set_pan(self.music_stream_id, PAN);
            }
            static mut VOLUME: f32 = 0.1;
            unsafe {
                if input.keyboard.is_down(Scancode::ArrowDown) {
                    VOLUME -= 0.01;
                }
                if input.keyboard.is_down(Scancode::ArrowUp) {
                    VOLUME += 0.01;
                }
                if VOLUME <= 0.0 {
                    VOLUME = 0.0;
                }
                if VOLUME >= 1.0 {
                    VOLUME = 1.0;
                }
                draw.debug_log(dformat!(VOLUME));
                audio.stream_set_volume(self.music_stream_id, VOLUME);
            }

            // Play drums and samples on a timeline
            let audiotime = audio.current_time_seconds_smoothed();
            let measure_length = INTERVAL_MEASURE.length_seconds();
            let halfbeat_length = INTERVAL_HALFBEAT.length_seconds();
            let measure_completion_ratio = ((audiotime % measure_length) / measure_length) as f32;
            let beat_completion_ratio = (4.0 * measure_completion_ratio) % 1.0;
            if self.current_measure < (audio.current_time_seconds() / measure_length) as usize {
                self.current_measure += 1;
                let halfbeats_per_measure = (measure_length / halfbeat_length).round() as usize;
                for index in 0..halfbeats_per_measure {
                    let drumtime = (self.current_measure + 1) as f64 * measure_length
                        + index as f64 * halfbeat_length;
                    audio.play_oneshot(
                        "drum",
                        SelectedScene::Choreographer as AudioGroupId,
                        drumtime,
                        0.3,
                        1.0,
                        0.0,
                        None,
                    );
                    self.drumtimes.push_back(drumtime);
                }
            }
            draw.debug_log(dformat!(self.current_measure));
            let measure_size_pixels = globals.canvas_width / 2.0;
            let beat_size_pixels = measure_size_pixels / 2.0;
            for index in 0..8 {
                let pos_x = index as f32 * beat_size_pixels;
                draw.draw_rect(
                    Rect::from_xy_width_height(pos_x, globals.canvas_height - 20.0, 2.0, 10.0),
                    true,
                    Drawparams::without_additivity(
                        DEPTH_DEBUG,
                        Color::greyscale(0.8),
                        Drawspace::Canvas,
                    ),
                )
            }
            for index in 0..2 {
                let pos_x = index as f32 * measure_size_pixels;
                draw.draw_rect(
                    Rect::from_xy_width_height(pos_x, globals.canvas_height - 20.0, 2.0, 10.0),
                    true,
                    Drawparams::without_additivity(
                        DEPTH_DEBUG,
                        Color::greyscale(0.2),
                        Drawspace::Canvas,
                    ),
                )
            }
            for time in &self.drumtimes {
                let pos_x = (time - audio.current_time_seconds()) / measure_length
                    * measure_size_pixels as f64;
                draw.draw_rect(
                    Rect::from_xy_width_height(
                        pos_x as f32,
                        globals.canvas_height - 20.0,
                        2.0,
                        10.0,
                    ),
                    true,
                    Drawparams::new(DEPTH_DEBUG, Color::red() * 0.5, 0.5, Drawspace::Canvas),
                )
            }
            self.drumtimes
                .retain(|&time| time >= audio.current_time_seconds());

            // Visualize current measure and beat
            draw.debug_log_visualize_value_percent(
                "beat   ",
                Color::magenta(),
                beat_completion_ratio,
            );
            draw.debug_log_visualize_value_percent(
                "measure",
                Color::blue(),
                measure_completion_ratio,
            );
            draw.draw_rect(
                Rect::from_xy_width_height(
                    0.0,
                    globals.canvas_height - 10.0,
                    measure_completion_ratio * globals.canvas_width,
                    10.0,
                ),
                true,
                Drawparams::without_additivity(DEPTH_DEBUG, Color::blue(), Drawspace::Canvas),
            );
        }
        // Background
        draw.draw_rect(
            Rect::from_width_height(globals.canvas_width, globals.canvas_height),
            true,
            Drawparams::without_additivity(DEPTH_DRAW, Color::greyscale(0.5), Drawspace::World),
        );

        let center = Vec2::new(globals.canvas_width, globals.canvas_height) / 2.0;

        // CONVERSATION
        //
        self.choreographer_conversation.update(globals.deltatime);
        (|| {
            // Based on https://github.com/RandyGaul/cute_headers/blob/master/cute_coroutine.h
            let colors = [Color::green(), Color::yellow()];
            let names = ["Bob", "Alice"];
            let messages = [
                "Yo Alice. I heard you like mudkips.",
                "No Bob. Not me. Who told you such a thing?",
                "Alice please, don't lie to me. We've known each other a long time.",
                "We have grown apart. I barely know myself.",
                "OK.",
                "Good bye Bob. I wish you the best.",
                "But do you like mudkips?",
                "<has left>",
                "Well, I like mudkips :)",
            ];

            for ((message, name), color) in messages
                .iter()
                .zip(names.iter().cycle())
                .zip(colors.iter().cycle())
            {
                if !self.choreographer_conversation.wait(1.0) {
                    return;
                }

                let (line, finished) = collect_line(
                    &mut self.choreographer_conversation,
                    &mut globals.random,
                    name,
                    message,
                );
                draw.debug_log_color(line, *color);

                if !finished {
                    return;
                }
            }
        })();

        // CIRCLES
        self.choreographer_tween.update(globals.deltatime);
        (|| {
            let (percentage, finished) = self.choreographer_tween.tween(1.0);
            self.circle_radius = lerp(20.0, 50.0, easing::cubic_inout(percentage));
            if !finished {
                return;
            }

            let (percentage, finished) = self.choreographer_tween.tween(1.0);
            self.circle_radius = lerp(50.0, 20.0, easing::cubic_inout(percentage));
            if !finished {
                return;
            }

            self.choreographer_tween.restart();
        })();

        let circle_pos = Vec2::new(globals.canvas_width, globals.canvas_height);
        draw.draw_circle_filled(
            circle_pos - Vec2::filled(100.0),
            self.circle_radius,
            Drawparams::with_depth_drawspace(DEPTH_DRAW, Drawspace::World),
        );

        draw.draw_ring(
            circle_pos - Vec2::filled(100.0),
            60.0,
            10.0,
            Drawparams::with_depth_drawspace(DEPTH_DRAW, Drawspace::World),
        );

        // CROSS
        //
        let rect1_initial = Rect::from_xy_width_height(
            block_centered_in_point(50.0, center.x),
            block_centered_in_point(200.0, center.y),
            50.0,
            200.0,
        );
        let rect2_initial = Rect::from_xy_width_height(
            block_centered_in_point(200.0, center.x),
            block_centered_in_point(50.0, center.y),
            200.0,
            50.0,
        );

        let mut rect1_width = rect1_initial.width();
        let mut rect2_height = rect2_initial.height();
        self.choreographer_rectangles.update(globals.deltatime);
        (|| {
            if !self.choreographer_rectangles.wait(1.0) {
                return;
            }

            let (percentage, finished) = self.choreographer_rectangles.tween(1.0);
            let percentage = easing::cubic_inout(percentage);
            rect1_width = rect1_initial.width() * (1.0 - percentage);
            if !finished {
                return;
            }

            let (percentage, finished) = self.choreographer_rectangles.tween(1.0);
            let percentage = easing::cubic_inout(percentage);
            rect2_height = rect2_initial.height() * (1.0 - percentage);
            if !finished {
                return;
            }

            let (percentage, finished) = self.choreographer_rectangles.tween(1.0);
            let percentage = easing::cubic_inout(percentage);
            rect1_width = rect1_initial.width() * percentage;
            rect2_height = rect2_initial.height() * percentage;
            if !finished {
                return;
            }

            self.choreographer_rectangles.restart();
        })();
        let rect1 = rect1_initial.with_new_width(rect1_width, AlignmentHorizontal::Center);
        let rect2 = rect2_initial.with_new_height(rect2_height, AlignmentVertical::Center);
        draw.draw_rect(
            rect1,
            true,
            Drawparams::without_additivity(DEPTH_DRAW, Color::white(), Drawspace::World),
        );
        draw.draw_rect(
            rect2,
            true,
            Drawparams::without_additivity(DEPTH_DRAW, Color::white(), Drawspace::World),
        );

        // HP BAR
        //
        if input.keyboard.recently_pressed(Scancode::D) {
            audio.play_oneshot(
                "drum",
                SelectedScene::Choreographer as AudioGroupId,
                music_get_next_point_in_time(audio.current_time_seconds(), INTERVAL_QUARTERBEAT),
                0.7,
                1.0,
                0.0,
                None,
            );

            self.hp_previous = self.hp;
            self.hp -= globals.random.f32_in_range(0.15, 0.3);
            if self.hp <= 0.01 {
                self.hp = 0.01;
            }
            self.choreographer_hp_back.restart();
            self.choreographer_hp_front.restart();
            self.choreographer_hp_refill.restart();
        }
        let hp_rect_initial =
            Rect::from_xy_width_height(globals.canvas_width - 200.0, 50.0, 100.0, 30.0);
        let mut hp_front_value = self.hp;
        let mut hp_back_value = self.hp;

        self.choreographer_hp_refill.update(globals.deltatime);
        (|| {
            if !self.choreographer_hp_refill.wait(1.0) {
                return;
            }

            let (percentage, finished) = self.choreographer_hp_refill.tween(2.0);
            let percentage = easing::cubic_inout(percentage);
            self.hp_previous = self.hp;
            self.hp = lerp(self.hp, 1.0, percentage);
            if !finished {
                return;
            }
        })();

        self.choreographer_hp_front.update(globals.deltatime);
        (|| {
            let (percentage, finished) = self.choreographer_hp_front.tween(0.3);
            let percentage = easing::cubic_inout(percentage);
            hp_front_value = lerp(self.hp_previous, self.hp, percentage);
            if !finished {
                return;
            }
        })();
        self.choreographer_hp_back.update(globals.deltatime);
        (|| {
            let (percentage, finished) = self.choreographer_hp_back.tween(1.0);
            let percentage = easing::cubic_inout(percentage);
            hp_back_value = lerp(self.hp_previous, self.hp, percentage);
            if !finished {
                return;
            }
        })();

        let hp_front_rect = hp_rect_initial.with_new_width(
            hp_front_value * hp_rect_initial.width(),
            AlignmentHorizontal::Left,
        );
        let hp_back_rect = hp_rect_initial.with_new_width(
            hp_back_value * hp_rect_initial.width(),
            AlignmentHorizontal::Left,
        );

        draw.draw_text(
            "Press 'D'",
            &assets.get_font(FONT_DEFAULT_TINY_NAME),
            1.0,
            hp_rect_initial.pos,
            Vec2::filled_y(-5.0),
            Some(TextAlignment::top_left(true, false)),
            None,
            Drawparams::without_additivity(DEPTH_DRAW, Color::white(), Drawspace::World),
        );
        draw.draw_rect(
            hp_back_rect,
            true,
            Drawparams::without_additivity(
                DEPTH_DRAW,
                Color::from_hex_rgba(0x884242ff),
                Drawspace::World,
            ),
        );
        draw.draw_rect(
            hp_front_rect,
            true,
            Drawparams::without_additivity(
                DEPTH_DRAW,
                Color::from_hex_rgba(0xf06969ff),
                Drawspace::World,
            ),
        );

        // PRINTING RANDOM NUMBERS
        //
        self.choreographer_randoms.update(globals.deltatime);
        (|| {
            for index in 0..10 {
                if !self.choreographer_randoms.wait(0.5) {
                    return;
                }

                if self.choreographer_randoms.once() {
                    println!("Random number {}: {}", index, globals.random.u32());
                }
            }
        })();

        // Text drawing test
        let test_font = assets.get_font("Grand9K_Pixel_bordered");
        let text = "Loaded font test gorgeous!|\u{08A8}";
        let text_width = test_font.get_text_bounding_rect(text, 1, false).dim.x;
        // Draw origin is top-left
        let draw_pos = Vec2::new(5.0, globals.canvas_height - 50.0);
        draw.draw_text(
            text,
            &test_font,
            1.0,
            draw_pos,
            Vec2::zero(),
            None,
            None,
            Drawparams::without_additivity(20.0, Color::magenta(), Drawspace::World),
        );
        draw.draw_line_bresenham(
            draw_pos + Vec2::new(0.0, test_font.baseline as f32),
            draw_pos + Vec2::new(text_width as f32, test_font.baseline as f32),
            false,
            Drawparams::without_additivity(20.0, 0.3 * Color::yellow(), Drawspace::World),
        );
        // Draw origin is baseline
        let draw_pos = Vec2::new(5.0, globals.canvas_height - 25.0);
        draw.draw_text(
            text,
            &test_font,
            1.0,
            draw_pos,
            Vec2::zero(),
            Some(TextAlignment::top_left(true, false)),
            None,
            Drawparams::without_additivity(20.0, Color::magenta(), Drawspace::World),
        );
        draw.draw_line_bresenham(
            draw_pos,
            draw_pos + Vec2::new(text_width as f32, 0.0),
            false,
            Drawparams::without_additivity(20.0, 0.3 * Color::yellow(), Drawspace::World),
        );
    }
}

// Based on https://github.com/RandyGaul/cute_headers/blob/master/cute_coroutine.h
fn collect_line(
    choreo: &mut Choreographer,
    random: &mut Random,
    name: &str,
    text: &str,
) -> (String, bool) {
    let mut line_accumulator = name.to_owned() + ": ";

    if !choreo.wait(0.750) {
        return (line_accumulator, false);
    }

    for letter in text.chars() {
        line_accumulator.push(letter);
        let pause_time = if letter == '.' || letter == ',' || letter == '?' {
            0.250
        } else {
            random.f32_in_range(0.03, 0.05)
        };

        if !choreo.wait(pause_time) {
            return (line_accumulator, false);
        }
    }

    (line_accumulator, true)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SPRITES

const DEPTH_SORCY: Depth = 20.0;
const DEPTH_RECT: Depth = 20.0;
const DEPTH_GHOSTY: Depth = 40.0;
const DEPTH_BACKGROUND: Depth = 0.0;

const AFTERIMAGE_LIFETIME: f32 = 0.3;
const AFTERIMAGE_ADDITIVITY_START: Additivity = ADDITIVITY_MAX;
const AFTERIMAGE_ADDITIVITY_END: Additivity = ADDITIVITY_MAX;
const AFTERIMAGE_COLOR_START: Color = Color::new(0.5, 0.5, 0.5, 0.5);
const AFTERIMAGE_COLOR_END: Color = Color::transparent();
const AFTERIMAGE_COUNT_MAX: usize = 8;

#[derive(Clone)]
struct SceneSprites {
    anim_sorcy_idle: AnimationPlayer<Sprite>,
    anim_sorcy_run: AnimationPlayer<Sprite>,
    anim_ghosty_idle: AnimationPlayer<Sprite>,

    anim_test_wiggle: AnimationPlayer<f32>,
    anim_test_squash_horizontal: AnimationPlayer<f32>,
    anim_test_squash_vertical: AnimationPlayer<f32>,

    ghosty_afterimage: Afterimage,

    music_stream_id: AudioStreamId,
}

impl SceneSprites {
    pub fn new(
        _draw: &mut Drawstate,
        _audio: &mut Audiostate,
        assets: &GameAssets,
        _input: &InputState,
    ) -> SceneSprites {
        let anim_rotate = {
            let mut anim = Animation::new_empty("test_wiggle".to_owned());
            anim.add_frame(0.1, -20.0);
            anim.add_frame(0.1, -10.0);
            anim.add_frame(0.1, 0.0);
            anim.add_frame(0.1, 10.0);
            anim.add_frame(0.1, 20.0);
            anim.add_frame(0.1, 10.0);
            anim.add_frame(0.1, 0.0);
            anim.add_frame(0.1, -10.0);
            AnimationPlayer::new_from_beginning(anim, 1.0, false)
        };

        let anim_squash_horizontal = {
            let mut anim = Animation::new_empty("test_squash_x".to_owned());
            anim.add_frame(0.1, 1.6);
            anim.add_frame(0.1, 1.5);
            anim.add_frame(0.1, 1.2);
            anim.add_frame(0.1, 1.0);
            anim.add_frame(0.1, 1.0);
            anim.add_frame(0.1, 1.2);
            anim.add_frame(0.1, 1.5);
            AnimationPlayer::new_from_beginning(anim, 1.0, false)
        };

        let anim_squash_vertical = {
            let mut anim = Animation::new_empty("test_squash_y".to_owned());
            anim.add_frame(0.1, 1.0 - 0.6);
            anim.add_frame(0.1, 1.0 - 0.5);
            anim.add_frame(0.1, 1.0 - 0.2);
            anim.add_frame(0.1, 1.0 - 0.0);
            anim.add_frame(0.1, 1.0 - 0.0);
            anim.add_frame(0.1, 1.0 - 0.2);
            anim.add_frame(0.1, 1.0 - 0.5);
            AnimationPlayer::new_from_beginning(anim, 1.0, false)
        };

        let anim_sorcy_idle =
            AnimationPlayer::new_from_beginning(assets.get_anim("sorcy:idle").clone(), 1.0, false);
        let anim_sorcy_run =
            AnimationPlayer::new_from_beginning(assets.get_anim("sorcy:run").clone(), 1.0, false);
        let anim_ghosty_idle =
            AnimationPlayer::new_from_beginning(assets.get_anim("ghosty:idle").clone(), 1.0, false);

        let ghosty_afterimage = Afterimage::new(
            AFTERIMAGE_LIFETIME,
            AFTERIMAGE_ADDITIVITY_START,
            AFTERIMAGE_ADDITIVITY_END,
            AFTERIMAGE_COLOR_START,
            AFTERIMAGE_COLOR_END,
            AFTERIMAGE_COUNT_MAX,
        );

        SceneSprites {
            anim_sorcy_idle,
            anim_sorcy_run,
            anim_ghosty_idle,

            anim_test_wiggle: anim_rotate,
            anim_test_squash_horizontal: anim_squash_horizontal,
            anim_test_squash_vertical: anim_squash_vertical,

            ghosty_afterimage,
            music_stream_id: 0,
        }
    }

    pub fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        _out_systemcommands: &mut Vec<AppCommand>,
    ) {
        if self.music_stream_id == 0 {
            self.music_stream_id = audio.play(
                "bgboss",
                SelectedScene::Sprites as AudioGroupId,
                music_get_next_point_in_time(
                    audio.current_time_seconds(),
                    MusicalInterval::Measure {
                        beats_per_minute: 140,
                        beats_per_measure: 4,
                    },
                ),
                true,
                0.5,
                1.0,
                0.0,
                None,
            );
        }
        if input.keyboard.recently_pressed(Scancode::M) {
            audio.stream_mute(self.music_stream_id);
        }
        if input.keyboard.recently_pressed(Scancode::U) {
            audio.stream_unmute(self.music_stream_id);
        }

        let audiotime = audio.current_time_seconds_smoothed();
        let measure_length = INTERVAL_MEASURE.length_seconds();
        let measure_completion_ratio = ((audiotime % measure_length) / measure_length) as f32;
        let beat_completion_ratio = (4.0 * measure_completion_ratio) % 1.0;
        let halfbeat_completion_ratio = (8.0 * measure_completion_ratio) % 1.0;
        let measure_completion_angle = measure_completion_ratio * 360.0;

        draw.draw_sprite(
            assets.get_sprite("background"),
            Transform::from_pos(Vec2::zero()),
            false,
            false,
            Drawparams::without_additivity(DEPTH_BACKGROUND, Color::white(), Drawspace::World),
        );

        // TEST TRANSLUCENCE
        {
            let sorcy_pos = Vec2::new(CANVAS_WIDTH / 3.0, CANVAS_HEIGHT - 40.0);

            let ghosty_cycle = 2.0 * DEGREE_TO_RADIANS * measure_completion_angle / 2.0;
            let ghosty_pos = sorcy_pos + Vec2::filled_x(50.0 * (f32::cos(ghosty_cycle)));
            let flip = f32::sin(ghosty_cycle) > 0.0;
            let ghosty_xform = Transform::from_pos_scale(
                ghosty_pos,
                if flip {
                    3.0 * Vec2::ones()
                } else {
                    3.0 * Vec2::new(-1.0, 1.0)
                },
            )
            .pixel_snapped();

            // Draw sorcy before but in front of ghosty
            draw.draw_sprite(
                self.anim_sorcy_idle
                    .frame_at_percentage(measure_completion_ratio),
                Transform::from_pos(sorcy_pos + Vec2::filled_x(10.0)).pixel_snapped(),
                false,
                false,
                Drawparams::without_additivity(
                    DEPTH_GHOSTY + 1.0,
                    Color::white(),
                    Drawspace::World,
                ),
            );
            // Draw translucent additive ghosty
            draw.draw_sprite(
                self.anim_ghosty_idle
                    .frame_at_percentage(beat_completion_ratio),
                ghosty_xform,
                false,
                false,
                Drawparams::new(
                    DEPTH_GHOSTY,
                    0.5 * Color::white(),
                    0.5 * ADDITIVITY_MAX,
                    Drawspace::World,
                ),
            );
            // Draw sorcy after but behind ghosty
            draw.draw_sprite(
                self.anim_sorcy_idle
                    .frame_at_percentage(measure_completion_ratio),
                Transform::from_pos(sorcy_pos - Vec2::filled_x(10.0)).pixel_snapped(),
                false,
                true,
                Drawparams::without_additivity(
                    DEPTH_GHOSTY - 1.0,
                    Color::white(),
                    Drawspace::World,
                ),
            );
        }

        // ROTATING RECT
        {
            let testpos = Vec2::new(CANVAS_WIDTH - 50.0, CANVAS_HEIGHT - 60.0);
            draw.draw_rect_transformed(
                Vec2::new(30.0, 30.0),
                true,
                false,
                Vec2::zero(),
                Transform::from_pos_angle(testpos, measure_completion_angle).pixel_snapped(),
                Drawparams::without_additivity(DEPTH_RECT, Color::white(), Drawspace::World),
            );
            draw.draw_pixel(
                testpos,
                Drawparams::without_additivity(DEPTH_RECT, Color::magenta(), Drawspace::World),
            );
        }

        let sorcy_sprite_running = self
            .anim_sorcy_run
            .frame_at_percentage(halfbeat_completion_ratio);
        let xform_anim_wiggle = Transform {
            pos: Vec2::new(CANVAS_WIDTH - 50.0, CANVAS_HEIGHT / 3.0),
            scale: Vec2::new(1.0, 1.0),
            dir_angle: *self
                .anim_test_wiggle
                .frame_at_percentage(beat_completion_ratio),
        }
        .pixel_snapped();
        let xform_anim_squash = Transform {
            pos: Vec2::new(CANVAS_WIDTH - 100.0, CANVAS_HEIGHT / 3.0),
            scale: Vec2::new(
                *self
                    .anim_test_squash_horizontal
                    .frame_at_percentage(beat_completion_ratio),
                *self
                    .anim_test_squash_vertical
                    .frame_at_percentage(beat_completion_ratio),
            ),
            dir_angle: 0.0,
        }
        .pixel_snapped();
        let xform_rotating = Transform {
            pos: Vec2::new(CANVAS_WIDTH - 170.0, CANVAS_HEIGHT / 3.0),
            scale: Vec2::ones(),
            dir_angle: -measure_completion_angle,
        }
        .pixel_snapped();
        let xform_rotating_flipped = Transform {
            pos: Vec2::new(CANVAS_WIDTH - 170.0, CANVAS_HEIGHT / 3.0),
            scale: -Vec2::ones(),
            dir_angle: -measure_completion_angle,
        }
        .pixel_snapped();

        let xforms = [
            xform_anim_wiggle,
            xform_anim_squash,
            xform_rotating,
            xform_rotating_flipped,
        ];

        for &xform in &xforms {
            draw.draw_sprite(
                sorcy_sprite_running,
                xform,
                false,
                false,
                Drawparams::without_additivity(DEPTH_SORCY, Color::white(), Drawspace::World),
            );

            // ROTATION ANIMATION WITH ATTACHMENT
            let mut xform = xform;
            xform.pos.y += 60.0;

            draw.draw_sprite(
                sorcy_sprite_running,
                xform,
                false,
                false,
                Drawparams::without_additivity(DEPTH_SORCY, Color::white(), Drawspace::World),
            );

            let attachable_0 = sorcy_sprite_running.get_attachment_point_transformed(0, xform);
            let attachable_1 = sorcy_sprite_running.get_attachment_point_transformed(1, xform);
            let attachable_2 = sorcy_sprite_running.get_attachment_point_transformed(2, xform);
            let attachable_3 = sorcy_sprite_running.get_attachment_point_transformed(3, xform);
            draw.draw_pixel(
                attachable_0,
                Drawparams::without_additivity(DEPTH_SORCY, Color::red(), Drawspace::World),
            );
            draw.draw_pixel(
                attachable_1,
                Drawparams::without_additivity(DEPTH_SORCY, Color::green(), Drawspace::World),
            );
            draw.draw_pixel(
                attachable_2,
                Drawparams::without_additivity(DEPTH_SORCY, Color::magenta(), Drawspace::World),
            );
            draw.draw_pixel(
                attachable_3,
                Drawparams::without_additivity(DEPTH_SORCY, Color::black(), Drawspace::World),
            );

            // pivot
            draw.draw_pixel(
                xform.pos,
                Drawparams::without_additivity(DEPTH_SORCY, Color::magenta(), Drawspace::World),
            );
        }

        // AFTERIMAGES
        {
            let ghosty_cycle = 2.0 * DEGREE_TO_RADIANS * measure_completion_angle;
            let ghosty_pos = Vec2::new(260.0 + 50.0 * f32::cos(ghosty_cycle), 35.0);
            let flip = f32::sin(ghosty_cycle) > 0.0;

            let ghosty_xform = Transform::from_pos_scale(
                ghosty_pos,
                if flip {
                    Vec2::ones()
                } else {
                    Vec2::new(-1.0, 1.0)
                },
            )
            .pixel_snapped();
            draw.draw_sprite(
                self.anim_ghosty_idle
                    .frame_at_percentage(beat_completion_ratio),
                ghosty_xform,
                false,
                false,
                Drawparams::new(
                    DEPTH_GHOSTY,
                    0.5 * Color::white(),
                    0.4 * ADDITIVITY_MAX,
                    Drawspace::World,
                ),
            );
            self.ghosty_afterimage.add_afterimage_image_if_needed(
                globals.deltatime,
                self.anim_ghosty_idle
                    .frame_at_percentage(beat_completion_ratio)
                    .clone(),
                ghosty_xform,
                false,
                false,
                0.5 * Color::white(),
                0.4 * ADDITIVITY_MAX,
            );
            self.ghosty_afterimage.update_and_draw(
                draw,
                globals.deltatime,
                DEPTH_GHOSTY - 1.0,
                Drawspace::World,
            );
        }

        draw.debug_log(format!("helle klaine fee {:.3}", input.real_world_uptime));
        draw.debug_log_color(
            format!("sorcy wiggle {:.3}", measure_completion_angle),
            Color::yellow(),
        );
        draw.debug_log_color(
            format!(
                "sorcy squish {:.3}",
                self.anim_test_squash_horizontal
                    .frame_at_percentage(beat_completion_ratio)
            ),
            Color::magenta(),
        );
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// SPRITES 3D AND SPATIAL AUDIO

const DEPTH_SUSI: Depth = 20.0;
const DEPTH_GUI: Depth = 50.0;
const COLOR_BACKGROUND: Color = Color::from_rgb(16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0);

const TILE_SIZE: f32 = 16.0;

const MOVE_ACC: f32 = 500.0;
const MOVE_VEL: f32 = 12.0 * TILE_SIZE;
const MOVE_DEACCEL: f32 = 1000.0;
const REVERSE_MOVE_ACC_MULTIPLIER: f32 = 4.0;

const TURN_ACC: f32 = 3000.0;
const TURN_VEL: f32 = 2000.0;
const TURN_DEACCEL: f32 = 5000.0;
const REVERSE_TURN_ACC_MULTIPLIER: f32 = 5.0;

const MIN_ANGLE_DIFF_TO_START_MOVE_THRESHOLD: f32 = 35.0;
const MIN_ANGLE_DIFF_TO_START_TURN_THRESHOLD: f32 = 2.0;

const AXIS_CONSIDERED_PRESSED_THRESHOLD: f32 = 0.5;

#[derive(Default, Debug, Copy, Clone)]
struct Motion {
    pub vel: Vec2,
    pub acc: Vec2,

    /// Given in degrees [-360, 360] counterclockwise
    pub dir_angle_vel: f32,
    /// Given in degrees [-360, 360] counterclockwise
    pub dir_angle_acc: f32,
}
#[derive(Clone)]
struct EngineSound {
    stream_stand: AudioStreamId,
    stream_move: AudioStreamId,
    volume_base: f32,
    volume_stand: f32,
    volume_move: f32,
    pitch: f32,
}

impl EngineSound {
    pub fn new(
        pos: Vec2,
        audio: &mut Audiostate,
        volume_base: f32,
        volume_stand: f32,
        volume_move: f32,
        sound_name_stand: &str,
        sound_name_move: &str,
    ) -> EngineSound {
        let stream_stand = audio.play(
            sound_name_stand,
            SelectedScene::Sprites3DSpatialSound as AudioGroupId,
            0.0,
            true,
            volume_base * volume_stand,
            1.0,
            0.0,
            Some(AudioSpatialParams::new(
                pos,
                Vec2::zero(),
                1.0,
                AudioFalloffType::Natural,
                200.0,
                1000.0,
            )),
        );

        let stream_move = audio.play(
            sound_name_move,
            SelectedScene::Sprites3DSpatialSound as AudioGroupId,
            0.0,
            true,
            0.0,
            1.0,
            0.0,
            Some(AudioSpatialParams::new(
                pos,
                Vec2::zero(),
                1.0,
                AudioFalloffType::Natural,
                200.0,
                1000.0,
            )),
        );
        EngineSound {
            stream_stand,
            stream_move,
            pitch: 1.0,
            volume_base,
            volume_stand,
            volume_move,
        }
    }

    pub fn update(&mut self, pos: Vec2, vel: Vec2, audio: &mut Audiostate, speed_percent: f32) {
        let (volume_stand, volume_move) =
            crossfade_sinuoidal(self.volume_base, f32::clamp(speed_percent, 0.1, 0.9));

        let playback_speed_stand = 2.0;
        let playback_speed_move = 2.0 + speed_percent;

        audio.stream_set_volume(self.stream_stand, volume_stand * self.volume_stand);
        audio.stream_set_volume(self.stream_move, volume_move * self.volume_move);
        audio.stream_set_playback_speed(self.stream_stand, playback_speed_stand);
        audio.stream_set_playback_speed(self.stream_move, playback_speed_move);
        audio.stream_set_spatial_pos(self.stream_stand, pos);
        audio.stream_set_spatial_pos(self.stream_move, pos);
        audio.stream_set_spatial_vel(self.stream_stand, vel);
        audio.stream_set_spatial_vel(self.stream_move, vel);
    }
}

#[derive(Clone)]
struct Susi {
    xform: Transform,

    sprite_anim_player_base: AnimationPlayer<Sprite3D>,
    sprite_crosshair: Sprite,
    engine_sound: EngineSound,

    speed: f32,
    turn_speed: f32,

    target_position: Vec2,
}

impl Susi {
    pub fn new(pos: Vec2, dir_angle: f32, assets: &GameAssets, audio: &mut Audiostate) -> Susi {
        let anim_base = assets.get_anim_3d("susi_base_3d:default").clone();
        let sprite_anim_player_base = AnimationPlayer::new_from_beginning(anim_base, 1.0, true);
        let sprite_crosshair = assets.get_sprite("crosshair").clone();

        Susi {
            xform: Transform::from_pos_angle(pos, dir_angle),

            sprite_anim_player_base,
            sprite_crosshair,

            engine_sound: EngineSound::new(
                pos,
                audio,
                1.0,
                1.0,
                0.7,
                "engine_standing",
                "engine_running",
            ),

            speed: 0.0,
            turn_speed: 0.0,

            target_position: pos,
        }
    }

    pub fn update(
        &mut self,
        input: &InputState,
        audio: &mut Audiostate,
        draw: &mut Drawstate,
        globals: &Globals,
    ) {
        if input.mouse.button_left.recently_pressed() {
            self.target_position = globals.cursors.mouse.pos_world;
        }
        if let Some(finger) = globals.cursors.finger_primary {
            self.target_position = finger.pos_world;
        };

        // MOVE DIRECTION
        let input_move_dir = {
            let mut input_dir = Vec2::zero();
            if input.keyboard.is_down(Scancode::ArrowUp) || input.keyboard.is_down(Scancode::W) {
                input_dir += Vec2::new(0.0, -1.0);
            }
            if input.keyboard.is_down(Scancode::ArrowDown) || input.keyboard.is_down(Scancode::S) {
                input_dir += Vec2::new(0.0, 1.0);
            }
            if input.keyboard.is_down(Scancode::ArrowLeft) || input.keyboard.is_down(Scancode::A) {
                input_dir += Vec2::new(-1.0, 0.0);
            }
            if input.keyboard.is_down(Scancode::ArrowRight) || input.keyboard.is_down(Scancode::D) {
                input_dir += Vec2::new(1.0, 0.0);
            }

            if input_dir != Vec2::zero() {
                input_dir = input_dir.normalized();
            }

            let input_analog = if input.gamepad.is_connected {
                input.gamepad.stick_left.square_to_disk_transform()
            } else {
                Vec2::zero()
            };
            if input_analog.magnitude() > AXIS_CONSIDERED_PRESSED_THRESHOLD {
                input_dir = input_analog.normalized()
            }

            if input_dir.is_zero() {
                let diff_target = self.target_position - self.xform.pos;
                if diff_target.magnitude() > 10.0 {
                    input_dir = (self.target_position - self.xform.pos).clamped_abs(1.0);
                }
            }

            input_dir
        };

        let (acc_dir, turn_dir) = Susi::get_acc_and_turn_direction_for_input_direction(
            self.xform.dir_angle,
            input_move_dir,
        );
        Susi::update_move_and_turn_speed(
            &mut self.speed,
            &mut self.turn_speed,
            acc_dir,
            turn_dir,
            globals,
        );

        self.xform.dir_angle = (self.xform.dir_angle + self.turn_speed * globals.deltatime) % 360.0;
        let dir = Vec2::from_angle_flipped_y(self.xform.dir_angle);
        let vel = self.speed * dir;
        let move_distance = globals.deltatime * vel;
        self.xform.pos += move_distance;

        let move_speed_ratio = f32::clamp(
            self.speed.abs() / MOVE_VEL + self.turn_speed.abs() / TURN_VEL,
            0.0,
            1.0,
        );
        self.sprite_anim_player_base.playback_speed = self.speed.signum() * move_speed_ratio * 20.0;
        self.sprite_anim_player_base.update(globals.deltatime);
        self.engine_sound
            .update(self.xform.pos, vel, audio, move_speed_ratio);

        let sprite_base = self.sprite_anim_player_base.current_frame();
        draw.draw_sprite_3d(
            &sprite_base,
            self.xform,
            Drawparams::without_additivity(DEPTH_SUSI, Color::white(), Drawspace::World),
        );

        if Vec2::distance(self.xform.pos, self.target_position) > 5.0 {
            draw.draw_sprite(
                &self.sprite_crosshair,
                Transform::from_pos(self.target_position),
                false,
                false,
                Drawparams::without_additivity(DEPTH_GUI, Color::white(), Drawspace::World),
            );
        }
    }

    /// Returns the move and turn direction necessary to move into the given `input_dir`
    fn get_acc_and_turn_direction_for_input_direction(
        current_dir_angle: f32,
        desired_move_dir: Vec2,
    ) -> (f32, f32) {
        // Forward (1) or backward (-1) or stop (0)
        let mut acc_dir = 0.0;
        // Left (-1) or right (1) or stop (0)
        let mut turn_dir = 0.0;

        if desired_move_dir == Vec2::zero() {
            return (acc_dir, turn_dir);
        }

        let current_dir = Vec2::from_angle_flipped_y(current_dir_angle);
        let dot_dir_input = Vec2::dot(current_dir, desired_move_dir);
        let dot_dir_input_abs = dot_dir_input.abs();
        let angle_dir_input = Vec2::signed_angle_between(current_dir, desired_move_dir);
        let angle_dir_input_abs = angle_dir_input.abs();

        // Turn
        if angle_dir_input_abs % 180.0 <= MIN_ANGLE_DIFF_TO_START_TURN_THRESHOLD
            || (180.0 - angle_dir_input_abs) <= MIN_ANGLE_DIFF_TO_START_TURN_THRESHOLD
        {
            turn_dir = 0.0
        } else {
            turn_dir = angle_dir_input.signum();
            if dot_dir_input < 0.0 {
                turn_dir = -turn_dir;
            }
        }

        // Move
        if angle_dir_input_abs <= MIN_ANGLE_DIFF_TO_START_MOVE_THRESHOLD {
            acc_dir = dot_dir_input_abs;
        } else if (180.0 - angle_dir_input_abs) <= MIN_ANGLE_DIFF_TO_START_MOVE_THRESHOLD {
            acc_dir = -dot_dir_input_abs;
        }

        (acc_dir, turn_dir)
    }

    pub fn update_move_and_turn_speed(
        in_out_speed: &mut f32,
        in_out_turn_speed: &mut f32,
        move_direction: f32,
        turn_direction: f32,
        globals: &Globals,
    ) {
        // Accelerate forward/backward
        let mut speed = *in_out_speed;
        if move_direction == 0.0 {
            let mut speed_abs = speed.abs();
            speed_abs -= globals.deltatime * MOVE_DEACCEL;
            if speed_abs < 0.0 {
                speed_abs = 0.0;
            }
            speed = speed.signum() * speed_abs;
        } else {
            let move_acc_multiplier =
                if is_effectively_zero(speed) || move_direction == speed.signum() {
                    1.0
                } else {
                    // If the direction input is in the opposite direction of the current velocity, we
                    // want to change directions fast
                    REVERSE_MOVE_ACC_MULTIPLIER
                };

            let increment = move_direction * globals.deltatime * move_acc_multiplier * MOVE_ACC;
            speed = clampf_absolute(speed + increment, MOVE_VEL);
        }
        *in_out_speed = speed;

        // Turn left/right
        let mut turn_speed = *in_out_turn_speed;
        if turn_direction == 0.0 {
            let mut turn_speed_abs = turn_speed.abs();
            turn_speed_abs -= globals.deltatime * TURN_DEACCEL;
            if turn_speed_abs < 0.0 {
                turn_speed_abs = 0.0;
            }
            turn_speed = turn_speed.signum() * turn_speed_abs;
        } else {
            let turn_acc_multiplier =
                if is_effectively_zero(turn_speed) || turn_direction == turn_speed.signum() {
                    1.0
                } else {
                    // If the direction input is in the opposite direction of the current velocity, we
                    // want to change directions fast
                    REVERSE_TURN_ACC_MULTIPLIER
                };

            let increment = turn_direction * globals.deltatime * turn_acc_multiplier * TURN_ACC;
            turn_speed = clampf_absolute(turn_speed + increment, TURN_VEL);
        }
        *in_out_turn_speed = turn_speed;
    }
}

#[derive(Clone)]
pub struct SceneSprites3dSpatialSound {
    susi: Susi,
    tilemap: Grid<u32>,
    music_stream_id: AudioStreamId,
}

impl SceneSprites3dSpatialSound {
    pub fn new(
        _draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        _input: &InputState,
        globals: &mut Globals,
    ) -> SceneSprites3dSpatialSound {
        let tilemap_buffer: Vec<u32> = vec![
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0],
        ]
        .iter()
        .flatten()
        .map(|value| *value as u32)
        .collect();
        let tilemap = Grid::new_from_buffer(16, 16, tilemap_buffer);
        let mut tilemap_final = Grid::new_filled(64, 64, 0);
        for y in 0..4 {
            for x in 0..4 {
                tilemap.blit_to(
                    &mut tilemap_final,
                    TILE_SIZE as i32 * Vec2i::new(x, y),
                    None,
                );
            }
        }

        let susi = Susi::new(
            Vec2::new(globals.canvas_width, globals.canvas_height) / 2.0,
            0.0,
            assets,
            audio,
        );
        SceneSprites3dSpatialSound {
            susi,

            tilemap: tilemap_final,
            music_stream_id: 0,
        }
    }

    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        _out_systemcommands: &mut Vec<AppCommand>,
    ) {
        if self.music_stream_id == 0 {
            self.music_stream_id = audio.play(
                "bg_title",
                SelectedScene::Sprites3DSpatialSound as AudioGroupId,
                music_get_next_point_in_time(
                    audio.current_time_seconds(),
                    MusicalInterval::Measure {
                        beats_per_minute: 140,
                        beats_per_measure: 4,
                    },
                ),
                true,
                0.5,
                1.0,
                0.0,
                None,
            );
        }

        if input.keyboard.recently_pressed(Scancode::M) {
            audio.stream_mute(self.music_stream_id);
        }
        if input.keyboard.recently_pressed(Scancode::U) {
            audio.stream_unmute(self.music_stream_id);
        }

        draw.set_clear_color_and_depth(COLOR_BACKGROUND, DEPTH_BACKGROUND);

        draw.draw_text(
            "Press 'B'",
            &assets.get_font(FONT_DEFAULT_TINY_NAME),
            1.0,
            Vec2::new(CANVAS_WIDTH / 2.0, 20.0),
            Vec2::filled_y(-5.0),
            Some(TextAlignment::top_left(true, false)),
            None,
            Drawparams::without_additivity(DEPTH_DRAW, Color::white(), Drawspace::Canvas),
        );
        if input.keyboard.recently_pressed(Scancode::B) {
            let screen_shake = ModulatorScreenShake::new(&mut globals.random, 4.0, 1.0, 60.0);
            globals.camera.add_shake(screen_shake);
        }

        let sprite_tile = assets.get_sprite("test_tile");
        let sprite_wall = assets.get_sprite("test_wall");
        for y in 0..self.tilemap.height {
            for x in 0..self.tilemap.width {
                let rect = self.tilemap.get_cell_rect(x, y, TILE_SIZE as i32);
                let sprite = if self.tilemap.get(x, y) == 0 {
                    sprite_tile
                } else {
                    sprite_wall
                };

                draw.draw_sprite(
                    sprite,
                    Transform::from_pos(rect.pos.to_vec2()),
                    false,
                    false,
                    Drawparams::without_additivity(
                        DEPTH_BACKGROUND + y as f32 / self.tilemap.height as f32,
                        Color::white(),
                        Drawspace::World,
                    ),
                );
            }
        }

        self.susi.update(input, audio, draw, globals);

        audio.set_global_listener_pos(globals.camera.center());
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CREDITS

#[derive(Clone)]
pub struct SceneCredits {
    music_stream_id: AudioStreamId,
    color_background: Color,
}

impl SceneCredits {
    pub fn new(
        _draw: &mut Drawstate,
        _audio: &mut Audiostate,
        _assets: &GameAssets,
        _input: &InputState,
        _globals: &mut Globals,
    ) -> SceneCredits {
        SceneCredits {
            music_stream_id: 0,
            color_background: Color::black(),
        }
    }

    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        _out_systemcommands: &mut Vec<AppCommand>,
    ) {
        if self.music_stream_id == 0 {
            self.music_stream_id = audio.play(
                "bgboss",
                SelectedScene::Credits as AudioGroupId,
                music_get_next_point_in_time(
                    audio.current_time_seconds(),
                    MusicalInterval::Measure {
                        beats_per_minute: 140,
                        beats_per_measure: 4,
                    },
                ),
                true,
                0.5,
                1.0,
                0.0,
                None,
            );
        }
        if input.keyboard.recently_pressed(Scancode::M) {
            audio.stream_mute(self.music_stream_id);
        }
        if input.keyboard.recently_pressed(Scancode::U) {
            audio.stream_unmute(self.music_stream_id);
        }

        if let Some(new_value_r) = gui_horizontal_slider(
            draw,
            GuiElemId::new("background_color_r"),
            Rect::from_xy_width_height(5.0, globals.canvas_height - 75.0, 100.0, 20.0),
            self.color_background.r,
            DEPTH_DRAW,
        ) {
            self.color_background.r = new_value_r;
        }
        if let Some(new_value_g) = gui_horizontal_slider(
            draw,
            GuiElemId::new("background_color_g"),
            Rect::from_xy_width_height(5.0, globals.canvas_height - 50.0, 100.0, 20.0),
            self.color_background.g,
            DEPTH_DRAW,
        ) {
            self.color_background.g = new_value_g;
        }
        if let Some(new_value_b) = gui_horizontal_slider(
            draw,
            GuiElemId::new("background_color_b"),
            Rect::from_xy_width_height(5.0, globals.canvas_height - 25.0, 100.0, 20.0),
            self.color_background.b,
            DEPTH_DRAW,
        ) {
            self.color_background.b = new_value_b;
        }

        unsafe {
            static mut CREDITS_SCROLLER_POS: f32 = 0.0;
            static mut CREDITS_SCROLLER_VEL: f32 = 0.0;
            static mut CREDITS_SCROLLER_ACC: f32 = 0.0;

            let font = assets.get_font("default_tiny_bordered");
            let credits_text = String::from_utf8_lossy(assets.get_content_filedata("credits.txt"));
            let credits_text = font.wrap_text_for_pixelwidth(&credits_text, 300);
            let creadits_text_linecount = credits_text.lines().count();
            gui_text_scroller(
                draw,
                GuiElemId::new("credits"),
                globals.deltatime,
                Rect::from_xy_width_height(130.0, 50.0, 300.0, 200.0),
                assets.get_font("default_tiny_bordered"),
                1.0,
                Color::white(),
                &credits_text,
                creadits_text_linecount,
                &mut CREDITS_SCROLLER_POS,
                &mut CREDITS_SCROLLER_VEL,
                &mut CREDITS_SCROLLER_ACC,
                DEPTH_DRAW,
            );
        }

        draw.set_clear_color_and_depth(self.color_background, DEPTH_CLEAR);
    }
}
