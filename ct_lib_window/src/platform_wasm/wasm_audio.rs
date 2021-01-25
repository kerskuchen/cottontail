use ct_lib_core::log;

use std::sync::{Arc, RwLock};

use wasm_bindgen::{prelude::*, JsCast};

use super::{html_get_document, html_get_window};

const AUDIO_SAMPLE_RATE: usize = 44100;
const AUDIO_BUFFER_FRAMECOUNT: usize = 2048;
const AUDIO_NUM_CHANNELS: usize = 2;
const AUDIO_CHANNEL_LEFT: usize = 0;
const AUDIO_CHANNEL_RIGHT: usize = 1;
const AUDIO_BUFFER_LENGTH_SECONDS: f64 = AUDIO_BUFFER_FRAMECOUNT as f64 / AUDIO_SAMPLE_RATE as f64;

#[derive(Eq, PartialEq)]
enum AudioFadeState {
    FadingOut,
    FadedOut,
    FadingIn,
}

struct WASMAudioCallback {
    input_ringbuffer: ringbuf::Consumer<(f32, f32)>,

    // This is used to fade in / out the volume when we drop frames to reduce clicking
    fadestate: AudioFadeState,
    fader_current: f32,
    last_frame_written: (f32, f32),
}
impl WASMAudioCallback {
    fn new(audio_buffer_consumer: ringbuf::Consumer<(f32, f32)>) -> WASMAudioCallback {
        WASMAudioCallback {
            input_ringbuffer: audio_buffer_consumer,
            fader_current: 0.0,
            last_frame_written: (0.0, 0.0),
            fadestate: AudioFadeState::FadedOut,
        }
    }
}

pub struct AudioOutput {
    audio_playback_rate_hz: usize,
    frames_queue: ringbuf::Producer<(f32, f32)>,
    _audio_context: Arc<web_sys::AudioContext>,
    _swapchain_callbacks: Vec<Arc<RwLock<Option<Closure<dyn FnMut()>>>>>,
    _swapchain_is_running: Arc<RwLock<bool>>,
}

