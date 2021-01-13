use crate::camera::Camera;

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

    Bitmap::copy_region(
        source_bitmap,
        source_rect,
        &mut result_bitmap,
        result_rect,
        None,
    );

    result_bitmap
}

#[inline]
#[cfg(not(target_arch = "wasm32"))]
pub fn debug_save_sprite_as_png(assets: &GameAssets, sprite_name: &str, filepath: &str) {
    let sprite_bitmap = debug_get_bitmap_for_sprite(assets, sprite_name);
    Bitmap::write_to_png_file(&sprite_bitmap, filepath);
}

#[inline]
pub fn debug_draw_grid(
    draw: &mut Drawstate,
    camera: &Camera,
    world_grid_size: f32,
    screen_width: f32,
    screen_height: f32,
    line_thickness: i32,
    color: Color,
    depth: f32,
) {
    assert!(line_thickness > 0);

    let frustum = camera.bounds_pixelsnapped();
    let top = f32::floor(frustum.top() / world_grid_size) * world_grid_size;
    let bottom = f32::ceil(frustum.bottom() / world_grid_size) * world_grid_size;
    let left = f32::floor(frustum.left() / world_grid_size) * world_grid_size;
    let right = f32::ceil(frustum.right() / world_grid_size) * world_grid_size;

    let mut x = left;
    while x <= right {
        let start = Vec2::new(x, top);
        let end = Vec2::new(x, bottom);

        let start = camera.worldpoint_to_canvaspoint(start);
        let end = camera.worldpoint_to_canvaspoint(end);

        let start = canvas_point_to_screen_point(
            start.x,
            start.y,
            screen_width as u32,
            screen_height as u32,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );
        let end = canvas_point_to_screen_point(
            end.x,
            end.y,
            screen_width as u32,
            screen_height as u32,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            start.x + line_thickness as f32,
            end.y,
        );
        draw.draw_rect(
            rect,
            true,
            Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        x += world_grid_size;
    }
    let mut y = top;
    while y <= bottom {
        let start = Vec2::new(left, y);
        let end = Vec2::new(right, y);

        let start = camera.worldpoint_to_canvaspoint(start);
        let end = camera.worldpoint_to_canvaspoint(end);

        let start = canvas_point_to_screen_point(
            start.x,
            start.y,
            screen_width as u32,
            screen_height as u32,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );
        let end = canvas_point_to_screen_point(
            end.x,
            end.y,
            screen_width as u32,
            screen_height as u32,
            camera.dim_canvas.x as u32,
            camera.dim_canvas.y as u32,
        );

        let rect = Rect::from_bounds_left_top_right_bottom(
            start.x,
            start.y,
            end.x,
            start.y + line_thickness as f32,
        );
        draw.draw_rect(
            rect,
            true,
            Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
        );

        y += world_grid_size;
    }
}

pub fn debug_draw_crosshair(
    draw: &mut Drawstate,
    camera: &Camera,
    pos_world: Vec2,
    screen_width: f32,
    screen_height: f32,
    line_thickness: f32,
    color: Color,

    depth: f32,
) {
    let frustum = camera.bounds_pixelsnapped();

    let start = Vec2::new(frustum.left(), pos_world.y);
    let end = Vec2::new(frustum.right(), pos_world.y);

    let start = camera.worldpoint_to_canvaspoint(start);
    let end = camera.worldpoint_to_canvaspoint(end);

    let start = canvas_point_to_screen_point(
        start.x,
        start.y,
        screen_width as u32,
        screen_height as u32,
        camera.dim_canvas.x as u32,
        camera.dim_canvas.y as u32,
    );
    let end = canvas_point_to_screen_point(
        end.x,
        end.y,
        screen_width as u32,
        screen_height as u32,
        camera.dim_canvas.x as u32,
        camera.dim_canvas.y as u32,
    );

    draw.draw_line_with_thickness(
        start,
        end,
        line_thickness,
        true,
        Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
    );

    let start = Vec2::new(pos_world.x, frustum.top());
    let end = Vec2::new(pos_world.x, frustum.bottom());

    let start = camera.worldpoint_to_canvaspoint(start);
    let end = camera.worldpoint_to_canvaspoint(end);

    let start = canvas_point_to_screen_point(
        start.x,
        start.y,
        screen_width as u32,
        screen_height as u32,
        camera.dim_canvas.x as u32,
        camera.dim_canvas.y as u32,
    );
    let end = canvas_point_to_screen_point(
        end.x,
        end.y,
        screen_width as u32,
        screen_height as u32,
        camera.dim_canvas.x as u32,
        camera.dim_canvas.y as u32,
    );

    draw.draw_line_with_thickness(
        start,
        end,
        line_thickness,
        true,
        Drawparams::new(depth, color, ADDITIVITY_NONE, Drawspace::Screen),
    );
}
