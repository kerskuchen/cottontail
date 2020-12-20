use std::{cell::RefCell, rc::Rc};

use ct_lib::audio::{AudioChunkStereo, AudioFrame, Audiostate, AUDIO_CHUNKSIZE_IN_FRAMES};
use wasm_bindgen::{prelude::*, JsCast};

const AUDIO_SAMPLE_RATE: usize = 44100;
const AUDIO_BUFFER_FRAME_COUNT: usize = 2048;
const AUDIO_NUM_CHANNELS: usize = 2;
const AUDIO_CHANNEL_LEFT: usize = 0;
const AUDIO_CHANNEL_RIGHT: usize = 1;

#[derive(Eq, PartialEq)]
enum AudioFadeState {
    FadingOut,
    FadedOut,
    FadingIn,
}

struct WASMAudioCallback {
    input_ringbuffer: ringbuf::Consumer<AudioFrame>,

    // This is used fade in / out the volume when we drop frames to reduce clicking
    fadestate: AudioFadeState,
    fader_current: f32,
    last_frame_written: AudioFrame,
}
impl WASMAudioCallback {
    fn new(audio_buffer_consumer: ringbuf::Consumer<AudioFrame>) -> WASMAudioCallback {
        WASMAudioCallback {
            input_ringbuffer: audio_buffer_consumer,
            fader_current: 0.0,
            last_frame_written: AudioFrame::silence(),
            fadestate: AudioFadeState::FadedOut,
        }
    }
}

pub struct AudioOutput {
    pub audio_playback_rate_hz: usize,
    samples_queue: ringbuf::Producer<AudioFrame>,
    _audio_context: Rc<RefCell<web_sys::AudioContext>>,
    _audio_processor: web_sys::ScriptProcessorNode,
}

