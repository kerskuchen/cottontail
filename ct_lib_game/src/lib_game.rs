pub mod assets;

pub use assets::*;

use ct_lib_audio as audio;
use ct_lib_core as core;
use ct_lib_draw as draw;
use ct_lib_image as image;
use ct_lib_math as math;
use ct_lib_window as window;

use audio::*;
use draw::*;
use image::*;
use math::*;
use window::{
    input::*, platform::audio::AudioOutput, renderer_opengl::Renderer, AppCommand,
    AppContextInterface, AppInfo,
};

use crate::core::serde_derive::{Deserialize, Serialize};
use crate::core::*;

use std::collections::HashMap;

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
    fn get_game_config() -> GameInfo;
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

const SPLASHSCREEN_FADEIN_TIME: f32 = 0.5;
const SPLASHSCREEN_SUSTAIN_TIME: f32 = 0.5;
const SPLASHSCREEN_FADEOUT_TIME: f32 = 0.5;

#[derive(Clone)]
pub struct AppContext<GameStateType: GameStateInterface> {
    pub assets: GameAssets,
    pub game: Option<GameStateType>,
    pub draw: Option<Drawstate>,
    pub audio: Option<Audiostate>,
    pub splashscreen: Option<SplashScreen>,
    pub globals: Option<Globals>,

    audio_chunk_timer: f32,
}

impl<GameStateType: GameStateInterface> Default for AppContext<GameStateType> {
    fn default() -> Self {
        AppContext {
            assets: GameAssets::new("resources"),
            game: None,
            draw: None,
            audio: None,
            splashscreen: None,
            globals: None,
            audio_chunk_timer: 0.0,
        }
    }
}

impl<GameStateType: GameStateInterface + Clone> AppContextInterface for AppContext<GameStateType> {
    fn get_app_info() -> window::AppInfo {
        let config = GameStateType::get_game_config();
        AppInfo {
            window_title: config.game_window_title,
            save_folder_name: config.game_save_folder_name,
            company_name: config.game_company_name,
        }
    }

    fn new(renderer: &mut Renderer, input: &InputState, audio: &mut AudioOutput) -> Self {
        let TODO = "we can get rid of all the optional fields here";
        Self::default()
    }

    fn reset(&mut self) {
        todo!()
    }

    fn run_tick(
        &mut self,
        renderer: &mut Renderer,
        input: &InputState,
        audio_output: &mut AudioOutput,
        out_systemcommands: &mut Vec<AppCommand>,
    ) {
        if !self.assets.load_graphics() {
            return;
        }

        if input.has_focus {
            if self.draw.is_none() {
                let textures = self.assets.get_atlas_textures().to_vec();
                let untextured_sprite = self.assets.get_sprite("untextured").clone();
                let debug_log_font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
                let debug_log_font = self.assets.get_font(&debug_log_font_name).clone();

                let window_config = GameStateType::get_window_config();
                let mut draw = Drawstate::new(textures, untextured_sprite, debug_log_font);
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
                    Vec2::zero(),
                );
                self.draw = Some(draw);
            }
            let draw = self.draw.as_mut().unwrap();

            draw.begin_frame();

            if self.splashscreen.is_none() {
                let splash_sprite = self.assets.get_sprite("splash").clone();
                self.splashscreen = Some(SplashScreen::new(
                    splash_sprite,
                    SPLASHSCREEN_FADEIN_TIME,
                    SPLASHSCREEN_FADEOUT_TIME,
                    SPLASHSCREEN_SUSTAIN_TIME,
                ));
            }

            let splashscreen = self
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
            match splashscreen.update_and_draw(draw, input.deltatime, canvas_width, canvas_height) {
                SplashscreenState::StartedFadingIn => {}
                SplashscreenState::IsFadingIn => {}
                SplashscreenState::FinishedFadingIn => {
                    assert!(self.audio.is_none());

                    let audio_recordings = self.assets.load_audiorecordings();
                    if self.audio.is_none() {
                        let window_config = GameStateType::get_window_config();
                        self.audio = Some(Audiostate::new(
                            self.assets.audio.resource_sample_rate_hz,
                            window_config.canvas_width as f32 / 2.0,
                            10_000.0,
                        ));
                    }
                    let audio = self.audio.as_mut().unwrap();
                    audio.add_audio_recordings(audio_recordings);

                    assert!(self.game.is_none());
                    assert!(self.globals.is_none());

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
                        &self.assets,
                        &input,
                        &mut globals,
                    ));
                    self.globals = Some(globals);
                }

                SplashscreenState::Sustain => {}
                SplashscreenState::StartedFadingOut => {}
                SplashscreenState::IsFadingOut => {}
                SplashscreenState::FinishedFadingOut => {}
                SplashscreenState::IsDone => {}
            }

            if let Some(game) = self.game.as_mut() {
                let window_config = GameStateType::get_window_config();
                let globals = self.globals.as_mut().unwrap();
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
                if input
                    .keyboard
                    .recently_pressed_or_repeated(Scancode::NumpadAdd)
                {
                    globals.debug_deltatime_speed_factor += 0.1;
                }
                if input
                    .keyboard
                    .recently_pressed_or_repeated(Scancode::NumpadSubtract)
                {
                    globals.debug_deltatime_speed_factor -= 0.1;
                    if globals.debug_deltatime_speed_factor < 0.1 {
                        globals.debug_deltatime_speed_factor = 0.1;
                    }
                }
                if input.keyboard.recently_pressed(Scancode::Space) {
                    globals.is_paused = !globals.is_paused;
                }
                let deltatime_speed_factor =
                    globals.deltatime_speed_factor * globals.debug_deltatime_speed_factor;
                let deltatime = if globals.is_paused {
                    if input.keyboard.recently_pressed_or_repeated(Scancode::N) {
                        input.deltatime * deltatime_speed_factor
                    } else {
                        0.0
                    }
                } else {
                    input.deltatime * deltatime_speed_factor
                };
                globals.deltatime = deltatime;

                let audio = self.audio.as_mut().unwrap();
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

                game.update(
                    draw,
                    audio,
                    &self.assets,
                    input,
                    globals,
                    out_systemcommands,
                );
                game_handle_system_keys(&input.keyboard, out_systemcommands);

                globals.camera.update(globals.deltatime);
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
                    globals.camera.canvas_blit_offset(),
                );
            }

            if let Some(audio) = self.audio.as_mut() {
                self.audio_chunk_timer += input.deltatime;

                let mut audiochunk = AudioChunk::new_stereo();
                let audiochunk_length_in_seconds =
                    audiochunk.length_in_seconds(input.audio_playback_rate_hz) as f32;
                let audio_buffersize_in_frames = audio_output.get_audiobuffer_size_in_frames();

                // Render some chunks per frame to keep the load per frame somewhat stable
                while self.audio_chunk_timer >= audiochunk_length_in_seconds {
                    self.audio_chunk_timer -= audiochunk_length_in_seconds;
                    if audio_output.get_num_frames_in_queue() >= 2 * audio_buffersize_in_frames {
                        // We don't want to fill too much or else the latency is gonna be big.
                        // Filling it that much is also a symptom of our deltatime being too much
                        // out of sync with our realtime
                        log::warn!("Too many audiochunks queued up");
                        continue;
                    }
                    audio.render_audio_chunk(&mut audiochunk, input.audio_playback_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }

                // We need to always have a full audiobuffer worth of frames queued up.
                // If our steady submitting of chunks above was not enough we fill up the queue
                while audio_output.get_num_frames_in_queue() < audio_buffersize_in_frames {
                    audio.render_audio_chunk(&mut audiochunk, input.audio_playback_rate_hz);
                    let (samples_left, samples_right) = audiochunk.get_stereo_samples();
                    audio_output.submit_frames(samples_left, samples_right);
                    audiochunk.reset();
                }
            }

            draw.finish_frame();
        }

        if let Some(draw) = self.draw.as_mut() {
            draw.render_frame(renderer);
        }
    }
}

