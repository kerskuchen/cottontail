use super::bitmap::*;
use super::bitmap_atlas::BitmapAtlas;
use super::color::*;
use super::math::*;
use super::sprite::*;

use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

pub const FONT_DEFAULT_TINY_TTF: &[u8] = include_bytes!("../resources/fonts/ProggyTiny.ttf");
pub const FONT_DEFAULT_TINY_NAME: &str = "default_tiny";
pub const FONT_DEFAULT_TINY_PIXEL_HEIGHT: i32 = 10;
pub const FONT_DEFAULT_TINY_RASTER_OFFSET: Vec2 = Vec2::new(0.0, 0.5);

pub const FONT_DEFAULT_SMALL_TTF: &[u8] = include_bytes!("../resources/fonts/ProggySmall.ttf");
pub const FONT_DEFAULT_SMALL_NAME: &str = "default_small";
pub const FONT_DEFAULT_SMALL_PIXEL_HEIGHT: i32 = 10;
pub const FONT_DEFAULT_SMALL_RASTER_OFFSET: Vec2 = Vec2::new(0.0, 0.5);

pub const FONT_DEFAULT_REGULAR_TTF: &[u8] = include_bytes!("../resources/fonts/ProggyClean.ttf");
pub const FONT_DEFAULT_REGULAR_NAME: &str = "default_regular";
pub const FONT_DEFAULT_REGULAR_PIXEL_HEIGHT: i32 = 13;
pub const FONT_DEFAULT_REGULAR_RASTER_OFFSET: Vec2 = Vec2::new(0.0, 0.5);

pub const FONT_DEFAULT_SQUARE_TTF: &[u8] = include_bytes!("../resources/fonts/ProggySquare.ttf");
pub const FONT_DEFAULT_SQUARE_NAME: &str = "default_square";
pub const FONT_DEFAULT_SQUARE_PIXEL_HEIGHT: i32 = 11;
pub const FONT_DEFAULT_SQUARE_RASTER_OFFSET: Vec2 = Vec2::new(0.0, 0.5);

const FIRST_VISIBLE_ASCII_CODE_POINT: u8 = 32;
const LAST_ASCII_CODE_POINT: u8 = 126;

pub type Codepoint = i32;

#[derive(Clone)]
pub struct BitmapFont {
    pub font_name: String,
    pub font_height_in_pixels: i32,
    pub vertical_advance: i32,
    pub baseline: i32,
    pub glyphs: Vec<BitmapFontGlyph>,
}

impl BitmapFont {
    pub fn new(
        font_name: &str,
        font_ttf_bytes: &[u8],
        font_height: i32,
        font_raster_offset: Vec2,
        border_thickness: u32,
        atlas_padding: u32,
        color_glyph: PixelRGBA,
        color_border: PixelRGBA,
    ) -> BitmapFont {
        let font = rusttype::Font::from_bytes(font_ttf_bytes)
            .expect(&format!("Could not decode font from bytes"));

        // Font metrics
        let (descent, vertical_advance, baseline) = {
            let scale = rusttype::Scale::uniform(font_height as f32);
            let v_metrics = font.v_metrics(scale);
            let ascent = v_metrics.ascent + font_raster_offset.y;
            let descent = v_metrics.descent + font_raster_offset.y;
            let line_gap = v_metrics.line_gap;

            // Check if our vertical metrics are whole numbered. If not then the raster offsets we were
            // given are wrong
            if !is_effectively_zero(ascent - ascent.round())
                || !is_effectively_zero(descent - descent.round())
                || !is_effectively_zero(line_gap - line_gap.round())
            {
                log::warn!(
                    "Vertical metrics of pixelfont '{}' are not whole numbered\nascent: {}\ndescent: {}\nline_gap: {}\nThe given raster offset ({},{}) was not enough to correct this",
                    font_name,
                    v_metrics.ascent,
                    v_metrics.descent,
                    v_metrics.line_gap,
                    font_raster_offset.x,
                    font_raster_offset.y,
                );
            }

            let vertical_advance = ascent - descent + line_gap;
            let baseline = ascent + border_thickness as f32;

            (roundi(descent), roundi(vertical_advance), roundi(baseline))
        };

        // NOTE: We want to turn ASCII characters 0..127 into glyphs but want to treat the
        //       non-displayable characters 0..31 as just whitespace. So we repeat the whitespace
        //       character 32 times and chain it to the remaining ASCII characters.
        //       The reason we want to treat the non-displayable characters as whitespace is that
        //       if we just use their corresponding codepoints, the glyph will draw unwanted
        //       '▯' symbols instead.
        let code_points: Vec<char> = std::iter::repeat(' ')
            .take(FIRST_VISIBLE_ASCII_CODE_POINT as usize)
            .chain(
                (FIRST_VISIBLE_ASCII_CODE_POINT..=LAST_ASCII_CODE_POINT).map(|byte| byte as char),
            )
            .collect();

        // Create glyphs
        let glyphs: Vec<BitmapFontGlyph> = code_points
            .iter()
            .map(|&code_point| {
                BitmapFontGlyph::new(
                    &font,
                    font_name,
                    code_point,
                    font_height,
                    descent,
                    border_thickness as i32,
                    atlas_padding as i32,
                    color_glyph,
                    color_border,
                )
            })
            .collect();

        BitmapFont {
            font_name: font_name.to_owned(),
            font_height_in_pixels: font_height + 2 * border_thickness as i32,
            vertical_advance: vertical_advance + 2 * border_thickness as i32,
            baseline,
            glyphs,
        }
    }

