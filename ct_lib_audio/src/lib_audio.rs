pub mod audio;
pub use audio::*;

use ct_lib_math as math;
use lewton::inside_ogg::OggStreamReader;

#[inline]
fn convert_i16_sample_to_f32(sample: i16) -> f32 {
    sample as f32 / -(std::i16::MIN as f32)
}
#[inline]
fn convert_i32_sample_to_f32(sample: i32) -> f32 {
    sample as f32 / -(std::i32::MIN as f32)
}

/// Returns samplerate and audio chunk
pub fn decode_wav_from_bytes(wav_data: &[u8]) -> Result<(usize, AudioChunk), String> {
    let reader = hound::WavReader::new(std::io::Cursor::new(wav_data))
        .map_err(|error| format!("Could not decode wav audio data: {}", error))?;
    if reader.len() == 0 {
        return Err("Wav data is empty".to_owned());
    }
    let header = reader.spec();
    let sample_rate_hz = header.sample_rate as usize;
    let samples = {
        let samples: Result<Vec<AudioSample>, _> = match header.sample_format {
            hound::SampleFormat::Float => reader.into_samples::<AudioSample>().collect(),
            hound::SampleFormat::Int => match header.bits_per_sample {
                16 => reader
                    .into_samples::<i16>()
                    .map(|sample| sample.map(convert_i16_sample_to_f32))
                    .collect(),
                32 => reader
                    .into_samples::<i32>()
                    .map(|sample| sample.map(convert_i32_sample_to_f32))
                    .collect(),
                _ => {
                    return Err(format!(
                        "{} bit PCM wav data not supported",
                        header.bits_per_sample
                    ))
                }
            },
        };
        samples.map_err(|error| format!("Cannot decode samples: {}", error))?
    };

    let chunk = match header.channels {
        1 => AudioChunk::new_mono_from_frames(samples),
        2 => {
            let mut samples_left = Vec::with_capacity(samples.len() / 2);
            let mut samples_right = Vec::with_capacity(samples.len() / 2);
            samples.chunks_exact(2).into_iter().for_each(|frame| {
                let left = frame[0];
                let right = frame[1];
                samples_left.push(left);
                samples_right.push(right);
            });
            AudioChunk::new_stereo_from_frames(samples_left, samples_right)
        }
        _ => {
            return Err(format!(
                "Only mono and stereo wav data supported - got {} channels",
                header.channels
            ));
        }
    };

    Ok((sample_rate_hz, chunk))
}

/// Returns samplerate and audio chunk
pub fn decode_ogg_data_from_bytes(ogg_data: &[u8]) -> Result<(usize, AudioChunk), String> {
    let mut reader = OggStreamReader::new(std::io::Cursor::new(ogg_data))
        .map_err(|error| format!("Could not decode ogg audio data: {}", error))?;
    let sample_rate_hz = reader.ident_hdr.audio_sample_rate as usize;
    let channel_count = reader.ident_hdr.audio_channels as usize;
    match channel_count {
        1 | 2 => {}
        _ => {
            panic!("Unsupported channel count {} in ogg data", channel_count,);
        }
    }

    let mut frames_left = Vec::new();
    let mut frames_right = Vec::new();
    let mut packet_index = 0;
    while let Some(decoded_samples) = reader
        .read_dec_packet_generic::<Vec<Vec<f32>>>()
        .map_err(|error| format!("Could not decode ogg packet {}: {}", packet_index, error))?
    {
        packet_index += 1;
        match channel_count {
            1 => {
                for &sample in decoded_samples[0].iter() {
                    frames_left.push(sample);
                }
            }
            2 => {
                for (&left, &right) in decoded_samples[0].iter().zip(decoded_samples[1].iter()) {
                    frames_left.push(left);
                    frames_right.push(right);
                }
            }
            _ => unreachable!(),
        }
    }

    let chunk = match channel_count {
        1 => AudioChunk::new_mono_from_frames(frames_left),
        2 => AudioChunk::new_stereo_from_frames(frames_left, frames_right),
        _ => unreachable!(),
    };

    Ok((sample_rate_hz, chunk))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn decode_audio_file(filepath: &str) -> Result<(usize, AudioChunk), String> {
    let data = ct_lib_core::read_file_whole(filepath)?;
    if filepath.ends_with(".wav") {
        decode_wav_from_bytes(&data)
    } else if filepath.ends_with(".ogg") {
        decode_ogg_data_from_bytes(&data)
    } else {
        Err(format!(
            "File '{}' has unknown format (only .wav and .ogg supported)",
            filepath
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_audio_samples_to_wav_file(filepath: &str, chunk: &AudioChunk, samplerate: usize) {
    let header = hound::WavSpec {
        channels: chunk.channelcount() as u16,
        sample_rate: samplerate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(filepath, header).unwrap_or_else(|error| {
        panic!(
            "Could not open '{}' for writing wav data: {}",
            filepath, error
        )
    });
    match chunk.channels {
        AudioChannels::Mono => {
            for sample in chunk.get_mono_samples() {
                writer.write_sample(*sample).unwrap();
            }
        }
        AudioChannels::Stereo => {
            let (samples_left, samples_right) = chunk.get_stereo_samples();
            for (left, right) in samples_left.iter().zip(samples_right.iter()) {
                writer.write_sample(*left).unwrap();
                writer.write_sample(*right).unwrap();
            }
        }
    }
    writer
        .finalize()
        .unwrap_or_else(|error| panic!("Could not finalize wav file '{}': {}", filepath, error));
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Music

#[derive(Debug, Clone, Copy)]
pub enum MusicalInterval {
    Measure {
        beats_per_minute: usize,
        beats_per_measure: usize,
    },
    Beat {
        beats_per_minute: usize,
    },
    HalfBeat {
        beats_per_minute: usize,
    },
    QuarterBeat {
        beats_per_minute: usize,
    },
}
impl MusicalInterval {
    #[inline]
    pub fn length_seconds(&self) -> f64 {
        match self {
            MusicalInterval::Measure {
                beats_per_minute,
                beats_per_measure,
            } => music_measure_length_in_seconds(*beats_per_measure, *beats_per_minute),
            MusicalInterval::Beat { beats_per_minute } => {
                music_beat_length_in_seconds(*beats_per_minute)
            }
            MusicalInterval::HalfBeat {
                ref beats_per_minute,
            } => music_beat_length_in_seconds(*beats_per_minute) / 2.0,
            MusicalInterval::QuarterBeat { beats_per_minute } => {
                music_beat_length_in_seconds(*beats_per_minute) / 4.0
            }
        }
    }
}

#[inline]
pub fn music_beat_length_in_seconds(beats_per_minute: usize) -> f64 {
    60.0 / (beats_per_minute as f64)
}

#[inline]
pub fn music_measure_length_in_seconds(beats_per_measure: usize, beats_per_minute: usize) -> f64 {
    beats_per_measure as f64 * music_beat_length_in_seconds(beats_per_minute)
}

#[inline]
pub fn music_get_next_point_in_time(
    current_time_seconds: f64,
    interval_type: MusicalInterval,
) -> f64 {
    let segment_length_seconds = interval_type.length_seconds();
    f64::ceil(current_time_seconds / segment_length_seconds) * segment_length_seconds
}
