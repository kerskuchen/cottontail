pub mod audio;
pub mod bitmap;
pub mod color;
pub mod draw;
pub mod draw_common;
pub mod font;
pub mod grid;
pub mod random;
pub mod sprite;
pub mod system;

#[path = "game/mod_game.rs"]
pub mod game;
#[path = "math/mod_math.rs"]
pub mod math;

pub use bincode;
pub use indexmap;
pub use log;
pub use png;
pub use serde;
pub use serde_derive;
pub use serde_json;

use std::collections::HashSet;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debugging and performance

/// This is pretty similar to the dbg! macro only that dformat! returns a string
#[macro_export]
macro_rules! dformat {
    ($x:expr) => {
        format!("{} = {:?}", stringify!($x), $x)
    };
}

pub struct TimerScoped {
    use_logger: bool,
    log_message: String,
    creation_time: std::time::Instant,
}

impl Drop for TimerScoped {
    fn drop(&mut self) {
        let duration_since_creation = std::time::Instant::now()
            .duration_since(self.creation_time)
            .as_secs_f32();
        if self.use_logger {
            log::debug!(
                "{}: {:.3}ms",
                self.log_message,
                duration_since_creation * 1000.0
            );
        } else {
            println!(
                "{}: {:.3}ms",
                self.log_message,
                duration_since_creation * 1000.0
            );
        }
    }
}

