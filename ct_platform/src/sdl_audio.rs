use super::log;
use ct_lib::audio::{AudioChunkStereo, AudioFrame, Audiostate, AUDIO_CHUNKSIZE_IN_FRAMES};

#[derive(Eq, PartialEq)]
enum AudioFadeState {
    FadingOut,
    FadedOut,
    FadingIn,
}

struct SDLAudioCallback {
    input_ringbuffer: ringbuf::Consumer<(i16, i16)>,

    // This is used fade in / out the volume when we drop frames to reduce clicking
    fadestate: AudioFadeState,
    fader_current: f32,
    last_frame_written: (i16, i16),
}
impl SDLAudioCallback {
    fn new(audio_buffer_consumer: ringbuf::Consumer<(i16, i16)>) -> SDLAudioCallback {
        SDLAudioCallback {
            input_ringbuffer: audio_buffer_consumer,
            fader_current: 0.0,
            last_frame_written: (0, 0),
            fadestate: AudioFadeState::FadedOut,
        }
    }
}
impl sdl2::audio::AudioCallback for SDLAudioCallback {
    type Channel = i16;

    fn callback(&mut self, out_samples_stereo: &mut [i16]) {
        debug_assert!(out_samples_stereo.len() % 2 == 0);

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
                    frame_out[0] = (self.fader_current * self.last_frame_written.0 as f32) as i16;
                    frame_out[1] = (self.fader_current * self.last_frame_written.1 as f32) as i16;
                } else {
                    self.last_frame_written = frame;
                    frame_out[0] = (self.fader_current * frame.0 as f32) as i16;
                    frame_out[1] = (self.fader_current * frame.1 as f32) as i16;
                }
            } else {
                self.fadestate = AudioFadeState::FadingOut;
                frame_out[0] = (self.fader_current * self.last_frame_written.0 as f32) as i16;
                frame_out[1] = (self.fader_current * self.last_frame_written.1 as f32) as i16;
            }
        }
    }
}
pub struct AudioOutput {
    pub audio_playback_rate_hz: usize,
    samples_queue: ringbuf::Producer<(i16, i16)>,
    _sdl_audio_device: sdl2::audio::AudioDevice<SDLAudioCallback>,
}
impl AudioOutput {
    pub fn new(sdl_context: &sdl2::Sdl) -> AudioOutput {
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

        AudioOutput {
            _sdl_audio_device: audio_device,
            audio_playback_rate_hz,
            samples_queue: audio_ringbuffer_producer,
        }
    }

    fn submit_rendered_chunk(&mut self, chunk: &AudioChunkStereo) {
        for frame in chunk.iter() {
            if let Err(_) = self.samples_queue.push((
                (frame.left * std::i16::MAX as f32) as i16,
                (frame.right * std::i16::MAX as f32) as i16,
            )) {
                log::warn!("Audiobuffer: Could not push frame to queue - queue full?");
            }
        }
    }

    pub fn render_frames(
        &mut self,
        audio: &mut Audiostate,
        window_has_focus: bool,
        minimum_seconds_to_buffer: f32,
    ) {
        let chunkcount_to_render = {
            let minimum_buffer_size =
                (self.audio_playback_rate_hz as f32 * minimum_seconds_to_buffer) as usize;
            let framecount_to_render = {
                let framecount_queued = self.samples_queue.len() / 2;
                dbg!(framecount_queued);
                if framecount_queued < minimum_buffer_size {
                    minimum_buffer_size - framecount_queued
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
