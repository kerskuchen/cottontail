mod renderer_opengl;
mod sdl_input;
mod sdl_window;

use ct_lib::audio::*;
use ct_lib::game::{
    GameAssets, GameInput, GameMemory, GameStateInterface, Scancode, SystemCommand,
};
use ct_lib::system;

use ct_lib::log;
use ct_lib::serde_derive::{Deserialize, Serialize};

use std::{collections::VecDeque, time::Duration};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Configuration

const ENABLE_PANIC_MESSAGES: bool = false;
const ENABLE_FRAMETIME_LOGGING: bool = false;
const ENABLE_AUDIO_LOGGING: bool = false;

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

#[derive(Eq, PartialEq)]
enum FadeState {
    FadingOut,
    FadedOut,
    FadingIn,
}

struct SDLAudioCallback {
    input_ringbuffer: ringbuf::Consumer<(i16, i16)>,

    // This is used fade in / out the volume when we drop frames to reduce clicking
    fadestate: FadeState,
    fader_current: f32,
    last_frame_written: (i16, i16),
}
impl SDLAudioCallback {
    fn new(audio_buffer_consumer: ringbuf::Consumer<(i16, i16)>) -> SDLAudioCallback {
        SDLAudioCallback {
            input_ringbuffer: audio_buffer_consumer,
            fader_current: 0.0,
            last_frame_written: (0, 0),
            fadestate: FadeState::FadedOut,
        }
    }
}
impl sdl2::audio::AudioCallback for SDLAudioCallback {
    type Channel = i16;

    fn callback(&mut self, out_samples_stereo: &mut [i16]) {
        debug_assert!(out_samples_stereo.len() % 2 == 0);

        //dbg!(self.fader_current);
        for frame_out in out_samples_stereo.chunks_exact_mut(2) {
            match self.fadestate {
                FadeState::FadingOut => {
                    self.fader_current -= 1.0 / 2048.0;
                    if self.fader_current <= 0.0 {
                        self.fader_current = 0.0;
                        self.fadestate = FadeState::FadedOut;
                    }
                }
                FadeState::FadedOut => self.fader_current = 0.0,
                FadeState::FadingIn => {
                    self.fader_current = f32::min(1.0, self.fader_current + 1.0 / 4096.0);
                }
            }

            if let Some(frame) = self.input_ringbuffer.pop() {
                if self.fadestate == FadeState::FadedOut {
                    self.fadestate = FadeState::FadingIn;
                }
                if self.fadestate == FadeState::FadingOut {
                    frame_out[0] = (self.fader_current * self.last_frame_written.0 as f32) as i16;
                    frame_out[1] = (self.fader_current * self.last_frame_written.1 as f32) as i16;
                } else {
                    self.last_frame_written = frame;
                    frame_out[0] = (self.fader_current * frame.0 as f32) as i16;
                    frame_out[1] = (self.fader_current * frame.1 as f32) as i16;
                }
            } else {
                self.fadestate = FadeState::FadingOut;
                frame_out[0] = (self.fader_current * self.last_frame_written.0 as f32) as i16;
                frame_out[1] = (self.fader_current * self.last_frame_written.1 as f32) as i16;
            }
        }
    }
}
pub struct AudioOutputSDL {
    pub audio_playback_rate_hz: usize,
    samples_queue: ringbuf::Producer<(i16, i16)>,
    render_buffer: Vec<AudioFrame>,
    _sdl_audio_device: sdl2::audio::AudioDevice<SDLAudioCallback>,
}

impl AudioOutputSDL {
    pub fn new(sdl_context: &sdl2::Sdl) -> AudioOutputSDL {
        let audio_playback_rate_hz = 48000;
        let audio_channelcount = 2;
        let audio_format_desired = sdl2::audio::AudioSpecDesired {
            freq: Some(audio_playback_rate_hz as i32),
            channels: Some(audio_channelcount as u8),
            // IMPORTANT: `samples` is a misnomer - it is actually the frames
            samples: Some(256 as u16),
        };

        let audio_ringbuffer = ringbuf::RingBuffer::new(4 * audio_playback_rate_hz);
        let (audio_ringbuffer_producer, audio_ringbuffer_consumer) = audio_ringbuffer.split();

        let sdl_audio = sdl_context
            .audio()
            .expect("Failed to initialize SDL2 audio");
        let audio_device = sdl_audio
            .open_playback(None, &audio_format_desired, |spec| {
                assert!(
                    spec.freq == audio_playback_rate_hz as i32,
                    "Cannot initialize audio output with frequency {}",
                    audio_playback_rate_hz
                );
                assert!(
                    spec.channels == audio_channelcount as u8,
                    "Cannot initialize audio output with channel count {}",
                    audio_channelcount
                );
                assert!(
                    spec.samples == 256 as u16,
                    "Cannot initialize audio output audiobuffersize {}",
                    256
                );

                SDLAudioCallback::new(audio_ringbuffer_consumer)
            })
            .expect("Cannot initialize audio output");
        audio_device.resume();

        log::info!(
            "Opened audio channel on default output device: (frequency: {}, channelcount: {})",
            audio_playback_rate_hz,
            audio_channelcount,
        );

        AudioOutputSDL {
            _sdl_audio_device: audio_device,
            audio_playback_rate_hz,
            samples_queue: audio_ringbuffer_producer,
            render_buffer: Vec::with_capacity(4 * audio_playback_rate_hz),
        }
    }

