use super::{
    Animationname, Animationname3D, AssetAnimation, AssetAnimation3D, AssetSprite, AssetSprite3D,
    Spritename, Spritename3D,
};

use ct_lib::bitmap::*;
use ct_lib::math::*;
use ct_lib::sprite::*;
use ct_lib::system;

use ct_lib::indexmap::IndexMap;
use ct_lib::serde_derive::Deserialize;
use ct_lib::serde_json;

use rayon::prelude::*;
use std::collections::HashSet;

pub fn create_sheet_animations(
    image_filepath: &str,
    sheet_name: &str,
    output_filepath_without_extension: &str,
) -> (
    IndexMap<Spritename, AssetSprite>,
    IndexMap<Spritename3D, AssetSprite3D>,
    IndexMap<Animationname, AssetAnimation>,
    IndexMap<Animationname3D, AssetAnimation3D>,
) {
    if image_filepath.ends_with("_3d.ase") {
        create_sheet_animations_3d(
            image_filepath,
            sheet_name,
            output_filepath_without_extension,
        )
    } else {
        let (sprites, animations) = create_sheet_animations_2d(
            image_filepath,
            sheet_name,
            output_filepath_without_extension,
        );
        (sprites, IndexMap::new(), animations, IndexMap::new())
    }
}

pub fn create_sheet_animations_3d(
    image_filepath: &str,
    sheet_name: &str,
    output_filepath_without_extension: &str,
) -> (
    IndexMap<Spritename, AssetSprite>,
    IndexMap<Spritename3D, AssetSprite3D>,
    IndexMap<Animationname, AssetAnimation>,
    IndexMap<Animationname3D, AssetAnimation3D>,
) {
    let stack_layer_count = {
        // NOTE: This block is mainly for validation
        let mut layers = Vec::new();
        for layer_name in &aseprite_list_layers_of_file(image_filepath) {
            if layer_name != "pivot"
                && layer_name != "attachment_0"
                && layer_name != "attachment_1"
                && layer_name != "attachment_2"
                && layer_name != "attachment_3"
            {
                let layer_index = layer_name.parse::<usize>().expect(&format!(
                    "Found layer named '{}' in 3D sprite '{}', expected layernumber.\n
            NOTE: 3D sprites only support layer names 0,1,2,.., `pivot` and 'attachment_*'",
                    image_filepath, layer_name
                ));
                layers.push(layer_index);
            }
        }
        assert!(
            !layers.is_empty(),
            "No layer found in 3D sprite '{}'",
            image_filepath
        );
        assert!(
            layers.len() > 1,
            "Expected more than one layer in 3D sprite '{}'",
            image_filepath
        );
        layers.sort();
        for index in 0..layers.len() {
            assert!(
                index == layers[index],
                "Layer {} in 3D sprite '{}' has index {} - expected index {}",
                index,
                image_filepath,
                layers[index],
                index
            );
        }
        layers.len()
    };

    // Split out each of the 3D sprites stack layers into their own files and process each
    // separately
    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    let mut result_animations: IndexMap<Animationname, AssetAnimation> = IndexMap::new();
    let sprites_and_animations: Vec<(
        IndexMap<Spritename, AssetSprite>,
        IndexMap<Animationname, AssetAnimation>,
    )> = (0..stack_layer_count)
        .into_par_iter()
        .map(|current_stack_layer| {
            let stack_layer_sheet_name =
                sheet_name.to_string() + "#" + &current_stack_layer.to_string();
            let stack_layer_output_filepath_without_extension = output_filepath_without_extension
                .to_string()
                + "#"
                + &current_stack_layer.to_string();
            let stack_layer_image_filepath =
                stack_layer_output_filepath_without_extension.clone() + ".ase";

            let mut command = String::from("aseprite") + " --batch ";
            for ignored_stack_layer in 0..stack_layer_count {
                if current_stack_layer != ignored_stack_layer {
                    command += &format!(" --ignore-layer \"{}\" ", ignored_stack_layer);
                }
            }
            command += &image_filepath;
            command += " --save-as ";
            command += &stack_layer_image_filepath;

            system::run_systemcommand_fail_on_error(&command, false);

            assert!(
                system::path_exists(&stack_layer_image_filepath),
                "Failed to generate 3D sprite stack layer for '{}' - '{}' is missing",
                image_filepath,
                stack_layer_image_filepath
            );

            create_sheet_animations_2d(
                &stack_layer_image_filepath,
                &stack_layer_sheet_name,
                &stack_layer_output_filepath_without_extension,
            )
        })
        .collect();
    for (sprites, animations) in sprites_and_animations {
        result_sprites.extend(sprites);
        result_animations.extend(animations);
    }

    // 3D-Sprites
    let mut result_sprites_3d: IndexMap<Spritename3D, AssetSprite3D> = IndexMap::new();
    assert!(
        result_sprites.len() % stack_layer_count == 0,
        "Sprite count {} is not a multiple of stack layer count {} in 3D sprite '{}'",
        result_sprites.len(),
        stack_layer_count,
        image_filepath
    );
    let frame_count_3d = result_sprites.len() / stack_layer_count;
    for frame_index in 0..frame_count_3d {
        let mut layer_sprite_names = Vec::new();
        for stack_layer in 0..stack_layer_count {
            let sprite_name_2d =
                sprite_name_for_frameindex_and_stack_layer(sheet_name, stack_layer, frame_index);
            layer_sprite_names.push(sprite_name_2d);
        }

        let sprite_name_3d = sprite_name_for_frameindex(sheet_name, frame_index);
        result_sprites_3d.insert(
            sprite_name_3d.clone(),
            AssetSprite3D {
                name: sprite_name_3d,
                layer_sprite_names,
            },
        );
    }

    // 3D-Animations
    let mut result_animations_3d: IndexMap<Animationname3D, AssetAnimation3D> = IndexMap::new();
    assert!(
        result_animations.len() % stack_layer_count == 0,
        "Animation count {} is not a multiple of stack layer count {} in 3D sprite '{}'",
        result_sprites.len(),
        stack_layer_count,
        image_filepath
    );
    let anim_count_3d = result_animations.len() / stack_layer_count;
    let mut tag_names = HashSet::new();
    for animation_name in result_animations.keys() {
        tag_names.insert(animation_name.rsplit(":").next().unwrap());
    }
    assert_eq!(
        tag_names.len(),
        anim_count_3d,
        "Animation count {} is not the same as the animation count {} in 3D sprite '{}'",
        tag_names.len(),
        anim_count_3d,
        image_filepath
    );
    for tag_name in &tag_names {
        let (framecount, frame_durations_ms, frame_indices) = {
            let animation_2d = &result_animations[&format!("{}#{}:{}", sheet_name, 0, tag_name)];
            (
                animation_2d.framecount,
                animation_2d.frame_durations_ms.clone(),
                animation_2d
                    .sprite_names
                    .iter()
                    .map(|sprite_name_2d| {
                        sprite_name_2d.rsplit(".").next().unwrap().parse().unwrap()
                    })
                    .collect::<Vec<usize>>(),
            )
        };

        let animation_name_3d = format!("{}:{}", sheet_name, tag_name);
        let sprite_names = frame_indices
            .iter()
            .map(|&frame_index| sprite_name_for_frameindex(sheet_name, frame_index))
            .collect();

        result_animations_3d.insert(
            animation_name_3d.clone(),
            AssetAnimation3D {
                name: animation_name_3d,
                framecount,
                sprite_names,
                frame_durations_ms,
            },
        );
    }

    (
        result_sprites,
        result_sprites_3d,
        result_animations,
        result_animations_3d,
    )
}