impl TimerScoped {
    pub fn new_scoped(output_text: &str, use_logger: bool) -> TimerScoped {
        TimerScoped {
            use_logger,
            log_message: output_text.to_owned(),
            creation_time: std::time::Instant::now(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Logger

pub fn init_logging(logfile_path: &str, loglevel: log::LevelFilter) -> Result<(), String> {
    let logfile = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(logfile_path)
        .map_err(|error| format!("Could not create logfile at '{}' : {}", logfile_path, error))?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}::{}: {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(loglevel)
        .level_for("gfx_backend_dx11", log::LevelFilter::Warn)
        .level_for("gfx_backend_vulkan", log::LevelFilter::Warn)
        .level_for("wgpu_native", log::LevelFilter::Warn)
        .level_for("rusty_xinput", log::LevelFilter::Info)
        .level_for("gilrs::gamepad", log::LevelFilter::Info)
        .level_for("gilrs::ff::server", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(logfile)
        .apply()
        .map_err(|error| format!("Could initialize logger: {}", error))?;

    log::info!("Logger initialized");

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Convenience Serialization / Deserialization

pub fn serialize_to_file_binary<T>(data: &T, filepath: &str)
where
    T: serde::Serialize,
{
    let encoded_data = bincode::serialize(data).expect(&format!(
        "Could not encode data for serializing to binary file '{}'",
        filepath
    ));
    std::fs::write(filepath, encoded_data).expect(&format!(
        "Could not write serialized data to binary file '{}'",
        filepath
    ));
}

pub fn serialize_to_file_json<T>(data: &T, filepath: &str)
where
    T: serde::Serialize,
{
    let output_string = serde_json::to_string_pretty(data).expect(&format!(
        "Could not deserialize data to json for writing to '{}",
        filepath
    ));
    std::fs::write(filepath, output_string).expect(&format!(
        "Could write data string to json file '{}'",
        filepath
    ));
}

pub fn deserialize_from_file_binary<T>(filepath: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let file = std::fs::File::open(filepath).expect(&format!(
        "Could not open binary file '{}' for deserialization",
        filepath
    ));
    bincode::deserialize_from(&file).expect(&format!(
        "Could not deserialize from binary file '{}'",
        filepath
    ))
}

pub fn deserialize_from_file_json<T>(filepath: &str) -> T
where
    for<'de> T: serde::Deserialize<'de>,
{
    let file = std::fs::File::open(filepath).expect(&format!(
        "Could not open json file '{}' for deserialization",
        filepath
    ));
    serde_json::from_reader(&file).expect(&format!(
        "Could not deserialize from json file '{}'",
        filepath
    ))
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Utility

/// Makes a panic info a little easier to read by splitting it into the message and location
pub fn panic_message_split_to_message_and_location(
    panic_info: &std::panic::PanicInfo<'_>,
) -> (String, String) {
    let panic_info_content = format!("{}", panic_info).replace("panicked at '", "");
    if let Some(split_pos) = panic_info_content.rfind("', ") {
        let (message, location) = panic_info_content.split_at(split_pos);
        let location = location.replace("', ", "");
        (message.to_string(), location)
    } else {
        ("Panicked".to_string(), panic_info_content)
    }
}

pub fn panic_set_hook_wait_for_keypress() {
    std::panic::set_hook(Box::new(|panic_info| {
        let (message, location) = panic_message_split_to_message_and_location(panic_info);
        println!("\nError: '{}'\nError location: {}", message, location);
        println!("\n\nPRESS THE <ENTER> KEY TO CONTINUE");

        // Wait for keypress
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line).ok();
    }));
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Transmutation

pub unsafe fn transmute_to_byte_slice<S>(from: &[S]) -> &[u8] {
    std::slice::from_raw_parts(
        from.as_ptr() as *const u8,
        from.len() * std::mem::size_of::<S>(),
    )
}

pub unsafe fn transmute_to_byte_slice_mut<S>(from: &mut [S]) -> &mut [u8] {
    std::slice::from_raw_parts_mut(
        from.as_mut_ptr() as *mut u8,
        from.len() * std::mem::size_of::<S>(),
    )
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Finding in containers

#[inline]
pub fn slice_index_of_max<T: Ord>(slice: &[T]) -> Option<usize> {
    slice
        .iter()
        .enumerate()
        .max_by(|(_a_index, a_val), (_b_index, b_val)| a_val.cmp(b_val))
        .map(|(index, _value)| index)
}

#[inline]
pub fn slice_index_of_min<T: Ord>(slice: &[T]) -> Option<usize> {
    slice
        .iter()
        .enumerate()
        .min_by(|(_a_index, a_val), (_b_index, b_val)| a_val.cmp(b_val))
        .map(|(index, _value)| index)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Hashing

/// Hashes the input block using the FNV-1a hashfunction.
/// (https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function)
///
pub fn hash_string_64(input: &str) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    let mut hash = std::num::Wrapping(FNV_OFFSET_BASIS);
    let prime = std::num::Wrapping(FNV_PRIME);
    for byte in input.bytes() {
        hash.0 ^= byte as u64;
        hash *= prime;
    }
    hash.0
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Finding all common divisors of a given list (useful for finding scaled down display resolutions)

pub fn get_all_divisors(value: u32) -> Vec<u32> {
    (1..(value / 2)).filter(|x| value % x == 0).collect()
}

pub fn common_divisors(values: &[u32]) -> Vec<u32> {
    let mut divisor_sets: Vec<HashSet<u32>> = values
        .iter()
        .map(|value| get_all_divisors(*value).into_iter().collect())
        .collect();

    let initial_set = divisor_sets.pop().unwrap();
    let intersection_set: HashSet<u32> =
        divisor_sets.into_iter().fold(initial_set, |acc, other| {
            HashSet::intersection(&acc, &other).cloned().collect()
        });

    let mut result: Vec<u32> = intersection_set.into_iter().collect();
    result.sort();
    result
}

pub fn get_all_resolution_divisors(resolution: (u32, u32)) -> Vec<(u32, u32)> {
    common_divisors(&[resolution.0, resolution.1])
        .iter()
        .map(|divisor| (resolution.0 / divisor, resolution.1 / divisor))
        .collect()
}

pub fn common_resolutions(resolutions: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut divisor_sets: Vec<HashSet<(u32, u32)>> = resolutions
        .iter()
        .map(|value| get_all_resolution_divisors(*value).into_iter().collect())
        .collect();

    let initial_set = divisor_sets.pop().unwrap();
    let intersection_set: HashSet<(u32, u32)> =
        divisor_sets.into_iter().fold(initial_set, |acc, other| {
            HashSet::intersection(&acc, &other).cloned().collect()
        });

    let mut result: Vec<(u32, u32)> = intersection_set.into_iter().collect();
    result.sort();
    result
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Snippets

/*

// Sort floats

v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
v.sort_by(|a, b| a.partial_cmp(b).unwrap())


// Quick random values

use rand::Rng;
let color = Color::new(
    rand::thread_rng().gen::<f32>(),
    rand::thread_rng().gen::<f32>(),
    rand::thread_rng().gen::<f32>(),
    1.0,
);
*/
