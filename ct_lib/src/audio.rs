use super::math::*;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Samples and Frames

pub type AudioSample = f32;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct AudioFrame {
    pub left: AudioSample,
    pub right: AudioSample,
}

impl AudioFrame {
    #[inline(always)]
    pub fn new(left: f32, right: f32) -> AudioFrame {
        AudioFrame { left, right }
    }

    #[inline(always)]
    pub fn silence() -> AudioFrame {
        AudioFrame {
            left: 0.0,
            right: 0.0,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Timing

pub type AudioFrameIndex = i64;
pub type AudioSampleIndex = i64;

#[inline]
pub fn audio_frames_to_seconds(framecount: AudioFrameIndex, audio_samplerate_hz: usize) -> f64 {
    framecount as f64 / audio_samplerate_hz as f64
}

#[inline]
/// NOTE: This returns a float so we can round it down ourselves or use the value for further
///       calculations without forced rounding errors
pub fn audio_seconds_to_frames(time: f64, audio_samplerate_hz: usize) -> f64 {
    time * audio_samplerate_hz as f64
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Adapters

/// NOTE: This assumes (and works best with) values in the range [-1.0, 1.0]
#[derive(Debug, Clone)]
struct AudioIterativeFader {
    pub current: f32,
    pub target: f32,
}
impl AudioIterativeFader {
    fn new(initial_value: f32) -> AudioIterativeFader {
        AudioIterativeFader {
            current: initial_value,
            target: initial_value,
        }
    }

    fn next_value(&mut self) -> f32 {
        // NOTE: The value for `step_size` was chosen experimentally based on what sounded ok
        let distance = (self.target - self.current).abs();
        let step_size = f32::max(1.0 / i16::MAX as f32, distance / 1024.0);

        self.current = if self.target >= self.current {
            f32::min(self.current + step_size, self.target)
        } else {
            f32::max(self.current - step_size, self.target)
        };
        self.current
    }
}
impl Iterator for AudioIterativeFader {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_value())
    }
}

#[derive(Debug, Clone)]
struct VolumeAdapter {
    pub fader: AudioIterativeFader,
}
impl VolumeAdapter {
    fn new(volume: f32) -> VolumeAdapter {
        VolumeAdapter {
            fader: AudioIterativeFader::new(volume),
        }
    }
    fn set_volume(&mut self, volume: f32) {
        self.fader.target = volume;
    }
    fn process(&mut self, input_samples: &[AudioSample], output_samples: &mut [AudioSample]) {
        for (in_sample, out_sample) in input_samples.iter().zip(output_samples.iter_mut()) {
            let volume = self.fader.next_value();
            *out_sample = volume * in_sample;
        }
    }
}

#[derive(Clone)]
struct MonoToStereoAdapter {
    fader: AudioIterativeFader,
}
impl MonoToStereoAdapter {
    fn new(pan: f32) -> MonoToStereoAdapter {
        MonoToStereoAdapter {
            fader: AudioIterativeFader::new(pan),
        }
    }
    fn set_pan(&mut self, pan: f32) {
        self.fader.target = pan;
    }
    fn process(&mut self, input_samples: &[AudioSample], output_frames: &mut [AudioFrame]) {
        for (in_sample, out_frame) in input_samples.iter().zip(output_frames.iter_mut()) {
            let pan = 0.5 * (self.fader.next_value() + 1.0); // Transform [-1,1] -> [0,1]
            let (volume_left, volume_right) = crossfade_sinuoidal(*in_sample, pan);
            *out_frame = AudioFrame::new(volume_left, volume_right);
        }
    }
}

#[derive(Clone)]
struct PlaybackSpeedInterpolatorLinear {
    sample_current: Option<AudioSample>,
    sample_next: Option<AudioSample>,
    sample_time_percent: f32,
}
impl PlaybackSpeedInterpolatorLinear {
    fn new() -> PlaybackSpeedInterpolatorLinear {
        PlaybackSpeedInterpolatorLinear {
            sample_current: Some(0.0),
            sample_next: Some(0.0),
            sample_time_percent: 0.0,
        }
    }

    fn next_sample(
        &mut self,
        source_samples: &mut dyn Iterator<Item = AudioSample>,
        speed: f32,
    ) -> Option<AudioSample> {
        if self.sample_current.is_none() && self.sample_next.is_none() {
            return None;
        }

        assert!(speed > EPSILON);
        self.sample_time_percent += speed;

        while self.sample_time_percent >= 1.0 {
            self.sample_current = self.sample_next;
            self.sample_next = source_samples.next();
            self.sample_time_percent -= 1.0;
        }

        let interpolated_sample_value = lerp(
            self.sample_current.unwrap_or(0.0),
            self.sample_next.unwrap_or(0.0),
            self.sample_time_percent,
        );

        Some(interpolated_sample_value)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiobuffers and sources

#[derive(Clone)]
pub struct AudioBuffer<FrameType> {
    pub name: String,
    pub sample_rate_hz: usize,
    pub samples: Vec<FrameType>,

    /// Defaults to 0
    pub loopsection_start_sampleindex: usize,
    /// Defaults to samples.len()
    pub loopsection_samplecount: usize,
}
pub type AudioBufferMono = AudioBuffer<AudioSample>;
pub type AudioBufferStereo = AudioBuffer<AudioFrame>;

trait AudioSourceMono: Iterator<Item = AudioSample> {
    fn sample_rate_hz(&self) -> usize;
    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
    fn completion_ratio(&self) -> Option<f32>;
}

struct AudioBufferSourceMono {
    pub source_buffer: Rc<AudioBufferMono>,
    pub play_cursor_buffer_index: usize,
    pub is_looping: bool,
}
impl AudioBufferSourceMono {
    fn new(buffer: Rc<AudioBufferMono>, play_looped: bool) -> AudioBufferSourceMono {
        AudioBufferSourceMono {
            source_buffer: buffer,
            play_cursor_buffer_index: 0,
            is_looping: play_looped,
        }
    }
}
impl AudioSourceMono for AudioBufferSourceMono {
    fn sample_rate_hz(&self) -> usize {
        self.source_buffer.sample_rate_hz
    }
    fn has_finished(&self) -> bool {
        self.play_cursor_buffer_index >= self.source_buffer.samples.len()
    }
    fn is_looping(&self) -> bool {
        self.is_looping
    }
    fn completion_ratio(&self) -> Option<f32> {
        Some(self.play_cursor_buffer_index as f32 / self.source_buffer.samples.len() as f32)
    }
}
impl Iterator for AudioBufferSourceMono {
    type Item = AudioSample;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .source_buffer
            .samples
            .get(self.play_cursor_buffer_index)
            .cloned();

        self.play_cursor_buffer_index = usize::min(
            self.play_cursor_buffer_index + 1,
            self.source_buffer.samples.len(),
        );
        if self.is_looping {
            if self.play_cursor_buffer_index
                >= (self.source_buffer.loopsection_start_sampleindex
                    + self.source_buffer.loopsection_samplecount)
            {
                self.play_cursor_buffer_index = self.source_buffer.loopsection_start_sampleindex;
            }
        }
        result
    }
}

struct AudioSourceSine {
    sine_time: f64,
    sine_frequency: f64,
    sample_rate_hz: usize,
}
impl AudioSourceSine {
    fn new(sine_frequency: f64, stream_frames_per_second: usize) -> AudioSourceSine {
        AudioSourceSine {
            sine_frequency,
            sample_rate_hz: stream_frames_per_second,

            sine_time: 0.0,
        }
    }
}
impl AudioSourceMono for AudioSourceSine {
    fn sample_rate_hz(&self) -> usize {
        self.sample_rate_hz
    }
    fn has_finished(&self) -> bool {
        false
    }
    fn is_looping(&self) -> bool {
        true
    }
    fn completion_ratio(&self) -> Option<f32> {
        None
    }
}
impl Iterator for AudioSourceSine {
    type Item = AudioSample;
    fn next(&mut self) -> Option<Self::Item> {
        let sine_amplitude = f64::sin(self.sine_time * 2.0 * PI64);
        let time_increment = audio_frames_to_seconds(1, self.sample_rate_hz);
        self.sine_time += self.sine_frequency * time_increment;

        Some(sine_amplitude as AudioSample)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostreams

trait AudioStream {
    fn process_mono(&mut self, output_samples: &mut [AudioSample], output_sample_rate_hz: usize);
    fn process_stereo(&mut self, output_frames: &mut [AudioFrame], output_sample_rate_hz: usize);

    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
    fn completion_ratio(&self) -> Option<f32>;

    fn set_volume(&mut self, volume: f32);
    fn set_playback_speed(&mut self, playback_speed: f32);
}

struct AudioStreamScheduledMono {
    pub source: Box<dyn AudioSourceMono>,
    pub interpolator: PlaybackSpeedInterpolatorLinear,

    pub frames_left_till_start: usize,
    /// Must be > 0
    pub playback_speed: f32,
    pub has_finished: bool,
}
impl AudioStreamScheduledMono {
    fn new(
        source_stream: Box<dyn AudioSourceMono>,
        delay_framecount: usize,
        playback_speed: f32,
    ) -> AudioStreamScheduledMono {
        AudioStreamScheduledMono {
            source: source_stream,
            interpolator: PlaybackSpeedInterpolatorLinear::new(),
            frames_left_till_start: delay_framecount,
            playback_speed,
            has_finished: false,
        }
    }
}
impl AudioStream for AudioStreamScheduledMono {
    fn process_stereo(
        &mut self,
        _output_samples: &mut [AudioFrame],
        _output_sample_rate_hz: usize,
    ) {
        unimplemented!()
    }

    fn process_mono(&mut self, output_samples: &mut [AudioSample], output_sample_rate_hz: usize) {
        // Fill up with silence if our stream has not started yet
        let output_samples = if self.frames_left_till_start != 0 {
            let silence_framecount = self.frames_left_till_start.min(output_samples.len());
            if self.frames_left_till_start >= output_samples.len() {
                self.frames_left_till_start -= output_samples.len();
            } else {
                self.frames_left_till_start = 0;
            }
            for sample in &mut output_samples[0..silence_framecount] {
                *sample = 0.0;
            }
            if silence_framecount == output_samples.len() {
                return;
            }
            &mut output_samples[silence_framecount..]
        } else {
            output_samples
        };

        let source_sample_rate_hz = self.source.sample_rate_hz() as f32;
        let sample_rate_conversion_factor = source_sample_rate_hz / output_sample_rate_hz as f32;
        let playback_speed = self.playback_speed * sample_rate_conversion_factor;

        for out_sample in output_samples {
            if let Some(resampled_value) = self
                .interpolator
                .next_sample(&mut self.source, playback_speed)
            {
                *out_sample = resampled_value;
            } else {
                self.has_finished = true;
                *out_sample = 0.0;
            }
        }
    }

    fn has_finished(&self) -> bool {
        self.has_finished
    }

    fn is_looping(&self) -> bool {
        self.source.is_looping()
    }

    fn completion_ratio(&self) -> Option<f32> {
        if self.frames_left_till_start > 0 {
            None
        } else {
            self.source.completion_ratio()
        }
    }

    fn set_volume(&mut self, _volume: f32) {
        unimplemented!()
    }

    fn set_playback_speed(&mut self, playback_speed_factor: f32) {
        self.playback_speed = playback_speed_factor;
    }
}

struct AudioStreamStereo {
    pub stream: AudioStreamScheduledMono,
    pub volume: VolumeAdapter,
    pub pan: MonoToStereoAdapter,
}
impl AudioStreamStereo {
    fn new(
        source: Box<dyn AudioSourceMono>,
        delay_framecount: usize,
        playback_speed: f32,
        volume: f32,
        pan: f32,
    ) -> AudioStreamStereo {
        let stream = AudioStreamScheduledMono::new(source, delay_framecount, playback_speed);
        AudioStreamStereo {
            stream,
            volume: VolumeAdapter::new(volume),
            pan: MonoToStereoAdapter::new(pan),
        }
    }
}
impl AudioStream for AudioStreamStereo {
    fn process_mono(&mut self, _output_samples: &mut [AudioSample], _output_sample_rate_hz: usize) {
        unimplemented!()
    }
    fn process_stereo(&mut self, output_frames: &mut [AudioFrame], output_sample_rate_hz: usize) {
        // TODO: For this we need the Chunks
        let mut input_buffer_volume = vec![0.0; output_frames.len()];
        self.stream
            .process_mono(&mut input_buffer_volume, output_sample_rate_hz);
        let mut input_buffer_pan = vec![0.0; output_frames.len()];
        self.volume
            .process(&input_buffer_volume, &mut input_buffer_pan);
        self.pan.process(&input_buffer_pan, output_frames);
    }

    fn has_finished(&self) -> bool {
        self.stream.has_finished()
    }
    fn is_looping(&self) -> bool {
        self.stream.is_looping()
    }

    fn completion_ratio(&self) -> Option<f32> {
        self.stream.completion_ratio()
    }
    fn set_volume(&mut self, volume: f32) {
        self.volume.set_volume(volume);
    }
    fn set_playback_speed(&mut self, playback_speed: f32) {
        self.stream.set_playback_speed(playback_speed);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostate

/// This can never be zero for a valid stream
pub type AudioStreamId = u64;

pub struct Audiostate {
    next_frame_index_to_output: AudioFrameIndex,
    audio_playback_rate_hz: usize,

    global_playback_speed: f32,

    listener_pos: Vec2,
    listener_vel: Vec2,

    /// This can never be zero when used with `get_next_stream_id` method
    next_stream_id: AudioStreamId,

    streams: HashMap<AudioStreamId, Box<dyn AudioStream>>,
    streams_to_delete_after_finish: HashSet<AudioStreamId>,

    audio_recordings_mono: HashMap<String, Rc<AudioBufferMono>>,
    audio_recordings_stereo: HashMap<String, Rc<AudioBufferStereo>>,
}
impl Clone for Audiostate {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Audiostate {
    pub fn new(audio_playback_rate_hz: usize) -> Audiostate {
        Audiostate {
            next_frame_index_to_output: 0,
            audio_playback_rate_hz,

            global_playback_speed: 1.0,
            listener_pos: Vec2::zero(),

            listener_vel: Vec2::zero(),
            next_stream_id: 0,
            streams: HashMap::new(),
            streams_to_delete_after_finish: HashSet::new(),

            audio_recordings_mono: HashMap::new(),
            audio_recordings_stereo: HashMap::new(),
        }
    }

    pub fn add_audio_recordings_mono(
        &mut self,
        mut audio_recordings_mono: HashMap<String, AudioBufferMono>,
    ) {
        for (name, audiobuffer) in audio_recordings_mono.drain() {
            self.audio_recordings_mono
                .insert(name, Rc::new(audiobuffer));
        }
    }
    pub fn add_audio_recordings_stereo(
        &mut self,
        mut audio_recordings_stereo: HashMap<String, AudioBufferStereo>,
    ) {
        for (name, audiobuffer) in audio_recordings_stereo.drain() {
            self.audio_recordings_stereo
                .insert(name, Rc::new(audiobuffer));
        }
    }

    pub fn current_time_seconds(&self) -> f64 {
        audio_frames_to_seconds(self.next_frame_index_to_output, self.audio_playback_rate_hz)
    }

    fn get_stream(&self, stream_id: AudioStreamId) -> &Box<dyn AudioStream> {
        self.streams
            .get(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id))
    }
    fn get_stream_mut(&mut self, stream_id: AudioStreamId) -> &mut Box<dyn AudioStream> {
        self.streams
            .get_mut(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id))
    }

    pub fn stream_has_finished(&self, stream_id: AudioStreamId) -> bool {
        self.get_stream(stream_id).has_finished()
    }

    pub fn stream_forget(&mut self, stream_id: AudioStreamId) {
        let stream = self.get_stream(stream_id);
        assert!(
            !stream.is_looping(),
            "Cannot forget looping audio stream {}",
            stream_id
        );
        self.streams_to_delete_after_finish.insert(stream_id);
    }

    pub fn stream_completion_ratio(&self, stream_id: AudioStreamId) -> Option<f32> {
        let stream = self.get_stream(stream_id);
        stream.completion_ratio()
    }

    fn get_next_stream_id(&mut self) -> AudioStreamId {
        self.next_stream_id += 1;
        self.next_stream_id
    }

    pub fn set_global_playback_speed(&mut self, global_playback_speed: f32) {
        self.global_playback_speed = global_playback_speed;
    }

    pub fn render_audio(&mut self, out_frames: &mut [AudioFrame]) {
        // Remove streams that have finished
        let mut streams_removed = vec![];
        for &stream_id in &self.streams_to_delete_after_finish {
            if self.stream_has_finished(stream_id) {
                self.streams.remove(&stream_id);
                streams_removed.push(stream_id);
            }
        }
        for stream_id in streams_removed {
            self.streams_to_delete_after_finish.remove(&stream_id);
        }

        // Render samples
        for out_frame in out_frames.iter_mut() {
            *out_frame = AudioFrame::silence();
        }
        let mut temp_out = vec![AudioFrame::silence(); out_frames.len()];
        for stream in self.streams.values_mut() {
            stream.process_stereo(&mut temp_out, self.audio_playback_rate_hz);
            for (out_frame, temp_frame) in out_frames.iter_mut().zip(temp_out.iter()) {
                out_frame.left += temp_frame.left;
                out_frame.right += temp_frame.right;
            }
        }
        self.next_frame_index_to_output += out_frames.len() as AudioFrameIndex;
    }

    pub fn set_listener_pos(&mut self, new_listener_pos: Vec2) {
        // TODO
    }

    pub fn stereo_stream_set_pan(&mut self, stream_id: AudioStreamId, new_pan: f32) {

        // TODO
    }

    pub fn spatial_stream_set_pos(&mut self, stream_id: AudioStreamId, new_pos: Vec2) {
        // TODO
    }

    pub fn spatial_stream_set_vel(&mut self, stream_id: AudioStreamId, new_vel: Vec2) {
        // TODO
    }

    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, volume: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_volume(volume);
    }

    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, playback_speed: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_playback_speed(playback_speed);
    }

    fn start_delay_framecount_for_time(&self, schedule_time_seconds: f64) -> usize {
        let start_frame_index =
            audio_seconds_to_frames(schedule_time_seconds, self.audio_playback_rate_hz).round()
                as AudioFrameIndex;

        (start_frame_index - self.next_frame_index_to_output).max(0) as usize
    }

    #[must_use]
    pub fn play(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        play_looped: bool,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) -> AudioStreamId {
        let id = self.get_next_stream_id();
        let start_delay_framecount = self.start_delay_framecount_for_time(schedule_time_seconds);
        let stream = if recording_name == "sine" {
            Box::new(AudioStreamStereo::new(
                Box::new(AudioSourceSine::new(440.0, 44100)),
                start_delay_framecount,
                playback_speed,
                volume,
                pan,
            ))
        } else {
            let buffer = self
                .audio_recordings_mono
                .get(recording_name)
                .expect(&format!("Recording '{}' not found", recording_name));
            Box::new(AudioStreamStereo::new(
                Box::new(AudioBufferSourceMono::new(buffer.clone(), play_looped)),
                start_delay_framecount,
                playback_speed,
                volume,
                pan,
            ))
        };
        self.streams.insert(id, stream);
        id
    }
    pub fn play_oneshot(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) {
        let id = self.play(
            recording_name,
            schedule_time_seconds,
            false,
            volume,
            pan,
            playback_speed,
        );
        self.stream_forget(id);
    }
}
