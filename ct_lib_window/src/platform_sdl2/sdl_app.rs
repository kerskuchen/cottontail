pub mod sdl_audio;
mod sdl_input;
mod sdl_window;

pub use sdl_audio as audio;
use sdl_input::{sdl_keycode_to_our_keycode, sdl_scancode_to_our_scancode};

use crate::{
    AppEventHandler, AppInfo, FingerPlatformId, GamepadAxis, GamepadPlatformState, MouseButton,
    PlatformWindowCommand,
};

use ct_lib_core::log;
use ct_lib_core::serde_derive::{Deserialize, Serialize};
use ct_lib_core::*;
use ct_lib_core::{deserialize_from_json_file, serialize_to_json_file};

use std::{collections::HashMap, time::Duration};

use self::sdl_input::{gilrs_id_to_our_id, our_axis_to_gilrs_axis, our_button_to_gilrs_button};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Configuration

const ENABLE_PANIC_MESSAGES: bool = false;
const ENABLE_TOUCH_EMULATION: bool = false;

#[derive(Serialize, Deserialize)]
struct LauncherConfig {
    display_index_to_use: i32,
    controller_deadzone_threshold_x: f32,
    controller_deadzone_threshold_y: f32,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main event loop

static mut PLATFORM_WINDOW_COMMANDS: Vec<PlatformWindowCommand> = Vec::new();

pub fn run_main<AppEventHandlerType: AppEventHandler + 'static>(
    mut app: AppEventHandlerType,
    app_info: AppInfo,
) -> Result<(), String> {
    timer_initialize();
    let savedata_dir = get_user_savedata_dir(&app_info.company_name, &app_info.save_folder_name)
        .unwrap_or_else(|error| {
            sdl_window::Window::show_error_messagebox(&format!(
                "Could not get location for saving userdata: {}",
                error,
            ));
            panic!()
        });

    // ---------------------------------------------------------------------------------------------
    // Logging and error handling

    let logfile_path = path_join(&savedata_dir, "logging.txt");
    if let Err(error) = init_logging(&logfile_path, log::Level::Trace) {
        sdl_window::Window::show_error_messagebox(&format!(
            "Could not initialize logger at '{}' : {}",
            &logfile_path, error,
        ));
    }

    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("{}", panic_info);

        if ENABLE_PANIC_MESSAGES {
            let logfile_path_canonicalized =
                path_canonicalize(&logfile_path).unwrap_or(logfile_path.to_string());
            let messagebox_text = format!(
                "A Fatal error has occured:\n{}\nLogfile written to '{}'",
                &panic_info, &logfile_path_canonicalized,
            );
            sdl_window::Window::show_error_messagebox(&messagebox_text);
        }
    }));

    // ---------------------------------------------------------------------------------------------
    // Config

    // Get launcher config
    let launcher_config: LauncherConfig = {
        let config_filepath = path_join(&savedata_dir, "launcher_config.json");
        if path_exists(&config_filepath) {
            deserialize_from_json_file(&config_filepath)
        } else {
            let config = LauncherConfig {
                display_index_to_use: 0,
                controller_deadzone_threshold_x: 0.1,
                controller_deadzone_threshold_y: 0.1,
            };
            serialize_to_json_file(&config, &config_filepath);
            config
        }
    };

    // ---------------------------------------------------------------------------------------------
    // SDL subsystems

    let sdl_context = sdl2::init().expect("Failed to initialize SDL2");
    let sdl_video = sdl_context
        .video()
        .expect("Failed to initialize SDL2 video");

    // ---------------------------------------------------------------------------------------------
    // SDL Window

    let mut window = sdl_window::Window::new(
        sdl_video.clone(),
        launcher_config.display_index_to_use,
        &app_info.window_title,
    );
    let mut renderer = window.create_renderer();

    let (mut screen_width, mut screen_height) = window.dimensions();
    app.handle_window_resize(screen_width, screen_height, false);

    // ---------------------------------------------------------------------------------------------
    // Sound

    let mut audio = sdl_audio::AudioOutput::new(&sdl_context);

    // ---------------------------------------------------------------------------------------------
    // Input

    let mut gamepad_subsystem = {
        const GAME_CONTROLLER_DB: &[u8] = include_bytes!("../../resources/gamecontrollerdb.txt");

        let savedata_mappings_path = path_join(&savedata_dir, "gamecontrollerdb.txt");
        let gamepad_mappings = std::fs::read_to_string(&savedata_mappings_path)
            .or_else(|_error| {
                log::info!(
                    "Could not read gamepad mappings file at '{}' - using default one",
                    savedata_mappings_path
                );

                String::from_utf8(GAME_CONTROLLER_DB.to_vec()).map_err(|error| {
                    log::info!(
                        "Could not read gamepad mappings data - game data corrupt? : {}",
                        error
                    )
                })
            })
            .ok();

        let mut builder = gilrs::GilrsBuilder::new();
        if let Some(mapping) = gamepad_mappings {
            builder = builder.add_mappings(&mapping);
        }
        builder
            .build()
            .map_err(|error| {
                log::warn!("Could not initialize game controller subsystem: {}", error)
            })
            .ok()
    };

    // Print some info about the currently connected gamepads
    let mut gamepads = HashMap::new();
    if let Some(gamepad_subsystem) = &gamepad_subsystem {
        if gamepad_subsystem.gamepads().count() == 0 {
            log::info!("No gamepads connected");
        } else {
            let mut gamepad_info = "\nThe following gamepads were found:\n".to_string();
            for (id, gamepad) in gamepad_subsystem.gamepads() {
                gamepad_info += &format!(
                    "  {}: {} - {:?}\n",
                    id,
                    gamepad.name(),
                    gamepad.power_info()
                );

                let gamepad_id = gilrs_id_to_our_id(id);
                gamepads.insert(gamepad_id, GamepadPlatformState::new());
            }
            log::info!("{}", gamepad_info);
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Input

    let text_input = sdl_video.text_input();
    text_input.stop();
    let mut touch_emulation_left_button_pressed = false;

    // ---------------------------------------------------------------------------------------------
    // Mainloop setup

    let mut event_pump = sdl_context.event_pump().unwrap();

    let app_start_time = timer_current_time_seconds();
    let mut frame_start_time = app_start_time;
    log::debug!("Startup took {:.3}ms", app_start_time * 1000.0,);

    let mut is_running = true;

    // ---------------------------------------------------------------------------------------------
    // Begin Mainloop

    while is_running {
        //--------------------------------------------------------------------------------------
        // Event loop

        use sdl2::event::Event;
        use sdl2::event::WindowEvent;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    log::info!("Quit signal received");
                    is_running = false;
                }
                //----------------------------------------------------------------------------------
                // Window
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(width, height) => {
                        app.handle_window_resize(
                            width as u32,
                            height as u32,
                            window.fullscreen_active,
                        );
                        screen_width = width as u32;
                        screen_height = height as u32;
                    }
                    WindowEvent::FocusGained => {
                        app.handle_window_focus_gained();
                    }
                    WindowEvent::FocusLost => {
                        app.handle_window_focus_lost();
                    }
                    _ => {}
                },
                //----------------------------------------------------------------------------------
                // Keyboard
                //
                Event::KeyDown {
                    scancode: Some(sdl2_scancode),
                    keycode: Some(sdl2_keycode),
                    repeat,
                    ..
                } => {
                    let keycode = sdl_keycode_to_our_keycode(sdl2_keycode);
                    let scancode = sdl_scancode_to_our_scancode(sdl2_scancode);
                    app.handle_key_press(scancode, keycode, repeat);
                }
                Event::KeyUp {
                    scancode: Some(sdl2_scancode),
                    keycode: Some(sdl2_keycode),
                    ..
                } => {
                    let keycode = sdl_keycode_to_our_keycode(sdl2_keycode);
                    let scancode = sdl_scancode_to_our_scancode(sdl2_scancode);
                    app.handle_key_release(scancode, keycode);
                }
                //----------------------------------------------------------------------------------
                // Textinput
                //
                // TODO
                /*
                Event::TextInput { text, .. } => {
                    if input.textinput.is_textinput_enabled {
                        input.textinput.has_new_textinput_event = true;
                        input.textinput.inputtext += &text;
                    }
                }
                Event::TextEditing {
                    text,
                    start,
                    length,
                    ..
                } => {
                    if input.textinput.is_textinput_enabled {
                        input.textinput.has_new_composition_event = true;
                        input.textinput.composition_text += &text;
                        input.textinput.composition_cursor_pos = start;
                        input.textinput.composition_selection_length = length;
                    }
                }
                */
                //----------------------------------------------------------------------------------
                // Touch
                //
                Event::FingerDown {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::floor(x * screen_width as f32) as i32;
                    let pos_y = f32::floor(y * screen_height as f32) as i32;
                    app.handle_touch_press(finger_id as FingerPlatformId, pos_x, pos_y);
                }
                Event::FingerUp {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::floor(x * screen_width as f32) as i32;
                    let pos_y = f32::floor(y * screen_height as f32) as i32;
                    app.handle_touch_release(finger_id as FingerPlatformId, pos_x, pos_y);
                }
                Event::FingerMotion {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::floor(x * screen_width as f32) as i32;
                    let pos_y = f32::floor(y * screen_height as f32) as i32;
                    app.handle_touch_move(finger_id as FingerPlatformId, pos_x, pos_y);
                }
                //----------------------------------------------------------------------------------
                // Mouse
                //
                // NOTE: The mouse position is checked just once after
                //       all events are processed
                //
                Event::MouseButtonDown {
                    mouse_btn, x, y, ..
                } => {
                    if ENABLE_TOUCH_EMULATION {
                        match mouse_btn {
                            sdl2::mouse::MouseButton::Left => {
                                touch_emulation_left_button_pressed = true;
                                app.handle_touch_press(0, x, y);
                            }
                            _ => {}
                        }
                    } else {
                        match mouse_btn {
                            sdl2::mouse::MouseButton::Left => {
                                app.handle_mouse_press(MouseButton::Left, x, y);
                            }
                            sdl2::mouse::MouseButton::Middle => {
                                app.handle_mouse_press(MouseButton::Middle, x, y);
                            }
                            sdl2::mouse::MouseButton::Right => {
                                app.handle_mouse_press(MouseButton::Right, x, y);
                            }
                            sdl2::mouse::MouseButton::X1 => {
                                app.handle_mouse_press(MouseButton::X1, x, y);
                            }
                            sdl2::mouse::MouseButton::X2 => {
                                app.handle_mouse_press(MouseButton::X2, x, y);
                            }
                            _ => {}
                        }
                    }
                }
                Event::MouseButtonUp {
                    mouse_btn, x, y, ..
                } => {
                    if ENABLE_TOUCH_EMULATION {
                        match mouse_btn {
                            sdl2::mouse::MouseButton::Left => {
                                touch_emulation_left_button_pressed = false;
                                app.handle_touch_release(0, x, y);
                            }
                            _ => {}
                        }
                    } else {
                        match mouse_btn {
                            sdl2::mouse::MouseButton::Left => {
                                app.handle_mouse_release(MouseButton::Left, x, y);
                            }
                            sdl2::mouse::MouseButton::Middle => {
                                app.handle_mouse_release(MouseButton::Middle, x, y);
                            }
                            sdl2::mouse::MouseButton::Right => {
                                app.handle_mouse_release(MouseButton::Right, x, y);
                            }
                            sdl2::mouse::MouseButton::X1 => {
                                app.handle_mouse_release(MouseButton::X1, x, y);
                            }
                            sdl2::mouse::MouseButton::X2 => {
                                app.handle_mouse_release(MouseButton::X2, x, y);
                            }
                            _ => {}
                        }
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if ENABLE_TOUCH_EMULATION {
                        if touch_emulation_left_button_pressed {
                            app.handle_touch_move(0, x, y);
                        }
                    } else {
                        app.handle_mouse_move(x, y);
                    }
                }
                Event::MouseWheel { y, .. } => {
                    if !ENABLE_TOUCH_EMULATION {
                        app.handle_mouse_wheel_scroll(y);
                    }
                }
                _ => {}
            }
        }

        // Gamepad
        if let Some(gamepad_subsystem) = gamepad_subsystem.as_mut() {
            // NOTE: We collect connect/disconnect events here only and not individual button
            //       presses or axis events. Later on we just extract the whole state and forward
            //       it to the application
            while let Some(gilrs::Event { id, event, .. }) = gamepad_subsystem.next_event() {
                let gamepad_id = gilrs_id_to_our_id(id);
                match event {
                    gilrs::EventType::Connected => {
                        gamepads.insert(gamepad_id, GamepadPlatformState::new());
                        app.handle_gamepad_connected(gamepad_id);
                    }
                    gilrs::EventType::Disconnected => {
                        gamepads.remove(&gamepad_id);
                        app.handle_gamepad_disconnected(gamepad_id);
                    }
                    _ => {}
                }
            }

            // Collect and forward gamepad states to the application
            for (id, gamepad) in gamepad_subsystem.gamepads() {
                let gamepad_id = gilrs_id_to_our_id(id);
                let our_gamepad = gamepads.entry(gamepad_id).or_default();
                for (&our_button, is_pressed) in our_gamepad.buttons.iter_mut() {
                    *is_pressed = gamepad
                        .button_data(our_button_to_gilrs_button(our_button))
                        .map(|button_data| button_data.is_pressed())
                        .unwrap_or(false);
                }
                for (&our_axis, value) in our_gamepad.axes.iter_mut() {
                    *value = gamepad
                        .axis_data(our_axis_to_gilrs_axis(our_axis))
                        .map(|axis_data| axis_data.value())
                        .unwrap_or(0.0);
                    if our_axis == GamepadAxis::StickLeftY || our_axis == GamepadAxis::StickRightY {
                        *value *= -1.0;
                    }
                }
                app.handle_gamepad_new_state(gamepad_id, our_gamepad);
            }
        }

        renderer.update_screen_dimensions(screen_width, screen_height);

        //--------------------------------------------------------------------------------------
        // Tick

        let current_time = timer_current_time_seconds();
        let duration_frame = current_time - frame_start_time;
        frame_start_time = current_time;
        app.run_tick(
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
                PlatformWindowCommand::FullscreenToggle => window.toggle_fullscreen(),
                PlatformWindowCommand::TextinputStart {
                    /*
                    inputrect_x,
                    inputrect_y,
                    inputrect_width,
                    inputrect_height,
                     */
                    ..
                } => {
                    /* TODO
                    log::trace!("Textinput mode enabled");
                    input.textinput.is_textinput_enabled = true;
                    text_input.start();

                    let text_input_rect = sdl2::rect::Rect::new(
                        *inputrect_x,
                        *inputrect_y,
                        *inputrect_width,
                        *inputrect_height,
                    );
                    text_input.set_rect(text_input_rect);
                     */
                }
                PlatformWindowCommand::TextinputStop => {
                    /* TODO
                    log::trace!("Textinput mode disabled");
                    input.textinput.is_textinput_enabled = false;
                    text_input.stop();
                     */
                }
                PlatformWindowCommand::WindowedModeAllowResizing(allowed) => {
                    window.windowed_mode_set_resizable(allowed);
                }
                PlatformWindowCommand::WindowedModeAllow(allowed) => {
                    window.set_windowed_mode_allowed(allowed);
                }
                PlatformWindowCommand::WindowedModeSetSize {
                    width,
                    height,
                    minimum_width,
                    minimum_height,
                } => {
                    window.set_windowed_mode_size(width, height, minimum_width, minimum_height);
                }
                PlatformWindowCommand::ScreenSetGrabInput(grab_input) => {
                    window.set_input_grabbed(grab_input);
                }
                PlatformWindowCommand::Shutdown => {
                    log::info!("Received shutdown signal");
                    is_running = false;
                }
                PlatformWindowCommand::Restart => {
                    log::info!("Received restart signal");
                    app.reset();
                    renderer.reset();
                    audio.reset();
                }
            }
            }
        }

        window.sdl_window.gl_swap_window();
    }

    //--------------------------------------------------------------------------------------
    // Mainloop stopped

    let app_uptime = timer_current_time_seconds() - app_start_time;
    log::debug!("Application uptime: {:.3}s", app_uptime);

    // Make sure our sound output has time to wind down so it does not crack
    let audio_winddown_time_sec = (2.0 * audio.get_audiobuffer_size_in_frames() as f32
        + audio.get_num_frames_in_queue() as f32)
        / audio.get_audio_playback_rate_hz() as f32;
    std::thread::sleep(Duration::from_secs_f32(audio_winddown_time_sec));

    Ok(())
}

#[inline]
pub fn add_platform_window_command(appcommand: PlatformWindowCommand) {
    unsafe { PLATFORM_WINDOW_COMMANDS.push(appcommand) }
}
