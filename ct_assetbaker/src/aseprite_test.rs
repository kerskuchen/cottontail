use super::*;

type BYTE = u8;
type WORD = u16;
type SHORT = i16;
type DWORD = u32;
type LONG = i32;
// type FIXED = TODO;

// https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
const FILE_HEADER_MAGIC_NUMBER: WORD = 0xA5E0;
const FRAME_HEADER_MAGIC_NUMBER: WORD = 0xF1FA;

const CHUNK_TYPE_OLD_PALETTE_1: WORD = 0x0004; // DEPRECATED
const CHUNK_TYPE_OLD_PALETTE_2: WORD = 0x00011; // DEPRECATED
const CHUNK_TYPE_LAYER: WORD = 0x2004;
const CHUNK_TYPE_CEL: WORD = 0x2005;
const CHUNK_TYPE_EXTRA_CEL: WORD = 0x2006;
const CHUNK_TYPE_COLOR_PROFILE: WORD = 0x2007;
const CHUNK_TYPE_MASK: WORD = 0x2016; // DEPRECATED
const CHUNK_TYPE_PATH: WORD = 0x2017; // NEVER USED
const CHUNK_TYPE_TAGS: WORD = 0x2018;
const CHUNK_TYPE_PALETTE: WORD = 0x2019;
const CHUNK_TYPE_USER_DATA: WORD = 0x2020;
const CHUNK_TYPE_SLICE: WORD = 0x2022;

#[repr(packed)]
#[derive(Debug, Serialize, Deserialize)]
struct FileHeaderRaw {
    file_size_bytes: DWORD,
    /// Must be 0xA5E0
    magic_number: WORD,
    frame_count: WORD,
    width_in_pixels: WORD,
    height_in_pixels: WORD,
    /// Color depth in bits per pixel
    /// RGBA: 32 bit
    /// Grayscale:  16 bit
    /// Indexed:  8 bit
    color_depth: WORD,
    // Is =1 if layer opacity has valid value
    flags: DWORD,
    /// DEPRECATED: Should use the frame duration fields in frame headers
    frame_duration_ms: WORD,
    /// Must be 0
    must_be_zero1: DWORD,
    /// Must be 0
    must_be_zero2: DWORD,
    // Only used in indexed sprites. Represents the color palette entry that should be transparent
    // in all non-background laters
    transparent_color_palette_entry_index: BYTE,
    unused: [BYTE; 3],
    /// NOTE: In old sprites 0 means 256
    color_count: WORD,
    /// If this is zero it means pixel ratio is 1:1
    pixel_width: BYTE,
    /// If this is zero it means pixel ratio is 1:1
    pixel_height: BYTE,
    grid_pos_x: SHORT,
    grid_pos_y: SHORT,
    /// Zero if there is no grid
    grid_width: WORD,
    /// Zero if there is no grid
    grid_height: WORD,
    // NOTE: We used 3 reserved blocks that add up to 84 bytes so `derive(Deserialize)` just works
    reserved1: [BYTE; 32],
    reserved2: [BYTE; 32],
    reserved3: [BYTE; 20],
}

impl FileHeaderRaw {
    fn from_bytes(bytes: &[u8]) -> Result<FileHeaderRaw, String> {
        let header = deserialize_from_binary::<FileHeaderRaw>(bytes);
        if header.magic_number != FILE_HEADER_MAGIC_NUMBER {
            return Err(format!(
                "Invalid file header format: Expected magic number 0x{:X} - got 0x{:X}",
                FILE_HEADER_MAGIC_NUMBER, header.magic_number
            ));
        }
        if header.must_be_zero1 != 0 || header.must_be_zero2 != 0 {
            return Err(format!(
                "Invalid file header format - Expected bytes 20 to 28 to be zero",
            ));
        }
        if header.file_size_bytes != bytes.len() as u32 {
            return Err(format!(
                "Filesize in file header ({}) does not match bytecount of data ({})",
                header.file_size_bytes,
                bytes.len()
            ));
        }

        Ok(header)
    }

