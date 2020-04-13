// Copyright (c) 2019 Jakob Schwab
// This code is licensed under MIT license (see LICENSE.txt in repository root for details)

mod renderer_opengl;
mod sdl_input;
mod sdl_window;

use ct_lib::audio::*;
use ct_lib::game::{GameInput, GameMemory, GameStateInterface, Scancode, SystemCommand};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Configuration

const ENABLE_PANIC_MESSAGES: bool = false;

struct _GameConfig {
    display_index_to_use: u32,
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
// Appdata path

#[cfg(not(target_os = "windows"))]
fn platform_get_appdata_dir(company_name: &str, game_save_folder_name: &str) -> String {
    sdl2::filesystem::pref_path(company_name, game_save_folder_name)
        .expect("Cannot find suitable path to write data")
        .to_owned()
}

#[cfg(target_os = "windows")]
fn platform_get_appdata_dir(company_name: &str, game_save_folder_name: &str) -> String {
    let user_home_path = std::env::var("userprofile").expect("Cannot find user home path");
    let savegame_path =
        user_home_path + "\\Saved Games\\" + company_name + "\\" + game_save_folder_name;
    std::fs::create_dir_all(&savegame_path).expect(&format!(
        "Cannot create savegame directory at '{}'",
        &savegame_path
    ));
    savegame_path
}

fn platform_get_savegame_dir(company_name: &str, game_save_folder_name: &str) -> String {
    // Try local path first: Write test file to see if we even have writing permissions for './'
    if std::fs::write("test.txt", "test").is_ok() {
        if std::fs::remove_file("test.txt").is_ok() {
            return "".to_owned();
        }
    }

    platform_get_appdata_dir(company_name, game_save_folder_name)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main event loop

struct SDLAudioCallback {
    output: Arc<Mutex<AudioOutput>>,
}

impl sdl2::audio::AudioCallback for SDLAudioCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        debug_assert!(out.len() == AUDIO_CHUNKLENGTH_IN_SAMPLES);

        let chunk = {
            let (chunk_index, maybe_chunk) = self.output.lock().unwrap().next_chunk();
            maybe_chunk.unwrap_or_else(|| {
                log::debug!("Audiobuffer empty at chunk index {}", chunk_index);
                [0; AUDIO_CHUNKLENGTH_IN_SAMPLES]
            })
        };
        out.copy_from_slice(&chunk);
    }
}

