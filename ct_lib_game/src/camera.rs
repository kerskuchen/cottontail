use crate::choreographer::TimerSimple;

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Camera and coordinates

/// NOTE: Camera pos equals is the top-left of the screen or the center depending on the
/// `is_centered` flag. It has the following bounds in world coordinates:
/// non-centered: [pos.x, pos.x + dim_frustum.w] x [pos.y, pos.y + dim_frustum.h]
/// centered:     [pos.x - 0.5*dim_frustum.w, pos.x + 0.5*dim_frustum.w] x
///               [pos.y - 0.5*dim_frustum.h, pos.y + 0.5*dim_frustum.h]
///
/// with:
/// dim_frustum = dim_canvas / zoom
/// zoom > 1.0 -> zooming in
/// zoom < 1.0 -> zooming out
#[derive(Clone, Default)]
pub struct Camera {
    pub pos: Vec2,
    pub pos_pixelsnapped: Vec2,
    pub dim_frustum: Vec2,
    pub dim_canvas: Vec2,
    pub zoom_level: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub is_centered: bool,
}

// Coordinates
//
impl Camera {
    /// Converts a CanvasPoint to a Worldpoint
    #[inline]
    pub fn canvaspoint_to_worldpoint(&self, canvaspoint: Canvaspoint) -> Worldpoint {
        if self.is_centered {
            (canvaspoint - 0.5 * self.dim_canvas) / self.zoom_level + self.pos_pixelsnapped
        } else {
            (canvaspoint / self.zoom_level) + self.pos_pixelsnapped
        }
    }

    /// Converts a Worldpoint to a CanvasPoint
    #[inline]
    pub fn worldpoint_to_canvaspoint(&self, worldpoint: Worldpoint) -> Canvaspoint {
        if self.is_centered {
            (worldpoint - self.pos_pixelsnapped) * self.zoom_level + 0.5 * self.dim_canvas
        } else {
            (worldpoint - self.pos_pixelsnapped) * self.zoom_level
        }
    }

    /// Converts a Canvasvec to a Worldvec
    #[inline]
    pub fn canvas_vec_to_world_vec(&self, canvasvec: Canvasvec) -> Worldvec {
        canvasvec / self.zoom_level
    }

    /// Converts a Worldvec to a Canvasvec
    #[inline]
    pub fn world_vec_to_canvas_vec(&self, worldvec: Worldvec) -> Canvasvec {
        worldvec * self.zoom_level
    }
}

// Creation and properties
//
impl Camera {
    pub fn new(
        pos: Worldpoint,
        zoom_level: f32,
        canvas_width: u32,
        canvas_height: u32,
        z_near: f32,
        z_far: f32,
        is_centered: bool,
    ) -> Camera {
        let dim_canvas = Vec2::new(canvas_width as f32, canvas_height as f32);

        Camera {
            pos,
            pos_pixelsnapped: pos.pixel_snapped(),
            zoom_level,
            dim_canvas,
            dim_frustum: dim_canvas / zoom_level,
            z_near,
            z_far,
            is_centered,
        }
    }

    #[inline]
    pub fn center(&self) -> Worldpoint {
        if self.is_centered {
            self.pos
        } else {
            self.pos + 0.5 * self.dim_frustum
        }
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&mut self) -> Mat4 {
        let view = Mat4::scale(self.zoom_level, self.zoom_level, 1.0)
            * Mat4::translation(-self.pos_pixelsnapped.x, -self.pos_pixelsnapped.y, 0.0);

        let projection = if self.is_centered {
            Mat4::ortho_origin_center_flipped_y(
                self.dim_canvas.x,
                self.dim_canvas.y,
                self.z_near,
                self.z_far,
            )
        } else {
            Mat4::ortho_origin_left_top(
                self.dim_canvas.x,
                self.dim_canvas.y,
                self.z_near,
                self.z_far,
            )
        };
        projection * view
    }

