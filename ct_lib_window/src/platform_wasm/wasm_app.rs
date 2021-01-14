pub mod wasm_audio;
mod wasm_input;

pub use wasm_audio as audio;

use crate::{input::FingerPlatformId, AppEventHandler, MouseButton, PlatformWindowCommand};

use super::renderer_opengl::Renderer;

use ct_lib_core::log;
use ct_lib_core::*;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use std::{cell::RefCell, rc::Rc};

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
            html_get_canvas()
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
            html_get_canvas()
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
            html_get_canvas()
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
                let _promise = html_get_screen()
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

static mut PLATFORM_WINDOW_COMMANDS: Vec<PlatformWindowCommand> = Vec::new();

pub fn run_main<AppEventHandlerType: AppEventHandler + 'static>(
    appcontext: AppEventHandlerType,
) -> Result<(), JsValue> {
    init_logging("", log::Level::Trace).unwrap();
    log::info!("Starting up...");

    timer_initialize();

    let app = Rc::new(RefCell::new(appcontext));

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
    // INPUT CALLBACKS

    let device_pixel_ratio = html_get_window().device_pixel_ratio();
    let prevent_mouse_input_for_n_frames = Rc::new(RefCell::new(0));

    let mut fullscreen_handler =
        FullscreenHandler::new(Some(web_sys::OrientationLockType::Landscape));

    // Key down
    {
        let appcontext = app.clone();
        let keydown_callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let scancode = wasm_input::scancode_to_our_scancode(&event.code());
            let keycode = wasm_input::keycode_to_our_keycode(&event.key(), scancode);
            let is_repeat = event.repeat();
            appcontext
                .borrow_mut()
                .handle_key_press(scancode, keycode, is_repeat);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "keydown",
            keydown_callback.as_ref().unchecked_ref(),
        )?;
        keydown_callback.forget();
    }
    // Key up
    {
        let appcontext = app.clone();
        let keyup_callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let scancode = wasm_input::scancode_to_our_scancode(&event.code());
            let keycode = wasm_input::keycode_to_our_keycode(&event.key(), scancode);
            appcontext
                .borrow_mut()
                .handle_key_release(scancode, keycode)
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("keyup", keyup_callback.as_ref().unchecked_ref())?;
        keyup_callback.forget();
    }
    // Mouse down
    {
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let mousedown_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if *prevent_mouse_input_for_n_frames.borrow() != 0 {
                // We are currently using touch input exclusively
                return;
            }

            if event.button() >= 5 {
                // We only support five buttons
                return;
            }

            let x = (event.offset_x() as f64 * device_pixel_ratio).floor() as i32;
            let y = (event.offset_y() as f64 * device_pixel_ratio).floor() as i32;
            let button = match event.button() {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                3 => MouseButton::X1,
                4 => MouseButton::X2,
                _ => unreachable!(),
            };
            appcontext.borrow_mut().handle_mouse_press(button, x, y);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousedown",
            mousedown_callback.as_ref().unchecked_ref(),
        )?;
        mousedown_callback.forget();
    }
    // Mouse up
    {
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let mouseup_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if *prevent_mouse_input_for_n_frames.borrow() != 0 {
                // We are currently using touch input exclusively
                return;
            }

            if event.button() >= 5 {
                // We only support five buttons
                return;
            }

            let x = (event.offset_x() as f64 * device_pixel_ratio).floor() as i32;
            let y = (event.offset_y() as f64 * device_pixel_ratio).floor() as i32;
            let button = match event.button() {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                3 => MouseButton::X1,
                4 => MouseButton::X2,
                _ => unreachable!(),
            };
            appcontext.borrow_mut().handle_mouse_release(button, x, y);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mouseup",
            mouseup_callback.as_ref().unchecked_ref(),
        )?;
        mouseup_callback.forget();
    }
    // Mouse move
    {
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let mousemove_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if *prevent_mouse_input_for_n_frames.borrow() != 0 {
                // We are currently using touch input exclusively
                return;
            }

            let x = (event.offset_x() as f64 * device_pixel_ratio).floor() as i32;
            let y = (event.offset_y() as f64 * device_pixel_ratio).floor() as i32;
            appcontext.borrow_mut().handle_mouse_move(x, y);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousemove",
            mousemove_callback.as_ref().unchecked_ref(),
        )?;
        mousemove_callback.forget();
    }
    // Mouse wheel
    {
        let appcontext = app.clone();
        let wheel_callback = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
            let scroll_delta = event.delta_y() as i32;
            appcontext
                .borrow_mut()
                .handle_mouse_wheel_scroll(scroll_delta);
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("mouseup", wheel_callback.as_ref().unchecked_ref())?;
        wheel_callback.forget();
    }
    // Touch start
    {
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let touchstart_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            // Make touch input exclusive for a while
            *prevent_mouse_input_for_n_frames.borrow_mut() = 120;

            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut appcontext = appcontext.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let finger_id = touch.identifier() as FingerPlatformId;
                    let x =
                        ((touch.client_x() as f64 - offset_x) * device_pixel_ratio).floor() as i32;
                    let y =
                        ((touch.client_y() as f64 - offset_y) * device_pixel_ratio).floor() as i32;
                    appcontext.handle_touch_press(finger_id, x, y)
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
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let touchend_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            // Make touch input exclusive for a while
            *prevent_mouse_input_for_n_frames.borrow_mut() = 120;

            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut appcontext = appcontext.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let finger_id = touch.identifier() as FingerPlatformId;
                    let x =
                        ((touch.client_x() as f64 - offset_x) * device_pixel_ratio).floor() as i32;
                    let y =
                        ((touch.client_y() as f64 - offset_y) * device_pixel_ratio).floor() as i32;
                    appcontext.handle_touch_release(finger_id, x, y)
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchend",
            touchend_callback.as_ref().unchecked_ref(),
        )?;
        touchend_callback.forget();
    }
    // Touch move
    {
        let appcontext = app.clone();
        let prevent_mouse_input_for_n_frames = prevent_mouse_input_for_n_frames.clone();
        let touchmove_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            // Make touch input exclusive for a while
            *prevent_mouse_input_for_n_frames.borrow_mut() = 120;

            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut appcontext = appcontext.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let finger_id = touch.identifier() as FingerPlatformId;
                    let x =
                        ((touch.client_x() as f64 - offset_x) * device_pixel_ratio).floor() as i32;
                    let y =
                        ((touch.client_y() as f64 - offset_y) * device_pixel_ratio).floor() as i32;
                    appcontext.handle_touch_move(finger_id, x, y)
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchmove",
            touchmove_callback.as_ref().unchecked_ref(),
        )?;
        touchmove_callback.forget();
    }
    // Touch cancel
    {
        let appcontext = app.clone();
        let touchcancel_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let html_canvas = html_get_canvas();
            let offset_x = html_canvas.get_bounding_client_rect().left();
            let offset_y = html_canvas.get_bounding_client_rect().top();
            let mut appcontext = appcontext.borrow_mut();
            for index in 0..event.changed_touches().length() {
                if let Some(touch) = event.changed_touches().item(index) {
                    let finger_id = touch.identifier() as FingerPlatformId;
                    let x =
                        ((touch.client_x() as f64 - offset_x) * device_pixel_ratio).floor() as i32;
                    let y =
                        ((touch.client_y() as f64 - offset_y) * device_pixel_ratio).floor() as i32;
                    appcontext.handle_touch_release(finger_id, x, y)
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
        let appcontext = app.clone();
        let focus_callback = Closure::wrap(Box::new(move |_event: web_sys::FocusEvent| {
            appcontext.borrow_mut().handle_window_focus_gained();
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("focus", focus_callback.as_ref().unchecked_ref())?;
        focus_callback.forget();
    }
    // Unfocus
    {
        let appcontext = app.clone();
        let blur_callback = Closure::wrap(Box::new(move |_event: web_sys::FocusEvent| {
            appcontext.borrow_mut().handle_window_focus_lost();
        }) as Box<dyn FnMut(_)>);
        html_get_canvas()
            .add_event_listener_with_callback("blur", blur_callback.as_ref().unchecked_ref())?;
        blur_callback.forget();
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // MAINLOOP

    let app_start_time = timer_current_time_seconds();
    let mut frame_start_time = app_start_time;
    log::debug!("Startup took {:.3}ms", app_start_time * 1000.0,);

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
        // Touch exclusive mode
        {
            let mut prevent_mouse_input_for_n_frames =
                prevent_mouse_input_for_n_frames.borrow_mut();
            if *prevent_mouse_input_for_n_frames > 0 {
                *prevent_mouse_input_for_n_frames -= 1;
            }
        }

        // resize canvas if necessary
        {
            let html_canvas = html_get_canvas();
            let window_width = (html_canvas.client_width() as f64 * device_pixel_ratio).round();
            let window_height = (html_canvas.client_height() as f64 * device_pixel_ratio).round();
            let canvas_width = html_canvas.width();
            let canvas_height = html_canvas.height();
            if canvas_width as i32 != window_width as i32
                || canvas_height as i32 != window_height as i32
            {
                assert!(window_width >= 0.0);
                assert!(window_height >= 0.0);
                html_canvas.set_width(window_width as u32);
                html_canvas.set_height(window_height as u32);

                let is_fullscreen = FullscreenHandler::is_fullscreen_mode_active()
                    || FullscreenHandler::is_window_covering_fullscreen();

                app.borrow_mut().handle_window_resize(
                    window_width as u32,
                    window_height as u32,
                    is_fullscreen,
                );
            }
            renderer.update_screen_dimensions(window_width as u32, window_height as u32);
        }

        //--------------------------------------------------------------------------------------
        // Tick

        let current_time = timer_current_time_seconds();
        let duration_frame = current_time - frame_start_time;
        frame_start_time = current_time;
        app.borrow_mut().run_tick(
            duration_frame as f32,
            current_time,
            &mut renderer,
            &mut audio,
        );

        //--------------------------------------------------------------------------------------
        // System commands

        unsafe {
            for command in PLATFORM_WINDOW_COMMANDS.drain(..) {
                match command {
                    PlatformWindowCommand::FullscreenToggle => {
                        fullscreen_handler.toggle_fullscreen();
                    }
                    PlatformWindowCommand::TextinputStart {
                        inputrect_x: _,
                        inputrect_y: _,
                        inputrect_width: _,
                        inputrect_height: _,
                    } => {
                        todo!();
                    }
                    PlatformWindowCommand::TextinputStop => {
                        todo!();
                    }
                    PlatformWindowCommand::WindowedModeAllowResizing(_allowed) => {
                        log::trace!("`WindowedModeAllowResizing` Not available on this platform");
                    }
                    PlatformWindowCommand::WindowedModeAllow(_allowed) => {
                        log::trace!("`WindowedModeAllow` Not available on this platform");
                    }
                    PlatformWindowCommand::WindowedModeSetSize {
                        width: _,
                        height: _,
                        minimum_width: _,
                        minimum_height: _,
                    } => {
                        log::trace!("`WindowedModeSetSize` Not available on this platform");
                    }
                    PlatformWindowCommand::ScreenSetGrabInput(_grab_input) => {
                        todo!()
                    }
                    PlatformWindowCommand::Shutdown => {
                        log::trace!("`Shutdown` Not available on this platform");
                    }
                    PlatformWindowCommand::Restart => {
                        log::trace!("`Restart` Not available on this platform");
                    }
                }
            }
        }

        // Schedule ourself for another requestAnimationFrame callback.
        html_request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    html_request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

#[inline]
pub fn add_platform_window_command(appcommand: PlatformWindowCommand) {
    unsafe { PLATFORM_WINDOW_COMMANDS.push(appcommand) }
}
