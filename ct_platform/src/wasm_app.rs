mod renderer_opengl;

use ct_lib::game::{GameInput, GameMemory, GameStateInterface, Scancode, SystemCommand};

use std::{cell::RefCell, rc::Rc};

use renderer_opengl::Renderer;

use console_error_panic_hook;
use log::Level;

pub use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlProgram, WebGlRenderingContext, WebGlShader};

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

struct Input {
    pressed: bool,
    pos: (i32, i32),
}
impl Input {
    fn new() -> Input {
        Input {
            pressed: false,
            pos: (0, 0),
        }
    }
}

pub fn run_main<GameStateType: GameStateInterface + Clone>() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    console_log::init_with_level(Level::Trace).expect("error initializing log");
    log::info!("Hello world!");

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // AUDIO

    const AUDIO_SAMPLE_RATE: u32 = 44100;
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
    // INPUT CALLBACKS

    const SCREEN_ORIENTATION: web_sys::OrientationLockType =
        web_sys::OrientationLockType::Landscape;

    let input = Rc::new(RefCell::new(Input::new()));
    // Mouse down
    {
        let input = input.clone();
        let audio_ctx = audio_ctx.clone();
        let mousedown_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut input = input.borrow_mut();
            let audio_ctx = audio_ctx.borrow();
            input.pressed = true;
            input.pos = (event.offset_x(), event.offset_y());

            // Need handle fullscreen change here because of browser UX limitations
            fullscreen_toggle(Some(SCREEN_ORIENTATION));

            // Need enable audio here because of browser UX limitations
            if audio_ctx.state() == web_sys::AudioContextState::Suspended {
                audio_ctx.resume().ok();
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousedown",
            mousedown_callback.as_ref().unchecked_ref(),
        )?;
        mousedown_callback.forget();
    }
    // Mouse move
    {
        let input = input.clone();
        let mousemove_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut input = input.borrow_mut();
            input.pos = (event.offset_x(), event.offset_y());
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mousemove",
            mousemove_callback.as_ref().unchecked_ref(),
        )?;
        mousemove_callback.forget();
    }
    // Mouse up
    {
        let input = input.clone();
        let mouseup_callback = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut input = input.borrow_mut();
            input.pressed = false;
            input.pos = (event.offset_x(), event.offset_y());
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "mouseup",
            mouseup_callback.as_ref().unchecked_ref(),
        )?;
        mouseup_callback.forget();
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
            for touch_index in 0..event.touches().length() {
                if let Some(touch) = event.touches().item(touch_index) {
                    input.pressed = true;
                    input.pos = (
                        touch.client_x() - offset_x as i32,
                        touch.client_y() - offset_y as i32,
                    );
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
    // Touch move
    {
        let input = input.clone();
        let canvas = html_get_canvas();
        let touchmove_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let offset_x = canvas.get_bounding_client_rect().left();
            let offset_y = canvas.get_bounding_client_rect().top();

            let mut input = input.borrow_mut();
            for touch_index in 0..event.touches().length() {
                if let Some(touch) = event.touches().item(touch_index) {
                    input.pos = (
                        touch.client_x() - offset_x as i32,
                        touch.client_y() - offset_y as i32,
                    );
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchmove",
            touchmove_callback.as_ref().unchecked_ref(),
        )?;
        touchmove_callback.forget();
    }
    // Touch up
    {
        let input = input.clone();
        let canvas = html_get_canvas();
        let touchend_callback = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let offset_x = canvas.get_bounding_client_rect().left();
            let offset_y = canvas.get_bounding_client_rect().top();

            let mut input = input.borrow_mut();
            for touch_index in 0..event.touches().length() {
                if let Some(touch) = event.touches().item(touch_index) {
                    input.pressed = false;
                    input.pos = (
                        touch.client_x() - offset_x as i32,
                        touch.client_y() - offset_y as i32,
                    );
                }
            }
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchend",
            touchend_callback.as_ref().unchecked_ref(),
        )?;
        touchend_callback.forget();
    }
    // Touch cancel
    {
        let input = input.clone();
        let touchcancel_callback = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
            let mut input = input.borrow_mut();
            input.pressed = false;
        }) as Box<dyn FnMut(_)>);
        html_get_canvas().add_event_listener_with_callback(
            "touchcancel",
            touchcancel_callback.as_ref().unchecked_ref(),
        )?;
        touchcancel_callback.forget();
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // WEBGL

    let webgl = html_get_canvas()
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;
    let glow_context = glow::Context::from_webgl1_context(webgl);
    let mut renderer = Renderer::new(glow_context);

    /*
    let vert_shader = compile_shader(
        &gl,
        WebGlRenderingContext::VERTEX_SHADER,
        r#"
        attribute vec4 position;
        void main() {
            gl_Position = position;
        }
    "#,
    )?;
    let frag_shader = compile_shader(
        &gl,
        WebGlRenderingContext::FRAGMENT_SHADER,
        r#"
        void main() {
            gl_FragColor = vec4(1.0, 0.0, 1.0, 1.0);
        }
    "#,
    )?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    let buffer = gl.create_buffer().ok_or("failed to create buffer")?;
    gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));

    gl.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
    gl.enable_vertex_attrib_array(0);

    */

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // MAINLOOP

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

    let mut frame_count = 0;
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let dpr = html_get_window().device_pixel_ratio();
        //log::info!("dpr: {:?}", dpr);

        // resize canvas
        {
            let window_width = f64::round(html_get_canvas().client_width() as f64 * dpr);
            let window_height = f64::round(html_get_canvas().client_height() as f64 * dpr);
            let canvas_width = html_get_canvas().width();
            let canvas_height = html_get_canvas().height();
            if canvas_width as i32 != window_width as i32
                || canvas_height as i32 != window_height as i32
            {
                assert!(window_width >= 0.0);
                assert!(window_height >= 0.0);
                html_get_canvas().set_width(window_width as u32);
                html_get_canvas().set_height(window_height as u32);
            }
        }

        if frame_count > 256 {
            //html_body().set_text_content(Some("All done!"));
            //log::info!("All done!");

            // Drop our handle to this closure so that it will get cleaned
            // up once we return.
            // let _ = f.borrow_mut().take();
            // return;
        }

        // Set the body's text content to how many times this
        // requestAnimationFrame callback has fired.
        frame_count += 1;
        // log::info!(
        //     "requestAnimationFrame has been called {} times.",
        //     frame_count
        // );

        let (pressed, pos_x, pos_y) = {
            let input = input.borrow();
            let pos_x = f64::floor(input.pos.0 as f64 * dpr);
            let pos_y = f64::floor(input.pos.1 as f64 * dpr);
            (input.pressed, pos_x as i32, pos_y as i32)
        };

        let red = pos_x as f32 / html_get_canvas().width() as f32;
        let green = pos_y as f32 / html_get_canvas().height() as f32;
        let blue = if pressed { 1.0 } else { 0.0 };
        //log::info!("pos: {:?}x{:?}", pos_x, pos_y);

        /*

        let canvas_width = html_get_canvas().width() as i32;
        let canvas_height = html_get_canvas().height() as i32;
        gl.viewport(0, 0, canvas_width, canvas_height);

        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        let vertex_count = 3;
        unsafe {
            let vertices: [f32; 9] = [
                -0.7 * f32::sin(frame_count as f32 / 60.0),
                -0.7 * f32::sin(frame_count as f32 / 60.0),
                0.0,
                0.7,
                -0.7,
                0.0,
                0.0,
                0.7,
                0.0,
            ];
            let vert_array = js_sys::Float32Array::view(&vertices);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        // Draw a 1 pixel border around the edge using
        // the scissor test since it's easier than setting up
        // a lot of stuff
        gl.clear_color(1.0, 0.0, 0.0, 1.0); // red
        gl.disable(WebGlRenderingContext::SCISSOR_TEST);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.enable(WebGlRenderingContext::SCISSOR_TEST);
        gl.scissor(1, 1, canvas_width - 2, canvas_height - 2);
        gl.clear_color(0.0, 0.0, 1.0, 1.0); // blue
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.clear_color(red, green, blue, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, vertex_count);

        gl.scissor(
            pos_x,
            html_get_canvas().height() as i32 - pos_y,
            4 * dpr as i32,
            4 * dpr as i32,
        );
        gl.clear_color(1.0, 1.0, 1.0, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        */

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