    pub fn get_glyph_name(fontname: &str, codepoint: Codepoint) -> String {
        format!("{}_codepoint_{}", fontname, codepoint)
    }

    pub fn create_atlas(&self, fontname: &str) -> (Bitmap, IndexMap<String, Vec2i>) {
        let mut atlas = BitmapAtlas::new(64);
        for glyph in &self.glyphs {
            if let Some(bitmap) = &glyph.bitmap {
                let spritename = BitmapFont::get_glyph_name(fontname, glyph.codepoint as Codepoint);
                atlas.pack_bitmap_with_resize(&spritename, bitmap);
            }
        }
        atlas
            .atlas_texture
            .trim(false, false, true, true, PixelRGBA::transparent());

        // NOTE: We don't return the atlas itself because we trimmed the atlas texture so that it
        //       may not be in sync with our atlas rectangle packer anymore
        (atlas.atlas_texture, atlas.sprite_positions)
    }

    /// Returns the width and height of a given utf8 text
    pub fn get_text_dimensions(&self, text: &str) -> Vec2i {
        if text.len() == 0 {
            return Vec2i::zero();
        }

        let mut dimensions = Vec2i::new(0, self.vertical_advance);
        let mut pos = Vec2i::new(0, self.baseline);

        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = &self.glyphs[codepoint as usize];
                pos.x += glyph.horizontal_advance;
            } else {
                dimensions.x = i32::max(dimensions.x, pos.x);
                dimensions.y += self.vertical_advance;

                pos.x = 0;
                pos.y += self.vertical_advance;
            }
        }

        // In case we did not find a newline character
        dimensions.x = i32::max(dimensions.x, pos.x);

        dimensions
    }

    /// Returns the bounding rect of a given utf8 text. This ignores whitespace and tries to
    /// wrap the glyphs of the given text as tight as possible.
    pub fn get_text_bounding_rect_exact(&self, text: &str) -> Recti {
        if text.len() == 0 {
            return Recti::zero();
        }

        let mut left = std::i32::MAX;
        let mut top = std::i32::MAX;
        let mut right = 0;
        let mut bottom = 0;

        let mut next_glyph_pos = Vec2i::zero();
        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = &self.glyphs[codepoint as usize];
                if let Some(bitmap) = &glyph.bitmap {
                    let glyph_rect = bitmap.rect().translated_by(next_glyph_pos + glyph.offset);
                    if glyph_rect.left() < left {
                        left = glyph_rect.left();
                    }
                    if glyph_rect.top() < top {
                        top = glyph_rect.top();
                    }
                    if glyph_rect.right() > right {
                        right = glyph_rect.right();
                    }
                    if glyph_rect.bottom() > bottom {
                        bottom = glyph_rect.bottom();
                    }
                }
                next_glyph_pos.x += glyph.horizontal_advance;
            } else {
                next_glyph_pos.x = 0;
                next_glyph_pos.y += self.vertical_advance;
            }
        }

        if left >= right || top >= bottom {
            Recti::zero()
        } else {
            Recti::from_bounds_left_top_right_bottom(left, top, right, bottom)
        }
    }

    pub fn create_text_bitmap(&self, text: &str, background_color: PixelRGBA) -> Bitmap {
        let dim = self.get_text_dimensions(text);
        let mut result = Bitmap::new_filled(dim.x as u32, dim.y as u32, background_color);
        self.draw_text(&mut result, text, Vec2i::zero(), Vec2i::zero());
        result
    }

    pub fn draw_text_aligned_in_point(
        &self,
        image: &mut Bitmap,
        text: &str,
        origin: Vec2i,
        starting_offset: Vec2i,
        alignment_x: AlignmentHorizontal,
        alignment_y: AlignmentVertical,
    ) -> Vec2i {
        let text_dim = self.get_text_dimensions(text);
        let origin_aligned = Vec2i::new(
            block_aligned_in_point(text_dim.x, origin.x, alignment_x),
            block_aligned_in_point(text_dim.y, origin.y, alignment_y),
        );
        self.draw_text(image, text, origin_aligned, starting_offset)
    }

    /// Same as draw_text_aligned_in_point but ignoring whitespace and aligning glyphs as tight
    /// as possible
    pub fn draw_text_aligned_in_point_exact(
        &self,
        image: &mut Bitmap,
        text: &str,
        origin: Vec2i,
        starting_offset: Vec2i,
        alignment_x: AlignmentHorizontal,
        alignment_y: AlignmentVertical,
    ) -> Vec2i {
        let text_rect = self.get_text_bounding_rect_exact(text);
        let text_dim = text_rect.dim;
        let origin_aligned = Vec2i::new(
            block_aligned_in_point(text_dim.x, origin.x, alignment_x),
            block_aligned_in_point(text_dim.y, origin.y, alignment_y),
        ) - text_rect.pos;

        self.draw_text(image, text, origin_aligned, starting_offset)
    }

    /// Draws a given utf8 text to a given bitmap
    /// Returns the starting_offset for the next `draw_text` call
    pub fn draw_text(
        &self,
        image: &mut Bitmap,
        text: &str,
        origin: Vec2i,
        starting_offset: Vec2i,
    ) -> Vec2i {
        let mut pos = starting_offset;
        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = &self.glyphs[codepoint as usize];

                if let Some(glyph_bitmap) = &glyph.bitmap {
                    glyph_bitmap.blit_to(
                        image,
                        origin + pos + glyph.offset,
                        Some(PixelRGBA::transparent()),
                    );
                }

                pos.x += glyph.horizontal_advance;
            } else {
                pos.x = 0;
                pos.y += self.vertical_advance;
            }
        }

        pos
    }

    pub fn test_font_sizes(
        font_name: &str,
        font_ttf_bytes: &[u8],
        font_height_min: i32,
        font_height_max: i32,
        test_image_filepath: &str,
    ) {
        let test_text = "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#$%^&*()-+";
        let text_padding = 16;
        let (mut bitmap, lineskip) = {
            let max_text = format!("{}: {}", font_height_max, test_text,);
            let max_font = BitmapFont::new(
                font_name,
                font_ttf_bytes,
                font_height_max,
                Vec2::zero(),
                0,
                0,
                PixelRGBA::black(),
                PixelRGBA::transparent(),
            );
            let max_text_width = max_font.get_text_dimensions(&max_text).x;
            let max_text_height = max_font.font_height_in_pixels;

            let samplecount = 1 + font_height_max - font_height_min;
            (
                Bitmap::new_filled(
                    (2 * text_padding + max_text_width) as u32,
                    (2 * text_padding + samplecount * (text_padding + max_text_height)) as u32,
                    PixelRGBA::white(),
                ),
                max_text_height,
            )
        };

        for (index, font_height) in (font_height_min..=font_height_max).rev().enumerate() {
            let text = format!("{}: {}", font_height, test_text);
            let font = BitmapFont::new(
                font_name,
                font_ttf_bytes,
                font_height,
                Vec2::zero(),
                0,
                0,
                PixelRGBA::black(),
                PixelRGBA::transparent(),
            );
            let pos = Vec2i::new(
                text_padding,
                text_padding + index as i32 * (lineskip + text_padding),
            );
            font.draw_text(&mut bitmap, &text, pos, Vec2i::zero());
        }

        Bitmap::write_to_png_file(&bitmap, test_image_filepath);
    }
}

