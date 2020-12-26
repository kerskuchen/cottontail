use crate::core::log;
use ct_lib_audio::{
    audio::{AudioChunkStereo, AUDIO_CHUNKSIZE_IN_FRAMES},
    AudioFrame,
};

const AUDIO_SAMPLE_RATE: usize = 44100;
const AUDIO_BUFFER_FRAMECOUNT: usize = 2 * AUDIO_CHUNKSIZE_IN_FRAMES;
const AUDIO_QUEUE_FRAMECOUNT: usize = 2 * AUDIO_BUFFER_FRAMECOUNT;
const AUDIO_NUM_CHANNELS: usize = 2;

#[derive(Eq, PartialEq)]
enum AudioFadeState {
    FadingOut,
    FadedOut,
    FadingIn,
}

struct SDLAudioCallback {
    input_ringbuffer: ringbuf::Consumer<AudioFrame>,

    // This is used fade in / out the volume when we drop frames to reduce clicking
    fadestate: AudioFadeState,
    fader_current: f32,
    last_frame_written: AudioFrame,
}
impl SDLAudioCallback {
    fn new(audio_buffer_consumer: ringbuf::Consumer<AudioFrame>) -> SDLAudioCallback {
        SDLAudioCallback {
            input_ringbuffer: audio_buffer_consumer,
            fader_current: 0.0,
            last_frame_written: AudioFrame::silence(),
            fadestate: AudioFadeState::FadedOut,
        }
    }
}
impl sdl2::audio::AudioCallback for SDLAudioCallback {
    type Channel = f32;

    fn callback(&mut self, out_samples_stereo: &mut [f32]) {
        for frame_out in out_samples_stereo.chunks_exact_mut(2) {
            match self.fadestate {
                AudioFadeState::FadingOut => {
                    self.fader_current -= 1.0 / 2048.0;
                    if self.fader_current <= 0.0 {
                        self.fader_current = 0.0;
                        self.fadestate = AudioFadeState::FadedOut;
                    }
                }
                AudioFadeState::FadedOut => self.fader_current = 0.0,
                AudioFadeState::FadingIn => {
                    self.fader_current = f32::min(1.0, self.fader_current + 1.0 / 4096.0);
                }
            }

            if let Some(frame) = self.input_ringbuffer.pop() {
                if self.fadestate == AudioFadeState::FadedOut {
                    self.fadestate = AudioFadeState::FadingIn;
                }
                if self.fadestate == AudioFadeState::FadingOut {
                    frame_out[0] = self.fader_current * self.last_frame_written.left;
                    frame_out[1] = self.fader_current * self.last_frame_written.right;
                } else {
                    self.last_frame_written = frame;
                    frame_out[0] = self.fader_current * frame.left;
                    frame_out[1] = self.fader_current * frame.right;
                }
            } else {
                self.fadestate = AudioFadeState::FadingOut;
                frame_out[0] = self.fader_current * self.last_frame_written.left;
                frame_out[1] = self.fader_current * self.last_frame_written.right;
            }
        }
    }
}
pub struct AudioOutput {
    pub audio_playback_rate_hz: usize,
    frames_queue: ringbuf::Producer<AudioFrame>,
    _sdl_audio_device: sdl2::audio::AudioDevice<SDLAudioCallback>,
}
impl AudioOutput {
    pub fn new(sdl_context: &sdl2::Sdl) -> AudioOutput {
        let audio_format_desired = sdl2::audio::AudioSpecDesired {
            freq: Some(AUDIO_SAMPLE_RATE as i32),
            channels: Some(AUDIO_NUM_CHANNELS as u8),
            // IMPORTANT: `samples` is a misnomer - it is actually the frames
            samples: Some(AUDIO_BUFFER_FRAMECOUNT as u16),
        };

        let audio_ringbuffer = ringbuf::RingBuffer::new(AUDIO_SAMPLE_RATE);
        let (audio_ringbuffer_producer, audio_ringbuffer_consumer) = audio_ringbuffer.split();

        let sdl_audio = sdl_context
            .audio()
            .expect("Failed to initialize SDL2 audio");
        let audio_device = sdl_audio
            .open_playback(None, &audio_format_desired, |spec| {
                assert!(
                    spec.freq == AUDIO_SAMPLE_RATE as i32,
                    "Cannot initialize audio output with frequency {}",
                    AUDIO_SAMPLE_RATE
                );
                assert!(
                    spec.channels == AUDIO_NUM_CHANNELS as u8,
                    "Cannot initialize audio output with channel count {}",
                    AUDIO_NUM_CHANNELS
                );
                assert!(
                    spec.samples == AUDIO_BUFFER_FRAMECOUNT as u16,
                    "Cannot initialize audio output audiobuffersize {}",
                    AUDIO_BUFFER_FRAMECOUNT
                );

                SDLAudioCallback::new(audio_ringbuffer_consumer)
            })
            .expect("Cannot initialize audio output");
        audio_device.resume();

        log::info!(
            "Opened audio channel on default output device: (frequency: {}, channelcount: {})",
            AUDIO_SAMPLE_RATE,
            AUDIO_NUM_CHANNELS,
        );

        AudioOutput {
            _sdl_audio_device: audio_device,
            audio_playback_rate_hz: AUDIO_SAMPLE_RATE,
            frames_queue: audio_ringbuffer_producer,
        }
    }

    pub fn reset(&mut self) {
        // Do nothing here
    }

    pub fn get_num_chunks_to_submit(&self) -> usize {
        let framecount_to_render = {
            let framecount_queued = self.frames_queue.len();
            dbg!(framecount_queued);
            if framecount_queued < AUDIO_QUEUE_FRAMECOUNT {
                AUDIO_QUEUE_FRAMECOUNT - framecount_queued
            } else {
                0
            }
        };
        framecount_to_render / AUDIO_CHUNKSIZE_IN_FRAMES
    }

    pub fn submit_chunk(&mut self, audio_chunk: &AudioChunkStereo) {
        for frame in audio_chunk {
            if let Err(_) = self.frames_queue.push(*frame) {
                log::warn!("Audiobuffer: Could not push frame to queue - queue full?");
            }
        }
    }
}
