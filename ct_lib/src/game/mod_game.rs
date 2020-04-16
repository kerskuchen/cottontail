mod assets;
mod input;

pub use assets::*;
pub use input::*;

use super::audio::*;
use super::bitmap::*;
use super::draw::*;
use super::math::*;
use super::random::*;
use super::sprite::*;
use super::system;
use super::*;

use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

pub const DEPTH_DEBUG: Depth = 90.0;
pub const DEPTH_DEVELOP_OVERLAY: Depth = 80.0;
pub const DEPTH_SPLASH: Depth = 70.0;
pub const DEPTH_SCREEN_FADER: Depth = 60.0;

pub enum SystemCommand {
    FullscreenToggle,
    FullscreenEnable(bool),
    TextinputStart {
        inputrect_x: i32,
        inputrect_y: i32,
        inputrect_width: u32,
        inputrect_height: u32,
    },
    TextinputStop,
    ScreenSetGrabInput(bool),
    WindowedModeAllowResizing(bool),
    WindowedModeAllow(bool),
    WindowedModeSetSize {
        width: u32,
        height: u32,
        minimum_width: u32,
        minimum_height: u32,
    },
    Shutdown,
    Restart,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Gamestate

pub struct GameInfo {
    pub game_window_title: String,
    pub game_save_folder_name: String,
    pub game_company_name: String,
}

pub trait GameStateInterface {
    fn get_game_config() -> GameInfo;
    fn get_window_config() -> WindowConfig;
    fn new(
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &mut GameAssets,
        input: &GameInput,
    ) -> Self;
    fn update(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &mut GameAssets,
        input: &GameInput,
    );
}

const SPLASHSCREEN_FADEIN_TIME: f32 = 0.5;
const SPLASHSCREEN_SUSTAIN_TIME: f32 = 0.5;
const SPLASHSCREEN_FADEOUT_TIME: f32 = 0.5;

#[derive(Clone)]
pub struct GameMemory<GameStateType: GameStateInterface> {
    pub game: Option<GameStateType>,
    pub draw: Option<Drawstate>,
    pub audio: Option<Audiostate>,
    pub assets: Option<GameAssets>,

    pub splashscreen: Option<SplashScreen>,
}

impl<GameStateType: GameStateInterface> Default for GameMemory<GameStateType> {
    fn default() -> Self {
        GameMemory {
            game: None,
            draw: None,
            audio: None,
            assets: None,
            splashscreen: None,
        }
    }
}

impl<GameStateType: GameStateInterface> GameMemory<GameStateType> {
    pub fn update(
        &mut self,
        input: &GameInput,
        current_audio_frame_index: AudioFrameIndex,
        out_systemcommands: &mut Vec<SystemCommand>,
    ) {
        if self.draw.is_none() {
            let _drawstate_setup_timer = TimerScoped::new_scoped("Drawstate setup time");

            let window_config = GameStateType::get_window_config();
            let atlas = game_load_atlas("assets_baked");
            let fonts = game_load_fonts("assets_baked");
            let mut draw = Drawstate::new(atlas, fonts);
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
            );
            self.draw = Some(draw);
        }
        if self.assets.is_none() {
            let animations = game_load_animations("assets_baked");
            self.assets = Some(GameAssets::new(animations));
        }
        if self.audio.is_none() {
            self.audio = Some(Audiostate::new());
        }

        let draw = self.draw.as_mut().unwrap();
        let audio = self.audio.as_mut().unwrap();
        let assets = self.assets.as_mut().unwrap();

        audio.update_frame_index(current_audio_frame_index);
        draw.begin_frame();

