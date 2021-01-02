use ct_lib_core::log;
use lewton::inside_ogg::OggStreamReader;

use super::math::*;

use core::panic;
use std::{cell::RefCell, rc::Rc};
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Samples and Frames

pub type AudioSample = f32;
pub const AUDIO_CHUNKSIZE_IN_FRAMES: usize = 512;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AudioChannels {
    Mono,
    Stereo,
}

#[derive(Clone)]
pub struct AudioChunk {
    pub channels: AudioChannels,
    pub volume: f32,
    frames: [Vec<AudioSample>; 2],
}
impl AudioChunk {
    pub fn new_mono() -> AudioChunk {
        AudioChunk::new_mono_with_framecount(AUDIO_CHUNKSIZE_IN_FRAMES)
    }

    pub fn new_mono_with_framecount(framecount: usize) -> AudioChunk {
        AudioChunk::new_mono_from_frames(vec![0.0; framecount])
    }

    pub fn new_mono_from_frames(samples: Vec<AudioSample>) -> AudioChunk {
        AudioChunk {
            channels: AudioChannels::Mono,
            volume: 1.0,
            frames: [samples, Vec::new()],
        }
    }

    pub fn new_stereo() -> AudioChunk {
        AudioChunk::new_stereo_with_framecount(AUDIO_CHUNKSIZE_IN_FRAMES)
    }

    pub fn new_stereo_with_framecount(framecount: usize) -> AudioChunk {
        AudioChunk::new_stereo_from_frames(vec![0.0; framecount], vec![0.0; framecount])
    }

    pub fn new_stereo_from_frames(
        samples_left: Vec<AudioSample>,
        samples_right: Vec<AudioSample>,
    ) -> AudioChunk {
        AudioChunk {
            channels: AudioChannels::Stereo,
            volume: 1.0,
            frames: [samples_left, samples_right],
        }
    }

    pub fn len(&self) -> usize {
        self.frames[0].len()
    }

    pub fn length_in_seconds(&self, audio_samplerate_hz: usize) -> f64 {
        audio_frames_to_seconds(self.len() as i64, audio_samplerate_hz)
    }

    pub fn channelcount(&self) -> usize {
        match self.channels {
            AudioChannels::Mono => 1,
            AudioChannels::Stereo => 2,
        }
    }

    #[inline(always)]
    pub fn get_mono_samples(&self) -> &[AudioSample] {
        assert!(self.channels == AudioChannels::Mono);
        &self.frames[0]
    }
    #[inline(always)]
    pub fn get_mono_samples_mut(&mut self) -> &mut [AudioSample] {
        assert!(self.channels == AudioChannels::Mono);
        &mut self.frames[0]
    }
    #[inline(always)]
    pub fn get_stereo_samples(&self) -> (&[AudioSample], &[AudioSample]) {
        assert!(self.channels == AudioChannels::Stereo);
        let (samples_left, samples_right) = self.frames.split_at(1);
        (&samples_left[0], &samples_right[0])
    }
    #[inline(always)]
    pub fn get_stereo_samples_mut(&mut self) -> (&mut [AudioSample], &mut [AudioSample]) {
        assert!(self.channels == AudioChannels::Stereo);
        let (samples_left, samples_right) = self.frames.split_at_mut(1);
        (&mut samples_left[0], &mut samples_right[0])
    }
    #[inline(always)]
    pub fn get_stereo_samples_left(&self) -> &[AudioSample] {
        assert!(self.channels == AudioChannels::Stereo);
        &self.frames[0]
    }
    #[inline(always)]
    pub fn get_stereo_samples_left_mut(&mut self) -> &mut [AudioSample] {
        assert!(self.channels == AudioChannels::Stereo);
        &mut self.frames[0]
    }
    #[inline(always)]
    pub fn get_stereo_samples_right(&self) -> &[AudioSample] {
        assert!(self.channels == AudioChannels::Stereo);
        &self.frames[1]
    }
    #[inline(always)]
    pub fn get_stereo_samples_right_mut(&mut self) -> &mut [AudioSample] {
        assert!(self.channels == AudioChannels::Stereo);
        &mut self.frames[1]
    }

    pub fn reset(&mut self) {
        self.volume = 1.0;

        match self.channels {
            AudioChannels::Mono => {
                for sample in self.get_mono_samples_mut().iter_mut() {
                    *sample = 0.0
                }
            }
            AudioChannels::Stereo => {
                for sample_left in self.get_stereo_samples_left_mut().iter_mut() {
                    *sample_left = 0.0
                }
                for sample_right in self.get_stereo_samples_right_mut().iter_mut() {
                    *sample_right = 0.0
                }
            }
        }
    }

    pub fn add_from_chunk(&mut self, other: &AudioChunk) {
        assert!(self.channels == other.channels);
        assert!(self.len() == other.len());

        // Fastpath: Other chunk is silent
        if other.volume == 0.0 {
            return;
        }

        let chunk_volume = other.volume;
        match self.channels {
            AudioChannels::Mono => {
                for (sample_out, sample_in) in self
                    .get_mono_samples_mut()
                    .iter_mut()
                    .zip(other.get_mono_samples().iter())
                {
                    *sample_out += chunk_volume * sample_in;
                }
            }
            AudioChannels::Stereo => {
                for (sample_out_left, sample_in_left) in self
                    .get_stereo_samples_left_mut()
                    .iter_mut()
                    .zip(other.get_stereo_samples_left().iter())
                {
                    *sample_out_left += chunk_volume * sample_in_left;
                }
                for (sample_out_right, sample_in_right) in self
                    .get_stereo_samples_right_mut()
                    .iter_mut()
                    .zip(other.get_stereo_samples_right().iter())
                {
                    *sample_out_right += chunk_volume * sample_in_right;
                }
            }
        }
    }

    pub fn copy_chunks(
        source: &AudioChunk,
        target: &mut AudioChunk,
        source_offset: usize,
        target_offset: usize,
        framecount: usize,
    ) {
        assert!(source.volume == target.volume);
        assert!(source.channels == target.channels);
        assert!(source_offset < source.len());
        assert!(target_offset < target.len());
        assert!(source_offset + framecount <= source.len());
        assert!(target_offset + framecount <= target.len());

        match source.channels {
            AudioChannels::Mono => {
                let source_samples =
                    &source.get_mono_samples()[source_offset..source_offset + framecount];
                let target_samples =
                    &mut target.get_mono_samples_mut()[target_offset..target_offset + framecount];
                target_samples.copy_from_slice(source_samples)
            }

            AudioChannels::Stereo => {
                let (source_left, source_right) = {
                    let (left, right) = source.get_stereo_samples();
                    (
                        &left[source_offset..source_offset + framecount],
                        &right[source_offset..source_offset + framecount],
                    )
                };
                let (target_left, target_right) = {
                    let (left, right) = target.get_stereo_samples_mut();
                    (
                        &mut left[target_offset..target_offset + framecount],
                        &mut right[target_offset..target_offset + framecount],
                    )
                };
                target_left.copy_from_slice(source_left);
                target_right.copy_from_slice(source_right);
            }
        }
    }

