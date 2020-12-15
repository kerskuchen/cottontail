use super::bitmap::*;
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

pub type Codepoint = i32;

pub const FONT_MAX_NUM_FASTPATH_CODEPOINTS: usize = 256;
const FIRST_VISIBLE_ASCII_CODE_POINT: Codepoint = 32;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Font and Glyph traits

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TextAlignment {
    pub x: AlignmentHorizontal,
    pub y: AlignmentVertical,
    pub origin_is_baseline: bool,
    pub ignore_whitespace: bool,
}

pub trait Glyph {
    fn get_bitmap_rect(&self) -> Recti;
    fn horizontal_advance(&self) -> i32;
}

pub trait Font<GlyphType: Glyph> {
    fn baseline(&self) -> i32;
    fn vertical_advance(&self) -> i32;
    fn horizontal_advance_max(&self) -> i32;
    fn font_height_in_pixels(&self) -> i32;
    fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> &GlyphType;
    fn get_glyph_for_codepoint_copy(&self, codepoint: Codepoint) -> GlyphType;

    fn get_glyph_name(fontname: &str, codepoint: Codepoint) -> String {
        format!("{}_codepoint_{}", fontname, codepoint)
    }

    /// Returns the bounding rect for a given utf8 text for a given scale.
    /// For `ignore_whitespace = true` it ignores whitespace and tries to wrap the bounding rect to
    /// the non-whitespace-glyphs as tight as possible.
    fn get_text_bounding_rect(
        &self,
        text: &str,
        font_scale: i32,
        ignore_whitespace: bool,
    ) -> Recti {
        if text.len() == 0 {
            return Recti::zero();
        }

        if ignore_whitespace {
            // NOTE: We don't start at (0,0) because the first glyph may be a whitespace which we don't
            //       want contained in our rect
            let mut left = std::i32::MAX;
            let mut top = std::i32::MAX;
            let mut right = -std::i32::MAX;
            let mut bottom = -std::i32::MAX;

            self.iter_text_glyphs(
                text,
                font_scale,
                Vec2i::zero(),
                Vec2i::zero(),
                false,
                &mut |glyph, draw_pos, codepoint| {
                    // Ignore empty or whitespace glyphs
                    if codepoint.is_whitespace() {
                        return;
                    }
                    let glyph_rect = glyph.get_bitmap_rect();
                    if glyph_rect == Recti::zero() {
                        return;
                    }

                    let glyph_rect_transformed = Recti::from_pos_dim(
                        draw_pos + font_scale * glyph_rect.pos,
                        font_scale * glyph_rect.dim,
                    );
                    if glyph_rect_transformed.left() < left {
                        left = glyph_rect_transformed.left();
                    }
                    if glyph_rect_transformed.top() < top {
                        top = glyph_rect_transformed.top();
                    }
                    if glyph_rect_transformed.right() > right {
                        right = glyph_rect_transformed.right();
                    }
                    if glyph_rect_transformed.bottom() > bottom {
                        bottom = glyph_rect_transformed.bottom();
                    }
                },
            );

            if left >= right || top >= bottom {
                Recti::zero()
            } else {
                Recti::from_bounds_left_top_right_bottom(left, top, right, bottom)
            }
        } else {
            let mut left = 0;
            let mut top = 0;
            let mut right = 0;
            let mut bottom = 0;

            let final_pos = self.iter_text_glyphs(
                text,
                font_scale,
                Vec2i::zero(),
                Vec2i::zero(),
                false,
                &mut |glyph, draw_pos, _codepoint| {
                    left = left.min(draw_pos.x);
                    right = right.max(draw_pos.x + font_scale * glyph.horizontal_advance());

                    // NOTE: For top and bottom we need to look at the actual glyphs as they might be
                    //       weirdly positioned vertically. For example there are glyphs with
                    //       `y_offset = -1` so it is not always correct to have `rect.top >= 0`.
                    let rect = glyph.get_bitmap_rect();
                    top = top.min(draw_pos.y + font_scale * rect.top());
                    bottom = bottom
                        .max(draw_pos.y + font_scale * self.baseline())
                        .max(draw_pos.y + font_scale * rect.bottom());
                },
            );

            // In case we got trailing newlines
            bottom = bottom.max(final_pos.y);

            Recti::from_bounds_left_top_right_bottom(left, top, right, bottom)
        }
    }

