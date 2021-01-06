use crate::renderer_opengl::Renderer;
use ct_lib_core::log;

use std::collections::HashMap;

pub struct Window {
    _sdl_glcontext: sdl2::video::GLContext,
    pub sdl_window: sdl2::video::Window,
    sdl_video: sdl2::VideoSubsystem,

    pub fullscreen_active: bool,

    windowed_mode_allowed: bool,
    windowed_mode_resizing_allowed: bool,

    windowed_mode_pos_x: Option<i32>,
    windowed_mode_pos_y: Option<i32>,
    windowed_mode_width: Option<u32>,
    windowed_mode_height: Option<u32>,
    windowed_mode_minimum_width: Option<u32>,
    windowed_mode_minimum_height: Option<u32>,
}

impl Window {
    pub fn new(sdl_video: sdl2::VideoSubsystem, display_index: i32, window_title: &str) -> Window {
        // Collect display infos
        let display_info = sdl_get_display_info_for_index(&sdl_video, display_index)
            .unwrap_or_else(|| {
                log::warn!(
                    "Display with index {} does not exist - using main display as fallback",
                    display_index,
                );
                sdl_get_main_display_info(&sdl_video)
            });

        let gl_attr = sdl_video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        sdl2::hint::set_video_minimize_on_focus_loss(false);

        // Make window fullscreen borderless
        let sdl_window = sdl_video
            .window(window_title, display_info.width, display_info.height)
            .position(display_info.pos_x, display_info.pos_y)
            .borderless()
            .opengl()
            .build()
            .expect("Failed to create window");

        debug_assert_eq!(gl_attr.context_profile(), sdl2::video::GLProfile::Core);
        debug_assert_eq!(gl_attr.context_version(), (3, 3));

        let sdl_glcontext = sdl_window
            .gl_create_context()
            .expect("Could not create OpenGL context");
        sdl_window
            .gl_make_current(&sdl_glcontext)
            .expect("Could not make OpenGL context current");

        if let Err(error) = sdl_video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync) {
            log::warn!("Could not enable vsync: '{}'", error);
        }