#[derive(Clone)]
pub struct BitmapFontGlyph {
    pub codepoint: char,
    pub horizontal_advance: i32,
    pub offset: Vec2i,
    pub bitmap: Option<Bitmap>,
}

impl BitmapFontGlyph {
    pub fn new(
        font: &rusttype::Font,
        font_name: &str,
        codepoint: char,
        font_height: i32,
        descent: i32,
        border_thickness: i32,
        atlas_padding: i32,
        color_glyph: PixelRGBA,
        color_border: PixelRGBA,
    ) -> BitmapFontGlyph {
        let glyph = font
            .glyph(codepoint)
            .standalone()
            .scaled(rusttype::Scale::uniform(font_height as f32))
            // NOTE: We pre-position the glyph such that it vertically fits into the
            //       interval [0, pixel_text_height - 1], where 0 is a glyphs highest possible
            //       point, (pixel_text_height - 1) is a glyphs lowest possible point and
            //       (pixel_text_height - 1 + pixel_descent) represents the fonts baseline.
            .positioned(rusttype::point(0.0, (descent + font_height) as f32));

        // Glyph metrics
        let h_metrics = glyph.unpositioned().h_metrics();
        let advance_width = h_metrics.advance_width.round() as i32;
        let left_side_bearing = h_metrics.left_side_bearing.round() as i32;

        // Check if our horizontal metrics are whole numbered. If not then the raster offsets we were
        // given are wrong
        if !is_effectively_zero(advance_width as f32 - h_metrics.advance_width)
            || !is_effectively_zero(left_side_bearing as f32 - h_metrics.left_side_bearing)
        {
            log::warn!(
                "Horizontal metrics of pixelfont glyph '{}' are not whole numbered\nadvance_width: {}\nleft_side_bearing: {}",
                h_metrics.advance_width,
                BitmapFont::get_glyph_name(font_name, codepoint as Codepoint),
                h_metrics.left_side_bearing,
            );
        }

        let horizontal_advance = advance_width + border_thickness;
        // NOTE: The offset determines how many pixels the glyph-sprite needs to be offset
        //       from its origin (top-left corner) when drawn to the screen
        let offset_x = left_side_bearing - atlas_padding as i32;
        let mut offset_y = -(atlas_padding as i32);

        let maybe_image = create_glyph_image(
            &glyph,
            border_thickness as u32,
            atlas_padding as u32,
            color_glyph,
            color_border,
        );
        if maybe_image.is_some() {
            // NOTE: We can unwrap here because otherwise `maybe_image` would be `None` anyway
            let bounding_box = glyph.pixel_bounding_box().unwrap();
            // NOTE: We don't do `offset_x += bounding_box.min.x;` here because we already added
            //       the left side bearing when we initialized `offset_x`
            offset_y += bounding_box.min.y;
        }

        BitmapFontGlyph {
            codepoint: codepoint,
            horizontal_advance,
            offset: Vec2i::new(offset_x, offset_y),
            bitmap: maybe_image,
        }
    }
}

