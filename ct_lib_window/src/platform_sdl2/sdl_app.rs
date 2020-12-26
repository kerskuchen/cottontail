mod sdl_audio;
mod sdl_input;
mod sdl_window;

use super::core::log;
use super::core::serde_derive::{Deserialize, Serialize};
use super::core::*;
use super::core::{deserialize_from_json_file, serialize_to_json_file};
use super::game::{GameInput, GameMemory, GameStateInterface, Scancode, SystemCommand};

use std::{collections::VecDeque, time::Duration};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Configuration

const ENABLE_PANIC_MESSAGES: bool = false;
const ENABLE_FRAMETIME_LOGGING: bool = true;

#[derive(Serialize, Deserialize)]
struct LauncherConfig {
    display_index_to_use: i32,
    controller_deadzone_threshold_x: f32,
    controller_deadzone_threshold_y: f32,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Live looped input playback and recording

struct InputRecorder<GameStateType: GameStateInterface + Clone> {
    game_memory: GameMemory<GameStateType>,

    is_recording: bool,
    is_playing_back: bool,
    queue_playback: VecDeque<GameInput>,
    queue_recording: VecDeque<GameInput>,
}

impl<GameStateType: GameStateInterface + Clone> Default for InputRecorder<GameStateType> {
    fn default() -> Self {
        InputRecorder {
            game_memory: GameMemory::default(),

            is_recording: false,
            is_playing_back: false,
            queue_playback: VecDeque::new(),
            queue_recording: VecDeque::new(),
        }
    }
}

impl<GameStateType: GameStateInterface + Clone> InputRecorder<GameStateType> {
    fn start_recording(&mut self, game_memory: &GameMemory<GameStateType>) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = true;
        self.queue_recording.clear();
        self.game_memory = game_memory.clone();
    }

    fn stop_recording(&mut self) {
        assert!(self.is_recording);
        assert!(!self.is_playing_back);

        self.is_recording = false;
    }

    fn start_playback(&mut self, game_memory: &mut GameMemory<GameStateType>) {
        assert!(!self.is_recording);
        assert!(!self.is_playing_back);

        self.is_playing_back = true;
        self.queue_playback = self.queue_recording.clone();
        *game_memory = self.game_memory.clone();

        assert!(!self.queue_playback.is_empty());
    }

    fn stop_playback(&mut self) {
        assert!(!self.is_recording);
        assert!(self.is_playing_back);

        self.is_playing_back = false;
        self.queue_playback.clear();
    }

    fn record_input(&mut self, input: &GameInput) {
        assert!(self.is_recording);
        assert!(!self.is_playing_back);

        self.queue_recording.push_back(input.clone());
    }