    /// Iterates a given utf8 text and runs a given operation on each glyph.
    /// Returns the starting_offset for the next `iter_text_glyphs` call
    fn iter_text_glyphs<Operation: core::ops::FnMut(&GlyphType, Vec2i, char) -> ()>(
        &self,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        origin_is_baseline: bool,
        operation: &mut Operation,
    ) -> Vec2i {
        let mut origin = origin;
        if origin_is_baseline {
            // NOTE: Ascent will be drawn above the origin and descent below the origin
            origin.y -= font_scale * self.baseline();
        } else {
            // NOTE: Everything is drawn below the origin
        }

        let mut pos = starting_offset;
        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = self.get_glyph_for_codepoint(codepoint as Codepoint);
                let draw_pos = origin + pos;

                operation(glyph, draw_pos, codepoint);

                pos.x += font_scale * glyph.horizontal_advance();
            } else {
                pos.x = 0;
                pos.y += font_scale * self.vertical_advance();
            }
        }

        pos
    }

    /// Iterates a given utf8 text and runs a given operation on each glyph if it would be visible
    /// in the given clipping rect
    fn iter_text_glyphs_clipped<Operation: core::ops::FnMut(&GlyphType, Vec2i, char) -> ()>(
        &self,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        origin_is_baseline: bool,
        clipping_rect: Recti,
        operation: &mut Operation,
    ) {
        let mut origin = origin;
        if origin_is_baseline {
            // NOTE: Ascent will be drawn above the origin and descent below the origin
            origin.y -= font_scale * self.baseline();
        } else {
            // NOTE: Everything is drawn below the origin
        }

        // Check if we would begin drawing below our clipping rectangle
        let mut current_line_top = origin.y - font_scale * self.baseline();
        let mut current_line_bottom = current_line_top + self.vertical_advance();
        current_line_top += starting_offset.y;
        current_line_bottom += starting_offset.y;
        if current_line_top > clipping_rect.bottom() {
            // NOTE: Our text begins past the lower border of the bounding rect and all following
            //       lines would not be visible anymore
            return;
        }

        let mut pos = starting_offset;
        for codepoint in text.chars() {
            if codepoint != '\n' {
                let glyph = self.get_glyph_for_codepoint(codepoint as Codepoint);
                let draw_pos = origin + pos;

                operation(&glyph, draw_pos, codepoint);

                pos.x += font_scale * glyph.horizontal_advance();
            } else {
                pos.x = 0;
                pos.y += font_scale * self.vertical_advance();
            }
        }

        let mut pos = starting_offset;
        for line in text.lines() {
            // Skip lines until we are within our bounding rectangle
            //
            if current_line_bottom >= clipping_rect.top() {
                for codepoint in line.chars() {
                    let glyph = self.get_glyph_for_codepoint(codepoint as Codepoint);
                    let draw_pos = origin + pos;

                    operation(glyph, draw_pos, codepoint);

                    pos.x += font_scale * glyph.horizontal_advance();
                }
            }

            // We finished a line and need advance to the next line
            pos.x = 0;
            pos.y += font_scale * self.vertical_advance();

            current_line_top += font_scale * self.vertical_advance();
            current_line_bottom += font_scale * self.vertical_advance();
            if clipping_rect.bottom() <= current_line_top {
                // NOTE: We skipped past the lower border of the bounding rect and all following
                //       lines will not be visible anymore
                return;
            }
        }
    }

    fn iter_text_glyphs_aligned_in_point<
        Operation: core::ops::FnMut(&GlyphType, Vec2i, char) -> (),
    >(
        &self,
        text: &str,
        font_scale: i32,
        origin: Vec2i,
        starting_offset: Vec2i,
        alignment: Option<TextAlignment>,
        operation: &mut Operation,
    ) -> Vec2i {
        let (origin_aligned, origin_is_baseline) = if let Some(alignment) = alignment {
            let rect = self.get_text_bounding_rect(text, font_scale, alignment.ignore_whitespace);
            let origin_aligned = if alignment.ignore_whitespace {
                Vec2i::new(
                    block_aligned_in_point(rect.dim.x, origin.x, alignment.x),
                    block_aligned_in_point(rect.dim.y, origin.y, alignment.y),
                ) - rect.pos
            } else {
                Vec2i::new(
                    block_aligned_in_point(rect.dim.x, origin.x, alignment.x),
                    block_aligned_in_point(rect.dim.y, origin.y, alignment.y),
                )
            };

            (origin_aligned, alignment.origin_is_baseline)
        } else {
            (origin, false)
        };

        self.iter_text_glyphs(
            text,
            font_scale,
            origin_aligned,
            starting_offset.into(),
            origin_is_baseline,
            operation,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BitmapFont

#[derive(Clone)]
pub struct BitmapFont {
    pub font_name: String,
    pub font_height_in_pixels: i32,
    pub vertical_advance: i32,
    pub horizontal_advance_max: i32,
    pub baseline: i32,
    pub glyphs: IndexMap<Codepoint, BitmapGlyph>,
}

impl Font<BitmapGlyph> for BitmapFont {
    fn baseline(&self) -> i32 {
        self.baseline
    }
    fn vertical_advance(&self) -> i32 {
        self.vertical_advance
    }
    fn horizontal_advance_max(&self) -> i32 {
        self.horizontal_advance_max
    }
    fn font_height_in_pixels(&self) -> i32 {
        self.font_height_in_pixels
    }
    fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> &BitmapGlyph {
        self.glyphs
            .get(&codepoint)
            .or_else(|| self.glyphs.get(&0))
            .or_else(|| self.glyphs.get(&('?' as Codepoint)))
            .unwrap()
    }
    fn get_glyph_for_codepoint_copy(&self, codepoint: Codepoint) -> BitmapGlyph {
        self.get_glyph_for_codepoint(codepoint).clone()
    }
}

impl BitmapFont {
    pub fn new(
        font_name: &str,
        font_ttf_bytes: &[u8],
        font_height: i32,
        font_raster_offset: Vec2,
        border_thickness: i32,
        atlas_padding: i32,
        color_glyph: PixelRGBA,
        color_border: PixelRGBA,
    ) -> BitmapFont {
        let font = rusttype::Font::try_from_bytes(font_ttf_bytes)
            .expect(&format!("Could not decode font from bytes"));

        // Font metrics
        let (descent, vertical_advance, baseline) = {
            let scale = rusttype::Scale::uniform(font_height as f32);
            let v_metrics = font.v_metrics(scale);
            let ascent = v_metrics.ascent + font_raster_offset.y;
            let descent = v_metrics.descent + font_raster_offset.y;
            let line_gap = v_metrics.line_gap;

            let ascent_integer = roundi(ascent);
            let descent_integer = roundi(descent);
            let line_gap_integer = roundi(line_gap);

            // Check if our vertical metrics are whole numbered. If not then the raster offsets we
            // were given are wrong
            if !is_effectively_zero(ascent - ascent_integer as f32)
                || !is_effectively_zero(descent - descent_integer as f32)
                || !is_effectively_zero(line_gap - line_gap_integer as f32)
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

            // Check if our ascent + descent add up to the font height. If not then the raster
            // offsets we were given are wrong
            if font_height != (i32::abs(ascent_integer) + i32::abs(descent_integer)) {
                log::warn!(
                    "Fontheight and (ascent + descent) of pixelfont '{}' do not match\nascent: {}\ndescent: {}\nascent + descent: {}\nfont height: {}\nThe given raster offset ({},{}) was probably wrong",
                    font_name,
                    i32::abs(ascent_integer),
                    i32::abs(descent_integer),
                    i32::abs(ascent_integer) + i32::abs(descent_integer),
                    font_height,
                    font_raster_offset.x,
                    font_raster_offset.y,
                );
            }

            let vertical_advance = ascent_integer - descent_integer + line_gap_integer;
            let baseline = ascent_integer + border_thickness;

            (descent_integer, vertical_advance, baseline)
        };

        // Create glyphs
        let mut glyphs: IndexMap<Codepoint, BitmapGlyph> = IndexMap::new();
        for index in 0..std::u16::MAX as Codepoint {
            let codepoint = if index < 0 && index < FIRST_VISIBLE_ASCII_CODE_POINT {
                // NOTE: We want to turn ASCII characters 0..65535 into glyphs but want to treat the
                //       non-displayable characters 1..31 as just whitespace. So we repeat the whitespace
                //       character 32 times and chain it to the remaining ASCII characters.
                //       The reason we want to treat the non-displayable characters as whitespace is that
                //       if we just use their corresponding codepoints, the glyph will draw unwanted
                //       '▯' symbols instead.
                ' ' as Codepoint
            } else {
                index
            };

            let character = {
                let maybe_char = std::char::from_u32(codepoint as u32);
                if maybe_char.is_none() {
                    continue;
                } else {
                    maybe_char.unwrap()
                }
            };

            let glyph = font.glyph(character);
            if glyph.id() == rusttype::GlyphId(0) {
                // This glyph does not exist in the given font
                continue;
            }

            let glyph = BitmapGlyph::new(
                &font,
                font_name,
                character,
                font_height,
                descent,
                border_thickness as i32,
                atlas_padding as i32,
                color_glyph,
                color_border,
            );

            glyphs.insert(codepoint as Codepoint, glyph);
        }

        let horizontal_advance_max = glyphs
            .values()
            .map(|glyph| glyph.horizontal_advance)
            .max()
            .expect(&format!(
                "Pixelfont '{}' does not contain any glyphs",
                font_name
            ));

        BitmapFont {
            font_name: font_name.to_owned(),
            font_height_in_pixels: font_height + 2 * border_thickness as i32,
            vertical_advance: vertical_advance + 2 * border_thickness as i32,
            horizontal_advance_max: horizontal_advance_max + 2 * border_thickness as i32,
            baseline,
            glyphs,
        }
    }

    pub fn to_bitmap_atlas(&self, fontname: &str) -> (Bitmap, IndexMap<String, Vec2i>) {
        let mut atlas = BitmapAtlas::new(64);
        for glyph in self.glyphs.values() {
            if let Some(bitmap) = &glyph.bitmap {
                let spritename = BitmapFont::get_glyph_name(fontname, glyph.codepoint as Codepoint);
                atlas.pack_bitmap_with_resize(&spritename, bitmap);
            }
        }
        atlas
            .atlas_texture
            .trim_by_value(false, false, true, true, PixelRGBA::transparent());

        // NOTE: We don't return the atlas itself because we trimmed the atlas texture so that it
        //       may not be in sync with our atlas rectangle packer anymore
        (atlas.atlas_texture, atlas.sprite_positions)
    }

    pub fn test_font_sizes(
        font_name: &str,
        font_ttf_bytes: &[u8],
        font_raster_offset: Vec2,
        font_height_min: i32,
        font_height_max: i32,
        test_image_filepath: &str,
    ) {
        let test_text = "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!@#$%^&*()-+";
        let text_padding = 16;

        let mut bitmaps = Vec::new();
        for font_height in font_height_min..=font_height_max {
            let text = format!("{}: {}", font_height, test_text);
            let font = BitmapFont::new(
                font_name,
                font_ttf_bytes,
                font_height,
                font_raster_offset,
                0,
                0,
                PixelRGBA::black(),
                PixelRGBA::transparent(),
            );
            bitmaps.push(
                Bitmap::create_from_text(&font, &text, 1, PixelRGBA::white()).extended(
                    text_padding,
                    text_padding,
                    text_padding,
                    text_padding,
                    PixelRGBA::white(),
                ),
            );
        }
        let bitmap = Bitmap::glue_together_multiple(
            &bitmaps,
            GluePosition::BottomLeft,
            0,
            PixelRGBA::white(),
        );
        Bitmap::write_to_png_file(&bitmap, test_image_filepath);
    }
}

#[derive(Clone)]
pub struct BitmapGlyph {
    pub codepoint: char,
    pub horizontal_advance: i32,
    pub offset: Vec2i,
    pub bitmap: Option<Bitmap>,
}

impl Glyph for BitmapGlyph {
    fn get_bitmap_rect(&self) -> Recti {
        if let Some(bitmap) = &self.bitmap {
            bitmap.rect().translated_by(self.offset)
        } else {
            Recti::zero()
        }
    }

    fn horizontal_advance(&self) -> i32 {
        self.horizontal_advance
    }
}

impl BitmapGlyph {
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
    ) -> BitmapGlyph {
        let glyph = font
            .glyph(codepoint)
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

        BitmapGlyph {
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
// SpriteFont

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteGlyph {
    pub horizontal_advance: i32,
    pub sprite: Sprite,

    /// This is mainly used for text dimension calculations
    pub sprite_dimensions: Vec2i,
    /// This is mainly used for text dimension calculations
    pub sprite_draw_offset: Vec2i,
}

impl Glyph for SpriteGlyph {
    fn get_bitmap_rect(&self) -> Recti {
        Recti::from_pos_dim(self.sprite_draw_offset, self.sprite_dimensions)
    }

    fn horizontal_advance(&self) -> i32 {
        self.horizontal_advance
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpriteFont {
    pub name: String,

    pub baseline: i32,
    pub vertical_advance: i32,
    pub horizontal_advance_max: i32,
    pub font_height_in_pixels: i32,

    /// Fastpath glyphs for quick access (mainly latin glyphs)
    pub ascii_glyphs: Vec<SpriteGlyph>,
    /// Non-fastpath unicode glyphs for codepoints > FONT_MAX_NUM_FASTPATH_CODEPOINTS
    pub unicode_glyphs: HashMap<Codepoint, SpriteGlyph>,
}

impl Font<SpriteGlyph> for SpriteFont {
    fn baseline(&self) -> i32 {
        self.baseline
    }
    fn vertical_advance(&self) -> i32 {
        self.vertical_advance
    }
    fn horizontal_advance_max(&self) -> i32 {
        self.horizontal_advance_max
    }
    fn font_height_in_pixels(&self) -> i32 {
        self.font_height_in_pixels
    }
    fn get_glyph_for_codepoint_copy(&self, codepoint: Codepoint) -> SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            self.ascii_glyphs[codepoint as usize].clone()
        } else {
            let result = self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs[0usize]);
            if result.sprite.name != "" {
                result.clone()
            } else {
                self.ascii_glyphs['?' as usize].clone()
            }
        }
    }
    fn get_glyph_for_codepoint(&self, codepoint: Codepoint) -> &SpriteGlyph {
        if codepoint < FONT_MAX_NUM_FASTPATH_CODEPOINTS as i32 {
            &self.ascii_glyphs[codepoint as usize]
        } else {
            let result = self
                .unicode_glyphs
                .get(&codepoint)
                .unwrap_or(&self.ascii_glyphs[0usize]);
            if result.sprite.name != "" {
                result
            } else {
                &self.ascii_glyphs['?' as usize]
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
    fn fontsize_tester() {
        BitmapFont::test_font_sizes(
            FONT_DEFAULT_TINY_NAME,
            FONT_DEFAULT_TINY_TTF,
            FONT_DEFAULT_TINY_RASTER_OFFSET,
            4,
            32,
            "tests/fontsize_tester.png",
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
                bitmap.draw_text_aligned_in_point(
                    &font,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    1,
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: AlignmentHorizontal::Center,
                        y: AlignmentVertical::Top,
                        origin_is_baseline: false,
                        ignore_whitespace: false,
                    }),
                );
                bitmap.draw_text_aligned_in_point(
                    &font,
                    text,
                    1,
                    bitmap_center,
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: *alignment_x,
                        y: *alignment_y,
                        origin_is_baseline: false,
                        ignore_whitespace: false,
                    }),
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
                bitmap.draw_text_aligned_in_point(
                    &font,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    1,
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: AlignmentHorizontal::Center,
                        y: AlignmentVertical::Top,
                        origin_is_baseline: false,
                        ignore_whitespace: false,
                    }),
                );
                bitmap.draw_text_aligned_in_point(
                    &font,
                    text,
                    1,
                    bitmap_center,
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: *alignment_x,
                        y: *alignment_y,
                        origin_is_baseline: false,
                        ignore_whitespace: true,
                    }),
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
                bitmap.draw_text_aligned_in_point(
                    &font,
                    &format!("{:?} - {:?}", alignment_x, alignment_y),
                    1,
                    Vec2i::new(bitmap_width / 2, 0),
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: AlignmentHorizontal::Center,
                        y: AlignmentVertical::Top,
                        origin_is_baseline: false,
                        ignore_whitespace: false,
                    }),
                );
                bitmap.draw_text_aligned_in_point(
                    &font,
                    text,
                    1,
                    bitmap_center,
                    Vec2i::zero(),
                    Some(TextAlignment {
                        x: *alignment_x,
                        y: *alignment_y,
                        origin_is_baseline: false,
                        ignore_whitespace: true,
                    }),
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