pub fn start_mainloop<GameStateType: 'static + GameStateInterface>() {
    window::run_main::<AppContext<GameStateType>>();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Camera and coordinates

/// NOTE: Camera pos equals is the top-left of the screen or the center depending on the
/// `is_centered` flag. It has the following bounds in world coordinates:
/// non-centered: [pos.x, pos.x + dim_frustum.w] x [pos.y, pos.y + dim_frustum.h]
/// centered:     [pos.x - 0.5*dim_frustum.w, pos.x + 0.5*dim_frustum.w] x
///               [pos.y - 0.5*dim_frustum.h, pos.y + 0.5*dim_frustum.h]
///
/// with:
/// dim_frustum = dim_canvas / zoom
/// zoom > 1.0 -> zooming in
/// zoom < 1.0 -> zooming out
#[derive(Clone, Default)]
pub struct Camera {
    pub pos: Vec2,
    pub pos_pixelsnapped: Vec2,
    pub dim_frustum: Vec2,
    pub dim_canvas: Vec2,
    pub zoom_level: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub is_centered: bool,
}

// Coordinates
//
impl Camera {
    /// Converts a CanvasPoint to a Worldpoint
    #[inline]
    pub fn canvaspoint_to_worldpoint(&self, canvaspoint: Canvaspoint) -> Worldpoint {
        if self.is_centered {
            (canvaspoint - 0.5 * self.dim_canvas) / self.zoom_level + self.pos_pixelsnapped
        } else {
            (canvaspoint / self.zoom_level) + self.pos_pixelsnapped
        }
    }

    /// Converts a Worldpoint to a CanvasPoint
    #[inline]
    pub fn worldpoint_to_canvaspoint(&self, worldpoint: Worldpoint) -> Canvaspoint {
        if self.is_centered {
            (worldpoint - self.pos_pixelsnapped) * self.zoom_level + 0.5 * self.dim_canvas
        } else {
            (worldpoint - self.pos_pixelsnapped) * self.zoom_level
        }
    }

    /// Converts a Canvasvec to a Worldvec
    #[inline]
    pub fn canvas_vec_to_world_vec(&self, canvasvec: Canvasvec) -> Worldvec {
        canvasvec / self.zoom_level
    }

    /// Converts a Worldvec to a Canvasvec
    #[inline]
    pub fn world_vec_to_canvas_vec(&self, worldvec: Worldvec) -> Canvasvec {
        worldvec * self.zoom_level
    }
}

// Creation and properties
//
impl Camera {
    pub fn new(
        pos: Worldpoint,
        zoom_level: f32,
        canvas_width: u32,
        canvas_height: u32,
        z_near: f32,
        z_far: f32,
        is_centered: bool,
    ) -> Camera {
        let dim_canvas = Vec2::new(canvas_width as f32, canvas_height as f32);

        Camera {
            pos,
            pos_pixelsnapped: pos.pixel_snapped(),
            zoom_level,
            dim_canvas,
            dim_frustum: dim_canvas / zoom_level,
            z_near,
            z_far,
            is_centered,
        }
    }

    #[inline]
    pub fn center(&self) -> Worldpoint {
        if self.is_centered {
            self.pos
        } else {
            self.pos + 0.5 * self.dim_frustum
        }
    }

    pub fn canvas_blit_offset(&mut self) -> Vec2 {
        self.pos - self.pos_pixelsnapped
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&mut self) -> Mat4 {
        let view = Mat4::scale(self.zoom_level, self.zoom_level, 1.0)
            * Mat4::translation(-self.pos_pixelsnapped.x, -self.pos_pixelsnapped.y, 0.0);

        let projection = if self.is_centered {
            Mat4::ortho_origin_center_flipped_y(
                self.dim_canvas.x,
                self.dim_canvas.y,
                self.z_near,
                self.z_far,
            )
        } else {
            Mat4::ortho_origin_left_top(
                self.dim_canvas.x,
                self.dim_canvas.y,
                self.z_near,
                self.z_far,
            )
        };
        projection * view
    }

    #[inline]
    pub fn bounds_pixelsnapped(&mut self) -> Rect {
        if self.is_centered {
            Rect::from_bounds_left_top_right_bottom(
                self.pos_pixelsnapped.x - 0.5 * self.dim_frustum.x,
                self.pos_pixelsnapped.y + 0.5 * self.dim_frustum.y,
                self.pos_pixelsnapped.x + 0.5 * self.dim_frustum.x,
                self.pos_pixelsnapped.y - 0.5 * self.dim_frustum.y,
            )
        } else {
            Rect::from_bounds_left_top_right_bottom(
                self.pos_pixelsnapped.x,
                self.pos_pixelsnapped.y,
                self.pos_pixelsnapped.x + self.dim_frustum.x,
                self.pos_pixelsnapped.y + self.dim_frustum.y,
            )
        }
    }

    #[inline]
    pub fn bounds(&self) -> Rect {
        if self.is_centered {
            Rect::from_bounds_left_top_right_bottom(
                self.pos.x - 0.5 * self.dim_frustum.x,
                self.pos.y + 0.5 * self.dim_frustum.y,
                self.pos.x + 0.5 * self.dim_frustum.x,
                self.pos.y - 0.5 * self.dim_frustum.y,
            )
        } else {
            Rect::from_bounds_left_top_right_bottom(
                self.pos.x,
                self.pos.y,
                self.pos.x + self.dim_frustum.x,
                self.pos.y + self.dim_frustum.y,
            )
        }
    }
}

