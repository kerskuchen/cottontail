mod aseprite;

use audio::write_audio_samples_to_wav_file;
use ct_lib_audio as audio;
use ct_lib_core as core;
use ct_lib_draw as draw;
use ct_lib_game as game;
use ct_lib_image as image;
use ct_lib_math as math;

use crate::core::indexmap::indexmap;
use crate::core::indexmap::IndexMap;
use crate::core::serde_derive::{Deserialize, Serialize};
use crate::core::*;

use draw::*;
use game::*;
use image::*;
use math::*;

use rayon::prelude::*;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

type Imagename = String;
type Spritename = String;
type Spritename3D = String;
type Fontname = String;
type Animationname = String;
type Animationname3D = String;

#[derive(Debug, Clone, Serialize)]
pub struct AssetSprite {
    pub name: Spritename,
    pub atlas_texture_index: TextureIndex,
    pub has_translucency: bool,

    pub pivot_offset: Vec2i,
    pub attachment_points: [Vec2i; SPRITE_ATTACHMENT_POINTS_MAX_COUNT],

    pub untrimmed_dimensions: Vec2i,

    pub trimmed_rect: Recti,
    pub trimmed_uvs: Recti,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetSprite3D {
    pub name: Spritename3D,
    pub layer_sprite_names: Vec<Spritename>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation {
    pub name: Animationname,
    pub framecount: u32,
    pub sprite_names: Vec<Spritename>,
    pub frame_durations_ms: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation3D {
    pub name: Animationname3D,
    pub framecount: u32,
    pub sprite_names: Vec<Spritename3D>,
    pub frame_durations_ms: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetGlyph {
    pub codepoint: Codepoint,
    pub sprite_name: Spritename,

    pub horizontal_advance: i32,

    /// This is mainly used for text dimension calculations
    pub sprite_dimensions: Vec2i,
    /// This is mainly used for text dimension calculations
    pub sprite_draw_offset: Vec2i,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct AssetFont {
    pub name: Fontname,
    pub baseline: i32,
    pub vertical_advance: i32,
    pub horizontal_advance_max: i32,
    pub font_height_in_pixels: i32,
    pub glyphcount: u32,
    pub glyphs: IndexMap<Codepoint, AssetGlyph>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAtlas {
    pub texture_size: u32,
    pub texture_count: u32,
    pub texture_imagepaths: Vec<String>,
    pub sprite_positions: IndexMap<Spritename, BitmapAtlasPosition>,
}

fn bake_graphics_resources() {
    let mut result_sheet: GraphicsSheet = GraphicsSheet::new_empty();

    // Create fonts and its correspronding sprites
    let font_styles = load_font_styles();
    let font_properties = load_font_properties();
    let font_sheets: Vec<GraphicsSheet> = font_styles
        .par_iter()
        .map(|style| {
            let properties = font_properties.get(&style.fontname).expect(&format!(
                "No font and/or render parameters found for font name '{}'",
                &style.fontname
            ));
            create_sheet_from_ttf(
                &style.fontname,
                &properties.ttf_data_bytes,
                "target/assets_temp",
                properties.render_params.height_in_pixels,
                properties.render_params.raster_offset,
                style.bordered,
                style.color_glyph,
                style.color_border,
            )
        })
        .collect();
    for sheet in font_sheets {
        result_sheet.extend_by(sheet);
    }

    // Convert png and aseprite files to png sheets and move to them to `target/assets_temp`
    let sprite_sheets: Vec<GraphicsSheet> = {
        let mut imagepaths = vec![];
        imagepaths.append(&mut collect_files_by_extension_recursive("assets", ".ase"));
        imagepaths.append(&mut collect_files_by_extension_recursive("assets", ".png"));
        imagepaths
            .par_iter()
            .map(|imagepath| {
                let sheet_name = path_without_extension(imagepath).replace("assets/", "");
                let output_path_without_extension =
                    path_without_extension(imagepath).replace("assets", "target/assets_temp");
                aseprite::create_sheet(imagepath, &sheet_name, &output_path_without_extension)
            })
            .collect()
    };
    for sheet in sprite_sheets {
        result_sheet.extend_by(sheet)
    }

    result_sheet.pack_and_serialize();
}

fn bake_audio_resources(resource_pack_name: &str, audio_sample_rate_hz: usize) {
    let mut audio_resources = AudioResources::new(audio_sample_rate_hz);

    for filepath in &collect_files_recursive("assets") {
        if !filepath.ends_with(".wav") && !filepath.ends_with(".ogg") {
            continue;
        }

        let resource_name = path_without_extension(filepath).replace("assets/", "");
        let metadata_path = path_without_extension(filepath) + ".meta.json";
        let recreate_metadata = if path_exists(&metadata_path) {
            let last_modified_time = path_last_modified_time(filepath);
            let metadata_last_modified_time = path_last_modified_time(&metadata_path);
            metadata_last_modified_time < last_modified_time
        } else {
            true
        };

        let metadata_original = if recreate_metadata {
            let (samplerate_hz, audiochunk) =
                audio::decode_audio_file(filepath).unwrap_or_else(|error| {
                    panic!("Cannot decode audio file '{}': {}", filepath, error)
                });
            let channelcount = audiochunk.channelcount();
            let framecount = audiochunk.len();

            let original = AudioMetadata {
                original_filepath: filepath.to_owned(),
                resource_name: resource_name.clone(),
                samplerate_hz,
                channelcount,
                framecount,
                compression_quality: 5,
                loopsection_start_frameindex: None,
                loopsection_framecount: None,
            };
            serialize_to_json_file(&original, &metadata_path);
            original
        } else {
            deserialize_from_json_file(&metadata_path)
        };

        let metadata = metadata_original.clone_with_new_sample_rate(audio_sample_rate_hz);

        let ogg_data = if filepath.ends_with(".ogg") {
            // Check if we need to resample our ogg file
            if metadata.samplerate_hz == metadata_original.samplerate_hz {
                read_file_whole(filepath).unwrap_or_else(|error| {
                    panic!("Cannot open ogg file '{}': {}", filepath, error)
                })
            } else {
                let (original_samplerate_hz, audiochunk) = audio::decode_audio_file(filepath)
                    .unwrap_or_else(|error| {
                        panic!("Cannot decode audio file '{}': {}", filepath, error)
                    });
                let wav_output_temp_path = path_with_extension(&filepath, ".wav").replace(
                    "assets",
                    &format!("target/assets_temp/{}", resource_pack_name),
                );
                write_audio_samples_to_wav_file(
                    &wav_output_temp_path,
                    &audiochunk,
                    original_samplerate_hz,
                );

                let ogg_output_temp_path = filepath.replace(
                    "assets",
                    &format!("target/assets_temp/{}", resource_pack_name),
                );
                let command = format!(
                    "oggenc2 {} --quality {} --resample {} --converter 0 --output={}",
                    wav_output_temp_path,
                    metadata.compression_quality,
                    audio_sample_rate_hz,
                    ogg_output_temp_path
                );
                run_systemcommand_fail_on_error(&command, false);

                assert!(
                    path_exists(&ogg_output_temp_path),
                    "Failed to encode ogg file for '{}' - '{}' is missing",
                    filepath,
                    ogg_output_temp_path
                );

                read_file_whole(&ogg_output_temp_path).unwrap_or_else(|error| {
                    panic!("Cannot open ogg file '{}': {}", ogg_output_temp_path, error)
                })
            }
        } else {
            // Create resampled ogg file out of our wav file
            let ogg_output_temp_path = path_with_extension(&filepath, ".ogg").replace(
                "assets",
                &format!("target/assets_temp/{}", resource_pack_name),
            );
            let command = format!(
                "oggenc2 {} --quality {} --resample {} --converter 0 --output={}",
                filepath, metadata.compression_quality, audio_sample_rate_hz, ogg_output_temp_path
            );
            run_systemcommand_fail_on_error(&command, false);

            assert!(
                path_exists(&ogg_output_temp_path),
                "Failed to encode ogg file for '{}' - '{}' is missing",
                filepath,
                ogg_output_temp_path
            );

            read_file_whole(&ogg_output_temp_path).unwrap_or_else(|error| {
                panic!("Cannot open ogg file '{}': {}", ogg_output_temp_path, error)
            })
        };

        audio_resources.add_audio_resource(resource_name, metadata_original, metadata, ogg_data);
    }

    serialize_to_binary_file(
        &audio_resources,
        &format!("resources/{}.data", resource_pack_name,),
    );

    // Human readable
    serialize_to_json_file(
        &audio_resources.metadata,
        &format!(
            "target/assets_temp/{}/{}.json",
            resource_pack_name, resource_pack_name
        ),
    );
    // Copy ogg files to a human readable location
    for resource_name in &audio_resources.names {
        let metadata = audio_resources.metadata.get(resource_name).unwrap();
        let metadata_original = audio_resources
            .metadata_original
            .get(resource_name)
            .unwrap();

        let path = &metadata.original_filepath;
        if path.ends_with(".wav") {
            // Wav files are converted to ogg files and which will be already placed in temp folder
            continue;
        }

        if metadata.samplerate_hz != metadata_original.samplerate_hz {
            // We resampled our .ogg file and already placed it into the temp folder
            continue;
        }

        let temp_path = path.replace(
            "assets",
            &format!("target/assets_temp/{}", resource_pack_name),
        );
        path_copy_file(path, &temp_path);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Font properties and styles

#[derive(Deserialize)]
pub struct BitmapFontStyle {
    pub fontname: String,
    pub bordered: bool,
    pub color_glyph: PixelRGBA,
    pub color_border: PixelRGBA,
}

#[derive(Deserialize)]
pub struct BitmapFontRenderParams {
    pub height_in_pixels: i32,
    pub raster_offset: Vec2,
}

pub struct BitmapFontProperties {
    pub ttf_data_bytes: Vec<u8>,
    pub render_params: BitmapFontRenderParams,
}

fn load_font_properties() -> IndexMap<Fontname, BitmapFontProperties> {
    let mut result_properties = IndexMap::new();

    let font_filepaths = collect_files_by_extension_recursive("assets/fonts", ".ttf");
    for font_filepath in font_filepaths {
        let font_name = path_to_filename_without_extension(&font_filepath);
        let renderparams_filepath = path_with_extension(&font_filepath, "json");
        let ttf_data_bytes = std::fs::read(&font_filepath)
            .expect(&format!("Cannot read fontdata '{}'", &font_filepath));

        // NOTE: We only read the fontdata when the renderparams exist but don't throw an error when
        //       it does not exist. This helps us make test renders for a font before we have found
        //       out its correct render params.
        if let Some(params_string) = std::fs::read_to_string(&renderparams_filepath).ok() {
            let render_params: BitmapFontRenderParams = serde_json::from_str(&params_string)
                .expect(&format!(
                    "Cannot read render parameters for font: '{}'",
                    &font_filepath
                ));

            result_properties.insert(
                font_name,
                BitmapFontProperties {
                    ttf_data_bytes,
                    render_params,
                },
            );
        } else {
            let test_png_filepath = path_join(
                "target/assets_temp",
                &(font_name.clone() + "_fontsize_test.png"),
            );
            log::warn!(
                "Font is missing its render parameters: '{}' - Created font size test image at '{}'",
                &font_filepath,
                &test_png_filepath
            );
            let test_png_filepath = path_join(
                "target/assets_temp",
                &(font_name.clone() + "_fontsize_test_offset_-0.5.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data_bytes,
                Vec2::new(0.0, -0.5),
                4,
                32,
                &test_png_filepath,
            );
            let test_png_filepath = path_join(
                "target/assets_temp",
                &(font_name.clone() + "_fontsize_test_offset_0.0.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data_bytes,
                Vec2::new(0.0, 0.0),
                4,
                32,
                &test_png_filepath,
            );
            let test_png_filepath = path_join(
                "target/assets_temp",
                &(font_name.clone() + "_fontsize_test_offset_0.5.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data_bytes,
                Vec2::new(0.0, 0.5),
                4,
                32,
                &test_png_filepath,
            );
        }
    }

    // Add default fonts
    result_properties.insert(
        font::FONT_DEFAULT_TINY_NAME.to_owned(),
        BitmapFontProperties {
            ttf_data_bytes: font::FONT_DEFAULT_TINY_TTF.to_vec(),
            render_params: BitmapFontRenderParams {
                height_in_pixels: font::FONT_DEFAULT_TINY_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_TINY_RASTER_OFFSET,
            },
        },
    );
    result_properties.insert(
        font::FONT_DEFAULT_SMALL_NAME.to_owned(),
        BitmapFontProperties {
            ttf_data_bytes: font::FONT_DEFAULT_SMALL_TTF.to_vec(),
            render_params: BitmapFontRenderParams {
                height_in_pixels: font::FONT_DEFAULT_SMALL_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_SMALL_RASTER_OFFSET,
            },
        },
    );
    result_properties.insert(
        font::FONT_DEFAULT_REGULAR_NAME.to_owned(),
        BitmapFontProperties {
            ttf_data_bytes: font::FONT_DEFAULT_REGULAR_TTF.to_vec(),
            render_params: BitmapFontRenderParams {
                height_in_pixels: font::FONT_DEFAULT_REGULAR_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_REGULAR_RASTER_OFFSET,
            },
        },
    );
    result_properties.insert(
        font::FONT_DEFAULT_SQUARE_NAME.to_owned(),
        BitmapFontProperties {
            ttf_data_bytes: font::FONT_DEFAULT_SQUARE_TTF.to_vec(),
            render_params: BitmapFontRenderParams {
                height_in_pixels: font::FONT_DEFAULT_SQUARE_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_SQUARE_RASTER_OFFSET,
            },
        },
    );

    result_properties
}

fn load_font_styles() -> Vec<BitmapFontStyle> {
    // Load styles from styles file
    let mut result_styles: Vec<BitmapFontStyle> = {
        let font_styles_filepath = "assets/fonts/font_styles.json";
        let font_styles_str = std::fs::read_to_string(font_styles_filepath).expect(&format!(
            "Missing font styles file '{}'",
            font_styles_filepath
        ));
        serde_json::from_str(&font_styles_str).expect(&format!(
            "Cannot read font styles file '{}'",
            font_styles_filepath
        ))
    };

    // Add default fonts styles
    let default_color_glyph = PixelRGBA::new(255, 255, 255, 255);
    let default_color_border = PixelRGBA::new(0, 0, 0, 255);
    result_styles.push(BitmapFontStyle {
        fontname: font::FONT_DEFAULT_TINY_NAME.to_owned(),
        bordered: false,
        color_glyph: default_color_glyph,
        color_border: default_color_border,
    });
    result_styles.push(BitmapFontStyle {
        fontname: font::FONT_DEFAULT_TINY_NAME.to_owned(),
        bordered: true,
        color_glyph: default_color_glyph,
        color_border: default_color_border,
    });

    result_styles
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Font packing

fn sprite_create_from_glyph(
    sprite_name: &str,
    glyph: &BitmapGlyph,
    position_in_font_atlas: Option<Vec2i>,
) -> AssetSprite {
    let glyph_rect = glyph.get_bitmap_rect();
    let glyph_atlas_pos = if let Some(pos) = position_in_font_atlas {
        pos
    } else {
        Vec2i::zero()
    };

    // NOTE: The `atlas_texture_index` and the `trimmed_rect_uv` will be adjusted later when we
    // actually pack the sprites into atlas textures
    AssetSprite {
        name: sprite_name.to_owned(),

        has_translucency: false,

        atlas_texture_index: std::u32::MAX,

        pivot_offset: Vec2i::zero(),

        attachment_points: [Vec2i::zero(); SPRITE_ATTACHMENT_POINTS_MAX_COUNT],

        untrimmed_dimensions: glyph_rect.dim,

        trimmed_rect: glyph_rect,

        trimmed_uvs: Recti::from_xy_width_height(
            glyph_atlas_pos.x,
            glyph_atlas_pos.y,
            glyph_rect.width(),
            glyph_rect.height(),
        ),
    }
}

fn create_sheet_from_ttf(
    font_name: &str,
    font_ttf_bytes: &[u8],
    output_dir: &str,
    height_in_pixels: i32,
    raster_offset: Vec2,
    draw_border: bool,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> GraphicsSheet {
    let font_name = font_name.to_owned() + if draw_border { "_bordered" } else { "" };

    let output_filepath_without_extension = path_join(output_dir, &font_name);
    let output_filepath_png = output_filepath_without_extension.to_owned() + ".png";

    let border_thickness = if draw_border { 1 } else { 0 };

    // Create font and atlas
    let font = BitmapFont::new(
        &font_name,
        &font_ttf_bytes,
        height_in_pixels,
        raster_offset,
        border_thickness,
        0,
        color_glyph,
        color_border,
    );
    let (font_atlas_texture, font_atlas_glyph_positions) = font.to_bitmap_atlas(&font_name);
    Bitmap::write_to_png_file(&font_atlas_texture, &output_filepath_png);

    // Create sprites and glyphs
    let mut result_glyphs: IndexMap<Codepoint, AssetGlyph> = IndexMap::new();
    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    for glyph in font.glyphs.values() {
        let codepoint = glyph.codepoint as Codepoint;
        let sprite_name = BitmapFont::get_glyph_name(&font_name, glyph.codepoint as Codepoint);
        let sprite_pos = font_atlas_glyph_positions.get(&sprite_name).cloned();
        let sprite = sprite_create_from_glyph(&sprite_name, glyph, sprite_pos);

        let asset_glyph = AssetGlyph {
            codepoint,
            sprite_name: sprite_name.clone(),
            horizontal_advance: glyph.horizontal_advance,
            sprite_dimensions: sprite.trimmed_rect.dim,
            sprite_draw_offset: sprite.trimmed_rect.pos,
        };

        result_glyphs.insert(codepoint, asset_glyph);
        result_sprites.insert(sprite_name, sprite);
    }

    // Create Font
    let result_font = AssetFont {
        name: font_name.clone(),
        baseline: font.baseline,
        vertical_advance: font.vertical_advance,
        horizontal_advance_max: font.horizontal_advance_max,
        font_height_in_pixels: font.font_height_in_pixels,
        glyphcount: result_glyphs.len() as u32,
        glyphs: result_glyphs,
    };

    GraphicsSheet {
        images: indexmap! { font_name.clone() => font_atlas_texture, },
        fonts: indexmap! { font_name.clone() => result_font, },
        sprites: result_sprites,
        sprites_3d: IndexMap::new(),
        animations: IndexMap::new(),
        animations_3d: IndexMap::new(),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atlas packing

pub fn atlas_create_from_images(
    images: &IndexMap<Imagename, Bitmap>,
    output_dir: &str,
    atlas_texture_size: u32,
) -> AssetAtlas {
    let (atlas_textures, result_sprite_positions) = {
        let mut packer = BitmapMultiAtlas::new(atlas_texture_size);
        for (image_name, image) in images.iter() {
            packer.pack_bitmap(image_name, image);
        }
        packer.finish()
    };

    // Write textures to disk
    let result_texture_imagepaths = {
        let atlas_path_without_extension = path_join(output_dir, "atlas");
        let mut texture_imagepaths = Vec::new();
        for (index, atlas_texture) in atlas_textures.iter().enumerate() {
            let texture_path = format!("{}-{}.png", atlas_path_without_extension, index);
            Bitmap::write_to_png_file(&atlas_texture.to_premultiplied_alpha(), &texture_path);

            // NOTE: We assume that our atlas textures will be located at the root of our final destination,
            //       so we drop the prefix
            let texture_path_shortened = texture_path.replace("resources/", "");
            texture_imagepaths.push(texture_path_shortened);
        }
        texture_imagepaths
    };

    AssetAtlas {
        texture_size: atlas_texture_size,
        texture_count: result_texture_imagepaths.len() as u32,
        texture_imagepaths: result_texture_imagepaths,
        sprite_positions: result_sprite_positions,
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Asset conversion

fn convert_sprite(sprite: &AssetSprite, atlas_texture_size: u32) -> Sprite {
    let attachment_points = [
        Vec2::from(sprite.attachment_points[0]),
        Vec2::from(sprite.attachment_points[1]),
        Vec2::from(sprite.attachment_points[2]),
        Vec2::from(sprite.attachment_points[3]),
    ];
    Sprite {
        name: sprite.name.clone(),
        atlas_texture_index: sprite.atlas_texture_index,

        has_translucency: sprite.has_translucency,

        pivot_offset: Vec2::from(sprite.pivot_offset),
        attachment_points: attachment_points,

        untrimmed_dimensions: Vec2::from(sprite.untrimmed_dimensions),
        trimmed_rect: Rect::from(sprite.trimmed_rect),
        trimmed_uvs: AAQuad::from_rect(
            Rect::from(sprite.trimmed_uvs)
                .scaled_from_origin(Vec2::filled(1.0 / atlas_texture_size as f32)),
        ),
    }
}

fn convert_sprite_3d(
    sprite: &AssetSprite3D,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) -> Sprite3D {
    let layers = sprite
        .layer_sprite_names
        .iter()
        .map(|name| final_sprites_by_name[name].clone())
        .collect();
    Sprite3D {
        name: sprite.name.clone(),
        layers,
    }
}

fn convert_glyph(
    glyph: &AssetGlyph,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) -> SpriteGlyph {
    SpriteGlyph {
        horizontal_advance: glyph.horizontal_advance,
        sprite: final_sprites_by_name[&glyph.sprite_name].clone(),
        sprite_dimensions: glyph.sprite_dimensions,
        sprite_draw_offset: glyph.sprite_draw_offset,
    }
}

fn convert_font(
    font: &AssetFont,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) -> SpriteFont {
    let mut ascii_glyphs: Vec<SpriteGlyph> =
        vec![SpriteGlyph::default(); FONT_MAX_NUM_FASTPATH_CODEPOINTS];
    let mut unicode_glyphs: HashMap<Codepoint, SpriteGlyph> = HashMap::new();

    for glyph in font.glyphs.values() {
        let codepoint = glyph.codepoint;
        let converted_glyph = convert_glyph(glyph, final_sprites_by_name);
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            ascii_glyphs[codepoint as usize] = converted_glyph;
        } else {
            unicode_glyphs.insert(codepoint, converted_glyph);
        }
    }

    SpriteFont {
        name: font.name.clone(),
        baseline: font.baseline,
        vertical_advance: font.vertical_advance,
        horizontal_advance_max: font.horizontal_advance_max,
        font_height_in_pixels: font.font_height_in_pixels,
        ascii_glyphs,
        unicode_glyphs,
    }
}

fn convert_animation(
    anim: &AssetAnimation,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) -> Animation<Sprite> {
    assert!(anim.sprite_names.len() == anim.frame_durations_ms.len());
    let mut anim_result = Animation::new_empty(anim.name.clone());
    for (&frame_duration_ms, sprite_name) in
        anim.frame_durations_ms.iter().zip(anim.sprite_names.iter())
    {
        anim_result.add_frame(
            frame_duration_ms as f32 / 1000.0,
            final_sprites_by_name[sprite_name].clone(),
        );
    }
    anim_result
}

fn convert_animation_3d(
    anim_3d: &AssetAnimation3D,
    final_sprites_by_name_3d: &IndexMap<Spritename3D, Sprite3D>,
) -> Animation<Sprite3D> {
    assert!(anim_3d.sprite_names.len() == anim_3d.frame_durations_ms.len());
    let mut anim_result = Animation::new_empty(anim_3d.name.clone());
    for (&frame_duration_ms, sprite_name) in anim_3d
        .frame_durations_ms
        .iter()
        .zip(anim_3d.sprite_names.iter())
    {
        anim_result.add_frame(
            frame_duration_ms as f32 / 1000.0,
            final_sprites_by_name_3d[sprite_name].clone(),
        );
    }
    anim_result
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Serialization

fn serialize_sprites(
    sprite_map: &IndexMap<Spritename, AssetSprite>,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) {
    let human_readable: Vec<AssetSprite> = sprite_map.values().cloned().collect();
    std::fs::write(
        "target/assets_temp/graphics/sprites.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    std::fs::write(
        "resources/sprites.data",
        bincode::serialize(&final_sprites_by_name).unwrap(),
    )
    .unwrap();
}

fn serialize_sprites_3d(
    sprite_map_3d: &IndexMap<Spritename3D, AssetSprite3D>,
    final_sprites_by_name_3d: &IndexMap<Spritename3D, Sprite3D>,
) {
    let human_readable: Vec<AssetSprite3D> = sprite_map_3d.values().cloned().collect();
    std::fs::write(
        "target/assets_temp/graphics/sprites_3d.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    std::fs::write(
        "resources/sprites_3d.data",
        bincode::serialize(&final_sprites_by_name_3d).unwrap(),
    )
    .unwrap();
}

fn serialize_fonts(
    font_map: &IndexMap<Fontname, AssetFont>,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) {
    let human_readable: Vec<AssetFont> = font_map.values().cloned().collect();
    std::fs::write(
        "target/assets_temp/graphics/fonts.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: HashMap<String, SpriteFont> = font_map
        .iter()
        .map(|(name, font)| (name.clone(), convert_font(font, final_sprites_by_name)))
        .collect();
    std::fs::write("resources/fonts.data", bincode::serialize(&binary).unwrap()).unwrap();
}

fn serialize_animations(
    animation_map: &IndexMap<Animationname, AssetAnimation>,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) {
    let human_readable: Vec<AssetAnimation> = animation_map.values().cloned().collect();
    std::fs::write(
        "target/assets_temp/graphics/animations.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: HashMap<String, Animation<Sprite>> = animation_map
        .iter()
        .map(|(name, anim)| (name.clone(), convert_animation(anim, final_sprites_by_name)))
        .collect();
    std::fs::write(
        "resources/animations.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_animations_3d(
    animation_map_3d: &IndexMap<Animationname3D, AssetAnimation3D>,
    final_sprites_by_name_3d: &IndexMap<Spritename3D, Sprite3D>,
) {
    let human_readable: Vec<AssetAnimation3D> = animation_map_3d.values().cloned().collect();
    std::fs::write(
        "target/assets_temp/graphics/animations_3d.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: HashMap<String, Animation<Sprite3D>> = animation_map_3d
        .iter()
        .map(|(name, anim)| {
            (
                name.clone(),
                convert_animation_3d(anim, final_sprites_by_name_3d),
            )
        })
        .collect();
    std::fs::write(
        "resources/animations_3d.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_atlas(atlas: &AssetAtlas) {
    // Human readable
    std::fs::write(
        "target/assets_temp/graphics/atlas.json",
        serde_json::to_string_pretty(&atlas).unwrap(),
    )
    .unwrap();

    let binary: Vec<String> = atlas.texture_imagepaths.clone();
    std::fs::write("resources/atlas.data", bincode::serialize(&binary).unwrap()).unwrap();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Launcher icon

fn load_existing_launcher_icon_images(search_dir: &str) -> HashMap<i32, Bitmap> {
    let mut result = HashMap::new();
    let image_paths = collect_files_by_extension_recursive(search_dir, ".png");
    for image_path in &image_paths {
        let image = Bitmap::from_png_file_or_panic(image_path);
        let size = path_to_filename_without_extension(image_path)
            .parse()
            .expect(&format!(
                "Launcher icon name '{}' invalid, see README_ICONS.md",
                image_path,
            ));

        assert!(
            image.width == size && image.height == size,
            "Launcher icon name '{}' does not match its dimension ({}x{}), see README_ICONS.md",
            image_path,
            image.width,
            image.height
        );
        result.insert(size, image);
    }
    assert!(
        !result.is_empty(),
        "No launcher icons found at '{}'",
        search_dir
    );
    result
}

fn create_windows_launcher_icon_images(
    existing_launcher_icons: &HashMap<i32, Bitmap>,
) -> HashMap<i32, Bitmap> {
    let biggest_size = existing_launcher_icons.keys().max().unwrap();
    let windows_icon_sizes = [256, 128, 64, 48, 32, 16];
    let mut result = HashMap::new();
    for &size in windows_icon_sizes.iter() {
        if !existing_launcher_icons.contains_key(&size) {
            let scaled_image = existing_launcher_icons
                .get(&biggest_size)
                .unwrap()
                .scaled_to_sample_nearest_neighbor(size as u32, size as u32);
            result.insert(size, scaled_image);
        } else {
            let image = existing_launcher_icons.get(&size).unwrap();
            result.insert(size, image.clone());
        }
    }
    result
}

fn create_windows_launcher_icon(
    windows_icon_images: &HashMap<i32, Bitmap>,
    icon_output_filepath: &str,
) {
    let mut iconpacker = ico::IconDir::new(ico::ResourceType::Icon);
    for (_size, image) in windows_icon_images.iter() {
        let icon_image = ico::IconImage::from_rgba_data(
            image.width as u32,
            image.height as u32,
            image.to_bytes(),
        );

        iconpacker.add_entry(ico::IconDirEntry::encode(&icon_image).expect(&format!(
            "Cannot encode icon ({}x{}) into launcher icon",
            image.width, image.height,
        )));
    }
    let output_file = std::fs::File::create(icon_output_filepath)
        .expect(&format!("Could not create path '{}'", icon_output_filepath));
    iconpacker
        .write(output_file)
        .expect(&format!("Could not write to '{}'", icon_output_filepath));
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Main

fn create_credits_file(
    base_credits_file: &str,
    license_searchdirs: &[&str],
    output_filepath: &str,
) {
    let mut credits_content = std::fs::read_to_string(base_credits_file).expect(&format!(
        "Cannot read credits basefile '{}'",
        base_credits_file
    ));
    credits_content +=
        "\r\n\r\n\r\nThe following free assetes, open source software libraries and \
         frameworks went into the making of this software:\r\n\r\n\r\n";

    let mut license_files = Vec::new();
    for searchdir in license_searchdirs {
        license_files.append(&mut collect_files_by_extension_recursive(
            searchdir, ".license",
        ));
    }

    let mut license_texts = HashSet::new();
    for license_file in license_files {
        let license_text = std::fs::read_to_string(license_file).unwrap();
        license_texts.insert(license_text);
    }

    for license_text in license_texts {
        credits_content += "===============\r\n";
        credits_content += &license_text;
        credits_content += "\r\n\r\n\r\n";
    }

    std::fs::write(output_filepath, credits_content).unwrap();
}

fn recreate_directory(path: &str) {
    if path_exists(path) {
        loop {
            let dir_content = collect_files_recursive(path);

            let mut has_error = false;
            for path_to_delete in dir_content {
                if PathBuf::from(&path_to_delete).is_dir() {
                    if let Err(error) = std::fs::remove_dir_all(&path_to_delete) {
                        has_error = true;
                        log::warn!(
                            "Unable to delete '{}' dir, are files from this folder still open? : {}",
                            path_to_delete,
                            error,
                        );
                    }
                } else {
                    if let Err(error) = std::fs::remove_file(&path_to_delete) {
                        has_error = true;
                        log::warn!(
                            "Unable to delete file '{}', is it still open? : {}",
                            path_to_delete,
                            error,
                        );
                    }
                }
            }

            if has_error {
                std::thread::sleep(std::time::Duration::from_secs(1));
            } else {
                break;
            }
        }
    } else {
        std::fs::create_dir_all(path).expect(&format!("Unable to create '{}' dir", path));
    }
}

fn main() {
    let start_time = std::time::Instant::now();

    init_logging("target/assetbaker_log.txt", log::Level::Trace).expect("Unable to init logging");
    std::panic::set_hook(Box::new(|panic_info| {
        let (message, location) = core::panic_message_split_to_message_and_location(panic_info);
        let final_message = format!("{}\n\nError occured at: {}", message, location);
        log::error!("{}", final_message);

        // NOTE: This forces the other threads to shutdown as well
        std::process::abort();
    }));

    recreate_directory("target/assets_temp");
    recreate_directory("target/assets_temp/audio");
    recreate_directory("target/assets_temp/graphics");
    recreate_directory("resources");
    recreate_directory("resources_executable");

    if path_exists("assets") && !path_dir_empty("assets") {
        if path_exists("assets/credits.txt") {
            create_credits_file(
                "assets/credits.txt",
                &["assets", "assets_copy", "assets_executable", "cottontail"],
                "resources/credits.txt",
            );
        } else {
            log::warn!("No credits file found at 'assets/credits.txt'")
        }

        bake_graphics_resources();
        bake_audio_resources("audio", 44100);
        bake_audio_resources("audio_wasm", 22050);
    }

    if path_exists("assets_copy") {
        path_copy_directory_contents_recursive("assets_copy", "resources");
        // Delete license files that got accidentally copied over to output path.
        // NOTE: We don't need those because we will create a credits file containing all licenses
        for license_path in collect_files_by_extension_recursive("resources", ".license") {
            std::fs::remove_file(&license_path)
                .expect(&format!("Cannot delete '{}'", &license_path));
        }
    }

    if path_exists("assets_executable") {
        // Copy version info
        if path_exists("assets_executable/versioninfo.rc") {
            std::fs::copy(
                "assets_executable/versioninfo.rc",
                "resources_executable/versioninfo.rc",
            )
            .expect(
                "Could not copy from 'assets_executable/versioninfo.rc' to 'resources_executable/versioninfo.rc'",
            );
        }

        // Create launcher icon
        if path_exists("assets_executable/launcher_icon") {
            let existing_launcher_icons =
                load_existing_launcher_icon_images("assets_executable/launcher_icon");
            let windows_icon_images = create_windows_launcher_icon_images(&existing_launcher_icons);
            for (&size, image) in windows_icon_images.iter() {
                Bitmap::write_to_png_file(
                    image,
                    &format!("target/assets_temp/launcher_icons_windows/{}.png", size),
                );
            }
            create_windows_launcher_icon(&windows_icon_images, "resources_executable/launcher.ico");
        }
    }

    // Write indexfile
    let mut filelist_content = String::new();
    let filelist = collect_files_recursive("resources");
    for filepath in &filelist {
        filelist_content += &format!("{}\n", filepath);
    }
    std::fs::write("resources/index.txt", filelist_content.as_bytes())
        .expect("Could not write indexfile to 'resources/index.txt'");

    log::info!(
        "ASSETS SUCCESSFULLY BAKED: Elapsed time: {:.3}s",
        start_time.elapsed().as_secs_f64()
    );
}

//==================================================================================================
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct GraphicsSheet {
    images: IndexMap<Imagename, Bitmap>,
    fonts: IndexMap<Fontname, AssetFont>,
    sprites: IndexMap<Spritename, AssetSprite>,
    sprites_3d: IndexMap<Spritename3D, AssetSprite3D>,
    animations: IndexMap<Animationname, AssetAnimation>,
    animations_3d: IndexMap<Animationname3D, AssetAnimation3D>,
}

impl GraphicsSheet {
    fn new_empty() -> GraphicsSheet {
        GraphicsSheet {
            images: IndexMap::new(),
            fonts: IndexMap::new(),
            sprites: IndexMap::new(),
            sprites_3d: IndexMap::new(),
            animations: IndexMap::new(),
            animations_3d: IndexMap::new(),
        }
    }

    fn extend_by(&mut self, other: GraphicsSheet) {
        self.images.extend(other.images);
        self.fonts.extend(other.fonts);
        self.sprites.extend(other.sprites);
        self.sprites_3d.extend(other.sprites_3d);
        self.animations.extend(other.animations);
        self.animations_3d.extend(other.animations_3d);
    }

    fn pack_and_serialize(mut self) {
        // Create texture atlas and adjust positions of our sprites according to the final packed
        // atlas positions
        let result_atlas = atlas_create_from_images(&self.images, "resources", 1024);
        for (packed_sprite_name, sprite_pos) in &result_atlas.sprite_positions {
            if self.sprites.contains_key(packed_sprite_name) {
                // Atlas-sprite is a regular sprite
                let mut sprite = self.sprites.get_mut(packed_sprite_name).unwrap();
                sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
                sprite.trimmed_uvs = sprite
                    .trimmed_uvs
                    .translated_by(sprite_pos.atlas_texture_pixel_offset);
            } else if self.fonts.contains_key(packed_sprite_name) {
                // Atlas-sprite is a glyph-sheet of some font
                let font = &self.fonts[packed_sprite_name];
                for sprite_glyph_name in font.glyphs.values().map(|glyph| &glyph.sprite_name) {
                    let mut sprite = self.sprites.get_mut(sprite_glyph_name).unwrap();
                    sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
                    sprite.trimmed_uvs = sprite
                        .trimmed_uvs
                        .translated_by(sprite_pos.atlas_texture_pixel_offset);
                }
            } else {
                // AssetSprite must be an animation-sheet of some animation(s)
                let mut found_anim = false;
                for (animation_name, animation) in &self.animations {
                    if animation_name.starts_with(&(packed_sprite_name.to_owned() + ":"))
                        || animation_name == packed_sprite_name
                    {
                        found_anim = true;
                        for sprite_frame_name in &animation.sprite_names {
                            let mut sprite = self.sprites.get_mut(sprite_frame_name).unwrap();
                            sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
                            sprite.trimmed_uvs = sprite
                                .trimmed_uvs
                                .translated_by(sprite_pos.atlas_texture_pixel_offset);
                        }
                    }
                }

                assert!(
                    found_anim,
                    "Packed unknown sprite name '{}' into atlas at position ({},{},{})",
                    packed_sprite_name,
                    sprite_pos.atlas_texture_index,
                    sprite_pos.atlas_texture_pixel_offset.x,
                    sprite_pos.atlas_texture_pixel_offset.y,
                );
            }
        }

        let final_sprites_by_name: IndexMap<Spritename, Sprite> = self
            .sprites
            .iter()
            .map(|(name, sprite)| {
                (
                    name.clone(),
                    convert_sprite(&sprite, result_atlas.texture_size),
                )
            })
            .collect();
        let final_sprites_by_name_3d: IndexMap<Spritename3D, Sprite3D> = self
            .sprites_3d
            .iter()
            .map(|(name, sprite)| {
                (
                    name.clone(),
                    convert_sprite_3d(&sprite, &final_sprites_by_name),
                )
            })
            .collect();

        serialize_sprites(&self.sprites, &final_sprites_by_name);
        serialize_sprites_3d(&self.sprites_3d, &final_sprites_by_name_3d);
        serialize_fonts(&self.fonts, &final_sprites_by_name);
        serialize_animations(&self.animations, &final_sprites_by_name);
        serialize_animations_3d(&self.animations_3d, &final_sprites_by_name_3d);
        serialize_atlas(&result_atlas);
    }
}
