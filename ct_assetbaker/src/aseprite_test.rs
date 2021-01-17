use std::io::Read;

use super::*;

use fixed::{types, FixedU32};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

type BYTE = u8;
type WORD = u16;
type SHORT = i16;
type DWORD = u32;
type LONG = i32;

// https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
const FILE_HEADER_MAGIC_NUMBER: WORD = 0xA5E0;
const FRAME_HEADER_MAGIC_NUMBER: WORD = 0xF1FA;

#[derive(Debug, FromPrimitive)]
enum ChunkType {
    PaletteOld1 = 0x0004,  // DEPRECATED
    PaletteOld2 = 0x00011, // DEPRECATED
    Layer = 0x2004,
    Cel = 0x2005,
    CelExtra = 0x2006,
    ColorProfile = 0x2007,
    Mask = 0x2016, // DEPRECATED
    Path = 0x2017, // NEVER USED
    Tags = 0x2018,
    Palette = 0x2019,
    UserData = 0x2020,
    Slice = 0x2022,
}

impl ChunkType {
    fn from_word(word: WORD) -> Result<ChunkType, String> {
        FromPrimitive::from_u16(word).ok_or_else(|| format!("Unknown chunk type {:X}", word))
    }
}

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
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<FileHeaderRaw, String> {
        let filesize = deserializer.get_remaining_data().len();
        let header: FileHeaderRaw = deserializer.deserialize()?;
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
        if header.file_size_bytes != filesize as u32 {
            return Err(format!(
                "Filesize in file header ({}) does not match bytecount of data ({})",
                header.file_size_bytes, filesize
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
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<FrameHeaderRaw, String> {
        let header: FrameHeaderRaw = deserializer.deserialize()?;
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

pub fn run() -> Result<(), String> {
    let data = read_file_whole("assets/example/sprites/sorcy.ase")?;
    let mut file_deserializer = Deserializer::new(&data);
    let fileheader = FileHeaderRaw::from_deserializer(&mut file_deserializer)?;
    println!("header {:?}", &fileheader);

    for frame_index in 0..fileheader.frame_count() {
        let mut frame_deserializer = file_deserializer.clone();
        let frameheader = FrameHeaderRaw::from_deserializer(&mut frame_deserializer)?;
        println!("frame {}: {:?}", frame_index, &frameheader);

        for chunk_index in 0..frameheader.chunk_count() {
            let mut chunk_deserializer = frame_deserializer.clone();
            let chunk_size_bytes = chunk_deserializer.deserialize::<DWORD>()? as usize;
            let chunk_type = ChunkType::from_word(chunk_deserializer.deserialize::<WORD>()?)?;

            println!(
                "frame {} chunk {}: size: {}, type: {:?}",
                frame_index, chunk_index, chunk_size_bytes, chunk_type
            );

            let chunk = match chunk_type {
                ChunkType::Layer => {
                    let layer = ChunkLayer::from_deserializer(&mut chunk_deserializer).map_err(
                        |error| {
                            format!(
                                "Failed to read layer chunk {} ({:?}) in frame {}: {}",
                                chunk_index, chunk_type, frame_index, error
                            )
                        },
                    )?;
                    Some(Chunk::Layer(layer))
                }
                ChunkType::Cel => {
                    let remaining_chunk_size_bytes = chunk_size_bytes
                        - std::mem::size_of::<DWORD>()
                        - std::mem::size_of::<WORD>();
                    let cel = ChunkCel::from_deserializer(
                        &mut chunk_deserializer,
                        remaining_chunk_size_bytes,
                    )
                    .map_err(|error| {
                        format!(
                            "Failed to read cel chunk {} ({:?}) in frame {}: {}",
                            chunk_index, chunk_type, frame_index, error
                        )
                    })?;
                    Some(Chunk::Cel(cel))
                }
                ChunkType::ColorProfile => {
                    let color_profile = ChunkColorProfile::from_deserializer(
                        &mut chunk_deserializer,
                    )
                    .map_err(|error| {
                        format!(
                            "Failed to read color profile chunk {} ({:?}) in frame {}: {}",
                            chunk_index, chunk_type, frame_index, error
                        )
                    })?;
                    Some(Chunk::ColorProfile(color_profile))
                }
                ChunkType::Tags => {
                    let tags =
                        ChunkTags::from_deserializer(&mut chunk_deserializer).map_err(|error| {
                            format!(
                                "Failed to read tag chunk {} ({:?}) in frame {}: {}",
                                chunk_index, chunk_type, frame_index, error
                            )
                        })?;
                    Some(Chunk::Tags(tags))
                }
                _ => {
                    // Not supported
                    print!(
                        "Chunk with type {:?} is not supported and will be ignored",
                        chunk_type
                    );
                    None
                }
            };
            dbg!(&chunk);

            frame_deserializer.skip_bytes(chunk_size_bytes)?;
        }

        file_deserializer.skip_bytes(frameheader.frame_size_bytes as usize)?;
    }

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CHUNKS

#[derive(Debug)]
enum Chunk {
    Tags(ChunkTags),
    Layer(ChunkLayer),
    ColorProfile(ChunkColorProfile),
    Cel(ChunkCel),
}

//--------------------------------------------------------------------------------------------------
// CEL CHUNK

#[derive(Debug)]
enum Cel {
    Linked {
        frame_index_to_link_with: usize,
    },
    Image {
        width: usize,
        height: usize,
        pixeldata: Vec<u8>,
    },
}
#[derive(Debug)]
struct ChunkCel {
    layer_index: usize,
    opacity: u8,
    pos_x: i32,
    pos_y: i32,
    cel: Cel,
}

impl ChunkCel {
    fn from_deserializer(
        deserializer: &mut Deserializer,
        chunk_size_bytes: usize,
    ) -> Result<ChunkCel, String> {
        let deserializer_start_size = deserializer.get_remaining_data().len();
        let layer_index = deserializer.deserialize::<WORD>()? as usize;
        let pos_x = deserializer.deserialize::<SHORT>()? as i32;
        let pos_y = deserializer.deserialize::<SHORT>()? as i32;
        let opacity = deserializer.deserialize::<BYTE>()? as u8;
        let cel = deserializer.deserialize::<WORD>()? as u8;
        let _ignore_reserved = deserializer.skip_bytes(7)?;

        let cel = match cel {
            0 => {
                // Raw cell
                let width = deserializer.deserialize::<WORD>()? as usize;
                let height = deserializer.deserialize::<WORD>()? as usize;
                let TODO = " This is wrong for greyscale and indexed";
                let pixeldata_bytecount = 4 * width * height;

                let pixeldata = deserializer.get_remaining_data()[..pixeldata_bytecount].to_vec();
                Cel::Image {
                    width,
                    height,
                    pixeldata,
                }
            }
            1 => {
                // linked cel
                let frame_index_to_link_with = deserializer.deserialize::<WORD>()? as usize;
                Cel::Linked {
                    frame_index_to_link_with,
                }
            }
            2 => {
                // compresed image
                let width = deserializer.deserialize::<WORD>()? as usize;
                let height = deserializer.deserialize::<WORD>()? as usize;
                let TODO = " This is wrong for greyscale and indexed";
                let pixeldata_bytecount = 4 * width * height;

                let TODO = "Make deserializer only contain chunk buffer (what is needed)";
                let deserializer_current_size = deserializer.get_remaining_data().len();
                let remaining_chunk_size =
                    chunk_size_bytes - (deserializer_start_size - deserializer_current_size);

                let mut zlib_decoder = libflate::zlib::Decoder::new(
                    &deserializer.get_remaining_data()[..remaining_chunk_size],
                )
                .map_err(|error| {
                    format!("Could not prepare pixel data for decompression: {}", error)
                })?;
                let mut pixeldata = Vec::new();
                let decompressed_bytecount = zlib_decoder
                    .read_to_end(&mut pixeldata)
                    .map_err(|error| format!("Could not decompress pixel data: {}", error))?;
                if decompressed_bytecount == 0 {
                    return Err(format!(
                        "Decompressed {} bytes pixel data - expected {}",
                        decompressed_bytecount, pixeldata_bytecount
                    ));
                }
                Cel::Image {
                    width,
                    height,
                    pixeldata,
                }
            }
            _ => return Err(format!("Unknown cel type {:X}", cel)),
        };

        Ok(ChunkCel {
            layer_index,
            opacity,
            pos_x,
            pos_y,
            cel,
        })
    }
}

//--------------------------------------------------------------------------------------------------
// COLOR PROFILE CHUNK

#[derive(Debug, FromPrimitive, PartialEq, Eq)]
enum ColorProfileType {
    NoColorProfile = 0,
    UseSRGB = 1,
    UseEmbeddedICC = 2,
}
impl ColorProfileType {
    fn from_word(word: WORD) -> Result<ColorProfileType, String> {
        FromPrimitive::from_u16(word)
            .ok_or_else(|| format!("Unknown color profile type {:X}", word))
    }
}

#[derive(Debug)]
struct ChunkColorProfile {
    profile_type: ColorProfileType,
    gamma: Option<f32>,
    icc_data: Option<Vec<u8>>,
}

impl ChunkColorProfile {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<ChunkColorProfile, String> {
        let profile_type = ColorProfileType::from_word(deserializer.deserialize::<WORD>()?)?;
        let flags = deserializer.deserialize::<WORD>()?;
        let gamma = deserialize_aseprite_fixed_point_number(deserializer)?;
        let _ignored_reserved = deserializer.skip_bytes(8)?;
        let icc_data = if profile_type == ColorProfileType::UseEmbeddedICC {
            let icc_data_size_bytes = deserializer.deserialize::<DWORD>()? as usize;
            if deserializer.get_remaining_data().len() < icc_data_size_bytes {
                return Err(format!(
                    "Expected ICC profile size was {} bytes - got {} bytes",
                    icc_data_size_bytes,
                    deserializer.get_remaining_data().len()
                ));
            }
            Some(deserializer.get_remaining_data()[..icc_data_size_bytes].to_vec())
        } else {
            None
        };

        let flag_use_fixed_gamma = (flags & 1) == 1;
        let gamma = if flag_use_fixed_gamma {
            Some(gamma)
        } else {
            None
        };

        Ok(ChunkColorProfile {
            profile_type,
            gamma,
            icc_data,
        })
    }
}

//--------------------------------------------------------------------------------------------------
// LAYER CHUNK

#[derive(Debug, FromPrimitive)]
enum LayerType {
    Normal = 0,
    Group = 1,
}
impl LayerType {
    fn from_word(word: WORD) -> Result<LayerType, String> {
        FromPrimitive::from_u16(word).ok_or_else(|| format!("Unknown layer type {:X}", word))
    }
}

#[derive(Debug, FromPrimitive)]
enum LayerBlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
    Addition = 16,
    Subtract = 17,
    Divide = 18,
}
impl LayerBlendMode {
    fn from_word(word: WORD) -> Result<LayerBlendMode, String> {
        FromPrimitive::from_u16(word).ok_or_else(|| format!("Unknown layer blend mode {:X}", word))
    }
}

#[derive(Debug)]
struct ChunkLayer {
    name: String,

    flag_visible: bool,
    flag_editable: bool,
    flag_lock_movement: bool,
    flag_background: bool,
    flag_prefer_linked_cels: bool,
    flag_layer_group_should_be_displayed_collapsed: bool,
    flag_layer_is_reference_layer: bool,

    layer_type: LayerType,
    layer_child_level: usize,
    blend_mode: LayerBlendMode,

    opacity: u8,
}

impl ChunkLayer {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<ChunkLayer, String> {
        // Deserialization
        let flags = deserializer.deserialize::<WORD>()?;
        let layer_type = deserializer.deserialize::<WORD>()?;
        let layer_child_level = deserializer.deserialize::<WORD>()?;
        let _ignored_default_layer_width_pixels = deserializer.skip::<WORD>()?;
        let _ignored_default_layer_height_pixels = deserializer.skip::<WORD>()?;
        let blend_mode = deserializer.deserialize::<WORD>()?;
        let opacity = deserializer.deserialize::<BYTE>()?;
        let _reserved = deserializer.skip_bytes(3)?;
        let name = deserialize_aseprite_string(deserializer)?;

        // Conversion
        let flag_visible = (flags & 1) == 1;
        let flag_editable = (flags & 2) == 2;
        let flag_lock_movement = (flags & 4) == 4;
        let flag_background = (flags & 8) == 8;
        let flag_prefer_linked_cels = (flags & 16) == 16;
        let flag_layer_group_should_be_displayed_collapsed = (flags & 32) == 32;
        let flag_layer_is_reference_layer = (flags & 64) == 64;

        Ok(ChunkLayer {
            name,
            flag_visible,
            flag_editable,
            flag_lock_movement,
            flag_background,
            flag_prefer_linked_cels,
            flag_layer_group_should_be_displayed_collapsed,
            flag_layer_is_reference_layer,
            layer_type: LayerType::from_word(layer_type)?,
            layer_child_level: layer_child_level as usize,
            blend_mode: LayerBlendMode::from_word(blend_mode)?,
            opacity: opacity as u8,
        })
    }
}

//--------------------------------------------------------------------------------------------------
// TAGS CHUNK

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
        let frameindex_start = deserializer.deserialize::<WORD>()?;
        let frameindex_end = deserializer.deserialize::<WORD>()?;
        let animation_loop_direction_raw = deserializer.deserialize::<BYTE>()?;
        let _ignored_reserved = deserializer.skip_bytes(8)?;
        let color_r = deserializer.deserialize::<BYTE>()?;
        let color_g = deserializer.deserialize::<BYTE>()?;
        let color_b = deserializer.deserialize::<BYTE>()?;
        let _ignored_extra_byte = deserializer.skip::<BYTE>()?;
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
            frameindex_start: frameindex_start as usize,
            frameindex_end: frameindex_end as usize,
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
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<ChunkTags, String> {
        let tag_count = deserializer.deserialize::<WORD>()? as usize;
        let _ignore_reserved = deserializer.skip_bytes(8)?;
        let tags = {
            let tags: Result<Vec<Tag>, String> = (0..tag_count)
                .into_iter()
                .map(|_tag_index| Tag::from_deserializer(deserializer))
                .collect();
            tags?
        };
        Ok(ChunkTags { tags })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DESERIALIZER

#[derive(Clone)]
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

    pub fn skip<T>(&mut self) -> Result<(), String>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let size = std::mem::size_of::<T>();
        self.skip_bytes(size)
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

fn deserialize_aseprite_fixed_point_number(deserializer: &mut Deserializer) -> Result<f32, String> {
    let fixed_uint = deserializer.deserialize::<u32>()?;
    Ok(FixedU32::<types::extra::U16>::from_bits(fixed_uint).to_num::<f32>())
}
