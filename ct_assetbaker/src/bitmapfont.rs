// TODO: Remove Bitmapfont from this file and use only the one in cottontail lib

use super::{AssetFont, AssetGlyph, AssetSprite, Spritename};

use ct_lib::bitmap::*;
use ct_lib::draw::*;
use ct_lib::math::*;
use ct_lib::system;

use indexmap::IndexMap;
use rayon::prelude::*;

use serde_derive::Deserialize;

pub fn bitmapfont_create_from_ttf(
    ttf_filepath: &str,
    output_dir: &str,
    fontsize_pixels: u32,
    texture_size: u32,
    draw_border: bool,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> (IndexMap<Spritename, AssetSprite>, AssetFont) {
    let fontname = system::path_to_filename_without_extension(ttf_filepath)
        + if draw_border { "_bordered" } else { "" };

    let output_filepath_without_extension = system::path_join(output_dir, &fontname);
    let output_filepath_meta = output_filepath_without_extension.to_owned() + ".fnt";
    let output_filepath_png = output_filepath_without_extension.to_owned() + ".png";

    let padding = if draw_border { 1 } else { 0 };

    run_fontbm_for_ttf(
        ttf_filepath,
        &output_filepath_without_extension,
        fontsize_pixels,
        padding,
        texture_size,
    );

    if draw_border {
        colorize_glyphs_and_draw_borders_in_png(
            &output_filepath_png,
            color_glyph,
            Some(color_border),
        );
    } else {
        colorize_glyphs_and_draw_borders_in_png(&output_filepath_png, color_glyph, None);
    }

    let metadata_string = std::fs::read_to_string(output_filepath_meta).unwrap();
    let meta: BitmapFontJSON = serde_json::from_str(&metadata_string).unwrap();

    // Create sprite names
    let sprite_names: Vec<Spritename> = meta
        .chars
        .iter()
        .map(|glyph_meta| {
            let codepoint = glyph_meta.id;
            sprite_name_for_codepoint(&fontname, codepoint)
        })
        .collect();

    // Create glyphs
    let glyphs: IndexMap<Codepoint, AssetGlyph> = meta
        .chars
        .iter()
        .map(|glyph_meta| {
            let codepoint = glyph_meta.id;
            let sprite_name = sprite_name_for_codepoint(&fontname, codepoint);
            let new_glyph = AssetGlyph {
                codepoint,
                sprite_name,
                // NOTE: The `sprite_index` be set later when we finished collecting all
                //       our the sprites
                sprite_index: std::u32::MAX,
                horizontal_advance: glyph_meta.xadvance,
            };
            (codepoint, new_glyph)
        })
        .collect();

    // Create sprites
    let result_sprites: IndexMap<Spritename, AssetSprite> = meta
        .chars
        .iter()
        .map(|glyph_meta| {
            let codepoint = glyph_meta.id;
            let sprite_name = sprite_name_for_codepoint(&fontname, codepoint);
            let sprite = sprite_create_from_glyph_meta(&sprite_name, glyph_meta);
            (sprite_name, sprite)
        })
        .collect();

    // Create Font
    let result_font = AssetFont {
        name: fontname.clone(),
        name_hash: ct_lib::hash_string_64(&fontname),
        baseline: meta.common.base,
        vertical_advance: meta.common.line_height,
        glyphcount: glyphs.len() as u32,
        glyphs,
        sprite_names,
    };

    (result_sprites, result_font)
}

#[allow(dead_code)]
pub fn test_font_sizes(ttf_filepath: &str, output_dir: &str, fontsize_min: u32, fontsize_max: u32) {
    let fontname = system::path_to_filename_without_extension(ttf_filepath);

    (fontsize_min..=fontsize_max)
        .into_par_iter()
        .for_each(|fontsize| {
            let output_filepath_without_extension =
                system::path_join(output_dir, &format!("{}_{}", fontname, fontsize));

            run_fontbm_for_ttf(
                ttf_filepath,
                &output_filepath_without_extension,
                fontsize,
                1,
                256,
            );
        });
}

pub struct BitmapFontProperties {
    pub ttf_path: String,
    pub output_dir: String,
    pub fontsize_in_pixels: u32,
    pub texture_size: u32,
    pub bordered: bool,
    pub color_glyph: PixelRGBA,
    pub color_border: PixelRGBA,
}

fn sprite_name_for_codepoint(fontname: &str, codepoint: Codepoint) -> Spritename {
    format!("{}_codepoint_{}", fontname, codepoint)
}

fn colorize_glyphs_and_draw_borders_in_png(
    image_path: &str,
    color_glyph: PixelRGBA,
    color_border: Option<PixelRGBA>,
) {
    let mut image = Bitmap::create_from_png_file(image_path);
    assert!(image.width > 0);
    assert!(image.height > 0);

    // Colorize glyphs
    for pixel in image.data.iter_mut() {
        if pixel.a != 0 {
            *pixel = color_glyph;
        }
    }

    if color_border.is_none() {
        Bitmap::write_to_png_file(&image, image_path);
        return;
    }

    // Create a border around every glyph in the image
    let color_border = color_border.unwrap();
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            let pixel_value = image.get(x, y);
            if pixel_value == color_glyph {
                // We landed on a glyph's pixel. We need to paint a border in our neighbouring
                // pixels that are not themselves part of a glyph
                let neighbour_offsets = [
                    Vec2i::new(-1, 0),
                    Vec2i::new(1, 0),
                    Vec2i::new(0, -1),
                    Vec2i::new(0, 1),
                    Vec2i::new(1, 1),
                ];
                for &offset in &neighbour_offsets {
                    let neighbour_pos = Vec2i::new(x, y) + offset;
                    let neighbour_value =
                        image.get_or_default(neighbour_pos.x, neighbour_pos.y, color_glyph);
                    if neighbour_value != color_glyph {
                        image.set_safely(neighbour_pos.x, neighbour_pos.y, color_border);
                    }
                }
            }
        }
    }
    Bitmap::write_to_png_file(&image, image_path);
}

