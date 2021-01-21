use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug drawing

pub fn debug_get_bitmap_for_sprite(assets: &GameAssets, sprite_name: &str) -> Bitmap {
    let sprite = assets.get_sprite(sprite_name);
    let textures = assets.get_atlas_textures();

    let source_bitmap = &textures[sprite.atlas_texture_index as usize].borrow();

    let dim = Vec2i::from_vec2_rounded(sprite.trimmed_rect.dim);
    let texture_coordinates = AAQuad::from_rect(
        sprite
            .trimmed_uvs
            .to_rect()
            .scaled_from_origin(Vec2::filled(source_bitmap.width as f32)),
    );

    let source_rect = Recti::from_rect_rounded(texture_coordinates.to_rect());

    let mut result_bitmap = Bitmap::new(dim.x as u32, dim.y as u32);
    let result_rect = result_bitmap.rect();

    Bitmap::copy_region(source_bitmap, source_rect, &mut result_bitmap, result_rect);

    result_bitmap
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub fn debug_save_sprite_as_png(assets: &GameAssets, sprite_name: &str, filepath: &str) {
    let sprite_bitmap = debug_get_bitmap_for_sprite(assets, sprite_name);
    Bitmap::write_to_png_file(&sprite_bitmap, filepath);
}

#[inline]
pub fn draw_debug_grid(world_grid_size: f32, line_thickness: i32, color: Color, depth: f32) {
    assert!(line_thickness > 0);

    let frustum = get_camera().bounds_pixelsnapped();
    let top = f32::floor(frustum.top() / world_grid_size) * world_grid_size;
    let bottom = f32::ceil(frustum.bottom() / world_grid_size) * world_grid_size;
    let left = f32::floor(frustum.left() / world_grid_size) * world_grid_size;
    let right = f32::ceil(frustum.right() / world_grid_size) * world_grid_size;

    let mut x = left;
    while x <= right {
        let start = coordinates_world_to_screen(Vec2::new(x, top)).pixel_snapped();
        let end = coordinates_world_to_screen(Vec2::new(x, bottom)).pixel_snapped();

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            start.x + line_thickness as f32,
            end.y,
        );
        draw_rect(
            rect,
            true,
            Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        x += world_grid_size;
    }
    let mut y = top;
    while y <= bottom {
        let start = coordinates_world_to_screen(Vec2::new(left, y)).pixel_snapped();
        let end = coordinates_world_to_screen(Vec2::new(right, y)).pixel_snapped();

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            end.x,
            start.y + line_thickness as f32,
        );
        draw_rect(
            rect,
            true,
            Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        y += world_grid_size;
    }
}

#[inline]
pub fn draw_debug_crosshair(pos_world: Vec2, line_thickness: f32, color: Color, depth: f32) {
    let frustum = get_camera().bounds_pixelsnapped();

    let start = coordinates_world_to_screen(Vec2::new(frustum.left(), pos_world.y));
    let end = coordinates_world_to_screen(Vec2::new(frustum.right(), pos_world.y));
    draw_line_with_thickness(
        start,
        end,
        line_thickness,
        true,
        Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
    );

    let start = coordinates_world_to_screen(Vec2::new(pos_world.x, frustum.top()));
    let end = coordinates_world_to_screen(Vec2::new(pos_world.x, frustum.bottom()));
    draw_line_with_thickness(
        start,
        end,
        line_thickness,
        true,
        Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DEBUG DRAW LOGGING

pub struct DebugDrawLogState {
    font_name: String,
    font_scale: f32,
    draw_origin: Vec2,
    draw_offset: Vec2,
    draw_depth: Depth,
}

impl DebugDrawLogState {
    #[inline]
    pub fn new() -> DebugDrawLogState {
        let font_name = FONT_DEFAULT_TINY_NAME.to_owned() + "_bordered";
        DebugDrawLogState {
            font_name,
            font_scale: 2.0,
            draw_origin: Vec2::new(5.0, 5.0),
            draw_offset: Vec2::zero(),
            draw_depth: DEPTH_MAX,
        }
    }

    #[inline]
    pub fn begin_frame(&mut self) {
        self.draw_offset = Vec2::zero();
    }

    #[inline]
    pub fn log(&mut self, text: impl Into<String>) {
        self.log_color(text, Color::white())
    }

    #[inline]
    pub fn log_color(&mut self, text: impl Into<String>, color: Color) {
        let origin = self.draw_origin.pixel_snapped();
        let font = get_assets().get_font(&self.font_name);
        self.draw_offset = draw_text(
            &text.into(),
            font,
            self.font_scale,
            origin,
            self.draw_offset,
            None,
            None,
            Drawparams::new(self.draw_depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        // Add final '\n'
        self.draw_offset.x = 0.0;
        self.draw_offset.y += self.font_scale * font.vertical_advance as f32;
    }

    #[inline]
    pub fn log_visualize_value_percent(
        &mut self,
        label: impl Into<String>,
        color: Color,
        value_percent: f32,
    ) {
        self.log_visualize_value(label, color, value_percent, 0.0, 1.0)
    }

    #[inline]
    pub fn log_visualize_value(
        &mut self,
        label: impl Into<String>,
        color: Color,
        value: f32,
        value_min: f32,
        value_max: f32,
    ) {
        let origin = self.draw_origin.pixel_snapped();
        let font = get_assets().get_font(&self.font_name);
        self.draw_offset = draw_text(
            &label.into(),
            font,
            self.font_scale,
            origin,
            self.draw_offset,
            None,
            None,
            Drawparams::new(self.draw_depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        assert!(value_max > value_min);
        let percentage = (value - value_min) / (value_max - value_min);

        let offset = origin + self.draw_offset + Vec2::filled_x(font.horizontal_advance_max as f32);
        let rect_width = 10.0 * self.font_scale * font.vertical_advance as f32;
        let rect_height = self.font_scale * font.vertical_advance as f32;

        let rect_outline = Rect::from_width_height(rect_width, rect_height).translated_by(offset);

        let rect_fill = if percentage < 0.0 {
            Rect::from_width_height(rect_width, rect_height)
                .scaled_from_origin(Vec2::new(percentage, 1.0))
                .mirrored_horizontally_on_axis(0.0)
                .translated_by(offset)
        } else {
            Rect::from_width_height(rect_width, rect_height)
                .scaled_from_origin(Vec2::new(percentage, 1.0))
                .translated_by(offset)
        };

        draw_rect(
            rect_fill,
            true,
            Drawparams::new(self.draw_depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );
        draw_rect(
            rect_outline,
            false,
            Drawparams::new(
                self.draw_depth,
                color.with_color_multiplied_by(0.5),
                ADDITIVITY_NONE,
                Drawspace::Screen,
            ),
        );

        self.draw_offset.x += 11.0 * self.font_scale * font.vertical_advance as f32;

        self.draw_offset = draw_text(
            &format!("{}", value),
            font,
            self.font_scale,
            origin,
            self.draw_offset,
            None,
            None,
            Drawparams::new(self.draw_depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        // Add final '\n'
        self.draw_offset.x = 0.0;
        self.draw_offset.y += self.font_scale * font.vertical_advance as f32;
    }
}
