use super::math::*;

use std::collections::{HashMap, VecDeque};

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
// Audio output

pub type AudioFrameIndex = i64;
pub type AudioChunkIndex = i64;
pub type AudioSampleIndex = i64;

pub const AUDIO_FREQUENCY: usize = 44100;
pub const AUDIO_CHANNELCOUNT: usize = 2;

pub const AUDIO_FRAMELENGTH_IN_SECONDS: f64 = 1.0 / AUDIO_FREQUENCY as f64;

pub const AUDIO_CHUNKLENGTH_IN_FRAMES: usize = 256;
pub const AUDIO_CHUNKLENGTH_IN_SAMPLES: usize = AUDIO_CHUNKLENGTH_IN_FRAMES * 2;
pub const AUDIO_CHUNKLENGTH_IN_SECONDS: f64 =
    AUDIO_CHUNKLENGTH_IN_FRAMES as f64 / AUDIO_FREQUENCY as f64;

pub const AUDIO_BUFFERSIZE_IN_FRAMES: usize = (AUDIO_FREQUENCY / 2);
pub const AUDIO_BUFFERSIZE_IN_CHUNKS: usize =
    AUDIO_BUFFERSIZE_IN_FRAMES / AUDIO_CHUNKLENGTH_IN_FRAMES;

pub type Audiochunk = [i16; AUDIO_CHUNKLENGTH_IN_SAMPLES];

#[inline]
pub fn audio_chunks_to_frames(chunk_index: AudioChunkIndex) -> AudioFrameIndex {
    chunk_index * AUDIO_CHUNKLENGTH_IN_FRAMES as AudioFrameIndex
}

#[inline]
pub fn audio_chunks_to_seconds(chunk_index: AudioChunkIndex) -> f64 {
    chunk_index as f64 * AUDIO_CHUNKLENGTH_IN_SECONDS as f64
}

#[inline]
pub fn audio_frames_to_seconds(frame_index: AudioFrameIndex) -> f64 {
    frame_index as f64 / AUDIO_FREQUENCY as f64
}

#[inline]
/// NOTE: This returns a float so we can round it down ourselves or use the value for further
///       calculations without forced rounding errors
pub fn audio_seconds_to_frames(time: f64) -> f64 {
    time * AUDIO_FREQUENCY as f64
}

#[inline]
pub fn audio_beat_length_in_seconds(beats_per_minute: usize) -> f64 {
    60.0 / (beats_per_minute as f64)
}

#[inline]
pub fn audio_measure_length_in_seconds(beats_per_measure: usize, beats_per_minute: usize) -> f64 {
    beats_per_measure as f64 * audio_beat_length_in_seconds(beats_per_minute)
}

pub struct AudioOutput {
    pub next_chunk_index: AudioChunkIndex,
    pub buffer: VecDeque<Audiochunk>,

    pub previous_dsp_query_time: std::time::Instant,
    pub previous_dsp_query_next_chunk_index: AudioChunkIndex,
    pub dsp_time: f64,
}

impl AudioOutput {
    pub fn new() -> AudioOutput {
        AudioOutput {
            next_chunk_index: 0,
            buffer: VecDeque::new(),

            previous_dsp_query_time: std::time::Instant::now(),
            previous_dsp_query_next_chunk_index: 0,
            dsp_time: 0.0,
        }
    }

    pub fn get_audio_time_in_frames(&mut self) -> AudioFrameIndex {
        // Easing algorithm based on
        // https://www.reddit.com/r/gamedev/comments/13y26t/how_do_rhythm_games_stay_in_sync_with_the_music/

        let now_time = std::time::Instant::now();
        let deltatime = now_time
            .duration_since(self.previous_dsp_query_time)
            .as_secs_f64();
        self.previous_dsp_query_time = now_time;

        self.dsp_time += deltatime;
        if self.next_chunk_index != self.previous_dsp_query_next_chunk_index {
            self.dsp_time =
                (self.dsp_time + audio_chunks_to_seconds(self.next_chunk_index - 1)) / 2.0;
            self.previous_dsp_query_next_chunk_index = self.next_chunk_index;
        }

        audio_seconds_to_frames(self.dsp_time) as AudioFrameIndex
    }

    /// NOTE: This drains the given chunks
    pub fn replace_chunks(
        &mut self,
        first_chunk_index: AudioChunkIndex,
        chunks: &mut Vec<Audiochunk>,
    ) {
        // NOTE: The audio chunk given to us must start at the same index as our next chunk
        //       index. Everything else is a bug
        assert!(first_chunk_index == self.next_chunk_index);

        self.buffer.clear();
        self.buffer.extend(chunks.drain(..));
    }

