use super::bitmap::*;
use super::bitmap_atlas::BitmapAtlas;
use super::color::*;
use super::math::*;
use super::IndexMap;

pub const FONT_DEFAULT_TTF_TINY: &[u8] = include_bytes!("../resources/fonts/ProggyTiny.ttf");
pub const FONT_DEFAULT_TTF_SMALL: &[u8] = include_bytes!("../resources/fonts/ProggySmall.ttf");
pub const FONT_DEFAULT_TTF_REGULAR: &[u8] = include_bytes!("../resources/fonts/ProggyClean.ttf");
pub const FONT_DEFAULT_TTF_SQUARE: &[u8] = include_bytes!("../resources/fonts/ProggySquare.ttf");

const FIRST_VISIBLE_ASCII_CODE_POINT: u8 = 32;
const LAST_ASCII_CODE_POINT: u8 = 126;

<<<<<<< HEAD:cottontail/ct_lib/src/bitmap_font.rs
type Codepoint = i32;

=======
<<<<<<< HEAD:cottontail/ct_lib/src/bitmapfont.rs
=======
type Codepoint = i32;

>>>>>>> 24395fe131ae4b13b1d7598ebd720fe801151681:cottontail/ct_lib/src/bitmap_font.rs
>>>>>>> 76db92fe87536331eb69e156a1afded3b9eca72a:cottontail/ct_lib/src/bitmap_font.rs
#[derive(Clone)]
pub struct BitmapFont {
    pub font_height: i32,
    pub vertical_advance: i32,
    pub baseline: i32,
    pub glyphs: Vec<BitmapFontGlyph>,
}

impl BitmapFont {
    pub fn new(
        font_ttf_bytes: &[u8],
        font_height: i32,
        border_thickness: u32,
        atlas_padding: u32,
        color_glyph: PixelRGBA,
        color_border: PixelRGBA,
    ) -> BitmapFont {
        let font = rusttype::Font::from_bytes(font_ttf_bytes)
            .expect(&format!("Could not decode font from bytes"));

        // Font metrics
        let scale = rusttype::Scale::uniform(font_height as f32);
        let v_metrics = font.v_metrics(scale);
        let vertical_advance =
            (v_metrics.ascent - v_metrics.descent + v_metrics.line_gap).ceil() as i32;
        let baseline = v_metrics.ascent.ceil() as i32;
        let descent = v_metrics.descent.ceil() as i32;

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
                    code_point,
                    font_height,
                    descent,
                    border_thickness,
                    atlas_padding,
                    color_glyph,
                    color_border,
                )
            })
            .collect();

        BitmapFont {
            font_height: font_height + 2 * border_thickness as i32,
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

    pub fn create_text_bitmap(&self, text: &str, background_color: PixelRGBA) -> Bitmap {
        let dim = self.get_text_dimensions(text);
        let mut result = Bitmap::new_filled(dim.x as u32, dim.y as u32, background_color);
        self.draw_text(&mut result, text, Vec2i::zero(), Vec2i::zero());
        result
    }

    pub fn draw_text_aligned(
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
                font_ttf_bytes,
                font_height_max,
                0,
                0,
                PixelRGBA::black(),
                PixelRGBA::transparent(),
            );
            let max_text_width = max_font.get_text_dimensions(&max_text).x;
            let max_text_height = max_font.font_height;

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
                font_ttf_bytes,
                font_height,
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
        codepoint: char,
        font_height: i32,
        descent: i32,
        border_thickness: u32,
        atlas_padding: u32,
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
        let metrics = glyph.unpositioned().h_metrics();
        let horizontal_advance = metrics.advance_width.round() as i32 + border_thickness as i32;
        // NOTE: The offset determines how many pixels the glyph-sprite needs to be offset
        //       from its origin (top-left corner) when drawn to the screen
        let offset_x = metrics.left_side_bearing.round() as i32 - atlas_padding as i32;
        let mut offset_y = -(atlas_padding as i32);

        let maybe_image = create_glyph_image(
            &glyph,
            border_thickness,
            atlas_padding,
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
