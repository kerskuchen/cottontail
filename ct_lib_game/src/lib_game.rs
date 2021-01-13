pub mod animations_fx;
pub mod assets;
pub mod camera;
pub mod choreographer;
pub mod debug;
pub mod gui;
pub mod input;

pub use animations_fx::*;
pub use assets::*;
pub use camera::*;
pub use choreographer::*;
pub use debug::*;
pub use gui::*;
pub use input::*;

use camera::{CursorCoords, Cursors, GameCamera};
use choreographer::LoadingScreen;
use debug::{debug_draw_crosshair, debug_draw_grid};

use ct_lib_audio::*;
use ct_lib_draw::*;
use ct_lib_image::*;
use ct_lib_math::*;
use ct_lib_window::{
    platform::audio::AudioOutput, renderer_opengl::Renderer, run_main, AppCommand, AppEventHandler,
    AppInfo, FingerPlatformId, MouseButton,
};

use ct_lib_core::serde_derive::{Deserialize, Serialize};
use ct_lib_core::*;

use std::collections::VecDeque;

pub const DEPTH_DEBUG: Depth = 90.0;
pub const DEPTH_DEVELOP_OVERLAY: Depth = 80.0;
pub const DEPTH_SPLASH: Depth = 70.0;
pub const DEPTH_SCREEN_FADER: Depth = 60.0;

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
////////////////////////////////////////////////////////////////////////////////////////////////////
// Gamestate

#[derive(Clone)]
pub struct Globals {
    pub random: Random,
    pub camera: GameCamera,
    pub cursors: Cursors,

    pub debug_deltatime_speed_factor: f32,
    pub deltatime_speed_factor: f32,
    pub deltatime: f32,
    pub is_paused: bool,

    pub canvas_width: f32,
    pub canvas_height: f32,
}

pub struct GameInfo {
    pub game_window_title: String,
    pub game_save_folder_name: String,
    pub game_company_name: String,
}

pub trait GameStateInterface: Clone {
    fn get_game_info() -> GameInfo;
    fn get_window_config() -> WindowConfig;
    fn new(
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
    ) -> Self;
    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &InputState,
        globals: &mut Globals,
        out_systemcommands: &mut Vec<AppCommand>,
    );
}

const SPLASHSCREEN_FADEIN_TIME: f32 = 0.3;
const SPLASHSCREEN_FADEOUT_TIME: f32 = 0.5;

struct AppResources {
    pub assets: Option<GameAssets>,
    pub draw: Option<Drawstate>,
    pub audio: Option<Audiostate>,
    pub globals: Option<Globals>,
    pub input: Option<InputState>,
    pub out_systemcommands: Option<&'static mut Vec<AppCommand>>,
    // TODO input_recorder :Option<InputRecorder<AppTicker<>>
}

static mut APP_RESOURCES: AppResources = AppResources {
    assets: None,
    draw: None,
    audio: None,
    globals: None,
    input: None,
    out_systemcommands: None,
};

#[inline(always)]
fn get_resources() -> &'static mut AppResources {
    unsafe { &mut APP_RESOURCES }
}
#[inline(always)]
fn get_input() -> &'static mut InputState {
    unsafe { APP_RESOURCES.input.as_mut().unwrap() }
}

#[derive(Clone)]
pub struct AppTicker<GameStateType: GameStateInterface> {
    pub game: Option<GameStateType>,
    pub loadingscreen: LoadingScreen,
    audio_chunk_timer: f32,
}

impl<GameStateType: GameStateInterface> AppTicker<GameStateType> {
    fn new() -> Self {
        let window_config = GameStateType::get_window_config();
        get_resources().assets = Some(GameAssets::new("resources"));
        get_resources().input = Some(InputState::new());
        // TODO: let mut input_recorder = InputRecorder::new();
        AppTicker {
            loadingscreen: LoadingScreen::new(
                SPLASHSCREEN_FADEIN_TIME,
                SPLASHSCREEN_FADEOUT_TIME,
                window_config.color_splash_progressbar,
            ),
            game: None,
            audio_chunk_timer: 0.0,
        }
    }
}

