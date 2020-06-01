use super::math::*;

use std::collections::{HashMap, HashSet, VecDeque};

pub type AudioSample = f32;

const AUDIO_CHUNKLENGTH_IN_SAMPLES: usize = 256;

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
// Audio output

pub type AudioFrameIndex = i64;
pub type AudioChunkIndex = i64;
pub type AudioSampleIndex = i64;

#[inline]
pub fn audio_frames_to_seconds(framecount: AudioFrameIndex, audio_frames_per_second: usize) -> f64 {
    framecount as f64 / audio_frames_per_second as f64
}

#[inline]
/// NOTE: This returns a float so we can round it down ourselves or use the value for further
///       calculations without forced rounding errors
pub fn audio_seconds_to_frames(time: f64, audio_frames_per_second: usize) -> f64 {
    time * audio_frames_per_second as f64
}

#[inline]
pub fn audio_beat_length_in_seconds(beats_per_minute: usize) -> f64 {
    60.0 / (beats_per_minute as f64)
}

#[inline]
pub fn audio_measure_length_in_seconds(beats_per_measure: usize, beats_per_minute: usize) -> f64 {
    beats_per_measure as f64 * audio_beat_length_in_seconds(beats_per_minute)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Streams

/// This can never be zero for a valid stream
pub type AudioStreamId = u64;

#[derive(Debug, Clone, Copy)]
pub enum SchedulePlay {
    Immediately,
    OnNextMeasure {
        beats_per_minute: usize,
        beats_per_measure: usize,
    },
    OnNextBeat {
        beats_per_minute: usize,
    },
    OnNextHalfBeat {
        beats_per_minute: usize,
    },
    OnNextQuarterBeat {
        beats_per_minute: usize,
    },
}

#[derive(Debug, Clone)]
struct AudioStreamOld {
    recording_name: String,

    play_time: SchedulePlay,
    start_frame: Option<AudioFrameIndex>,

    remove_on_finish: bool,
    has_finished: bool,
    is_repeating: bool,

    playback_speed_current: f32,
    playback_speed_target: f32,

    /// Ranges in [0,1]
    /// Silence       = 0
    /// Full loudness = 1
    volume_current: f32,
    volume_target: f32,

    /// Ranges in [-1,1]
    /// Left   = -1
    /// Center =  0
    /// Right  =  1
    pan_current: f32,
    pan_target: f32,
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
// Audiostate

trait AudioSourceMono: Iterator<Item = AudioSample> {
    fn sample_rate_hz(&self) -> usize;
    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
}

#[derive(Clone)]
struct AudioBufferMono {
    pub name: String,
    pub sample_rate_hz: usize,
    pub samples: Vec<AudioSample>,
}
struct AudioBufferSourceMono {
    pub source_buffer: AudioBufferMono,
    pub play_cursor_buffer_index: usize,

    pub is_looping: bool,
    pub loop_start_sampleindex: usize,
    /// NOTE: The end sample index is included in the loop
    pub loop_end_sampleindex: usize,
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
}
impl Iterator for AudioBufferSourceMono {
    type Item = AudioSample;
    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .source_buffer
            .samples
            .get(self.play_cursor_buffer_index)
            .cloned();

        self.play_cursor_buffer_index = usize::max(
            self.play_cursor_buffer_index + 1,
            self.source_buffer.samples.len(),
        );
        if self.is_looping {
            if self.play_cursor_buffer_index > self.loop_end_sampleindex {
                self.play_cursor_buffer_index = self.loop_start_sampleindex;
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

trait AudioStream {
    fn process_mono(&mut self, output_samples: &mut [AudioSample], output_sample_rate_hz: usize);
    fn process_stereo(&mut self, output_frames: &mut [AudioFrame], output_sample_rate_hz: usize);
    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
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
    fn process_stereo(&mut self, output_samples: &mut [AudioFrame], output_sample_rate_hz: usize) {
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

    fn set_volume(&mut self, volume: f32) {
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
    fn process_mono(&mut self, output_samples: &mut [AudioSample], output_sample_rate_hz: usize) {
        unimplemented!()
    }
    fn set_volume(&mut self, volume: f32) {
        self.volume.set_volume(volume);
    }
    fn set_playback_speed(&mut self, playback_speed: f32) {
        self.stream.set_playback_speed(playback_speed);
    }
}

pub struct Audiostate {
    next_frame_index_to_output: AudioFrameIndex,
    audio_playback_rate_hz: usize,

    dsp_time: f64,
    previous_dsp_query_time: std::time::Instant,
    previous_dsp_query_next_frame_index: AudioFrameIndex,

    global_playback_speed: f32,

    listener_pos: Vec2,
    listener_vel: Vec2,

    /// This can never be 0 when used with `get_next_stream_id` method
    next_stream_id: AudioStreamId,

    streams: HashMap<AudioStreamId, Box<dyn AudioStream>>,
    streams_to_delete_after_finish: HashSet<AudioStreamId>,
}
impl Clone for Audiostate {
    fn clone(&self) -> Self {
        todo!()
    }
}

enum StreamCompletion {
    StartingInSeconds(f32),
    RunningPercentage(f32),
    FinishedSecondsAgo(f32),
}

impl Audiostate {
    pub fn new(audio_playback_rate_hz: usize) -> Audiostate {
        Audiostate {
            next_frame_index_to_output: 0,
            audio_playback_rate_hz,

            dsp_time: 0.0,
            previous_dsp_query_time: std::time::Instant::now(),
            previous_dsp_query_next_frame_index: 0,

            global_playback_speed: 1.0,
            listener_pos: Vec2::zero(),

            listener_vel: Vec2::zero(),
            next_stream_id: 0,
            streams: HashMap::new(),
            streams_to_delete_after_finish: HashSet::new(),
        }
    }

    pub fn get_audio_time_estimate(&mut self) -> f64 {
        // Easing algorithm based on
        // https://www.reddit.com/r/gamedev/comments/13y26t/how_do_rhythm_games_stay_in_sync_with_the_music/

        let now_time = std::time::Instant::now();
        let time_since_last_query = now_time
            .duration_since(self.previous_dsp_query_time)
            .as_secs_f64();
        self.previous_dsp_query_time = now_time;

        self.dsp_time += time_since_last_query;
        if self.next_frame_index_to_output != self.previous_dsp_query_next_frame_index {
            self.dsp_time = (self.dsp_time
                + audio_frames_to_seconds(
                    self.next_frame_index_to_output,
                    self.audio_playback_rate_hz,
                ))
                / 2.0;
            self.previous_dsp_query_next_frame_index = self.next_frame_index_to_output;
        }

        self.dsp_time
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

    pub fn stream_completion_ratio(
        &self,
        stream_id: AudioStreamId,
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) -> Option<f32> {
        todo!()
        /*
        if let Some(stream) = self.streams_buffer_mono.get(&stream_id) {


            self.currently_playing_frame_index < stream.start_frame
            if stream.has_finished {
                return Some(1.0);
            }

            if let Some(start_frame_index) = stream.start_frame {
                if self.current_frame_index < start_frame_index {
                    return None;
                }
                let stream_frames = recordings.get(&stream.recording_name).unwrap();
                let stream_len = stream_frames.len() as AudioFrameIndex;

                // NOTE: We use modulus here to account for repeating streams
                let completed_frames_count =
                    (self.current_frame_index - start_frame_index) % stream_len;
                return Some(completed_frames_count as f32 / stream_len as f32);
            }

            return None;
        }
        */
    }

    fn get_next_stream_id(&mut self) -> AudioStreamId {
        self.next_stream_id += 1;
        self.next_stream_id
    }

    pub fn set_global_playback_speed(&mut self, global_playback_speed: f32) {
        self.global_playback_speed = global_playback_speed;
    }

    pub fn render_audio(
        &mut self,
        out_frames: &mut [AudioFrame],
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) {
        // Remove streams that have finished
        let mut streams_removed = vec![];
        for &stream_id in &self.streams_to_delete_after_finish {
            if self.get_stream(stream_id).has_finished() {
                self.streams.remove(&stream_id);
            }
            streams_removed.push(stream_id);
        }
        for stream_id in streams_removed {
            self.streams_to_delete_after_finish.remove(&stream_id);
        }

        // Render samples
        for stream in self.streams.values_mut() {
            stream.process_stereo(out_frames, self.audio_playback_rate_hz);
        }
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

    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, volume: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_volume(volume);
    }

    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, playback_speed: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_playback_speed(playback_speed);
    }

    #[must_use]
    pub fn play_spatial(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        is_repeating: bool,
        volume: f32,
        playback_speed: f32,
        pos: Vec2,
    ) -> AudioStreamId {
        // TODO
        0
    }

    #[must_use]
    pub fn play(
        &mut self,
        recording_name: &str,
        schedule_time: SchedulePlay,
        is_repeating: bool,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) -> AudioStreamId {
        let id = self.get_next_stream_id();
        if recording_name == "sine" {
            let stream = Box::new(AudioStreamStereo::new(
                Box::new(AudioSourceSine::new(440.0, 44100)),
                0,
                1.0,
                1.0,
                0.0,
            ));
            self.streams.insert(id, stream);
        }
        // TODO
        id
    }
    pub fn play_oneshot(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) {
        let _ = self.play(
            recording_name,
            play_time,
            false,
            volume,
            pan,
            playback_speed,
        );
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct AudiostateOld {
    /// This is can be used for interpolating time-based things that are dependent on music / beats
    current_frame_index: AudioFrameIndex,

    streams: HashMap<AudioStreamId, AudioStreamOld>,
    next_stream_id: AudioStreamId,
}

impl AudiostateOld {
    pub fn new() -> AudiostateOld {
        AudiostateOld {
            current_frame_index: 0,

            streams: HashMap::new(),
            next_stream_id: 1,
        }
    }

    pub fn update_frame_index(&mut self, current_frame_index: AudioFrameIndex) {
        self.current_frame_index = current_frame_index;
    }

    pub fn play_oneshot(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) {
        let _ = self.play(
            recording_name,
            play_time,
            false,
            volume,
            pan,
            playback_speed,
        );
    }

    #[must_use]
    pub fn play(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        is_repeating: bool,
        volume: f32,
        pan: f32,
        playback_speed: f32,
    ) -> AudioStreamId {
        let id = self.next_stream_id;
        let stream = AudioStreamOld {
            recording_name: recording_name.to_owned(),
            play_time,
            remove_on_finish: false,
            has_finished: false,
            start_frame: None,
            is_repeating,
            volume_current: volume,
            volume_target: volume,
            pan_current: pan,
            pan_target: pan,
            playback_speed_current: playback_speed,
            playback_speed_target: playback_speed,
        };
        self.streams.insert(id, stream);
        self.next_stream_id += 1;

        id
    }

    /*

    /// NOTE: This method needs to be fast because we are effectively blocking our audio callback
    ///       thread here
    pub fn render_audio_old(
        &mut self,
        first_chunk_index: AudioChunkIndex,
        out_chunk_count: usize,
        out_chunks: &mut Vec<Audiochunk>,
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) {
        // NOTE: We just want to make sure that the caller works with the same buffersize as we do
        assert!(out_chunk_count == AUDIO_BUFFERSIZE_IN_CHUNKS);

        // Clear output
        self.out_frames.iter_mut().for_each(|frame| {
            *frame = AudioFrame::silence();
        });

        let out_start_frame = audio_chunks_to_frames(first_chunk_index);

        for stream in self.streams.values_mut() {
            if stream.start_frame.is_none() {
                let segment_length = match stream.play_time {
                    SchedulePlay::Immediately => audio_frames_to_seconds(1),
                    SchedulePlay::OnNextMeasure {
                        beats_per_minute,
                        beats_per_measure,
                    } => audio_measure_length_in_seconds(beats_per_measure, beats_per_minute),
                    SchedulePlay::OnNextBeat { beats_per_minute } => {
                        audio_beat_length_in_seconds(beats_per_minute)
                    }
                    SchedulePlay::OnNextHalfBeat { beats_per_minute } => {
                        audio_beat_length_in_seconds(beats_per_minute) / 2.0
                    }
                    SchedulePlay::OnNextQuarterBeat { beats_per_minute } => {
                        audio_beat_length_in_seconds(beats_per_minute) / 4.0
                    }
                };

                let start_second = audio_frames_to_seconds(out_start_frame);
                let start_frame = audio_seconds_to_frames(
                    f64::ceil(start_second / segment_length) * segment_length,
                ) as AudioFrameIndex;

                stream.start_frame = Some(start_frame);
            }

            let stream_start_frame = stream.start_frame.unwrap();
            if out_start_frame + self.out_frames.len() as AudioFrameIndex <= stream_start_frame {
                // This stream will not start yet
                continue;
            }

            let pan = 0.5 * (stream.pan_current + 1.0); // Transform [-1,1] -> [0,1]
            let (volume_left, volume_right) = crossfade_sinuoidal(stream.volume_current, pan);

            let stream_frames = recordings.get(&stream.recording_name).unwrap();
            stream.has_finished = if stream.is_repeating {
                audio_add_stream_repeated(
                    out_start_frame,
                    &mut self.out_frames,
                    stream_start_frame,
                    stream_frames,
                    volume_left,
                    volume_right,
                );
                false
            } else {
                audio_add_stream(
                    out_start_frame,
                    &mut self.out_frames,
                    stream_start_frame,
                    stream_frames,
                    volume_left,
                    volume_right,
                )
            };
        }

        // Remove streams that have finished
        self.streams.retain(|_id, stream| !stream.has_finished);

        // Create audio chunks from our frames
        for frame_chunk in self.out_frames.chunks_exact(AUDIO_CHUNKLENGTH_IN_FRAMES) {
            let mut sample_chunk = [0; AUDIO_CHUNKLENGTH_IN_SAMPLES];
            for (sample_pair, frame) in sample_chunk.chunks_exact_mut(2).zip(frame_chunk.iter()) {
                sample_pair[0] = (frame.left * std::i16::MAX as f32) as i16;
                sample_pair[1] = (frame.right * std::i16::MAX as f32) as i16;
            }
            out_chunks.push(sample_chunk);
        }
    }
    */
}

/// Returns true if the given stream has finished
fn audio_add_stream(
    out_start_frame: AudioFrameIndex,
    out_frames: &mut [AudioFrame],
    stream_start_frame: AudioFrameIndex,
    stream_frames: &[AudioFrame],
    volume_left: f32,
    volume_right: f32,
) -> bool {
    let out_interval = Interval::new(out_start_frame, out_frames.len());
    let stream_interval = Interval::new(stream_start_frame, stream_frames.len());
    let intersection_interval = Interval::intersect(out_interval, stream_interval);

    // Check if our stream is even hit in this write cycle
    if intersection_interval.len() == 0 {
        if stream_interval.end <= out_start_frame {
            // NOTE: The stream has finished
            return true;
        } else {
            // NOTE: The stream has not started yet
            return false;
        }
    }

    // Calculate read and write ranges
    let read_interval = intersection_interval.offsetted_by(-stream_start_frame);
    let write_interval = intersection_interval.offsetted_by(-out_start_frame);
    assert!(read_interval.len() == write_interval.len());

    let read_range = read_interval.as_range_usize();
    let write_range = write_interval.as_range_usize();

    // Sum recording into our output
    for (write_frame, read_frame) in out_frames[write_range]
        .iter_mut()
        .zip(stream_frames[read_range].iter())
    {
        write_frame.left += read_frame.left * volume_left;
        write_frame.right += read_frame.right * volume_right;
    }

    false
}

fn audio_add_stream_repeated(
    out_start_frame: AudioFrameIndex,
    out_frames: &mut [AudioFrame],
    stream_start_frame: AudioFrameIndex,
    stream_frames: &[AudioFrame],
    volume_left: f32,
    volume_right: f32,
) {
    let out_interval = Interval::new(out_start_frame, out_frames.len());
    let stream_interval_repeated = Interval::from_start_end(stream_start_frame, std::i64::MAX);

    let intersection_interval = Interval::intersect(out_interval, stream_interval_repeated);
    if intersection_interval.is_empty() {
        // NOTE: The stream has not started yet
        return;
    }

    // Examples:
    // ..............[...|xx]xxxxxxxxxxxxxx|xxxxxxxxxxxxxxxxx|xxxxxxxxxxxxxxxxx|
    // ..................|xxxxx[xxxxxx]xxxx|xxxxxxxxxxxxxxxxx|xxxxxxxxxxxxxxxxx|
    // ..................|xxxxxxxxxxxxxxxxx|xxxxxxxxxxxxx[xxx|xx]xxxxxxxxxxxxxx|
    // ..............[...|xx|xx|xx|xx|x]|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|
    // ..................|xx|xx|xx|x[|xx|xx|xx|xx|xx|x]|xx|xx|xx|xx|xx|xx|xx|xx|
    // ..[...............|]x|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|xx|
    //
    // We want to shift our stream_interval to the right just until we overlap with our
    // out_interval such that the end of our shifted stream_interval window is right after the
    // start of our out_interval.
    //
    // In other words: We want to find the smallest integer shift >= 0 such that:
    // (stream_start_frame + stream_length) + shift * stream_length >= out_start_frame

    // NOTE: We need to make sure that the shift is never negative so we max it with 0.
    //       A negative shift can happen i.e. for the last case of the above examples
    let shift = AudioFrameIndex::max(
        0,
        (out_start_frame - stream_start_frame) / (stream_frames.len() as AudioFrameIndex),
    );

    let adjusted_stream_start_frame =
        stream_start_frame + shift * stream_frames.len() as AudioFrameIndex;
    let mut window = Interval::new(adjusted_stream_start_frame, stream_frames.len());

    // We can now render our frames while moving the window right until it does not overlap with the
    // out_frames intervall anymore
    while window.start < out_interval.end {
        audio_add_stream(
            out_start_frame,
            out_frames,
            window.start,
            stream_frames,
            volume_left,
            volume_right,
        );
        window = window.offsetted_by(stream_frames.len() as i64);
    }
}

//--------------------------------------------------------------------------------------------------
// Intervals

use std::ops::Range;

/// This represents the half open integer interval [start, end[ or [start, end-1] respectively
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Interval {
    pub start: i64,
    pub end: i64,
}

use std::{convert::TryFrom, sync::Arc};

// Conversion
impl Interval {
    pub fn as_range(self) -> Range<i64> {
        self.start..self.end
    }

    pub fn as_range_usize(self) -> Range<usize> {
        assert!(0 <= self.start && self.start <= self.end);
        let start = usize::try_from(self.start).expect(&format!(
            "Failed to create range: cannot convert start={} to usize",
            self.start
        ));
        let end = usize::try_from(self.end).expect(&format!(
            "Failed to create range: cannot convert end={} to usize",
            self.end
        ));
        start..end
    }
}

// Creation
impl Interval {
    #[inline]
    pub fn new(start: i64, length: usize) -> Interval {
        Interval {
            start,
            end: start + length as i64,
        }
    }

    pub fn from_range(range: Range<i64>) -> Interval {
        Interval {
            start: range.start,
            end: range.end,
        }
    }

    pub fn from_start_end(start: i64, end: i64) -> Interval {
        Interval { start, end }
    }
}

// Operations
impl Interval {
    #[inline]
    pub fn len(self) -> usize {
        let len = i64::max(0, self.end - self.start);
        usize::try_from(len).expect(&format!(
            "Failed to determine length of range: cannot convert {} to usize",
            len
        ))
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }

    #[must_use]
    #[inline]
    pub fn offsetted_by(self, offset: i64) -> Interval {
        Interval {
            start: self.start + offset,
            end: self.end + offset,
        }
    }

    #[inline]
    pub fn intersect(a: Interval, b: Interval) -> Interval {
        Interval {
            start: i64::max(a.start, b.start),
            end: i64::min(a.end, b.end),
        }
    }

    /// Returns the set-theoretic difference
    ///   `a - b = a / (a intersection b)`
    /// as (left, right)
    #[inline]
    pub fn difference(a: Interval, b: Interval) -> (Interval, Interval) {
        let intersection = Interval::intersect(a, b);
        let left = Interval {
            start: a.start,
            end: intersection.start,
        };
        let right = Interval {
            start: intersection.end,
            end: a.end,
        };

        (left, right)
    }
}