    pub fn copy_from_chunk(
        &mut self,
        other: &AudioChunk,
        read_offset: usize,
        write_offset: usize,
        framecount: usize,
    ) {
        AudioChunk::copy_chunks(other, self, read_offset, write_offset, framecount);
    }

    pub fn copy_from_chunk_complete(&mut self, other: &AudioChunk) {
        assert!(self.channels == other.channels);
        assert!(self.len() == other.len());

        self.volume = other.volume;

        // Fastpath - chunk is silent
        if self.volume == 0.0 {
            return;
        }

        AudioChunk::copy_chunks(other, self, 0, 0, other.len());
    }

    pub fn copy_from_slice_mono(&mut self, source: &[AudioSample], offset: usize) {
        assert!(self.channels == AudioChannels::Mono);
        assert!(offset < self.len());
        assert!(offset + source.len() <= self.len());

        let target: &mut [f32] = self.get_mono_samples_mut();
        let target = &mut target[offset..offset + source.len()];
        target.copy_from_slice(source);
    }

    pub fn copy_from_slice_stereo(
        &mut self,
        source_left: &[AudioSample],
        source_right: &[AudioSample],
        offset: usize,
    ) {
        assert!(self.channels == AudioChannels::Stereo);
        assert!(source_left.len() == source_right.len());
        assert!(offset < self.len());
        assert!(offset + source_left.len() <= self.len());

        let framecount = source_left.len();
        let (target_left, target_right) = {
            let (left, right) = self.get_stereo_samples_mut();
            (
                &mut left[offset..offset + framecount],
                &mut right[offset..offset + framecount],
            )
        };

        target_left.copy_from_slice(source_left);
        target_right.copy_from_slice(source_right);
    }

    pub fn fill_silence_complete(&mut self) {
        self.volume = 0.0;
    }

    pub fn fill_silence_from_offset(&mut self, offset: usize) {
        self.fill_silence_range(offset, self.len());
    }

    pub fn fill_silence_offset_framecount(&mut self, offset: usize, framecount: usize) {
        self.fill_silence_range(offset, offset + framecount);
    }

    // NOTE: `end` is not inclusive and can be `end == chunk.len()`
    // Example: To fill the whole range we can call `chunk.fill_silence_range(0, chunk.len())`
    pub fn fill_silence_range(&mut self, start: usize, end: usize) {
        assert!(start <= end);
        assert!(start < self.len());
        assert!(end <= self.len());

        // Fastpath: Whole chunk is silent
        if start == 0 && end == self.len() {
            self.fill_silence_complete();
            return;
        }

        match self.channels {
            AudioChannels::Mono => {
                for sample in self.get_mono_samples_mut()[start..end].iter_mut() {
                    *sample = 0.0
                }
            }
            AudioChannels::Stereo => {
                for sample_left in self.get_stereo_samples_left_mut()[start..end].iter_mut() {
                    *sample_left = 0.0
                }
                for sample_right in self.get_stereo_samples_right_mut()[start..end].iter_mut() {
                    *sample_right = 0.0
                }
            }
        }
    }

    pub fn multipliy_volume_ramp(&mut self, volume_start: f32, volume_end: f32) {
        // Fast path - chunk is silent
        if self.volume == 0.0 {
            return;
        }

        // Fast path - all values are the same for the chunk
        if volume_start == volume_end {
            self.volume *= volume_end;
            return;
        }

        // Slow path - need to ramp up/down volume
        let volume_increment = (volume_end - volume_start) / self.len() as f32;
        let mut volume_current = volume_start;
        match self.channels {
            AudioChannels::Mono => {
                for out_sample in self.get_mono_samples_mut().iter_mut() {
                    *out_sample = volume_current * *out_sample;
                    volume_current += volume_increment;
                }
            }
            AudioChannels::Stereo => {
                let (samples_left, samples_right) = self.get_stereo_samples_mut();
                for (left, right) in samples_left.iter_mut().zip(samples_right.iter_mut()) {
                    *left = volume_current * *left;
                    *right = volume_current * *right;
                    volume_current += volume_increment;
                }
            }
        }
    }

