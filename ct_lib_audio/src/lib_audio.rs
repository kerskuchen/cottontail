pub mod audio;
pub use audio::*;

use ct_lib_math as math;

#[inline]
fn convert_u8_sample_to_f32(sample: u8) -> f32 {
    (sample as f32 - std::i8::MAX as f32) / -(std::i8::MIN as f32)
}
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
pub fn decode_wav_from_bytes(wav_data: &[u8]) -> Result<(usize, Vec<AudioSample>), String> {
    let (header, data) = wav::read(&mut std::io::Cursor::new(wav_data))
        .map_err(|error| format!("Could not decode wav audio data: {}", error))?;

    if header.channel_count != 1 {
        return Err("Stereo wav data not supported".to_owned());
    }
    let sample_rate_hz = header.sampling_rate as usize;
    let samples: Vec<AudioSample> = match data {
        wav::BitDepth::Eight(samples_u8) => samples_u8
            .into_iter()
            .map(convert_u8_sample_to_f32)
            .collect(),
        wav::BitDepth::Sixteen(samples_i16) => samples_i16
            .into_iter()
            .map(convert_i16_sample_to_f32)
            .collect(),
        wav::BitDepth::TwentyFour(samples_i32) => samples_i32
            .into_iter()
            .map(convert_i32_sample_to_f32)
            .collect(),
        wav::BitDepth::Empty => return Err("Wav data is empty".to_owned()),
    };
    Ok((sample_rate_hz, samples))
}
