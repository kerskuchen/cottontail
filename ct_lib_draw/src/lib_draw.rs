pub mod draw;
pub mod sprite;

pub use draw::*;
pub use sprite::*;

use ct_lib_core as core;
use ct_lib_image as image;
use ct_lib_math as math;

use math::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Coordinates

/// A point in world-coordinate-space. One 1x1 unit-square in world-space equals to a pixel on the
/// canvas on a default zoom level
pub type Worldpoint = Point;

/// Same as Worldpoint only as vector
pub type Worldvec = Vec2;

/// A point in canvas-coordinate-space. Given in the range
/// [0, CANVAS_WIDTH - 1]x[0, CANVAS_HEIGHT - 1]
/// where (0,0) is the top-left corner
pub type Canvaspoint = Point;

/// Same as Canvaspoint only as vector
pub type Canvasvec = Vec2;

pub trait PixelSnapped {
    fn pixel_snapped(self) -> Self;
}

impl PixelSnapped for Worldpoint {
    /// For a given Worldpoint returns the nearest Worldpoint that is aligned to the
    /// canvas's pixel grid when drawn.
    ///
    /// For example pixel-snapping the cameras position before drawing prevents pixel-jittering
    /// artifacts on visible objects if the camera is moving at sub-pixel distances.
    #[inline]
    fn pixel_snapped(self) -> Worldpoint {
        Worldpoint {
            x: f32::floor(self.x),
            y: f32::floor(self.y),
        }
    }
}

impl PixelSnapped for Transform {
    #[inline]
    fn pixel_snapped(self) -> Transform {
        Transform {
            pos: self.pos.pixel_snapped(),
            scale: self.scale,
            dir_angle: self.dir_angle,
        }
    }
}

