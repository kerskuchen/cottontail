use ct_lib_core::log;

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use wasm_bindgen::{prelude::*, JsCast};

use super::html_get_document;

const AUDIO_SAMPLE_RATE: usize = 44100;
const AUDIO_BUFFER_FRAMECOUNT: usize = 512;
const AUDIO_NUM_CHANNELS: usize = 2;
const AUDIO_CHANNEL_LEFT: usize = 0;
const AUDIO_CHANNEL_RIGHT: usize = 1;
const AUDIO_BUFFER_LENGTH_SECONDS: f64 = AUDIO_BUFFER_FRAMECOUNT as f64 / AUDIO_SAMPLE_RATE as f64;

struct ScheduledAudioBuffer {
    start_time: f64,
    buffer: web_sys::AudioBuffer,
}

pub struct AudioOutput {
    playback_rate_hz: usize,
    context: Rc<RefCell<web_sys::AudioContext>>,
    buffers: VecDeque<ScheduledAudioBuffer>,
    next_schedule_time: f64,
}

impl AudioOutput {
    pub fn new() -> AudioOutput {
        let mut options = web_sys::AudioContextOptions::new();
        options.sample_rate(AUDIO_SAMPLE_RATE as f32);
        let context = Rc::new(RefCell::new(
            web_sys::AudioContext::new_with_context_options(&options)
                .unwrap_or_else(|error| panic!("Could not create WebAudio context: {:?}", error)),
        ));

        // Activation callbacks
        // NOTE: Need to enable audio here because of browser UX limitations
        {
            let context = context.clone();
            let resume_callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if context.borrow().resume().is_ok() {
                    log::info!("Audio output activated by user action");
                }
            }) as Box<dyn FnMut(_)>);

            let mut callback_options = web_sys::AddEventListenerOptions::new();
            callback_options.once(true);
            html_get_document()
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "click",
                    resume_callback.as_ref().unchecked_ref(),
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
                    resume_callback.as_ref().unchecked_ref(),
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
                    resume_callback.as_ref().unchecked_ref(),
                    &callback_options,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "Could not set 'keydown' callback for audio activation: {:?}",
                        error
                    )
                });

            resume_callback.forget();
        }

        log::info!(
            "Opened audio channel on default output device: (samplerate: {}, channelcount: {})",
            AUDIO_SAMPLE_RATE,
            AUDIO_NUM_CHANNELS,
        );

        AudioOutput {
            playback_rate_hz: AUDIO_SAMPLE_RATE,
            context,
            buffers: VecDeque::new(),
            next_schedule_time: 0.0,
        }
    }

    pub fn get_audio_playback_rate_hz(&self) -> usize {
        self.playback_rate_hz
    }

    pub fn get_num_frames_in_queue(&self) -> usize {
        let buffers_end_time = self
            .buffers
            .back()
            .map(|buffer| buffer.start_time + AUDIO_BUFFER_LENGTH_SECONDS)
            .unwrap_or(0.0);
        let current_time = self.context.borrow().current_time();
        let time_left_to_play = f64::max(0.0, buffers_end_time - current_time);
        (time_left_to_play * AUDIO_SAMPLE_RATE as f64) as usize
    }

    pub fn get_audiobuffer_size_in_frames(&self) -> usize {
        4 * AUDIO_BUFFER_FRAMECOUNT
    }

    pub fn submit_frames(&mut self, samples_left: &[f32], samples_right: &[f32]) {
        assert!(samples_left.len() == samples_right.len());
        assert!(
            samples_left.len() == AUDIO_BUFFER_FRAMECOUNT,
            "Submitted framecount needs to be {}",
            AUDIO_BUFFER_FRAMECOUNT
        );

        // Get next free buffer
        let current_time = self.context.borrow().current_time();
        let mut buffer = self.get_or_create_next_free_buffer(current_time);

        // Copy frames
        // TODO: Remove these copies when (https://github.com/rustwasm/wasm-bindgen/issues/2434)
        //       is solved
        let mut samples_left = samples_left.to_vec();
        let mut samples_right = samples_left.to_vec();
        buffer
            .buffer
            .copy_to_channel(&mut samples_left, AUDIO_CHANNEL_LEFT as i32)
            .unwrap_or_else(|error| {
                panic!(
                    "Could not write sample data to left ouput channel: {:?}",
                    error
                )
            });
        buffer
            .buffer
            .copy_to_channel(&mut samples_right, AUDIO_CHANNEL_RIGHT as i32)
            .unwrap_or_else(|error| {
                panic!(
                    "Could not write sample data to right ouput channel: {:?}",
                    error
                )
            });

        // Prepare our buffer for playback
        let audio_buffer_source = self
            .context
            .borrow()
            .create_buffer_source()
            .unwrap_or_else(|error| panic!("Could not create audio buffer source: {:?}", error));
        audio_buffer_source.set_buffer(Some(&buffer.buffer));
        audio_buffer_source
            .connect_with_audio_node(&self.context.borrow().destination())
            .unwrap_or_else(|error| {
                panic!(
                    "Could not connect audio buffer source to output: {:?}",
                    error
                )
            });

        // Schedule playback
        if current_time > self.next_schedule_time {
            log::debug!("Skipped audio: {}s", current_time - self.next_schedule_time);
            self.next_schedule_time = current_time;
        }
        buffer.start_time = self.next_schedule_time;
        audio_buffer_source
            .start_with_when(buffer.start_time)
            .unwrap_or_else(|error| panic!("Could not start audio buffer source: {:?}", error));
        self.next_schedule_time += AUDIO_BUFFER_LENGTH_SECONDS;

        self.buffers.push_back(buffer);
    }

    fn get_or_create_next_free_buffer(&mut self, current_time: f64) -> ScheduledAudioBuffer {
        if !self.buffers.is_empty() {
            let buffer = self.buffers.front().unwrap();
            if buffer.start_time + AUDIO_BUFFER_LENGTH_SECONDS < current_time {
                return self.buffers.pop_front().unwrap();
            }
        }

        // We have not found a free buffer previously so we create a new one
        let buffer = self
            .context
            .borrow()
            .create_buffer(
                AUDIO_NUM_CHANNELS as u32,
                AUDIO_BUFFER_FRAMECOUNT as u32,
                AUDIO_SAMPLE_RATE as f32,
            )
            .unwrap_or_else(|error| panic!("Could not create Audio output buffer: {:?}", error));
        ScheduledAudioBuffer {
            start_time: -1.0,
            buffer,
        }
    }
}
