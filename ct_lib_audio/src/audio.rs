use ct_lib_core::transmute_slices;

use super::math::*;

use core::panic;
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

pub const AUDIO_CHUNKSIZE_IN_FRAMES: usize = 512;

#[derive(Clone, Copy)]
struct AudioChunkMono {
    volume: f32,
    samples: [AudioSample; AUDIO_CHUNKSIZE_IN_FRAMES],
}
impl AudioChunkMono {
    pub fn new() -> AudioChunkMono {
        AudioChunkMono {
            volume: 1.0,
            samples: [0.0; AUDIO_CHUNKSIZE_IN_FRAMES],
        }
    }
}
#[derive(Clone, Copy)]
pub struct AudioChunkStereo {
    volume: f32,
    frames: [AudioFrame; AUDIO_CHUNKSIZE_IN_FRAMES],
}
impl AudioChunkStereo {
    pub fn new() -> AudioChunkStereo {
        AudioChunkStereo {
            volume: 1.0,
            frames: [AudioFrame::silence(); AUDIO_CHUNKSIZE_IN_FRAMES],
        }
    }
    pub fn as_interleaved_f32_slice(&self) -> &[f32] {
        unsafe { transmute_slices(&self.frames) }
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

#[derive(Clone)]
struct VolumeAdapter {
    pub current: f32,
    pub target: f32,
}
impl VolumeAdapter {
    fn new(volume: f32) -> VolumeAdapter {
        VolumeAdapter {
            current: volume,
            target: volume,
        }
    }
    fn set_volume(&mut self, volume: f32) {
        self.target = volume;
    }
    fn process(&mut self, input: &AudioChunkMono, output: &mut AudioChunkMono) {
        if input.volume == 0.0 {
            // Fast path - input is silent - no need to ramp up/down volume
            output.volume = 0.0;
            self.current = self.target;
            return;
        }

        if self.target == self.current {
            // Fast path - all values are the same for the chunk
            let volume = self.target;

            if volume == 0.0 {
                // No need to copy silence
                output.volume = 0.0;
            } else {
                *output = *input;
                output.volume *= volume;
            }
        } else {
            // Slow path - need to ramp up/down volume
            let volume_increase = (self.target - self.current) / AUDIO_CHUNKSIZE_IN_FRAMES as f32;
            let mut volume = self.current;
            for (in_sample, out_sample) in input.samples.iter().zip(output.samples.iter_mut()) {
                *out_sample = volume * in_sample;
                volume += volume_increase;
            }
            // We assign here to prevent rounding errors and assuring the fastpath next time
            self.current = self.target;
        }
    }
}

#[derive(Clone)]
struct MonoToStereoAdapter {
    pub current: f32,
    pub target: f32,
}
impl MonoToStereoAdapter {
    fn new(pan: f32) -> MonoToStereoAdapter {
        MonoToStereoAdapter {
            current: pan,
            target: pan,
        }
    }
    fn set_pan(&mut self, pan: f32) {
        self.target = pan;
    }
    fn process(&mut self, input: &AudioChunkMono, output: &mut AudioChunkStereo) {
        if input.volume == 0.0 {
            // Fast path - input is silent - no need to ramp up/down pan
            output.volume = 0.0;
            self.current = self.target;
            return;
        }

        if self.target == self.current {
            // Fast path - all values are the same for the chunk
            let percent = 0.5 * (self.target + 1.0); // Transform [-1,1] -> [0,1]
            let (volume_left, volume_right) = crossfade_squareroot(1.0, percent);
            for (in_sample, out_frame) in input.samples.iter().zip(output.frames.iter_mut()) {
                out_frame.left = volume_left * in_sample;
                out_frame.right = volume_right * in_sample;
            }
        } else {
            // Slow path - need to ramp up/down pan
            let percent_target = 0.5 * (self.target + 1.0); // Transform [-1,1] -> [0,1]
            let percent_start = 0.5 * (self.current + 1.0); // Transform [-1,1] -> [0,1]
            let percent_increase =
                (percent_target - percent_start) / AUDIO_CHUNKSIZE_IN_FRAMES as f32;
            let mut percent = percent_start;
            for (in_sample, out_frame) in input.samples.iter().zip(output.frames.iter_mut()) {
                let (volume_left, volume_right) = crossfade_squareroot(*in_sample, percent);
                *out_frame = AudioFrame::new(volume_left, volume_right);
                percent += percent_increase;
            }
            // We assign here to prevent rounding errors and assuring the fastpath next time
            self.current = self.target;
        }
    }
}

#[derive(Clone)]
struct PlaybackSpeedInterpolator {
    source: AudioSource,
    sample_current: AudioSample,
    sample_next: AudioSample,
    sample_time_percent: f32,

    internal_buffer: AudioChunkMono,
    internal_buffer_readpos: usize,
}
impl PlaybackSpeedInterpolator {
    fn new(source: AudioSource) -> PlaybackSpeedInterpolator {
        PlaybackSpeedInterpolator {
            source,
            sample_current: 0.0,
            sample_next: 0.0,
            sample_time_percent: 0.0,

            internal_buffer: AudioChunkMono::new(),
            internal_buffer_readpos: AUDIO_CHUNKSIZE_IN_FRAMES,
        }
    }

    fn is_looping(&self) -> bool {
        self.source.is_looping()
    }

    fn completion_ratio(&self) -> Option<f32> {
        let TODO = "this is slightly wrong if we still have samples in our internal buffer";
        self.source.completion_ratio()
    }

    fn has_finished(&self) -> bool {
        self.source.has_finished()
            && self.internal_buffer_readpos >= self.internal_buffer.samples.len()
    }

    fn get_next_sample(&mut self) -> AudioSample {
        if self.internal_buffer_readpos >= self.internal_buffer.samples.len() {
            // We have depleted our internal buffer and need to replenish it from our source

            if self.source.has_finished() {
                self.internal_buffer.volume = 0.0;
                return 0.0;
            } else {
                self.source.produce_chunk_mono(&mut self.internal_buffer);
                self.internal_buffer_readpos = 0;
            }
        }

        let sample = unsafe {
            self.internal_buffer
                .samples
                .get_unchecked(self.internal_buffer_readpos)
        };
        self.internal_buffer_readpos += 1;
        *sample
    }

    fn produce_samples(
        &mut self,
        output: &mut [AudioSample],
        output_sample_rate_hz: f32,
        playback_speed_factor: f32,
    ) {
        assert!(playback_speed_factor > EPSILON);

        if self.has_finished() {
            // Our source and internal buffer are empty - we are done here
            return;
        }

        let playback_speed_increment = {
            let sample_rate_conversion_factor =
                self.source.sample_rate_hz() as f32 / output_sample_rate_hz;
            playback_speed_factor * sample_rate_conversion_factor
        };

        for out_sample in output.iter_mut() {
            self.sample_time_percent += playback_speed_increment;

            while self.sample_time_percent >= 1.0 {
                self.sample_current = self.sample_next;
                self.sample_next = self.get_next_sample();
                self.sample_time_percent -= 1.0;
            }

            let interpolated_sample_value = lerp(
                self.sample_current,
                self.sample_next,
                self.sample_time_percent,
            );

            *out_sample = interpolated_sample_value;
        }
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

trait AudioSourceTrait: Clone {
    fn sample_rate_hz(&self) -> usize;
    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
    fn completion_ratio(&self) -> Option<f32>;
    fn produce_chunk_mono(&mut self, output: &mut AudioChunkMono);
    fn produce_chunk_stereo(&mut self, output: &mut AudioChunkStereo);
}

#[derive(Clone)]
struct AudioSourceBufferMono {
    source_buffer: Rc<AudioBufferMono>,
    play_cursor_buffer_index: usize,
    is_looping: bool,
}
impl AudioSourceBufferMono {
    fn new(buffer: Rc<AudioBufferMono>, play_looped: bool) -> AudioSourceBufferMono {
        AudioSourceBufferMono {
            source_buffer: buffer,
            play_cursor_buffer_index: 0,
            is_looping: play_looped,
        }
    }
}
impl AudioSourceTrait for AudioSourceBufferMono {
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

    fn produce_chunk_mono(&mut self, output: &mut AudioChunkMono) {
        if self.has_finished() {
            output.volume = 0.0;
            return;
        }

        output.volume = 1.0;
        for out_sample in &mut output.samples {
            let result = self
                .source_buffer
                .samples
                .get(self.play_cursor_buffer_index)
                .cloned();

            self.play_cursor_buffer_index += 1;
            if self.is_looping {
                if self.play_cursor_buffer_index
                    >= (self.source_buffer.loopsection_start_sampleindex
                        + self.source_buffer.loopsection_samplecount)
                {
                    self.play_cursor_buffer_index =
                        self.source_buffer.loopsection_start_sampleindex;
                }
            }
            *out_sample = result.unwrap_or(0.0);
        }
    }

    fn produce_chunk_stereo(&mut self, _output: &mut AudioChunkStereo) {
        unreachable!()
    }
}
#[derive(Clone)]
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
impl AudioSourceTrait for AudioSourceSine {
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

    fn produce_chunk_mono(&mut self, output: &mut AudioChunkMono) {
        output.volume = 1.0;
        let time_increment = audio_frames_to_seconds(1, self.sample_rate_hz);
        for out_sample in &mut output.samples {
            let sine_amplitude = f64::sin(self.sine_time * 2.0 * PI64);
            self.sine_time += self.sine_frequency * time_increment;
            *out_sample = sine_amplitude as AudioSample;
        }
    }

    fn produce_chunk_stereo(&mut self, _output: &mut AudioChunkStereo) {
        unreachable!()
    }
}

#[derive(Clone)]
enum AudioSource {
    BufferMono(AudioSourceBufferMono),
    Sine(AudioSourceSine),
}
impl AudioSource {
    fn new_from_buffer(buffer: Rc<AudioBufferMono>, play_looped: bool) -> AudioSource {
        AudioSource::BufferMono(AudioSourceBufferMono::new(buffer, play_looped))
    }
    fn new_from_sine(sine_frequency: f64, stream_frames_per_second: usize) -> AudioSource {
        AudioSource::Sine(AudioSourceSine::new(
            sine_frequency,
            stream_frames_per_second,
        ))
    }

    fn sample_rate_hz(&self) -> usize {
        match self {
            AudioSource::BufferMono(buffer) => buffer.sample_rate_hz(),
            AudioSource::Sine(sine) => sine.sample_rate_hz(),
        }
    }
    fn has_finished(&self) -> bool {
        match self {
            AudioSource::BufferMono(buffer) => buffer.has_finished(),
            AudioSource::Sine(sine) => sine.has_finished(),
        }
    }
    fn is_looping(&self) -> bool {
        match self {
            AudioSource::BufferMono(buffer) => buffer.is_looping(),
            AudioSource::Sine(sine) => sine.is_looping(),
        }
    }
    fn completion_ratio(&self) -> Option<f32> {
        match self {
            AudioSource::BufferMono(buffer) => buffer.completion_ratio(),
            AudioSource::Sine(sine) => sine.completion_ratio(),
        }
    }

    fn produce_chunk_mono(&mut self, output: &mut AudioChunkMono) {
        match self {
            AudioSource::BufferMono(buffer) => buffer.produce_chunk_mono(output),
            AudioSource::Sine(sine) => sine.produce_chunk_mono(output),
        }
    }

    fn produce_chunk_stereo(&mut self, output: &mut AudioChunkStereo) {
        match self {
            AudioSource::BufferMono(buffer) => buffer.produce_chunk_stereo(output),
            AudioSource::Sine(sine) => sine.produce_chunk_stereo(output),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostreams Basic

#[derive(Clone, Copy)]
struct AudioRenderParams {
    pub audio_sample_rate_hz: usize,
    pub global_playback_speed: f32,
    pub listener_pos: Vec2,
    pub listener_vel: Vec2,
    pub doppler_effect_medium_velocity_abs_max: f32,
    /// Tells how much units to the left/right an audio source position needs to be away from the
    /// listener_pos to max out the pan to -1.0/1.0
    pub distance_for_max_pan: f32,
}

#[derive(Clone)]
struct AudioStream {
    pub name: String,

    pub playback_speed_interpolator: PlaybackSpeedInterpolator,

    pub frames_left_till_start: usize,
    pub volume_base: f32,
    /// Only used when we don't have spatial params
    pub pan_base: f32,
    /// Must be > 0
    pub playback_speed_base: f32,
    pub has_finished: bool,

    pub volume_adapter: VolumeAdapter,
    pub pan_adapter: MonoToStereoAdapter,

    output_mono: AudioChunkMono,
    output_mono_temp: AudioChunkMono,
    output_stereo: AudioChunkStereo,

    pub spatial_params: Option<SpatialParams>,
}

impl AudioStream {
    fn new(
        name: String,
        source: AudioSource,
        delay_framecount: usize,
        playback_speed_percent: f32,
        volume: f32,
        pan: f32,
        spatial_params: Option<SpatialParams>,
    ) -> AudioStream {
        AudioStream {
            name,

            playback_speed_interpolator: PlaybackSpeedInterpolator::new(source),

            frames_left_till_start: delay_framecount,
            volume_base: volume,
            pan_base: pan,
            playback_speed_base: playback_speed_percent,
            has_finished: false,

            volume_adapter: VolumeAdapter::new(volume),
            pan_adapter: MonoToStereoAdapter::new(0.0),

            output_mono: AudioChunkMono::new(),
            output_mono_temp: AudioChunkMono::new(),
            output_stereo: AudioChunkStereo::new(),

            spatial_params,
        }
    }

    fn produce_output(&mut self, output_params: AudioRenderParams) {
        // Reset volume for output chunks
        self.output_mono.volume = 1.0;
        self.output_mono_temp.volume = 1.0;
        self.output_stereo.volume = 1.0;

        // Fast path - we are finished
        if self.has_finished {
            self.output_mono.volume = 0.0;
            self.output_mono_temp.volume = 0.0;
            self.output_stereo.volume = 0.0;
            return;
        }

        let silence_framecount = {
            let silence_framecount =
                usize::min(self.frames_left_till_start, AUDIO_CHUNKSIZE_IN_FRAMES);
            self.frames_left_till_start =
                if self.frames_left_till_start >= AUDIO_CHUNKSIZE_IN_FRAMES {
                    self.frames_left_till_start - AUDIO_CHUNKSIZE_IN_FRAMES
                } else {
                    0
                };
            silence_framecount
        };

        // Fast path - our stream won't start this chunk
        if silence_framecount == AUDIO_CHUNKSIZE_IN_FRAMES {
            self.output_mono.volume = 0.0;
            self.output_mono_temp.volume = 0.0;
            self.output_stereo.volume = 0.0;
            return;
        }

        // Write remaining delay frames
        for sample in &mut self.output_mono_temp.samples[0..silence_framecount] {
            *sample = 0.0;
        }

        // Calculate spatial settings if necessary
        let (final_volume, final_pan, final_playback_speed_factor) =
            if let Some(spatial_params) = self.spatial_params {
                let (spatial_volume_factor, spatial_pan, spatial_playback_speed_factor) =
                    spatial_params.calculate_volume_pan_playback_speed(&output_params);
                (
                    self.volume_base * spatial_volume_factor,
                    spatial_pan,
                    self.playback_speed_base * spatial_playback_speed_factor,
                )
            } else {
                (self.volume_base, self.pan_base, self.playback_speed_base)
            };
        self.set_volume(final_volume);
        self.pan_adapter.set_pan(final_pan);

        self.playback_speed_interpolator.produce_samples(
            &mut self.output_mono_temp.samples[silence_framecount..],
            output_params.audio_sample_rate_hz as f32,
            final_playback_speed_factor,
        );
        self.has_finished = self.playback_speed_interpolator.has_finished();

        self.volume_adapter
            .process(&self.output_mono_temp, &mut self.output_mono);
        self.pan_adapter
            .process(&self.output_mono, &mut self.output_stereo);
    }

    fn get_output_chunk_stereo(&self) -> &AudioChunkStereo {
        &self.output_stereo
    }

    fn has_started(&self) -> bool {
        self.frames_left_till_start == 0
    }

    fn has_finished(&self) -> bool {
        self.has_finished
    }

    fn is_looping(&self) -> bool {
        self.playback_speed_interpolator.is_looping()
    }

    fn completion_ratio(&self) -> Option<f32> {
        if self.has_started() {
            self.playback_speed_interpolator.completion_ratio()
        } else {
            None
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume_base = volume;
    }

    fn set_pan(&mut self, pan: f32) {
        self.pan_base = pan;
    }

    fn set_playback_speed(&mut self, playback_speed_percent: f32) {
        self.playback_speed_base = playback_speed_percent;
    }

    fn set_spatial_pos(&mut self, pos: Vec2) {
        if let Some(spatial) = &mut self.spatial_params {
            spatial.pos = pos;
        } else {
            panic!(
                "Stream '{}' has no spatial component to set position",
                self.name
            );
        }
    }

    fn set_spatial_vel(&mut self, vel: Vec2) {
        if let Some(spatial) = &mut self.spatial_params {
            spatial.vel = vel;
        } else {
            panic!(
                "Stream '{}' has no spatial component to set velocity",
                self.name
            );
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostream Spatial

#[derive(Copy, Clone)]
struct SpatialParams {
    pub pos: Vec2,
    pub vel: Vec2,
    pub doppler_effect_strength: f32,
    /// The higher the exponent, the faster the falloff
    /// ...
    /// 0.5 - squareroot
    /// 1.0 - linear
    /// 2.0 - quadratic
    /// 3.0 - cubic
    /// ...
    pub falloff_type: AudioFalloffType,
    pub falloff_distance_start: f32,
    pub falloff_distance_end: f32,
}

impl SpatialParams {
    fn new(
        pos: Vec2,
        vel: Vec2,
        doppler_effect_strength: f32,
        falloff_type: AudioFalloffType,
        falloff_distance_start: f32,
        falloff_distance_end: f32,
    ) -> SpatialParams {
        SpatialParams {
            pos,
            vel,
            doppler_effect_strength,
            falloff_type,
            falloff_distance_start,
            falloff_distance_end,
        }
    }

    fn calculate_volume_pan_playback_speed(
        &self,
        output_params: &AudioRenderParams,
    ) -> (f32, f32, f32) {
        let volume_factor = spatial_volume_factor(
            self.pos,
            output_params.listener_pos,
            self.falloff_type,
            self.falloff_distance_start,
            self.falloff_distance_end,
        );
        let pan = spatial_pan(
            self.pos,
            output_params.listener_pos,
            output_params.distance_for_max_pan,
        );
        let playback_speed_factor = spatial_playback_speed_factor(
            self.pos,
            self.vel,
            output_params.listener_pos,
            output_params.listener_vel,
            self.doppler_effect_strength,
            output_params.doppler_effect_medium_velocity_abs_max,
        );
        (volume_factor, pan, playback_speed_factor)
    }
}

#[derive(Copy, Clone)]
pub enum AudioFalloffType {
    None,
    /// For large non-focused sounds
    Linear,
    /// For focused sounds
    Natural,
    /// Like `Natural` but can still be heard outside the falloff distance
    NaturalUnbounded {
        minimum_volume: f32,
    },
}
impl AudioFalloffType {
    fn value_for_distance(
        &self,
        distance: f32,
        falloff_distance_start: f32,
        falloff_distance_end: f32,
    ) -> f32 {
        if let AudioFalloffType::None = self {
            return 1.0;
        }

        let minimum_volume = if let AudioFalloffType::NaturalUnbounded { minimum_volume } = self {
            *minimum_volume
        } else {
            0.0
        };

        if distance < falloff_distance_start {
            1.0
        } else if distance > falloff_distance_end {
            minimum_volume
        } else {
            let distance_ratio = (distance - falloff_distance_start)
                / (falloff_distance_end - falloff_distance_start);
            match self {
                AudioFalloffType::Linear => distance_ratio,
                AudioFalloffType::Natural => f32::exp(-6.0 * distance_ratio),
                AudioFalloffType::NaturalUnbounded { minimum_volume } => {
                    minimum_volume + (1.0 - minimum_volume) * f32::exp(-6.0 * distance_ratio)
                }
                _ => unreachable!(),
            }
        }
    }
}

fn spatial_pan(source_pos: Vec2, listener_pos: Vec2, distance_for_max_pan: f32) -> f32 {
    clampf(
        (source_pos.x - listener_pos.x) / distance_for_max_pan,
        -1.0,
        1.0,
    )
}

fn spatial_playback_speed_factor(
    source_pos: Vec2,
    source_vel: Vec2,
    listener_pos: Vec2,
    listener_vel: Vec2,
    doppler_effect_strength: f32,
    doppler_effect_medium_velocity_abs_max: f32,
) -> f32 {
    // This uses the stationary observer doppler effect forumla
    // https://en.wikipedia.org/wiki/Doppler_effect#Consequences
    let dir_to_source = {
        let listener_to_source = source_pos - listener_pos;
        let listener_to_source_distance = listener_to_source.magnitude();
        if is_effectively_zero(listener_to_source_distance) {
            Vec2::unit_x()
        } else {
            listener_to_source / listener_to_source_distance
        }
    };
    let vel_relative = source_vel - listener_vel;
    let vel_relative_source = Vec2::dot(vel_relative, dir_to_source);
    let vel_relative_source_ratio =
        doppler_effect_strength * vel_relative_source / doppler_effect_medium_velocity_abs_max;

    1.0 / (1.0 + clampf(vel_relative_source_ratio, -0.5, 0.5))
}

fn spatial_volume_factor(
    source_pos: Vec2,
    listener_pos: Vec2,
    falloff_type: AudioFalloffType,
    falloff_distance_start: f32,
    falloff_distance_end: f32,
) -> f32 {
    let distance = Vec2::distance(source_pos, listener_pos);
    falloff_type.value_for_distance(distance, falloff_distance_start, falloff_distance_end)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostate

/// This can never be zero for a valid stream
pub type AudioStreamId = u64;

#[derive(Clone)]
pub struct Audiostate {
    next_frame_index_to_output: AudioFrameIndex,
    output_render_params: AudioRenderParams,

    /// This can never be zero when used with `get_next_stream_id` method
    next_stream_id: AudioStreamId,

    streams: HashMap<AudioStreamId, AudioStream>,
    streams_to_delete_after_finish: HashSet<AudioStreamId>,

    audio_recordings_mono: HashMap<String, Rc<AudioBufferMono>>,
    audio_recordings_stereo: HashMap<String, Rc<AudioBufferStereo>>,
}

impl Audiostate {
    pub fn new(
        audio_sample_rate_hz: usize,
        distance_for_max_pan: f32,
        doppler_effect_medium_velocity_abs_max: f32,
    ) -> Audiostate {
        Audiostate {
            next_frame_index_to_output: 0,

            output_render_params: AudioRenderParams {
                audio_sample_rate_hz,
                global_playback_speed: 1.0,
                listener_pos: Vec2::zero(),
                listener_vel: Vec2::zero(),
                distance_for_max_pan,
                doppler_effect_medium_velocity_abs_max,
            },

            next_stream_id: 0,
            streams: HashMap::new(),
            streams_to_delete_after_finish: HashSet::new(),

            audio_recordings_mono: HashMap::new(),
            audio_recordings_stereo: HashMap::new(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.next_frame_index_to_output = 0;

        self.output_render_params.global_playback_speed = 1.0;
        self.output_render_params.listener_pos = Vec2::zero();
        self.output_render_params.listener_vel = Vec2::zero();

        self.next_stream_id = 0;
        self.streams = HashMap::new();
        self.streams_to_delete_after_finish = HashSet::new();
    }

    #[inline]
    pub fn add_audio_recordings_mono(
        &mut self,
        mut audio_recordings_mono: HashMap<String, AudioBufferMono>,
    ) {
        for (name, audiobuffer) in audio_recordings_mono.drain() {
            self.audio_recordings_mono
                .insert(name, Rc::new(audiobuffer));
        }
    }
    #[inline]
    pub fn add_audio_recordings_stereo(
        &mut self,
        mut audio_recordings_stereo: HashMap<String, AudioBufferStereo>,
    ) {
        for (name, audiobuffer) in audio_recordings_stereo.drain() {
            self.audio_recordings_stereo
                .insert(name, Rc::new(audiobuffer));
        }
    }

    #[inline]
    pub fn current_time_seconds(&self) -> f64 {
        audio_frames_to_seconds(
            self.next_frame_index_to_output,
            self.output_render_params.audio_sample_rate_hz,
        )
    }

    fn get_stream(&self, stream_id: AudioStreamId) -> &AudioStream {
        self.streams
            .get(&stream_id)
            .unwrap_or_else(|| panic!("No audio stream found for id {}", stream_id))
    }
    fn get_stream_mut(&mut self, stream_id: AudioStreamId) -> &mut AudioStream {
        self.streams
            .get_mut(&stream_id)
            .unwrap_or_else(|| panic!("No audio stream found for id {}", stream_id))
    }

    #[inline]
    pub fn stream_has_finished(&self, stream_id: AudioStreamId) -> bool {
        self.get_stream(stream_id).has_finished()
    }

    #[inline]
    pub fn stream_forget(&mut self, stream_id: AudioStreamId) {
        let stream = self.get_stream(stream_id);
        assert!(
            !stream.is_looping(),
            "Cannot forget looping audio stream {}",
            stream_id
        );
        self.streams_to_delete_after_finish.insert(stream_id);
    }

    #[inline]
    pub fn stream_completion_ratio(&self, stream_id: AudioStreamId) -> Option<f32> {
        let stream = self.get_stream(stream_id);
        stream.completion_ratio()
    }

    #[inline]
    fn get_next_stream_id(&mut self) -> AudioStreamId {
        self.next_stream_id += 1;
        self.next_stream_id
    }

    #[inline]
    pub fn set_global_playback_speed_factor(&mut self, global_playback_speed: f32) {
        self.output_render_params.global_playback_speed = global_playback_speed;
    }

    /// It is assumed that `out_chunk` is filled with silence
    #[inline]
    pub fn render_audio_chunk(&mut self, out_chunk: &mut AudioChunkStereo) {
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
        for stream in self.streams.values_mut() {
            stream.produce_output(self.output_render_params);
            if stream.get_output_chunk_stereo().volume == 0.0 {
                continue;
            }

            let chunk_volume = out_chunk.volume;
            for (out_frame, rendered_chunk) in out_chunk
                .frames
                .iter_mut()
                .zip(stream.get_output_chunk_stereo().frames.iter())
            {
                out_frame.left += chunk_volume * rendered_chunk.left;
                out_frame.right += chunk_volume * rendered_chunk.right;
            }
        }
        self.next_frame_index_to_output += AUDIO_CHUNKSIZE_IN_FRAMES as AudioFrameIndex;
    }

    #[inline]
    pub fn set_listener_pos(&mut self, listener_pos: Vec2) {
        self.output_render_params.listener_pos = listener_pos;
    }

    #[inline]
    pub fn set_listener_vel(&mut self, listener_vel: Vec2) {
        self.output_render_params.listener_vel = listener_vel;
    }

    #[inline]
    pub fn stream_set_spatial_pos(&mut self, stream_id: AudioStreamId, pos: Vec2) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_spatial_pos(pos);
    }

    #[inline]
    pub fn stream_set_spatial_vel(&mut self, stream_id: AudioStreamId, vel: Vec2) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_spatial_vel(vel);
    }

    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, volume: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_volume(volume);
    }

    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, playback_speed: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_playback_speed(playback_speed);
    }

    fn start_time_to_delay_framecount(&self, schedule_time_seconds: f64) -> usize {
        let start_frame_index = audio_seconds_to_frames(
            schedule_time_seconds,
            self.output_render_params.audio_sample_rate_hz,
        )
        .round() as AudioFrameIndex;

        (start_frame_index - self.next_frame_index_to_output).max(0) as usize
    }

    #[must_use]
    #[inline]
    pub fn play(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        play_looped: bool,
        volume: f32,
        playback_speed: f32,
        pan: f32,
    ) -> AudioStreamId {
        let id = self.get_next_stream_id();
        let start_delay_framecount = self.start_time_to_delay_framecount(schedule_time_seconds);
        let source = if recording_name == "sine" {
            AudioSource::new_from_sine(440.0, self.output_render_params.audio_sample_rate_hz)
        } else {
            let buffer = self
                .audio_recordings_mono
                .get(recording_name)
                .unwrap_or_else(|| panic!("Recording '{}' not found", recording_name));
            AudioSource::new_from_buffer(buffer.clone(), play_looped)
        };
        let stream = AudioStream::new(
            format!("{}:{}", recording_name, id),
            source,
            start_delay_framecount,
            playback_speed,
            volume,
            pan,
            None,
        );
        self.streams.insert(id, stream);
        id
    }
    #[inline]
    pub fn play_oneshot(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        volume: f32,
        playback_speed: f32,
        pan: f32,
    ) {
        let id = self.play(
            recording_name,
            schedule_time_seconds,
            false,
            volume,
            playback_speed,
            pan,
        );
        self.stream_forget(id);
    }

    #[must_use]
    #[inline]
    pub fn play_spatial(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        play_looped: bool,
        volume: f32,
        playback_speed: f32,
        pos: Vec2,
        vel: Vec2,
        doppler_effect_strength: f32,
        falloff_type: AudioFalloffType,
        falloff_distance_start: f32,
        falloff_distance_end: f32,
    ) -> AudioStreamId {
        let id = self.get_next_stream_id();
        let start_delay_framecount = self.start_time_to_delay_framecount(schedule_time_seconds);
        let stream = {
            let initial_pan = spatial_pan(
                pos,
                self.output_render_params.listener_pos,
                self.output_render_params.distance_for_max_pan,
            );
            let source = {
                let buffer = self
                    .audio_recordings_mono
                    .get(recording_name)
                    .unwrap_or_else(|| panic!("Recording '{}' not found", recording_name));
                AudioSource::new_from_buffer(buffer.clone(), play_looped)
            };
            AudioStream::new(
                format!("{}:{}", recording_name, id),
                source,
                start_delay_framecount,
                playback_speed,
                volume,
                initial_pan,
                Some(SpatialParams::new(
                    pos,
                    vel,
                    doppler_effect_strength,
                    falloff_type,
                    falloff_distance_start,
                    falloff_distance_end,
                )),
            )
        };
        self.streams.insert(id, stream);
        id
    }

    #[must_use]
    #[inline]
    pub fn play_spatial_oneshot(
        &mut self,
        recording_name: &str,
        schedule_time_seconds: f64,
        volume: f32,
        playback_speed: f32,
        pos: Vec2,
        vel: Vec2,
        doppler_effect_strength: f32,
        falloff_type: AudioFalloffType,
        falloff_distance_start: f32,
        falloff_distance_end: f32,
    ) {
        let id = self.play_spatial(
            recording_name,
            schedule_time_seconds,
            false,
            volume,
            playback_speed,
            pos,
            vel,
            doppler_effect_strength,
            falloff_type,
            falloff_distance_start,
            falloff_distance_end,
        );
        self.stream_forget(id);
    }
}