    fn convert_mono_to_stereo_ramp(
        input: &AudioChunk,
        output: &mut AudioChunk,
        pan_start: f32,
        pan_end: f32,
    ) {
        assert!(input.channels == AudioChannels::Mono);
        assert!(output.channels == AudioChannels::Stereo);
        assert!(input.len() == output.len());

        output.volume = input.volume;

        // Fast path - input is silent
        if input.volume == 0.0 {
            return;
        }

        // Fast path - all pan values are the same for the chunk - no need to ramp up/down pan
        if pan_start == pan_end {
            let pan_percent = 0.5 * (pan_end + 1.0); // Transform [-1,1] -> [0,1]
            let (volume_left, volume_right) = crossfade_squareroot(1.0, pan_percent);

            let (samples_left, samples_right) = output.get_stereo_samples_mut();
            for (sample_mono, (out_left, out_right)) in input
                .get_mono_samples()
                .iter()
                .zip(samples_left.iter_mut().zip(samples_right.iter_mut()))
            {
                *out_left = volume_left * *sample_mono;
                *out_right = volume_right * *sample_mono;
            }
            return;
        }

        // Slow path - need to ramp up/down pan
        let percent_end = 0.5 * (pan_end + 1.0); // Transform [-1,1] -> [0,1]
        let percent_start = 0.5 * (pan_start + 1.0); // Transform [-1,1] -> [0,1]
        let percent_increment = (percent_end - percent_start) / input.len() as f32;
        let mut percent_current = percent_start;

        let (samples_left, samples_right) = output.get_stereo_samples_mut();
        for (sample_mono, (out_left, out_right)) in input
            .get_mono_samples()
            .iter()
            .zip(samples_left.iter_mut().zip(samples_right.iter_mut()))
        {
            let (sample_left, sample_right) = crossfade_squareroot(*sample_mono, percent_current);
            *out_left = sample_left;
            *out_right = sample_right;
            percent_current += percent_increment;
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Timing

pub type AudioFrameIndex = i64;

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
// Audiorecordings

pub struct AudioRecording {
    pub name: String,
    pub sample_rate_hz: usize,
    pub framechunk: AudioChunk,

    stream_reader: Option<OggStreamReader<Cursor<Vec<u8>>>>,
    stream_reader_decoded_framecount: usize,

    /// Defaults to 0
    pub loopsection_start_frameindex: usize,
    /// Defaults to framechunk.len()
    pub loopsection_framecount: usize,
}

impl AudioRecording {
    pub fn new(name: String, sample_rate_hz: usize, chunk: AudioChunk) -> AudioRecording {
        let framecount = chunk.len();
        AudioRecording::new_with_loopsection(name, sample_rate_hz, chunk, 0, framecount)
    }

    pub fn new_with_loopsection(
        name: String,
        sample_rate_hz: usize,
        framechunk: AudioChunk,
        loopsection_start_frameindex: usize,
        loopsection_framecount: usize,
    ) -> AudioRecording {
        let framecount = framechunk.len();
        AudioRecording {
            name,
            sample_rate_hz,
            framechunk,
            stream_reader: None,
            stream_reader_decoded_framecount: framecount,
            loopsection_start_frameindex,
            loopsection_framecount,
        }
    }

    pub fn new_from_ogg_stream(
        name: String,
        framecount: usize,
        ogg_data: Vec<u8>,
    ) -> Result<AudioRecording, String> {
        AudioRecording::new_from_ogg_stream_with_loopsection(
            name, framecount, ogg_data, 0, framecount,
        )
    }

    pub fn new_from_ogg_stream_with_loopsection(
        name: String,
        framecount: usize,
        ogg_data: Vec<u8>,
        loopsection_start_frameindex: usize,
        loopsection_framecount: usize,
    ) -> Result<AudioRecording, String> {
        let stream_reader = OggStreamReader::new(std::io::Cursor::new(ogg_data))
            .map_err(|error| format!("Could not decode ogg audio data: {}", error))?;

        let sample_rate_hz = stream_reader.ident_hdr.audio_sample_rate as usize;
        let framechunk = match stream_reader.ident_hdr.audio_channels {
            1 => AudioChunk::new_mono_with_framecount(framecount),
            2 => AudioChunk::new_stereo_with_framecount(framecount),
            _ => {
                return Err(format!(
                    "Expected ogg stream with 1 or 2 channels - got {} channels",
                    stream_reader.ident_hdr.audio_channels
                ))
            }
        };

        let mut result = AudioRecording {
            name: name.clone(),
            sample_rate_hz,
            framechunk,
            stream_reader: Some(stream_reader),
            stream_reader_decoded_framecount: 0,
            loopsection_framecount,
            loopsection_start_frameindex,
        };

        // NOTE: We pre-decode up to 1 seconds worth of audio frames directly because it is fast
        //       enough and most sounds are short enough to be completely decoded that way. Also
        //       it checks if a given stream is even decodable so that we can crash as soon as
        //       possible if it is not
        let initial_predecoded_framecount = usize::min(framecount, sample_rate_hz);
        result
            .decode_frames_till(initial_predecoded_framecount)
            .map_err(|error| {
                format!(
                    "Could not pre-decode ogg audio data of stream '{}': {}",
                    name, error
                )
            })?;

        Ok(result)
    }

    pub fn channels(&self) -> AudioChannels {
        self.framechunk.channels
    }

    pub fn len(&self) -> usize {
        self.framechunk.len()
    }

    pub fn output_frames(
        &mut self,
        source_start_frameindex: usize,
        framecount: usize,
        out_chunk: &mut AudioChunk,
        out_chunk_write_offset: usize,
    ) {
        assert!(source_start_frameindex < self.framechunk.len());
        assert!(source_start_frameindex + framecount <= self.framechunk.len());

        self.decode_frames_till(source_start_frameindex + framecount)
            .unwrap();

        out_chunk.copy_from_chunk(
            &self.framechunk,
            source_start_frameindex,
            out_chunk_write_offset,
            framecount,
        );
    }

    // NOTE: `frameindex` is allowed to be out of range and will default to maximum possible value
    pub fn decode_frames_till(&mut self, frameindex: usize) -> Result<(), String> {
        let frameindex = usize::min(frameindex, self.framechunk.len());
        if frameindex < self.stream_reader_decoded_framecount {
            // Nothing to do
            return Ok(());
        }

        assert!(
            self.stream_reader.is_some(),
            "Stream reader not existing but decoded framecount smaller than actual framecount"
        );

        let stream_reader = self.stream_reader.as_mut().unwrap();
        while let Some(decoded_samples) = stream_reader
            .read_dec_packet_generic::<Vec<Vec<AudioSample>>>()
            .map_err(|error| format!("Could not decode ogg packet: {}", error))?
        {
            let decoded_framecount = decoded_samples[0].len();
            if self.stream_reader_decoded_framecount + decoded_framecount > self.framechunk.len() {
                log::trace!(
                    "Decoded {} frames but expected {} frames in '{}'",
                    self.stream_reader_decoded_framecount + decoded_framecount,
                    self.framechunk.len(),
                    &self.name
                );
            }

            // Make sure we don't try to write more frames than we have
            let framecount_to_write = usize::min(
                decoded_samples[0].len(),
                self.framechunk.len() - self.stream_reader_decoded_framecount,
            );
            match self.framechunk.channels {
                AudioChannels::Mono => {
                    self.framechunk.copy_from_slice_mono(
                        &decoded_samples[0][..framecount_to_write],
                        self.stream_reader_decoded_framecount,
                    );
                }
                AudioChannels::Stereo => {
                    self.framechunk.copy_from_slice_stereo(
                        &decoded_samples[0][..framecount_to_write],
                        &decoded_samples[1][..framecount_to_write],
                        self.stream_reader_decoded_framecount,
                    );
                }
            }

            self.stream_reader_decoded_framecount += decoded_samples[0].len();
            if self.stream_reader_decoded_framecount == self.framechunk.len() {
                log::trace!("Finished decoding '{}'", &self.name);
            }
            if self.stream_reader_decoded_framecount >= frameindex {
                break;
            }
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiosources

trait AudioSourceTrait: Clone {
    fn sample_rate_hz(&self) -> usize;
    fn has_finished(&self) -> bool;
    fn is_looping(&self) -> bool;
    fn channels(&self) -> AudioChannels;
    fn completion_ratio(&self) -> Option<f32>;
    fn produce_chunk(&mut self, out_chunk: &mut AudioChunk);
    fn framecount(&self) -> Option<usize>;
    fn playcursor_pos(&self) -> Option<usize>;
}

#[derive(Clone)]
struct AudioSourceRecording {
    source_recording: Rc<RefCell<AudioRecording>>,
    play_cursor_pos: usize,
    is_looping: bool,
}
impl AudioSourceRecording {
    fn new(buffer: Rc<RefCell<AudioRecording>>, play_looped: bool) -> AudioSourceRecording {
        AudioSourceRecording {
            source_recording: buffer,
            play_cursor_pos: 0,
            is_looping: play_looped,
        }
    }
}
impl AudioSourceTrait for AudioSourceRecording {
    fn sample_rate_hz(&self) -> usize {
        self.source_recording.borrow().sample_rate_hz
    }
    fn has_finished(&self) -> bool {
        !self.is_looping && self.play_cursor_pos >= self.source_recording.borrow().framechunk.len()
    }
    fn is_looping(&self) -> bool {
        self.is_looping
    }
    fn completion_ratio(&self) -> Option<f32> {
        Some(self.play_cursor_pos as f32 / self.source_recording.borrow().framechunk.len() as f32)
    }

    fn produce_chunk(&mut self, out_chunk: &mut AudioChunk) {
        if self.has_finished() {
            out_chunk.volume = 0.0;
            return;
        }

        let mut source = self.source_recording.borrow_mut();
        assert!(out_chunk.channels == source.channels());

        out_chunk.volume = 1.0;

        if self.is_looping {
            let loopsection_end =
                source.loopsection_start_frameindex + source.loopsection_framecount;

            let mut framecount_remaining_to_output = out_chunk.len();
            while framecount_remaining_to_output > 0 {
                assert!(self.play_cursor_pos < loopsection_end);
                let framecount_remaining_in_loopsection = loopsection_end - self.play_cursor_pos;
                let write_framecount = usize::min(
                    framecount_remaining_to_output,
                    framecount_remaining_in_loopsection,
                );
                let out_chunk_write_offset = out_chunk.len() - framecount_remaining_to_output;
                source.output_frames(
                    self.play_cursor_pos,
                    write_framecount,
                    out_chunk,
                    out_chunk_write_offset,
                );

                self.play_cursor_pos += write_framecount;
                framecount_remaining_to_output -= write_framecount;

                assert!(self.play_cursor_pos <= loopsection_end);
                if self.play_cursor_pos == loopsection_end {
                    self.play_cursor_pos = source.loopsection_start_frameindex;
                }
            }
        } else {
            assert!(self.play_cursor_pos < source.len());
            let framecount_remaining_in_source = source.len() - self.play_cursor_pos;
            let write_framecount = usize::min(out_chunk.len(), framecount_remaining_in_source);
            source.output_frames(self.play_cursor_pos, write_framecount, out_chunk, 0);
            self.play_cursor_pos += write_framecount;
            let framecount_remaining_to_output = out_chunk.len() - write_framecount;

            if framecount_remaining_to_output > 0 {
                let silence_offset = out_chunk.len() - framecount_remaining_to_output;
                out_chunk.fill_silence_from_offset(silence_offset);
            }
        }
    }
    fn channels(&self) -> AudioChannels {
        self.source_recording.borrow().channels()
    }

    fn framecount(&self) -> Option<usize> {
        Some(self.source_recording.borrow().framechunk.len())
    }

    fn playcursor_pos(&self) -> Option<usize> {
        Some(self.play_cursor_pos)
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

    fn produce_chunk(&mut self, out_chunk: &mut AudioChunk) {
        assert!(out_chunk.channels == AudioChannels::Mono);

        out_chunk.volume = 1.0;
        let time_increment = audio_frames_to_seconds(1, self.sample_rate_hz);
        for out_frame in out_chunk.get_mono_samples_mut().iter_mut() {
            let sine_amplitude = f64::sin(self.sine_time * 2.0 * PI64);
            self.sine_time += self.sine_frequency * time_increment;
            *out_frame = sine_amplitude as AudioSample;
        }
    }

    fn channels(&self) -> AudioChannels {
        AudioChannels::Mono
    }

    fn framecount(&self) -> Option<usize> {
        None
    }

    fn playcursor_pos(&self) -> Option<usize> {
        None
    }
}

#[derive(Clone)]
enum AudioSource {
    Recording(AudioSourceRecording),
    Sine(AudioSourceSine),
}
impl AudioSource {
    fn new_from_recording(buffer: Rc<RefCell<AudioRecording>>, play_looped: bool) -> AudioSource {
        AudioSource::Recording(AudioSourceRecording::new(buffer, play_looped))
    }
    fn new_from_sine(sine_frequency: f64, stream_frames_per_second: usize) -> AudioSource {
        AudioSource::Sine(AudioSourceSine::new(
            sine_frequency,
            stream_frames_per_second,
        ))
    }
}
impl AudioSourceTrait for AudioSource {
    fn sample_rate_hz(&self) -> usize {
        match self {
            AudioSource::Recording(buffer) => buffer.sample_rate_hz(),
            AudioSource::Sine(sine) => sine.sample_rate_hz(),
        }
    }
    fn has_finished(&self) -> bool {
        match self {
            AudioSource::Recording(buffer) => buffer.has_finished(),
            AudioSource::Sine(sine) => sine.has_finished(),
        }
    }
    fn is_looping(&self) -> bool {
        match self {
            AudioSource::Recording(buffer) => buffer.is_looping(),
            AudioSource::Sine(sine) => sine.is_looping(),
        }
    }
    fn completion_ratio(&self) -> Option<f32> {
        match self {
            AudioSource::Recording(buffer) => buffer.completion_ratio(),
            AudioSource::Sine(sine) => sine.completion_ratio(),
        }
    }

    fn produce_chunk(&mut self, out_chunk: &mut AudioChunk) {
        match self {
            AudioSource::Recording(buffer) => buffer.produce_chunk(out_chunk),
            AudioSource::Sine(sine) => sine.produce_chunk(out_chunk),
        }
    }

    fn channels(&self) -> AudioChannels {
        match self {
            AudioSource::Recording(buffer) => buffer.channels(),
            AudioSource::Sine(sine) => sine.channels(),
        }
    }

    fn framecount(&self) -> Option<usize> {
        match self {
            AudioSource::Recording(buffer) => buffer.framecount(),
            AudioSource::Sine(sine) => sine.framecount(),
        }
    }

    fn playcursor_pos(&self) -> Option<usize> {
        match self {
            AudioSource::Recording(buffer) => buffer.playcursor_pos(),
            AudioSource::Sine(sine) => sine.playcursor_pos(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Adapters

#[derive(Clone)]
struct VolumeAdapter {
    pub volume_current: f32,
    pub volume_target: f32,
}
impl VolumeAdapter {
    fn new(volume: f32) -> VolumeAdapter {
        VolumeAdapter {
            volume_current: volume,
            volume_target: volume,
        }
    }
    fn set_volume(&mut self, volume: f32) {
        self.volume_target = volume;
    }
    fn process_chunk(&mut self, chunk: &mut AudioChunk) {
        chunk.multipliy_volume_ramp(self.volume_current, self.volume_target);
        self.volume_current = self.volume_target;
    }
}

#[derive(Clone)]
struct MonoToStereoAdapter {
    pub pan_current: f32,
    pub pan_target: f32,
}
impl MonoToStereoAdapter {
    fn new(pan: f32) -> MonoToStereoAdapter {
        MonoToStereoAdapter {
            pan_current: pan,
            pan_target: pan,
        }
    }
    fn set_pan(&mut self, pan: f32) {
        self.pan_target = pan;
    }
    fn process_chunk(&mut self, input: &AudioChunk, output: &mut AudioChunk) {
        AudioChunk::convert_mono_to_stereo_ramp(input, output, self.pan_current, self.pan_target);
        self.pan_current = self.pan_target;
    }
}

#[derive(Clone)]
struct Resampler {
    frame_current: (AudioSample, AudioSample),
    frame_next: (AudioSample, AudioSample),
    frame_time_percent: f32,

    pub internal_chunk: AudioChunk,
    pub internal_chunk_readpos: usize,
}

impl Resampler {
    pub fn new_mono() -> Resampler {
        let internal_chunk = AudioChunk::new_mono();
        let internal_chunk_readpos = internal_chunk.len();
        Resampler {
            frame_current: (0.0, 0.0),
            frame_next: (0.0, 0.0),
            frame_time_percent: 0.0,

            internal_chunk,
            internal_chunk_readpos,
        }
    }

    pub fn new_stereo() -> Resampler {
        let internal_chunk = AudioChunk::new_stereo();
        let internal_chunk_readpos = internal_chunk.len();
        Resampler {
            frame_current: (0.0, 0.0),
            frame_next: (0.0, 0.0),
            frame_time_percent: 0.0,

            internal_chunk,
            internal_chunk_readpos,
        }
    }

    /// Returns true if resampler has no more frames to output with the given source
    pub fn produce_chunk_using_source(
        &mut self,
        source: &mut AudioSource,
        output: &mut AudioChunk,
        out_write_offset: usize,
        playback_speed_factor: f32,
    ) -> bool {
        let mut resampler_write_offset = out_write_offset;
        loop {
            if self.internal_buffer_depleted() {
                if source.has_finished() {
                    self.produce_tail(output, resampler_write_offset, playback_speed_factor);
                    break;
                } else {
                    source.produce_chunk(&mut self.internal_chunk);
                    self.internal_chunk_readpos = 0;
                }
            }
            resampler_write_offset +=
                self.produce_frames(output, resampler_write_offset, playback_speed_factor);

            if resampler_write_offset >= output.len() {
                break;
            }
        }
        source.has_finished() && self.has_finished()
    }

    pub fn calculate_sample_rate_conversion_factor(
        input_sample_rate_hz: usize,
        output_sample_rate_hz: usize,
    ) -> f32 {
        input_sample_rate_hz as f32 / output_sample_rate_hz as f32
    }

    pub fn calculate_playback_speed_ratio(
        input_sample_rate_hz: usize,
        output_sample_rate_hz: usize,
        playback_speed_factor: f32,
    ) -> f32 {
        playback_speed_factor
            * Resampler::calculate_sample_rate_conversion_factor(
                input_sample_rate_hz,
                output_sample_rate_hz,
            )
    }

    pub fn playcursor_pos(&self) -> usize {
        self.internal_chunk_readpos
    }

    pub fn framecount(&self) -> usize {
        self.internal_chunk.len() + 2
    }

    pub fn has_finished(&self) -> bool {
        // NOTE: Plus two for our internal `frame_current` and `frame_next` frames
        self.internal_chunk_readpos >= self.framecount()
    }

    pub fn internal_buffer_depleted(&self) -> bool {
        self.internal_chunk_readpos >= self.internal_chunk.len()
    }

    pub fn assign_input_chunk(&mut self, input: &AudioChunk) {
        assert!(
            self.internal_buffer_depleted(),
            "Previous input buffer is not empty"
        );
        assert!(
            self.internal_chunk.volume == 1.0 || self.internal_chunk.volume == 0.0,
            "Resampler does not support sources that produce buffers with volume between 0 and 1"
        );
        assert!(input.channels == self.internal_chunk.channels);

        self.internal_chunk.copy_from_chunk_complete(input);
        self.internal_chunk_readpos = 0;
    }

    // Writes out remaining samples in the internal buffer and after that fills the rest of the
    // output with silence
    pub fn produce_tail(
        &mut self,
        output: &mut AudioChunk,
        out_write_offset: usize,
        playback_speed_factor: f32,
    ) {
        assert!(output.channels == self.internal_chunk.channels);
        assert!(out_write_offset < output.len());
        assert!(playback_speed_factor > EPSILON);
        assert!(
            self.internal_chunk.volume == 1.0 || self.internal_chunk.volume == 0.0,
            "Resampler does not support sources that produce buffers with volume between 0 and 1"
        );

        match output.channels {
            AudioChannels::Mono => {
                for out_frames in &mut output.get_mono_samples_mut()[out_write_offset..] {
                    self.frame_time_percent += playback_speed_factor;

                    while self.frame_time_percent >= 1.0 {
                        self.frame_current = self.frame_next;
                        self.frame_next = (
                            *self
                                .internal_chunk
                                .get_mono_samples()
                                .get(self.internal_chunk_readpos)
                                .unwrap_or(&0.0),
                            0.0,
                        );
                        self.internal_chunk_readpos += 1;
                        self.frame_time_percent -= 1.0;
                    }

                    let interpolated_sample_value = lerp(
                        self.frame_current.0,
                        self.frame_next.0,
                        self.frame_time_percent,
                    );

                    *out_frames = interpolated_sample_value;
                }
            }
            AudioChannels::Stereo => {
                let (out_samples_left, out_samples_right) = output.get_stereo_samples_mut();
                let out_samples_left = &mut out_samples_left[out_write_offset..];
                let out_samples_right = &mut out_samples_right[out_write_offset..];

                for (out_left, out_right) in out_samples_left
                    .iter_mut()
                    .zip(out_samples_right.iter_mut())
                {
                    self.frame_time_percent += playback_speed_factor;

                    while self.frame_time_percent >= 1.0 {
                        self.frame_current = self.frame_next;
                        self.frame_next = (
                            *self
                                .internal_chunk
                                .get_stereo_samples_left()
                                .get(self.internal_chunk_readpos)
                                .unwrap_or(&0.0),
                            *self
                                .internal_chunk
                                .get_stereo_samples_right()
                                .get(self.internal_chunk_readpos)
                                .unwrap_or(&0.0),
                        );
                        self.internal_chunk_readpos += 1;
                        self.frame_time_percent -= 1.0;
                    }

                    let interpolated_sample_left = lerp(
                        self.frame_current.0,
                        self.frame_next.0,
                        self.frame_time_percent,
                    );
                    let interpolated_sample_right = lerp(
                        self.frame_current.1,
                        self.frame_next.1,
                        self.frame_time_percent,
                    );

                    *out_left = interpolated_sample_left;
                    *out_right = interpolated_sample_right;
                }
            }
        }
    }

    // Returns the number of actually written frames. It can be lower than
    // `output.len() - out_write_offset` if the internal buffer was depleted while processing
    pub fn produce_frames(
        &mut self,
        output: &mut AudioChunk,
        out_write_offset: usize,
        playback_speed_factor: f32,
    ) -> usize {
        assert!(output.channels == self.internal_chunk.channels);
        assert!(out_write_offset < output.len());
        assert!(playback_speed_factor > EPSILON);
        assert!(
            self.internal_chunk.volume == 1.0 || self.internal_chunk.volume == 0.0,
            "Resampler does not support sources that produce buffers with volume between 0 and 1"
        );

        if self.internal_buffer_depleted() {
            return 0;
        }

        let mut num_frames_written = 0;
        match output.channels {
            AudioChannels::Mono => {
                for out_frames in &mut output.get_mono_samples_mut()[out_write_offset..] {
                    if self.internal_buffer_depleted() {
                        return num_frames_written;
                    }

                    self.frame_time_percent += playback_speed_factor;

                    while self.frame_time_percent >= 1.0 {
                        if self.internal_buffer_depleted() {
                            return num_frames_written;
                        }
                        self.frame_current = self.frame_next;
                        self.frame_next = unsafe {
                            (
                                *self
                                    .internal_chunk
                                    .get_mono_samples()
                                    .get_unchecked(self.internal_chunk_readpos),
                                0.0,
                            )
                        };
                        self.internal_chunk_readpos += 1;
                        self.frame_time_percent -= 1.0;
                    }

                    let interpolated_sample_value = lerp(
                        self.frame_current.0,
                        self.frame_next.0,
                        self.frame_time_percent,
                    );

                    *out_frames = interpolated_sample_value;
                    num_frames_written += 1;
                }
            }
            AudioChannels::Stereo => {
                let (out_samples_left, out_samples_right) = output.get_stereo_samples_mut();
                let out_samples_left = &mut out_samples_left[out_write_offset..];
                let out_samples_right = &mut out_samples_right[out_write_offset..];

                for (out_left, out_right) in out_samples_left
                    .iter_mut()
                    .zip(out_samples_right.iter_mut())
                {
                    if self.internal_buffer_depleted() {
                        return num_frames_written;
                    }

                    self.frame_time_percent += playback_speed_factor;

                    while self.frame_time_percent >= 1.0 {
                        if self.internal_buffer_depleted() {
                            return num_frames_written;
                        }
                        self.frame_current = self.frame_next;
                        self.frame_next = unsafe {
                            (
                                *self
                                    .internal_chunk
                                    .get_stereo_samples_left()
                                    .get_unchecked(self.internal_chunk_readpos),
                                *self
                                    .internal_chunk
                                    .get_stereo_samples_right()
                                    .get_unchecked(self.internal_chunk_readpos),
                            )
                        };
                        self.internal_chunk_readpos += 1;
                        self.frame_time_percent -= 1.0;
                    }

                    let interpolated_sample_left = lerp(
                        self.frame_current.0,
                        self.frame_next.0,
                        self.frame_time_percent,
                    );
                    let interpolated_sample_right = lerp(
                        self.frame_current.1,
                        self.frame_next.1,
                        self.frame_time_percent,
                    );

                    *out_left = interpolated_sample_left;
                    *out_right = interpolated_sample_right;

                    num_frames_written += 1;
                }
            }
        }
        num_frames_written
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostreams Basic

#[derive(Clone, Copy)]
struct AudioRenderParams {
    /// The samplerate of our audio recordings
    pub internal_sample_rate_hz: usize,
    pub global_playback_speed: f32,
    pub global_volume: f32,

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

    source: AudioSource,

    frames_left_till_start: usize,
    has_finished: bool,

    playback_speed_resampler: Resampler,
    /// Must be > 0
    playback_speed_base: f32,

    volume_adapter: VolumeAdapter,
    volume_base: f32,

    pan_adapter: MonoToStereoAdapter,
    /// Only used when we don't have spatial params
    pan_base: f32,

    chunk_internal: AudioChunk,
    chunk_output: AudioChunk,

    spatial_params: Option<SpatialParams>,
}

impl AudioStream {
    pub fn new(
        name: String,
        source: AudioSource,
        delay_framecount: usize,
        playback_speed_percent: f32,
        volume: f32,
        pan: f32,
        spatial_params: Option<SpatialParams>,
    ) -> AudioStream {
        let (chunk_internal, playback_speed_resampler) = match source.channels() {
            AudioChannels::Mono => (AudioChunk::new_mono(), Resampler::new_mono()),
            AudioChannels::Stereo => (AudioChunk::new_stereo(), Resampler::new_stereo()),
        };
        AudioStream {
            name,
            source,

            frames_left_till_start: delay_framecount,
            has_finished: false,

            playback_speed_resampler,
            playback_speed_base: playback_speed_percent,

            volume_adapter: VolumeAdapter::new(volume),
            volume_base: volume,

            pan_adapter: MonoToStereoAdapter::new(0.0),
            pan_base: pan,

            chunk_internal,
            chunk_output: AudioChunk::new_stereo(),

            spatial_params,
        }
    }

    pub fn produce_output_chunk(&mut self, output_params: AudioRenderParams) {
        // Reset volume for output chunks
        self.chunk_internal.volume = 1.0;
        self.chunk_output.volume = 1.0;

        // Fast path - we are finished
        if self.has_finished {
            self.chunk_internal.volume = 0.0;
            self.chunk_output.volume = 0.0;
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
            self.chunk_internal.volume = 0.0;
            self.chunk_output.volume = 0.0;
            return;
        }

        // Write remaining delay frames as silence
        self.chunk_internal
            .fill_silence_offset_framecount(0, silence_framecount);

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

        // Resampler stage
        self.has_finished = self.playback_speed_resampler.produce_chunk_using_source(
            &mut self.source,
            &mut self.chunk_internal,
            silence_framecount,
            final_playback_speed_factor,
        );

        // Volume stage
        self.volume_adapter.set_volume(final_volume);
        self.volume_adapter.process_chunk(&mut self.chunk_internal);

        // Mono -> stereo stage
        match self.chunk_internal.channels {
            AudioChannels::Mono => {
                self.pan_adapter.set_pan(final_pan);
                self.pan_adapter
                    .process_chunk(&self.chunk_internal, &mut self.chunk_output);
            }
            AudioChannels::Stereo => {
                // No conversion needed - just copy to output
                self.chunk_output
                    .copy_from_chunk_complete(&self.chunk_internal);
            }
        }
    }

    pub fn get_output_chunk(&self) -> &AudioChunk {
        &self.chunk_output
    }

    pub fn get_output_chunk_mut(&mut self) -> &mut AudioChunk {
        &mut self.chunk_output
    }

    pub fn has_started(&self) -> bool {
        self.frames_left_till_start == 0
    }

    pub fn has_finished(&self) -> bool {
        self.has_finished
    }

    pub fn is_looping(&self) -> bool {
        self.source.is_looping()
    }

    pub fn completion_ratio(&self) -> Option<f32> {
        if self.has_started() {
            if self.source.framecount().is_none() || self.source.playcursor_pos().is_none() {
                return None;
            }

            let source_framecount = self.source.framecount().unwrap();
            let source_cursor_pos = self.source.playcursor_pos().unwrap();

            let resampler_framecount = self.playback_speed_resampler.framecount();
            let resampler_cursor_pos = self.playback_speed_resampler.playcursor_pos();

            let framecount = source_framecount + resampler_framecount;
            let cursor_pos = source_cursor_pos + resampler_cursor_pos;

            Some(clampf(cursor_pos as f32 / framecount as f32, 0.0, 1.0))
        } else {
            None
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume_base = volume;
    }

    pub fn set_pan(&mut self, pan: f32) {
        self.pan_base = pan;
    }

    pub fn set_playback_speed(&mut self, playback_speed_percent: f32) {
        self.playback_speed_base = playback_speed_percent;
    }

    pub fn set_spatial_pos(&mut self, pos: Vec2) {
        if let Some(spatial) = &mut self.spatial_params {
            spatial.pos = pos;
        } else {
            panic!(
                "Stream '{}' has no spatial component to set position",
                self.name
            );
        }
    }

    pub fn set_spatial_vel(&mut self, vel: Vec2) {
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
    pub fn value_for_distance(
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

#[derive(Copy, Clone)]
struct SpatialParams {
    pub pos: Vec2,
    pub vel: Vec2,
    pub doppler_effect_strength: f32,
    pub falloff_type: AudioFalloffType,
    pub falloff_distance_start: f32,
    pub falloff_distance_end: f32,
}

impl SpatialParams {
    pub fn new(
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

    pub fn calculate_volume_pan_playback_speed(
        &self,
        output_params: &AudioRenderParams,
    ) -> (f32, f32, f32) {
        let volume_factor = SpatialParams::calculate_spatial_volume_factor(
            self.pos,
            output_params.listener_pos,
            self.falloff_type,
            self.falloff_distance_start,
            self.falloff_distance_end,
        );
        let pan = SpatialParams::calculate_spatial_pan(
            self.pos,
            output_params.listener_pos,
            output_params.distance_for_max_pan,
        );
        let playback_speed_factor = SpatialParams::calculate_spatial_playback_speed_factor(
            self.pos,
            self.vel,
            output_params.listener_pos,
            output_params.listener_vel,
            self.doppler_effect_strength,
            output_params.doppler_effect_medium_velocity_abs_max,
        );
        (volume_factor, pan, playback_speed_factor)
    }

    fn calculate_spatial_pan(
        source_pos: Vec2,
        listener_pos: Vec2,
        distance_for_max_pan: f32,
    ) -> f32 {
        clampf(
            (source_pos.x - listener_pos.x) / distance_for_max_pan,
            -1.0,
            1.0,
        )
    }

    fn calculate_spatial_playback_speed_factor(
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

    fn calculate_spatial_volume_factor(
        source_pos: Vec2,
        listener_pos: Vec2,
        falloff_type: AudioFalloffType,
        falloff_distance_start: f32,
        falloff_distance_end: f32,
    ) -> f32 {
        let distance = Vec2::distance(source_pos, listener_pos);
        falloff_type.value_for_distance(distance, falloff_distance_start, falloff_distance_end)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Audiostate

/// NOTE: This can never be zero for a valid stream
pub type AudioStreamId = u64;

#[derive(Clone)]
pub struct Audiostate {
    next_frame_index_to_output: AudioFrameIndex,

    audio_time: f64,
    audio_time_smoothed: f64,

    render_params: AudioRenderParams,

    /// This can never be zero when used with `get_next_stream_id` method
    next_stream_id: AudioStreamId,

    streams: HashMap<AudioStreamId, AudioStream>,
    streams_to_delete_after_finish: HashSet<AudioStreamId>,

    audio_recordings: HashMap<String, Rc<RefCell<AudioRecording>>>,

    resampler: Resampler,
    internal_chunk: AudioChunk,
}

impl Audiostate {
    pub fn new(
        internal_sample_rate_hz: usize,
        distance_for_max_pan: f32,
        doppler_effect_medium_velocity_abs_max: f32,
    ) -> Audiostate {
        Audiostate {
            next_frame_index_to_output: 0,
            audio_time: 0.0,
            audio_time_smoothed: 0.0,

            render_params: AudioRenderParams {
                internal_sample_rate_hz,
                global_playback_speed: 1.0,
                global_volume: 1.0,
                listener_pos: Vec2::zero(),
                listener_vel: Vec2::zero(),
                doppler_effect_medium_velocity_abs_max,
                distance_for_max_pan,
            },

            next_stream_id: 0,
            streams: HashMap::new(),
            streams_to_delete_after_finish: HashSet::new(),

            audio_recordings: HashMap::new(),

            resampler: Resampler::new_stereo(),
            internal_chunk: AudioChunk::new_stereo(),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.next_frame_index_to_output = 0;

        self.render_params.global_playback_speed = 1.0;
        self.render_params.listener_pos = Vec2::zero();
        self.render_params.listener_vel = Vec2::zero();

        self.next_stream_id = 0;
        self.streams = HashMap::new();
        self.streams_to_delete_after_finish = HashSet::new();
    }

    #[inline]
    pub fn add_audio_recordings(&mut self, mut audio_recordings: HashMap<String, AudioRecording>) {
        for (name, recording) in audio_recordings.drain() {
            assert!(
                recording.sample_rate_hz == self.render_params.internal_sample_rate_hz,
                "Resource '{}' has sample_rate {}Hz - expected {}Hz",
                name,
                recording.sample_rate_hz,
                self.render_params.internal_sample_rate_hz
            );
            self.audio_recordings
                .insert(name, Rc::new(RefCell::new(recording)));
        }
    }

    #[inline]
    pub fn current_time_seconds(&self) -> f64 {
        self.audio_time
    }

    pub fn current_time_seconds_smoothed(&self) -> f64 {
        self.audio_time_smoothed
    }

    /// IMPORTANT: This needs to be called exactly once per frame to have correct time reporting
    pub fn update_deltatime(&mut self, deltatime: f32) {
        self.audio_time_smoothed += deltatime as f64;
    }

    #[inline]
    pub fn set_global_volume(&mut self, volume: f32) {
        self.render_params.global_volume = volume;
    }

    #[inline]
    pub fn set_global_playback_speed_factor(&mut self, global_playback_speed: f32) {
        self.render_params.global_playback_speed = global_playback_speed;
    }

    #[inline]
    pub fn set_listener_pos(&mut self, listener_pos: Vec2) {
        self.render_params.listener_pos = listener_pos;
    }

    #[inline]
    pub fn set_listener_vel(&mut self, listener_vel: Vec2) {
        self.render_params.listener_vel = listener_vel;
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
        let id = self.create_next_stream_id();
        let start_delay_framecount = self.start_time_to_delay_framecount(schedule_time_seconds);
        let source = if recording_name == "sine" {
            AudioSource::new_from_sine(440.0, self.render_params.internal_sample_rate_hz)
        } else {
            let buffer = self
                .audio_recordings
                .get(recording_name)
                .unwrap_or_else(|| panic!("Recording '{}' not found", recording_name));
            AudioSource::new_from_recording(buffer.clone(), play_looped)
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
        let id = self.create_next_stream_id();
        let start_delay_framecount = self.start_time_to_delay_framecount(schedule_time_seconds);
        let stream = {
            let initial_pan = SpatialParams::calculate_spatial_pan(
                pos,
                self.render_params.listener_pos,
                self.render_params.distance_for_max_pan,
            );
            let source = {
                let buffer = self
                    .audio_recordings
                    .get(recording_name)
                    .unwrap_or_else(|| panic!("Recording '{}' not found", recording_name));
                AudioSource::new_from_recording(buffer.clone(), play_looped)
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
    pub fn stream_set_spatial_pos(&mut self, stream_id: AudioStreamId, pos: Vec2) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_spatial_pos(pos);
    }

    #[inline]
    pub fn stream_set_spatial_vel(&mut self, stream_id: AudioStreamId, vel: Vec2) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_spatial_vel(vel);
    }

    #[inline]
    pub fn stream_set_volume(&mut self, stream_id: AudioStreamId, volume: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_volume(volume);
    }

    #[inline]
    pub fn stream_set_pan(&mut self, stream_id: AudioStreamId, pan: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_pan(pan);
    }

    #[inline]
    pub fn stream_set_playback_speed(&mut self, stream_id: AudioStreamId, playback_speed: f32) {
        let stream = self.get_stream_mut(stream_id);
        stream.set_playback_speed(playback_speed);
    }

    /// It is assumed that `out_chunk` is filled with silence
    #[inline]
    pub fn render_audio_chunk(&mut self, out_chunk: &mut AudioChunk, output_sample_rate_hz: usize) {
        let playback_speed_factor = Resampler::calculate_playback_speed_ratio(
            self.render_params.internal_sample_rate_hz,
            output_sample_rate_hz,
            self.render_params.global_playback_speed,
        );
        let mut resampler_write_offset = 0;
        loop {
            if self.resampler.internal_buffer_depleted() {
                self.render_audio_chunk_internal();
                self.resampler.assign_input_chunk(&self.internal_chunk);
            }
            resampler_write_offset += self.resampler.produce_frames(
                out_chunk,
                resampler_write_offset,
                playback_speed_factor,
            );

            if resampler_write_offset >= out_chunk.len() {
                break;
            }
        }
    }

    #[inline]
    pub fn render_audio_chunk_internal(&mut self) {
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
        self.internal_chunk.reset();
        for stream in self.streams.values_mut() {
            stream.produce_output_chunk(self.render_params);
            let mut rendered_chunk = stream.get_output_chunk_mut();
            rendered_chunk.volume *= self.render_params.global_volume;
            self.internal_chunk.add_from_chunk(rendered_chunk);
        }

        // Update internal timers
        self.next_frame_index_to_output += AUDIO_CHUNKSIZE_IN_FRAMES as AudioFrameIndex;
        let new_audio_time = audio_frames_to_seconds(
            self.next_frame_index_to_output,
            self.render_params.internal_sample_rate_hz,
        );
        if self.audio_time != new_audio_time {
            self.audio_time = new_audio_time;
            self.audio_time_smoothed = (self.audio_time_smoothed + new_audio_time) / 2.0;
        }
    }

    fn start_time_to_delay_framecount(&self, schedule_time_seconds: f64) -> usize {
        let start_frame_index = audio_seconds_to_frames(
            schedule_time_seconds,
            self.render_params.internal_sample_rate_hz,
        )
        .round() as AudioFrameIndex;

        (start_frame_index - self.next_frame_index_to_output).max(0) as usize
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
    fn create_next_stream_id(&mut self) -> AudioStreamId {
        self.next_stream_id += 1;
        self.next_stream_id
    }
}