impl AudioOutput {
    pub fn new() -> AudioOutput {
        // Audio context
        let mut audio_options = web_sys::AudioContextOptions::new();
        audio_options.sample_rate(AUDIO_SAMPLE_RATE as f32);
        let audio_context = Arc::new(
            web_sys::AudioContext::new_with_context_options(&audio_options)
                .unwrap_or_else(|error| panic!("Could not create WebAudio context: {:?}", error)),
        );

        let audio_current_time = Arc::new(RwLock::new(0.0));
        let audio_ringbuffer = ringbuf::RingBuffer::new(AUDIO_SAMPLE_RATE);
        let (audio_ringbuffer_producer, audio_ringbuffer_consumer) = audio_ringbuffer.split();
        let audio_callback_context = Arc::new(RwLock::new(WASMAudioCallback::new(
            audio_ringbuffer_consumer,
        )));

        // Create buffers and its swap-chain callbacks
        let mut swapchain_callbacks: Vec<Arc<RwLock<Option<Closure<dyn FnMut()>>>>> = Vec::new();
        for _swapchain_index in 0..2 {
            let audio_context = audio_context.clone();
            let audio_current_time = audio_current_time.clone();
            let audio_callback_context = audio_callback_context.clone();

            let mut channel_output_left = vec![0f32; AUDIO_BUFFER_FRAMECOUNT];
            let mut channel_output_right = vec![0f32; AUDIO_BUFFER_FRAMECOUNT];
            let audio_context_buffer = audio_context
                .create_buffer(
                    AUDIO_NUM_CHANNELS as u32,
                    AUDIO_BUFFER_FRAMECOUNT as u32,
                    AUDIO_SAMPLE_RATE as f32,
                )
                .unwrap_or_else(|error| {
                    panic!("Could not create Audio output buffer: {:?}", error)
                });

            let swapchain_callback: Arc<RwLock<Option<Closure<dyn FnMut()>>>> =
                Arc::new(RwLock::new(None));
            let swapchain_callback_clone = swapchain_callback.clone();

            swapchain_callback
                .write()
                .unwrap()
                .replace(Closure::wrap(Box::new(move || {
                    // Deinterleave and write output frames
                    {
                        let mut audio_callback_context = audio_callback_context.write().unwrap();

                        for (frame_out_left, frame_out_right) in channel_output_left
                            .iter_mut()
                            .zip(channel_output_right.iter_mut())
                        {
                            match audio_callback_context.fadestate {
                                AudioFadeState::FadingOut => {
                                    audio_callback_context.fader_current -= 1.0 / 2048.0;
                                    if audio_callback_context.fader_current <= 0.0 {
                                        audio_callback_context.fader_current = 0.0;
                                        audio_callback_context.fadestate = AudioFadeState::FadedOut;
                                    }
                                }
                                AudioFadeState::FadedOut => {
                                    audio_callback_context.fader_current = 0.0
                                }
                                AudioFadeState::FadingIn => {
                                    audio_callback_context.fader_current = f32::min(
                                        1.0,
                                        audio_callback_context.fader_current + 1.0 / 4096.0,
                                    );
                                }
                            }

                            if let Some(frame) = audio_callback_context.input_ringbuffer.pop() {
                                if audio_callback_context.fadestate == AudioFadeState::FadedOut {
                                    audio_callback_context.fadestate = AudioFadeState::FadingIn;
                                }
                                if audio_callback_context.fadestate == AudioFadeState::FadingOut {
                                    *frame_out_left = audio_callback_context.fader_current
                                        * audio_callback_context.last_frame_written.0;
                                    *frame_out_right = audio_callback_context.fader_current
                                        * audio_callback_context.last_frame_written.1;
                                } else {
                                    audio_callback_context.last_frame_written = frame;
                                    *frame_out_left =
                                        audio_callback_context.fader_current * frame.0;
                                    *frame_out_right =
                                        audio_callback_context.fader_current * frame.1;
                                }
                            } else {
                                audio_callback_context.fadestate = AudioFadeState::FadingOut;
                                *frame_out_left = audio_callback_context.fader_current
                                    * audio_callback_context.last_frame_written.0;
                                *frame_out_right = audio_callback_context.fader_current
                                    * audio_callback_context.last_frame_written.1;
                            }
                        }

                        audio_context_buffer
                            .copy_to_channel(&mut channel_output_left, AUDIO_CHANNEL_LEFT as i32)
                            .unwrap_or_else(|error| {
                                panic!(
                                    "Could not write sample data to left ouput channel: {:?}",
                                    error
                                )
                            });
                        audio_context_buffer
                            .copy_to_channel(&mut channel_output_right, AUDIO_CHANNEL_RIGHT as i32)
                            .unwrap_or_else(|error| {
                                panic!(
                                    "Could not write sample data to right ouput channel: {:?}",
                                    error
                                )
                            });
                    }

                    // Prepare our buffer for playback
                    let audio_buffer_source =
                        audio_context
                            .create_buffer_source()
                            .unwrap_or_else(|error| {
                                panic!("Could not create audio buffer source: {:?}", error)
                            });
                    audio_buffer_source.set_buffer(Some(&audio_context_buffer));
                    audio_buffer_source
                        .connect_with_audio_node(&audio_context.destination())
                        .unwrap_or_else(|error| {
                            panic!(
                                "Could not connect audio buffer source to output: {:?}",
                                error
                            )
                        });
                    audio_buffer_source.set_onended(Some(
                        swapchain_callback_clone
                            .read()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .as_ref()
                            .unchecked_ref(),
                    ));

                    // Schedule the playback
                    let buffer_start_time = *audio_current_time.read().unwrap();
                    audio_buffer_source
                        .start_with_when(buffer_start_time)
                        .unwrap_or_else(|error| {
                            panic!("Could not start audio buffer source: {:?}", error)
                        });
                    *audio_current_time.write().unwrap() =
                        buffer_start_time + AUDIO_BUFFER_LENGTH_SECONDS;
                }) as Box<dyn FnMut()>));

            swapchain_callbacks.push(swapchain_callback);
        }

        // Activation callbacks
        // NOTE: Need to enable audio here because of browser UX limitations
        let swapchain_is_running = Arc::new(RwLock::new(false));
        {
            let audio_current_time = audio_current_time.clone();
            let audio_context = audio_context.clone();
            let swapchain_callbacks = swapchain_callbacks.clone();
            let swapchain_is_running = swapchain_is_running.clone();

            let audio_resume_callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if audio_context.resume().is_ok() {
                    log::info!("Audio output activated by user action");
                    let mut started_callback_chain = swapchain_is_running.write().unwrap();
                    if !*started_callback_chain {
                        *audio_current_time.write().unwrap() = audio_context.current_time();
                        let mut start_time_ms = 0;
                        for swapchain_callback in swapchain_callbacks.iter() {
                            html_get_window()
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    swapchain_callback
                                        .read()
                                        .unwrap()
                                        .as_ref()
                                        .unwrap()
                                        .as_ref()
                                        .unchecked_ref(),
                                    start_time_ms,
                                )
                                .unwrap();
                            // NOTE: We divide by 2 so we begin earlier to not miss
                            start_time_ms += ((AUDIO_BUFFER_LENGTH_SECONDS * 1000.0) as i32) / 2;
                        }
                        *started_callback_chain = true;
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let mut callback_options = web_sys::AddEventListenerOptions::new();
            callback_options.once(true);
            html_get_document()
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "click",
                    audio_resume_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not set 'click' callback for audio activation: {:?}",
                        error
                    )
                });
            html_get_document()
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "touchstart",
                    audio_resume_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not set 'touchstart' callback for audio activation: {:?}",
                        error
                    )
                });
            html_get_document()
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "keydown",
                    audio_resume_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not set 'keydown' callback for audio activation: {:?}",
                        error
                    )
                });

            audio_resume_callback.forget();
        }

        log::info!(
            "Opened audio channel on default output device: (samplerate: {}, channelcount: {})",
            AUDIO_SAMPLE_RATE,
            AUDIO_NUM_CHANNELS,
        );

        AudioOutput {
            audio_playback_rate_hz: AUDIO_SAMPLE_RATE,
            frames_queue: audio_ringbuffer_producer,
            _audio_context: audio_context,
            _swapchain_callbacks: swapchain_callbacks,
            _swapchain_is_running: swapchain_is_running,
        }
    }

    pub fn get_audio_playback_rate_hz(&self) -> usize {
        self.audio_playback_rate_hz
    }

    pub fn get_num_frames_in_queue(&self) -> usize {
        self.frames_queue.len()
    }

    pub fn get_audiobuffer_size_in_frames(&self) -> usize {
        AUDIO_BUFFER_FRAMECOUNT
    }

    pub fn submit_frames(&mut self, samples_left: &[f32], samples_right: &[f32]) {
        assert!(samples_left.len() == samples_right.len());
        for (left, right) in samples_left.iter().zip(samples_right.iter()) {
            if let Err(_) = self.frames_queue.push((*left, *right)) {
                log::warn!("Audiobuffer: Could not push frame to queue - queue full?");
            }
        }
    }
}