fn create_glyph_image(
    glyph: &rusttype::PositionedGlyph,
    border_thickness: u32,
    image_padding: u32,
    color_glyph: PixelRGBA,
    color_border: PixelRGBA,
) -> Option<Bitmap> {
    glyph.pixel_bounding_box().map(|bounding_box| {
        let mut image = Bitmap::new_filled(
            bounding_box.width() as u32 + 2 * u32::from(image_padding + border_thickness),
            bounding_box.height() as u32 + 2 * u32::from(image_padding + border_thickness),
            PixelRGBA::new(0, 0, 0, 0),
        );

        glyph.draw(|x, y, v| {
            // NOTE: We only use the values that are above 50% opacity and draw them with full
            //       intensity. This way we get nice and crisp edges and a uniform color.
            // WARNING: This only works for pixel-fonts. Regular fonts are not supported
            if v > 0.5 {
                image.set(
                    (x + image_padding + border_thickness) as i32,
                    (y + image_padding + border_thickness) as i32,
                    color_glyph,
                )
            }
        });

        if border_thickness != 0 {
            if border_thickness == 1 {
                draw_glyph_border(&mut image, color_glyph, color_border);
            } else {
                unimplemented!("We only support borders with thickness 1 for now")
            }
        }

        image
    })
}

fn draw_glyph_border(image: &mut Bitmap, color_glyph: PixelRGBA, color_border: PixelRGBA) {
    // Create a border around every glyph in the image
    for y in 0..image.height {
        for x in 0..image.width {
            let pixel_val = image.get(x, y);
            if pixel_val == color_glyph {
                // We landed on a glyph's pixel. We need to paint a border in our neighbouring
                // pixels that are not themselves part of a glyph
                let pairs = vec![(-1, 0), (1, 0), (0, -1), (0, 1), (1, 1)];
                for pair in pairs {
                    let neighbor_x = x + pair.0;
                    let neighbor_y = y + pair.1;
                    let neighbor_pixel_val =
                        image.get_or_default(neighbor_x, neighbor_y, color_glyph);

                    if neighbor_pixel_val != color_glyph {
                        image.set(neighbor_x, neighbor_y, color_border);
                    }
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn get_default_font() -> BitmapFont {
        BitmapFont::new(
            FONT_DEFAULT_TINY_NAME,
            FONT_DEFAULT_TINY_TTF,
            FONT_DEFAULT_TINY_PIXEL_HEIGHT,
            FONT_DEFAULT_TINY_RASTER_OFFSET,
            0,
            0,
            PixelRGBA::black(),
            PixelRGBA::transparent(),
        )
    }

    #[test]
    fn aligned_text_drawing_in_point() {
        let font = get_default_font();
        let text = "hello, good day!";
        let bitmap_width = 256;
        let bitmap_height = 256;
        let bitmap_center = Vec2i::new(bitmap_width, bitmap_height) / 2;

        let mut bitmaps = Vec::new();
        for alignment_x in &[
            AlignmentHorizontal::Left,
            AlignmentHorizontal::Center,
            AlignmentHorizontal::Right,
        ] {
            for alignment_y in &[
                AlignmentVertical::Top,
                AlignmentVertical::Center,
                AlignmentVertical::Bottom,
            ] {
                let mut bitmap = Bitmap::new(bitmap_width as u32, bitmap_height as u32);
                font.draw_text_aligned_in_point(
                    &mut bitmap,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    AlignmentHorizontal::Center,
                    AlignmentVertical::Top,
                );
                font.draw_text_aligned_in_point(
                    &mut bitmap,
                    text,
                    bitmap_center,
                    Vec2i::zero(),
                    *alignment_x,
                    *alignment_y,
                );
                bitmap.set(bitmap_center.x, bitmap_center.y, PixelRGBA::magenta());
                bitmaps.push(bitmap);
            }
        }

        let final_bitmap =
            Bitmap::glue_together_multiple(&bitmaps, GluePosition::RightTop, 1, PixelRGBA::black());
        Bitmap::write_to_png_file(&final_bitmap, "tests/bitmapfont_aligned_text_drawing.png");
    }

    #[test]
    fn aligned_text_drawing_in_point_tight() {
        let font = get_default_font();
        let text = "
                   
            aaaa   
            gggg   
            gggg   
                   
        ";
        let bitmap_width = 128;
        let bitmap_height = 128;
        let bitmap_center = Vec2i::new(bitmap_width, bitmap_height) / 2;

        let mut bitmaps = Vec::new();
        for alignment_x in &[
            AlignmentHorizontal::Left,
            AlignmentHorizontal::Center,
            AlignmentHorizontal::Right,
        ] {
            for alignment_y in &[
                AlignmentVertical::Top,
                AlignmentVertical::Center,
                AlignmentVertical::Bottom,
            ] {
                let mut bitmap = Bitmap::new(bitmap_width as u32, bitmap_height as u32);
                font.draw_text_aligned_in_point(
                    &mut bitmap,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    AlignmentHorizontal::Center,
                    AlignmentVertical::Top,
                );
                font.draw_text_aligned_in_point_exact(
                    &mut bitmap,
                    text,
                    bitmap_center,
                    Vec2i::zero(),
                    *alignment_x,
                    *alignment_y,
                );
                bitmap.set(bitmap_center.x, bitmap_center.y, PixelRGBA::magenta());
                bitmaps.push(bitmap);
            }
        }

        let final_bitmap =
            Bitmap::glue_together_multiple(&bitmaps, GluePosition::RightTop, 1, PixelRGBA::black());
        Bitmap::write_to_png_file(
            &final_bitmap,
            "tests/bitmapfont_aligned_text_drawing_exact.png",
        );
    }

    #[test]
    fn aligned_text_drawing_in_point_tight_single_glyph() {
        let font = get_default_font();
        let text = "a";
        let bitmap_width = 128;
        let bitmap_height = 128;
        let bitmap_center = Vec2i::new(bitmap_width, bitmap_height) / 2;

        let mut bitmaps = Vec::new();
        for alignment_x in &[
            AlignmentHorizontal::Left,
            AlignmentHorizontal::Center,
            AlignmentHorizontal::Right,
        ] {
            for alignment_y in &[
                AlignmentVertical::Top,
                AlignmentVertical::Center,
                AlignmentVertical::Bottom,
            ] {
                let mut bitmap = Bitmap::new(bitmap_width as u32, bitmap_height as u32);
                font.draw_text_aligned_in_point(
                    &mut bitmap,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    AlignmentHorizontal::Center,
                    AlignmentVertical::Top,
                );
                font.draw_text_aligned_in_point_exact(
                    &mut bitmap,
                    text,
                    bitmap_center,
                    Vec2i::zero(),
                    *alignment_x,
                    *alignment_y,
                );
                bitmap.set(bitmap_center.x, bitmap_center.y, PixelRGBA::magenta());
                bitmaps.push(bitmap);
            }
        }

        let final_bitmap =
            Bitmap::glue_together_multiple(&bitmaps, GluePosition::RightTop, 1, PixelRGBA::black());
        Bitmap::write_to_png_file(
            &final_bitmap,
            "tests/bitmapfont_aligned_text_drawing_exact_single_glyph.png",
        );
    }
}

//--------------------------------------------------------------------------------------------------
// SpriteFont

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SpriteGlyph {
    pub horizontal_advance: f32,
    pub sprite_index: SpriteIndex,
}

pub const FONT_MAX_NUM_FASTPATH_CODEPOINTS: usize = 256;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteFont {
    pub name: String,

    pub baseline: f32,
    pub vertical_advance: f32,

    /// Fastpath glyphs for quick access (mainly latin glyphs)
    pub ascii_glyphs: Vec<SpriteGlyph>,
    /// Non-fastpath unicode glyphs for codepoints > FONT_MAX_NUM_FASTPATH_CODEPOINTS
    pub unicode_glyphs: HashMap<Codepoint, SpriteGlyph>,
}

impl SpriteFont {
    #[inline]
    pub fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            self.ascii_glyphs[codepoint as usize]
        } else {
            *self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs['?' as usize])
        }
    }

    /// Returns width and height of a given utf8 text for a given font and font scale.
    pub fn get_text_dimensions(&self, font_scale: f32, text: &str) -> Vec2 {
        if text.len() == 0 {
            return Vec2::zero();
        }

        let mut dimensions = Vec2::new(0.0, font_scale * self.vertical_advance);
        let mut pos = Vec2::new(0.0, font_scale * self.baseline);

        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = self.get_glyph_for_codepoint(codepoint as i32);
                pos.x += font_scale * glyph.horizontal_advance;
            } else {
                dimensions.x = f32::max(dimensions.x, pos.x);
                dimensions.y += font_scale * self.vertical_advance;

                pos.x = 0.0;
                pos.y += font_scale * self.vertical_advance;
            }
        }

        // In case we did not find a newline character
        dimensions.x = f32::max(dimensions.x, pos.x);

        dimensions
    }

    pub fn get_text_width(&self, font_scale: f32, text: &str) -> f32 {
        let mut text_width = 0.0;
        for codepoint in text.chars() {
            let glyph = self.get_glyph_for_codepoint(codepoint as i32);
            text_width += font_scale * glyph.horizontal_advance;
        }
        text_width
    }

    pub fn get_text_height(font: &SpriteFont, font_scale: f32, linecount: usize) -> f32 {
        assert!(linecount > 0);
        (font_scale * font.baseline) + (linecount - 1) as f32 * font_scale * font.vertical_advance
    }
}