impl<GameStateType: GameStateInterface> AppEventHandler for AppTicker<GameStateType> {
    fn get_app_info(&self) -> AppInfo {
        let config = GameStateType::get_game_info();
        AppInfo {
            window_title: config.game_window_title,
            save_folder_name: config.game_save_folder_name,
            company_name: config.game_company_name,
        }
    }

    fn reset(&mut self) {
        todo!()
    }

    fn run_tick(
        &mut self,
        frametime: f32,
        real_world_uptime: f64,
        renderer: &mut Renderer,
        audio_output: &mut AudioOutput,
        out_systemcommands: &mut Vec<AppCommand>,
    ) {
        let resources = get_resources();
        let assets = resources.assets.as_mut().unwrap();
        let input = resources.input.as_mut().unwrap();

        {
            // TODO: Put this into a member function

            // Mouse x in [0, screen_framebuffer_width - 1]  (left to right)
            // Mouse y in [0, screen_framebuffer_height - 1] (top to bottom)
            //
            // NOTE: We get the mouse delta from querying instead of accumulating
            //       events, as it is faster, more accurate and less error-prone
            input.touch.calculate_move_deltas();
            input.mouse.delta_x = input.mouse.pos_x - input.mouse.pos_previous_x;
            input.mouse.delta_y = input.mouse.pos_y - input.mouse.pos_previous_y;

            input.deltatime = snap_deltatime_to_nearest_common_refresh_rate(frametime);
            input.real_world_uptime = real_world_uptime;
        }

        //--------------------------------------------------------------------------------------
        // Start/stop input-recording/-playback

        // TODO
        let mut input_recorder = InputRecorder::<Self>::new();
        if input.keyboard.recently_released(Scancode::O) {
            if !input_recorder.is_playing_back {
                if input_recorder.is_recording {
                    log::info!("Stopping input recording");
                    input_recorder.stop_recording();
                } else {
                    log::info!("Starting input recording");
                    // Clear keyboard input so that we won't get the the `O` Scancode at the
                    // beginning of the recording
                    input.keyboard.clear_transitions();
                    input_recorder.start_recording(self);
                }
            }
        } else if input.keyboard.recently_released(Scancode::P) {
            if !input_recorder.is_recording {
                if input_recorder.is_playing_back {
                    log::info!("Stopping input playback");
                    input_recorder.stop_playback();
                    input.keyboard.clear_state_and_transitions();
                } else {
                    log::info!("Starting input playback");
                    input_recorder.start_playback(self);
                }
            }
        }

        // Playback/record input events
        //
        // NOTE: We can move the playback part before polling events to be more interactive!
        //       For this we need to handle the mouse and keyboard a little different. Maybe we
        //       can have `input_last` and `input_current`?
        if input_recorder.is_recording {
            input_recorder.record_input(&input);
        } else if input_recorder.is_playing_back {
            // NOTE: We need to save the state of the playback-key or the keystate will get
            //       confused. This can happen when we press down the playback-key and hold it for
            //       several frames. While we do that the input playback overwrites the state of the
            //       playback-key. If we release the playback-key the keystate will think it is
            //       already released (due to the overwrite) but will get an additional release
            //       event (which is not good)
            let previous_playback_key_state = input.keyboard.keys[&Scancode::P].clone();
            *input = input_recorder.playback_input(self);
            *input.keyboard.keys.get_mut(&Scancode::P).unwrap() = previous_playback_key_state;
        }

        match assets.update() {
            AssetLoadingStage::SplashStart => return,
            AssetLoadingStage::SplashProgress => return,
            AssetLoadingStage::SplashFinish => {
                let textures_splash = assets.get_atlas_textures().clone();
                let untextured_sprite = assets.get_sprite("untextured").clone();
                let debug_log_font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
                let debug_log_font = assets.get_font(&debug_log_font_name).clone();

                let window_config = GameStateType::get_window_config();
                let mut draw = Drawstate::new(textures_splash, untextured_sprite, debug_log_font);
                game_setup_window(
                    &mut draw,
                    &window_config,
                    input.screen_framebuffer_width,
                    input.screen_framebuffer_height,
                    out_systemcommands,
                );
                draw.set_shaderparams_simple(
                    Color::white(),
                    Mat4::ortho_origin_left_top(
                        window_config.canvas_width as f32,
                        window_config.canvas_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        window_config.canvas_width as f32,
                        window_config.canvas_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        input.screen_framebuffer_width as f32,
                        input.screen_framebuffer_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                );

                assert!(resources.draw.is_none());
                resources.draw = Some(draw);

                self.loadingscreen.start_fading_in();
            }
            AssetLoadingStage::WaitingToStartFilesLoading => {
                if self.loadingscreen.is_faded_in() {
                    assets.start_loading_files();
                }
            }
            AssetLoadingStage::FilesStart => {}
            AssetLoadingStage::FilesProgress => {}
            AssetLoadingStage::FilesFinish => {}
            AssetLoadingStage::DecodingStart => {}
            AssetLoadingStage::DecodingProgress => {}
            AssetLoadingStage::DecodingFinish => {
                assert!(resources.draw.is_some());

                let textures = assets.get_atlas_textures().clone();
                let untextured_sprite = assets.get_sprite("untextured").clone();
                let debug_log_font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
                let debug_log_font = assets.get_font(&debug_log_font_name).clone();

                let draw = resources.draw.as_mut().unwrap();
                draw.assign_textures(textures, untextured_sprite, debug_log_font);

                assert!(resources.audio.is_none());

                if resources.audio.is_none() {
                    let window_config = GameStateType::get_window_config();
                    resources.audio = Some(Audiostate::new(
                        assets.audio.resource_sample_rate_hz,
                        window_config.canvas_width as f32 / 2.0,
                        10_000.0,
                    ));
                }
                let audio = resources.audio.as_mut().unwrap();
                let audio_recordings = assets.get_audiorecordings().clone();
                audio.assign_audio_recordings(audio_recordings);

                assert!(self.game.is_none());
                assert!(resources.globals.is_none());

                let window_config = GameStateType::get_window_config();
                let random = Random::new_from_seed((input.deltatime * 1000000.0) as u64);
                let camera = GameCamera::new(
                    Vec2::zero(),
                    window_config.canvas_width,
                    window_config.canvas_height,
                    false,
                );
                let cursors = Cursors::new(
                    &camera.cam,
                    &input.mouse,
                    &input.touch,
                    input.screen_framebuffer_width,
                    input.screen_framebuffer_height,
                    window_config.canvas_width,
                    window_config.canvas_height,
                );

                let mut globals = Globals {
                    random,
                    camera,
                    cursors,

                    debug_deltatime_speed_factor: 1.0,
                    deltatime_speed_factor: 1.0,
                    deltatime: input.deltatime,
                    is_paused: false,

                    canvas_width: window_config.canvas_width as f32,
                    canvas_height: window_config.canvas_height as f32,
                };

                self.game = Some(GameStateType::new(
                    draw,
                    audio,
                    &assets,
                    &input,
                    &mut globals,
                ));
                resources.globals = Some(globals);

                self.loadingscreen.start_fading_out();
            }
            AssetLoadingStage::Idle => {}
        }

        // Asset hotreloading
        if assets.hotreload_assets() {
            let audio = resources.audio.as_mut().unwrap();
            let draw = resources.draw.as_mut().unwrap();

            let textures = assets.get_atlas_textures().clone();
            let untextured_sprite = assets.get_sprite("untextured").clone();
            let debug_log_font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
            let debug_log_font = assets.get_font(&debug_log_font_name).clone();
            draw.assign_textures(textures, untextured_sprite, debug_log_font);

            let audio_recordings = assets.get_audiorecordings().clone();
            audio.assign_audio_recordings(audio_recordings);
            log::info!("Hotreloaded assets");
        }

        let draw = resources.draw.as_mut().unwrap();

        if input.has_focus || !self.loadingscreen.is_faded_out() {
            draw.begin_frame();

            // Draw loadscreen if necessary
            if !self.loadingscreen.is_faded_out() {
                let (canvas_width, canvas_height) = draw.get_canvas_dimensions().unwrap_or((
                    input.screen_framebuffer_width,
                    input.screen_framebuffer_height,
                ));
                let splash_sprite = assets.get_sprite("splashscreen");
                self.loadingscreen.update_and_draw(
                    draw,
                    input.deltatime,
                    canvas_width,
                    canvas_height,
                    splash_sprite,
                    assets.get_loading_percentage(),
                );
            }

            if let Some(game) = self.game.as_mut() {
                let window_config = GameStateType::get_window_config();
                let globals = resources.globals.as_mut().unwrap();
                globals.cursors = Cursors::new(
                    &globals.camera.cam,
                    &input.mouse,
                    &input.touch,
                    input.screen_framebuffer_width,
                    input.screen_framebuffer_height,
                    window_config.canvas_width,
                    window_config.canvas_height,
                );

                // DEBUG GAMESPEED MANIPULATION
                //
                if input.keyboard.recently_pressed(Scancode::NumpadAdd) {
                    globals.debug_deltatime_speed_factor += 0.1;
                }
                if input.keyboard.recently_pressed(Scancode::NumpadSubtract) {
                    globals.debug_deltatime_speed_factor -= 0.1;
                    if globals.debug_deltatime_speed_factor < 0.1 {
                        globals.debug_deltatime_speed_factor = 0.1;
                    }
                }
                if input
                    .keyboard
                    .recently_pressed_ignore_repeat(Scancode::Space)
                {
                    globals.is_paused = !globals.is_paused;
                }
                let deltatime_speed_factor =
                    globals.deltatime_speed_factor * globals.debug_deltatime_speed_factor;
                let deltatime = if globals.is_paused {
                    if input.keyboard.recently_pressed(Scancode::N) {
                        input.deltatime * deltatime_speed_factor
                    } else {
                        0.0
                    }
                } else {
                    input.deltatime * deltatime_speed_factor
                };
                globals.deltatime = deltatime;

                let audio = resources.audio.as_mut().unwrap();
                audio.set_global_playback_speed_factor(deltatime_speed_factor);
                audio.update_deltatime(deltatime);

                if !is_effectively_zero(globals.debug_deltatime_speed_factor - 1.0) {
                    draw.debug_log(format!("Timefactor: {:.3}", globals.deltatime_speed_factor));
                    draw.debug_log(format!(
                        "Debug Timefactor: {:.1}",
                        globals.debug_deltatime_speed_factor
                    ));
                    draw.debug_log(format!(
                        "Cumulative timefactor: {:.1}",
                        deltatime_speed_factor
                    ));
                }
                draw.debug_log(format!("Deltatime: {:.6}", globals.deltatime));

                gui_begin_frame(draw, &globals.cursors, input);
                game.update(draw, audio, &assets, input, globals, out_systemcommands);
                gui_end_frame(draw);

                let mouse_coords = globals.cursors.mouse;
                game_handle_mouse_camera_zooming_panning(
                    &mut globals.camera,
                    &input.mouse,
                    &mouse_coords,
                );
                globals.camera.update(globals.deltatime);
                game_handle_system_keys(&input.keyboard, out_systemcommands);

                debug_draw_crosshair(
                    draw,
                    &globals.camera.cam,
                    mouse_coords.pos_world,
                    input.screen_framebuffer_width as f32,
                    input.screen_framebuffer_height as f32,
                    2.0,
                    Color::red(),
                    DEPTH_MAX,
                );

                debug_draw_grid(
                    draw,
                    &globals.camera.cam,
                    16.0,
                    input.screen_framebuffer_width as f32,
                    input.screen_framebuffer_height as f32,
                    1,
                    Color::greyscale(0.5),
                    DEPTH_MAX,
                );

                draw.set_shaderparams_simple(
                    Color::white(),
                    globals.camera.proj_view_matrix(),
                    Mat4::ortho_origin_left_top(
                        window_config.canvas_width as f32,
                        window_config.canvas_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        input.screen_framebuffer_width as f32,
                        input.screen_framebuffer_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                );
            }

            if let Some(audio) = resources.audio.as_mut() {
                let globals = resources.globals.as_mut().unwrap();
                let output_sample_rate_hz = audio_output.get_audio_playback_rate_hz();
                audio.set_global_listener_pos(globals.camera.center());

                self.audio_chunk_timer += input.deltatime;

                let mut audiochunk = AudioChunk::new_stereo();
                let audiochunk_length_in_seconds =
                    audiochunk.length_in_seconds(output_sample_rate_hz) as f32;
                let audio_buffersize_in_frames = audio_output.get_audiobuffer_size_in_frames();

                // Render some chunks per frame to keep the load per frame somewhat stable
                while self.audio_chunk_timer >= audiochunk_length_in_seconds {
                    self.audio_chunk_timer -= audiochunk_length_in_seconds;
                    if audio_output.get_num_frames_in_queue() >= 2 * audio_buffersize_in_frames {
                        // We don't want to fill too much or else the latency is gonna be big.
                        // Filling it that much is also a symptom of our deltatime being too much
                        // out of sync with our realtime
                        continue;
                    }
                    audio.render_audio_chunk(&mut audiochunk, output_sample_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }

                // We need to always have a full audiobuffer worth of frames queued up.
                // If our steady submitting of chunks above was not enough we fill up the queue
                while audio_output.get_num_frames_in_queue() < audio_buffersize_in_frames {
                    audio.render_audio_chunk(&mut audiochunk, output_sample_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }
            }

            draw.finish_frame();
        }

        draw.render_frame(renderer);

        {
            // TODO: Put this into a member function

            // Clear input state
            input.screen_framebuffer_dimensions_changed = false;
            input.has_foreground_event = false;
            input.has_focus_event = false;

            input.keyboard.clear_transitions();
            input.mouse.clear_transitions();
            input.touch.clear_transitions();

            if input.textinput.is_textinput_enabled {
                // Reset textinput
                input.textinput.has_new_textinput_event = false;
                input.textinput.has_new_composition_event = false;
                input.textinput.inputtext.clear();
                input.textinput.composition_text.clear();
            }
        }
    }

    fn handle_window_resize(&mut self, new_width: u32, new_height: u32, is_fullscreen: bool) {
        let input = get_input();
        log::debug!(
            "Window resized {}x{} -> {}x{}",
            input.screen_framebuffer_width,
            input.screen_framebuffer_height,
            new_width,
            new_height
        );
        input.screen_framebuffer_width = new_width;
        input.screen_framebuffer_height = new_height;
        input.screen_framebuffer_dimensions_changed = true;
        input.screen_is_fullscreen = is_fullscreen;
    }

    fn handle_window_focus_gained(&mut self) {
        let input = get_input();
        input.has_focus = true;
        input.has_focus_event = true;
        log::debug!("Gained window focus");
    }

    fn handle_window_focus_lost(&mut self) {
        let input = get_input();
        input.has_focus = false;
        input.has_focus_event = true;
        log::debug!("Lost window focus");
    }

    fn handle_key_press(&mut self, scancode: Scancode, keycode: Keycode, is_repeat: bool) {
        let input = get_input();
        input.keyboard.has_press_event = true;
        input.keyboard.has_system_repeat_event |= is_repeat;
        input.keyboard.process_key_press_event(scancode, keycode);
    }

    fn handle_key_release(&mut self, scancode: Scancode, keycode: Keycode) {
        let input = get_input();
        input.keyboard.has_release_event = true;
        input.keyboard.process_key_release_event(scancode, keycode);
    }

    fn handle_mouse_press(&mut self, button: MouseButton, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.mouse.has_press_event = true;
        input.mouse.pos_x = pos_x;
        input.mouse.pos_y = pos_y;
        match button {
            MouseButton::Left => input.mouse.button_left.process_press_event(),
            MouseButton::Right => input.mouse.button_right.process_press_event(),
            MouseButton::Middle => input.mouse.button_middle.process_press_event(),
            MouseButton::X1 => input.mouse.button_x1.process_press_event(),
            MouseButton::X2 => input.mouse.button_x2.process_press_event(),
        }
    }

    fn handle_mouse_release(&mut self, button: MouseButton, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.mouse.has_release_event = true;
        input.mouse.pos_x = pos_x;
        input.mouse.pos_y = pos_y;
        match button {
            MouseButton::Left => input.mouse.button_left.process_release_event(),
            MouseButton::Right => input.mouse.button_right.process_release_event(),
            MouseButton::Middle => input.mouse.button_middle.process_release_event(),
            MouseButton::X1 => input.mouse.button_x1.process_release_event(),
            MouseButton::X2 => input.mouse.button_x2.process_release_event(),
        }
    }

    fn handle_mouse_move(&mut self, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.mouse.has_moved = true;
        input.mouse.pos_x = pos_x;
        input.mouse.pos_y = pos_y;
    }

    fn handle_mouse_wheel_scroll(&mut self, scroll_delta: i32) {
        let input = get_input();
        input.mouse.has_wheel_event = true;
        input.mouse.wheel_delta = scroll_delta;
    }

    fn handle_touch_press(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.touch.process_finger_down(finger_id, pos_x, pos_y);
    }

    fn handle_touch_release(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.touch.process_finger_up(finger_id, pos_x, pos_y);
    }

    fn handle_touch_move(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.touch.process_finger_move(finger_id, pos_x, pos_y);
    }

    fn handle_touch_cancelled(&mut self, finger_id: FingerPlatformId, pos_x: i32, pos_y: i32) {
        let input = get_input();
        input.touch.process_finger_up(finger_id, pos_x, pos_y);
    }
}

pub fn start_mainloop<GameStateType: 'static + GameStateInterface>() {
    let app_context = AppTicker::<GameStateType>::new();
    run_main(app_context);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Convenience functions

/// Convenience function for camera movement with mouse
pub fn game_handle_mouse_camera_zooming_panning(
    camera: &mut GameCamera,
    mouse: &MouseState,
    mouse_coords: &CursorCoords,
) {
    if mouse.button_middle.is_pressed {
        camera.pan(mouse_coords.delta_canvas);
    }
    if mouse.has_wheel_event {
        if mouse.wheel_delta > 0 {
            let new_zoom_level = f32::min(camera.cam.zoom_level * 2.0, 8.0);
            camera.zoom_to_world_point(mouse_coords.pos_world, new_zoom_level);
        } else if mouse.wheel_delta < 0 {
            let new_zoom_level = f32::max(camera.cam.zoom_level / 2.0, 1.0 / 32.0);
            camera.zoom_to_world_point(mouse_coords.pos_world, new_zoom_level);
        }
    }
}

#[derive(Clone, Copy)]
pub struct WindowConfig {
    pub has_canvas: bool,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub canvas_color_letterbox: Color,

    pub windowed_mode_allow: bool,
    pub windowed_mode_allow_resizing: bool,

    pub grab_input: bool,

    pub color_clear: Color,
    pub color_splash_progressbar: Color,
}

pub fn game_setup_window(
    draw: &mut Drawstate,
    config: &WindowConfig,
    screen_resolution_x: u32,
    screen_resolution_y: u32,
    out_systemcommands: &mut Vec<AppCommand>,
) {
    draw.set_clear_color_and_depth(config.color_clear, DEPTH_CLEAR);

    if config.has_canvas {
        draw.update_canvas_dimensions(config.canvas_width, config.canvas_height);
        draw.set_letterbox_color(config.canvas_color_letterbox);

        out_systemcommands.push(AppCommand::WindowedModeAllow(config.windowed_mode_allow));
        if config.windowed_mode_allow {
            out_systemcommands.push(AppCommand::WindowedModeAllowResizing(
                config.windowed_mode_allow_resizing,
            ));

            // NOTE: Pick the biggest window dimension possible which is smaller
            //       than our monitor resolution
            let mut window_width = config.canvas_width;
            let mut window_height = config.canvas_height;
            for factor in 1..100 {
                let width = config.canvas_width * factor;
                let height = config.canvas_height * factor;

                if width >= screen_resolution_x || height >= screen_resolution_y {
                    break;
                }

                window_width = width;
                window_height = height;
            }

            out_systemcommands.push(AppCommand::WindowedModeSetSize {
                width: window_width,
                height: window_height,
                minimum_width: config.canvas_width,
                minimum_height: config.canvas_height,
            });
        }
    }

    if config.grab_input {
        out_systemcommands.push(AppCommand::ScreenSetGrabInput(config.grab_input));
    }
}

pub fn game_handle_system_keys(keyboard: &KeyboardState, out_systemcommands: &mut Vec<AppCommand>) {
    if keyboard.recently_pressed(Scancode::Escape) {
        out_systemcommands.push(AppCommand::Shutdown);
    }
    if keyboard.recently_pressed(Scancode::Enter)
        && (keyboard.is_down(Scancode::AltLeft) || keyboard.is_down(Scancode::AltRight))
    {
        out_systemcommands.push(AppCommand::FullscreenToggle);
    }
    if keyboard.recently_pressed(Scancode::F8) {
        out_systemcommands.push(AppCommand::Restart);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Live looped input playback and recording

#[derive(Default)]
struct InputRecorder<AppEventHandlerType: AppEventHandler> {
    app: Option<AppEventHandlerType>,

    is_recording: bool,
    is_playing_back: bool,
    queue_playback: VecDeque<InputState>,
    queue_recording: VecDeque<InputState>,
}

impl<AppEventHandlerType: AppEventHandler> InputRecorder<AppEventHandlerType> {
    fn new() -> InputRecorder<AppEventHandlerType> {
        InputRecorder {
            app: None,
            is_recording: false,
            is_playing_back: false,
            queue_playback: VecDeque::new(),
            queue_recording: VecDeque::new(),
        }
    }

    fn start_recording(&mut self, app_context: &AppEventHandlerType) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = true;
        self.queue_recording.clear();
        self.app = Some(app_context.clone());
    }

    fn stop_recording(&mut self) {
        assert!(self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = false;
    }

    fn start_playback(&mut self, app_context: &mut AppEventHandlerType) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_playing_back = true;
        self.queue_playback = self.queue_recording.clone();
        *app_context = self
            .app
            .as_ref()
            .expect("Recording is missing app context")
            .clone();

        assert!(!self.queue_playback.is_empty());
    }

    fn stop_playback(&mut self) {
        assert!(!self.is_recording);
        assert!(self.is_playing_back);

        self.is_playing_back = false;
        self.queue_playback.clear();
    }

    fn record_input(&mut self, input: &InputState) {
        assert!(self.is_recording);
        assert!(!self.is_playing_back);

        self.queue_recording.push_back(input.clone());
    }

    fn playback_input(&mut self, app_context: &mut AppEventHandlerType) -> InputState {
        assert!(!self.is_recording);
        assert!(self.is_playing_back);

        if let Some(input) = self.queue_playback.pop_front() {
            input
        } else {
            // We hit the end of the stream -> go back to the beginning
            self.stop_playback();
            self.start_playback(app_context);

            // As we could not read the input before we try again
            self.queue_playback.pop_front().unwrap()
        }
    }
}