    fn frame_count(&self) -> usize {
        self.frame_count as usize
    }
}

#[repr(packed)]
#[derive(Debug, Serialize, Deserialize)]
struct FrameHeaderRaw {
    frame_size_bytes: DWORD,
    /// Must be 0xF1FA
    magic_number: WORD,
    /// If this is 0xFFFF we need to use `chunk_count_new` instead
    chunk_count_old: WORD,
    frame_duration_ms: WORD,
    reserved: [BYTE; 2],
    /// If this is 0x0 we need to use `chunk_count_old` instead
    chunk_count_new: DWORD,
}

impl FrameHeaderRaw {
    fn from_bytes(bytes: &[u8]) -> Result<FrameHeaderRaw, String> {
        let header = deserialize_from_binary::<FrameHeaderRaw>(bytes);
        if header.magic_number != FRAME_HEADER_MAGIC_NUMBER {
            return Err(format!(
                "Invalid frame header format: Expected magic number 0x{:X} - got 0x{:X}",
                FRAME_HEADER_MAGIC_NUMBER, header.magic_number
            ));
        }

        if header.chunk_count_new == 0 && header.chunk_count_old == 0xFF {
            return Err(format!(
                "Invalid chunk count in frame header: New chunk count field was 0 but old chunk count field was 0xFF",
            ));
        }

        Ok(header)
    }

    fn chunk_count(&self) -> usize {
        if self.chunk_count_new != 0 {
            self.chunk_count_new as usize
        } else {
            self.chunk_count_old as usize
        }
    }
}

#[repr(packed)]
#[derive(Debug, Serialize, Deserialize)]
struct ChunkHeaderRaw {
    chunk_size_bytes: DWORD,
    chunk_type: WORD,
}

impl ChunkHeaderRaw {
    fn chunk_size_bytes(&self) -> usize {
        self.chunk_size_bytes as usize
    }
}

