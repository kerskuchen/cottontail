use std::{io::Cursor, unimplemented};

use ase::ColorDepth;

use super::*;

#[derive(Debug)]
pub struct Layer {
    name: String,
    index: usize,
    is_visible: bool,
    // frames: Vec<Bitmap>
}

#[derive(Debug)]
pub struct AnimationTag {
    name: String,
    frameindex_start: usize,
    frameindex_end: usize,
    loop_direction: String,
}

#[derive(Debug)]
pub struct Aseprite {
    width: usize,
    height: usize,
    layers: Vec<Layer>,
    frame_durations_ms: Vec<u32>,
    animation_tags: HashMap<String, AnimationTag>,
}

pub fn aseprite_list_layers_of_file(filepath: &str) -> Vec<String> {
    let aseprite = parse_aseprite_file(filepath).expect("Failed to list aseprite file layers");
    aseprite
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect()
}

fn aseprite_get_offsets_for_layer(filepath: &str, layer_name: &str, out_offsets: &mut Vec<Vec2i>) {
    let aseprite = parse_aseprite_file(filepath).expect("Failed to list aseprite file layers");
    let framecount = out_offsets.len();
    assert!(framecount > 0);

    /*
       let command = String::from("aseprite")
           + " --batch"
           + " --list-layers"
           + " --list-tags"
           + " --layer"
           + " \""
           + layer_name
           + "\""
           + " --format \"json-array\""
           + " --trim"
           + " --ignore-empty "
           + image_filepath
           + " --sheet "
           + output_filepath_image
           + " --data "
           + output_filepath_meta;
       run_systemcommand_fail_on_error(&command, false);

       assert!(
           path_exists(&output_filepath_image),
           "Failed to generate offset information for '{}' - '{}' is missing",
           image_filepath,
           output_filepath_image
       );
       assert!(
           path_exists(&output_filepath_meta),
           "Failed to generate offset information for '{}' - '{}' is missing",
           image_filepath,
           output_filepath_meta
       );

       // We don't need the actual offset image as it is just a bunch of merged pixels. We do need to
       // rename the image though so it does not get the texture packer confused in a later stage
       std::fs::rename(
           &output_filepath_image,
           &(output_filepath_image.to_owned() + ".backup"),
       )
       .unwrap();

       let metadata_string = std::fs::read_to_string(output_filepath_meta).unwrap();
       if metadata_string.len() == 0 {
           // NOTE: Sometimes we get an empty json file for images without offsets
           return;
       }

       let meta: AsepriteJSON = serde_json::from_str(&metadata_string).expect(&format!(
           "Failed to generate offset information for '{}' - Cannot parse metadata '{}'",
           image_filepath, output_filepath_meta
       ));

       assert!(
           meta.frames.len() == 0 || meta.frames.len() == framecount,
           "Failed to generate offset information for '{}' - Offset points in layer '{}' need
               to be placed either on every frame or on none",
           image_filepath,
           layer_name
       );

       for (index, frame) in meta.frames.iter().enumerate() {
           out_offsets[index] = Vec2i::new(frame.sprite_source_size.x, frame.sprite_source_size.y);
       }
    */
    todo!()
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct BitmapIndex {
    frameindex: usize,
    layerindex: usize,
}

pub fn parse_aseprite_file(filepath: &str) -> Result<Aseprite, String> {
    let data = read_file_whole(filepath)?;
    let aseprite = ase::Aseprite::from_read(&mut Cursor::new(data))
        .map_err(|error| format!("Could not parse aseprite file '{}': {}", filepath, error))?;

    let out = dformat_pretty!(&aseprite);
    std::fs::write(&format!("{}.txt", filepath), out).unwrap();

    let sprite_width = aseprite.header.width_in_pixels as usize;
    let sprite_height = aseprite.header.height_in_pixels as usize;

    let mut frame_durations_ms = Vec::new();
    let mut layers = Vec::new();
    let mut current_layer_index = 0;
    let mut animation_tags = HashMap::new();
    let mut bitmap_links = HashMap::new();
    let mut bitmaps = HashMap::new();
    for (current_frame_index, frame) in aseprite.frames.iter().enumerate() {
        frame_durations_ms.push(frame.frame_duration_milliseconds as u32);
        for (current_chunk_index, chunk) in frame.chunks.iter().enumerate() {
            match &chunk.chunk_data {
                ase::ChunkData::LayerChunk(layer_chunk) => {
                    use ase::layer_chunk::Flags;
                    let layer = Layer {
                        name: layer_chunk.layer_name.clone(),
                        index: current_layer_index,
                        is_visible: layer_chunk.flags & Flags::Visible == Flags::Visible,
                    };
                    layers.push(layer);
                    current_layer_index += 1;
                }
                ase::ChunkData::FrameTagsChunk(tags_chunk) => {
                    for tag in &tags_chunk.tags {
                        let loop_direction = match tag.loop_animation_direction {
                            ase::LoopAnimationDirection::Forward => "forward",
                            ase::LoopAnimationDirection::Reverse => "reverse",
                            ase::LoopAnimationDirection::PingPong => "pingpong",
                        }
                        .to_owned();

                        animation_tags.insert(
                            tag.tag_name.clone(),
                            AnimationTag {
                                name: tag.tag_name.clone(),
                                frameindex_start: tag.from_tag as usize,
                                frameindex_end: tag.to_tag as usize,
                                loop_direction,
                            },
                        );
                    }
                }
                ase::ChunkData::CelChunk(cel_chunk) => {
                    let layerindex = cel_chunk.layer_index as usize;
                    let mut bitmap = Bitmap::new(sprite_width as u32, sprite_height as u32);
                    // cel_chunk.opacity_level
                    // cel_chunk.x_position
                    // cel_chunk.y_position
                    let has_pixels = match cel_chunk.cel {
                        ase::Cel::LinkedCel { frame_position } => {
                            bitmap_links.insert(
                                BitmapIndex {
                                    frameindex: current_frame_index as usize,
                                    layerindex,
                                },
                                BitmapIndex {
                                    frameindex: frame_position as usize,
                                    layerindex,
                                },
                            );
                            false
                        }
                        _ => true,
                    };

                    if has_pixels {
                        let frame_bitmap = Bitmap::new(sprite_width, sprite_height);
                        Bitmap::new_from_bytes(sprite_width, sprite_height, bytes);
                        let width = cel_chunk.cel.w().unwrap() as usize;
                        let height = cel_chunk.cel.h().unwrap() as usize;
                        let pixels = match cel_chunk.cel.pixels(&ase::ColorDepth::RGBA).unwrap() {
                            ase::Pixels::RGBA(pixels) => pixels,
                            _ => unreachable!(),
                        };
                        bitmaps.insert(
                            BitmapIndex {
                                frameindex: current_frame_index as usize,
                                layerindex,
                            },
                            bitmap,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    Ok(Aseprite {
        width: sprite_width,
        height: sprite_height,
        frame_durations_ms,
        layers,
        animation_tags,
    })
}

pub fn run() -> Result<(), String> {
    let filepath = "assets/example/sprites/sorcy.ase";
    let aseprite = parse_aseprite_file(filepath)?;

    dbg!(aseprite);

    Ok(())
}
