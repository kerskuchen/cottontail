pub mod wasm_audio;
mod wasm_input;

pub use wasm_audio as audio;

use crate::{
    input::{FingerPlatformId, InputState},
    AppCommand, AppContextInterface,
};

use super::renderer_opengl::Renderer;

use ct_lib_core::log;
use ct_lib_core::*;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use std::{cell::RefCell, rc::Rc};

const ENABLE_FRAMETIME_LOGGING: bool = false;

fn html_get_window() -> &'static web_sys::Window {
    static mut WINDOW: Option<web_sys::Window> = None;
    unsafe {
        if let Some(window) = WINDOW.as_ref() {
            window
        } else {
            WINDOW = Some(web_sys::window().expect("no global `window` exists"));
            WINDOW.as_ref().unwrap()
        }
    }
}

fn html_get_screen() -> &'static web_sys::Screen {
    static mut SCREEN: Option<web_sys::Screen> = None;
    unsafe {
        if let Some(screen) = SCREEN.as_ref() {
            screen
        } else {
            SCREEN = Some(
                html_get_window()
                    .screen()
                    .expect("Could not get screen handle"),
            );
            SCREEN.as_ref().unwrap()
        }
    }
}

fn html_get_document() -> &'static web_sys::Document {
    static mut DOCUMENT: Option<web_sys::Document> = None;
    unsafe {
        if let Some(document) = DOCUMENT.as_ref() {
            document
        } else {
            DOCUMENT = Some(
                html_get_window()
                    .document()
                    .expect("should have a document on window"),
            );
            DOCUMENT.as_ref().unwrap()
        }
    }
}

fn html_get_canvas() -> &'static web_sys::HtmlCanvasElement {
    static mut CANVAS: Option<web_sys::HtmlCanvasElement> = None;
    unsafe {
        if let Some(canvas) = CANVAS.as_ref() {
            canvas
        } else {
            let canvas = html_get_document()
                .get_element_by_id("canvas")
                .expect("HTML element 'canvas' not found");
            CANVAS = Some(
                canvas
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .expect("'canvas' is not a HTML Canvas element"),
            );
            CANVAS.as_ref().unwrap()
        }
    }
}