fn sprite_create_from_glyph_meta(sprite_name: &str, glyph: &Char) -> AssetSprite {
    // NOTE: The `atlas_texture_index` and the `trimmed_rect_uv` will be adjusted later when we
    // actually pack the sprites into atlas textures
    AssetSprite {
        name: sprite_name.to_owned(),
        name_hash: ct_lib::hash_string_64(sprite_name),

        has_translucency: false,

        atlas_texture_index: std::u32::MAX,

        pivot_offset: Vec2i::zero(),

        attachment_points: [Vec2i::zero(); SPRITE_ATTACHMENT_POINTS_MAX_COUNT],

        untrimmed_dimensions: Vec2i::new(glyph.width, glyph.height),

        trimmed_rect: Recti::from_xy_width_height(
            glyph.xoffset,
            glyph.yoffset,
            glyph.width,
            glyph.height,
        ),

        trimmed_uvs: Recti::from_xy_width_height(glyph.x, glyph.y, glyph.width, glyph.height),
    }
}

fn run_fontbm_for_ttf(
    ttf_filepath: &str,
    output_filepath_without_extension: &str,
    fontsize_in_pixels: u32,
    padding_in_pixels: u32,
    texture_size: u32,
) {
    let command = String::from("fontbm")
        + " --font-file "
        + ttf_filepath
        + " --chars 0-65536"
        + " --font-size "
        + &fontsize_in_pixels.to_string()
        + " --data-format \"json\""
        + " --padding-up "
        + &padding_in_pixels.to_string()
        + " --padding-right "
        + &padding_in_pixels.to_string()
        + " --padding-down "
        + &padding_in_pixels.to_string()
        + " --padding-left "
        + &padding_in_pixels.to_string()
        + " --texture-width "
        + &texture_size.to_string()
        + " --texture-height "
        + &texture_size.to_string()
        + " --output "
        + output_filepath_without_extension;
    system::run_systemcommand_fail_on_error(&command, false);

    let output_path_meta = output_filepath_without_extension.to_owned() + ".fnt";
    let output_path_png = output_filepath_without_extension.to_owned() + "_0.png";
    let output_path_png_final = output_filepath_without_extension.to_owned() + ".png";
    let output_path_png_overflow = output_filepath_without_extension.to_owned() + "_1.png";

    if !system::path_exists(&output_path_meta) {
        panic!(
            "Failed font conversion: '{}' - '{}' is missing",
            ttf_filepath, output_path_meta
        );
    }
    if !system::path_exists(&output_path_png) {
        panic!(
            "Failed font conversion: '{}' - '{}' is missing",
            ttf_filepath, output_path_png
        );
    }
    if !system::path_exists(&output_path_png) {
        panic!(
            "Failed font conversion: '{}' - something went wrong",
            ttf_filepath
        );
    }
    if system::path_exists(&output_path_png_overflow) {
        panic!(
            "Failed font conversion: '{}' - font did not fit into single texture",
            ttf_filepath
        );
    }
    std::fs::rename(&output_path_png, &output_path_png_final).unwrap();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Generated JSON structs

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct BitmapFontJSON {
    chars: Vec<Char>,
    common: Common,
    info: Info,
    kernings: Vec<::serde_json::Value>,
    pages: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Char {
    chnl: i32,
    height: i32,
    id: i32,
    page: i32,
    width: i32,
    x: i32,
    xadvance: i32,
    xoffset: i32,
    y: i32,
    yoffset: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Common {
    #[serde(rename = "alphaChnl")]
    alpha_chnl: i32,
    base: i32,
    #[serde(rename = "blueChnl")]
    blue_chnl: i32,
    #[serde(rename = "greenChnl")]
    green_chnl: i32,
    #[serde(rename = "lineHeight")]
    line_height: i32,
    packed: bool,
    pages: i32,
    #[serde(rename = "redChnl")]
    red_chnl: i32,
    #[serde(rename = "scaleH")]
    scale_h: i32,
    #[serde(rename = "scaleW")]
    scale_w: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Info {
    aa: i32,
    bold: bool,
    charset: i32,
    face: String,
    italic: bool,
    outline: i32,
    padding: Padding,
    size: i32,
    smooth: bool,
    spacing: Spacing,
    #[serde(rename = "stretchH")]
    stretch_h: i32,
    unicode: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Padding {
    down: i32,
    left: i32,
    right: i32,
    up: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct Spacing {
    horizontal: i32,
    vertical: i32,
}