pub fn create_sheet_animations_2d(
    image_filepath: &str,
    sheet_name: &str,
    output_path_without_extension: &str,
) -> (
    IndexMap<Spritename, AssetSprite>,
    IndexMap<Animationname, AssetAnimation>,
) {
    let output_path_image = output_path_without_extension.to_string() + ".png";
    let output_path_meta = output_path_without_extension.to_string() + ".json";

    aseprite_run_sheet_packer(&image_filepath, &output_path_image, &output_path_meta);

    let metadata_string = std::fs::read_to_string(&output_path_meta).unwrap();
    let meta: AsepriteJSON = serde_json::from_str(&metadata_string).expect(&format!(
        "Failed to generate offset information for '{}' - Cannot parse metadata '{}'",
        image_filepath, output_path_meta
    ));

    let framecount = meta.frames.len();
    assert!(framecount > 0);

    // Check for translucent pixels
    let output_bitmap = Bitmap::from_png_file_or_panic(&output_path_image);
    let has_translucency = output_bitmap
        .data
        .iter()
        .any(|pixel| pixel.a != 255 && pixel.a != 0);

    if has_translucency {
        println!("Translucent spritesheet detected: '{}'", image_filepath);
    }

    // Collect offsets
    let mut offsets_pivot = vec![Vec2i::zero(); framecount];
    let mut offsets_attachment_0 = vec![Vec2i::zero(); framecount];
    let mut offsets_attachment_1 = vec![Vec2i::zero(); framecount];
    let mut offsets_attachment_2 = vec![Vec2i::zero(); framecount];
    let mut offsets_attachment_3 = vec![Vec2i::zero(); framecount];
    {
        let layers = aseprite_list_layers_of_file(&image_filepath);

        let output_path_pivots_image = output_path_without_extension.to_string() + "_pivots.png";
        let output_path_pivots_meta = output_path_without_extension.to_string() + "_pivots.json";

        let output_path_attachment_0_image =
            output_path_without_extension.to_string() + "_attachment_0.png";
        let output_path_attachment_0_meta =
            output_path_without_extension.to_string() + "_attachment_0.json";
        let output_path_attachment_1_image =
            output_path_without_extension.to_string() + "_attachment_1.png";
        let output_path_attachment_1_meta =
            output_path_without_extension.to_string() + "_attachment_1.json";
        let output_path_attachment_2_image =
            output_path_without_extension.to_string() + "_attachment_2.png";
        let output_path_attachment_2_meta =
            output_path_without_extension.to_string() + "_attachment_2.json";
        let output_path_attachment_3_image =
            output_path_without_extension.to_string() + "_attachment_3.png";
        let output_path_attachment_3_meta =
            output_path_without_extension.to_string() + "_attachment_3.json";

        for layername in layers {
            if layername == "pivot" {
                aseprite_get_offsets_for_layer(
                    &image_filepath,
                    &output_path_pivots_image,
                    &output_path_pivots_meta,
                    "pivot",
                    &mut offsets_pivot,
                );
            } else if layername == "attachment_0" {
                aseprite_get_offsets_for_layer(
                    &image_filepath,
                    &output_path_attachment_0_image,
                    &output_path_attachment_0_meta,
                    "attachment_0",
                    &mut offsets_attachment_0,
                );
            } else if layername == "attachment_1" {
                aseprite_get_offsets_for_layer(
                    &image_filepath,
                    &output_path_attachment_1_image,
                    &output_path_attachment_1_meta,
                    "attachment_1",
                    &mut offsets_attachment_1,
                );
            } else if layername == "attachment_2" {
                aseprite_get_offsets_for_layer(
                    &image_filepath,
                    &output_path_attachment_2_image,
                    &output_path_attachment_2_meta,
                    "attachment_2",
                    &mut offsets_attachment_2,
                );
            } else if layername == "attachment_3" {
                aseprite_get_offsets_for_layer(
                    &image_filepath,
                    &output_path_attachment_3_image,
                    &output_path_attachment_3_meta,
                    "attachment_3",
                    &mut offsets_attachment_3,
                );
            }
        }
    }

    // Create sprites
    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    for (frame_index, frame) in meta.frames.iter().enumerate() {
        let sprite_name = sprite_name_for_frameindex(&sheet_name, frame_index);

        let attachment_points = [
            offsets_attachment_0[frame_index],
            offsets_attachment_1[frame_index],
            offsets_attachment_2[frame_index],
            offsets_attachment_3[frame_index],
        ];
        let new_sprite = sprite_create_from_frame_metadata(
            &sprite_name,
            has_translucency,
            offsets_pivot[frame_index],
            attachment_points,
            frame,
        );
        result_sprites.insert(sprite_name, new_sprite);
    }

    // Create animation tags
    let frametags: Vec<FrameTag> = if meta.meta.frame_tags.len() == 0 {
        // If we have no animation tags we treat the whole frame-range as one big tagless animation
        vec![FrameTag {
            name: "default".to_string(),
            from: 0,
            to: (framecount as i32 - 1),
            direction: "forward".to_string(),
        }]
    } else {
        meta.meta.frame_tags.clone()
    };

    // Create animations
    let mut result_animations: IndexMap<Animationname, AssetAnimation> = IndexMap::new();
    for frametag in frametags {
        let animation_name = sheet_name.to_string() + ":" + &frametag.name;

        let mut sprite_names: Vec<Spritename> = Vec::new();
        let mut frame_durations_ms: Vec<u32> = Vec::new();
        for frame_index in frametag.from..=frametag.to {
            let sprite_name = sprite_name_for_frameindex(&sheet_name, frame_index as usize);
            sprite_names.push(sprite_name);
            frame_durations_ms.push(meta.frames[frame_index as usize].duration);
        }

        let new_animation = AssetAnimation {
            name: animation_name.clone(),
            framecount: sprite_names.len() as u32,
            sprite_names,
            frame_durations_ms,
        };

        result_animations.insert(animation_name, new_animation);
    }

    (result_sprites, result_animations)
}