        Window {
            _sdl_glcontext: sdl_glcontext,
            sdl_window,
            sdl_video,

            fullscreen_active: true,
            windowed_mode_allowed: false,
            windowed_mode_resizing_allowed: false,

            windowed_mode_pos_x: None,
            windowed_mode_pos_y: None,
            windowed_mode_width: None,
            windowed_mode_height: None,
            windowed_mode_minimum_width: None,
            windowed_mode_minimum_height: None,
        }
    }

    pub fn create_renderer(&self) -> Renderer {
        let context = unsafe {
            glow::Context::from_loader_function(|s| {
                self.sdl_video.gl_get_proc_address(s) as *const _
            })
        };
        Renderer::new(context)
    }

    pub fn show_error_messagebox(message: &str) {
        sdl2::messagebox::show_simple_message_box(
            sdl2::messagebox::MessageBoxFlag::ERROR,
            "Error",
            message,
            None,
        )
        .unwrap_or(());
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.sdl_window.drawable_size()
    }

    pub fn enable_fullscreen(&mut self) {
        assert!(!self.fullscreen_active);

        self.fullscreen_active = true;

        // Store previous window position and size
        let (pos_x, pos_y) = self.sdl_window.position();
        let (width, height) = self.sdl_window.size();
        self.windowed_mode_pos_x = Some(pos_x);
        self.windowed_mode_pos_y = Some(pos_y);
        self.windowed_mode_width = Some(width);
        self.windowed_mode_height = Some(height);

        self.set_resizable_internal(false);
        self.set_bordered_internal(false);

        let display_info = self.get_display_info_for_current_display();
        self.set_position_internal(display_info.pos_x, display_info.pos_y);
        self.set_size_internal(display_info.width, display_info.height)
    }

    pub fn disable_fullscreen(&mut self) {
        assert!(self.fullscreen_active);
        assert!(self.windowed_mode_allowed);

        self.fullscreen_active = false;

        self.set_resizable_internal(self.windowed_mode_resizing_allowed);
        self.set_bordered_internal(true);

        // Restore previous window position and size
        self.set_position_internal(
            self.windowed_mode_pos_x
                .expect("Window dimensions uninitialized"),
            self.windowed_mode_pos_y
                .expect("Window dimensions uninitialized"),
        );
        self.set_size_internal(
            self.windowed_mode_width
                .expect("Window dimensions uninitialized"),
            self.windowed_mode_height
                .expect("Window dimensions uninitialized"),
        );
        self.set_minimum_size_internal(
            self.windowed_mode_minimum_width
                .expect("Window dimensions uninitialized"),
            self.windowed_mode_minimum_height
                .expect("Window dimensions uninitialized"),
        );
    }

    pub fn set_windowed_mode_allowed(&mut self, allowed: bool) {
        self.windowed_mode_allowed = allowed;
    }

    pub fn set_input_grabbed(&mut self, grab_input: bool) {
        self.sdl_window.set_grab(grab_input);
    }

    pub fn set_windowed_mode_size(
        &mut self,
        width: u32,
        height: u32,
        minimum_width: u32,
        minimum_height: u32,
    ) {
        assert!(self.windowed_mode_allowed);

        let display_info = self.get_display_info_for_current_display();
        assert!(width > 0 && height > 0);
        assert!(minimum_width > 0 && minimum_height > 0);
        assert!(minimum_width <= width && minimum_height <= height);
        assert!(width < display_info.width && height < display_info.height);

        // Determine the position of the window for the case that it is centered on its display
        let pos_x = display_info.pos_x as u32 + (display_info.width / 2) - (width / 2);
        let pos_y = display_info.pos_y as u32 + (display_info.height / 2) - (height / 2);

        self.windowed_mode_pos_x = Some(pos_x as i32);
        self.windowed_mode_pos_y = Some(pos_y as i32);
        self.windowed_mode_width = Some(width);
        self.windowed_mode_height = Some(height);
        self.windowed_mode_minimum_width = Some(minimum_width);
        self.windowed_mode_minimum_height = Some(minimum_height);

        self.set_minimum_size_internal(
            self.windowed_mode_minimum_width.unwrap(),
            self.windowed_mode_minimum_height.unwrap(),
        );

        if !self.fullscreen_active {
            self.set_position_internal(
                self.windowed_mode_pos_x.unwrap(),
                self.windowed_mode_pos_y.unwrap(),
            );
            self.set_size_internal(
                self.windowed_mode_width.unwrap(),
                self.windowed_mode_height.unwrap(),
            );
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        if self.fullscreen_active {
            self.disable_fullscreen();
        } else {
            self.enable_fullscreen();
        }
    }

    pub fn windowed_mode_set_resizable(&mut self, resizable: bool) {
        assert!(self.windowed_mode_allowed);

        self.windowed_mode_resizing_allowed = resizable;
        if !self.fullscreen_active {
            self.set_resizable_internal(resizable);
        }
    }

    fn get_display_info_for_current_display(&self) -> DisplayInfo {
        let display_index = self.get_display_index_internal();
        let display_infos = sdl_collect_display_infos(&self.sdl_video);

        if display_infos.contains_key(&display_index) {
            *display_infos.get(&display_index).unwrap()
        } else {
            log::warn!(
                "Display with index {} does not exist - using main display as fallback",
                display_index,
            );
            sdl_get_main_display_info(&self.sdl_video)
        }
    }

    fn set_minimum_size_internal(&mut self, minimum_width: u32, minimum_height: u32) {
        self.sdl_window
            .set_minimum_size(minimum_width, minimum_height)
            .expect("Failed to set a minimum size on window");
    }

    fn set_position_internal(&mut self, x: i32, y: i32) {
        self.sdl_window.set_position(
            sdl2::video::WindowPos::Positioned(x),
            sdl2::video::WindowPos::Positioned(y),
        )
    }

    fn set_size_internal(&mut self, width: u32, height: u32) {
        self.sdl_window
            .set_size(width, height)
            .expect(&format!("Cannot resize window to {}x{}", width, height));
    }

    fn set_resizable_internal(&mut self, resizable: bool) {
        unsafe {
            sdl2::sys::SDL_SetWindowResizable(
                self.sdl_window.raw(),
                if resizable {
                    sdl2::sys::SDL_bool::SDL_TRUE
                } else {
                    sdl2::sys::SDL_bool::SDL_FALSE
                },
            );
        }
    }

    fn set_bordered_internal(&mut self, bordered: bool) {
        unsafe {
            sdl2::sys::SDL_SetWindowBordered(
                self.sdl_window.raw(),
                if bordered {
                    sdl2::sys::SDL_bool::SDL_TRUE
                } else {
                    sdl2::sys::SDL_bool::SDL_FALSE
                },
            );
        }
    }

    fn get_display_index_internal(&self) -> i32 {
        self.sdl_window
            .display_index()
            .expect("Cannot determine display index of window")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Display info
#[derive(Clone, Copy)]
struct DisplayInfo {
    pub index: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

fn sdl_get_display_info_for_index(
    sdl_video: &sdl2::VideoSubsystem,
    display_index: i32,
) -> Option<DisplayInfo> {
    let query_result = sdl_video.desktop_display_mode(display_index);
    if let Err(error) = query_result {
        log::warn!(
            "Failed to determine resolution of display {} : {}",
            display_index,
            error
        );
        return None;
    }
    let display_mode = query_result.unwrap();

    let query_result = sdl_video.display_bounds(display_index);
    if let Err(error) = query_result {
        log::warn!(
            "Failed to determine display bounds of display {} : {}",
            display_index,
            error
        );
        return None;
    }
    let display_bounds = query_result.unwrap();

    if display_mode.w <= 0
        || display_mode.h <= 0
        || display_mode.refresh_rate <= 0
        || display_bounds.x < 0
        || display_bounds.y < 0
        || display_bounds.w <= 0
        || display_bounds.h <= 0
    {
        log::warn!("Failed to get display info for display {}", display_index);
        return None;
    }

    Some(DisplayInfo {
        index: display_index as u32,
        pos_x: display_bounds.x,
        pos_y: display_bounds.y,
        width: display_bounds.w as u32,
        height: display_bounds.h as u32,
        refresh_rate: display_mode.refresh_rate as u32,
    })
}

fn sdl_collect_display_infos(sdl_video: &sdl2::VideoSubsystem) -> HashMap<i32, DisplayInfo> {
    let display_count = sdl_video
        .num_video_displays()
        .expect("Cannot enumerate displays");
    assert!(display_count > 0, "Cannot enumerate displays");

    let mut result = HashMap::new();
    for display_index in 0..display_count {
        if let Some(display_info) = sdl_get_display_info_for_index(&sdl_video, display_index) {
            result.insert(display_index, display_info);
        }
    }
    if result.len() == 0 {
        panic!("No display found to draw on");
    }

    result
}

fn sdl_get_main_display_info(sdl_video: &sdl2::VideoSubsystem) -> DisplayInfo {
    let display_infos = sdl_collect_display_infos(sdl_video);
    let &min_display_index = display_infos.keys().min().unwrap();
    display_infos.get(&min_display_index).unwrap().clone()
}
