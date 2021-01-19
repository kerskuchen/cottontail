/// Based on
/// https://github.com/aseprite/aseprite/blob/master/docs/ase-file-specs.md
///
use super::*;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use std::io::Read;

type BYTE = u8;
type WORD = u16;
type SHORT = i16;
type DWORD = u32;

#[derive(Debug)]
struct FileHeader {
    pub file_size_bytes: usize,
    pub frame_count: usize,
    pub width_in_pixels: usize,
    pub height_in_pixels: usize,
}

impl FileHeader {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<FileHeader, String> {
        let buffer_size_bytes = deserializer.get_remaining_data().len();
        let file_size_bytes = deserializer.deserialize::<DWORD>()? as usize;
        if file_size_bytes != buffer_size_bytes {
            return Err(format!(
                "Filesize in file header ({}) does not match bytecount of data buffer ({})",
                file_size_bytes, buffer_size_bytes
            ));
        }

        let magic_number = deserializer.deserialize::<WORD>()?;
        const FILE_HEADER_MAGIC_NUMBER: WORD = 0xA5E0;
        if magic_number != FILE_HEADER_MAGIC_NUMBER {
            return Err(format!(
                "Invalid file header format: Expected magic number 0x{:X} - got 0x{:X}",
                FILE_HEADER_MAGIC_NUMBER, magic_number
            ));
        }

        let frame_count = deserializer.deserialize::<WORD>()? as usize;
        let width_in_pixels = deserializer.deserialize::<WORD>()? as usize;
        let height_in_pixels = deserializer.deserialize::<WORD>()? as usize;

        // Color depth in bits per pixel
        // RGBA: 32 bit
        // Grayscale:  16 bit
        // Indexed:  8 bit
        let color_depth = deserializer.deserialize::<WORD>()?;
        if color_depth != 32 {
            return Err(format!(
                "Unsupported color depth - expected RGBA (32 bit) - got {}",
                color_depth
            ));
        }

        // Is =1 if layer opacity has valid value
        let flags = deserializer.deserialize::<DWORD>()?;
        if flags != 1 {
            return Err(format!(
                "Expected layer opacity flag to be 0x01 - got 0x{:X}",
                flags
            ));
        }

        // DEPRECATED: Should use the frame duration fields in frame headers
        let _frame_duration_ms = deserializer.deserialize::<WORD>()?;

        let must_be_zero1 = deserializer.deserialize::<DWORD>()?;
        let must_be_zero2 = deserializer.deserialize::<DWORD>()?;
        if must_be_zero1 != 0 || must_be_zero2 != 0 {
            return Err(format!(
                "Invalid file header format - Expected bytes 20 till 28 to be zero",
            ));
        }

        // Only used in indexed sprites. Represents the color palette entry that should be transparent
        // in all non-background laters
        let _transparent_color_palette_entry_index = deserializer.deserialize::<BYTE>()?;
        let _unused = deserializer.deserialize::<[BYTE; 3]>()?;
        // NOTE: In old sprites 0 means 256
        let _color_count = deserializer.deserialize::<WORD>()?;

        // If this is zero it means pixel ratio is 1:1
        let pixel_width = deserializer.deserialize::<BYTE>()?;
        // If this is zero it means pixel ratio is 1:1
        let pixel_height = deserializer.deserialize::<BYTE>()?;
        if pixel_width > 1 || pixel_height > 1 {
            return Err(format!(
                "Unsupported pixel aspect ration - expected 1:1 - got {}:{}",
                pixel_width, pixel_height
            ));
        }

        let _grid_pos_x = deserializer.deserialize::<SHORT>()?;
        let _grid_pos_y = deserializer.deserialize::<SHORT>()?;
        // Zero if there is no grid
        let _grid_width = deserializer.deserialize::<WORD>()?;
        // Zero if there is no grid
        let _grid_height = deserializer.deserialize::<WORD>()?;
        // NOTE: We used 3 reserved blocks that add up to 84 bytes so `derive(Deserialize)` just works
        let _reserved1 = deserializer.deserialize::<[BYTE; 32]>()?;
        let _reserved2 = deserializer.deserialize::<[BYTE; 32]>()?;
        let _reserved3 = deserializer.deserialize::<[BYTE; 20]>()?;

        Ok(FileHeader {
            file_size_bytes,
            frame_count,
            width_in_pixels,
            height_in_pixels,
        })
    }
}

