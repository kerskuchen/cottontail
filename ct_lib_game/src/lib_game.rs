pub mod easyapi;
pub use easyapi::*;

pub mod animations_fx;
pub use animations_fx::*;

pub mod assets;
pub use assets::*;

pub mod camera;
pub use camera::*;

pub mod choreographer;
pub use choreographer::*;

pub mod debug;
pub use debug::*;

pub mod gui;
pub use gui::*;

mod input;
use input::{InputState, MouseState, TouchState};

use ct_lib_audio::*;
use ct_lib_core::serde_derive::{Deserialize, Serialize};
use ct_lib_core::*;
use ct_lib_draw::*;
use ct_lib_image::*;
use ct_lib_math::*;
use ct_lib_window::input::*;
use ct_lib_window::*;

use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
};

pub const DEPTH_DEBUG: Depth = 90.0;
pub const DEPTH_DEVELOP_OVERLAY: Depth = 80.0;
pub const DEPTH_SPLASH: Depth = 70.0;
pub const DEPTH_SCREEN_FADER: Depth = 60.0;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Gamestate

#[derive(Clone)]
pub struct Globals {
    pub random: Random,
    pub camera: GameCamera,
    pub cursors: Cursors,

    pub deltatime: f32,
    pub deltatime_without_speedup: f32,

    pub deltatime_speed_factor_user: f32,
    pub deltatime_speed_factor_debug: f32,

    pub time_since_startup: f64,
    pub is_paused: bool,

    pub canvas_width: f32,
    pub canvas_height: f32,
}

pub struct GameInfo {
    pub game_window_title: String,
    pub game_save_folder_name: String,
    pub game_company_name: String,
}

pub trait AppStateInterface: Clone {
    fn get_game_info() -> GameInfo;
    fn get_window_preferences() -> WindowPreferences;
    fn new() -> Self;
    fn update(&mut self);
}

const SPLASHSCREEN_FADEIN_TIME: f32 = 0.3;
const SPLASHSCREEN_FADEOUT_TIME: f32 = 0.5;

struct AppResources {
    pub assets: GameAssets,
    pub input: InputState,
    /// NOTE: This depends on drawstate to be available
    pub gui: GuiState,
    /// NOTE: This depends on drawstate to be available
    pub debug_draw_logger: DebugDrawLogState,
    /// NOTE: This depends on graphics assets to be available
    pub draw: Drawstate,
    /// NOTE: This depends on audio assets to be available
    pub audio: Audiostate,

    pub globals: Option<Globals>,
}

static mut APP_RESOURCES: Option<AppResources> = None;

#[inline(always)]
fn get_resources() -> &'static mut AppResources {
    unsafe {
        match APP_RESOURCES.as_mut() {
            Some(resources) => resources,
            None => {
                APP_RESOURCES = Some(AppResources {
                    assets: GameAssets::new("resources"),
                    input: InputState::new(),
                    gui: GuiState::new(),
                    debug_draw_logger: DebugDrawLogState::new(),
                    draw: Drawstate::new(),
                    audio: Audiostate::new(),

                    globals: None,
                });

                APP_RESOURCES.as_mut().unwrap()
            }
        }
    }
}

#[inline(always)]
fn get_assets() -> &'static mut GameAssets {
    &mut get_resources().assets
}
#[inline(always)]
fn get_input() -> &'static mut InputState {
    &mut get_resources().input
}
#[inline(always)]
fn get_gui() -> &'static mut GuiState {
    &mut get_resources().gui
}
#[inline(always)]
fn get_globals() -> &'static mut Globals {
    get_resources().globals.as_mut().unwrap()
}
#[inline(always)]
fn get_draw() -> &'static mut Drawstate {
    &mut get_resources().draw
}
#[inline(always)]
fn get_audio() -> &'static mut Audiostate {
    &mut get_resources().audio
}
#[inline(always)]
fn get_debug_draw_logger() -> &'static mut DebugDrawLogState {
    &mut get_resources().debug_draw_logger
}

