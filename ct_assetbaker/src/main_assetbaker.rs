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

use std::collections::{HashMap, HashSet};

const TEST_TEXTURE_PACKER: bool = false;

#[derive(Debug, Clone, Serialize)]
pub struct AssetSprite {
    pub name: ResourceName,
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
    pub name: ResourceName,
    pub layer_sprite_names: Vec<ResourceName>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation {
    pub name: ResourceName,
    pub framecount: u32,
    pub sprite_names: Vec<ResourceName>,
    pub frame_durations_ms: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation3D {
    pub name: ResourceName,
    pub framecount: u32,
    pub sprite_names: Vec<ResourceName>,
    pub frame_durations_ms: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetGlyph {
    pub codepoint: Codepoint,
    pub sprite_name: ResourceName,

    pub horizontal_advance: i32,

    /// This is mainly used for text dimension calculations
    pub sprite_dimensions: Vec2i,
    /// This is mainly used for text dimension calculations
    pub sprite_draw_offset: Vec2i,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct AssetFont {
    pub name: ResourceName,
    pub baseline: i32,
    pub vertical_advance: i32,
    pub horizontal_advance_max: i32,
    pub is_fixed_width_font: bool,
    pub font_height_in_pixels: i32,
    pub glyphcount: u32,
    pub glyphs: IndexMap<Codepoint, AssetGlyph>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAtlas {
    pub textures_dimension: u32,
    pub textures_png_data: Vec<Vec<u8>>,
    pub sprite_positions: IndexMap<ResourceName, BitmapAtlasPosition>,
}

fn bake_graphics_resources() {
    let mut result_sheet: GraphicsSheet = GraphicsSheet::new_empty();

    // Load fonts and drawstyles
    let font_drawstyles = collect_font_drawstyles();
    let font_resources = collect_font_resources();

    // Create fonts and its correspronding sprites
    let font_sheets: Vec<GraphicsSheet> = font_drawstyles
        .par_iter()
        .map(|style| {
            let font = font_resources.get(&style.fontname).expect(&format!(
                "No font and/or render parameters found for font name '{}'",
                &style.fontname
            ));
            create_sheet_from_ttf(
                &style.fontname,
                &font.ttf_data_bytes,
                font.metadata.height_in_pixels,
                font.metadata.raster_offset,
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
                let sheet_name = path_to_filename_without_extension(imagepath);
                aseprite::create_sheet(imagepath, &sheet_name)
            })
            .collect()
    };
    for sheet in sprite_sheets {
        result_sheet.extend_by(sheet)
    }

    result_sheet.pack_and_serialize("graphics");

    // Create minimal prelude graphics sheet that starts up fast and only shows splashscreen
    let mut prelude_sheet = GraphicsSheet::new_empty();

    let splashcreen_path = {
        let mut splashcreen_paths = collect_files_by_glob_pattern("assets", "**/splashscreen.*");
        assert!(
            !splashcreen_paths.is_empty(),
            "No 'splashscreen.png' or 'splashscreen.ase' found in assets directory"
        );
        assert!(
        splashcreen_paths.len() == 1,
        "There can only exist exactly one 'splashscreen' sprite in the assets directory - found at least '{}' and '{}'",
        splashcreen_paths[0], splashcreen_paths[1]
        );
        splashcreen_paths.pop().unwrap()
    };

    let splashscreen_sheet = aseprite::create_sheet(&splashcreen_path, "splashscreen");
    // Create fonts and its correspronding sprites
    let default_font_sheet = create_sheet_from_ttf(
        FONT_DEFAULT_TINY_NAME,
        FONT_DEFAULT_TINY_TTF,
        FONT_DEFAULT_TINY_PIXEL_HEIGHT,
        FONT_DEFAULT_TINY_RASTER_OFFSET,
        true,
        PixelRGBA::white(),
        PixelRGBA::black(),
    );
    prelude_sheet.extend_by(default_font_sheet);
    prelude_sheet.extend_by(splashscreen_sheet);
    prelude_sheet.pack_and_serialize("graphics_splash");
}

fn collect_font_drawstyles() -> Vec<BitmapFontDrawStyle> {
    // Check that we have drawstyles defined for each font in our assets directory
    collect_files_recursive("assets")
        .into_iter()
        .filter(|path| path.ends_with(".ttf"))
        .for_each(|path| {
            let fontname = path_to_filename_without_extension(&path);
            let drawstyle_filepath = path_with_extension(&path, ".font_drawstyles.json");

            if !path_exists(&drawstyle_filepath) {
                log::warn!(
                    "Font '{}' was missing a drawing style - a default one was added one to '{}'",
                    &fontname,
                    drawstyle_filepath
                );
                let default_drawstyles = vec![
                    BitmapFontDrawStyle {
                        fontname: fontname.clone(),
                        bordered: true,
                        color_glyph: PixelRGBA::white(),
                        color_border: PixelRGBA::black(),
                    },
                    BitmapFontDrawStyle {
                        fontname: fontname.clone(),
                        bordered: false,
                        color_glyph: PixelRGBA::white(),
                        color_border: PixelRGBA::black(),
                    },
                ];
                serialize_to_json_file(&default_drawstyles, &drawstyle_filepath);
            }
        });

    // Collect all font drawstyles
    let font_drawstyles = {
        let font_drawstyles: Vec<BitmapFontDrawStyle> = collect_files_recursive("assets")
            .into_iter()
            .filter(|path| path.ends_with(".font_drawstyles.json"))
            .map(|path| deserialize_from_json_file::<Vec<BitmapFontDrawStyle>>(&path))
            .flatten()
            .collect();

        // Check that all fonts referenced in our drawstyles actually exist
        for style in &font_drawstyles {
            if style.fontname == FONT_DEFAULT_REGULAR_NAME
                || style.fontname == FONT_DEFAULT_SQUARE_NAME
                || style.fontname == FONT_DEFAULT_SMALL_NAME
                || style.fontname == FONT_DEFAULT_TINY_NAME
            {
                // Default fonts always exist
                continue;
            }

            let found_fonts_count = collect_files_recursive("assets")
                .into_iter()
                .filter(|path| path.ends_with(&format!("{}.ttf", style.fontname)))
                .count();
            assert!(
                found_fonts_count != 0,
                "Assets folder is missing font '{}.ttf' referenced in drawstyle",
                style.fontname
            );
            assert!(
                found_fonts_count == 1,
                "Font '{}'.ttf exists multiple times in assets folder in drawstyle",
                style.fontname
            );
        }

        if font_drawstyles.is_empty() {
            let default_drawstyles_path = format!(
                "assets/{}.font_drawstyles.json",
                font::FONT_DEFAULT_TINY_NAME
            );
            log::warn!(
                "No font drawstyles found in assets folder. Created default drawstyles at '{}'",
                default_drawstyles_path
            );

            // Create drawstyles for the default font
            let mut result = Vec::new();
            let default_color_glyph = PixelRGBA::new(255, 255, 255, 255);
            let default_color_border = PixelRGBA::new(0, 0, 0, 255);
            result.push(BitmapFontDrawStyle {
                fontname: font::FONT_DEFAULT_TINY_NAME.to_owned(),
                bordered: false,
                color_glyph: default_color_glyph,
                color_border: default_color_border,
            });
            result.push(BitmapFontDrawStyle {
                fontname: font::FONT_DEFAULT_TINY_NAME.to_owned(),
                bordered: true,
                color_glyph: default_color_glyph,
                color_border: default_color_border,
            });

            serialize_to_json_file(&result, &default_drawstyles_path);
            result
        } else {
            font_drawstyles
        }
    };

    // Check that our style definitions are unique
    let mut duplicate_check = HashSet::new();
    for style in font_drawstyles.iter() {
        assert!(
            !duplicate_check.contains(style),
            "Found duplicate font style definition {:?}",
            style
        );
        duplicate_check.insert(style.clone());
    }

    font_drawstyles
}

fn bake_audio_resources(resource_pack_name: &str, audio_sample_rate_hz: usize) {
    let mut audio_resources = AudioResources::new(audio_sample_rate_hz);

    // Search assets folder for metadata files without corresponding audio files
    for filepath in &collect_files_recursive("assets") {
        if !filepath.ends_with(".audiometa.json") {
            // This is no audio metadata file
            continue;
        }

        let corresponding_audio_filepath_wav = filepath.replace(".audiometa.json", ".wav");
        let corresponding_audio_filepath_ogg = filepath.replace(".audiometa.json", ".ogg");
        if !path_exists(&corresponding_audio_filepath_wav)
            && !path_exists(&corresponding_audio_filepath_ogg)
        {
            panic!(
                "Found audio metadata file '{}' without corresponding .wav or .ogg file",
                filepath
            );
        }
    }

    // Collect and process audio files
    for filepath in &collect_files_recursive("assets") {
        if !filepath.ends_with(".wav") && !filepath.ends_with(".ogg") {
            continue;
        }

        let resource_name = path_to_filename_without_extension(filepath);
        let metadata_path = path_with_extension(filepath, ".audiometa.json");
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
            let framecount = audiochunk.framecount();

            let original = AudioMetadata {
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

        let ogg_output_path = &format!(
            "target/assets_temp/{}/{}.ogg",
            resource_pack_name, resource_name
        );
        if filepath.ends_with(".ogg") {
            // Check if we need to resample our ogg file
            if metadata.samplerate_hz == metadata_original.samplerate_hz {
                // Make a human readable copy of our file in the temp dir
                path_copy_file(filepath, &ogg_output_path);
            } else {
                let (original_samplerate_hz, audiochunk) = audio::decode_audio_file(filepath)
                    .unwrap_or_else(|error| {
                        panic!("Cannot decode audio file '{}': {}", filepath, error)
                    });
                let wav_output_temp_path = path_with_extension(ogg_output_path, ".wav");
                write_audio_samples_to_wav_file(
                    &wav_output_temp_path,
                    &audiochunk,
                    original_samplerate_hz,
                );

                let command = format!(
                    "oggenc2 {} --quality {} --resample {} --converter 0 --output={}",
                    wav_output_temp_path,
                    metadata.compression_quality,
                    audio_sample_rate_hz,
                    ogg_output_path,
                );
                run_systemcommand_fail_on_error(&command, false);
            }
        } else {
            // Create resampled ogg file out of our wav file
            let command = format!(
                "oggenc2 {} --quality {} --resample {} --converter 0 --output={}",
                filepath, metadata.compression_quality, audio_sample_rate_hz, ogg_output_path
            );
            run_systemcommand_fail_on_error(&command, false);
        };

        let ogg_data = read_file_whole(&ogg_output_path).unwrap_or_else(|error| {
            panic!(
                "Failed to copy/encode ogg file for '{}' - '{}' is missing: {}",
                filepath, ogg_output_path, error
            )
        });

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
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Font resources and styles

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct BitmapFontDrawStyle {
    pub fontname: String,
    pub bordered: bool,
    pub color_glyph: PixelRGBA,
    pub color_border: PixelRGBA,
}

#[derive(Serialize, Deserialize)]
pub struct BitmapFontMetadata {
    pub height_in_pixels: i32,
    pub raster_offset: Vec2,
}

pub struct BitmapFontResource {
    pub ttf_data_bytes: Vec<u8>,
    pub metadata: BitmapFontMetadata,
}

fn collect_font_resources() -> IndexMap<ResourceName, BitmapFontResource> {
    // Search assets folder for metadata files without corresponding font files
    for filepath in &collect_files_recursive("assets") {
        if !filepath.ends_with(".fontmeta.json") {
            // This is no font metadata file
            continue;
        }

        let corresponding_ttf_filepath = filepath.replace(".fontmeta.json", ".ttf");
        if !path_exists(&corresponding_ttf_filepath) {
            panic!(
                "Found font metadata file '{}' without corresponding .ttf file",
                filepath
            );
        }
    }

    let mut result = IndexMap::new();

    let font_filepaths = collect_files_by_extension_recursive("assets", ".ttf");
    for font_filepath in font_filepaths {
        let font_name = path_to_filename_without_extension(&font_filepath);
        let metadata_filepath = path_with_extension(&font_filepath, ".fontmeta.json");
        let ttf_data = std::fs::read(&font_filepath)
            .expect(&format!("Cannot read font data '{}'", &font_filepath));

        // NOTE: We only read the fontdata when the renderparams exist but don't throw an error when
        //       it does not exist. This helps us make test renders for a font before we have found
        //       out its correct render params.
        if let Some(metadata_string) = std::fs::read_to_string(&metadata_filepath).ok() {
            let metadata: BitmapFontMetadata = serde_json::from_str(&metadata_string).expect(
                &format!("Cannot read metadata for font: '{}'", &font_filepath),
            );

            result.insert(
                font_name,
                BitmapFontResource {
                    ttf_data_bytes: ttf_data,
                    metadata,
                },
            );
        } else {
            let test_png_filepath = path_join(
                "target/assets_temp/font_test",
                &(font_name.clone() + "_fontsize_test_offset_-0.5.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data,
                Vec2::new(0.0, -0.5),
                4,
                32,
                &test_png_filepath,
            );
            let test_png_filepath = path_join(
                "target/assets_temp/font_test",
                &(font_name.clone() + "_fontsize_test_offset_0.0.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data,
                Vec2::new(0.0, 0.0),
                4,
                32,
                &test_png_filepath,
            );
            let test_png_filepath = path_join(
                "target/assets_temp/font_test",
                &(font_name.clone() + "_fontsize_test_offset_0.5.png"),
            );
            BitmapFont::test_font_sizes(
                &font_name,
                &ttf_data,
                Vec2::new(0.0, 0.5),
                4,
                32,
                &test_png_filepath,
            );

            let metadata_filepath = path_join(
                "target/assets_temp/font_test",
                &(font_name.clone() + "fontmeta.json"),
            );
            serialize_to_binary_file(
                &BitmapFontMetadata {
                    height_in_pixels: 12,
                    raster_offset: Vec2::zero(),
                },
                &metadata_filepath,
            );

            panic!(
                "Font '{}' is missing its metadata - 
                 Please look at the font size test images at 'target/assets_temp/font_test' and then
                 fill out and copy the '{}.fontmeta.json' from 'target/assets_temp/font_test' to '{}'",
                &font_name,
                &font_name,
                path_without_filename(&font_filepath)
            );
        }
    }

    // Add default fonts
    result.insert(
        font::FONT_DEFAULT_TINY_NAME.to_owned(),
        BitmapFontResource {
            ttf_data_bytes: font::FONT_DEFAULT_TINY_TTF.to_vec(),
            metadata: BitmapFontMetadata {
                height_in_pixels: font::FONT_DEFAULT_TINY_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_TINY_RASTER_OFFSET,
            },
        },
    );
    result.insert(
        font::FONT_DEFAULT_SMALL_NAME.to_owned(),
        BitmapFontResource {
            ttf_data_bytes: font::FONT_DEFAULT_SMALL_TTF.to_vec(),
            metadata: BitmapFontMetadata {
                height_in_pixels: font::FONT_DEFAULT_SMALL_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_SMALL_RASTER_OFFSET,
            },
        },
    );
    result.insert(
        font::FONT_DEFAULT_REGULAR_NAME.to_owned(),
        BitmapFontResource {
            ttf_data_bytes: font::FONT_DEFAULT_REGULAR_TTF.to_vec(),
            metadata: BitmapFontMetadata {
                height_in_pixels: font::FONT_DEFAULT_REGULAR_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_REGULAR_RASTER_OFFSET,
            },
        },
    );
    result.insert(
        font::FONT_DEFAULT_SQUARE_NAME.to_owned(),
        BitmapFontResource {
            ttf_data_bytes: font::FONT_DEFAULT_SQUARE_TTF.to_vec(),
            metadata: BitmapFontMetadata {
                height_in_pixels: font::FONT_DEFAULT_SQUARE_PIXEL_HEIGHT,
                raster_offset: font::FONT_DEFAULT_SQUARE_RASTER_OFFSET,
            },
        },
    );

    result
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
    height_in_pixels: i32,
    raster_offset: Vec2,
    draw_border: bool,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> GraphicsSheet {
    let font_name = font_name.to_owned() + if draw_border { "_bordered" } else { "" };
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

    // Human readable output
    let output_png_filepath = format!("target/assets_temp/fonts/{}.png", font_name);
    font_atlas_texture.write_to_png_file(&output_png_filepath);

    // Create sprites and glyphs
    let mut result_glyphs: IndexMap<Codepoint, AssetGlyph> = IndexMap::new();
    let mut result_sprites: IndexMap<ResourceName, AssetSprite> = IndexMap::new();
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
        is_fixed_width_font: font.is_fixed_width_font,
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
// Asset conversion

fn convert_sprite(sprite: &AssetSprite, atlas_texture_sizes: &[u32]) -> Sprite {
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
        trimmed_uvs: AAQuad::from_rect(Rect::from(sprite.trimmed_uvs).scaled_from_origin(
            Vec2::filled(1.0 / atlas_texture_sizes[sprite.atlas_texture_index as usize] as f32),
        )),
    }
}

fn convert_sprite_3d(
    sprite: &AssetSprite3D,
    final_sprites_by_name: &IndexMap<ResourceName, Sprite>,
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
    final_sprites_by_name: &IndexMap<ResourceName, Sprite>,
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
    final_sprites_by_name: &IndexMap<ResourceName, Sprite>,
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
        is_fixed_width_font: font.is_fixed_width_font,
        font_height_in_pixels: font.font_height_in_pixels,
        ascii_glyphs,
        unicode_glyphs,
    }
}

fn convert_animation(
    anim: &AssetAnimation,
    final_sprites_by_name: &IndexMap<ResourceName, Sprite>,
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
    final_sprites_by_name_3d: &IndexMap<ResourceName, Sprite3D>,
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

    if (path_exists("assets") && !path_dir_empty("assets"))
        || (path_exists("assets_copy") && !path_dir_empty("assets_copy"))
    {
        path_recreate_directory_looped("resources");
        std::fs::write(
            "resources/assetbaker.lock",
            "Asset baking in progress".as_bytes(),
        )
        .expect("Cannot write lockfile");
    }

    if path_exists("assets_copy") && !path_dir_empty("assets_copy") {
        let files_to_copy = collect_files_recursive("assets_copy");
        for filepath in &files_to_copy {
            path_copy_file(&filepath, &filepath.replace("assets_copy/", "resources/"));
        }
    }

    if path_exists("assets") && !path_dir_empty("assets") {
        path_recreate_directory_looped("target/assets_temp");
        if path_exists("assets/credits.txt") {
            path_recreate_directory_looped("target/assets_temp/content");
            create_credits_file(
                "assets/credits.txt",
                &["assets", "assets_executable", "cottontail"],
                "target/assets_temp/content/credits.txt",
            );
        } else {
            log::warn!("No credits file found at 'assets/credits.txt'")
        }

        // Create some random pngs
        if TEST_TEXTURE_PACKER {
            std::fs::remove_dir_all("assets/random_pngs_test").ok();
            let mut random = Random::new_from_seed(12345678);
            let indices_widths_heights_colors: Vec<_> = (0..1000)
                .map(|index| {
                    (
                        index,
                        random.u32_in_range(8, 128) + 1,
                        random.u32_in_range(8, 128) + 1,
                        PixelRGBA::new_random_non_translucent(&mut random),
                    )
                })
                .collect();
            indices_widths_heights_colors
                .par_iter()
                .for_each(|(index, width, height, color)| {
                    let bitmap = Bitmap::new_filled(*width, *height, *color);
                    bitmap.write_to_png_file(&format!(
                        "assets/random_pngs_test/random_{}x{}_{}.png",
                        bitmap.width, bitmap.height, index
                    ));
                });
        }

        bake_graphics_resources();
        bake_audio_resources("audio", 44100);
        bake_audio_resources("audio_wasm", 22050);

        if TEST_TEXTURE_PACKER {
            std::fs::remove_dir_all("assets/random_pngs_test").expect("Cannot remove png test dir");
        }
    }

    // Add remaining files as content data
    let mut content_filepaths = Vec::new();
    if path_exists("assets") {
        content_filepaths.extend(collect_files_recursive("assets"));
    }
    if !content_filepaths.is_empty() {
        for filepath in &content_filepaths {
            if filepath.ends_with(".wav")
                || filepath.ends_with(".ogg")
                || filepath.ends_with(".png")
                || filepath.ends_with(".ase")
                || filepath.ends_with(".license")
                || filepath.ends_with(".audiometa.json")
                || filepath.ends_with(".fontmeta.json")
                || filepath.ends_with(".ttf")
                || filepath.ends_with("font_drawstyles.json")
                || filepath.ends_with("credits.txt")
            {
                // We already processed the above files types by other means
                continue;
            }

            let content_filename = path_to_filename(&filepath);

            // Copy file to human readable location
            path_copy_file(
                &filepath,
                &format!("target/assets_temp/content/{}", &content_filename),
            );
        }

        let mut contents: HashMap<String, Vec<u8>> = HashMap::new();
        for filepath in &collect_files_recursive("target/assets_temp/content") {
            let content_filename = path_to_filename(&filepath);
            let content_data = read_file_whole(&filepath).unwrap();
            contents.insert(content_filename, content_data);
        }
        serialize_to_binary_file(&contents, "resources/content.data");
    }

    if path_exists("resources/assetbaker.lock") {
        std::fs::remove_file("resources/assetbaker.lock").expect("Cannot remove lockfile");
    }

    log::info!(
        "ASSETS SUCCESSFULLY BAKED: Elapsed time: {:.3}s",
        start_time.elapsed().as_secs_f64()
    );
}

//==================================================================================================
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct GraphicsSheet {
    images: IndexMap<ResourceName, Bitmap>,
    fonts: IndexMap<ResourceName, AssetFont>,
    sprites: IndexMap<ResourceName, AssetSprite>,
    sprites_3d: IndexMap<ResourceName, AssetSprite3D>,
    animations: IndexMap<ResourceName, AssetAnimation>,
    animations_3d: IndexMap<ResourceName, AssetAnimation3D>,
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

    fn pack_and_serialize(mut self, pack_name: &str) {
        let pack_temp_out_dir = format!("target/assets_temp/{}", pack_name);
        let pack_out_filepath = format!("resources/{}.data", pack_name);
        path_recreate_directory_looped(&pack_temp_out_dir);

        // Pack textures
        let (mut textures, sprite_positions) = {
            let mut packer = BitmapMultiAtlas::new(1024, Some(2048), true);
            for (image_name, image) in self.images.iter() {
                packer.pack_bitmap(image_name, image);
            }
            packer.finish()
        };

        // NOTE: Drawstate assumes that every texture has a white pixel in its bottom-right corner
        //       to optimize shape drawing. As we made sure that the last row of every texture is
        //       reserved by passing a true to the `BitmapMultiAtlas` above, we just draw the
        //       white pixel into the bottom right corner
        for texture in &mut textures {
            assert!(texture.get(texture.width - 1, texture.height - 1) == PixelRGBA::transparent());
            texture.set(texture.width - 1, texture.height - 1, PixelRGBA::white());
        }

        // Create png files
        let textures_dimensions: Vec<u32> = textures
            .iter()
            .map(|texture| texture.width as u32)
            .collect();
        let textures_png_data: Vec<Vec<u8>> = textures
            .into_iter()
            .map(|texture| texture.to_premultiplied_alpha().encoded_as_png())
            .collect();

        // Create texture atlas and adjust positions of our sprites according to the final packed
        // atlas positions
        for (packed_sprite_name, sprite_pos) in &sprite_positions {
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

        // HUMAN READABLE OUTPUT
        let human_readable_sprites: Vec<AssetSprite> = self.sprites.values().cloned().collect();
        serialize_to_json_file(
            &human_readable_sprites,
            &path_join(&pack_temp_out_dir, "sprites.json"),
        );
        let human_readable_sprites_3d: Vec<AssetSprite3D> =
            self.sprites_3d.values().cloned().collect();
        serialize_to_json_file(
            &human_readable_sprites_3d,
            &path_join(&pack_temp_out_dir, "sprites_3d.json"),
        );
        let human_readable_fonts: Vec<AssetFont> = self.fonts.values().cloned().collect();
        serialize_to_json_file(
            &human_readable_fonts,
            &path_join(&pack_temp_out_dir, "fonts.json"),
        );
        let human_readable_animations: Vec<AssetAnimation> =
            self.animations.values().cloned().collect();
        serialize_to_json_file(
            &human_readable_animations,
            &path_join(&pack_temp_out_dir, "animations.json"),
        );
        let human_readable_animations_3d: Vec<AssetAnimation3D> =
            self.animations_3d.values().cloned().collect();
        serialize_to_json_file(
            &human_readable_animations_3d,
            &path_join(&pack_temp_out_dir, "animations_3d.json"),
        );
        for (index, (png_data, texture_dimension)) in textures_png_data
            .iter()
            .zip(textures_dimensions.iter())
            .enumerate()
        {
            let texture_path = format!(
                "{}/atlas-{}x{}-{}.png",
                pack_temp_out_dir, texture_dimension, texture_dimension, index
            );
            std::fs::write(&texture_path, &png_data).unwrap_or_else(|error| {
                panic!(
                    "Could not write atlas texture '{}': {}",
                    texture_path, error
                )
            });
        }

        // Convert resources into final format
        let sprites: IndexMap<ResourceName, Sprite> = self
            .sprites
            .iter()
            .map(|(name, sprite)| (name.clone(), convert_sprite(&sprite, &textures_dimensions)))
            .collect();
        let sprites_3d: IndexMap<ResourceName, Sprite3D> = self
            .sprites_3d
            .iter()
            .map(|(name, sprite)| (name.clone(), convert_sprite_3d(&sprite, &sprites)))
            .collect();
        let animations = self
            .animations
            .iter()
            .map(|(name, anim)| (name.clone(), convert_animation(anim, &sprites)))
            .collect();
        let animations_3d = self
            .animations_3d
            .iter()
            .map(|(name, anim)| (name.clone(), convert_animation_3d(anim, &sprites_3d)))
            .collect();
        let fonts = self
            .fonts
            .iter()
            .map(|(name, font)| (name.clone(), convert_font(font, &sprites)))
            .collect();

        // Create GraphicsResources for serialization
        let graphics_resources = GraphicResources {
            animations,
            animations_3d,
            fonts,
            sprites,
            sprites_3d,
            textures_png_data,
        };

        serialize_to_binary_file(&graphics_resources, &pack_out_filepath);
    }
}