impl PixelSnapped for Rect {
    #[inline]
    fn pixel_snapped(self) -> Rect {
        Rect::from_pos_dim(self.pos.pixel_snapped(), self.dim.round())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Canvas and screen blitting and transformations

/// Returns the `blit_rectangle` of for given canvas and screen dimensions.
/// The `blit-rectange` is the area of the screen where the content of the canvas is drawn onto.
/// It is as big as the canvas proportionally stretched and centered to fill the whole
/// screen.
///
/// It may or may not be smaller than the full screen size depending on the aspect
/// ratio of both the screen and the canvas. The `blit_rectange` is guaranteed to either have
/// the same width a as the screen (with letterboxing if needed) or the same height as the
/// screen (with columnboxing if needed) or completely fill the screen.
///
/// # Examples
/// ```
/// // +------+  +--------------+  +---------------+
/// // |canvas|  |   screen     |  |               | <- screen
/// // | 8x4  |  |    16x12     |  +---------------+
/// // +------+  |              |  |   blit-rect   |
/// //           |              |  |     16x10     |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  |               |
/// //           |              |  +---------------+
/// //           |              |  |               |
/// //           +--------------+  +---------------+
/// //
/// // +------+  +----------------+  +-+-------------+-+
/// // |canvas|  |     screen     |  | |             | |
/// // | 8x4  |  |      18x8      |  | |             | |
/// // +------+  |                |  | |  blit-rect  | |
/// //           |                |  | |    16x8     | |
/// //           |                |  | |             | |
/// //           |                |  | |             | |
/// //           +----------------+  +-+-------------+-+
/// //                                                ^---- screen
/// //
/// // +------+  +----------------+  +-----------------+
/// // |canvas|  |     screen     |  |                 |
/// // | 8x4  |  |      16x8      |  |                 |
/// // +------+  |                |  |    blit-rect    |
/// //           |                |  |      16x8       |
/// //           |                |  |                 |
/// //           |                |  |                 |
/// //           +----------------+  +-----------------+
/// //                                                ^---- blit-rect == screen
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct BlitRect {
    pub offset_x: i32,
    pub offset_y: i32,
    pub width: i32,
    pub height: i32,
}

impl BlitRect {
    #[inline]
    pub fn new_from_dimensions(width: u32, height: u32) -> BlitRect {
        BlitRect {
            offset_x: 0,
            offset_y: 0,
            width: width as i32,
            height: height as i32,
        }
    }

    pub fn to_recti(self) -> Recti {
        Recti::from_xy_width_height(self.offset_x, self.offset_y, self.width, self.height)
    }

    /// Creates a canvas of fixed size that is stretched to the screen with aspect ratio correction
    #[inline]
    pub fn new_for_fixed_canvas_size(
        screen_width: u32,
        screen_height: u32,
        canvas_width: u32,
        canvas_height: u32,
    ) -> BlitRect {
        let aspect_ratio = canvas_height as f32 / canvas_width as f32;
        let mut blit_width = screen_width as f32;
        let mut blit_height = blit_width * aspect_ratio;

        if blit_height > screen_height as f32 {
            blit_height = screen_height as f32;
            blit_width = blit_height / aspect_ratio;
        }

        BlitRect {
            offset_x: f32::round((screen_width as f32 / 2.0) - (blit_width / 2.0)) as i32,
            offset_y: f32::round((screen_height as f32 / 2.0) - (blit_height / 2.0)) as i32,
            width: f32::round(blit_width) as i32,
            height: f32::round(blit_height) as i32,
        }
    }
}

/// Converts a screen point to coordinates respecting the canvas
/// dimensions and its offsets
///
/// screen_pos_x in [0, screen_width - 1] (left to right)
/// screen_pos_y in [0, screen_height - 1] (top to bottom)
/// result in [0, canvas_width - 1]x[0, canvas_height - 1] (relative to clamped canvas area,
///                                                         top-left to bottom-right)
///
/// WARNING: This does not work optimally if the pixel-scale-factor
/// (which is screen_width / canvas_width) is not an integer value
///
#[inline]
pub fn screen_point_to_canvas_point(
    screen_width: u32,
    screen_height: u32,
    canvas_width: u32,
    canvas_height: u32,
    screen_pos_x: i32,
    screen_pos_y: i32,
) -> Pointi {
    let blit_rect = BlitRect::new_for_fixed_canvas_size(
        screen_width,
        screen_height,
        canvas_width,
        canvas_height,
    );

    let pos_blitrect_x = clampi(screen_pos_x - blit_rect.offset_x, 0, blit_rect.width - 1);
    let pos_blitrect_y = clampi(screen_pos_y - blit_rect.offset_y, 0, blit_rect.height - 1);

    let pos_canvas_x = canvas_width as f32 * (pos_blitrect_x as f32 / blit_rect.width as f32);
    let pos_canvas_y = canvas_height as f32 * (pos_blitrect_y as f32 / blit_rect.height as f32);

    Pointi::new(floori(pos_canvas_x), floori(pos_canvas_y))
}

pub fn letterbox_rects_create(
    center_width: i32,
    center_height: i32,
    canvas_width: i32,
    canvas_height: i32,
) -> (Recti, [Recti; 4]) {
    let pos_x = floori(block_centered_in_point(
        center_width as f32,
        canvas_width as f32 / 2.0,
    ));
    let pos_y = floori(block_centered_in_point(
        center_height as f32,
        canvas_height as f32 / 2.0,
    ));
    let center_rect = Recti::from_xy_width_height(pos_x, pos_y, center_width, center_height);

    let letterbox_rects = [
        // Top
        Recti::from_bounds_left_top_right_bottom(0, 0, canvas_width, center_rect.top()),
        // Left
        Recti::from_bounds_left_top_right_bottom(
            0,
            center_rect.top(),
            center_rect.left(),
            center_rect.bottom(),
        ),
        // Right
        Recti::from_bounds_left_top_right_bottom(
            center_rect.right(),
            center_rect.top(),
            canvas_width,
            center_rect.bottom(),
        ),
        // Bottom
        Recti::from_bounds_left_top_right_bottom(
            0,
            center_rect.bottom(),
            canvas_width,
            canvas_height,
        ),
    ];
    (center_rect, letterbox_rects)
}
