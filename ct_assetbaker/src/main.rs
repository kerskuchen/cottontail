mod aseprite;

use ct_lib::bitmap_atlas::{BitmapAtlasPosition, BitmapMultiAtlas};
use ct_lib::bitmap_font::*;
use ct_lib::color::*;
use ct_lib::draw::*;
use ct_lib::game::*;
use ct_lib::math::*;
use ct_lib::system;
use ct_lib::IndexMap;

use rayon::prelude::*;
use serde_derive::Serialize;

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
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct AssetFont {
    pub name: Fontname,
    pub name_hash: u64,
    pub baseline: i32,
    pub vertical_advance: i32,
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

pub struct BitmapFontProperties {
    pub ttf_path: String,
    pub output_dir: String,
    pub fontsize_in_pixels: u32,
    pub bordered: bool,
    pub color_glyph: PixelRGBA,
    pub color_border: PixelRGBA,
}

fn bake_graphics_resources() {
    // bitmapfont::test_font_sizes( "assets/fonts/Proggy/ProggyTiny.ttf", "assets_temp", 5, 24,);

    let color_glyph = PixelRGBA::new(255, 255, 255, 255);
    let color_border = PixelRGBA::new(0, 0, 0, 255);

    let font_properties = vec![
        BitmapFontProperties {
            ttf_path: "assets/fonts/Proggy/ProggyTiny.ttf".to_owned(),
            output_dir: "assets_temp".to_owned(),
            fontsize_in_pixels: 10,
            bordered: false,
            color_glyph,
            color_border,
        },
        BitmapFontProperties {
            ttf_path: "assets/fonts/Proggy/ProggyTiny.ttf".to_owned(),
            output_dir: "assets_temp".to_owned(),
            fontsize_in_pixels: 10,
            bordered: true,
            color_glyph,
            color_border,
        },
        // bitmapfont::BitmapFontProperties {
        //     ttf_path: "assets/fonts/EnterCommand/EnterCommand.ttf".to_owned(),
        //     output_dir: "assets_temp".to_owned(),
        //     fontsize_in_pixels: 16,
        //     texture_size: 160,
        //     bordered: false,
        //     color_glyph,
        //     color_border,
        // },
        // bitmapfont::BitmapFontProperties {
        //     ttf_path: "assets/fonts/EnterCommand/EnterCommand.ttf".to_owned(),
        //     output_dir: "assets_temp".to_owned(),
        //     fontsize_in_pixels: 16,
        //     texture_size: 200,
        //     bordered: true,
        //     color_glyph,
        //     color_border,
        // },
    ];

    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    let mut result_fonts: IndexMap<Fontname, AssetFont> = IndexMap::new();
    let mut result_animations: IndexMap<Animationname, AssetAnimation> = IndexMap::new();

    // Collect fonts and corresponding sprites
    let sprites_and_fonts: Vec<(IndexMap<Spritename, AssetSprite>, AssetFont)> = font_properties
        .par_iter()
        .map(|property| {
            bitmapfont_create_from_ttf(
                &property.ttf_path,
                &property.output_dir,
                property.fontsize_in_pixels,
                property.bordered,
                property.color_glyph,
                property.color_border,
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
// Font packing

fn sprite_create_from_glyph_meta(
    sprite_name: &str,
    glyph: &BitmapFontGlyph,
    position_in_font_atlas: Option<Vec2i>,
) -> AssetSprite {
    let (glyph_width, glyph_height, glyph_offset) = if let Some(bitmap) = &glyph.bitmap {
        (bitmap.width, bitmap.height, glyph.offset)
    } else {
        (0, 0, Vec2i::zero())
    };
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

        untrimmed_dimensions: Vec2i::new(glyph_width, glyph_height),

        trimmed_rect: Recti::from_xy_width_height(
            glyph_offset.x,
            glyph_offset.y,
            glyph_width,
            glyph_height,
        ),

        trimmed_uvs: Recti::from_xy_width_height(
            glyph_atlas_pos.x,
            glyph_atlas_pos.y,
            glyph_width,
            glyph_height,
        ),
    }
}

pub fn bitmapfont_create_from_ttf(
    ttf_filepath: &str,
    output_dir: &str,
    fontsize_pixels: u32,
    draw_border: bool,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> (IndexMap<Spritename, AssetSprite>, AssetFont) {
    let fontname = system::path_to_filename_without_extension(ttf_filepath)
        + if draw_border { "_bordered" } else { "" };

    let output_filepath_without_extension = system::path_join(output_dir, &fontname);
    let output_filepath_png = output_filepath_without_extension.to_owned() + ".png";

    let border_thickness = if draw_border { 1 } else { 0 };

    // Create font and atlas
    let ttf_bytes =
        std::fs::read(ttf_filepath).expect(&format!("Cannot read fontdata '{}'", ttf_filepath));
    let font = BitmapFont::new(
        &ttf_bytes,
        fontsize_pixels as i32,
        border_thickness,
        0,
        color_glyph,
        color_border,
    );
    let (font_atlas_texture, font_atlas_glyph_positions) = font.create_atlas(&fontname);
    Bitmap::write_to_png_file(&font_atlas_texture, &output_filepath_png);

    // Create sprites and glyphs
    let mut result_glyphs: IndexMap<Codepoint, AssetGlyph> = IndexMap::new();
    let mut result_sprites: IndexMap<Spritename, AssetSprite> = IndexMap::new();
    for glyph in &font.glyphs {
        let codepoint = glyph.codepoint as Codepoint;
        let sprite_name = BitmapFont::get_glyph_name(&fontname, glyph.codepoint as Codepoint);

        let asset_glyph = AssetGlyph {
            codepoint,
            sprite_name: sprite_name.clone(),
            // NOTE: The `sprite_index` be set later when we finished collecting all
            //       our the sprites
            sprite_index: std::u32::MAX,
            horizontal_advance: glyph.horizontal_advance,
        };
        let sprite_pos = font_atlas_glyph_positions.get(&sprite_name).cloned();
        let sprite = sprite_create_from_glyph_meta(&sprite_name, glyph, sprite_pos);

        result_glyphs.insert(codepoint, asset_glyph);
        result_sprites.insert(sprite_name, sprite);
    }

    // Create Font
    let result_font = AssetFont {
        name: fontname.clone(),
        name_hash: ct_lib::hash_string_64(&fontname),
        baseline: font.baseline,
        vertical_advance: font.vertical_advance,
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

fn convert_glyph(glyph: &AssetGlyph) -> Glyph {
    Glyph {
        horizontal_advance: glyph.horizontal_advance as f32,
        sprite_index: glyph.sprite_index,
    }
}

fn convert_font(font: &AssetFont) -> Font {
    let mut ascii_glyphs: Vec<Glyph> = vec![Glyph::default(); FONT_MAX_NUM_FASTPATH_CODEPOINTS];
    let mut unicode_glyphs: HashMap<Codepoint, Glyph> = HashMap::new();

    for glyph in font.glyphs.values() {
        let codepoint = glyph.codepoint;
        let converted_glyph = convert_glyph(glyph);
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            ascii_glyphs[codepoint as usize] = converted_glyph;
        } else {
            unicode_glyphs.insert(codepoint, converted_glyph);
        }
    }

    Font {
        name: font.name.clone(),
        baseline: font.baseline as f32,
        vertical_advance: font.vertical_advance as f32,
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

    let binary: HashMap<String, Font> = font_map
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