pub struct AppTicker<AppStateType: AppStateInterface> {
    game: Option<AppStateType>,
    loadingscreen: LoadingScreen,
    audio_chunk_timer: f32,
    debug_input_recorder: InputRecorder<AppStateType>,
}

impl<AppStateType: AppStateInterface> AppTicker<AppStateType> {
    fn new() -> Self {
        let window_config = AppStateType::get_window_preferences();
        AppTicker {
            loadingscreen: LoadingScreen::new(
                SPLASHSCREEN_FADEIN_TIME,
                SPLASHSCREEN_FADEOUT_TIME,
                window_config.color_splash_progressbar,
            ),
            game: None,
            audio_chunk_timer: 0.0,
            debug_input_recorder: InputRecorder::new(),
        }
    }
}

impl<GameStateType: AppStateInterface> AppEventHandler for AppTicker<GameStateType> {
    fn get_app_info(&self) -> AppInfo {
        let config = GameStateType::get_game_info();
        AppInfo {
            window_title: config.game_window_title,
            save_folder_name: config.game_save_folder_name,
            company_name: config.game_company_name,
        }
    }

    fn reset(&mut self) {
        if let Some(game) = self.game.as_mut() {
            get_audio().reset();
            *game = GameStateType::new();
        }
    }

    fn run_tick(
        &mut self,
        time_since_last_frame: f32,
        time_since_startup: f64,
        renderer: &mut Renderer,
        audio_output: &mut AudioOutput,
    ) {
        get_input().begin_frame(
            snap_deltatime_to_nearest_common_refresh_rate(time_since_last_frame),
            time_since_startup,
        );

        if let Some(game) = self.game.as_mut() {
            self.debug_input_recorder.tick(game);
        }

        match get_assets().update() {
            AssetLoadingStage::SplashStart => return,
            AssetLoadingStage::SplashProgress => return,
            AssetLoadingStage::SplashFinish => {
                let textures_splash = get_assets().get_atlas_textures().clone();
                get_draw().assign_textures(textures_splash);

                let window_preferences = GameStateType::get_window_preferences();
                game_setup_window(
                    get_draw(),
                    &window_preferences,
                    window_screen_width(),
                    window_screen_height(),
                );
                get_draw().set_shaderparams_default(
                    Color::white(),
                    Mat4::ortho_origin_left_top(
                        window_preferences.canvas_width as f32,
                        window_preferences.canvas_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        window_preferences.canvas_width as f32,
                        window_preferences.canvas_height as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        window_screen_width() as f32,
                        window_screen_height() as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                );

                self.loadingscreen.start_fading_in();
            }
            AssetLoadingStage::WaitingToStartFilesLoading => {
                if self.loadingscreen.is_faded_in() {
                    get_assets().start_loading_files();
                }
            }
            AssetLoadingStage::FilesStart => {}
            AssetLoadingStage::FilesProgress => {}
            AssetLoadingStage::FilesFinish => {}
            AssetLoadingStage::DecodingStart => {}
            AssetLoadingStage::DecodingProgress => {}
            AssetLoadingStage::DecodingFinish => {
                let textures = get_assets().get_atlas_textures().clone();
                get_draw().assign_textures(textures);

                let audio_recordings = get_assets().get_audiorecordings().clone();
                get_audio().assign_audio_recordings(audio_recordings);

                let (canvas_width, canvas_height) = get_draw()
                    .get_canvas_dimensions()
                    .expect("No canvas dimensions found");
                {
                    assert!(get_resources().globals.is_none());
                    let random = Random::new_from_seed((time_since_startup * 1000000000.0) as u64);
                    let camera = GameCamera::new(Vec2::zero(), canvas_width, canvas_height, false);
                    let cursors = {
                        let input = get_input();
                        Cursors::new(
                            &camera.cam,
                            &input.mouse,
                            &input.touch,
                            input.screen_framebuffer_width,
                            input.screen_framebuffer_height,
                        )
                    };
                    get_resources().globals = Some(Globals {
                        random,
                        camera,
                        cursors,

                        deltatime: get_input().deltatime,
                        deltatime_without_speedup: get_input().deltatime,
                        deltatime_speed_factor_user: 1.0,
                        deltatime_speed_factor_debug: 1.0,
                        time_since_startup,

                        is_paused: false,
                        canvas_width: canvas_width as f32,
                        canvas_height: canvas_height as f32,
                    });
                }

                assert!(self.game.is_none());
                self.game = Some(GameStateType::new());

                self.loadingscreen.start_fading_out();
            }
            AssetLoadingStage::Idle => {}
        }

        // Asset hotreloading
        if get_assets().hotreload_assets() {
            let draw = get_draw();
            let textures = get_assets().get_atlas_textures().clone();
            draw.assign_textures(textures);

            let audio = get_audio();
            let audio_recordings = get_assets().get_audiorecordings().clone();
            audio.assign_audio_recordings(audio_recordings);

            log::info!("Hotreloaded assets");
        }

        if window_has_focus() || !self.loadingscreen.is_faded_out() {
            get_draw().begin_frame();
            get_debug_draw_logger().begin_frame();

            // Draw loadscreen if necessary
            if !self.loadingscreen.is_faded_out() {
                let (canvas_width, canvas_height) = get_draw()
                    .get_canvas_dimensions()
                    .unwrap_or((window_screen_width(), window_screen_height()));
                let splash_sprite = get_assets().get_sprite("splashscreen");
                self.loadingscreen.update_and_draw(
                    get_input().deltatime,
                    canvas_width,
                    canvas_height,
                    splash_sprite,
                    get_assets().get_loading_percentage(),
                );
            }

            if let Some(game) = self.game.as_mut() {
                get_globals().cursors = {
                    Cursors::new(
                        &get_camera().cam,
                        &get_input().mouse,
                        &get_input().touch,
                        window_screen_width(),
                        window_screen_height(),
                    )
                };

                // DEBUG GAMESPEED MANIPULATION
                //
                {
                    let globals = get_globals();

                    if key_recently_pressed(Scancode::NumpadAdd) {
                        globals.deltatime_speed_factor_debug += 0.1;
                    }
                    if key_recently_pressed(Scancode::NumpadSubtract) {
                        globals.deltatime_speed_factor_debug -= 0.1;
                        if globals.deltatime_speed_factor_debug < 0.1 {
                            globals.deltatime_speed_factor_debug = 0.1;
                        }
                    }
                    if key_recently_pressed_ignore_repeat(Scancode::Space) {
                        globals.is_paused = !globals.is_paused;
                    }
                    let deltatime_speed_factor =
                        globals.deltatime_speed_factor_user * globals.deltatime_speed_factor_debug;
                    let final_deltatime = if globals.is_paused {
                        if key_recently_pressed(Scancode::N) {
                            get_input().deltatime * deltatime_speed_factor
                        } else {
                            0.0
                        }
                    } else {
                        get_input().deltatime * deltatime_speed_factor
                    };

                    globals.deltatime = final_deltatime;
                    globals.deltatime_without_speedup = get_input().deltatime;
                    globals.time_since_startup = time_since_startup;

                    if !is_effectively_zero(globals.deltatime_speed_factor_debug - 1.0) {
                        draw_debug_log(format!(
                            "Timefactor: {:.3}",
                            globals.deltatime_speed_factor_user
                        ));
                        draw_debug_log(format!(
                            "Debug Timefactor: {:.1}",
                            globals.deltatime_speed_factor_debug
                        ));
                        draw_debug_log(format!(
                            "Cumulative timefactor: {:.1}",
                            deltatime_speed_factor
                        ));
                    }
                    draw_debug_log(format!("Deltatime: {:.6}", globals.deltatime));
                }

                get_audio().set_global_playback_speed_factor(time_deltatime_speed_factor_total());
                get_audio().update_deltatime(time_deltatime());

                gui_begin_frame();
                game.update();
                gui_end_frame();

                game_handle_system_keys();
                game_handle_mouse_camera_zooming_panning();
                get_camera().update(time_deltatime());

                get_draw().set_shaderparams_default(
                    Color::white(),
                    get_camera().proj_view_matrix(),
                    Mat4::ortho_origin_left_top(
                        canvas_width(),
                        canvas_height(),
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                    Mat4::ortho_origin_left_top(
                        window_screen_width() as f32,
                        window_screen_height() as f32,
                        DEFAULT_WORLD_ZNEAR,
                        DEFAULT_WORLD_ZFAR,
                    ),
                );
            }

            if self.game.is_some() {
                let output_sample_rate_hz = audio_output.get_audio_playback_rate_hz();
                get_audio().set_global_spatial_params(AudioGlobalSpatialParams {
                    listener_pos: get_camera().center(),
                    listener_vel: get_camera().velocity(),
                    doppler_effect_medium_velocity_abs_max: 10_000.0,
                    distance_for_max_pan: get_camera().cam.dim_frustum.x / 2.0,
                });

                self.audio_chunk_timer += get_input().deltatime;

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
                    get_audio().render_audio_chunk(&mut audiochunk, output_sample_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }

                // We need to always have a full audiobuffer worth of frames queued up.
                // If our steady submitting of chunks above was not enough we fill up the queue
                while audio_output.get_num_frames_in_queue() < audio_buffersize_in_frames {
                    get_audio().render_audio_chunk(&mut audiochunk, output_sample_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }
            }

            get_draw().finish_frame();
        }

        get_draw().render_frame(renderer);
        get_input().end_frame();
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
        input.mouse.has_move_event = true;
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

    fn handle_gamepad_connected(&mut self, gamepad_id: GamepadPlatformId) {
        if gamepad_id != 0 {
            // TODO: Currently we only support one gamepad
            return;
        }
        let input = get_input();
        input.gamepad.is_connected = true;
    }

    fn handle_gamepad_disconnected(&mut self, gamepad_id: GamepadPlatformId) {
        if gamepad_id != 0 {
            // TODO: Currently we only support one gamepad
            return;
        }
        let input = get_input();
        input.gamepad.is_connected = false;
    }

    fn handle_gamepad_new_state(
        &mut self,
        gamepad_id: GamepadPlatformId,
        state: &GamepadPlatformState,
    ) {
        if gamepad_id != 0 {
            // TODO: Currently we only support one gamepad
            return;
        }

        let input = get_input();
        for (&button_name, &is_pressed) in state.buttons.iter() {
            input.gamepad.process_button_state(button_name, is_pressed);
        }
        for (&axis_name, &value) in state.axes.iter() {
            input.gamepad.process_axis_state(axis_name, value);
        }
    }
}

pub fn start_mainloop<GameStateType: 'static + AppStateInterface>() {
    let app_context = AppTicker::<GameStateType>::new();
    run_main(app_context);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Convenience functions

/// Convenience function for camera movement with mouse
pub fn game_handle_mouse_camera_zooming_panning() {
    let camera = get_camera();
    if mouse_is_down_middle() {
        camera.pan(mouse_delta_canvas());
    }
    if mouse_wheel_event_happened() {
        if mouse_wheel_delta() > 0 {
            let new_zoom_level = f32::min(camera.cam.zoom_level * 2.0, 8.0);
            camera.zoom_to_world_point(mouse_pos_world(), new_zoom_level);
        } else if mouse_wheel_delta() < 0 {
            let new_zoom_level = f32::max(camera.cam.zoom_level / 2.0, 1.0 / 32.0);
            camera.zoom_to_world_point(mouse_pos_world(), new_zoom_level);
        }
    }
}

#[derive(Clone, Copy)]
pub struct WindowPreferences {
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
    config: &WindowPreferences,
    screen_resolution_x: u32,
    screen_resolution_y: u32,
) {
    draw.set_clear_color_and_depth(config.color_clear, DEPTH_CLEAR);

    if config.has_canvas {
        draw.update_canvas_dimensions(config.canvas_width, config.canvas_height);
        draw.set_letterbox_color(config.canvas_color_letterbox);

        platform_window_set_allow_windowed_mode(config.windowed_mode_allow);
        if config.windowed_mode_allow {
            platform_window_set_allow_window_resizing_by_user(config.windowed_mode_allow_resizing);

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

            platform_window_set_window_size(
                window_width,
                window_height,
                config.canvas_width,
                config.canvas_height,
            );
        }
    }

    if config.grab_input {
        platform_window_set_cursor_grabbing(config.grab_input);
    }
}

pub fn game_handle_system_keys() {
    if key_recently_pressed(Scancode::F5) {
        platform_window_restart();
    }
    if key_recently_pressed(Scancode::Escape) {
        platform_window_shutdown();
    }
    if key_recently_pressed(Scancode::Enter) && key_is_down_modifier(KeyModifier::Alt) {
        platform_window_toggle_fullscreen();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Live looped input playback and recording

struct InputRecorder<AppStateType: AppStateInterface> {
    appstate: Option<AppStateType>,
    draw: Option<Drawstate>,
    audio: Option<Audiostate>,
    globals: Option<Globals>,

    is_recording: bool,
    is_playing_back: bool,
    queue_playback: VecDeque<InputState>,
    queue_recording: VecDeque<InputState>,
}

impl<AppStateType: AppStateInterface> InputRecorder<AppStateType> {
    fn new() -> InputRecorder<AppStateType> {
        InputRecorder {
            appstate: None,
            draw: None,
            audio: None,
            globals: None,

            is_recording: false,
            is_playing_back: false,
            queue_playback: VecDeque::new(),
            queue_recording: VecDeque::new(),
        }
    }

    fn tick(&mut self, game: &mut AppStateType) {
        let draw = get_draw();
        let audio = get_audio();
        let globals = get_globals();

        if key_recently_released(Scancode::O) {
            if !self.is_playing_back {
                if self.is_recording {
                    log::info!("Stopping input recording");
                    self.stop_recording();
                } else {
                    log::info!("Starting input recording");
                    // Clear keyboard input so that we won't get the the `O` Scancode at the
                    // beginning of the recording
                    get_input().keyboard.clear_transitions();
                    self.start_recording(game, draw, audio, globals);
                }
            }
        } else if key_recently_released(Scancode::P) {
            if !self.is_recording {
                if self.is_playing_back {
                    log::info!("Stopping input playback");
                    self.stop_playback();
                    get_input().keyboard.clear_state_and_transitions();
                } else {
                    log::info!("Starting input playback");
                    self.start_playback(game, draw, audio, globals);
                }
            }
        }

        // Playback/record input events
        //
        // NOTE: We can move the playback part before polling events to be more interactive!
        //       For this we need to handle the mouse and keyboard a little different. Maybe we
        //       can have `input_last` and `input_current`?
        if self.is_recording {
            self.record_input(get_input());
        } else if self.is_playing_back {
            // NOTE: We need to save the state of the playback-key or the keystate will get
            //       confused. This can happen when we press down the playback-key and hold it for
            //       several frames. While we do that the input playback overwrites the state of the
            //       playback-key. If we release the playback-key the keystate will think it is
            //       already released (due to the overwrite) but will get an additional release
            //       event (which is not good)
            let input = get_input();
            if let Some(previous_playback_key_state) =
                input.keyboard.keys.get(&Scancode::P).cloned()
            {
                *input = self.playback_input(game, draw, audio, globals);
                input
                    .keyboard
                    .keys
                    .insert(Scancode::P, previous_playback_key_state);
            } else {
                *input = self.playback_input(game, draw, audio, globals);
            }
        }
    }

    fn start_recording(
        &mut self,
        appstate: &AppStateType,
        draw: &Drawstate,
        audio: &Audiostate,
        globals: &Globals,
    ) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = true;
        self.queue_recording.clear();
        self.appstate = Some(appstate.clone());
        self.draw = Some(draw.clone());
        self.audio = Some(audio.clone());
        self.globals = Some(globals.clone());
    }

    fn stop_recording(&mut self) {
        assert!(self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = false;
    }

    fn start_playback(
        &mut self,
        appstate: &mut AppStateType,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        globals: &mut Globals,
    ) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_playing_back = true;
        self.queue_playback = self.queue_recording.clone();
        *appstate = self
            .appstate
            .as_ref()
            .expect("Recording is missing app context")
            .clone();
        *draw = self
            .draw
            .as_ref()
            .expect("Recording is missing draw context")
            .clone();
        *audio = self
            .audio
            .as_ref()
            .expect("Recording is missing audio context")
            .clone();
        *globals = self
            .globals
            .as_ref()
            .expect("Recording is missing globals context")
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

    fn playback_input(
        &mut self,
        appstate: &mut AppStateType,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        globals: &mut Globals,
    ) -> InputState {
        assert!(!self.is_recording);
        assert!(self.is_playing_back);

        if let Some(input) = self.queue_playback.pop_front() {
            input
        } else {
            // We hit the end of the stream -> go back to the beginning
            self.stop_playback();
            self.start_playback(appstate, draw, audio, globals);

            // As we could not read the input before we try again
            self.queue_playback.pop_front().unwrap()
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Cursors

#[derive(Debug, Default, Clone, Copy)]
pub struct CursorCoords {
    pub pos_screen: Canvaspoint,
    pub pos_canvas: Canvaspoint,
    pub pos_world: Worldpoint,

    pub delta_screen: Canvasvec,
    pub delta_canvas: Canvasvec,
    pub delta_world: Worldvec,
}

impl CursorCoords {
    fn new(
        camera: &Camera,
        screen_width: u32,
        screen_height: u32,
        screen_cursor_pos_x: i32,
        screen_cursor_pos_y: i32,
        screen_cursor_delta_x: i32,
        screen_cursor_delta_y: i32,
    ) -> CursorCoords {
        let screen_cursor_pos_previous_x = screen_cursor_pos_x - screen_cursor_delta_x;
        let screen_cursor_pos_previous_y = screen_cursor_pos_y - screen_cursor_delta_y;

        let canvas_pos = screen_point_to_canvas_point(
            screen_cursor_pos_x,
            screen_cursor_pos_y,
            screen_width,
            screen_height,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );
        let canvas_pos_previous = screen_point_to_canvas_point(
            screen_cursor_pos_previous_x,
            screen_cursor_pos_previous_y,
            screen_width,
            screen_height,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );

        // NOTE: We don't transform the screen cursor delta directly because that leads to rounding
        //       errors that can accumulate. For example if we have a small canvas and big screen we can
        //       move the cursor slowly such that the delta keeps being (0,0) but the canvas position
        //       changes
        let canvas_delta = canvas_pos - canvas_pos_previous;

        CursorCoords {
            pos_screen: Vec2::new(screen_cursor_pos_x as f32, screen_cursor_pos_y as f32),
            pos_canvas: Vec2::from(canvas_pos),
            pos_world: camera.canvaspoint_to_worldpoint(Vec2::from(canvas_pos)),

            delta_screen: Vec2::new(screen_cursor_delta_x as f32, screen_cursor_delta_y as f32),
            delta_canvas: Vec2::from(canvas_delta),
            delta_world: camera.canvas_vec_to_world_vec(Vec2::from(canvas_delta)),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Cursors {
    pub mouse: CursorCoords,
    pub fingers: HashMap<FingerId, CursorCoords>,
}

impl Cursors {
    pub fn new(
        camera: &Camera,
        mouse: &MouseState,
        touch: &TouchState,
        screen_width: u32,
        screen_height: u32,
    ) -> Cursors {
        let mouse = CursorCoords::new(
            camera,
            screen_width,
            screen_height,
            mouse.pos_x,
            mouse.pos_y,
            mouse.pos_x - mouse.pos_previous_x,
            mouse.pos_y - mouse.pos_previous_y,
        );
        let fingers = touch
            .fingers
            .iter()
            .map(|(id, finger)| {
                (
                    *id,
                    CursorCoords::new(
                        camera,
                        screen_width,
                        screen_height,
                        finger.pos_x,
                        finger.pos_y,
                        finger.pos_x - finger.pos_previous_x,
                        finger.pos_y - finger.pos_previous_y,
                    ),
                )
            })
            .collect();

        Cursors { mouse, fingers }
    }
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