fn html_request_animation_frame(f: &Closure<dyn FnMut()>) {
    html_get_window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

struct FullscreenHandler {
    fullscreen_requested: Rc<RefCell<bool>>,
    preferred_screen_orientation: Option<web_sys::OrientationLockType>,
}

impl FullscreenHandler {
    fn new(
        preferred_screen_orientation: Option<web_sys::OrientationLockType>,
    ) -> FullscreenHandler {
        let fullscreen_requested = Rc::new(RefCell::new(false));

        // Key up
        {
            let fullscreen_requested = fullscreen_requested.clone();
            let keyup_callback = Closure::wrap(Box::new(move |_event: web_sys::KeyboardEvent| {
                let mut fullscreen_requested = fullscreen_requested.borrow_mut();
                if *fullscreen_requested {
                    FullscreenHandler::activate_fullscreen(preferred_screen_orientation);
                    *fullscreen_requested = false;
                }
            }) as Box<dyn FnMut(_)>);
            html_get_document()
                .add_event_listener_with_callback("keyup", keyup_callback.as_ref().unchecked_ref())
                .expect("Cannot register 'keyup' callback for fullscreen mode");
            keyup_callback.forget();
        }
        // Mouse up
        {
            let fullscreen_requested = fullscreen_requested.clone();
            let mouseup_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                let mut fullscreen_requested = fullscreen_requested.borrow_mut();
                if *fullscreen_requested {
                    FullscreenHandler::activate_fullscreen(preferred_screen_orientation);
                    *fullscreen_requested = false;
                }
            }) as Box<dyn FnMut(_)>);
            html_get_document()
                .add_event_listener_with_callback(
                    "mouseup",
                    mouseup_callback.as_ref().unchecked_ref(),
                )
                .expect("Cannot register 'mouseup' callback for fullscreen mode");
            mouseup_callback.forget();
        }
        // Touch end
        {
            let fullscreen_requested = fullscreen_requested.clone();
            let touchup_callback = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
                let mut fullscreen_requested = fullscreen_requested.borrow_mut();
                if *fullscreen_requested {
                    FullscreenHandler::activate_fullscreen(preferred_screen_orientation);
                    *fullscreen_requested = false;
                }
            }) as Box<dyn FnMut(_)>);
            html_get_document()
                .add_event_listener_with_callback(
                    "touchend",
                    touchup_callback.as_ref().unchecked_ref(),
                )
                .expect("Cannot register 'touchend' callback for fullscreen mode");
            touchup_callback.forget();
        }

        FullscreenHandler {
            fullscreen_requested,
            preferred_screen_orientation,
        }
    }

    pub fn is_fullscreen_mode_active() -> bool {
        html_get_document().fullscreen_element().is_some()
    }

    // NOTE: Can be true i.e. if user themself pressed F11 on the desktop browser
    pub fn is_window_covering_fullscreen() -> bool {
        let window = html_get_window();
        let screen = html_get_screen();
        let window_width = window
            .inner_width()
            .expect("Cannot determine window inner width")
            .as_f64()
            .expect("Window inner width has wrong type") as i32;
        let window_height = window
            .inner_height()
            .expect("Cannot determine window inner width")
            .as_f64()
            .expect("Window inner height has wrong type") as i32;
        let screen_width = screen.width().expect("Could not get screen width");
        let screen_height = screen.height().expect("Could not get screen width");
        screen_width == window_width && screen_height == window_height
    }

    pub fn toggle_fullscreen(&mut self) {
        if !FullscreenHandler::is_fullscreen_programmatically_toggleable() {
            return;
        }

        if FullscreenHandler::is_fullscreen_mode_active() {
            html_get_document().exit_fullscreen();
            if self.preferred_screen_orientation.is_some() {
                // NOTE: This promise produces an exception on devices that don't support screen
                //       orientation change. This is a little annoying but doesn't break anything
                //       so we leave it be due to code complexity reasons
                let _promis = html_get_screen()
                    .orientation()
                    .unlock()
                    .expect("Failed to unlock screen orientation");
            }
        } else {
            *self.fullscreen_requested.borrow_mut() = true;
        }
    }

    // Based on https://www.rossis.red/wasm.html
    fn is_fullscreen_programmatically_toggleable() -> bool {
        let can_fullscreen_be_enabled = html_get_document().fullscreen_enabled();
        let is_fullscreen_active = FullscreenHandler::is_fullscreen_mode_active();
        let has_fullsize_window_already = FullscreenHandler::is_window_covering_fullscreen();

        can_fullscreen_be_enabled && (is_fullscreen_active || !has_fullsize_window_already)
    }

    fn activate_fullscreen(orientation_type: Option<web_sys::OrientationLockType>) {
        if !FullscreenHandler::is_fullscreen_mode_active() {
            html_get_document()
                .document_element()
                .expect("Failed to get document element")
                .request_fullscreen()
                .expect("Failed to enter fullscreen");
            if let Some(orientation_type) = orientation_type {
                // NOTE: This promise produces an exception on devices that don't support screen
                //       orientation change. This is a little annoying but doesn't break anything
                //       so we leave it be due to code complexity reasons
                let _promise = html_get_screen()
                    .orientation()
                    .lock(orientation_type)
                    .expect("Failed to lock screen orientation");
            }
        }
    }
}

fn log_frametimes(_duration_frame: f64, _duration_input: f64, _duration_update: f64) {
    if ENABLE_FRAMETIME_LOGGING {
        log::trace!(
            "frame: {:.3}ms  input: {:.3}ms  update: {:.3}ms",
            _duration_frame * 1000.0,
            _duration_input * 1000.0,
            _duration_update * 1000.0,
        );
    }
}