pub fn run_main<GameStateType: GameStateInterface + Clone>() {
    let launcher_start_time = std::time::Instant::now();
    let game_config = GameStateType::get_game_config();
    let savadata_dir = platform_get_savegame_dir(
        &game_config.game_company_name,
        &game_config.game_save_folder_name,
    );

    // ---------------------------------------------------------------------------------------------
    // Logging and error handling

    let logfile_path = ct_lib::system::path_join(&savadata_dir, "logging.txt");
    if ct_lib::system::path_exists(&logfile_path) {
        let remove_result = std::fs::remove_file(&logfile_path);
        if let Err(error) = remove_result {
            sdl_window::Window::show_error_messagebox(&format!(
                "Could not remove previous logfile at '{}' : {}",
                logfile_path, error,
            ));
        }
    }

    let logger_create_result = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("{}: {}\r", record.level(), message))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .chain(fern::log_file(&logfile_path).expect("Could not open logfile"))
        .apply();
    if let Err(error) = logger_create_result {
        sdl_window::Window::show_error_messagebox(&format!(
            "Could not remove previous logfile at '{}' : {}",
            &logfile_path, error,
        ));
    }

    if ENABLE_PANIC_MESSAGES {
        std::panic::set_hook(Box::new(|panic_info| {
            log::error!("{}", &panic_info);
            let backtrace = backtrace::Backtrace::new();
            log::error!("BACKTRACE:\r\n{:?}", backtrace);

            let game_config = GameStateType::get_game_config();
            let logfile_path = ct_lib::system::path_join(
                &platform_get_savegame_dir(
                    &game_config.game_company_name,
                    &game_config.game_save_folder_name,
                ),
                "logging.txt",
            )
            .replace("/", "\\");
            let logfile_path_canonicalized = std::path::Path::new(&logfile_path).canonicalize();

            let messagebox_text = if let Ok(logfile_path_canonicalized) = logfile_path_canonicalized
            {
                let logfile_path_canonicalized_without_extended_length_path_syntax =
                    logfile_path_canonicalized
                        .to_string_lossy()
                        .replace("\\\\?\\", "");

                format!(
                    "A Fatal error has occured:\n{}\nLogfile written to '{}'",
                    &panic_info, &logfile_path_canonicalized_without_extended_length_path_syntax,
                )
            } else {
                format!(
                    "A Fatal error has occured:\n{}\nLogfile written to '{}'",
                    &panic_info, &logfile_path,
                )
            };

            sdl_window::Window::show_error_messagebox(&messagebox_text);
        }));
    }

    // ---------------------------------------------------------------------------------------------
    // SDL subsystems

    let sdl_context = sdl2::init().expect("Failed to initialize SDL2");
    let sdl_video = sdl_context
        .video()
        .expect("Failed to initialize SDL2 video");
    let sdl_audio = sdl_context
        .audio()
        .expect("Failed to initialize SDL2 audio");
    let sdl_controller = sdl_context
        .game_controller()
        .expect("Cannot initialize controller subsystem");

    // ---------------------------------------------------------------------------------------------
    // SDL Window

    let mut window = sdl_window::Window::new(sdl_video.clone(), 0, &game_config.game_window_title);
    let mut renderer = window.create_renderer();

    let target_updates_per_second = window.refresh_rate();
    let target_seconds_per_frame = 1.0 / target_updates_per_second as f32;

    let (screen_width, screen_height) = window.dimensions();

    // Check if vsync is enabled
    // ---------------------------------------------------------------------------------------------

    let vsync_test_framecount = 4;
    let vsync_test_duration_target = target_seconds_per_frame * vsync_test_framecount as f32;
    let vsync_test_duration = {
        let vsync_test_start_time = std::time::Instant::now();

        for _ in 0..vsync_test_framecount {
            renderer.clear();
            window.sdl_window.gl_swap_window();
        }
        std::time::Instant::now()
            .duration_since(vsync_test_start_time)
            .as_secs_f32()
    };
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

    let audio_format_desired = sdl2::audio::AudioSpecDesired {
        freq: Some(AUDIO_FREQUENCY as i32),
        channels: Some(AUDIO_CHANNELCOUNT as u8),
        // IMPORTANT: `samples` is a misnomer - it is actually the frames
        samples: Some(AUDIO_CHUNKLENGTH_IN_FRAMES as u16),
    };

    let audio_output = Arc::new(Mutex::new(AudioOutput::new()));
    let audio_output_clone = Arc::clone(&audio_output);

    let audio_device = sdl_audio
        .open_playback(None, &audio_format_desired, |spec| {
            assert!(
                spec.freq == AUDIO_FREQUENCY as i32,
                "Cannot initialize audio output with frequency {}",
                AUDIO_FREQUENCY
            );
            assert!(
                spec.channels == AUDIO_CHANNELCOUNT as u8,
                "Cannot initialize audio output with channel count {}",
                AUDIO_CHANNELCOUNT
            );
            assert!(
                spec.samples == AUDIO_CHUNKLENGTH_IN_FRAMES as u16,
                "Cannot initialize audio output audiobuffersize {}",
                AUDIO_CHUNKLENGTH_IN_SAMPLES
            );

            SDLAudioCallback {
                output: audio_output_clone,
            }
        })
        .expect("Cannot initialize audio output");

    log::info!(
        "Opened audio channel on default output device: (frequency: {}, channelcount: {})",
        AUDIO_FREQUENCY,
        AUDIO_CHANNELCOUNT,
    );

    let mut audiochunks = Vec::<Audiochunk>::new();

    // ---------------------------------------------------------------------------------------------
    // Input

    sdl_controller
        .load_mappings(ct_lib::system::path_join(
            &savadata_dir,
            "gamecontrollerdb.txt",
        ))
        .unwrap_or_else(|_error| {
            log::info!(
                "Could not find 'gamecontrollerdb.txt' at '{}' using default one from game data",
                &savadata_dir
            );
            sdl_controller
                .load_mappings("assets_baked/gamecontrollerdb.txt")
                .expect("Cannot find 'assets_baked/gamecontrollerdb.txt' - game data corrupt?")
        });

    // ---------------------------------------------------------------------------------------------
    // Game memory and input

    let mut input = GameInput::new();
    input.screen_framebuffer_width = screen_width;
    input.screen_framebuffer_height = screen_height;
    input.screen_framebuffer_dimensions_changed = true;
    input.keyboard = sdl_input::keyboardstate_create();
    input.textinput.is_textinput_enabled = false;

    let mut game_memory = GameMemory::<GameStateType>::default();

    // ---------------------------------------------------------------------------------------------
    // Mainloop setup

    let text_input = sdl_video.text_input();
    text_input.stop();

    let mut input_recorder = InputRecorder::default();

    let mut systemcommands: Vec<SystemCommand> = Vec::new();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut debug_print_counter = 0;

    audio_device.resume();
    let game_start_time = std::time::Instant::now();
    let mut frame_start_time = game_start_time;
    let mut post_wait_time = game_start_time;
    let duration_startup = game_start_time
        .duration_since(launcher_start_time)
        .as_secs_f32();
    log::debug!("Startup took {:.3}ms", duration_startup * 1000.0,);

    let mut mouse_pos_previous_x = input.mouse.pos_x;
    let mut mouse_pos_previous_y = input.mouse.pos_y;

    let mut current_tick = 0;
    let mut is_running = true;

    // ---------------------------------------------------------------------------------------------
    // Begin Mainloop

    while is_running {
        let pre_input_time = std::time::Instant::now();

        current_tick += 1;

        //--------------------------------------------------------------------------------------
        // System commands

        for command in &systemcommands {
            match command {
                SystemCommand::FullscreenToggle => window.toggle_fullscreen(),
                SystemCommand::FullscreenEnable(enabled) => {
                    if *enabled {
                        window.enable_fullscreen();
                    } else {
                        window.disable_fullscreen();
                    }
                }
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
                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as i64 {
                        let finger = &mut input.touch.fingers[finger_id as usize];

                        // IMPORTANT: At this point we may have an out of date screen dimensions
                        //            if the window size changed since last frame.
                        //            Because we use touch input only on mobile (where we disable
                        //            resizing the window) this is ok.
                        finger.pos_x = f32::round(x * screen_width as f32) as i32;
                        finger.pos_y = f32::round(y * screen_height as f32) as i32;

                        // NOTE: We don't want fake deltas when pressing. This can happen when our
                        //       last release was not the same as our press position.
                        finger.delta_x = 0;
                        finger.delta_y = 0;

                        input.touch.has_press_event = true;
                        finger.state.process_event(true, false, current_tick);
                    }
                }
                Event::FingerUp {
                    finger_id, x, y, ..
                } => {
                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as i64 {
                        let finger = &mut input.touch.fingers[finger_id as usize];
                        let finger_previous = &mut input.touch.fingers_previous[finger_id as usize];

                        // IMPORTANT: At this point we may have an out of date screen dimensions
                        //            if the window size changed since last frame.
                        //            Because we use touch input only on mobile (where we disable
                        //            resizing the window) this is ok.
                        finger.pos_x = f32::round(x * screen_width as f32) as i32;
                        finger.pos_y = f32::round(y * screen_height as f32) as i32;

                        finger.delta_x = finger.pos_x - finger_previous.pos_x;
                        finger.delta_y = finger.pos_y - finger_previous.pos_y;

                        input.touch.has_release_event = true;
                        input.touch.fingers[finger_id as usize].state.process_event(
                            false,
                            false,
                            current_tick,
                        );
                    }
                }
                Event::FingerMotion {
                    finger_id, x, y, ..
                } => {
                    let finger = &mut input.touch.fingers[finger_id as usize];
                    let finger_previous = &mut input.touch.fingers_previous[finger_id as usize];

                    // IMPORTANT: At this point we may have an out of date screen dimensions
                    //            if the window size changed since last frame.
                    //            Because we use touch input only on mobile (where we disable
                    //            resizing the window) this is ok.
                    finger.pos_x = f32::round(x * screen_width as f32) as i32;
                    finger.pos_y = f32::round(y * screen_height as f32) as i32;

                    finger.delta_x = finger.pos_x - finger_previous.pos_x;
                    finger.delta_y = finger.pos_y - finger_previous.pos_y;

                    if finger_id < ct_lib::game::TOUCH_MAX_FINGER_COUNT as i64 {
                        input.touch.has_move_event = true;
                    }
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
                        input.mouse.button_left.process_event(true, false, 0)
                    }
                    sdl2::mouse::MouseButton::Middle => {
                        input.mouse.has_press_event = true;
                        input.mouse.button_middle.process_event(true, false, 0)
                    }
                    sdl2::mouse::MouseButton::Right => {
                        input.mouse.has_press_event = true;
                        input.mouse.button_right.process_event(true, false, 0)
                    }
                    sdl2::mouse::MouseButton::X1 => {
                        input.mouse.has_press_event = true;
                        input.mouse.button_x1.process_event(true, false, 0)
                    }
                    sdl2::mouse::MouseButton::X2 => {
                        input.mouse.has_press_event = true;
                        input.mouse.button_x2.process_event(true, false, 0)
                    }
                    _ => {}
                },
                Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    sdl2::mouse::MouseButton::Left => {
                        input.mouse.has_release_event = true;
                        input.mouse.button_left.process_event(false, false, 0)
                    }
                    sdl2::mouse::MouseButton::Middle => {
                        input.mouse.has_release_event = true;
                        input.mouse.button_middle.process_event(false, false, 0)
                    }
                    sdl2::mouse::MouseButton::Right => {
                        input.mouse.has_release_event = true;
                        input.mouse.button_right.process_event(false, false, 0)
                    }
                    sdl2::mouse::MouseButton::X1 => {
                        input.mouse.has_release_event = true;
                        input.mouse.button_x1.process_event(false, false, 0)
                    }
                    sdl2::mouse::MouseButton::X2 => {
                        input.mouse.has_release_event = true;
                        input.mouse.button_x2.process_event(false, false, 0)
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

        // Mouse x in [0, screen_framebuffer_width - 1]  (left to right)
        // Mouse y in [0, screen_framebuffer_height - 1] (top to bottom)
        //
        // NOTE: We get the mouse position and delta from querying SDL instead of accumulating
        //       events, as it is faster, more accurate and less error-prone
        input.mouse.pos_x = event_pump.mouse_state().x();
        input.mouse.pos_y = event_pump.mouse_state().y();
        input.mouse.delta_x = input.mouse.pos_x - mouse_pos_previous_x;
        input.mouse.delta_y = input.mouse.pos_y - mouse_pos_previous_y;

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

        let post_input_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Timings, update and drawing

        let pre_update_time = post_input_time;

        let duration_frame = pre_update_time
            .duration_since(frame_start_time)
            .as_secs_f32();
        frame_start_time = pre_update_time;

        input.deltatime = duration_frame;
        input.target_deltatime = target_seconds_per_frame;
        input.real_world_uptime = frame_start_time
            .duration_since(launcher_start_time)
            .as_secs_f64();

        let current_audio_frame_index = { audio_output.lock().unwrap().get_audio_time_in_frames() };
        game_memory.update(&input, current_audio_frame_index, &mut systemcommands);

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

        let post_update_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Sound output

        let pre_sound_time = post_update_time;

        // NOTE: Whatever happens in block needs to be fast because we are effectively
        //       locking our audio callback thread here
        {
            let mut audio_output = audio_output.lock().unwrap();
            let first_chunk_index = audio_output.next_chunk_index;
            game_memory
                .audio
                .as_mut()
                .expect("No audiostate initialized")
                .render_audio(
                    first_chunk_index,
                    AUDIO_BUFFERSIZE_IN_CHUNKS,
                    &mut audiochunks,
                );
            audio_output.replace_chunks(first_chunk_index, &mut audiochunks);
        }

        let post_sound_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Drawcommands

        let pre_render_time = post_sound_time;

        renderer.process_drawcommands(
            input.screen_framebuffer_width,
            input.screen_framebuffer_height,
            &game_memory
                .draw
                .as_ref()
                .expect("No drawstate initialized")
                .drawcommands,
        );

        let post_render_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Swap framebuffers

        let pre_swap_time = post_render_time;

        window.sdl_window.gl_swap_window();

        let post_swap_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Wait for target frame time

        let pre_wait_time = post_swap_time;

        if !vsync_enabled {
            // NOTE: We need to manually wait to reach the target seconds a frame can take
            loop {
                let time_left_till_flip = target_seconds_per_frame
                    - std::time::Instant::now()
                        .duration_since(post_wait_time)
                        .as_secs_f32();

                if time_left_till_flip > 0.002 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                } else {
                    // NOTE: busywait in this loop
                }

                if time_left_till_flip <= 0.0 {
                    break;
                }
            }
        }

        post_wait_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Debug timing output

        let duration_input = post_input_time.duration_since(pre_input_time).as_secs_f32();
        let duration_update = post_update_time
            .duration_since(pre_update_time)
            .as_secs_f32();
        let duration_sound = post_sound_time.duration_since(pre_sound_time).as_secs_f32();
        let duration_render = post_render_time
            .duration_since(pre_render_time)
            .as_secs_f32();
        let duration_swap = post_swap_time.duration_since(pre_swap_time).as_secs_f32();
        let duration_wait = post_wait_time.duration_since(pre_wait_time).as_secs_f32();

        debug_print_counter += 1;
        debug_print_counter = debug_print_counter % 1;
        if debug_print_counter == 0 {
            log::trace!(
                  "frame: {:.3}ms  input: {:.3}ms  update: {:.3}ms  sound: {:.3}ms  render: {:.3}ms  swap: {:.3}ms  idle: {:.3}ms",
                  duration_frame * 1000.0,
                  duration_input * 1000.0,
                  duration_update * 1000.0,
                  duration_sound * 1000.0,
                  duration_render * 1000.0,
                  duration_swap * 1000.0,
                  duration_wait * 1000.0,
              );
        }
    }

    //--------------------------------------------------------------------------------------
    // Mainloop stopped

    let duration_gameplay = std::time::Instant::now()
        .duration_since(game_start_time)
        .as_secs_f64();
    log::debug!("Playtime: {:.3}s", duration_gameplay);
}