fn sprite_name_for_frameindex(sheet_name: &str, frame_index: usize) -> Spritename {
    format!("{}.{}", sheet_name, frame_index)
}

fn sprite_name_for_frameindex_and_stack_layer(
    sheet_name: &str,
    stack_layer_index: usize,
    frame_index: usize,
) -> Spritename {
    format!("{}#{}.{}", sheet_name, stack_layer_index, frame_index)
}

fn aseprite_run_sheet_packer(
    image_filepath: &str,
    output_filepath_image: &str,
    output_filepath_meta: &str,
) {
    let command = String::from("aseprite")
        + " --batch"
        + " --list-layers"
        + " --list-tags"
        + " --ignore-layer"
        + " \"pivot\""
        + " --ignore-layer"
        + " \"attachment_0\""
        + " --ignore-layer"
        + " \"attachment_1\""
        + " --ignore-layer"
        + " \"attachment_2\""
        + " --ignore-layer"
        + " \"attachment_3\""
        + " --format"
        + " \"json-array\""
        + " --sheet-pack"
        + " --trim "
        + image_filepath
        + " --color-mode rgb "
        + " --sheet "
        + output_filepath_image
        + " --data "
        + output_filepath_meta;
    system::run_systemcommand_fail_on_error(&command, false);

    assert!(
        system::path_exists(&output_filepath_image),
        "Failed to generate sprite sheet for '{}' - '{}' is missing",
        image_filepath,
        output_filepath_image
    );
    assert!(
        system::path_exists(&output_filepath_meta),
        "Failed to generate sprite sheet for '{}' - '{}' is missing",
        image_filepath,
        output_filepath_meta
    );
}