pub fn run_main<AppContextType: 'static + AppContextInterface>() -> Result<(), JsValue> {
    init_logging("", log::Level::Trace).unwrap();
    log::info!("Starting up...");

    timer_initialize();

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // AUDIO

    let mut audio = wasm_audio::AudioOutput::new();

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // WEBGL

    let webgl = html_get_canvas()
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<web_sys::WebGlRenderingContext>()?;
    let glow_context = glow::Context::from_webgl1_context(webgl);
    let mut renderer = Renderer::new(glow_context);

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // MAINLOOP

    // ---------------------------------------------------------------------------------------------
    // Mainloop setup

    let mut appcommands: Vec<AppCommand> = Vec::new();
    let app_start_time = timer_current_time_seconds();
    let mut frame_start_time = app_start_time;
    log::debug!("Startup took {:.3}ms", app_start_time * 1000.0,);

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // INPUT CALLBACKS

    let dpr = html_get_window().device_pixel_ratio();
    let input = Rc::new(RefCell::new(InputState::new()));
    let mut mouse_pos_previous_x = 0;
    let mut mouse_pos_previous_y = 0;

    // Key down
    {
        let input = input.clone();
        let keydown_callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut input = input.borrow_mut();

            input.keyboard.has_press_event = true;
            if event.repeat() {
                input.keyboard.has_system_repeat_event = true;
            }
            let scancode = wasm_input::scancode_to_our_scancode(&event.code());
            let keycode = wasm_input::keycode_to_our_keycode(&event.key(), scancode);
            input.keyboard.process_key_press_event(scancode, keycode);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "keydown",
            keydown_callback.as_ref().unchecked_ref(),
        )?;
        keydown_callback.forget();
    }
    // Key up
    {
        let input = input.clone();
        let keyup_callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut input = input.borrow_mut();

            input.keyboard.has_release_event = true;
            let scancode = wasm_input::scancode_to_our_scancode(&event.code());
            let keycode = wasm_input::keycode_to_our_keycode(&event.key(), scancode);
            input.keyboard.process_key_release_event(scancode, keycode);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("keyup", keyup_callback.as_ref().unchecked_ref())?;
        keyup_callback.forget();
    }
    // Mouse down
    {
        let input = input.clone();
        let mousedown_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if event.button() >= 3 {
                // We only support three buttons
                return;
            }

            let mut input = input.borrow_mut();
            input.mouse.has_press_event = true;
            input.mouse.pos_x = (event.offset_x() as f64 * dpr).floor() as i32;
            input.mouse.pos_y = (event.offset_y() as f64 * dpr).floor() as i32;
            match event.button() {
                0 => input.mouse.button_left.process_press_event(),
                1 => input.mouse.button_middle.process_press_event(),
                2 => input.mouse.button_right.process_press_event(),
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousedown",
            mousedown_callback.as_ref().unchecked_ref(),
        )?;
        mousedown_callback.forget();
    }
    // Mouse up
    {
        let input = input.clone();
        let mouseup_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if event.button() >= 3 {
                // We only support three buttons
                return;
            }

            let mut input = input.borrow_mut();
            input.mouse.has_release_event = true;
            input.mouse.pos_x = (event.offset_x() as f64 * dpr).floor() as i32;
            input.mouse.pos_y = (event.offset_y() as f64 * dpr).floor() as i32;
            match event.button() {
                0 => input.mouse.button_left.process_release_event(),
                1 => input.mouse.button_middle.process_release_event(),
                2 => input.mouse.button_right.process_release_event(),
                _ => {}
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mouseup",
            mouseup_callback.as_ref().unchecked_ref(),
        )?;
        mouseup_callback.forget();
    }
    // Mouse move
    {
        let input = input.clone();
        let mousemove_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut input = input.borrow_mut();

            input.mouse.has_moved = true;
            input.mouse.pos_x = (event.offset_x() as f64 * dpr).floor() as i32;
            input.mouse.pos_y = (event.offset_y() as f64 * dpr).floor() as i32;
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousemove",
            mousemove_callback.as_ref().unchecked_ref(),
        )?;
        mousemove_callback.forget();
    }
    // Mouse wheel
    {
        let input = input.clone();
        let wheel_callback = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
            let mut input = input.borrow_mut();

            input.mouse.has_wheel_event = true;
            input.mouse.wheel_delta = event.delta_y() as i32;
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("mouseup", wheel_callback.as_ref().unchecked_ref())?;
        wheel_callback.forget();
    }
    // Touch start
    {
        let input = input.clone();
        let touchstart_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            event.prevent_default();
            let html_canvas = html_get_canvas();
            // NOTE: Because we do `prevent_default` on the event (because we don't want mouseclicks
            //       to be simulated), we need to manually focus our canvas
            html_canvas
                .focus()
                .unwrap_or_else(|error| panic!("Cannot focus on canvas: {}", error));
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut input = input.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                    let pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;
                    input.touch.process_finger_down(
                        touch.identifier() as FingerPlatformId,
                        pos_x,
                        pos_y,
                    )
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchstart",
            touchstart_callback.as_ref().unchecked_ref(),
        )?;
        touchstart_callback.forget();
    }
    // Touch up
    {
        let input = input.clone();
        let touchend_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut input = input.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                    let pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;
                    input.touch.process_finger_up(
                        touch.identifier() as FingerPlatformId,
                        pos_x,
                        pos_y,
                    )
                }
            }
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchend",
            touchend_callback.as_ref().unchecked_ref(),
        )?;
        touchend_callback.forget();
    }
    // Touch move
    {
        let input = input.clone();
        let touchmove_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut input = input.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                    let pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;
                    input.touch.process_finger_move(
                        touch.identifier() as FingerPlatformId,
                        pos_x,
                        pos_y,
                    )
                }
            }
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchmove",
            touchmove_callback.as_ref().unchecked_ref(),
        )?;
        touchmove_callback.forget();
    }
    // Touch cancel
    {
        let input = input.clone();
        let touchcancel_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut input = input.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                    let pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;
                    input.touch.process_finger_up(
                        touch.identifier() as FingerPlatformId,
                        pos_x,
                        pos_y,
                    )
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchcancel",
            touchcancel_callback.as_ref().unchecked_ref(),
        )?;
        touchcancel_callback.forget();
    }
    // Focus
    {
        let input = input.clone();
        let focus_callback = Closure::wrap(Box::new(move |_event: web_sys::FocusEvent| {
            let mut input = input.borrow_mut();
            input.has_focus = true;
            input.has_focus_event = true;
            log::debug!("Gained input focus");
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("focus", focus_callback.as_ref().unchecked_ref())?;
        focus_callback.forget();
    }
    // Unfocus
    {
        let input = input.clone();
        let blur_callback = Closure::wrap(Box::new(move |_event: web_sys::FocusEvent| {
            let mut input = input.borrow_mut();
            input.has_focus = false;
            input.has_focus_event = true;
            log::debug!("Lost input focus");
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("blur", blur_callback.as_ref().unchecked_ref())?;
        blur_callback.forget();
    }

    let mut fullscreen_handler =
        FullscreenHandler::new(Some(web_sys::OrientationLockType::Landscape));

    let mut app_context = AppContextType::new(&mut renderer, &input.borrow(), &mut audio);

    // Here we want to call `requestAnimationFrame` in a loop, but only a fixed
    // number of times. After it's done we want all our resources cleaned up. To
    // achieve this we're using an `Rc`. The `Rc` will eventually store the
    // closure we want to execute on each frame, but to start out it contains
    // `None`.
    //
    // After the `Rc` is made we'll actually create the closure, and the closure
    // will reference one of the `Rc` instances. The other `Rc` reference is
    // used to store the closure, request the first frame, and then is dropped
    // by this function.
    //
    // Inside the closure we've got a persistent `Rc` reference, which we use
    // for all future iterations of the loop
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let pre_input_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Event loop

        // resize canvas if necessary
        {
            let mut input = input.borrow_mut();
            input.screen_is_fullscreen = FullscreenHandler::is_fullscreen_mode_active()
                || FullscreenHandler::is_window_covering_fullscreen();

            let html_canvas = html_get_canvas();
            let window_width = (html_canvas.client_width() as f64 * dpr).round();
            let window_height = (html_canvas.client_height() as f64 * dpr).round();
            let canvas_width = html_canvas.width();
            let canvas_height = html_canvas.height();
            if canvas_width as i32 != window_width as i32
                || canvas_height as i32 != window_height as i32
            {
                assert!(window_width >= 0.0);
                assert!(window_height >= 0.0);
                html_canvas.set_width(window_width as u32);
                html_canvas.set_height(window_height as u32);

                input.screen_framebuffer_width = window_width as u32;
                input.screen_framebuffer_height = window_height as u32;
                input.screen_framebuffer_dimensions_changed = true;
            }
            renderer.update_screen_dimensions(
                input.screen_framebuffer_width,
                input.screen_framebuffer_height,
            );
        }
        // Mouse x in [0, screen_framebuffer_width - 1]  (left to right)
        // Mouse y in [0, screen_framebuffer_height - 1] (top to bottom)
        //
        // NOTE: We get the mouse position and delta from querying SDL instead of accumulating
        //       events, as it is faster, more accurate and less error-prone
        {
            let mut input = input.borrow_mut();
            input.touch.calculate_move_deltas();
            input.mouse.delta_x = input.mouse.pos_x - mouse_pos_previous_x;
            input.mouse.delta_y = input.mouse.pos_y - mouse_pos_previous_y;
        }

        let post_input_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Timings, update and drawing

        let pre_update_time = post_input_time;

        let duration_frame = pre_update_time - frame_start_time;
        frame_start_time = pre_update_time;

        {
            let mut input = input.borrow_mut();
            input.deltatime =
                super::snap_deltatime_to_nearest_common_refresh_rate(duration_frame as f32);
            input.real_world_uptime = frame_start_time;
            input.audio_playback_rate_hz = audio.audio_playback_rate_hz;
        }
        app_context.run_tick(&mut renderer, &input.borrow(), &mut audio, &mut appcommands);
        {
            let mut input = input.borrow_mut();
            // Clear input state
            input.screen_framebuffer_dimensions_changed = false;
            input.has_foreground_event = false;
            input.has_focus_event = false;

            input.keyboard.clear_transitions();
            input.mouse.clear_transitions();
            input.touch.clear_transitions();

            mouse_pos_previous_x = input.mouse.pos_x;
            mouse_pos_previous_y = input.mouse.pos_y;

            if input.textinput.is_textinput_enabled {
                // Reset textinput
                input.textinput.has_new_textinput_event = false;
                input.textinput.has_new_composition_event = false;
                input.textinput.inputtext.clear();
                input.textinput.composition_text.clear();
            }
        }

        //--------------------------------------------------------------------------------------
        // System commands

        for command in &appcommands {
            match command {
                AppCommand::FullscreenToggle => {
                    fullscreen_handler.toggle_fullscreen();
                }
                AppCommand::TextinputStart {
                    inputrect_x: _,
                    inputrect_y: _,
                    inputrect_width: _,
                    inputrect_height: _,
                } => {
                    todo!();
                }
                AppCommand::TextinputStop => {
                    todo!();
                }
                AppCommand::WindowedModeAllowResizing(_allowed) => {
                    log::trace!("`WindowedModeAllowResizing` Not available on this platform");
                }
                AppCommand::WindowedModeAllow(_allowed) => {
                    log::trace!("`WindowedModeAllow` Not available on this platform");
                }
                AppCommand::WindowedModeSetSize {
                    width: _,
                    height: _,
                    minimum_width: _,
                    minimum_height: _,
                } => {
                    log::trace!("`WindowedModeSetSize` Not available on this platform");
                }
                AppCommand::ScreenSetGrabInput(_grab_input) => {
                    todo!()
                }
                AppCommand::Shutdown => {
                    log::trace!("`Shutdown` Not available on this platform");
                }
                AppCommand::Restart => {
                    log::trace!("`Restart` Not available on this platform");
                }
            }
        }
        appcommands.clear();

        let post_update_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Debug timing output

        let duration_input = post_input_time - pre_input_time;
        let duration_update = post_update_time - pre_update_time;

        log_frametimes(duration_frame, duration_input, duration_update);
        // Schedule ourself for another requestAnimationFrame callback.
        html_request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    html_request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}
