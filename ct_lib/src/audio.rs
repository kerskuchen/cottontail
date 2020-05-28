use super::math::*;

use std::collections::HashMap;

pub type AudioSample = f32;

pub fn db_to_volume(db: f32) -> f32 {
    f32::powf(10.0, 0.05 * db)
}

pub fn volume_to_db(volume: f32) -> f32 {
    20.0 * f32::log10(volume)
}

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
// Audiostate

#[derive(Clone)]
struct AudioStreamSine {
    sine_frequency: f64,
    volume: f64,
    stream_frames_per_second: usize,

    sine_time: f64,
}

impl AudioStreamSine {
    fn new(sine_frequency: f64, stream_frames_per_second: usize, volume: f64) -> AudioStreamSine {
        AudioStreamSine {
            sine_frequency,
            volume,
            stream_frames_per_second,

            sine_time: 0.0,
        }
    }

    fn mix(&mut self, out_frames: &mut [AudioFrame]) {
        let time_increment = audio_frames_to_seconds(1, self.stream_frames_per_second);
        for write_frame in out_frames {
            let sine_amplitude = f64::sin(self.sine_time * 2.0 * PI64);

            write_frame.left += (sine_amplitude * self.volume) as f32;
            write_frame.right += (sine_amplitude * self.volume) as f32;

            self.sine_time += self.sine_frequency * time_increment;
        }
    }
}

#[derive(Clone)]
pub struct Audiostate {
    /// This is can be used for interpolating time-based things that are dependent on music / beats
    currently_playing_frame_index: AudioFrameIndex,
    next_frame_index_to_output: AudioFrameIndex,

    output_frames_per_second: usize,

    streams: HashMap<AudioStreamId, AudioStreamOld>,
    next_stream_id: AudioStreamId,

    sine_stream: AudioStreamSine,
}

impl Audiostate {
    pub fn new(output_frames_per_second: usize) -> Audiostate {
        Audiostate {
            currently_playing_frame_index: 0,
            next_frame_index_to_output: 0,
            streams: HashMap::new(),
            next_stream_id: 1,
            output_frames_per_second,

            sine_stream: AudioStreamSine::new(440.0, output_frames_per_second, 0.1),
        }
    }

    pub fn update_current_playcursor_time(&mut self, current_playcursor_time: f64) {
        self.currently_playing_frame_index =
            audio_seconds_to_frames(current_playcursor_time, self.output_frames_per_second)
                as AudioFrameIndex
    }

    pub fn render_audio(
        &mut self,
        out_frames: &mut [AudioFrame],
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) {
        self.sine_stream.mix(out_frames);
    }

    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, new_volume: f32) {
        self.sine_stream.volume = new_volume as f64;
        // TODO
    }

    pub fn stream_set_frequency(&mut self, stream_id: AudioStreamId, frequency: f32) {
        self.sine_stream.sine_frequency = 440.0 + 220.0 * frequency as f64;
        // TODO
    }

    pub fn stream_set_pan(&mut self, stream_id: AudioStreamId, new_pan: f32) {

        // TODO
    }

    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, new_playback_speed: f32) {
        // TODO
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
        // TODO
        0
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
    pub fn stream_completion_ratio(
        &self,
        stream_id: AudioStreamId,
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) -> Option<f32> {
        // TODO
        return None;
    }
}

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

    pub fn stream_completion_ratio(
        &self,
        stream_id: AudioStreamId,
        recordings: &HashMap<String, Vec<AudioFrame>>,
    ) -> Option<f32> {
        let stream = self
            .streams
            .get(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id));

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

    pub fn stream_has_finished(&self, stream_id: AudioStreamId) -> bool {
        self.streams
            .get(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id))
            .has_finished
    }

    pub fn stream_forget(&mut self, stream_id: AudioStreamId) {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id));

        assert!(
            !stream.is_repeating,
            "Cannot forget repeating audio stream {:?}",
            stream
        );
        stream.remove_on_finish = true;
    }

    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, new_volume: f32) {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id));
        stream.volume_target = new_volume;
        stream.volume_current = new_volume;
    }

    pub fn stream_set_pan(&mut self, stream_id: AudioStreamId, new_pan: f32) {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id));
        stream.pan_target = new_pan;
        stream.pan_current = new_pan;
    }

    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, new_playback_speed: f32) {
        let stream = self
            .streams
            .get_mut(&stream_id)
            .expect(&format!("No audio stream found for id {}", stream_id));
        stream.playback_speed_target = new_playback_speed;
        stream.playback_speed_current = new_playback_speed;
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

use std::convert::TryFrom;

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
