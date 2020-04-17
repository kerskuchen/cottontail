mod aseprite;

use ct_lib::bitmap::{BitmapAtlasPosition, BitmapMultiAtlas};
use ct_lib::color::*;
use ct_lib::draw::*;
use ct_lib::font;
use ct_lib::game::*;
use ct_lib::math::*;
use ct_lib::sprite::*;
use ct_lib::system;
use ct_lib::IndexMap;

use fern;
use log;
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

type Spritename = String;
type Fontname = String;
type Animationname = String;

#[derive(Clone, Serialize)]
pub struct AssetSprite {
    pub name: Spritename,
    pub name_hash: u64,
    pub atlas_texture_index: TextureIndex,
    pub has_translucency: bool,

    pub pivot_offset: Vec2i,
    pub attachment_points: [Vec2i; SPRITE_ATTACHMENT_POINTS_MAX_COUNT],

    pub untrimmed_dimensions: Vec2i,

    pub trimmed_rect: Recti,
    pub trimmed_uvs: Recti,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation {
    pub name: Animationname,
    pub name_hash: u64,
    pub framecount: u32,
    pub sprite_names: Vec<Spritename>,
    pub sprite_indices: Vec<u32>,
    pub frame_durations_ms: Vec<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetGlyph {
    pub codepoint: Codepoint,
    pub sprite_name: Spritename,
    pub sprite_index: SpriteIndex,
    pub horizontal_advance: i32,

    /// This is mainly used for text dimension calculations
    pub sprite_dimensions: Vec2i,
    /// This is mainly used for text dimension calculations
    pub sprite_draw_offset: Vec2i,
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct AssetFont {
    pub name: Fontname,
    pub name_hash: u64,
    pub baseline: i32,
    pub vertical_advance: i32,
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
    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    let mut result_fonts: IndexMap<Fontname, AssetFont> = IndexMap::new();
    let mut result_animations: IndexMap<Animationname, AssetAnimation> = IndexMap::new();

    // Create fonts and its correspronding sprites
    let font_styles = load_font_styles();
    let font_properties = load_font_properties();
    let sprites_and_fonts: Vec<(IndexMap<Spritename, AssetSprite>, AssetFont)> = font_styles
        .par_iter()
        .map(|style| {
            let properties = font_properties.get(&style.fontname).expect(&format!(
                "No font and/or render parameters found for font name '{}'",
                &style.fontname
            ));
            bitmapfont_create_from_ttf(
                &style.fontname,
                &properties.ttf_data_bytes,
                "assets_temp",
                properties.render_params.height_in_pixels,
                properties.render_params.raster_offset,
                style.bordered,
                style.color_glyph,
                style.color_border,
            )
        })
        .collect();
    for (sprites, font) in sprites_and_fonts {
        result_sprites.extend(sprites);
        result_fonts.insert(font.name.clone(), font);
    }

    // Convert png and aseprite files to png sheets and move to them to `assets_temp`
    let mut imagepaths = vec![];
    imagepaths.append(&mut system::collect_files_by_extension_recursive(
        "assets", ".ase",
    ));
    imagepaths.append(&mut system::collect_files_by_extension_recursive(
        "assets", ".png",
    ));
    let sprites_and_animations: Vec<(
        IndexMap<Spritename, AssetSprite>,
        IndexMap<Animationname, AssetAnimation>,
    )> = imagepaths
        .par_iter()
        .map(|imagepath| aseprite::create_sheet_animations(imagepath, "assets", "assets_temp"))
        .collect();
    for (sprites, animations) in sprites_and_animations {
        result_sprites.extend(sprites);
        result_animations.extend(animations);
    }

    // Create texture atlas and Adjust positions of our sprites according to the final packed
    // atlas positions
    let result_atlas = atlas_create_from_pngs("assets_temp", "assets_baked", 1024);
    for (sprite_name, sprite_pos) in &result_atlas.sprite_positions {
        if result_sprites.contains_key(sprite_name) {
            // Atlas-sprite is a regular sprite
            let mut sprite = result_sprites.get_mut(sprite_name).unwrap();
            sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
            sprite.trimmed_uvs = sprite
                .trimmed_uvs
                .translated_by(sprite_pos.atlas_texture_pixel_offset);
        } else if result_fonts.contains_key(sprite_name) {
            // Atlas-sprite is a glyph-sheet of some font
            let font = &result_fonts[sprite_name];
            for sprite_glyph_name in font.glyphs.values().map(|glyph| &glyph.sprite_name) {
                let mut sprite = result_sprites.get_mut(sprite_glyph_name).unwrap();
                sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
                sprite.trimmed_uvs = sprite
                    .trimmed_uvs
                    .translated_by(sprite_pos.atlas_texture_pixel_offset);
            }
        } else {
            // AssetSprite must be an animation-sheet of some animation(s)
            let mut found_anim = false;
            for (animation_name, animation) in &result_animations {
                if animation_name.starts_with(&(sprite_name.to_owned() + ".")) {
                    found_anim = true;
                    for sprite_frame_name in &animation.sprite_names {
                        let mut sprite = result_sprites.get_mut(sprite_frame_name).unwrap();
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
                sprite_name,
                sprite_pos.atlas_texture_index,
                sprite_pos.atlas_texture_pixel_offset.x,
                sprite_pos.atlas_texture_pixel_offset.y,
            );
        }
    }

    // Assign sprite indices to glyphs and animation frames
    let spritename_to_index_map: IndexMap<Spritename, u32> = result_sprites
        .keys()
        .enumerate()
        .map(|(index, key)| (key.clone(), index as u32))
        .collect();
    for font in result_fonts.values_mut() {
        for glyph in font.glyphs.values_mut() {
            glyph.sprite_index = spritename_to_index_map[&glyph.sprite_name];
        }
    }
    for animation in result_animations.values_mut() {
        for (index, sprite_name) in animation.sprite_names.iter().enumerate() {
            animation.sprite_indices[index] = spritename_to_index_map[sprite_name];
        }
    }

    serialize_sprites(&result_sprites, result_atlas.texture_size);
    serialize_fonts(&result_fonts);
    serialize_animations(&result_animations);
    serialize_atlas(&result_atlas);
}

fn bake_audio_resources() {
    let ogg_paths = system::collect_files_by_extension_recursive("assets", ".ogg");
    for ogg_path_source in &ogg_paths {
        let ogg_path_dest = ogg_path_source.replace("assets", "assets_baked");
        std::fs::create_dir_all(system::path_without_filename(&ogg_path_dest)).unwrap();
        std::fs::copy(ogg_path_source, ogg_path_dest).unwrap();
    }

    let wav_paths = system::collect_files_by_extension_recursive("assets", ".wav");
    for wav_path_source in &wav_paths {
        let wav_path_dest = wav_path_source.replace("assets", "assets_baked");
        std::fs::create_dir_all(system::path_without_filename(&wav_path_dest)).unwrap();
        std::fs::copy(wav_path_source, wav_path_dest).unwrap();
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

    let font_filepaths = system::collect_files_by_extension_recursive("assets/fonts", ".ttf");
    for font_filepath in font_filepaths {
        let font_name = system::path_to_filename_without_extension(&font_filepath);
        let renderparams_filepath = system::path_with_extension(&font_filepath, "json");
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
            let test_png_filepath =
                system::path_join("assets_temp", &(font_name.clone() + "_fontsize_test.png"));
            println!(
                "Font is missing its render parameters: '{}' - Created font size test image at '{}'",
                &font_filepath,
                &test_png_filepath
            );
            BitmapFont::test_font_sizes(&font_name, &ttf_data_bytes, 4, 32, &test_png_filepath);
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
    let glyph_rect = glyph.get_trimmed_rect();
    let glyph_atlas_pos = if let Some(pos) = position_in_font_atlas {
        pos
    } else {
        Vec2i::zero()
    };

    // NOTE: The `atlas_texture_index` and the `trimmed_rect_uv` will be adjusted later when we
    // actually pack the sprites into atlas textures
    AssetSprite {
        name: sprite_name.to_owned(),
        name_hash: ct_lib::hash_string_64(sprite_name),

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

pub fn bitmapfont_create_from_ttf(
    font_name: &str,
    font_ttf_bytes: &[u8],
    output_dir: &str,
    height_in_pixels: i32,
    raster_offset: Vec2,
    draw_border: bool,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> (IndexMap<Spritename, AssetSprite>, AssetFont) {
    let font_name = font_name.to_owned() + if draw_border { "_bordered" } else { "" };

    let output_filepath_without_extension = system::path_join(output_dir, &font_name);
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
            // NOTE: The `sprite_index` be set later when we finished collecting all
            //       our the sprites
            sprite_index: std::u32::MAX,
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
        name_hash: ct_lib::hash_string_64(&font_name),
        baseline: font.baseline,
        vertical_advance: font.vertical_advance,
        font_height_in_pixels: font.font_height_in_pixels,
        glyphcount: result_glyphs.len() as u32,
        glyphs: result_glyphs,
    };

    (result_sprites, result_font)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atlas packing

pub fn atlas_create_from_pngs(
    png_search_dir: &str,
    output_dir: &str,
    atlas_texture_size: u32,
) -> AssetAtlas {
    let sprite_imagepaths = system::collect_files_by_extension_recursive(png_search_dir, ".png");

    // Pack sprites
    let (atlas_textures, result_sprite_positions) = {
        let mut packer = BitmapMultiAtlas::new(atlas_texture_size as i32);
        for image_path in sprite_imagepaths.into_iter() {
            let image = Bitmap::create_from_png_file(&image_path);
            let sprite_name = system::path_without_extension(&image_path)
                .replace(&format!("{}/", png_search_dir), "");
            packer.pack_bitmap(&sprite_name, &image);
        }
        packer.finish()
    };

    // Write textures to disk
    let result_texture_imagepaths = {
        let atlas_path_without_extension = system::path_join(output_dir, "atlas");
        let mut texture_imagepaths = Vec::new();
        for (index, atlas_texture) in atlas_textures.iter().enumerate() {
            let texture_path = format!("{}-{}.png", atlas_path_without_extension, index);
            Bitmap::write_to_png_file(&atlas_texture.to_premultiplied(), &texture_path);

            // NOTE: We assume that our atlas textures will be located at the root of our final destination,
            //       so we drop the prefix
            let texture_path_shortened = texture_path.replace("assets_baked/", "");
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

fn convert_sprite(
    sprite: &AssetSprite,
    sprite_index: SpriteIndex,
    atlas_texture_size: u32,
) -> Sprite {
    let attachment_points = [
        Vec2::from(sprite.attachment_points[0]),
        Vec2::from(sprite.attachment_points[1]),
        Vec2::from(sprite.attachment_points[2]),
        Vec2::from(sprite.attachment_points[3]),
    ];
    Sprite {
        name: sprite.name.clone(),
        index: sprite_index,
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

fn convert_glyph(glyph: &AssetGlyph) -> SpriteGlyph {
    SpriteGlyph {
        horizontal_advance: glyph.horizontal_advance,
        sprite_index: glyph.sprite_index,
        sprite_dimensions: glyph.sprite_dimensions,
        sprite_draw_offset: glyph.sprite_draw_offset,
    }
}

fn convert_font(font: &AssetFont) -> SpriteFont {
    let mut ascii_glyphs: Vec<SpriteGlyph> =
        vec![SpriteGlyph::default(); FONT_MAX_NUM_FASTPATH_CODEPOINTS];
    let mut unicode_glyphs: HashMap<Codepoint, SpriteGlyph> = HashMap::new();

    for glyph in font.glyphs.values() {
        let codepoint = glyph.codepoint;
        let converted_glyph = convert_glyph(glyph);
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
        font_height_in_pixels: font.font_height_in_pixels,
        ascii_glyphs,
        unicode_glyphs,
    }
}

fn convert_animation(anim: &AssetAnimation) -> Animation {
    assert!(anim.sprite_indices.len() == anim.frame_durations_ms.len());
    let mut anim_result = Animation::new_empty(&anim.name);
    for (&frame_duration_ms, &sprite_index) in anim
        .frame_durations_ms
        .iter()
        .zip(anim.sprite_indices.iter())
    {
        anim_result.add_frame(frame_duration_ms as f32 / 1000.0, sprite_index as f32);
    }
    anim_result
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Serialization

fn serialize_sprites(sprite_map: &IndexMap<Spritename, AssetSprite>, atlas_texture_size: u32) {
    let human_readable: Vec<AssetSprite> = sprite_map.values().cloned().collect();
    std::fs::write(
        "assets_baked/sprites.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: Vec<Sprite> = human_readable
        .iter()
        .enumerate()
        .map(|(index, sprite)| convert_sprite(sprite, index as u32, atlas_texture_size))
        .collect();
    std::fs::write(
        "assets_baked/sprites.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_fonts(font_map: &IndexMap<Fontname, AssetFont>) {
    let human_readable: Vec<AssetFont> = font_map.values().cloned().collect();
    std::fs::write(
        "assets_baked/fonts.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: HashMap<String, SpriteFont> = font_map
        .iter()
        .map(|(name, font)| (name.clone(), convert_font(font)))
        .collect();
    std::fs::write(
        "assets_baked/fonts.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_animations(animation_map: &IndexMap<Animationname, AssetAnimation>) {
    let human_readable: Vec<AssetAnimation> = animation_map.values().cloned().collect();
    std::fs::write(
        "assets_baked/animations.json",
        serde_json::to_string_pretty(&human_readable).unwrap(),
    )
    .unwrap();

    let binary: HashMap<String, Animation> = animation_map
        .iter()
        .map(|(name, anim)| (name.clone(), convert_animation(anim)))
        .collect();
    std::fs::write(
        "assets_baked/animations.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_atlas(atlas: &AssetAtlas) {
    // Human readable
    std::fs::write(
        "assets_baked/atlas.json",
        serde_json::to_string_pretty(&atlas).unwrap(),
    )
    .unwrap();

    let binary: Vec<String> = atlas.texture_imagepaths.clone();
    std::fs::write(
        "assets_baked/atlas.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
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

    let mut license_files = vec![];
    for searchdir in license_searchdirs {
        license_files.append(&mut system::collect_files_by_extension_recursive(
            searchdir, ".license",
        ));
    }

    for license_file in license_files {
        credits_content += "===============\r\n";
        credits_content += &std::fs::read_to_string(license_file).unwrap();
        credits_content += "\r\n\r\n\r\n";
    }

    std::fs::write(output_filepath, credits_content).unwrap();
}

fn main() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("{}: {}\r", record.level(), message))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()
        .expect("Failed to start logger");

    std::panic::set_hook(Box::new(|panic_info| {
        let (message, location) = ct_lib::panic_message_split_to_message_and_location(panic_info);
        let final_message = format!("{}\n\nError occured at: {}", message, location);
        println!("{}", final_message);

        // NOTE: This forces the other threads to shutdown as well
        std::process::abort();
    }));

    let start_time = std::time::Instant::now();

    if system::path_exists("assets_temp") {
        loop {
            if std::fs::remove_dir_all("assets_temp").is_ok() {
                break;
            }
            println!("Unable to delete 'assets_temp' dir, are files from this folder still open?");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    std::fs::create_dir_all("assets_temp").expect("Unable to create 'assets_temp' dir");

    if system::path_exists("assets_baked") {
        loop {
            if std::fs::remove_dir_all("assets_baked").is_ok() {
                break;
            }
            println!("Unable to delete 'assets_baked' dir, are files from this folder still open?");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    std::fs::create_dir_all("assets_baked").expect("Unable to create 'assets_baked' dir");

    create_credits_file(
        "assets/credits.txt",
        &["assets", "code"],
        "assets_baked/credits.txt",
    );

    std::fs::copy(
        "assets/etc/gamecontrollerdb.txt",
        "assets_baked/gamecontrollerdb.txt",
    )
    .unwrap();

    bake_graphics_resources();
    bake_audio_resources();

    println!(
        "ASSETS SUCCESSFULLY BAKED: Elapsed time: {:.3}s",
        start_time.elapsed().as_secs_f64()
    );
}
