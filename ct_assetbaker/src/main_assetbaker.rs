mod aseprite;

use ct_lib::bitmap::{BitmapAtlasPosition, BitmapMultiAtlas};
use ct_lib::color::*;
use ct_lib::draw::*;
use ct_lib::font;
use ct_lib::game::*;
use ct_lib::math::*;
use ct_lib::sprite::*;
use ct_lib::system;

use ct_lib::bincode;
use ct_lib::indexmap::IndexMap;
use ct_lib::log;
use ct_lib::serde_derive::{Deserialize, Serialize};
use ct_lib::serde_json;

use fern;
use ico;
use rayon::prelude::*;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

type Spritename = String;
type Fontname = String;
type Animationname = String;

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

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AssetAnimation {
    pub name: Animationname,
    pub framecount: u32,
    pub sprite_names: Vec<Spritename>,
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
                "target/assets_temp",
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

    // Convert png and aseprite files to png sheets and move to them to `target/assets_temp`
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
        .map(|imagepath| {
            let sheet_name = system::path_without_extension(imagepath).replace("assets/", "");
            let output_path_without_extension =
                system::path_without_extension(imagepath).replace("assets", "target/assets_temp");
            aseprite::create_sheet_animations(
                imagepath,
                &sheet_name,
                &output_path_without_extension,
            )
        })
        .collect();
    for (sprites, animations) in sprites_and_animations {
        result_sprites.extend(sprites);
        result_animations.extend(animations);
    }

    // Create texture atlas and Adjust positions of our sprites according to the final packed
    // atlas positions
    let result_atlas = atlas_create_from_pngs("target/assets_temp", "resources", 1024);
    for (packed_sprite_name, sprite_pos) in &result_atlas.sprite_positions {
        if result_sprites.contains_key(packed_sprite_name) {
            // Atlas-sprite is a regular sprite
            let mut sprite = result_sprites.get_mut(packed_sprite_name).unwrap();
            sprite.atlas_texture_index = sprite_pos.atlas_texture_index;
            sprite.trimmed_uvs = sprite
                .trimmed_uvs
                .translated_by(sprite_pos.atlas_texture_pixel_offset);
        } else if result_fonts.contains_key(packed_sprite_name) {
            // Atlas-sprite is a glyph-sheet of some font
            let font = &result_fonts[packed_sprite_name];
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
                if animation_name.starts_with(&(packed_sprite_name.to_owned() + ":"))
                    || animation_name == packed_sprite_name
                {
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
                packed_sprite_name,
                sprite_pos.atlas_texture_index,
                sprite_pos.atlas_texture_pixel_offset.x,
                sprite_pos.atlas_texture_pixel_offset.y,
            );
        }
    }

    let final_sprites_by_name: IndexMap<Spritename, Sprite> = result_sprites
        .iter()
        .map(|(name, sprite)| {
            (
                name.clone(),
                convert_sprite(&sprite, result_atlas.texture_size),
            )
        })
        .collect();

    serialize_sprites(&result_sprites, result_atlas.texture_size);
    serialize_fonts(&result_fonts, &final_sprites_by_name);
    serialize_animations(&result_animations, &final_sprites_by_name);
    serialize_atlas(&result_atlas);
}

fn bake_audio_resources() {
    let ogg_paths = system::collect_files_by_extension_recursive("assets", ".ogg");
    for ogg_path_source in &ogg_paths {
        let ogg_path_dest = ogg_path_source.replace("assets", "resources");
        std::fs::create_dir_all(system::path_without_filename(&ogg_path_dest)).unwrap();
        std::fs::copy(ogg_path_source, ogg_path_dest).unwrap();
    }

    let wav_paths = system::collect_files_by_extension_recursive("assets", ".wav");
    for wav_path_source in &wav_paths {
        let wav_path_dest = wav_path_source.replace("assets", "resources");
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
            let test_png_filepath = system::path_join(
                "target/assets_temp",
                &(font_name.clone() + "_fontsize_test.png"),
            );
            log::warn!(
                "Font is missing its render parameters: '{}' - Created font size test image at '{}'",
                &font_filepath,
                &test_png_filepath
            );
            let test_png_filepath = system::path_join(
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
            let test_png_filepath = system::path_join(
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
            let test_png_filepath = system::path_join(
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
            let image = Bitmap::from_png_file_or_panic(&image_path);
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
    let mut anim_result = Animation::new_empty(&anim.name);
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Serialization

fn serialize_sprites(sprite_map: &IndexMap<Spritename, AssetSprite>, atlas_texture_size: u32) {
    std::fs::write(
        "resources/sprites.json",
        serde_json::to_string_pretty(sprite_map).unwrap(),
    )
    .unwrap();

    let binary: IndexMap<Spritename, Sprite> = sprite_map
        .iter()
        .map(|(name, sprite)| (name.clone(), convert_sprite(sprite, atlas_texture_size)))
        .collect();
    std::fs::write(
        "resources/sprites.data",
        bincode::serialize(&binary).unwrap(),
    )
    .unwrap();
}

fn serialize_fonts(
    font_map: &IndexMap<Fontname, AssetFont>,
    final_sprites_by_name: &IndexMap<Spritename, Sprite>,
) {
    let human_readable: Vec<AssetFont> = font_map.values().cloned().collect();
    std::fs::write(
        "resources/fonts.json",
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
        "resources/animations.json",
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

fn serialize_atlas(atlas: &AssetAtlas) {
    // Human readable
    std::fs::write(
        "resources/atlas.json",
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
    let image_paths = system::collect_files_by_extension_recursive(search_dir, ".png");
    for image_path in &image_paths {
        let image = Bitmap::from_png_file_or_panic(image_path);
        let size = system::path_to_filename_without_extension(image_path)
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
        license_files.append(&mut system::collect_files_by_extension_recursive(
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
    if system::path_exists(path) {
        loop {
            let dir_content = system::collect_files_recursive(path);

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
        log::error!("{}", final_message);

        // NOTE: This forces the other threads to shutdown as well
        std::process::abort();
    }));

    recreate_directory("target/assets_temp");
    recreate_directory("resources");
    recreate_directory("resources_executable");

    if system::path_exists("assets") && !system::path_dir_empty("assets") {
        if system::path_exists("assets/credits.txt") {
            create_credits_file(
                "assets/credits.txt",
                &["assets", "assets_copy", "assets_executable", "cottontail"],
                "resources/credits.txt",
            );
        } else {
            log::warn!("No credits file found at 'assets/credits.txt'")
        }

        bake_graphics_resources();
        bake_audio_resources();
    }

    if system::path_exists("assets_copy") {
        system::path_copy_directory_contents_recursive("assets_copy", "resources");
        // Delete license files that got accidentally copied over to output path.
        // NOTE: We don't need those because we will create a credits file containing all licenses
        for license_path in system::collect_files_by_extension_recursive("resources", ".license") {
            std::fs::remove_file(&license_path)
                .expect(&format!("Cannot delete '{}'", &license_path));
        }
    }

    if system::path_exists("assets_executable") {
        // Copy version info
        if system::path_exists("assets_executable/versioninfo.rc") {
            std::fs::copy(
                "assets_executable/versioninfo.rc",
                "resources_executable/versioninfo.rc",
            )
            .expect(
                "Could not copy from 'assets_executable/versioninfo.rc' to 'resources_executable/versioninfo.rc'",
            );
        }

        // Create launcher icon
        if system::path_exists("assets_executable/launcher_icon") {
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

    log::info!(
        "ASSETS SUCCESSFULLY BAKED: Elapsed time: {:.3}s",
        start_time.elapsed().as_secs_f64()
    );
}