// Manipulation
//
impl Camera {
    /// Zooms the camera to or away from a given world point.
    ///
    /// new_zoom_level > old_zoom_level -> magnify
    /// new_zoom_level < old_zoom_level -> minify
    #[inline]
    pub fn zoom_to_world_point(&mut self, worldpoint: Worldpoint, new_zoom_level: f32) {
        let old_zoom_level = self.zoom_level;
        self.zoom_level = new_zoom_level;
        self.dim_frustum = self.dim_canvas / new_zoom_level;
        self.pos = (self.pos - worldpoint) * (old_zoom_level / new_zoom_level) + worldpoint;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }

    /// Pans the camera using cursor movement distance on the canvas
    #[inline]
    pub fn pan(&mut self, canvas_move_distance: Canvasvec) {
        self.pos -= canvas_move_distance / self.zoom_level;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }

    #[inline]
    pub fn set_pos(&mut self, worldpoint: Worldpoint) {
        self.pos = worldpoint;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Game Camera
//

#[derive(Clone)]
pub struct GameCamera {
    pub cam: Camera,

    pub pos: Vec2,
    pub pos_target: Vec2,
    pub use_pixel_perfect_smoothing: bool,

    pub drag_margin_left: f32,
    pub drag_margin_top: f32,
    pub drag_margin_right: f32,
    pub drag_margin_bottom: f32,

    pub screenshake_offset: Vec2,
    pub screenshakers: Vec<ModulatorScreenShake>,
}

impl GameCamera {
    pub fn new(pos: Vec2, canvas_width: u32, canvas_height: u32, is_centered: bool) -> GameCamera {
        let cam = Camera::new(
            pos,
            1.0,
            canvas_width,
            canvas_height,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
            is_centered,
        );

        GameCamera {
            cam,
            pos,
            screenshake_offset: Vec2::zero(),
            screenshakers: Vec::new(),
            pos_target: pos,
            use_pixel_perfect_smoothing: false,

            drag_margin_left: 0.2,
            drag_margin_top: 0.1,
            drag_margin_right: 0.2,
            drag_margin_bottom: 0.1,
        }
    }

    pub fn add_shake(&mut self, shake: ModulatorScreenShake) {
        self.screenshakers.push(shake);
    }

    pub fn update(&mut self, deltatime: f32) {
        self.screenshake_offset = Vec2::zero();

        for shaker in self.screenshakers.iter_mut() {
            self.screenshake_offset += shaker.update_and_get_value(deltatime);
        }

        self.screenshakers
            .retain(|shaker| shaker.timer.is_running());

        self.pos = if self.use_pixel_perfect_smoothing {
            let mut points_till_target = Vec::new();
            iterate_line_bresenham(
                self.pos.pixel_snapped().to_i32(),
                self.pos_target.pixel_snapped().to_i32(),
                false,
                &mut |x, y| points_till_target.push(Vec2::new(x as f32, y as f32)),
            );

            let point_count = points_till_target.len();
            let skip_count = if point_count <= 1 {
                0
            } else if point_count <= 10 {
                1
            } else if point_count <= 20 {
                2
            } else if point_count <= 40 {
                3
            } else if point_count <= 80 {
                4
            } else if point_count <= 160 {
                5
            } else if point_count <= 320 {
                6
            } else {
                7
            };

            *points_till_target.iter().skip(skip_count).next().unwrap()
        } else {
            Vec2::lerp(self.pos, self.pos_target, 0.05)
        };
    }

    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
        self.pos_target = pos;
    }

    pub fn set_target_pos(&mut self, target_pos: Vec2, use_pixel_perfect_smoothing: bool) {
        self.use_pixel_perfect_smoothing = use_pixel_perfect_smoothing;
        self.pos_target = target_pos;
    }

    /// Zooms the camera to or away from a given world point.
    ///
    /// new_zoom_level > old_zoom_level -> magnify
    /// new_zoom_level < old_zoom_level -> minify
    #[inline]
    pub fn zoom_to_world_point(&mut self, worldpoint: Worldpoint, new_zoom_level: f32) {
        let old_zoom_level = self.cam.zoom_level;
        self.cam.zoom_level = new_zoom_level;
        self.cam.dim_frustum = self.cam.dim_canvas / new_zoom_level;
        self.pos = (self.pos - worldpoint) * (old_zoom_level / new_zoom_level) + worldpoint;
        self.pos_target = self.pos;
    }

    /// Pans the camera using cursor movement distance on the canvas
    #[inline]
    pub fn pan(&mut self, canvas_move_distance: Canvasvec) {
        self.pos -= canvas_move_distance / self.cam.zoom_level;
        self.pos_target = self.pos;
    }

    #[inline]
    pub fn center(&mut self) -> Worldpoint {
        self.sync_pos_internal();
        self.cam.center()
    }

    pub fn canvas_blit_offset(&mut self) -> Vec2 {
        self.sync_pos_internal();
        if self.use_pixel_perfect_smoothing {
            Vec2::zero()
        } else {
            self.cam.canvas_blit_offset()
        }
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&mut self) -> Mat4 {
        self.sync_pos_internal();
        self.cam.proj_view_matrix()
    }

    #[inline]
    pub fn bounds_pixelsnapped(&mut self) -> Rect {
        self.sync_pos_internal();
        self.cam.bounds_pixelsnapped()
    }

    #[inline]
    pub fn bounds(&mut self) -> Rect {
        self.sync_pos_internal();
        self.cam.bounds()
    }

    #[inline]
    fn sync_pos_internal(&mut self) {
        self.cam.set_pos(self.pos + self.screenshake_offset);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Camera shake
//
// Based on https://jonny.morrill.me/en/blog/gamedev-how-to-implement-a-camera-shake-effect/
//

#[derive(Clone)]
pub struct ModulatorScreenShake {
    pub amplitude: f32,
    pub frequency: f32,
    pub samples: Vec<Vec2>,
    pub timer: TimerSimple,
}

impl ModulatorScreenShake {
    pub fn new(
        random: &mut Random,
        amplitude: f32,
        duration: f32,
        frequency: f32,
    ) -> ModulatorScreenShake {
        let samplecount = ceili(duration * frequency) as usize;
        let samples: Vec<Vec2> = (0..samplecount)
            .map(|_sample_index| amplitude * random.vec2_in_unit_rect())
            .collect();

        ModulatorScreenShake {
            amplitude,
            frequency,
            samples,
            timer: TimerSimple::new_started(duration),
        }
    }

    pub fn update_and_get_value(&mut self, deltatime: f32) -> Vec2 {
        self.timer.update(deltatime);
        let percentage = self.timer.completion_ratio();

        let last_sample_index = self.samples.len() - 1;
        let sample_index = floori(last_sample_index as f32 * percentage) as usize;
        let sample_index_next = std::cmp::min(last_sample_index, sample_index + 1);

        let sample = self.samples[sample_index];
        let sample_next = self.samples[sample_index_next];

        let decay = 1.0 - percentage;

        decay * Vec2::lerp(sample, sample_next, percentage)
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
        canvas_width: u32,
        canvas_height: u32,
        screen_cursor_pos_x: i32,
        screen_cursor_pos_y: i32,
        screen_cursor_delta_x: i32,
        screen_cursor_delta_y: i32,
    ) -> CursorCoords {
        let screen_cursor_pos_previous_x = screen_cursor_pos_x - screen_cursor_delta_x;
        let screen_cursor_pos_previous_y = screen_cursor_pos_y - screen_cursor_delta_y;

        let canvas_pos = screen_point_to_canvas_point(
            screen_width,
            screen_height,
            canvas_width,
            canvas_height,
            screen_cursor_pos_x,
            screen_cursor_pos_y,
        );
        let canvas_pos_previous = screen_point_to_canvas_point(
            screen_width,
            screen_height,
            canvas_width,
            canvas_height,
            screen_cursor_pos_previous_x,
            screen_cursor_pos_previous_y,
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
    pub finger_primary: Option<CursorCoords>,
    pub finger_secondary: Option<CursorCoords>,
}

impl Cursors {
    pub fn new(
        camera: &Camera,
        mouse: &MouseState,
        touch: &TouchState,
        screen_width: u32,
        screen_height: u32,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Cursors {
        let mouse = CursorCoords::new(
            camera,
            screen_width,
            screen_height,
            canvas_width,
            canvas_height,
            mouse.pos_x,
            mouse.pos_y,
            mouse.delta_x,
            mouse.delta_y,
        );
        let finger_primary = touch.fingers.get(&0).map(|finger| {
            CursorCoords::new(
                camera,
                screen_width,
                screen_height,
                canvas_width,
                canvas_height,
                finger.pos_x,
                finger.pos_y,
                finger.delta_x,
                finger.delta_y,
            )
        });
        let finger_secondary = touch.fingers.get(&1).map(|finger| {
            CursorCoords::new(
                camera,
                screen_width,
                screen_height,
                canvas_width,
                canvas_height,
                finger.pos_x,
                finger.pos_y,
                finger.delta_x,
                finger.delta_y,
            )
        });

        Cursors {
            mouse,
            finger_primary,
            finger_secondary,
        }
    }
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

    out_systemcommands.push(AppCommand::ScreenSetGrabInput(config.grab_input));
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
// Gate

pub struct Gate {
    pub is_open: bool,
}

impl Gate {
    pub fn new_opened() -> Gate {
        Gate { is_open: true }
    }

    pub fn new_closed() -> Gate {
        Gate { is_open: false }
    }

    pub fn open(&mut self) -> bool {
        let was_opened = self.is_open;
        self.is_open = true;
        was_opened
    }

    pub fn close(&mut self) -> bool {
        let was_opened = self.is_open;
        self.is_open = false;
        was_opened
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Simple timer

#[derive(Debug, Clone, Copy)]
pub struct TimerSimple {
    pub time_cur: f32,
    pub time_end: f32,
}

impl Default for TimerSimple {
    fn default() -> Self {
        TimerSimple::new_started(1.0)
    }
}

impl TimerSimple {
    pub fn new_started(end_time: f32) -> TimerSimple {
        TimerSimple {
            time_cur: 0.0,
            time_end: end_time,
        }
    }

    pub fn new_stopped(end_time: f32) -> TimerSimple {
        TimerSimple {
            time_cur: end_time,
            time_end: end_time,
        }
    }

    pub fn update(&mut self, deltatime: f32) {
        self.time_cur = f32::min(self.time_cur + deltatime, self.time_end);
    }

    pub fn update_and_check_if_triggered(&mut self, deltatime: f32) -> bool {
        let time_previous = self.time_cur;
        self.time_cur = f32::min(self.time_cur + deltatime, self.time_end);

        self.time_cur == self.time_end && time_previous != self.time_end
    }

    pub fn is_running(&self) -> bool {
        self.time_cur < self.time_end
    }

    pub fn is_finished(&self) -> bool {
        !self.is_running()
    }

    pub fn completion_ratio(&self) -> f32 {
        self.time_cur / self.time_end
    }

    pub fn stop(&mut self) {
        self.time_cur = self.time_end;
    }

    pub fn restart(&mut self) {
        self.time_cur = 0.0;
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Timer

#[derive(Debug, Clone, Copy)]
pub enum Timerstate {
    Running {
        completion_ratio: f32,
    },
    Triggered {
        trigger_count: u64,
        remaining_delta: f32,
    },
    Paused,
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    time_cur: f32,
    time_end: f32,
    trigger_count: u64,
    trigger_count_max: u64,
    pub is_paused: bool,
}

impl Timer {
    pub fn new_started(trigger_time: f32) -> Timer {
        Timer {
            time_cur: 0.0,
            time_end: trigger_time,
            trigger_count: 0,
            trigger_count_max: 1,
            is_paused: false,
        }
    }

    pub fn new_stopped(trigger_time: f32) -> Timer {
        Timer {
            time_cur: trigger_time,
            time_end: trigger_time,
            trigger_count: 1,
            trigger_count_max: 1,
            is_paused: false,
        }
    }

    pub fn new_repeating_started(trigger_time: f32) -> Timer {
        Timer {
            time_cur: 0.0,
            time_end: trigger_time,
            trigger_count: 0,
            trigger_count_max: std::u64::MAX,
            is_paused: false,
        }
    }

    pub fn new_repeating_stopped(trigger_time: f32) -> Timer {
        Timer {
            time_cur: trigger_time,
            time_end: trigger_time,
            trigger_count: std::u64::MAX,
            trigger_count_max: std::u64::MAX,
            is_paused: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.trigger_count == self.trigger_count_max
    }

    pub fn is_running(&self) -> bool {
        !self.is_finished()
    }

    pub fn completion_ratio(&self) -> f32 {
        (self.time_cur % self.time_end) / self.time_end
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = true;
    }

    pub fn stop(&mut self) {
        self.time_cur = self.time_end;
        self.trigger_count = self.trigger_count_max;
    }

    pub fn restart(&mut self) {
        self.time_cur = 0.0;
        self.trigger_count = 0;
    }

    pub fn update(&mut self, deltatime: f32) -> Timerstate {
        if self.trigger_count >= self.trigger_count_max {
            return Timerstate::Done;
        }
        if self.is_paused {
            return Timerstate::Paused;
        }

        self.time_cur += deltatime;

        if self.time_cur > self.time_end {
            self.time_cur -= self.time_end;
            self.trigger_count += 1;

            let remaining_delta = if self.trigger_count == self.trigger_count_max {
                // NOTE: This was the last possible trigger event so we also return any
                //       remaining time we accumulated and set the current time to its max so that
                //       the completion ratio is still correct.
                let remainder = self.time_cur;
                self.time_cur = self.time_end;
                remainder
            } else {
                0.0
            };

            return Timerstate::Triggered {
                trigger_count: self.trigger_count,
                remaining_delta,
            };
        }

        Timerstate::Running {
            completion_ratio: self.completion_ratio(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Special timers

#[derive(Debug, Clone, Copy)]
pub struct TriggerRepeating {
    timer: Timer,
    triggertime_initial: f32,
    triggertime_repeating: f32,
}

impl TriggerRepeating {
    #[inline]
    pub fn new(trigger_time: f32) -> TriggerRepeating {
        TriggerRepeating {
            timer: Timer::new_repeating_started(trigger_time),
            triggertime_initial: trigger_time,
            triggertime_repeating: trigger_time,
        }
    }

    #[inline]
    pub fn new_with_distinct_triggertimes(
        trigger_time_initial: f32,
        trigger_time_repeat: f32,
    ) -> TriggerRepeating {
        TriggerRepeating {
            timer: Timer::new_repeating_started(trigger_time_initial),
            triggertime_initial: trigger_time_initial,
            triggertime_repeating: trigger_time_repeat,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.timer = Timer::new_repeating_started(self.triggertime_initial);
    }

    #[inline]
    pub fn completion_ratio(&self) -> f32 {
        self.timer.completion_ratio()
    }

    /// Returns true if actually triggered
    #[inline]
    pub fn update_and_check(&mut self, deltatime: f32) -> bool {
        match self.timer.update(deltatime) {
            Timerstate::Triggered { trigger_count, .. } => {
                if trigger_count == 1 {
                    self.timer.time_end = self.triggertime_repeating;
                }
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TimerStateSwitchBinary {
    pub repeat_timer: TriggerRepeating,
    pub active: bool,
}

impl TimerStateSwitchBinary {
    pub fn new(start_active: bool, start_time: f32, phase_duration: f32) -> TimerStateSwitchBinary {
        TimerStateSwitchBinary {
            repeat_timer: TriggerRepeating::new_with_distinct_triggertimes(
                start_time,
                phase_duration,
            ),
            active: start_active,
        }
    }
    pub fn update_and_check(&mut self, deltatime: f32) -> bool {
        if self.repeat_timer.update_and_check(deltatime) {
            self.active = !self.active;
        }
        self.active
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Choreographer

#[derive(Debug, Clone)]
pub struct Choreographer {
    current_stage: usize,
    stages: Vec<Timer>,
    specials: HashMap<String, usize>,
    pub time_accumulator: f32,
}

impl Choreographer {
    pub fn new() -> Choreographer {
        Choreographer {
            current_stage: 0,
            stages: Vec::new(),
            specials: HashMap::new(),
            time_accumulator: 0.0,
        }
    }

    pub fn restart(&mut self) {
        self.current_stage = 0;
        self.stages.clear();
        self.time_accumulator = 0.0;
    }

    pub fn update(&mut self, deltatime: f32) -> &mut Self {
        self.current_stage = 0;
        self.time_accumulator += deltatime;
        self
    }

    pub fn get_previous_triggercount(&self) -> u64 {
        assert!(self.current_stage > 0);
        self.stages[self.current_stage - 1].trigger_count
    }

    /// NOTE: This only resets the last `current_time` and `trigger_time` but NOT
    ///       the `trigger_count`
    pub fn reset_previous(&mut self, new_delay: f32) {
        assert!(self.current_stage > 0);
        self.stages[self.current_stage - 1].time_cur = 0.0;
        self.stages[self.current_stage - 1].time_end = new_delay;
    }

    pub fn wait(&mut self, delay: f32) -> bool {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_started(delay));
        }
        let timer = &mut self.stages[current_stage];

        match timer.update(self.time_accumulator) {
            Timerstate::Triggered {
                remaining_delta, ..
            } => {
                self.time_accumulator = remaining_delta;
                true
            }
            Timerstate::Done => true,
            Timerstate::Running { .. } => {
                self.time_accumulator = 0.0;
                false
            }
            Timerstate::Paused => unreachable!(),
        }
    }

    pub fn tween(&mut self, tween_time: f32) -> (f32, bool) {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_started(tween_time));
        }
        let timer = &mut self.stages[current_stage];

        match timer.update(self.time_accumulator) {
            Timerstate::Triggered {
                remaining_delta, ..
            } => {
                self.time_accumulator = remaining_delta;
                (1.0, true)
            }
            Timerstate::Done => (1.0, true),
            Timerstate::Running { completion_ratio } => {
                self.time_accumulator = 0.0;
                (completion_ratio, false)
            }
            Timerstate::Paused => unreachable!(),
        }
    }

    pub fn once(&mut self) -> bool {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_stopped(1.0));
            return true;
        }

        false
    }

    pub fn hitcount(&mut self) -> u64 {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_repeating_started(0.0));
        }

        let timer = &mut self.stages[current_stage];
        timer.trigger_count
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fader

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Fadestate {
    FadedIn,
    FadedOut,
    FadingIn,
    FadingOut,
}

#[derive(Clone)]
pub struct Fader {
    pub timer: TimerSimple,
    pub state: Fadestate,
}

impl Fader {
    pub fn new_faded_out() -> Fader {
        Fader {
            timer: TimerSimple::new_stopped(1.0),
            state: Fadestate::FadedOut,
        }
    }
    pub fn new_faded_in() -> Fader {
        Fader {
            timer: TimerSimple::new_stopped(1.0),
            state: Fadestate::FadedIn,
        }
    }

    pub fn start_fading_out(&mut self, fade_out_time: f32) {
        self.state = Fadestate::FadingOut;
        self.timer = TimerSimple::new_started(fade_out_time);
    }

    pub fn start_fading_in(&mut self, fade_in_time: f32) {
        self.state = Fadestate::FadingIn;
        self.timer = TimerSimple::new_started(fade_in_time);
    }

    pub fn opacity(&self) -> f32 {
        match self.state {
            Fadestate::FadedIn => 1.0,
            Fadestate::FadedOut => 0.0,
            Fadestate::FadingIn => self.timer.completion_ratio(),
            Fadestate::FadingOut => 1.0 - self.timer.completion_ratio(),
        }
    }

    pub fn update(&mut self, deltatime: f32) {
        if self.state == Fadestate::FadedIn || self.state == Fadestate::FadedOut {
            return;
        }

        self.timer.update(deltatime);

        if self.timer.is_finished() {
            if self.state == Fadestate::FadingIn {
                self.state = Fadestate::FadedIn;
            } else {
                self.state = Fadestate::FadedOut;
            }
        }
    }

    pub fn is_fading(self) -> bool {
        self.state == Fadestate::FadingIn || self.state == Fadestate::FadingOut
    }

    pub fn is_finished(self) -> bool {
        self.state == Fadestate::FadedIn || self.state == Fadestate::FadedOut
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ScreenFader

#[derive(Clone)]
pub struct CanvasFader {
    pub color_start: Color,
    pub color_end: Color,
    pub timer: TimerSimple,
}

impl CanvasFader {
    pub fn new(color_start: Color, color_end: Color, fade_time_seconds: f32) -> CanvasFader {
        CanvasFader {
            color_start,
            color_end,
            timer: TimerSimple::new_started(fade_time_seconds),
        }
    }

    pub fn completion_ratio(&self) -> f32 {
        self.timer.completion_ratio()
    }

    pub fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        deltatime: f32,
        canvas_width: u32,
        canvas_height: u32,
    ) {
        self.timer.update(deltatime);

        let percent = self.timer.completion_ratio();
        let color = Color::mix(self.color_start, self.color_end, percent);
        if color.a > 0.0 {
            draw.draw_rect(
                Rect::from_width_height(canvas_width as f32, canvas_height as f32),
                true,
                DEPTH_SCREEN_FADER,
                color,
                ADDITIVITY_NONE,
                DrawSpace::Canvas,
            );
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Splashscreen

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SplashscreenState {
    StartedFadingIn,
    IsFadingIn,
    FinishedFadingIn,
    Sustain,
    StartedFadingOut,
    IsFadingOut,
    FinishedFadingOut,
    IsDone,
}

#[derive(Clone)]
pub struct SplashScreen {
    time_fade_in: f32,
    time_fade_out: f32,
    time_sustain_max: f32,
    time_sustain_current: f32,

    sprite: Sprite,
    fader: CanvasFader,
    state: SplashscreenState,
}

impl SplashScreen {
    pub fn new(
        sprite: Sprite,
        time_fade_in: f32,
        time_fade_out: f32,
        time_sustain: f32,
    ) -> SplashScreen {
        SplashScreen {
            time_fade_in,
            time_fade_out,
            time_sustain_max: time_sustain,
            time_sustain_current: 0.0,
            sprite,
            fader: CanvasFader::new(Color::black(), Color::white(), time_fade_in),
            state: SplashscreenState::StartedFadingIn,
        }
    }

    pub fn force_fast_forward(&mut self) {
        self.time_sustain_current = self.time_sustain_max;
    }

    pub fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        deltatime: f32,
        canvas_width: u32,
        canvas_height: u32,
    ) -> SplashscreenState {
        if self.state == SplashscreenState::IsDone {
            return self.state;
        }

        self.fader
            .update_and_draw(draw, deltatime, canvas_width, canvas_height);

        let opacity = if self.state <= SplashscreenState::Sustain {
            self.fader.completion_ratio()
        } else {
            1.0 - self.fader.completion_ratio()
        };

        let (splash_rect, letterbox_rects) = letterbox_rects_create(
            self.sprite.untrimmed_dimensions.x as i32,
            self.sprite.untrimmed_dimensions.y as i32,
            canvas_width as i32,
            canvas_height as i32,
        );
        draw.draw_sprite(
            &self.sprite,
            Transform::from_pos(Vec2::new(
                splash_rect.left() as f32,
                splash_rect.top() as f32,
            )),
            false,
            false,
            DEPTH_SPLASH,
            opacity * Color::white(),
            ADDITIVITY_NONE,
            DrawSpace::Canvas,
        );

        for letterbox_rect in &letterbox_rects {
            draw.draw_rect(
                Rect::from(*letterbox_rect),
                true,
                DEPTH_SCREEN_FADER,
                opacity * Color::white(),
                ADDITIVITY_NONE,
                DrawSpace::Canvas,
            );
        }

        match self.state {
            SplashscreenState::StartedFadingIn => {
                self.state = SplashscreenState::IsFadingIn;
                SplashscreenState::StartedFadingIn
            }
            SplashscreenState::IsFadingIn => {
                if self.fader.completion_ratio() == 1.0 {
                    self.state = SplashscreenState::FinishedFadingIn;
                }
                SplashscreenState::IsFadingIn
            }
            SplashscreenState::FinishedFadingIn => {
                self.state = SplashscreenState::Sustain;
                SplashscreenState::FinishedFadingIn
            }
            SplashscreenState::Sustain => {
                if self.time_sustain_current < self.time_sustain_max {
                    self.time_sustain_current += deltatime;
                } else {
                    self.state = SplashscreenState::StartedFadingOut;
                    self.fader =
                        CanvasFader::new(Color::white(), Color::transparent(), self.time_fade_out);
                }
                SplashscreenState::Sustain
            }
            SplashscreenState::StartedFadingOut => {
                self.state = SplashscreenState::IsFadingOut;
                SplashscreenState::StartedFadingOut
            }
            SplashscreenState::IsFadingOut => {
                if self.fader.completion_ratio() == 1.0 {
                    self.state = SplashscreenState::FinishedFadingOut;
                }
                SplashscreenState::IsFadingOut
            }
            SplashscreenState::FinishedFadingOut => {
                self.state = SplashscreenState::IsDone;
                SplashscreenState::FinishedFadingOut
            }
            SplashscreenState::IsDone => SplashscreenState::IsDone,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Animations

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnimationFrame<FrameType: Clone> {
    pub duration_seconds: f32,
    #[serde(bound(deserialize = "FrameType: serde::de::DeserializeOwned"))]
    pub value: FrameType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Animation<FrameType: Clone> {
    pub name: String,
    #[serde(bound(deserialize = "FrameType: serde::de::DeserializeOwned"))]
    pub frames: Vec<AnimationFrame<FrameType>>,
    pub length: f32,
}

impl<FrameType: Clone> Animation<FrameType> {
    pub fn new_empty(name: String) -> Animation<FrameType> {
        Animation {
            name,
            frames: Vec::with_capacity(32),
            length: 0.0,
        }
    }

    pub fn add_frame(&mut self, duration_seconds: f32, value: FrameType) {
        assert!(duration_seconds > 0.0);

        self.length += duration_seconds;
        self.frames.push(AnimationFrame {
            duration_seconds,
            value,
        });
    }

    fn frame_index_and_percent_at_time(&self, time: f32, wrap_around: bool) -> (usize, f32) {
        assert!(!self.frames.is_empty());

        let time = if wrap_around {
            wrap_value_in_range(time, self.length)
        } else {
            clampf(time, 0.0, self.length)
        };

        let mut frame_start = 0.0;

        for (index, frame) in self.frames.iter().enumerate() {
            let frame_end = frame_start + frame.duration_seconds;

            if time < frame_end {
                let percent = (time - frame_start) / frame.duration_seconds;
                return (index, percent);
            }

            frame_start = frame_end;
        }

        (self.frames.len() - 1, 1.0)
    }

    pub fn frame_at_time(&self, time: f32, wrap_around: bool) -> &FrameType {
        let (index, _percent) = self.frame_index_and_percent_at_time(time, wrap_around);
        &self.frames[index].value
    }

    pub fn frame_at_percentage(&self, percentage: f32) -> &FrameType {
        debug_assert!(0.0 <= percentage && percentage <= 1.0);
        let time = percentage * self.length;
        self.frame_at_time(time, false)
    }
}

impl Animation<f32> {
    pub fn value_at_time_interpolated_linear(&self, time: f32, wrap_around: bool) -> f32 {
        let (frame_index, frametime_percent) =
            self.frame_index_and_percent_at_time(time, wrap_around);
        let next_frame_index = if wrap_around {
            (frame_index + 1) % self.frames.len()
        } else {
            usize::min(frame_index + 1, self.frames.len() - 1)
        };

        let value_start = self.frames[frame_index].value;
        let value_end = self.frames[next_frame_index].value;

        lerp(value_start, value_end, frametime_percent)
    }
}

#[derive(Clone)]
pub struct AnimationPlayer<FrameType: Clone> {
    pub current_frametime: f32,
    pub playback_speed: f32,
    pub looping: bool,
    pub animation: Animation<FrameType>,
    pub has_finished: bool,
}

impl<FrameType: Clone> AnimationPlayer<FrameType> {
    pub fn new_from_beginning(
        animation: Animation<FrameType>,
        playback_speed: f32,
        looping: bool,
    ) -> AnimationPlayer<FrameType> {
        assert!(animation.length > 0.0);

        AnimationPlayer {
            current_frametime: 0.0,
            playback_speed,
            looping,
            animation,
            has_finished: false,
        }
    }

    pub fn new_from_end(
        animation: Animation<FrameType>,
        playback_speed: f32,
        looping: bool,
    ) -> AnimationPlayer<FrameType> {
        let mut result = AnimationPlayer::new_from_beginning(animation, playback_speed, looping);
        result.restart_from_end();
        result
    }

    pub fn restart_from_beginning(&mut self) {
        self.current_frametime = 0.0;
        self.has_finished = false;
    }

    pub fn restart_from_end(&mut self) {
        self.current_frametime = self.animation.length;
        self.has_finished = false;
    }

    pub fn update(&mut self, deltatime: f32) {
        if self.playback_speed == 0.0 {
            return;
        }

        let new_frametime = self.current_frametime + self.playback_speed * deltatime;
        if self.looping {
            self.current_frametime = wrap_value_in_range(new_frametime, self.animation.length);
        } else {
            self.current_frametime = clampf(new_frametime, 0.0, self.animation.length);
            if self.current_frametime == self.animation.length && self.playback_speed > 0.0 {
                self.has_finished = true;
            }
            if self.current_frametime == 0.0 && self.playback_speed < 0.0 {
                self.has_finished = true;
            }
        }
    }

    pub fn frame_at_percentage(&self, percentage: f32) -> &FrameType {
        self.animation.frame_at_percentage(percentage)
    }

    pub fn current_frame(&self) -> &FrameType {
        self.animation
            .frame_at_time(self.current_frametime, self.looping)
    }
}

impl AnimationPlayer<f32> {
    pub fn value_current_interpolated_linear(&self) -> f32 {
        self.animation
            .value_at_time_interpolated_linear(self.current_frametime, self.looping)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Particles

#[derive(Copy, Clone, Default)]
pub struct ParticleSystemParams {
    pub gravity: Vec2,
    pub vel_start: Vec2,
    pub vel_max: f32,
    pub scale_start: f32,
    pub scale_end: f32,
    pub spawn_radius: f32,
    pub lifetime: f32,
    pub additivity_start: f32,
    pub additivity_end: f32,
    pub color_start: Color,
    pub color_end: Color,
}

#[derive(Clone)]
pub struct ParticleSystem {
    count_max: usize,
    root_pos: Vec2,

    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    age: Vec<f32>,

    pub params: ParticleSystemParams,

    time_since_last_spawn: f32,
}

impl ParticleSystem {
    pub fn new(params: ParticleSystemParams, count_max: usize, root_pos: Vec2) -> ParticleSystem {
        ParticleSystem {
            count_max,
            root_pos,
            pos: Vec::with_capacity(count_max),
            vel: Vec::with_capacity(count_max),
            age: Vec::with_capacity(count_max),
            params,
            time_since_last_spawn: 0.0,
        }
    }

    pub fn set_count_max(&mut self, count_max: usize) {
        self.count_max = count_max;
    }

    pub fn move_to(&mut self, pos: Vec2) {
        self.root_pos = pos;
    }

    pub fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        random: &mut Random,
        deltatime: f32,
        depth: f32,
        drawspace: DrawSpace,
    ) {
        // Update
        for index in 0..self.pos.len() {
            linear_motion_integrate_v2(
                &mut self.pos[index],
                &mut self.vel[index],
                self.params.gravity,
                self.params.vel_max,
                deltatime,
            );
        }

        // Draw
        for index in 0..self.pos.len() {
            let age_percentage = self.age[index] / self.params.lifetime;
            let scale = lerp(
                self.params.scale_start,
                self.params.scale_end,
                age_percentage,
            );
            let additivity = lerp(
                self.params.additivity_start,
                self.params.additivity_end,
                age_percentage,
            );
            let color = Color::mix(
                self.params.color_start,
                self.params.color_end,
                age_percentage,
            );
            draw.draw_rect_transformed(
                Vec2::ones(),
                true,
                true,
                Vec2::zero(),
                Transform::from_pos_scale_uniform(self.pos[index].pixel_snapped(), scale),
                depth,
                color,
                additivity,
                drawspace,
            );
        }

        // Remove old
        for index in (0..self.pos.len()).rev() {
            self.age[index] += deltatime;
            if self.age[index] > self.params.lifetime {
                self.pos.swap_remove(index);
                self.vel.swap_remove(index);
                self.age.swap_remove(index);
            }
        }

        self.time_since_last_spawn += deltatime;

        // Spawn new
        if self.count_max > 0 {
            let time_between_spawns = self.params.lifetime / self.count_max as f32;
            if self.pos.len() < self.count_max && self.time_since_last_spawn >= time_between_spawns
            {
                self.time_since_last_spawn -= time_between_spawns;
                let pos = self.root_pos + self.params.spawn_radius * random.vec2_in_unit_disk();
                let vel = self.params.vel_start;

                self.pos.push(pos);
                self.vel.push(vel);
                self.age.push(0.0);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Afterimage

#[derive(Clone)]
pub struct Afterimage {
    count_max: usize,

    lifetime: f32,
    additivity_modulate_start: f32,
    additivity_modulate_end: f32,
    color_modulate_start: Color,
    color_modulate_end: Color,

    sprite: Vec<Sprite>,
    age: Vec<f32>,
    xform: Vec<Transform>,
    flip_horizontally: Vec<bool>,
    flip_vertically: Vec<bool>,
    color_modulate: Vec<Color>,
    additivity: Vec<f32>,

    time_since_last_spawn: f32,
}

impl Afterimage {
    pub fn new(
        lifetime: f32,
        additivity_modulate_start: f32,
        additivity_modulate_end: f32,
        color_modulate_start: Color,
        color_modulate_end: Color,
        count_max: usize,
    ) -> Afterimage {
        Afterimage {
            count_max,

            lifetime,
            additivity_modulate_start,
            additivity_modulate_end,
            color_modulate_start,
            color_modulate_end,

            sprite: Vec::with_capacity(count_max),
            age: Vec::with_capacity(count_max),
            xform: Vec::with_capacity(count_max),
            flip_horizontally: Vec::with_capacity(count_max),
            flip_vertically: Vec::with_capacity(count_max),
            color_modulate: Vec::with_capacity(count_max),
            additivity: Vec::with_capacity(count_max),

            time_since_last_spawn: 0.0,
        }
    }

    pub fn set_count_max(&mut self, count_max: usize) {
        self.count_max = count_max;
    }

    pub fn add_afterimage_image_if_needed(
        &mut self,
        deltatime: f32,
        newimage_sprite: Sprite,
        newimage_xform: Transform,
        newimage_flip_horizontally: bool,
        newimage_flip_vertically: bool,
        newimage_color_modulate: Color,
        newimage_additivity: f32,
    ) {
        self.time_since_last_spawn += deltatime;

        if self.count_max > 0 {
            let time_between_spawns = self.lifetime / self.count_max as f32;
            if self.xform.len() < self.count_max
                && self.time_since_last_spawn >= time_between_spawns
            {
                self.time_since_last_spawn -= time_between_spawns;

                self.sprite.push(newimage_sprite);
                self.age.push(0.0);
                self.xform.push(newimage_xform);
                self.flip_horizontally.push(newimage_flip_horizontally);
                self.flip_vertically.push(newimage_flip_vertically);
                self.color_modulate.push(newimage_color_modulate);
                self.additivity.push(newimage_additivity);
            }
        }
    }

    pub fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        deltatime: f32,
        draw_depth: f32,
        drawspace: DrawSpace,
    ) {
        for index in 0..self.sprite.len() {
            let age_percentage = self.age[index] / self.lifetime;
            let additivity = lerp(
                self.additivity_modulate_start,
                self.additivity_modulate_end,
                age_percentage,
            );
            let color = Color::mix(
                self.color_modulate_start,
                self.color_modulate_end,
                age_percentage,
            );

            draw.draw_sprite(
                &self.sprite[index],
                self.xform[index],
                self.flip_horizontally[index],
                self.flip_vertically[index],
                draw_depth,
                color * self.color_modulate[index],
                additivity * self.additivity[index],
                drawspace,
            );
        }

        for index in (0..self.xform.len()).rev() {
            self.age[index] += deltatime;
            if self.age[index] > self.lifetime {
                self.sprite.swap_remove(index);
                self.age.swap_remove(index);
                self.xform.swap_remove(index);
                self.flip_horizontally.swap_remove(index);
                self.flip_vertically.swap_remove(index);
                self.color_modulate.swap_remove(index);
                self.additivity.swap_remove(index);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug drawing

#[inline]
pub fn debug_draw_grid(
    draw: &mut Drawstate,
    camera: &Camera,
    world_grid_size: f32,
    screen_width: f32,
    screen_height: f32,
    line_thickness: i32,
    color: Color,
    depth: f32,
) {
    assert!(line_thickness > 0);

    let frustum = camera.bounds();
    let top = f32::floor(frustum.top() / world_grid_size) * world_grid_size;
    let bottom = f32::ceil(frustum.bottom() / world_grid_size) * world_grid_size;
    let left = f32::floor(frustum.left() / world_grid_size) * world_grid_size;
    let right = f32::ceil(frustum.right() / world_grid_size) * world_grid_size;

    let mut x = left;
    while x <= right {
        let start = Vec2::new(x, top);
        let end = Vec2::new(x, bottom);

        let start = camera.worldpoint_to_canvaspoint(start);
        let end = camera.worldpoint_to_canvaspoint(end);

        let start = (start / camera.dim_canvas) * Vec2::new(screen_width, screen_height);
        let end = (end / camera.dim_canvas) * Vec2::new(screen_width, screen_height);

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            start.x + line_thickness as f32,
            end.y,
        );
        draw.draw_rect(rect, true, depth, color, ADDITIVITY_NONE, DrawSpace::Screen);

        x += world_grid_size;
    }
    let mut y = top;
    while y <= bottom {
        let start = Vec2::new(left, y);
        let end = Vec2::new(right, y);

        let start = camera.worldpoint_to_canvaspoint(start);
        let end = camera.worldpoint_to_canvaspoint(end);

        let start = (start / camera.dim_canvas) * Vec2::new(screen_width, screen_height);
        let end = (end / camera.dim_canvas) * Vec2::new(screen_width, screen_height);

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            end.x,
            start.y + line_thickness as f32,
        );
        draw.draw_rect(rect, true, depth, color, ADDITIVITY_NONE, DrawSpace::Screen);

        y += world_grid_size;
    }
}
