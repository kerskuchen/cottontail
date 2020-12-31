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

/// IMPORTANT: This Assumes mono
/// Returns samplerate and a vector of samples
pub fn decode_wav_from_bytes(wav_data: &[u8]) -> Result<(usize, Vec<AudioFrameMono>), String> {
    let reader = hound::WavReader::new(std::io::Cursor::new(wav_data))
        .map_err(|error| format!("Could not decode wav audio data: {}", error))?;
    if reader.len() == 0 {
        return Err("Wav data is empty".to_owned());
    }
    let header = reader.spec();
    if header.channels != 1 {
        return Err("Stereo wav data not supported".to_owned());
    }
    let sample_rate_hz = header.sample_rate as usize;
    let samples = {
        let samples: Result<Vec<AudioFrameMono>, _> = match header.sample_format {
            hound::SampleFormat::Float => reader.into_samples::<AudioFrameMono>().collect(),
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

    Ok((sample_rate_hz, samples))
}

/// Returns samplerate, channelcount
pub fn decode_ogg_samplerate_channelcount(ogg_data: &[u8]) -> Result<(usize, usize), String> {
    let reader = OggStreamReader::new(std::io::Cursor::new(ogg_data))
        .map_err(|error| format!("Could not decode ogg audio data: {}", error))?;
    let sample_rate_hz = reader.ident_hdr.audio_sample_rate as usize;
    let channel_count = reader.ident_hdr.audio_channels as usize;

    Ok((sample_rate_hz, channel_count))
}

/// Returns and a vector of interleaved samples
pub fn decode_ogg_frames<FrameType: AudioFrame>(ogg_data: &[u8]) -> Result<Vec<FrameType>, String> {
    let mut reader = OggStreamReader::new(std::io::Cursor::new(ogg_data))
        .map_err(|error| format!("Could not decode ogg audio data: {}", error))?;
    if reader.ident_hdr.audio_channels as usize != FrameType::channel_count() {
        return Err(format!(
            "Expected ogg stream with {} channels - got {} channels",
            FrameType::channel_count(),
            reader.ident_hdr.audio_channels
        ));
    }

    let mut result_frames = Vec::new();
    let mut packet_index = 0;
    while let Some(decoded_samples) = reader
        .read_dec_packet_generic::<Vec<Vec<f32>>>()
        .map_err(|error| format!("Could not decode ogg packet {}: {}", packet_index, error))?
    {
        packet_index += 1;
        match reader.ident_hdr.audio_channels {
            1 => {
                for &sample in decoded_samples[0].iter() {
                    result_frames.push(FrameType::new_mono(sample));
                }
            }
            2 => {
                for (&left, &right) in decoded_samples[0].iter().zip(decoded_samples[1].iter()) {
                    result_frames.push(FrameType::new_stereo(left, right));
                }
            }
            _ => {
                unreachable!()
            }
        }
    }

    Ok(result_frames)
}

/// Returns samplerate and a vector of samples
#[cfg(not(target_arch = "wasm32"))]
pub fn write_audio_samples_to_wav_file(
    filepath: &str,
    frames: &[AudioFrameStereo],
    samplerate: usize,
) {
    let header = hound::WavSpec {
        channels: 2,
        sample_rate: samplerate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(filepath, header).expect(&format!(
        "Could not open '{}' for writing wav data",
        filepath
    ));
    for frame in frames {
        writer.write_sample(frame.left).unwrap();
        writer.write_sample(frame.right).unwrap();
    }
    writer
        .finalize()
        .expect(&format!("Could not finalize wav file '{}'", filepath));
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