#[derive(Debug)]
struct FrameHeader {
    pub frame_size_bytes: usize,
    pub chunk_count: usize,
    pub frame_duration_ms: usize,
}

impl FrameHeader {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<FrameHeader, String> {
        let frame_size_bytes = deserializer.deserialize::<DWORD>()? as usize;

        let magic_number = deserializer.deserialize::<WORD>()?;
        const FRAME_HEADER_MAGIC_NUMBER: WORD = 0xF1FA;
        if magic_number != FRAME_HEADER_MAGIC_NUMBER {
            return Err(format!(
                "Invalid frame header format: Expected magic number 0x{:X} - got 0x{:X}",
                FRAME_HEADER_MAGIC_NUMBER, magic_number
            ));
        }

        // If this is 0xFFFF we need to use `chunk_count_new` instead
        let chunk_count_old = deserializer.deserialize::<WORD>()? as usize;
        let frame_duration_ms = deserializer.deserialize::<WORD>()? as usize;
        let _reserved = deserializer.deserialize::<[BYTE; 2]>()?;
        // If this is 0x0 we need to use `chunk_count_old` instead
        let chunk_count_new = deserializer.deserialize::<DWORD>()? as usize;
        if chunk_count_new == 0 && chunk_count_old == 0xFF {
            return Err(format!(
                "Invalid chunk count in frame header: New chunk count field was 0 but old chunk count field was 0xFF",
            ));
        }
        let chunk_count = if chunk_count_new != 0 {
            chunk_count_new
        } else {
            chunk_count_old
        };

        Ok(FrameHeader {
            frame_size_bytes,
            chunk_count,
            frame_duration_ms,
        })
    }
}

