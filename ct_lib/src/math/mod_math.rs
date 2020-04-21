pub use ezing as easing;

mod rect;
mod recti;
pub use rect::*;
pub use recti::*;

mod vec2;
mod vec2i;
mod vec3;
mod vec4;
pub use vec2::*;
pub use vec2i::*;
pub use vec3::*;
pub use vec4::*;

mod mat4;
pub use mat4::*;

mod intersection;
pub use intersection::*;

pub type Point = Vec2;
pub type Pointi = Vec2i;

use num_traits::Num;

//--------------------------------------------------------------------------------------------------
// Misc

// NOTE: For f32 We can use -16383.999 to 16383.999 and still have a precision of at least 0.001!

pub const PI: f32 = std::f32::consts::PI;
pub const PI_2: f32 = std::f32::consts::PI / 2.0;
pub const PI64: f64 = std::f64::consts::PI;
pub const PI64_2: f64 = std::f64::consts::PI / 2.0;

#[inline]
pub fn rad_to_deg(x: f32) -> f32 {
    57.2957795 * x
}
#[inline]
pub fn deg_to_rad(x: f32) -> f32 {
    0.01745329251 * x
}

#[inline]
pub fn have_same_sign(x: f32, y: f32) -> bool {
    x * y >= 0.0
}

#[inline]
pub fn lerp(start: f32, end: f32, percent: f32) -> f32 {
    start + percent * (end - start)
}

#[inline]
pub fn squared(x: f32) -> f32 {
    x * x
}
#[inline]
pub fn cubed(x: f32) -> f32 {
    x * x * x
}
#[inline]
pub fn quadrupled(x: f32) -> f32 {
    x * x * x * x
}

#[inline]
pub fn compare_floats(a: f32, b: f32) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

#[inline]
pub const fn make_even_upwards(x: i32) -> i32 {
    (x / 2) * 2
}

//--------------------------------------------------------------------------------------------------
// Epsilon

pub const EPSILON: f32 = 0.000_01;

#[inline]
pub fn is_effectively_zero(x: f32) -> bool {
    f32::abs(x) < EPSILON
}

#[inline]
pub fn is_effectively_positive(x: f32) -> bool {
    x > EPSILON
}

//--------------------------------------------------------------------------------------------------
// Min, Max, Clamping, Wrapping

#[inline]
pub fn in_interval(val: f32, min: f32, max: f32) -> bool {
    min <= val && val <= max
}

#[inline]
pub fn in_intervali(val: i32, min: i32, max: i32) -> bool {
    min <= val && val <= max
}

#[inline]
pub fn in_intervali_exlusive_max(val: i32, min: i32, max: i32) -> bool {
    min <= val && val < max
}

/// For upper_abs > 0 returns:
/// abs(x) <= upper_abs : x
/// abs(x) >  upper_abs : sign(x) * upper_abs
#[inline]
pub fn clampf_absolute_upper(x: f32, upper_abs: f32) -> f32 {
    debug_assert!(upper_abs >= 0.0);
    f32::signum(x) * f32::min(f32::abs(x), upper_abs)
}

/// For lower_abs > 0 returns:
/// abs(x) <= lower_abs : sign(x) * lower_abs
/// abs(x) >  lower_abs : x
#[inline]
pub fn clampf_absolute_lower(x: f32, lower_abs: f32) -> f32 {
    debug_assert!(lower_abs > 0.0);
    f32::signum(x) * f32::max(f32::abs(x), lower_abs)
}

/// Returns the given number clamped to a given maximum length
#[inline]
pub fn clampf_absolute(x: f32, abs_max: f32) -> f32 {
    debug_assert!(abs_max >= 0.0);
    let sign = f32::signum(x);
    let x_positive = f32::abs(x);
    let clamped = f32::min(x_positive, abs_max);
    sign * clamped
}

#[inline]
pub fn clampf(x: f32, min: f32, max: f32) -> f32 {
    f32::min(max, f32::max(min, x))
}

#[inline]
pub fn clampi(x: i32, min: i32, max: i32) -> i32 {
    i32::min(max, i32::max(min, x))
}

/// Adds `to_add` to the value of a given number `x`. Returns 0 if adding `to_add` to `x` changes
/// the sign of x
///
/// Example:
/// abs_add(5, 3) = 8
/// abs_add(-2, 4) = 0
/// abs_add(4, -2) = 2
/// abs_add(4, -5) = 0
#[inline]
pub fn add_or_zero_when_changing_sign(x: f32, to_add: f32) -> f32 {
    let sum = x + to_add;
    if have_same_sign(sum, x) {
        // Adding did not change the sign of x
        sum
    } else {
        0.0
    }
}