pub fn run() -> Result<(), String> {
    let data = read_file_whole("assets/example/sprites/sorcy.ase").unwrap();
    let fileheader_slice = &data[..];
    let fileheader = FileHeaderRaw::from_bytes(&fileheader_slice).unwrap();
    dbg!(&fileheader);

    let mut frames_slice = &fileheader_slice[std::mem::size_of::<FileHeaderRaw>()..];
    for frame_index in 0..fileheader.frame_count() {
        let frameheader = FrameHeaderRaw::from_bytes(&frames_slice).unwrap();
        println!("frame {}: {:?}", frame_index, &frameheader);

        let mut chunks_slice = &frames_slice[std::mem::size_of::<FrameHeaderRaw>()..];
        for chunk_index in 0..frameheader.chunk_count() {
            let chunkheader = deserialize_from_binary::<ChunkHeaderRaw>(chunks_slice);

            println!(
                "frame {} chunk {}: size: {}, type: {:X?}",
                frame_index,
                chunk_index,
                chunkheader.chunk_size_bytes(),
                chunkheader.chunk_type
            );

            {
                let chunkdata_slice = &chunks_slice[std::mem::size_of::<ChunkHeaderRaw>()..];
                if chunkheader.chunk_type == CHUNK_TYPE_TAGS {
                    let tags = ChunkTags::from_bytes(chunkdata_slice).map_err(|error| {
                        format!(
                            "Failed to read chunk {} (type: {:X}) in frame {}: {}",
                            chunk_index, chunkheader.chunk_type, frame_index, error
                        )
                    })?;
                    dbg!(tags);
                }
            }

            // skip chunk
            chunks_slice = &chunks_slice[chunkheader.chunk_size_bytes()..];
        }

        frames_slice = &frames_slice[frameheader.frame_size_bytes as usize..];
    }

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// TAGS

#[derive(Debug)]
enum AnimationLoopDirection {
    Forward = 0,
    Reverse = 1,
    PingPong = 2,
}

#[derive(Debug)]
struct Tag {
    name: String,
    frameindex_start: usize,
    frameindex_end: usize,
    animation_loop_direction: AnimationLoopDirection,
    color: PixelRGBA,
}
impl Tag {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<Tag, String> {
        let frameindex_start = deserializer.deserialize::<WORD>()? as usize;
        let frameindex_end = deserializer.deserialize::<WORD>()? as usize;
        let animation_loop_direction_raw = deserializer.deserialize::<BYTE>()?;
        deserializer.skip_bytes(8)?; // Reserved
        let color_r = deserializer.deserialize::<BYTE>()?;
        let color_g = deserializer.deserialize::<BYTE>()?;
        let color_b = deserializer.deserialize::<BYTE>()?;
        deserializer.skip_bytes(1)?; // Extra byte
        let name = deserialize_aseprite_string(deserializer)?;

        let animation_loop_direction = match animation_loop_direction_raw {
            0 => AnimationLoopDirection::Forward,
            1 => AnimationLoopDirection::Reverse,
            2 => AnimationLoopDirection::PingPong,
            _ => {
                return Err(format!(
                    "Tag has invalid loop direction field '{}' - expected 0,1 or 2",
                    animation_loop_direction_raw
                ))
            }
        };

        Ok(Tag {
            name,
            frameindex_start,
            frameindex_end,
            animation_loop_direction,
            color: PixelRGBA::new(color_r, color_g, color_b, 255),
        })
    }
}

#[derive(Debug)]
struct ChunkTags {
    tags: Vec<Tag>,
}
impl ChunkTags {
    fn from_bytes(bytes: &[u8]) -> Result<ChunkTags, String> {
        let mut deserializer = Deserializer::new(bytes);
        let tag_count = deserializer.deserialize::<WORD>()? as usize;
        deserializer.skip_bytes(8)?; // Reserved
        let tags = {
            let tags: Result<Vec<Tag>, String> = (0..tag_count)
                .into_iter()
                .map(|_tag_index| Tag::from_deserializer(&mut deserializer))
                .collect();
            tags?
        };
        Ok(ChunkTags { tags })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DESERIALIZER

struct Deserializer<'a> {
    data: &'a [u8],
}

impl Deserializer<'_> {
    pub fn new(data: &[u8]) -> Deserializer {
        Deserializer { data }
    }

    pub fn get_remaining_data(&self) -> &[u8] {
        self.data
    }

    pub fn skip_bytes(&mut self, byte_count: usize) -> Result<(), String> {
        if byte_count > self.data.len() {
            return Err(format!(
                "Cannot not skip {} bytes, internal buffer has only {} bytes left",
                byte_count,
                self.data.len()
            ));
        }
        self.data = &self.data[byte_count..];
        Ok(())
    }

    pub fn deserialize<T>(&mut self) -> Result<T, String>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let size = std::mem::size_of::<T>();
        if size > self.data.len() {
            return Err(format!(
                "Cannot not deserialize {} which required {} bytes, but internal buffer has only {} bytes left",
                std::any::type_name::<T>(),
                size,
                self.data.len()
            ));
        }
        let result = bincode::deserialize::<T>(self.data).map_err(|error| {
            format!(
                "Could not deserialize {}: {}",
                std::any::type_name::<T>(),
                error
            )
        })?;
        self.data = &self.data[size..];
        Ok(result)
    }
}

fn deserialize_aseprite_string(deserializer: &mut Deserializer) -> Result<String, String> {
    let string_length_bytes = deserializer.deserialize::<WORD>()? as usize;
    let result = {
        let (string_part, _remainder) = deserializer
            .get_remaining_data()
            .split_at(string_length_bytes);

        String::from_utf8(string_part.to_vec()).map_err(|error| {
            format!(
                "Deserialized string is not a valid UTF-8 string: {} ",
                error
            )
        })
    };
    deserializer.skip_bytes(string_length_bytes)?;
    result
}