pub fn run() -> Result<(), String> {
    let filepath = "assets/example/sprites/sorcy.ase";
    let data = read_file_whole(filepath)?;

    let mut file_deserializer = Deserializer::new(&data);
    let fileheader = FileHeader::from_deserializer(&mut file_deserializer)?;
    println!("header {:?}", &fileheader);

    let mut animation_tags = HashMap::new();
    let mut layers = Vec::new();
    let mut cels = HashMap::new();
    for frame_index in 0..fileheader.frame_count {
        let mut frame_deserializer = file_deserializer.clone();
        let frameheader = FrameHeader::from_deserializer(&mut frame_deserializer)?;
        println!("frame {}: {:?}", frame_index, &frameheader);

        for chunk_index in 0..frameheader.chunk_count {
            let mut chunk_deserializer = frame_deserializer.clone();
            let chunk_size_bytes = chunk_deserializer.deserialize::<DWORD>()? as usize;
            let chunk_type = ChunkType::from_word(chunk_deserializer.deserialize::<WORD>()?)?;
            let remaining_chunk_size_bytes =
                chunk_size_bytes - std::mem::size_of::<DWORD>() - std::mem::size_of::<WORD>();

            println!(
                "frame {} chunk {}: size: {}, type: {:?}",
                frame_index, chunk_index, chunk_size_bytes, &chunk_type
            );

            match chunk_type {
                ChunkType::Layer => {
                    let layer = ChunkLayer::from_deserializer(&mut chunk_deserializer).map_err(
                        |error| {
                            format!(
                                "Failed to read layer chunk {} ({:?}) in frame {}: {}",
                                chunk_index, chunk_type, frame_index, error
                            )
                        },
                    )?;
                    layers.push(layer);
                }
                ChunkType::Cel => {
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
                    cels.insert((frame_index, cel.layer_index), cel);
                }
                ChunkType::Tags => {
                    let tags =
                        ChunkTags::from_deserializer(&mut chunk_deserializer).map_err(|error| {
                            format!(
                                "Failed to read tag chunk {} ({:?}) in frame {}: {}",
                                chunk_index, chunk_type, frame_index, error
                            )
                        })?;
                    for tag in tags.tags.into_iter() {
                        animation_tags.insert(tag.name.clone(), tag);
                    }
                }
                _ => {
                    // Not supported
                    log::trace!(
                        "Chunk with type {:?} is not supported and will be ignored",
                        chunk_type
                    );
                }
            }
            //dbg!(&chunk);

            frame_deserializer.skip_bytes(chunk_size_bytes)?;
        }

        file_deserializer.skip_bytes(frameheader.frame_size_bytes as usize)?;
    }

    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CHUNKS

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

//--------------------------------------------------------------------------------------------------
// CEL CHUNK

#[derive(Debug)]
enum Cel {
    Linked { frame_index_to_link_with: usize },
    Image { bitmap: Bitmap },
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
                let pixeldata_bytecount = 4 * width * height;
                let pixeldata = &deserializer.get_remaining_data()[..pixeldata_bytecount];
                Cel::Image {
                    bitmap: Bitmap::new_from_bytes(width as u32, height as u32, pixeldata),
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
                let pixeldata_bytecount = 4 * width * height;

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
                    bitmap: Bitmap::new_from_bytes(width as u32, height as u32, &pixeldata),
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
// LAYER CHUNK

#[derive(Debug, PartialEq, Eq, FromPrimitive)]
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
    blend_mode: LayerBlendMode,
    opacity: u8,
}

impl ChunkLayer {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<ChunkLayer, String> {
        // Deserialization
        let flags = deserializer.deserialize::<WORD>()?;
        let layer_type = deserializer.deserialize::<WORD>()?;
        let _layer_child_level = deserializer.deserialize::<WORD>()?;
        let _ignored_default_layer_width_pixels = deserializer.skip::<WORD>()?;
        let _ignored_default_layer_height_pixels = deserializer.skip::<WORD>()?;
        let blend_mode = deserializer.deserialize::<WORD>()?;
        let opacity = deserializer.deserialize::<BYTE>()?;
        let _reserved = deserializer.skip_bytes(3)?;
        let name = deserialize_aseprite_string(deserializer)?;

        // Conversion
        let flag_visible = (flags & 1) == 1;
        let _flag_editable = (flags & 2) == 2;
        let _flag_lock_movement = (flags & 4) == 4;
        let _flag_background = (flags & 8) == 8;
        let _flag_prefer_linked_cels = (flags & 16) == 16;
        let _flag_layer_group_should_be_displayed_collapsed = (flags & 32) == 32;
        let _flag_layer_is_reference_layer = (flags & 64) == 64;

        let layer_type = LayerType::from_word(layer_type)?;
        if layer_type == LayerType::Group {
            return Err(format!("Group layers not supported - in layer '{}'", name));
        }

        Ok(ChunkLayer {
            name,
            flag_visible,
            blend_mode: LayerBlendMode::from_word(blend_mode)?,
            opacity: opacity as u8,
        })
    }
}

//--------------------------------------------------------------------------------------------------
// TAGS CHUNK

#[derive(Debug)]
struct AnimationTag {
    name: String,
    frameindex_start: usize,
    frameindex_end: usize,
    animation_loop_direction: String,
}

impl AnimationTag {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<AnimationTag, String> {
        let frameindex_start = deserializer.deserialize::<WORD>()? as usize;
        let frameindex_end = deserializer.deserialize::<WORD>()? as usize;
        let animation_loop_direction_raw = deserializer.deserialize::<BYTE>()?;
        let _ignored_reserved = deserializer.skip_bytes(8)?;
        let _color_r = deserializer.deserialize::<BYTE>()?;
        let _color_g = deserializer.deserialize::<BYTE>()?;
        let _color_b = deserializer.deserialize::<BYTE>()?;
        let _ignored_extra_byte = deserializer.skip::<BYTE>()?;
        let name = deserialize_aseprite_string(deserializer)?;

        let animation_loop_direction = match animation_loop_direction_raw {
            0 => "forward",
            1 => "reverse",
            2 => "pingpong",
            _ => {
                return Err(format!(
                    "Tag has invalid loop direction field '{}' - expected 0,1 or 2",
                    animation_loop_direction_raw
                ))
            }
        }
        .to_owned();

        Ok(AnimationTag {
            name,
            frameindex_start,
            frameindex_end,
            animation_loop_direction,
        })
    }
}

#[derive(Debug)]
struct ChunkTags {
    tags: Vec<AnimationTag>,
}
impl ChunkTags {
    fn from_deserializer(deserializer: &mut Deserializer) -> Result<ChunkTags, String> {
        let tag_count = deserializer.deserialize::<WORD>()? as usize;
        let _ignore_reserved = deserializer.skip_bytes(8)?;
        let tags = {
            let tags: Result<Vec<AnimationTag>, String> = (0..tag_count)
                .into_iter()
                .map(|_tag_index| AnimationTag::from_deserializer(deserializer))
                .collect();
            tags?
        };
        Ok(ChunkTags { tags })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ASEPRITE STRINGS

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