/// Wraps an angle to the range [-2*PI, 2*PI]. Note that this operation does not change the cos and
/// sin of the angle value
#[inline]
pub fn wrap_angle_2pi(angle: f32) -> f32 {
    angle % (2.0 * PI)
}

//--------------------------------------------------------------------------------------------------
// Rounding

#[inline]
pub fn roundi(x: f32) -> i32 {
    f32::round(x) as i32
}
#[inline]
pub fn floori(x: f32) -> i32 {
    f32::floor(x) as i32
}
#[inline]
pub fn ceili(x: f32) -> i32 {
    f32::ceil(x) as i32
}

#[inline]
pub fn round_to_nearest_multiple_of_target(value: f32, target: f32) -> f32 {
    f32::round(value / target) * target
}

#[inline]
pub fn round_down_to_nearest_multiple_of_target(value: f32, target: f32) -> f32 {
    f32::floor(value / target) * target
}

#[inline]
pub fn round_up_to_nearest_multiple_of_target(value: f32, target: f32) -> f32 {
    f32::ceil(value / target) * target
}

//--------------------------------------------------------------------------------------------------
// Centering / alignment of blocks, mirroring

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Alignment {
    /// Also Left or Top
    Begin,
    Center,
    /// Also Right or Bottom
    End,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlignmentHorizontal {
    Left,
    Center,
    Right,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlignmentVertical {
    Top,
    Center,
    Bottom,
}

impl std::convert::Into<Alignment> for AlignmentHorizontal {
    #[inline]
    fn into(self) -> Alignment {
        match self {
            AlignmentHorizontal::Left => Alignment::Begin,
            AlignmentHorizontal::Center => Alignment::Center,
            AlignmentHorizontal::Right => Alignment::End,
        }
    }
}

impl std::convert::Into<Alignment> for AlignmentVertical {
    #[inline]
    fn into(self) -> Alignment {
        match self {
            AlignmentVertical::Top => Alignment::Begin,
            AlignmentVertical::Center => Alignment::Center,
            AlignmentVertical::Bottom => Alignment::End,
        }
    }
}

/// Returns the left-most point of a block with given width that is aligned in a given point
#[inline(always)]
pub fn block_aligned_in_point<AlignmentType, NumberType>(
    block_width: NumberType,
    point: NumberType,
    alignment: AlignmentType,
) -> NumberType
where
    AlignmentType: Into<Alignment>,
    NumberType: num_traits::Num,
{
    match alignment.into() {
        Alignment::Begin => point,
        Alignment::Center => block_centered_in_point(block_width, point),
        Alignment::End => point - block_width,
    }
}

/// Returns the left-most point of a block with given width that is aligned in a another block
/// with given width
#[inline(always)]
pub fn block_aligned_in_block<AlignmentType, NumberType>(
    block_width: NumberType,
    other_width: NumberType,
    alignment: AlignmentType,
) -> NumberType
where
    AlignmentType: Into<Alignment>,
    NumberType: Num,
{
    match alignment.into() {
        Alignment::Begin => NumberType::zero(),
        Alignment::Center => block_centered_in_block(block_width, other_width),
        Alignment::End => other_width - block_width,
    }
}

/// Returns the left-most point of a block with given width that is centered in a given point
#[inline(always)]
pub fn block_centered_in_point<NumberType: Num>(
    block_width: NumberType,
    point: NumberType,
) -> NumberType {
    let two = NumberType::one() + NumberType::one();
    point - (block_width / two)
}

/// Returns the left-most point of a block with given width that is aligned in a another block
/// with given width
#[inline(always)]
pub fn block_centered_in_block<NumberType: Num>(
    block_width: NumberType,
    other_width: NumberType,
) -> NumberType {
    let two = NumberType::one() + NumberType::one();
    (other_width - block_width) / two
}

#[inline(always)]
pub fn point_mirrored_on_axis<NumberType: Num>(
    point: NumberType,
    axis_pos: NumberType,
) -> NumberType {
    let two = NumberType::one() + NumberType::one();
    two * axis_pos - point
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Triangle

/// Returns the barycentric coordinates (u,v,w) for a given point p in given triangle (a,b,c).
/// where p = ua + vb + wc
#[inline]
pub fn triangle_barycentric_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> (f32, f32, f32) {
    // Derivation:
    // given: p = ua + vb + wc
    // where: w = 1 - u - v
    // <=> p = ua + vb + (1 - u - v)c
    // <=> p = ua + vb + c - uc - vc
    // <=> p = u(a - c) + v(b - c) + c
    // <=> p - c = u(a - c) + v(b - c)
    //
    // The following solves the 2x2 linear system:
    // u(a - c) + v(b - c) = p - c
    // for (u,v) via Cramer's rule:
    // https://en.wikipedia.org/wiki/Cramer's_rule#Explicit_formulas_for_small_systems
    let ac = a - c;
    let bc = b - c;
    let pc = p - c;
    let determinant = ac.x * bc.y - bc.x * ac.y;
    let u = (pc.x * bc.y - bc.x * pc.y) / determinant;
    let v = (ac.x * pc.y - pc.x * ac.y) / determinant;
    let w = 1.0 - u - v;
    (u, v, w)
}

#[inline]
pub fn triangle_get_bounds(a: Vec2, b: Vec2, c: Vec2) -> Rect {
    Rect::from_bounds_left_top_right_bottom(
        f32::min(a.x, f32::min(b.x, c.x)),
        f32::min(a.y, f32::min(b.y, c.y)),
        f32::max(a.x, f32::max(b.x, c.x)),
        f32::max(a.y, f32::max(b.y, c.y)),
    )
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Quad

#[derive(Debug, Default, Clone, Copy)]
pub struct Quad {
    pub vert_right_top: Vec2,
    pub vert_right_bottom: Vec2,
    pub vert_left_bottom: Vec2,
    pub vert_left_top: Vec2,
}

impl Quad {
    #[inline]
    pub fn from_rect(rect: Rect) -> Quad {
        Quad {
            vert_right_top: Vec2::new(rect.right(), rect.top()),
            vert_right_bottom: Vec2::new(rect.right(), rect.bottom()),
            vert_left_bottom: Vec2::new(rect.left(), rect.bottom()),
            vert_left_top: Vec2::new(rect.left(), rect.top()),
        }
    }

    #[inline]
    pub fn from_rect_transformed(
        rect_dim: Vec2,
        pivot: Vec2,
        translation: Vec2,
        scale: Vec2,
        rotation_dir: Vec2,
    ) -> Quad {
        Quad {
            vert_right_top: Vec2::new(rect_dim.x, 0.0).transformed(
                pivot,
                translation,
                scale,
                rotation_dir,
            ),
            vert_right_bottom: Vec2::new(rect_dim.x, rect_dim.y).transformed(
                pivot,
                translation,
                scale,
                rotation_dir,
            ),
            vert_left_bottom: Vec2::new(0.0, rect_dim.y).transformed(
                pivot,
                translation,
                scale,
                rotation_dir,
            ),
            vert_left_top: Vec2::new(0.0, 0.0).transformed(pivot, translation, scale, rotation_dir),
        }
    }

    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        let mut left = f32::min(self.vert_left_top.x, self.vert_left_bottom.x);
        let mut top = f32::min(self.vert_left_top.y, self.vert_right_top.y);
        let mut right = f32::max(self.vert_right_top.x, self.vert_right_bottom.x);
        let mut bottom = f32::max(self.vert_left_bottom.y, self.vert_right_bottom.y);

        // NOTE: It can happen that our quad is weirdly mirrored and/or rotated so we possibly need to
        // normalize our rect
        if left > right {
            std::mem::swap(&mut left, &mut right);
        }
        if top > bottom {
            std::mem::swap(&mut top, &mut bottom);
        }

        Rect::from_bounds_left_top_right_bottom(left, top, right, bottom)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Line

#[derive(Debug, Clone, Copy, Default)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Line {
    #[inline]
    pub fn new(start: Point, end: Point) -> Line {
        Line { start, end }
    }

    #[must_use]
    #[inline]
    pub fn translated_by(self, translation: Vec2) -> Line {
        Line {
            start: self.start + translation,
            end: self.end + translation,
        }
    }

    #[inline]
    pub fn intersection_point(self, t: f32) -> Point {
        self.start + t * (self.end - self.start)
    }

    #[inline]
    pub fn dir(self) -> Vec2 {
        self.end - self.start
    }

    #[inline]
    pub fn length(self) -> f32 {
        Vec2::distance(self.start, self.end)
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        Vec2::distance_squared(self.start, self.end)
    }

    #[inline]
    pub fn normal(self) -> Vec2 {
        (self.end - self.start).perpendicular().normalized()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Circle

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Point,
    pub radius: f32,
}

impl Circle {
    #[inline]
    pub fn new(center: Point, radius: f32) -> Circle {
        Circle { center, radius }
    }

    /// For a given radius (in pixels) this returns the number of vertices a circle needs to have a
    /// visible pixel error of less than 1/2 pixels
    pub fn get_optimal_vertex_count_for_drawing(radius: f32) -> usize {
        if radius < 0.5 {
            return 4;
        }

        // Based on https://stackoverflow.com/a/11774493 with maximum error of 1/2 pixel
        let num_vertices =
            ceili(2.0 * PI / f32::acos(2.0 * (1.0 - 0.5 / radius) * (1.0 - 0.5 / radius) - 1.0));

        clampi(make_even_upwards(num_vertices), 4, 128) as usize
    }

    #[inline]
    pub fn to_linesegments(self, num_segments: usize) -> Vec<Line> {
        let points: Vec<Point> = (0..=num_segments)
            .map(|index| {
                Point::new(
                    f32::cos(2.0 * (index as f32) * PI / (num_segments as f32)),
                    f32::sin(2.0 * (index as f32) * PI / (num_segments as f32)),
                )
            })
            .map(|point| self.center + self.radius * point)
            .collect();

        let mut lines = Vec::new();
        for index in 0..num_segments {
            let line = Line::new(points[index], points[index + 1]);
            lines.push(line);
        }
        lines
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Physics

/// Solves the linear motion equations:
///
/// distance_final(t) = vel_start * t + 1/2 * acc_start * t^2
/// vel(t) = vel_start + acc_start * t
///
/// for final distance_final
#[inline]
pub fn linear_motion_get_final_distance(vel_start: f32, acc_start: f32, time_final: f32) -> f32 {
    vel_start + 0.5 * acc_start * (time_final * time_final)
}

/// Solves the linear motion equations:
///
/// distance_final(t) = vel_start * t + 1/2 * acc_start * t^2
/// vel(t) = vel_start + acc_start * t
///
/// for `vel_start` and `acc_start` such that
///
/// vel(time_final) = 0
/// distance_final(time_final) = distance_final
///
#[inline]
pub fn linear_motion_get_start_vel_and_start_acc(
    distance_final: f32,
    time_final: f32,
    out_vel_start: &mut f32,
    out_acc_start: &mut f32,
) {
    let acc_start = -2.0 * distance_final / (time_final * time_final);
    let vel_start = -acc_start * time_final;
    *out_acc_start = acc_start;
    *out_vel_start = vel_start;
}

/// Solves the linear motion equations:
///
/// distance_final(t) = vel_start * t + 1/2 * acc_start * t^2
/// vel(t) = vel_start + acc_start * t
///
/// for `distance_final` and `acc_start` such that
///
/// vel(0) = vel_start
/// vel(time_final) = 0
///
#[inline]
pub fn linear_motion_get_start_acc_and_final_resting_distance(
    vel_start: f32,
    time_final: f32,
    out_distance_final: &mut f32,
    out_acc_start: &mut f32,
) {
    debug_assert!(!is_effectively_zero(time_final));
    let acc_start = -vel_start / time_final;
    let distance_final = (vel_start * time_final) / 2.0;
    *out_distance_final = distance_final;
    *out_acc_start = acc_start;
}

/// Solves the linear motion equations:
///
/// distance(t) = vel_start * t + 1/2 * acc_start * t^2
/// vel(t) = vel_start + acc_start * t
///
/// for `vel_start` such that
///
/// distance_final(time_final) = distance_final
/// with given acc_start
///
#[inline]
pub fn linear_motion_relative_get_starting_vel(
    distance_final: f32,
    time_final: f32,
    acc_start: f32,
) -> f32 {
    debug_assert!(!is_effectively_zero(time_final));
    (distance_final / time_final) - (acc_start * time_final) / 2.0
}

/// Solves the linear motion equations:
///
/// pos(t) = pos_start + vel_start * t + 1/2 * acc_start * t^2
/// vel(t) = vel_start + acc_start * t
///
/// for `vel_start` such that
///
/// pos(time_final) = pos_final
/// with given acc_start
///
#[inline]
pub fn linear_motion_absolute_get_starting_vel(
    pos_start: f32,
    pos_final: f32,
    time_final: f32,
    acc_start: f32,
) -> f32 {
    let distance_final = pos_final - pos_start;
    linear_motion_relative_get_starting_vel(distance_final, time_final, acc_start)
}

#[inline]
pub fn trajectory_get_starting_vel(
    hit_time: f32,
    start_pos: Vec2,
    hit_pos: Vec2,
    forces: Vec2,
) -> Vec2 {
    let vel_start_x =
        linear_motion_absolute_get_starting_vel(start_pos.x, hit_pos.x, hit_time, forces.x);
    let vel_start_y =
        linear_motion_absolute_get_starting_vel(start_pos.y, hit_pos.y, hit_time, forces.y);
    Vec2::new(vel_start_x, vel_start_y)
}

#[inline]
pub fn trajectory_get_position(
    vel_start: Vec2,
    pos_start: Vec2,
    time_current: f32,
    forces: Vec2,
) -> Vec2 {
    pos_start + vel_start * time_current + (forces * time_current * time_current) / 2.0
}

#[inline]
pub fn linear_motion_integrate(
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    acc: f32,
    vel_max_abs: f32,
    deltatime: f32,
) {
    // This uses velocity-verlet integration because euler integration has a huge error even for
    // fixed delta times
    // https://jdickinsongames.wordpress.com/2015/01/22/numerical-integration-in-games-development-2/

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    let mut vel_halfstep = vel + (acc * deltatime / 2.0);
    vel_halfstep = clampf_absolute(vel_halfstep, vel_max_abs);
    pos += vel_halfstep * deltatime;
    vel = vel_halfstep + (acc * deltatime / 2.0);
    vel = clampf_absolute(vel, vel_max_abs);

    *inout_pos = pos;
    *inout_vel = vel;
}

#[inline]
pub fn linear_motion_integrate_with_drag(
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    acc: f32,
    drag: f32,
    vel_max: f32,
    deltatime: f32,
) {
    // This uses velocity-verlet integration because euler integration has a huge error even for
    // fixed delta times
    // https://jdickinsongames.wordpress.com/2015/01/22/numerical-integration-in-games-development-2/

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    let drag_final = -f32::signum(vel) * drag;
    if f32::abs(drag_final) > f32::abs(acc) {
        // Prevent velocity sign flips if we decelerate via drag forces
        let acc_final = acc + drag_final;
        let mut vel_halfstep = add_or_zero_when_changing_sign(vel, acc_final * deltatime / 2.0);
        vel_halfstep = clampf_absolute(vel_halfstep, vel_max);
        pos += vel_halfstep * deltatime;
        vel = add_or_zero_when_changing_sign(vel_halfstep, acc_final * deltatime / 2.0);
        vel = clampf_absolute(vel, vel_max);
    } else {
        // Normal acceleration with possible velocity sign-flip
        let acc_final = acc + drag_final;
        let mut vel_halfstep = vel + (acc_final * deltatime / 2.0);
        vel_halfstep = clampf_absolute(vel_halfstep, vel_max);
        pos += vel_halfstep * deltatime;
        vel = vel_halfstep + acc_final * deltatime / 2.0;
        vel = clampf_absolute(vel, vel_max);
    }

    *inout_pos = pos;
    *inout_vel = vel;
}

#[inline]
pub fn linear_motion_integrate_v2(
    inout_pos: &mut Vec2,
    inout_vel: &mut Vec2,
    acc: Vec2,
    vel_max: f32,
    deltatime: f32,
) {
    // This uses velocity-verlet integration because euler integration has a huge error even for
    // fixed delta times
    // https://jdickinsongames.wordpress.com/2015/01/22/numerical-integration-in-games-development-2/

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    let mut vel_halfstep = vel + (acc * deltatime / 2.0);
    vel_halfstep = Vec2::clamped_abs(vel_halfstep, vel_max);
    pos += vel_halfstep * deltatime;
    vel = vel_halfstep + acc * deltatime / 2.0;
    vel = Vec2::clamped_abs(vel, vel_max);

    *inout_pos = pos;
    *inout_vel = vel;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sampling / interpolation

/// IMPORTANT: source_min <= source_samplepoint < source_max
///            dest_min < dest_max
pub fn sample_integer_upper_exclusive_floored(
    source_samplepoint: i32,
    source_min: i32,
    source_max: i32,
    dest_min: i32,
    dest_max: i32,
) -> i32 {
    debug_assert!(source_min <= source_samplepoint && source_samplepoint < source_max);
    debug_assert!(dest_min < dest_max);

    let source_width = source_max - source_min;
    let dest_width = dest_max - dest_min;

    let relative_point = (source_samplepoint - source_min) as f32 / source_width as f32;
    let dest_point = dest_min as f32 + relative_point * dest_width as f32;

    f32::floor(dest_point) as i32
}