impl AudioOutput {
    pub fn new() -> AudioOutput {
        let mut audio_options = web_sys::AudioContextOptions::new();
        audio_options.sample_rate(AUDIO_SAMPLE_RATE as f32);

        let audio_context = Rc::new(RefCell::new(
            web_sys::AudioContext::new_with_context_options(&audio_options)
                .expect("WebAudio not available"),
        ));
        let audio_processor = audio_context.borrow().create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(AUDIO_BUFFER_FRAME_COUNT as u32, 0, AUDIO_NUM_CHANNELS as u32)
        .expect("Could not create AudioProcessor node");

        let audio_ringbuffer = ringbuf::RingBuffer::new(1 * AUDIO_SAMPLE_RATE);
        let (audio_ringbuffer_producer, audio_ringbuffer_consumer) = audio_ringbuffer.split();
        {
            let mut audio_callback_context = WASMAudioCallback::new(audio_ringbuffer_consumer);
            let mut channel_output_left = vec![0f32; AUDIO_BUFFER_FRAME_COUNT];
            let mut channel_output_right = vec![0f32; AUDIO_BUFFER_FRAME_COUNT];

            let audio_callback =
                Closure::wrap(Box::new(move |event: web_sys::AudioProcessingEvent| {
                    let output_buffer = event.output_buffer().unwrap();
                    let num_frames = output_buffer.length() as usize;
                    let num_channels = output_buffer.number_of_channels() as usize;
                    assert!(num_frames == AUDIO_BUFFER_FRAME_COUNT);
                    assert!(num_channels == AUDIO_NUM_CHANNELS);

                    // Deinterleave and write output frames
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
                            AudioFadeState::FadedOut => audio_callback_context.fader_current = 0.0,
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
                                    * audio_callback_context.last_frame_written.left;
                                *frame_out_right = audio_callback_context.fader_current
                                    * audio_callback_context.last_frame_written.right;
                            } else {
                                audio_callback_context.last_frame_written = frame;
                                *frame_out_left = audio_callback_context.fader_current * frame.left;
                                *frame_out_right =
                                    audio_callback_context.fader_current * frame.right;
                            }
                        } else {
                            audio_callback_context.fadestate = AudioFadeState::FadingOut;
                            *frame_out_left = audio_callback_context.fader_current
                                * audio_callback_context.last_frame_written.left;
                            *frame_out_right = audio_callback_context.fader_current
                                * audio_callback_context.last_frame_written.right;
                        }
                    }

                    output_buffer
                        .copy_to_channel(&mut channel_output_left, AUDIO_CHANNEL_LEFT as i32)
                        .expect("Unable to write sample data into the audio context buffer");
                    output_buffer
                        .copy_to_channel(&mut channel_output_right, AUDIO_CHANNEL_RIGHT as i32)
                        .expect("Unable to write sample data into the audio context buffer");
                }) as Box<dyn FnMut(_)>);
            audio_processor.set_onaudioprocess(Some(audio_callback.as_ref().unchecked_ref()));
            audio_callback.forget();
        }
        audio_processor
            .connect_with_audio_node(&audio_context.borrow().destination())
            .expect("Could not connect AudioScriptProcessor node");

        // Activation callbacks
        // NOTE: Need enable audio here because of browser UX limitations
        let document = web_sys::window()
            .expect("no global `window` exists")
            .document()
            .expect("should have a document on window");
        let mut callback_options = web_sys::AddEventListenerOptions::new();
        callback_options.once(true);
        {
            let audio_context = audio_context.clone();
            let click_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                let audio_context = audio_context.borrow();
                if audio_context.state() == web_sys::AudioContextState::Suspended {
                    audio_context.resume().ok();
                    log::info!("Audio output activated by user action");
                }
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "click",
                    click_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .expect("Could not set 'click' callback for audio activation");
            click_callback.forget();
        }
        {
            let audio_context = audio_context.clone();
            let touchstart_callback = Closure::wrap(Box::new(move |_event: web_sys::TouchEvent| {
                let audio_context = audio_context.borrow();
                if audio_context.state() == web_sys::AudioContextState::Suspended {
                    audio_context.resume().ok();
                    log::info!("Audio output activated by user action");
                }
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "touchstart",
                    touchstart_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .expect("Could not set 'touchstart' callback for audio activation");
            touchstart_callback.forget();
        }
        {
            let audio_context = audio_context.clone();
            let keydown_callback = Closure::wrap(Box::new(move |_event: web_sys::KeyboardEvent| {
                let audio_context = audio_context.borrow();
                if audio_context.state() == web_sys::AudioContextState::Suspended {
                    audio_context.resume().ok();
                    log::info!("Audio output activated by user action");
                }
            }) as Box<dyn FnMut(_)>);
            document
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "keydown",
                    keydown_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .expect("Could not set 'keydown' callback for audio activation");
            keydown_callback.forget();
        }

        log::info!(
            "Opened audio channel on default output device: (samplerate: {}, channelcount: {})",
            AUDIO_SAMPLE_RATE,
            AUDIO_NUM_CHANNELS,
        );

        AudioOutput {
            audio_playback_rate_hz: AUDIO_SAMPLE_RATE,
            samples_queue: audio_ringbuffer_producer,
            _audio_context: audio_context,
            _audio_processor: audio_processor,
        }
    }

    fn submit_rendered_chunk(&mut self, chunk: &AudioChunkStereo) {
        for frame in chunk.iter() {
            if let Err(_) = self.samples_queue.push(*frame) {
                log::warn!("Audiobuffer: Could not push frame to queue - queue full?");
            }
        }
    }

    pub fn render_frames(&mut self, audio: &mut Audiostate, window_has_focus: bool) {
        let chunkcount_to_render = {
            let framecount_to_render = {
                let framecount_queued = self.samples_queue.len() / 2;
                if framecount_queued < AUDIO_BUFFER_FRAME_COUNT {
                    AUDIO_BUFFER_FRAME_COUNT - framecount_queued
                } else {
                    0
                }
            };
            (framecount_to_render as f32 / AUDIO_CHUNKSIZE_IN_FRAMES as f32).ceil() as usize
        };

        for _ in 0..chunkcount_to_render {
            let mut out_chunk = [AudioFrame::silence(); AUDIO_CHUNKSIZE_IN_FRAMES];
            audio.render_audio_chunk(&mut out_chunk);
            if window_has_focus {
                // NOTE: We want to avoid submitting frames because we cannot guarentee that it will
                //       sound ok when our window is not in focus. We still want to let the
                //       Audiostate render chunks though so that it can keep track of time.
                //       When not submitting new frames the callback will automatically fade out
                //       to avoid cracking
                self.submit_rendered_chunk(&out_chunk);
            }
        }
    }
}