    fn playback_input(&mut self, game_memory: &mut GameMemory<GameStateType>) -> GameInput {
        assert!(!self.is_recording);
        assert!(self.is_playing_back);

        if let Some(input) = self.queue_playback.pop_front() {
            input
        } else {
            // We hit the end of the stream -> go back to the beginning
            self.stop_playback();
            self.start_playback(game_memory);

            // As we could not read the input before we try again
            self.queue_playback.pop_front().unwrap()
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main event loop

fn log_frametimes(
    _duration_frame: f64,
    _duration_input: f64,
    _duration_update: f64,
    _duration_sound: f64,
    _duration_render: f64,
    _duration_swap: f64,
    _duration_wait: f64,
) {
    if ENABLE_FRAMETIME_LOGGING {
        log::trace!(
        "frame: {:.3}ms  input: {:.3}ms  update: {:.3}ms  sound: {:.3}ms  render: {:.3}ms  swap: {:.3}ms  idle: {:.3}ms",
        _duration_frame * 1000.0,
        _duration_input * 1000.0,
        _duration_update * 1000.0,
        _duration_sound * 1000.0,
        _duration_render * 1000.0,
        _duration_swap * 1000.0,
        _duration_wait * 1000.0
    );
    }
}

pub fn run_main<GameStateType: GameStateInterface + Clone>() {
    timer_initialize();
    let game_config = GameStateType::get_game_config();
    let savedata_dir = get_savegame_dir(
        &game_config.game_company_name,
        &game_config.game_save_folder_name,
        true,
    )
    .unwrap_or_else(|error| {
        sdl_window::Window::show_error_messagebox(&format!(
            "Could not get savegame location: {}",
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
        &game_config.game_window_title,
    );
    let mut renderer = window.create_renderer();

    let target_updates_per_second = window.refresh_rate();
    let target_seconds_per_frame = 1.0 / target_updates_per_second as f32;

    let (screen_width, screen_height) = window.dimensions();

    // Check if vsync is enabled
    // ---------------------------------------------------------------------------------------------

    let vsync_test_framecount = 4;
    let vsync_test_duration_target = target_seconds_per_frame * vsync_test_framecount as f32;
    let vsync_test_duration = {
        let vsync_test_start_time = timer_current_time_seconds();

        for _ in 0..vsync_test_framecount {
            renderer.clear_screen();
            window.sdl_window.gl_swap_window();
        }

        timer_current_time_seconds() - vsync_test_start_time
    } as f32;
    let ratio = vsync_test_duration / vsync_test_duration_target;
    let vsync_enabled = ratio > 0.5;
    log::debug!(
        "VSYNC test took {:.3}ms - it should take >{:.3}ms with VSYNC enabled -> \
         VSYNC seems to be {}",
        vsync_test_duration * 1000.0,
        (vsync_test_duration_target * 1000.0) / 2.0,
        if vsync_enabled { "enabled" } else { "disabled" }
    );

    log::info!(
        "Running with vsync {}",
        if vsync_enabled { "enabled" } else { "disabled" }
    );

    // ---------------------------------------------------------------------------------------------
    // Sound

    let mut audio_output = sdl_audio::AudioOutput::new(&sdl_context);

    // ---------------------------------------------------------------------------------------------
    // Input

    let mut gamepad_subsystem = {
        let savedata_mappings_path = path_join(&savedata_dir, "gamecontrollerdb.txt");
        let gamedata_mappings_path = "resources/gamecontrollerdb.txt".to_string();
        let gamepad_mappings = std::fs::read_to_string(&savedata_mappings_path)
            .or_else(|_error| {
                log::info!(
                    "Could not read gamepad mappings file at '{}' - using default one",
                    savedata_mappings_path
                );
                std::fs::read_to_string(&gamedata_mappings_path).map_err(|error| {
                    log::info!(
                        "Could not read gamepad mappings file at '{}' - game data corrupt? : {}",
                        gamedata_mappings_path,
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
            }
            log::info!("{}", gamepad_info);
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Game memory and input

    let mut input = GameInput::new();
    input.screen_framebuffer_width = screen_width;
    input.screen_framebuffer_height = screen_height;
    input.screen_framebuffer_dimensions_changed = true;

    let mut game_memory = GameMemory::<GameStateType>::default();

    // ---------------------------------------------------------------------------------------------
    // Mainloop setup

    let text_input = sdl_video.text_input();
    text_input.stop();

    let mut input_recorder = InputRecorder::default();

    let mut systemcommands: Vec<SystemCommand> = Vec::new();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let game_start_time = timer_current_time_seconds();
    let mut frame_start_time = game_start_time;
    let mut post_wait_time = game_start_time;
    log::debug!("Startup took {:.3}ms", game_start_time * 1000.0,);

    let mut mouse_pos_previous_x = input.mouse.pos_x;
    let mut mouse_pos_previous_y = input.mouse.pos_y;

    let mut current_tick = 0;
    let mut is_running = true;

    // ---------------------------------------------------------------------------------------------
    // Begin Mainloop

    while is_running {
        let pre_input_time = timer_current_time_seconds();

        current_tick += 1;

        //--------------------------------------------------------------------------------------
        // System commands

        for command in &systemcommands {
            match command {
                SystemCommand::FullscreenToggle => window.toggle_fullscreen(),
                SystemCommand::TextinputStart {
                    inputrect_x,
                    inputrect_y,
                    inputrect_width,
                    inputrect_height,
                } => {
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
                }
                SystemCommand::TextinputStop => {
                    log::trace!("Textinput mode disabled");
                    input.textinput.is_textinput_enabled = false;
                    text_input.stop();
                }
                SystemCommand::WindowedModeAllowResizing(allowed) => {
                    window.windowed_mode_set_resizable(*allowed);
                }
                SystemCommand::WindowedModeAllow(allowed) => {
                    window.set_windowed_mode_allowed(*allowed);
                }
                SystemCommand::WindowedModeSetSize {
                    width,
                    height,
                    minimum_width,
                    minimum_height,
                } => {
                    window.set_windowed_mode_size(*width, *height, *minimum_width, *minimum_height);
                }
                SystemCommand::ScreenSetGrabInput(grab_input) => {
                    window.set_input_grabbed(*grab_input);
                }
                SystemCommand::Shutdown => {
                    log::info!("Received shutdown signal");
                    is_running = false;
                }
                SystemCommand::Restart => {
                    log::info!("Received restart signal");
                    game_memory = GameMemory::default();
                    renderer.reset();
                }
            }
        }
        systemcommands.clear();

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
                Event::AppWillEnterForeground { .. } | Event::AppDidEnterForeground { .. } => {
                    // NOTE: This is Android only
                    input.has_foreground_event = true;
                }
                //----------------------------------------------------------------------------------
                // Window
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::SizeChanged(width, height) => {
                        input.screen_framebuffer_dimensions_changed = true;
                        input.screen_framebuffer_width = width as u32;
                        input.screen_framebuffer_height = height as u32;
                    }
                    WindowEvent::FocusGained => {
                        input.has_focus_event = true;
                        input.has_focus = true;
                    }
                    WindowEvent::FocusLost => {
                        input.has_focus_event = true;
                        input.has_focus = false;
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
                    input.keyboard.has_press_event = true;
                    if repeat {
                        input.keyboard.has_system_repeat_event = true;
                    }
                    let keycode = sdl_input::keycode_to_our_keycode(sdl2_keycode);
                    let scancode = sdl_input::scancode_to_our_scancode(sdl2_scancode);
                    input
                        .keyboard
                        .process_key_event(scancode, keycode, true, repeat, current_tick);
                }
                Event::KeyUp {
                    scancode: Some(sdl2_scancode),
                    keycode: Some(sdl2_keycode),
                    ..
                } => {
                    input.keyboard.has_release_event = true;
                    let keycode = sdl_input::keycode_to_our_keycode(sdl2_keycode);
                    let scancode = sdl_input::scancode_to_our_scancode(sdl2_scancode);
                    input
                        .keyboard
                        .process_key_event(scancode, keycode, false, false, current_tick);
                }
                //----------------------------------------------------------------------------------
                // Textinput
                //
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

                //----------------------------------------------------------------------------------
                // Touch
                //
                Event::FingerDown {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::round(x * screen_width as f32) as i32;
                    let pos_y = f32::round(y * screen_height as f32) as i32;
                    input
                        .touch
                        .process_finger_down(finger_id, pos_x, pos_y, current_tick);
                }
                Event::FingerUp {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::round(x * screen_width as f32) as i32;
                    let pos_y = f32::round(y * screen_height as f32) as i32;
                    input
                        .touch
                        .process_finger_up(finger_id, pos_x, pos_y, current_tick);
                }
                Event::FingerMotion {
                    finger_id, x, y, ..
                } => {
                    let pos_x = f32::round(x * screen_width as f32) as i32;
                    let pos_y = f32::round(y * screen_height as f32) as i32;
                    input.touch.process_finger_move(finger_id, pos_x, pos_y);
                }
                //----------------------------------------------------------------------------------
                // Mouse
                //
                // NOTE: The mouse position is checked just once after
                //       all events are processed
                //
                Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                    sdl2::mouse::MouseButton::Left => {
                        input.mouse.has_press_event = true;
                        input
                            .mouse
                            .button_left
                            .process_event(true, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::Middle => {
                        input.mouse.has_press_event = true;
                        input
                            .mouse
                            .button_middle
                            .process_event(true, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::Right => {
                        input.mouse.has_press_event = true;
                        input
                            .mouse
                            .button_right
                            .process_event(true, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::X1 => {
                        input.mouse.has_press_event = true;
                        input
                            .mouse
                            .button_x1
                            .process_event(true, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::X2 => {
                        input.mouse.has_press_event = true;
                        input
                            .mouse
                            .button_x2
                            .process_event(true, false, current_tick)
                    }
                    _ => {}
                },
                Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    sdl2::mouse::MouseButton::Left => {
                        input.mouse.has_release_event = true;
                        input
                            .mouse
                            .button_left
                            .process_event(false, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::Middle => {
                        input.mouse.has_release_event = true;
                        input
                            .mouse
                            .button_middle
                            .process_event(false, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::Right => {
                        input.mouse.has_release_event = true;
                        input
                            .mouse
                            .button_right
                            .process_event(false, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::X1 => {
                        input.mouse.has_release_event = true;
                        input
                            .mouse
                            .button_x1
                            .process_event(false, false, current_tick)
                    }
                    sdl2::mouse::MouseButton::X2 => {
                        input.mouse.has_release_event = true;
                        input
                            .mouse
                            .button_x2
                            .process_event(false, false, current_tick)
                    }
                    _ => {}
                },
                Event::MouseMotion { .. } => {
                    input.mouse.has_moved = true;
                }
                Event::MouseWheel { y, .. } => {
                    input.mouse.has_wheel_event = true;
                    input.mouse.wheel_delta = y;
                }
                _ => {}
            }
        }

        // Gamepad events
        // NOTE: Currently we collect events from all available gamepads
        if let Some(gamepad_subsystem) = &mut gamepad_subsystem {
            while let Some(gilrs::Event { event, .. }) = gamepad_subsystem.next_event() {
                let maybe_button_event = match event {
                    gilrs::EventType::ButtonPressed(button, _) => Some((button, true)),
                    gilrs::EventType::ButtonReleased(button, _) => Some((button, false)),
                    gilrs::EventType::Connected => {
                        input.gamepad.is_connected = true;
                        None
                    }
                    gilrs::EventType::Disconnected => {
                        input.gamepad.is_connected = true;
                        None
                    }
                    gilrs::EventType::AxisChanged(axis, value, _) => {
                        input.gamepad.is_connected = true;
                        match axis {
                            gilrs::Axis::LeftStickX => input.gamepad.stick_left.x = value,
                            gilrs::Axis::LeftStickY => input.gamepad.stick_left.y = -value,
                            gilrs::Axis::LeftZ => input.gamepad.trigger_left = value,
                            gilrs::Axis::RightStickX => input.gamepad.stick_right.x = value,
                            gilrs::Axis::RightStickY => input.gamepad.stick_right.y = -value,
                            gilrs::Axis::RightZ => input.gamepad.trigger_right = value,
                            _ => {}
                        };
                        None
                    }
                    _ => None,
                };

                if let Some((button, is_pressed)) = maybe_button_event {
                    input.gamepad.is_connected = true;
                    let gamepad_button = match button {
                        gilrs::Button::South => Some(&mut input.gamepad.action_down),
                        gilrs::Button::East => Some(&mut input.gamepad.action_right),
                        gilrs::Button::North => Some(&mut input.gamepad.action_up),
                        gilrs::Button::West => Some(&mut input.gamepad.action_left),
                        gilrs::Button::LeftTrigger => {
                            Some(&mut input.gamepad.trigger_button_left_1)
                        }
                        gilrs::Button::LeftTrigger2 => {
                            Some(&mut input.gamepad.trigger_button_left_2)
                        }
                        gilrs::Button::RightTrigger => {
                            Some(&mut input.gamepad.trigger_button_right_1)
                        }
                        gilrs::Button::RightTrigger2 => {
                            Some(&mut input.gamepad.trigger_button_right_2)
                        }
                        gilrs::Button::Select => Some(&mut input.gamepad.back),
                        gilrs::Button::Start => Some(&mut input.gamepad.start),
                        gilrs::Button::Mode => Some(&mut input.gamepad.home),
                        gilrs::Button::LeftThumb => Some(&mut input.gamepad.stick_button_left),
                        gilrs::Button::RightThumb => Some(&mut input.gamepad.stick_button_right),
                        gilrs::Button::DPadUp => Some(&mut input.gamepad.move_up),
                        gilrs::Button::DPadDown => Some(&mut input.gamepad.move_down),
                        gilrs::Button::DPadLeft => Some(&mut input.gamepad.move_left),
                        gilrs::Button::DPadRight => Some(&mut input.gamepad.move_right),
                        gilrs::Button::C => None,
                        gilrs::Button::Z => None,
                        gilrs::Button::Unknown => None,
                    };

                    if let Some(button) = gamepad_button {
                        button.process_event(is_pressed, false, current_tick);
                    }
                }
            }
        }

        // Mouse x in [0, screen_framebuffer_width - 1]  (left to right)
        // Mouse y in [0, screen_framebuffer_height - 1] (top to bottom)
        //
        // NOTE: We get the mouse position and delta from querying SDL instead of accumulating
        //       events, as it is faster, more accurate and less error-prone
        input.mouse.pos_x = event_pump.mouse_state().x();
        input.mouse.pos_y = event_pump.mouse_state().y();
        input.mouse.delta_x = input.mouse.pos_x - mouse_pos_previous_x;
        input.mouse.delta_y = input.mouse.pos_y - mouse_pos_previous_y;
        input.touch.calculate_move_deltas();
        input.screen_is_fullscreen = window.fullscreen_active;

        //--------------------------------------------------------------------------------------
        // Start/stop input-recording/-playback

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
                    input_recorder.start_recording(&game_memory);
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
                    input_recorder.start_playback(&mut game_memory);
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
            input = input_recorder.playback_input(&mut game_memory);
            *input.keyboard.keys.get_mut(&Scancode::P).unwrap() = previous_playback_key_state;
        }

        let post_input_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Timings, update and drawing

        let pre_update_time = post_input_time;

        let duration_frame = pre_update_time - frame_start_time;
        frame_start_time = pre_update_time;

        input.deltatime =
            super::snap_deltatime_to_nearest_common_refresh_rate(duration_frame as f32);
        input.real_world_uptime = frame_start_time;
        input.audio_playback_rate_hz = audio_output.audio_playback_rate_hz;

        if input.has_focus {
            game_memory.update(&input, &mut systemcommands);
        } else {
            let TODO = "just repeat the drawcommands from last time - but without the 
                update/create texture commands or other expensive/complex commands";
        }

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

        let post_update_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Sound output

        let pre_sound_time = post_update_time;

        let TODO = "make it so that audio is always there and can handle loading its sounds later";
        if game_memory.audio.is_some() {
            if input.has_focus {
                let audio = game_memory
                    .audio
                    .as_mut()
                    .expect("No audiostate initialized");
                audio_output.render_frames(audio, 2.0 * target_seconds_per_frame);
            }
        }

        let post_sound_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Drawcommands

        let pre_render_time = post_sound_time;

        let TODO = "make it so that draw is always there and can handle loading its sounds later";
        if game_memory.draw.is_some() {
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

        let post_render_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Swap framebuffers

        let pre_swap_time = post_render_time;

        window.sdl_window.gl_swap_window();

        let post_swap_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Wait for target frame time

        let pre_wait_time = post_swap_time;

        if !vsync_enabled {
            // NOTE: We need to manually wait to reach target frame rate
            loop {
                let time_left_till_flip = target_seconds_per_frame
                    - (timer_current_time_seconds() - post_wait_time) as f32;

                if time_left_till_flip > 0.002 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                } else {
                    // NOTE: Busywait for the remaining time
                }

                if time_left_till_flip <= 0.0 {
                    break;
                }
            }
        }

        post_wait_time = timer_current_time_seconds();

        //--------------------------------------------------------------------------------------
        // Debug timing output

        let duration_input = post_input_time - pre_input_time;
        let duration_update = post_update_time - pre_update_time;
        let duration_sound = post_sound_time - pre_sound_time;
        let duration_render = post_render_time - pre_render_time;
        let duration_swap = post_swap_time - pre_swap_time;
        let duration_wait = post_wait_time - pre_wait_time;

        log_frametimes(
            duration_frame,
            duration_input,
            duration_update,
            duration_sound,
            duration_render,
            duration_swap,
            duration_wait,
        );
    }

    //--------------------------------------------------------------------------------------
    // Mainloop stopped

    let duration_gameplay = timer_current_time_seconds() - game_start_time;
    log::debug!("Playtime: {:.3}s", duration_gameplay);

    // Make sure our sound output has time to wind down
    std::thread::sleep(Duration::from_secs_f32(4.0 * target_seconds_per_frame))
}
