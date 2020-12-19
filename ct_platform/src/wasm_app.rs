mod renderer_opengl;

use ct_lib::{
    game::{GameInput, GameMemory, GameStateInterface, Scancode, SystemCommand},
    platform::{current_time_seconds, init_logging},
};

use std::{cell::RefCell, rc::Rc};

use renderer_opengl::Renderer;

pub use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlProgram, WebGlRenderingContext, WebGlShader};

const ENABLE_PANIC_MESSAGES: bool = false;
const ENABLE_FRAMETIME_LOGGING: bool = true;

fn html_get_window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn html_request_animation_frame(f: &Closure<dyn FnMut()>) {
    html_get_window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn html_get_document() -> web_sys::Document {
    html_get_window()
        .document()
        .expect("should have a document on window")
}

fn html_get_canvas() -> web_sys::HtmlCanvasElement {
    let canvas = html_get_document()
        .get_element_by_id("canvas")
        .expect("HTML element 'canvas' not found");
    canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("'canvas' is not a HTML Canvas element")
}

fn fullscreen_is_enabled() -> bool {
    html_get_document().fullscreen()
}

fn fullscreen_set_enabled(orientation_type: Option<web_sys::OrientationLockType>) {
    if !fullscreen_is_enabled() {
        html_get_canvas()
            .request_fullscreen()
            .expect("Failed to enter fullscreen");
        if let Some(orientation_type) = orientation_type {
            let _promise = html_get_window()
                .screen()
                .expect("Could not get screen handle")
                .orientation()
                .lock(orientation_type)
                .expect("Failed to lock screen orientation");
        }
    }
}

fn fullscreen_set_disabled() {
    if fullscreen_is_enabled() {
        html_get_document().exit_fullscreen();
        html_get_window()
            .screen()
            .expect("Could not get screen handle")
            .orientation()
            .unlock()
            .expect("Failed to lock screen orientation");
    }
}

fn fullscreen_toggle(orientation_type: Option<web_sys::OrientationLockType>) {
    if fullscreen_is_enabled() {
        fullscreen_set_disabled();
    } else {
        fullscreen_set_enabled(orientation_type);
    }
}

fn log_frametimes(
    _duration_frame: f64,
    _duration_input: f64,
    _duration_update: f64,
    _duration_sound: f64,
    _duration_render: f64,
) {
    if ENABLE_FRAMETIME_LOGGING {
        log::trace!(
            "frame: {:.3}ms  input: {:.3}ms  update: {:.3}ms  sound: {:.3}ms  render: {:.3}ms",
            _duration_frame * 1000.0,
            _duration_input * 1000.0,
            _duration_update * 1000.0,
            _duration_sound * 1000.0,
            _duration_render * 1000.0,
        );
    }
}

pub fn run_main<GameStateType: 'static + GameStateInterface + Clone>() -> Result<(), JsValue> {
    init_logging("", log::Level::Trace).unwrap();
    log::info!("Starting up...");

    let launcher_start_time = current_time_seconds();

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // AUDIO

    const AUDIO_SAMPLE_RATE: usize = 44100;
    const AUDIO_BUFFER_FRAME_COUNT: usize = 2048;
    const AUDIO_NUM_CHANNELS: usize = 2;

    let mut audio_options = web_sys::AudioContextOptions::new();
    audio_options.sample_rate(AUDIO_SAMPLE_RATE as f32);

    let audio_ctx = Rc::new(RefCell::new(
        web_sys::AudioContext::new_with_context_options(&audio_options)
            .expect("WebAudio not available"),
    ));
    let audio_processor = audio_ctx.borrow().create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(AUDIO_BUFFER_FRAME_COUNT as u32, 0, AUDIO_NUM_CHANNELS as u32)
        .expect("Could not create AudioProcessor node");
    {
        let mut interleaved_output = vec![0f32; AUDIO_NUM_CHANNELS * AUDIO_BUFFER_FRAME_COUNT];
        let mut channel_output = vec![0f32; AUDIO_BUFFER_FRAME_COUNT];
        let mut frame_index = 0;
        let closure = Closure::wrap(Box::new(move |event: web_sys::AudioProcessingEvent| {
            let output_buffer = event.output_buffer().unwrap();
            let num_frames = output_buffer.length() as usize;
            let num_channels = output_buffer.number_of_channels() as usize;

            assert!(num_frames == AUDIO_BUFFER_FRAME_COUNT);
            assert!(num_channels == AUDIO_NUM_CHANNELS);

            for frame in interleaved_output.chunks_exact_mut(2) {
                frame[0] = 0.1
                    * f64::sin(
                        2.0 * std::f64::consts::PI
                            * 440.0
                            * (frame_index as f64 / AUDIO_SAMPLE_RATE as f64),
                    ) as f32;
                frame[1] = 0.1
                    * f64::sin(
                        2.0 * std::f64::consts::PI
                            * 440.0
                            * (frame_index as f64 / AUDIO_SAMPLE_RATE as f64),
                    ) as f32;
                frame_index += 1;
            }

            for channel in 0..num_channels {
                for frame in 0..num_frames {
                    channel_output[frame] = interleaved_output[num_channels * frame + channel];
                }
                output_buffer
                    .copy_to_channel(&mut channel_output, channel as i32)
                    .expect("Unable to write sample data into the audio context buffer");
            }
        }) as Box<dyn FnMut(_)>);
        audio_processor.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }
    audio_processor
        .connect_with_audio_node(&audio_ctx.borrow().destination())
        .expect("Could not connect AudioScriptProcessor node");

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // WEBGL

    let webgl = html_get_canvas()
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;
    let glow_context = glow::Context::from_webgl1_context(webgl);
    let mut renderer = Renderer::new(glow_context);

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // MAINLOOP

    // ---------------------------------------------------------------------------------------------
    // Game memory and input

    let mut game_memory = GameMemory::<GameStateType>::default();

    // ---------------------------------------------------------------------------------------------
    // Mainloop setup

    let mut systemcommands: Vec<SystemCommand> = Vec::new();

    let game_start_time = current_time_seconds();
    let mut frame_start_time = game_start_time;
    let duration_startup = game_start_time - launcher_start_time;
    log::debug!("Startup took {:.3}ms", duration_startup * 1000.0,);

    let mut current_tick = 0;

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // INPUT CALLBACKS

    const SCREEN_ORIENTATION: web_sys::OrientationLockType =
        web_sys::OrientationLockType::Landscape;

    let dpr = html_get_window().device_pixel_ratio();
    let input = Rc::new(RefCell::new(GameInput::new()));
    let mut mouse_pos_previous_x = 0;
    let mut mouse_pos_previous_y = 0;

    // Mouse down
    {
        let input = input.clone();
        let audio_ctx = audio_ctx.clone();
        let mousedown_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            // Need handle fullscreen change here because of browser UX limitations
            // fullscreen_toggle(Some(SCREEN_ORIENTATION));

            // Need enable audio here because of browser UX limitations
            let audio_ctx = audio_ctx.borrow();
            if audio_ctx.state() == web_sys::AudioContextState::Suspended {
                audio_ctx.resume().ok();
            }

            if event.button() >= 3 {
                // We only support three buttons
                return;
            }

            let mut input = input.borrow_mut();
            input.mouse.has_press_event = true;
            input.mouse.pos_x = (event.offset_x() as f64 * dpr).floor() as i32;
            input.mouse.pos_y = (event.offset_y() as f64 * dpr).floor() as i32;
            match event.button() {
                0 => input
                    .mouse
                    .button_left
                    .process_event(true, false, current_tick),
                1 => input
                    .mouse
                    .button_middle
                    .process_event(true, false, current_tick),
                2 => input
                    .mouse
                    .button_right
                    .process_event(true, false, current_tick),
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
                0 => input
                    .mouse
                    .button_left
                    .process_event(false, false, current_tick),
                1 => input
                    .mouse
                    .button_middle
                    .process_event(false, false, current_tick),
                2 => input
                    .mouse
                    .button_right
                    .process_event(false, false, current_tick),
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
        let audio_ctx = audio_ctx.clone();
        let canvas = html_get_canvas();
        let touchstart_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let offset_x = canvas.get_bounding_client_rect().left();
            let offset_y = canvas.get_bounding_client_rect().top();

            let mut input = input.borrow_mut();
            let audio_ctx = audio_ctx.borrow();
            for finger_id in 0..event.target_touches().length() {
                if let Some(touch) = event.target_touches().item(finger_id) {
                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as u32 {
                        input.touch.has_press_event = true;

                        let finger = &mut input.touch.fingers[finger_id as usize];

                        // IMPORTANT: At this point we may have an out of date screen dimensions
                        //            if the window size changed since last frame.
                        finger.pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                        finger.pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;

                        // NOTE: We don't want fake deltas when pressing. This can happen when our
                        //       last release was not the same as our press position.
                        finger.delta_x = 0;
                        finger.delta_y = 0;

                        finger.state.process_event(true, false, current_tick);
                    }
                }
            }

            // Need enable audio here because of browser UX limitations
            if audio_ctx.state() == web_sys::AudioContextState::Suspended {
                audio_ctx.resume().ok();
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
        let canvas = html_get_canvas();
        let touchend_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let offset_x = canvas.get_bounding_client_rect().left();
            let offset_y = canvas.get_bounding_client_rect().top();

            let mut input = input.borrow_mut();
            for finger_id in 0..event.target_touches().length() {
                if let Some(touch) = event.target_touches().item(finger_id) {
                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as u32 {
                        input.touch.has_release_event = true;
                        let finger_previous_pos_x =
                            input.touch.fingers_previous[finger_id as usize].pos_x;
                        let finger_previous_pos_y =
                            input.touch.fingers_previous[finger_id as usize].pos_y;

                        let finger = &mut input.touch.fingers[finger_id as usize];

                        // IMPORTANT: At this point we may have an out of date screen dimensions
                        //            if the window size changed since last frame.
                        finger.pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                        finger.pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;

                        finger.delta_x = finger.pos_x - finger_previous_pos_x;
                        finger.delta_y = finger.pos_y - finger_previous_pos_y;

                        finger.state.process_event(false, false, current_tick);
                    }
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
        let input = input.clone();
        let canvas = html_get_canvas();
        let touchmove_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let offset_x = canvas.get_bounding_client_rect().left();
            let offset_y = canvas.get_bounding_client_rect().top();

            let mut input = input.borrow_mut();
            for finger_id in 0..event.target_touches().length() {
                if let Some(touch) = event.target_touches().item(finger_id) {
                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as u32 {
                        input.touch.has_move_event = true;
                        let finger_previous_pos_x =
                            input.touch.fingers_previous[finger_id as usize].pos_x;
                        let finger_previous_pos_y =
                            input.touch.fingers_previous[finger_id as usize].pos_y;

                        let finger = &mut input.touch.fingers[finger_id as usize];

                        // IMPORTANT: At this point we may have an out of date screen dimensions
                        //            if the window size changed since last frame.
                        finger.pos_x = ((touch.client_x() as f64 - offset_x) * dpr).floor() as i32;
                        finger.pos_y = ((touch.client_y() as f64 - offset_y) * dpr).floor() as i32;

                        finger.delta_x = finger.pos_x - finger_previous_pos_x;
                        finger.delta_y = finger.pos_y - finger_previous_pos_y;
                    }
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
        let input = input.clone();
        let touchcancel_callback = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
            let mut input = input.borrow_mut();
            for finger_id in 0..ct_lib::game::TOUCH_MAX_FINGER_COUNT {
                let finger = &mut input.touch.fingers[finger_id];
                finger.state.process_event(false, false, current_tick);
                input.touch.has_release_event = true;
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchcancel",
            touchcancel_callback.as_ref().unchecked_ref(),
        )?;
        touchcancel_callback.forget();
    }

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
        let pre_input_time = current_time_seconds();

        current_tick += 1;

        //--------------------------------------------------------------------------------------
        // System commands

        for command in &systemcommands {
            match command {
                SystemCommand::FullscreenToggle => {
                    todo!();
                }
                SystemCommand::FullscreenEnable(enabled) => {
                    todo!();
                }
                SystemCommand::TextinputStart {
                    inputrect_x,
                    inputrect_y,
                    inputrect_width,
                    inputrect_height,
                } => {
                    todo!();
                }
                SystemCommand::TextinputStop => {
                    todo!();
                }
                SystemCommand::WindowedModeAllowResizing(allowed) => {
                    log::trace!("`WindowedModeAllowResizing` Not available on this platform");
                }
                SystemCommand::WindowedModeAllow(allowed) => {
                    log::trace!("`WindowedModeAllow` Not available on this platform");
                }
                SystemCommand::WindowedModeSetSize {
                    width,
                    height,
                    minimum_width,
                    minimum_height,
                } => {
                    log::trace!("`WindowedModeSetSize` Not available on this platform");
                }
                SystemCommand::ScreenSetGrabInput(grab_input) => {
                    let TODO = true;
                }
                SystemCommand::Shutdown => {
                    log::trace!("`Shutdown` Not available on this platform");
                }
                SystemCommand::Restart => {
                    log::trace!("`Restart` Not available on this platform");
                }
            }
        }
        systemcommands.clear();

        //--------------------------------------------------------------------------------------
        // Event loop

        // resize canvas
        {
            let window_width = (html_get_canvas().client_width() as f64 * dpr).round();
            let window_height = (html_get_canvas().client_height() as f64 * dpr).round();
            let canvas_width = html_get_canvas().width();
            let canvas_height = html_get_canvas().height();
            if canvas_width as i32 != window_width as i32
                || canvas_height as i32 != window_height as i32
            {
                assert!(window_width >= 0.0);
                assert!(window_height >= 0.0);
                html_get_canvas().set_width(window_width as u32);
                html_get_canvas().set_height(window_height as u32);

                let mut input = input.borrow_mut();
                input.screen_framebuffer_width = window_width as u32;
                input.screen_framebuffer_height = window_height as u32;
                input.screen_framebuffer_dimensions_changed = true;
            }
        }
        // Mouse x in [0, screen_framebuffer_width - 1]  (left to right)
        // Mouse y in [0, screen_framebuffer_height - 1] (top to bottom)
        //
        // NOTE: We get the mouse position and delta from querying SDL instead of accumulating
        //       events, as it is faster, more accurate and less error-prone
        {
            let mut input = input.borrow_mut();
            input.mouse.delta_x = input.mouse.pos_x - mouse_pos_previous_x;
            input.mouse.delta_y = input.mouse.pos_y - mouse_pos_previous_y;
        }

        let post_input_time = current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Timings, update and drawing

        let pre_update_time = post_input_time;

        let duration_frame = pre_update_time - frame_start_time;
        frame_start_time = pre_update_time;

        {
            let mut input = input.borrow_mut();
            input.deltatime = duration_frame as f32;
            input.target_deltatime = f32::min(duration_frame as f32, 1.0 / 30.0);
            input.real_world_uptime = frame_start_time - launcher_start_time;
            input.audio_playback_rate_hz = AUDIO_SAMPLE_RATE;

            game_memory.update(&input, &mut systemcommands);

            // Clear input state
            input.screen_framebuffer_dimensions_changed = false;
            input.has_foreground_event = false;
            input.has_focus_event = false;

            input.keyboard.clear_transitions();
            input.mouse.clear_transitions();
            input.touch.touchstate_clear_transitions();

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

        let post_update_time = current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Sound output

        let pre_sound_time = post_update_time;

        if game_memory.audio.is_some() {
            let audio = game_memory
                .audio
                .as_mut()
                .expect("No audiostate initialized");
            // audio_output.render_frames(audio, input.has_focus, 2.0 * target_seconds_per_frame);
        }

        let post_sound_time = current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Drawcommands

        let pre_render_time = post_sound_time;

        let TODO = "make it so that draw is always there and can handle loading its sounds later";
        if game_memory.draw.is_some() {
            let input = input.borrow();
            renderer.process_drawcommands(
                input.screen_framebuffer_width,
                input.screen_framebuffer_height,
                &game_memory
                    .draw
                    .as_ref()
                    .expect("No drawstate initialized")
                    .drawcommands,
            );
        }

        let post_render_time = current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Debug timing output

        let duration_input = post_input_time - pre_input_time;
        let duration_update = post_update_time - pre_update_time;
        let duration_sound = post_sound_time - pre_sound_time;
        let duration_render = post_render_time - pre_render_time;

        log_frametimes(
            duration_frame,
            duration_input,
            duration_update,
            duration_sound,
            duration_render,
        );
        // Schedule ourself for another requestAnimationFrame callback.
        html_request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    html_request_animation_frame(g.borrow().as_ref().unwrap());
    Ok(())
}

pub fn compile_shader(
    gl: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    gl: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    gl.attach_shader(&program, vert_shader);
    gl.attach_shader(&program, frag_shader);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