    #[inline]
    pub fn bounds_pixelsnapped(&self) -> Rect {
        if self.is_centered {
            Rect::from_bounds_left_top_right_bottom(
                self.pos_pixelsnapped.x - 0.5 * self.dim_frustum.x,
                self.pos_pixelsnapped.y + 0.5 * self.dim_frustum.y,
                self.pos_pixelsnapped.x + 0.5 * self.dim_frustum.x,
                self.pos_pixelsnapped.y - 0.5 * self.dim_frustum.y,
            )
        } else {
            Rect::from_bounds_left_top_right_bottom(
                self.pos_pixelsnapped.x,
                self.pos_pixelsnapped.y,
                self.pos_pixelsnapped.x + self.dim_frustum.x,
                self.pos_pixelsnapped.y + self.dim_frustum.y,
            )
        }
    }

    #[inline]
    pub fn bounds(&self) -> Rect {
        if self.is_centered {
            Rect::from_bounds_left_top_right_bottom(
                self.pos.x - 0.5 * self.dim_frustum.x,
                self.pos.y + 0.5 * self.dim_frustum.y,
                self.pos.x + 0.5 * self.dim_frustum.x,
                self.pos.y - 0.5 * self.dim_frustum.y,
            )
        } else {
            Rect::from_bounds_left_top_right_bottom(
                self.pos.x,
                self.pos.y,
                self.pos.x + self.dim_frustum.x,
                self.pos.y + self.dim_frustum.y,
            )
        }
    }
}

// Manipulation
//
impl Camera {
    /// Zooms the camera to or away from a given world point.
    ///
    /// new_zoom_level > old_zoom_level -> magnify
    /// new_zoom_level < old_zoom_level -> minify
    #[inline]
    pub fn zoom_to_world_point(&mut self, worldpoint: Worldpoint, new_zoom_level: f32) {
        let old_zoom_level = self.zoom_level;
        self.zoom_level = new_zoom_level;
        self.dim_frustum = self.dim_canvas / new_zoom_level;
        self.pos = (self.pos - worldpoint) * (old_zoom_level / new_zoom_level) + worldpoint;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }

    /// Pans the camera using cursor movement distance on the canvas
    #[inline]
    pub fn pan(&mut self, canvas_move_distance: Canvasvec) {
        self.pos -= canvas_move_distance / self.zoom_level;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }

    #[inline]
    pub fn set_pos(&mut self, worldpoint: Worldpoint) {
        self.pos = worldpoint;
        self.pos_pixelsnapped = self.pos.pixel_snapped();
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Game Camera
//

#[derive(Clone)]
pub struct GameCamera {
    pub cam: Camera,

    pub pos: Vec2,
    pub pos_target: Vec2,
    pub use_pixel_perfect_smoothing: bool,

    pub drag_margin_left: f32,
    pub drag_margin_top: f32,
    pub drag_margin_right: f32,
    pub drag_margin_bottom: f32,

    pub screenshake_offset: Vec2,
    pub screenshakers: Vec<ModulatorScreenShake>,
}

impl GameCamera {
    pub fn new(pos: Vec2, canvas_width: u32, canvas_height: u32, is_centered: bool) -> GameCamera {
        let cam = Camera::new(
            pos,
            1.0,
            canvas_width,
            canvas_height,
            DEFAULT_WORLD_ZNEAR,
            DEFAULT_WORLD_ZFAR,
            is_centered,
        );

        GameCamera {
            cam,
            pos,
            screenshake_offset: Vec2::zero(),
            screenshakers: Vec::new(),
            pos_target: pos,
            use_pixel_perfect_smoothing: false,

            drag_margin_left: 0.2,
            drag_margin_top: 0.1,
            drag_margin_right: 0.2,
            drag_margin_bottom: 0.1,
        }
    }

    pub fn add_shake(&mut self, shake: ModulatorScreenShake) {
        self.screenshakers.push(shake);
    }

    pub fn update(&mut self, deltatime: f32) {
        self.screenshake_offset = Vec2::zero();

        for shaker in self.screenshakers.iter_mut() {
            self.screenshake_offset += shaker.update_and_get_value(deltatime);
        }

        self.screenshakers
            .retain(|shaker| shaker.timer.is_running());

        self.pos = if self.use_pixel_perfect_smoothing {
            let mut points_till_target = Vec::new();
            iterate_line_bresenham(
                self.pos.pixel_snapped().to_i32(),
                self.pos_target.pixel_snapped().to_i32(),
                false,
                &mut |x, y| points_till_target.push(Vec2::new(x as f32, y as f32)),
            );

            let point_count = points_till_target.len();
            let skip_count = if point_count <= 1 {
                0
            } else if point_count <= 10 {
                1
            } else if point_count <= 20 {
                2
            } else if point_count <= 40 {
                3
            } else if point_count <= 80 {
                4
            } else if point_count <= 160 {
                5
            } else if point_count <= 320 {
                6
            } else {
                7
            };

            *points_till_target.iter().skip(skip_count).next().unwrap()
        } else {
            Vec2::lerp(self.pos, self.pos_target, 0.05)
        };
    }

    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
        self.pos_target = pos;
    }

    pub fn set_target_pos(&mut self, target_pos: Vec2, use_pixel_perfect_smoothing: bool) {
        self.use_pixel_perfect_smoothing = use_pixel_perfect_smoothing;
        self.pos_target = target_pos;
    }

    /// Zooms the camera to or away from a given world point.
    ///
    /// new_zoom_level > old_zoom_level -> magnify
    /// new_zoom_level < old_zoom_level -> minify
    #[inline]
    pub fn zoom_to_world_point(&mut self, worldpoint: Worldpoint, new_zoom_level: f32) {
        let old_zoom_level = self.cam.zoom_level;
        self.cam.zoom_level = new_zoom_level;
        self.cam.dim_frustum = self.cam.dim_canvas / new_zoom_level;
        self.pos = (self.pos - worldpoint) * (old_zoom_level / new_zoom_level) + worldpoint;
        self.pos_target = self.pos;
    }

    /// Pans the camera using cursor movement distance on the canvas
    #[inline]
    pub fn pan(&mut self, canvas_move_distance: Canvasvec) {
        self.pos -= canvas_move_distance / self.cam.zoom_level;
        self.pos_target = self.pos;
    }

    #[inline]
    pub fn center(&mut self) -> Worldpoint {
        self.sync_pos_internal();
        self.cam.center()
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&mut self) -> Mat4 {
        self.sync_pos_internal();
        self.cam.proj_view_matrix()
    }

    #[inline]
    pub fn bounds_pixelsnapped(&mut self) -> Rect {
        self.sync_pos_internal();
        self.cam.bounds_pixelsnapped()
    }

    #[inline]
    pub fn bounds(&mut self) -> Rect {
        self.sync_pos_internal();
        self.cam.bounds()
    }

    #[inline]
    fn sync_pos_internal(&mut self) {
        self.cam.set_pos(self.pos + self.screenshake_offset);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Camera shake
//
// Based on https://jonny.morrill.me/en/blog/gamedev-how-to-implement-a-camera-shake-effect/
//

#[derive(Clone)]
pub struct ModulatorScreenShake {
    pub amplitude: f32,
    pub frequency: f32,
    pub samples: Vec<Vec2>,
    pub timer: TimerSimple,
}

impl ModulatorScreenShake {
    pub fn new(
        random: &mut Random,
        amplitude: f32,
        duration: f32,
        frequency: f32,
    ) -> ModulatorScreenShake {
        let samplecount = ceili(duration * frequency) as usize;
        let samples: Vec<Vec2> = (0..samplecount)
            .map(|_sample_index| amplitude * random.vec2_in_unit_rect())
            .collect();

        ModulatorScreenShake {
            amplitude,
            frequency,
            samples,
            timer: TimerSimple::new_started(duration),
        }
    }

    pub fn update_and_get_value(&mut self, deltatime: f32) -> Vec2 {
        self.timer.update(deltatime);
        let percentage = self.timer.completion_ratio();

        let last_sample_index = self.samples.len() - 1;
        let sample_index = floori(last_sample_index as f32 * percentage) as usize;
        let sample_index_next = std::cmp::min(last_sample_index, sample_index + 1);

        let sample = self.samples[sample_index];
        let sample_next = self.samples[sample_index_next];

        let decay = 1.0 - percentage;

        decay * Vec2::lerp(sample, sample_next, percentage)
    }
}