fn aseprite_get_offsets_for_layer(
    image_filepath: &str,
    output_filepath_image: &str,
    output_filepath_meta: &str,
    layer_name: &str,
    out_offsets: &mut Vec<Vec2i>,
) {
    let framecount = out_offsets.len();
    assert!(framecount > 0);

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
    system::run_systemcommand_fail_on_error(&command, false);

    assert!(
        system::path_exists(&output_filepath_image),
        "Failed to generate offset information for '{}' - '{}' is missing",
        image_filepath,
        output_filepath_image
    );
    assert!(
        system::path_exists(&output_filepath_meta),
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
}

fn aseprite_list_layers_of_file(file_path: &str) -> Vec<String> {
    let command = String::from("aseprite ") + " --batch" + " --list-layers " + file_path;
    let command_stdout = system::run_systemcommand_fail_on_error(&command, false).stdout;
    command_stdout.lines().map(|line| line.to_owned()).collect()
}

fn sprite_create_from_frame_metadata(
    sprite_name: &str,
    has_translucency: bool,
    pivot_offset: Vec2i,
    attachment_points: [Vec2i; SPRITE_ATTACHMENT_POINTS_MAX_COUNT],
    frame: &Frame,
) -> AssetSprite {
    let (trimmed_rect, trimmed_uvs) = if frame.sprite_source_size.w == 0
        && frame.sprite_source_size.h == 0
    {
        // NOTE: The sprite is zero sized. This is useful for example if a character has an
        //       animation where it can be invisible in one frame
        (
            Recti::from_width_height(0, 0),
            Recti::from_width_height(0, 0),
        )
    } else {
        (
            Recti::from_xy_width_height(
                frame.sprite_source_size.x,
                frame.sprite_source_size.y,
                frame.frame.w,
                frame.frame.h,
            ),
            Recti::from_xy_width_height(frame.frame.x, frame.frame.y, frame.frame.w, frame.frame.h),
        )
    };

    // NOTE: The `atlas_texture_index` and the `trimmed_rect_uv` will be adjusted later when we
    // actually pack the sprites into atlas textures
    AssetSprite {
        name: sprite_name.to_owned(),

        has_translucency,
        atlas_texture_index: std::u32::MAX,

        pivot_offset: pivot_offset,

        attachment_points: attachment_points,

        untrimmed_dimensions: Vec2i::new(frame.source_size.w, frame.source_size.h),

        trimmed_rect,
        trimmed_uvs,
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Generated JSON structs

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct AsepriteJSON {
    frames: Vec<Frame>,
    meta: Meta,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Frame {
    filename: String,
    frame: Frame2,
    rotated: bool,
    trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: SpriteSourceSize,
    #[serde(rename = "sourceSize")]
    source_size: SourceSize,
    duration: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Frame2 {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct SpriteSourceSize {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct SourceSize {
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Meta {
    app: String,
    version: String,
    image: String,
    format: String,
    size: Size,
    scale: String,
    #[serde(rename = "frameTags")]
    frame_tags: Vec<FrameTag>,
    layers: Vec<Layer>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Size {
    w: i32,
    h: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct FrameTag {
    name: String,
    from: i32,
    to: i32,
    direction: String,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Layer {
    name: String,
    opacity: i32,
    #[serde(rename = "blendMode")]
    blend_mode: String,
}