/*

//--------------------------------------------------------------------------------------------------
// Text drawing

/// Returns width and height of a given utf8 text for a given font and font scale.
/// NOTE: This returns a more accurate dimension than `font_get_text_dimensions` which calculates
///       the text-width based on the horizontal advance. This function on the other hand calculates
///       the text width based on the actual glyph bitmap widths.
pub fn get_text_dimensions(&mut self, font: &SpriteFont, font_scale: f32, text: &str) -> Vec2 {
    if text.len() == 0 {
        return Vec2::zero();
    }

    let mut dimensions = Vec2::new(0.0, font_scale * font.vertical_advance);
    let mut pos = Vec2::new(0.0, font_scale * font.baseline);

    for codepoint in text.chars() {
        if codepoint != '\n' {
            let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);

            let sprite = self.get_sprite_by_index(glyph.sprite_index);
            let sprite_width = sprite.trimmed_rect.width() * font_scale;
            dimensions.x = f32::max(dimensions.x, pos.x + sprite_width);

            pos.x += font_scale * glyph.horizontal_advance;
        } else {
            pos.x = 0.0;
            pos.y += font_scale * font.vertical_advance;
            dimensions.y += font_scale * font.vertical_advance;
        }
    }

    dimensions
}

/// Draws a given utf8 text with a given font
/// Returns the starting_offset for the next `text` or `text_formatted` call
pub fn draw_text(
    &mut self,
    text: &str,
    font: &SpriteFont,
    font_scale: f32,
    starting_origin: Vec2,
    starting_offset: Vec2,
    origin_is_baseline: bool,
    depth: Depth,
    color_modulate: Color,
    additivity: Additivity,
) -> Vec2 {
    let mut origin = worldpoint_pixel_snapped(starting_origin);
    if origin_is_baseline {
        // NOTE: Ascent will be drawn above the origin and descent below the origin
        origin.y -= font_scale * font.baseline;
    } else {
        // NOTE: Everything is drawn below the origin
    }

    let mut pos = starting_offset;
    for codepoint in text.chars() {
        if codepoint != '\n' {
            let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);

            self.draw_sprite_pixel_snapped(
                SpriteBy::Index(glyph.sprite_index),
                origin + pos,
                Vec2::new(font_scale, font_scale),
                Vec2::unit_x(),
                false,
                false,
                depth,
                color_modulate,
                additivity,
            );

            pos.x += font_scale * glyph.horizontal_advance;
        } else {
            pos.x = 0.0;
            pos.y += font_scale * font.vertical_advance;
        }
    }

    pos
}

/// Draws a given utf8 text in a given font using a clipping rectangle
/// NOTE: The given text should be already pre-wrapped for a good result
pub fn draw_text_clipped(
    &mut self,
    text: &str,
    font: &SpriteFont,
    font_scale: f32,
    starting_origin: Vec2,
    starting_offset: Vec2,
    origin_is_baseline: bool,
    clipping_rect: Rect,
    depth: Depth,
    color_modulate: Color,
    additivity: Additivity,
) {
    let mut origin = worldpoint_pixel_snapped(starting_origin);
    if origin_is_baseline {
        // NOTE: Ascent will be drawn above the origin and descent below the origin
        origin.y -= font_scale * font.baseline;
    } else {
        // NOTE: Everything is drawn below the origin
    }

    // Check if we would begin drawing below our clipping rectangle
    let mut current_line_top = origin.y - font_scale * font.baseline;
    let mut current_line_bottom = current_line_top + font.vertical_advance;
    current_line_top += starting_offset.y;
    current_line_bottom += starting_offset.y;
    if current_line_top > clipping_rect.bottom() {
        // NOTE: Our text begins past the lower border of the bounding rect and all following
        //       lines would not be visible anymore
        return;
    }

    let mut pos = starting_offset;
    for line in text.lines() {
        // Skip lines until we are within our bounding rectangle
        //
        if current_line_bottom >= clipping_rect.top() {
            for codepoint in line.chars() {
                let glyph = font.get_glyph_for_codepoint(codepoint as Codepoint);
                self.draw_sprite_clipped(
                    SpriteBy::Index(glyph.sprite_index),
                    origin + pos,
                    Vec2::new(font_scale, font_scale),
                    clipping_rect,
                    depth,
                    color_modulate,
                    additivity,
                );

                pos.x += font_scale * glyph.horizontal_advance;
            }
        }

        // We finished a line and need advance to the next line
        pos.x = 0.0;
        pos.y += font_scale * font.vertical_advance;

        current_line_top += font_scale * font.vertical_advance;
        current_line_bottom += font_scale * font.vertical_advance;
        if clipping_rect.bottom() <= current_line_top {
            // NOTE: We skipped past the lower border of the bounding rect and all following
            //       lines will not be visible anymore
            return;
        }
    }
}

*/