    pub fn prepare_empty_render_buffer(
        &mut self,
        minimum_framecount_buffered: usize,
    ) -> &mut [AudioFrame] {
        let framecount_to_render = {
            let framecount_queued = self.samples_queue.len() / 2;
            if framecount_queued < minimum_framecount_buffered {
                minimum_framecount_buffered - framecount_queued
            } else {
                0
            }
        };
        self.render_buffer.reserve(framecount_to_render);
        self.render_buffer.clear();
        for _ in 0..framecount_to_render {
            self.render_buffer.push(AudioFrame::silence());
        }
        &mut self.render_buffer[..framecount_to_render]
    }

    fn submit_rendered_frames(&mut self) {
        for frame in self.render_buffer.drain(..) {
            if let Err(_) = self.samples_queue.push((
                (frame.left * std::i16::MAX as f32) as i16,
                (frame.right * std::i16::MAX as f32) as i16,
            )) {
                log::warn!("Audiobuffer: Could not push frame to queue - queue full?");
            }
        }
    }

    fn render_frames(
        &mut self,
        audio: &mut Audiostate,
        assets: &GameAssets,
        input: &GameInput,
        minimum_seconds_to_buffer: f32,
    ) {
        let minimum_frames_in_queue =
            (self.audio_playback_rate_hz as f32 * minimum_seconds_to_buffer) as usize;
        let mut audio_frames_to_queue = self.prepare_empty_render_buffer(minimum_frames_in_queue);
        if audio_frames_to_queue.len() > 0 {
            audio.render_audio(&mut audio_frames_to_queue, assets.get_audio_recordings());
            if !input.keyboard.is_down(Scancode::Q) {
                self.submit_rendered_frames();
            }
        }
    }
}

fn log_frametimes(
    _duration_frame: f32,
    _duration_input: f32,
    _duration_update: f32,
    _duration_sound: f32,
    _duration_render: f32,
    _duration_swap: f32,
    _duration_wait: f32,
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
    let launcher_start_time = std::time::Instant::now();
    let game_config = GameStateType::get_game_config();
    let savedata_dir = system::get_savegame_dir(
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

    let logfile_path = ct_lib::system::path_join(&savedata_dir, "logging.txt");
    if let Err(error) = ct_lib::init_logging(&logfile_path, log::LevelFilter::Trace) {
        sdl_window::Window::show_error_messagebox(&format!(
            "Could not initialize logger at '{}' : {}",
            &logfile_path, error,
        ));
    }

    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("{}", panic_info);

        if ENABLE_PANIC_MESSAGES {
            let logfile_path_canonicalized =
                system::path_canonicalize(&logfile_path).unwrap_or(logfile_path.to_string());
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
        let config_filepath = system::path_join(&savedata_dir, "launcher_config.json");
        if system::path_exists(&config_filepath) {
            ct_lib::deserialize_from_file_json(&config_filepath)
        } else {
            let config = LauncherConfig {
                display_index_to_use: 0,
                controller_deadzone_threshold_x: 0.1,
                controller_deadzone_threshold_y: 0.1,
            };
            ct_lib::serialize_to_file_json(&config, &config_filepath);
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

    let mut audio_output = AudioOutputSDL::new(&sdl_context);

    // ---------------------------------------------------------------------------------------------
    // Input

    let mut gamepad_subsystem = {
        let savedata_mappings_path = system::path_join(&savedata_dir, "gamecontrollerdb.txt");
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
        input.audio_playback_rate_hz = audio_output.audio_playback_rate_hz;

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

        let post_update_time = std::time::Instant::now();

        //--------------------------------------------------------------------------------------
        // Sound output

        let pre_sound_time = post_update_time;

        let audio = game_memory
            .audio
            .as_mut()
            .expect("No audiostate initialized");
        let assets = game_memory
            .assets
            .as_ref()
            .expect("No audio assets initialized");
        audio_output.render_frames(audio, assets, &input, 2.0 * target_seconds_per_frame);

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

    let duration_gameplay = std::time::Instant::now()
        .duration_since(game_start_time)
        .as_secs_f64();
    log::debug!("Playtime: {:.3}s", duration_gameplay);

    // Make sure our sound output has time to wind down
    std::thread::sleep(Duration::from_secs_f32(2.0 * target_seconds_per_frame))
}