    pub fn next_chunk(&mut self) -> (AudioChunkIndex, Option<Audiochunk>) {
        let chunk_index = self.next_chunk_index;
        self.next_chunk_index += 1;
        (chunk_index, self.buffer.pop_front())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostate

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
struct AudioStream {
    recording_name: String,

    play_time: SchedulePlay,
    start_frame: Option<AudioFrameIndex>,

    remove_on_finish: bool,
    has_finished: bool,
    is_repeating: bool,

    /// Ranges in [0,1]
    /// Silence       = 0
    /// Full loudness = 1
    volume: f32,

    /// Ranges in [-1,1]
    /// Left   = -1
    /// Center =  0
    /// Right  =  1
    pan: f32,
}

#[derive(Clone)]
pub struct Audiostate {
    /// This is can be used for interpolating time-based things that are dependent on music / beats
    current_frame_index: AudioFrameIndex,

    recordings: HashMap<String, Vec<AudioFrame>>,

    streams: HashMap<AudioStreamId, AudioStream>,
    next_stream_id: AudioStreamId,

    out_frames: Vec<AudioFrame>,
}

impl Audiostate {
    pub fn new() -> Audiostate {
        Audiostate {
            current_frame_index: 0,

            recordings: HashMap::new(),

            streams: HashMap::new(),
            next_stream_id: 1,

            out_frames: vec![AudioFrame::silence(); AUDIO_BUFFERSIZE_IN_FRAMES],
        }
    }

    pub fn update_frame_index(&mut self, current_frame_index: AudioFrameIndex) {
        self.current_frame_index = current_frame_index;
    }

    pub fn stream_completion_ratio(&self, stream_id: AudioStreamId) -> Option<f32> {
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
            let stream_frames = self.recordings.get(&stream.recording_name).unwrap();
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

    pub fn play_oneshot(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        volume: f32,
        pan: f32,
    ) {
        let _ = self.play(recording_name, play_time, false, volume, pan);
    }

    #[must_use]
    pub fn play(
        &mut self,
        recording_name: &str,
        play_time: SchedulePlay,
        is_repeating: bool,
        volume: f32,
        pan: f32,
    ) -> AudioStreamId {
        let id = self.next_stream_id;
        let stream = AudioStream {
            recording_name: recording_name.to_owned(),
            play_time,
            remove_on_finish: false,
            has_finished: false,
            start_frame: None,
            is_repeating,
            volume,
            pan,
        };
        self.streams.insert(id, stream);
        self.next_stream_id += 1;

        id
    }

    pub fn add_recording_mono(&mut self, name: &str, data: Vec<AudioSample>) {
        assert!(!self.recordings.contains_key(name));

        let recording_frames = data
            .into_iter()
            .map(|mono_sample| AudioFrame {
                left: mono_sample,
                right: mono_sample,
            })
            .collect();

        self.recordings.insert(name.to_owned(), recording_frames);
    }

    /// NOTE: This method needs to be fast because we are effectively blocking our audio callback
    ///       thread here
    pub fn render_audio(
        &mut self,
        first_chunk_index: AudioChunkIndex,
        out_chunk_count: usize,
        out_chunks: &mut Vec<Audiochunk>,
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

            // NOTE: We use sinuoidal panning:
            //       http://folk.ntnu.no/oyvinbra/delete/Lesson1Panning.html
            let pan = 0.5 * (stream.pan + 1.0); // Transform [-1,1] -> [0,1]
            let volume_left = stream.volume * f32::cos((PI / 2.0) * pan);
            let volume_right = stream.volume * f32::sin((PI / 2.0) * pan);

            let stream_frames = self.recordings.get(&stream.recording_name).unwrap();
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
}

/// Returns if the given stream has finished
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

#[allow(dead_code)]
fn audio_add_sine(
    out_start_frame: AudioFrameIndex,
    out_frames: &mut [AudioFrame],
    volume_left: f32,
    volume_right: f32,
    sine_frequency: f32,
) {
    let sine_frequency = sine_frequency as f64;

    let mut sine_time = out_start_frame as f64 * AUDIO_FRAMELENGTH_IN_SECONDS;
    for write_frame in out_frames {
        let sine_amplitude = f64::sin(sine_frequency * sine_time * 2.0 * PI64);
        sine_time += AUDIO_FRAMELENGTH_IN_SECONDS;

        write_frame.left += sine_amplitude as f32 * volume_left;
        write_frame.right += sine_amplitude as f32 * volume_right;
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