        if self.splashscreen.is_none() {
            let splash_sprite = draw.get_sprite_by_name("splash").clone();
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
        match splashscreen.update_and_draw(
            draw,
            input.target_deltatime,
            canvas_width,
            canvas_height,
        ) {
            SplashscreenState::StartedFadingIn => {}
            SplashscreenState::IsFadingIn => {}
            SplashscreenState::FinishedFadingIn => {
                {
                    let _audiostate_setup_timer = TimerScoped::new_scoped("Audiostate setup time");

                    let audiorecordings_mono = game_load_audiorecordings_mono("assets_baked");
                    for (recording_name, buffer) in audiorecordings_mono.into_iter() {
                        audio.add_recording_mono(&recording_name, buffer);
                    }
                }

                {
                    let _gamestate_setup_timer = TimerScoped::new_scoped("Gamestate setup time");

                    assert!(self.game.is_none());
                    self.game = Some(GameStateType::new(draw, audio, assets, &input));
                }
            }

            SplashscreenState::Sustain => {}
            SplashscreenState::StartedFadingOut => {}
            SplashscreenState::IsFadingOut => {}
            SplashscreenState::FinishedFadingOut => {}
            SplashscreenState::IsDone => {}
        }

        if let Some(game) = self.game.as_mut() {
            game.update(draw, audio, assets, input);
            game_handle_system_keys(&input.keyboard, out_systemcommands);
        }

        draw.finish_frame(
            input.screen_framebuffer_width,
            input.screen_framebuffer_height,
        );
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Camera and coordinates

/// Camera with its position in the center of its view-rect.
///
/// zoom_level > 1.0 : zoomed in
/// zoom_level < 1.0 : zoomed out
///
/// # Example: Camera bounds
/// ```
/// # use game_lib::math::*;
///
/// let pos = Point::new(50.0, -50.0);
/// let dim = Vec2::new(200.0, 100.0);
/// let zoom = 2.0;
///
///
/// let cam_origin = Point::new(12.0, 34.0);
/// let mut cam = Camera::new(cam_origin, dim.x, dim.y, -1.0, 1.0);
///
/// // NOTE: Our panning vector is the negative of our move vector. This is to simulate the
/// //       mouse grabbing and panning of the canvas, like i.e. touch-navigation on mobile devices.
/// let move_vec = pos - cam_origin;
/// let panning_vec = -move_vec;
/// cam.pan(panning_vec);
/// assert_eq!(cam.pos(), pos);
///
/// cam.zoom_to_world_point(pos, zoom);
/// assert_eq!(cam.zoom_level, zoom);
/// assert_eq!(cam.dim_zoomed(), dim / zoom);
///
/// let left =   pos.x - 0.5 * dim.x / zoom;
/// let right =  pos.x + 0.5 * dim.x / zoom;
/// let top =    pos.y - 0.5 * dim.y / zoom;
/// let bottom = pos.y + 0.5 * dim.y / zoom;
///
/// let bounds = cam.frustum();
/// assert_eq!(bounds.left, left);
/// assert_eq!(bounds.right, right);
/// assert_eq!(bounds.bottom, bottom);
/// assert_eq!(bounds.top, top);
/// ```
///
/// # Example: Mouse panning and zooming
///
/// ```
/// # use game_lib::math::*;
///
/// // Canvas and camera setup
/// let canvas_width = 320.0;
/// let canvas_height = 180.0;
/// let mut cam = Camera::new(Point::zero(), canvas_width, canvas_height, -1.0, 1.0);
///
/// // Current and old mouse state
/// let mouse_pos_canvas = CanvasPoint::new(50.0, 130.0);
/// let mouse_delta_canvas = Canvasvec::new(15.0, -20.0);
/// let mouse_button_right_pressed = true;
/// let mouse_button_middle_pressed = false;
/// let mouse_wheel_delta = 0;
///
/// // World mouse position and delta
/// let mouse_pos_world = cam.canvas_point_to_world_point(mouse_pos_canvas);
/// let mouse_delta_world = cam.canvas_vec_to_world_vec(mouse_pos_canvas);
///
/// // Pan camera
/// if mouse_button_right_pressed {
///     cam.pan(mouse_delta_canvas);
/// }
/// // Reset zoom
/// if mouse_button_middle_pressed {
///     cam.zoom_to_world_point(mouse_pos_world, 1.0);
/// }
/// // Zoom in or out by factors of two
/// if mouse_wheel_delta > 0 {
///     // Magnify up till 8x
///     let new_zoom_level = f32::min(cam.zoom_level * 2.0, 8.0);
///     cam.zoom_to_world_point(mouse_pos_world, new_zoom_level);
/// } else if mouse_wheel_delta < 0 {
///     // Minify down till 1/8
///     let new_zoom_level = f32::max(cam.zoom_level / 2.0, 1.0 / 8.0);
///     cam.zoom_to_world_point(mouse_pos_world, new_zoom_level);
/// }
///
/// // Get project-view-matrix from cam and use it for drawing
/// let transform = cam.proj_view_matrix();
///
/// // ..
/// ```

/// NOTE: Camera pos equals is the top-left of the screen or the center depending on the
/// `is_centered` flag. It has the following bounds in world coordinates:
/// non-centered: [pos.x, pos.x + dim_frustum.w] x [pos.y, pos.y + dim_frustum.h]
/// centered:     [pos.x - 0.5*dim_frustum.w, pos.x + 0.5*dim_frustum.w] x
///               [pos.y - 0.5*dim_frustum.h, pos.y + 0.5*dim_frustum.h]
///
/// with:
/// dim_frustum = dim_canvas / zoom
/// zoom > 0.0 -> zooming in
/// zoom < 0.0 -> zooming out
#[derive(Clone, Default)]
pub struct Camera {
    pos: Vec2,
    pos_pixelsnapped: Vec2,
    dim_frustum: Vec2,
    dim_canvas: Vec2,
    zoom_level: f32,
    z_near: f32,
    z_far: f32,
    is_centered: bool,
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
            pos_pixelsnapped: worldpoint_pixel_snapped(pos),
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
            self.pos + 0.5 * self.dim_frustum
        } else {
            self.pos
        }
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
    pub fn bounds(&mut self) -> Rect {
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
        self.pos_pixelsnapped = worldpoint_pixel_snapped(self.pos);
    }

    /// Pans the camera using cursor movement distance on the canvas
    #[inline]
    pub fn pan(&mut self, canvas_move_distance: Canvasvec) {
        self.pos -= canvas_move_distance / self.zoom_level;
        self.pos_pixelsnapped = worldpoint_pixel_snapped(self.pos);
    }

    #[inline]
    pub fn set_pos(&mut self, worldpoint: Worldpoint) {
        self.pos = worldpoint;
        self.pos_pixelsnapped = worldpoint_pixel_snapped(self.pos);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Game Camera
//

#[derive(Clone)]
pub struct GameCamera {
    pub cam: Camera,
    pub pos: Vec2,
    pub screenshake_offset: Vec2,
    pub screenshakers: Vec<ModulatorScreenShake>,
}

impl GameCamera {
    pub fn new(pos: Vec2, canvas_width: f32, canvas_height: f32) -> GameCamera {
        let cam = Camera::new(
            pos,
            1.0,
            canvas_width as u32,
            canvas_height as u32,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
            false,
        );

        GameCamera {
            cam,
            pos,
            screenshake_offset: Vec2::zero(),
            screenshakers: Vec::new(),
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
    }

    #[inline]
    pub fn center(&mut self) -> Worldpoint {
        self.sync_pos_internal();
        self.cam.center()
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

#[derive(Debug, Default, Clone, Copy)]
pub struct Cursors {
    pub mouse_coords: CursorCoords,
    pub finger_coords: [CursorCoords; TOUCH_MAX_FINGER_COUNT],
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
        let mouse_coords = CursorCoords::new(
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

        let mut finger_coords = [CursorCoords::default(); TOUCH_MAX_FINGER_COUNT];
        for (finger, coord) in finger_coords.iter_mut().enumerate() {
            *coord = CursorCoords::new(
                camera,
                screen_width,
                screen_height,
                canvas_width,
                canvas_height,
                touch.fingers[finger].pos_x,
                touch.fingers[finger].pos_y,
                touch.fingers[finger].delta_x,
                touch.fingers[finger].delta_y,
            )
        }

        Cursors {
            mouse_coords,
            finger_coords,
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
        camera.cam.pan(mouse_coords.delta_canvas);
    }
    if mouse.has_wheel_event {
        if mouse.wheel_delta > 0 {
            let new_zoom_level = f32::min(camera.cam.zoom_level * 2.0, 8.0);
            camera
                .cam
                .zoom_to_world_point(mouse_coords.pos_world, new_zoom_level);
        } else if mouse.wheel_delta < 0 {
            let new_zoom_level = f32::max(camera.cam.zoom_level / 2.0, 1.0 / 32.0);
            camera
                .cam
                .zoom_to_world_point(mouse_coords.pos_world, new_zoom_level);
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
    out_systemcommands: &mut Vec<SystemCommand>,
) {
    draw.set_clear_color_and_depth(config.color_clear, DEPTH_CLEAR);

    if config.has_canvas {
        draw.change_canvas_dimensions(config.canvas_width, config.canvas_height);
        draw.set_letterbox_color(config.canvas_color_letterbox);

        out_systemcommands.push(SystemCommand::WindowedModeAllow(config.windowed_mode_allow));
        if config.windowed_mode_allow {
            out_systemcommands.push(SystemCommand::WindowedModeAllowResizing(
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

            out_systemcommands.push(SystemCommand::WindowedModeSetSize {
                width: window_width,
                height: window_height,
                minimum_width: config.canvas_width,
                minimum_height: config.canvas_height,
            });
        }
    }

    out_systemcommands.push(SystemCommand::ScreenSetGrabInput(config.grab_input));
}

pub fn game_handle_system_keys(
    keyboard: &KeyboardState,
    out_systemcommands: &mut Vec<SystemCommand>,
) {
    if keyboard.recently_pressed(Scancode::Escape) {
        out_systemcommands.push(SystemCommand::Shutdown);
    }
    if keyboard.recently_pressed(Scancode::Return)
        && (keyboard.is_down(Scancode::LAlt) || keyboard.is_down(Scancode::RAlt))
    {
        out_systemcommands.push(SystemCommand::FullscreenToggle);
    }
    if keyboard.recently_pressed(Scancode::F8) {
        out_systemcommands.push(SystemCommand::Restart);
    }
}

pub fn game_debug_save_sprite_as_png(draw: &Drawstate, sprite: SpriteBy, filepath: &str) {
    let sprite_bitmap = draw.debug_get_sprite_as_bitmap(sprite);
    Bitmap::write_to_png_file(&sprite_bitmap, filepath);
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

type TimerCountdown = TimerSimple;

#[derive(Debug, Clone, Copy)]
pub struct TriggerRepeating {
    timer: Timer,
    triggertime_initial: f32,
    triggertime_repeating: f32,
}

impl Default for TriggerRepeating {
    fn default() -> Self {
        TriggerRepeating::new(1.0)
    }
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
    pub fn update(&mut self, deltatime: f32) -> bool {
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
    pub timer: TimerCountdown,
    pub state: Fadestate,
}

impl Fader {
    pub fn new_faded_out() -> Fader {
        Fader {
            timer: TimerCountdown::new_stopped(1.0),
            state: Fadestate::FadedOut,
        }
    }
    pub fn new_faded_in() -> Fader {
        Fader {
            timer: TimerCountdown::new_stopped(1.0),
            state: Fadestate::FadedIn,
        }
    }

    pub fn start_fading_out(&mut self, fade_out_time: f32) {
        self.state = Fadestate::FadingOut;
        self.timer = TimerCountdown::new_started(fade_out_time);
    }

    pub fn start_fading_in(&mut self, fade_in_time: f32) {
        self.state = Fadestate::FadingIn;
        self.timer = TimerCountdown::new_started(fade_in_time);
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
pub struct ScreenFader {
    pub color_start: Color,
    pub color_end: Color,
    pub timer: TimerCountdown,
}

impl ScreenFader {
    pub fn new(color_start: Color, color_end: Color, fade_time_seconds: f32) -> ScreenFader {
        ScreenFader {
            color_start,
            color_end,
            timer: TimerCountdown::new_started(fade_time_seconds),
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
                DEPTH_SCREEN_FADER,
                color,
                ADDITIVITY_NONE,
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
    fader: ScreenFader,
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
            fader: ScreenFader::new(Color::black(), Color::white(), time_fade_in),
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
        draw.draw_sprite_pixel_snapped(
            SpriteBy::Ref(&self.sprite),
            Vec2::new(splash_rect.left() as f32, splash_rect.top() as f32),
            Vec2::ones(),
            Vec2::unit_x(),
            false,
            false,
            DEPTH_SPLASH,
            opacity * Color::white(),
            ADDITIVITY_NONE,
        );

        for letterbox_rect in &letterbox_rects {
            draw.draw_rect(
                Rect::from(*letterbox_rect),
                DEPTH_SCREEN_FADER,
                opacity * Color::white(),
                ADDITIVITY_NONE,
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
                        ScreenFader::new(Color::white(), Color::transparent(), self.time_fade_out);
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

const ANIMATION_MAX_FRAME_COUNT: usize = 32;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct AnimationFrame {
    pub duration_seconds: f32,
    pub value: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub frame_count: usize,
    pub frames: [AnimationFrame; ANIMATION_MAX_FRAME_COUNT],
}

impl Animation {
    pub fn new_empty(name: &str) -> Animation {
        Animation {
            name: name.to_owned(),
            frame_count: 0,
            frames: [AnimationFrame {
                duration_seconds: 0.0,
                value: 0.0,
            }; ANIMATION_MAX_FRAME_COUNT],
        }
    }

    pub fn add_frame(&mut self, duration_seconds: f32, value: f32) {
        assert!(self.frame_count < ANIMATION_MAX_FRAME_COUNT);
        self.frames[self.frame_count].duration_seconds = duration_seconds;
        self.frames[self.frame_count].value = value;
        self.frame_count += 1;
    }
}

#[derive(Clone)]
pub struct AnimationPlayer {
    current_frametime: f32,
    current_frame_index: usize,
    playback_speed: f32,
    stop_after_finish: bool,
    play_reversed: bool,
    is_paused: bool,
    animation: Animation,
}

impl AnimationPlayer {
    pub fn new(anim: Animation, playback_speed: f32, stop_after_finished: bool) -> AnimationPlayer {
        AnimationPlayer {
            current_frame_index: 0,
            current_frametime: 0.0,
            playback_speed: playback_speed,
            stop_after_finish: stop_after_finished,
            play_reversed: false,
            is_paused: false,
            animation: anim,
        }
    }

    pub fn new_reversed(
        anim: &Animation,
        playback_speed: f32,
        stop_after_finished: bool,
    ) -> AnimationPlayer {
        let last_frame_index = anim.frame_count - 1;
        let current_frame_index = last_frame_index;
        let current_frametime = anim.frames[last_frame_index].duration_seconds;

        AnimationPlayer {
            current_frame_index,
            current_frametime,
            playback_speed: playback_speed,
            stop_after_finish: stop_after_finished,
            play_reversed: true,
            is_paused: false,
            animation: anim.clone(),
        }
    }

    pub fn finished_playing(&self) -> bool {
        if !self.stop_after_finish {
            return false;
        }

        if self.play_reversed {
            if self.current_frame_index > 0 {
                return false;
            }
            return self.current_frametime == 0.0;
        } else {
            if self.current_frame_index < self.animation.frame_count - 1 {
                return false;
            }

            let max_frametime = self.animation.frames[self.current_frame_index].duration_seconds;
            return self.current_frametime == max_frametime;
        }
    }

    pub fn update(&mut self, deltatime: f32) {
        if self.is_paused {
            return;
        }

        if self.play_reversed {
            self.current_frametime -= deltatime * self.playback_speed;

            if self.current_frametime < 0.0 {
                // NOTE: We need to cast to i32 here because we want to handle the negative case
                let next_frameindex: i32 = (self.current_frame_index as i32) - 1;
                if next_frameindex >= 0 {
                    let next_frameindex = next_frameindex as usize;
                    let frametime = self.animation.frames[next_frameindex].duration_seconds;
                    self.current_frame_index = next_frameindex;
                    self.current_frametime += frametime;
                } else {
                    // We reached the end of our animation
                    if self.stop_after_finish {
                        // Just let it stay zero so `animationplayer_is_finished` returns
                        // correct values
                        self.current_frametime = 0.0;
                    } else {
                        let next_frameindex = self.animation.frame_count - 1;
                        let frametime = self.animation.frames[next_frameindex].duration_seconds;
                        self.current_frame_index = next_frameindex;
                        self.current_frametime -= frametime;
                    }
                }
            }
        } else {
            let max_frametime = self.animation.frames[self.current_frame_index].duration_seconds;
            self.current_frametime += deltatime * self.playback_speed;

            if self.current_frametime > max_frametime {
                let framecount = self.animation.frame_count;
                let next_frameindex = self.current_frame_index + 1;
                if next_frameindex < framecount {
                    self.current_frame_index = next_frameindex;
                    self.current_frametime -= max_frametime;
                } else {
                    // We reached the end of our animation
                    if self.stop_after_finish {
                        // Just let it stay `max_frametime` so `animationplayer_is_finished` returns
                        // correct values
                        self.current_frametime = max_frametime;
                    } else {
                        self.current_frame_index = 0;
                        self.current_frametime -= max_frametime;
                    }
                }
            }
        }
    }

    pub fn value_fixed_for_percentage(&self, percentage: f32) -> f32 {
        let frame_index = floori(percentage * self.animation.frame_count as f32);
        self.animation.frames[frame_index as usize].value
    }

    pub fn value_current(&self) -> f32 {
        self.animation.frames[self.current_frame_index].value
    }

    pub fn value_current_interpolated_linear(&self) -> f32 {
        let max_frametime = self.animation.frames[self.current_frame_index].duration_seconds;
        let frametime_percent = self.current_frametime / max_frametime;

        let current_index = self.current_frame_index;
        let next_index = (current_index + 1) % self.animation.frame_count;

        let value_start = self.animation.frames[current_index].value;
        let value_end = self.animation.frames[next_index].value;

        lerp(value_start, value_end, frametime_percent)
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
                Vec2::zero(),
                worldpoint_pixel_snapped(self.pos[index]),
                Vec2::filled(scale),
                Vec2::unit_x(),
                depth,
                color,
                additivity,
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

    sprite: Vec<SpriteIndex>,
    age: Vec<f32>,
    pos: Vec<Vec2>,
    scale: Vec<Vec2>,
    rotation_dir: Vec<Vec2>,
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
            pos: Vec::with_capacity(count_max),
            scale: Vec::with_capacity(count_max),
            rotation_dir: Vec::with_capacity(count_max),
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
        newimage_sprite: SpriteIndex,
        newimage_pos: Vec2,
        newimage_scale: Vec2,
        newimage_rotation_dir: Vec2,
        newimage_flip_horizontally: bool,
        newimage_flip_vertically: bool,
        newimage_color_modulate: Color,
        newimage_additivity: f32,
    ) {
        self.time_since_last_spawn += deltatime;

        if self.count_max > 0 {
            let time_between_spawns = self.lifetime / self.count_max as f32;
            if self.pos.len() < self.count_max && self.time_since_last_spawn >= time_between_spawns
            {
                self.time_since_last_spawn -= time_between_spawns;

                self.sprite.push(newimage_sprite);
                self.age.push(0.0);
                self.pos.push(newimage_pos);
                self.scale.push(newimage_scale);
                self.rotation_dir.push(newimage_rotation_dir);
                self.flip_horizontally.push(newimage_flip_horizontally);
                self.flip_vertically.push(newimage_flip_vertically);
                self.color_modulate.push(newimage_color_modulate);
                self.additivity.push(newimage_additivity);
            }
        }
    }

    pub fn update_and_draw(&mut self, draw: &mut Drawstate, deltatime: f32, draw_depth: f32) {
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

            draw.draw_sprite_pixel_snapped(
                SpriteBy::Index(self.sprite[index]),
                self.pos[index],
                self.scale[index],
                self.rotation_dir[index],
                self.flip_horizontally[index],
                self.flip_vertically[index],
                draw_depth,
                color * self.color_modulate[index],
                additivity * self.additivity[index],
            );
        }

        for index in (0..self.pos.len()).rev() {
            self.age[index] += deltatime;
            if self.age[index] > self.lifetime {
                self.sprite.swap_remove(index);
                self.age.swap_remove(index);
                self.pos.swap_remove(index);
                self.scale.swap_remove(index);
                self.rotation_dir.swap_remove(index);
                self.flip_horizontally.swap_remove(index);
                self.flip_vertically.swap_remove(index);
                self.color_modulate.swap_remove(index);
                self.additivity.swap_remove(index);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Asset loading

pub fn game_load_atlas(assets_folder: &str) -> SpriteAtlas {
    let textures_list_filepath = system::path_join(assets_folder, "atlas.data");
    let textures_list: Vec<String> =
        bincode::deserialize(&std::fs::read(&textures_list_filepath).expect(&format!(
            "Could not read '{}' - Gamedata corrupt?",
            &textures_list_filepath
        )))
        .expect(&format!(
            "Could not deserialize '{}' - Gamedata corrupt?",
            &textures_list_filepath
        ));
    let mut textures = Vec::new();
    for texture_filepath_relative in &textures_list {
        let texture_filepath = system::path_join(assets_folder, texture_filepath_relative);
        textures.push(Bitmap::create_from_png_file(&texture_filepath));
    }

    let sprites_filepath = system::path_join(assets_folder, "sprites.data");
    let sprites = bincode::deserialize(&std::fs::read(&sprites_filepath).expect(&format!(
        "Could not read '{}' - Gamedata corrupt?",
        sprites_filepath
    )))
    .expect(&format!(
        "Could not deserialize '{}' - Gamedata corrupt?",
        sprites_filepath
    ));

    SpriteAtlas::new(textures, sprites)
}

pub fn game_load_fonts(assets_folder: &str) -> HashMap<String, SpriteFont> {
    let fonts_filepath = system::path_join(assets_folder, "fonts.data");
    let fonts = bincode::deserialize(&std::fs::read(&fonts_filepath).expect(&format!(
        "Could not read '{}' - Gamedata corrupt?",
        fonts_filepath
    )))
    .expect(&format!(
        "Could not deserialize '{}' - Gamedata corrupt?",
        fonts_filepath
    ));

    fonts
}

pub fn game_load_animations(assets_folder: &str) -> HashMap<String, Animation> {
    let animations_filepath = system::path_join(assets_folder, "animations.data");
    let animations = bincode::deserialize(&std::fs::read(&animations_filepath).expect(&format!(
        "Could not read '{}' - Gamedata corrupt?",
        animations_filepath
    )))
    .expect(&format!(
        "Could not deserialize '{}' - Gamedata corrupt?",
        animations_filepath
    ));
    animations
}

pub fn game_load_audiorecordings_mono(assets_folder: &str) -> HashMap<String, Vec<AudioSample>> {
    let mut audiorecordings = HashMap::new();

    let wav_filepaths = system::collect_files_by_extension_recursive(assets_folder, ".wav");
    for wav_filepath in &wav_filepaths {
        let mut wav_file = audrey::open(&wav_filepath).expect(&format!(
            "Could not open audio file for reading: '{}'",
            wav_filepath
        ));
        let wav_buffer: Vec<AudioSample> = wav_file.samples().map(Result::unwrap).collect();
        let recording_name = system::path_to_filename_without_extension(wav_filepath);
        audiorecordings.insert(recording_name, wav_buffer);
    }

    audiorecordings
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Scene Management

#[derive(Clone)]
pub struct Globals {
    pub random: Random,
    pub camera: GameCamera,
    pub cursors: Cursors,

    pub deltatime_speed_factor: f32,
    pub deltatime: f32,
    pub is_paused: bool,

    pub canvas_width: f32,
    pub canvas_height: f32,

    pub font_default: SpriteFont,
    pub font_default_no_border: SpriteFont,
}

pub trait Scene: Clone {
    fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        assets: &mut GameAssets,
        input: &GameInput,
        globals: &mut Globals,
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug Scene

#[derive(Clone)]
pub struct SceneDebug {
    glitter: ParticleSystem,
    music_stream_id: AudioStreamId,

    measure_completion_ratio_values: Vec<f32>,
    last_measure_completion_ratio: f32,

    choreographer_randoms: Choreographer,
    choreographer_tween: Choreographer,
    choreographer_conversation: Choreographer,
    choreographer_rectangles: Choreographer,
    choreographer_hp_front: Choreographer,
    choreographer_hp_back: Choreographer,
    choreographer_hp_refill: Choreographer,

    loaded_font_name: String,

    hp: f32,
    hp_previous: f32,

    circle_radius: f32,
}

impl SceneDebug {
    pub fn new(
        _draw: &mut Drawstate,
        _audio: &mut Audiostate,
        _assets: &mut GameAssets,
        _input: &GameInput,
        loaded_font_name: &str,
    ) -> SceneDebug {
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

        SceneDebug {
            glitter: ParticleSystem::new(glitter_params, 30, Vec2::zero()),

            music_stream_id: 0,

            measure_completion_ratio_values: Vec::new(),
            last_measure_completion_ratio: 0.0,

            choreographer_randoms: Choreographer::new(),
            choreographer_tween: Choreographer::new(),
            choreographer_conversation: Choreographer::new(),
            choreographer_rectangles: Choreographer::new(),
            choreographer_hp_front: Choreographer::new(),
            choreographer_hp_back: Choreographer::new(),
            choreographer_hp_refill: Choreographer::new(),

            loaded_font_name: loaded_font_name.to_owned(),

            hp: 1.0,
            hp_previous: 1.0,

            circle_radius: 50.0,
        }
    }
}

impl Scene for SceneDebug {
    fn update_and_draw(
        &mut self,
        draw: &mut Drawstate,
        audio: &mut Audiostate,
        _assets: &mut GameAssets,
        input: &GameInput,
        globals: &mut Globals,
    ) {
        const DEPTH_DRAW: Depth = 20.0;

        let deltatime = globals.deltatime;

        if self.music_stream_id == 0 {
            self.music_stream_id = audio.play(
                "loop_bell",
                SchedulePlay::OnNextMeasure {
                    beats_per_minute: 120,
                    beats_per_measure: 4,
                },
                true,
                0.1,
                0.0,
            );
        }

        draw.draw_rect(
            Rect::from_width_height(globals.canvas_width, globals.canvas_height),
            DEPTH_DRAW,
            Color::greyscale(0.5),
            ADDITIVITY_NONE,
        );

        let center = Vec2::new(globals.canvas_width, globals.canvas_height) / 2.0;

        // CONVERSATION
        //
        self.choreographer_conversation.update(deltatime);
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
                draw.debug_log_color(*color, line);

                if !finished {
                    return;
                }
            }
        })();

        // CIRCLES
        self.choreographer_tween.update(input.deltatime);
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
            DEPTH_DRAW,
            Color::white(),
            ADDITIVITY_NONE,
        );

        draw.draw_ring(
            circle_pos - Vec2::filled(100.0),
            60.0,
            10.0,
            DEPTH_DRAW,
            Color::white(),
            ADDITIVITY_NONE,
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
        self.choreographer_rectangles.update(input.deltatime);
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
        draw.draw_rect(rect1, DEPTH_DRAW, Color::white(), ADDITIVITY_NONE);
        draw.draw_rect(rect2, DEPTH_DRAW, Color::white(), ADDITIVITY_NONE);

        // HP BAR
        //
        if input.keyboard.recently_pressed(Scancode::D) {
            audio.play_oneshot(
                "drum",
                SchedulePlay::OnNextQuarterBeat {
                    beats_per_minute: 140,
                },
                0.7,
                0.0,
            );

            self.hp_previous = self.hp;
            self.hp -= globals.random.f32_in_range_closed(0.15, 0.3);
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

        self.choreographer_hp_refill.update(input.deltatime);
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

        self.choreographer_hp_front.update(input.deltatime);
        (|| {
            let (percentage, finished) = self.choreographer_hp_front.tween(0.3);
            let percentage = easing::cubic_inout(percentage);
            hp_front_value = lerp(self.hp_previous, self.hp, percentage);
            if !finished {
                return;
            }
        })();
        self.choreographer_hp_back.update(input.deltatime);
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
            &globals.font_default,
            1.0,
            hp_rect_initial.pos,
            Vec2::filled_y(-5.0),
            true,
            DEPTH_DRAW,
            Color::white(),
            ADDITIVITY_NONE,
        );
        draw.draw_rect(
            hp_back_rect,
            DEPTH_DRAW,
            Color::from_hex_rgba(0x884242ff),
            ADDITIVITY_NONE,
        );
        draw.draw_rect(
            hp_front_rect,
            DEPTH_DRAW,
            Color::from_hex_rgba(0xf06969ff),
            ADDITIVITY_NONE,
        );

        // PRINTING RANDOM NUMBERS
        //
        self.choreographer_randoms.update(input.deltatime);
        (|| {
            for index in 0..10 {
                if !self.choreographer_randoms.wait(0.5) {
                    return;
                }

                if self.choreographer_randoms.once() {
                    println!("Random number {}: {}", index, globals.random.next_u64());
                }
            }
        })();

        let measure_completion_ratio = audio
            .stream_completion_ratio(self.music_stream_id)
            .unwrap_or(0.0);
        let beat_completion_ratio = (4.0 * measure_completion_ratio) % 1.0;

        draw.draw_pixel(
            globals.cursors.mouse_coords.pos_world,
            DEPTH_DEBUG,
            Color::magenta(),
            ADDITIVITY_NONE,
        );

        self.glitter.move_to(globals.cursors.mouse_coords.pos_world);
        self.glitter
            .update_and_draw(draw, &mut globals.random, deltatime, DEPTH_DRAW);

        draw.draw_rect(
            Rect::from_xy_width_height(5.0, 220.0, beat_completion_ratio * 30.0, 5.0),
            DEPTH_DEBUG,
            Color::magenta(),
            ADDITIVITY_NONE,
        );
        draw.draw_rect(
            Rect::from_xy_width_height(5.0, 225.0, measure_completion_ratio * 30.0, 5.0),
            DEPTH_DEBUG,
            Color::blue(),
            ADDITIVITY_NONE,
        );

        draw.draw_rect(
            Rect::from_xy_width_height(
                0.0,
                globals.canvas_height - 10.0,
                measure_completion_ratio * globals.canvas_width,
                10.0,
            ),
            DEPTH_DEBUG,
            Color::blue(),
            ADDITIVITY_NONE,
        );

        // Text drawing test
        let test_font = draw.get_font(&self.loaded_font_name);
        let text = "Loaded font test gorgeous!|\u{08A8}";
        let text_width = test_font.get_text_width(1, text);
        // Draw origin is top-left
        let draw_pos = Vec2::new(5.0, globals.canvas_height - 40.0);
        draw.draw_text(
            text,
            &test_font,
            1.0,
            draw_pos,
            Vec2::zero(),
            false,
            20.0,
            Color::magenta(),
            ADDITIVITY_NONE,
        );
        draw.draw_line_bresenham(
            draw_pos + Vec2::new(0.0, test_font.baseline as f32),
            draw_pos + Vec2::new(text_width as f32, test_font.baseline as f32),
            20.0,
            0.3 * Color::yellow(),
            ADDITIVITY_NONE,
        );
        // Draw origin is baseline
        let draw_pos = Vec2::new(5.0, globals.canvas_height - 15.0);
        draw.draw_text(
            text,
            &test_font,
            1.0,
            draw_pos,
            Vec2::zero(),
            true,
            20.0,
            Color::magenta(),
            ADDITIVITY_NONE,
        );
        draw.draw_line_bresenham(
            draw_pos,
            draw_pos + Vec2::new(text_width as f32, 0.0),
            20.0,
            0.3 * Color::yellow(),
            ADDITIVITY_NONE,
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
            random.f32_in_range_closed(0.03, 0.05)
        };

        if !choreo.wait(pause_time) {
            return (line_accumulator, false);
        }
    }

    (line_accumulator, true)
}
