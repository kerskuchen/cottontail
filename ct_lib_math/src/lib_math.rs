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

pub mod easing;

mod random;
pub use random::*;

pub type Point = Vec2;
pub type Pointi = Vec2i;

use num_traits::Num;

//--------------------------------------------------------------------------------------------------
// Misc

// NOTE: For f32 We can use -16383.999 to 16383.999 and still have a precision of at least 0.001!

pub const PI: f32 = std::f32::consts::PI;
pub const PI64: f64 = std::f64::consts::PI;

pub const RADIANS_TO_DEGREE: f32 = 57.2957795;
pub const DEGREE_TO_RADIANS: f32 = 1.0 / 57.2957795;

pub trait Lerp: Copy {
    fn lerp_value(start: Self, end: Self, percent: f32) -> Self;
}

impl Lerp for f32 {
    #[inline]
    fn lerp_value(start: f32, end: f32, percent: f32) -> f32 {
        lerp(start, end, percent)
    }
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

/// Based on linear panning (http://gdsp.hf.ntnu.no/lessons/1/5/)
#[inline]
pub fn crossfade_linear(factor: f32, percent: f32) -> (f32, f32) {
    let left = factor * lerp(0.0, 1.0, percent);
    let right = factor * lerp(1.0, 0.0, percent);
    (left, right)
}

/// Based on squareroot panning (http://gdsp.hf.ntnu.no/lessons/1/5/)
#[inline]
pub fn crossfade_squareroot(factor: f32, percent: f32) -> (f32, f32) {
    let left = factor * f32::sqrt(1.0 - percent);
    let right = factor * f32::sqrt(percent);
    (left, right)
}

/// Based on sinuoidal panning (http://gdsp.hf.ntnu.no/lessons/1/5/)
#[inline]
pub fn crossfade_sinuoidal(factor: f32, percent: f32) -> (f32, f32) {
    // NOTE: We need to either clamp here or make the factor in the cos term sligthly smaller
    // because we we get `cos((PI/2.0) * percent) = -0.00000004371139` for `percent = 1`
    // Because clamp is expensive we use the numeric hack
    let left = factor * f32::cos(((PI / 2.0) - f32::EPSILON) * percent);
    let right = factor * f32::sin((PI / 2.0) * percent);
    (left, right)
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

/// Wraps a value in the range [start, begin]
#[inline]
pub fn wrap_value_in_interval(value: f32, start: f32, end: f32) -> f32 {
    debug_assert!(start < end);
    let range = end - start;
    start + f32::rem_euclid(value, range)
}

/// Wraps a value in the range [0, range]
#[inline]
pub fn wrap_value_in_range(value: f32, range: f32) -> f32 {
    debug_assert!(range > 0.0);
    f32::rem_euclid(value, range)
}

/// Wraps an angle to the range [-360, 360]. Note that this operation does not change the cos and
/// sin of the angle value
#[inline]
pub fn wrap_angle(deg_angle: f32) -> f32 {
    deg_angle % 360.0
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
pub fn ceil_to_multiple_of_target_i32(value: i32, target: i32) -> i32 {
    assert!(target > 0);

    let remainder = value % target;
    if remainder == 0 {
        return value;
    }
    if value >= 0 {
        value + (target - remainder)
    } else {
        // NOTE: remainer is negative because value is negative
        value - remainder
    }
}

#[inline]
pub fn floor_to_multiple_of_target_i32(value: i32, target: i32) -> i32 {
    assert!(target > 0);

    let remainder = value % target;
    if remainder == 0 {
        return value;
    }
    if value >= 0 {
        value - remainder
    } else {
        // NOTE: remainer is negative because value is negative
        value - (target + remainder)
    }
}

#[inline]
pub fn round_to_multiple_of_target(value: f32, target: f32) -> f32 {
    f32::round(value / target) * target
}

#[inline]
pub fn floor_to_multiple_of_target(value: f32, target: f32) -> f32 {
    f32::floor(value / target) * target
}

#[inline]
pub fn ceil_to_multiple_of_target(value: f32, target: f32) -> f32 {
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

#[derive(Default, Debug, Copy, Clone)]
pub struct Transform {
    pub pos: Vec2,
    pub scale: Vec2,
    /// Angle given in degrees [-360, 360] counterclockwise
    pub dir_angle: f32,
}

impl Transform {
    #[inline]
    pub const fn from_pos(pos: Vec2) -> Transform {
        Transform {
            pos,
            scale: Vec2::ones(),
            dir_angle: 0.0,
        }
    }

    #[inline]
    pub const fn from_pos_angle(pos: Vec2, dir_angle: f32) -> Transform {
        Transform {
            pos,
            scale: Vec2::ones(),
            dir_angle,
        }
    }

    #[inline]
    pub fn from_pos_dir(pos: Vec2, dir: Vec2) -> Transform {
        Transform {
            pos,
            scale: Vec2::ones(),
            dir_angle: dir.to_angle_flipped_y(),
        }
    }

    #[inline]
    pub const fn from_pos_scale(pos: Vec2, scale: Vec2) -> Transform {
        Transform {
            pos,
            scale,
            dir_angle: 0.0,
        }
    }

    #[inline]
    pub const fn from_pos_scale_uniform(pos: Vec2, scale: f32) -> Transform {
        Transform {
            pos,
            scale: Vec2::filled(scale),
            dir_angle: 0.0,
        }
    }

    #[inline]
    pub const fn from_pos_scale_angle(pos: Vec2, scale: Vec2, dir_angle: f32) -> Transform {
        Transform {
            pos,
            scale,
            dir_angle,
        }
    }

    #[inline]
    pub fn rotation_dir(&self) -> Vec2 {
        Vec2::from_angle_flipped_y(self.dir_angle)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Quad {
    pub vert_right_top: Vec2,
    pub vert_right_bottom: Vec2,
    pub vert_left_bottom: Vec2,
    pub vert_left_top: Vec2,
}

impl Quad {
    #[inline]
    pub fn to_linestrip(&self) -> [Vec2; 5] {
        [
            self.vert_right_top,
            self.vert_right_bottom,
            self.vert_left_bottom,
            self.vert_left_top,
            self.vert_right_top,
        ]
    }

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
    pub fn from_rect_transformed(rect_dim: Vec2, pivot: Vec2, xform: Transform) -> Quad {
        Quad {
            vert_right_top: Vec2::new(rect_dim.x, 0.0).transformed(pivot, xform),
            vert_right_bottom: Vec2::new(rect_dim.x, rect_dim.y).transformed(pivot, xform),
            vert_left_bottom: Vec2::new(0.0, rect_dim.y).transformed(pivot, xform),
            vert_left_top: Vec2::new(0.0, 0.0).transformed(pivot, xform),
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

#[inline]
pub fn iterate_line_bresenham<FunctorType: FnMut(i32, i32)>(
    start: Vec2i,
    end: Vec2i,
    skip_last_point: bool,
    action: &mut FunctorType,
) {
    // Based on (the last one of)
    // https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm#All_cases
    let width = (end.x - start.x).abs();
    let height = -(end.y - start.y).abs();

    let increment_x = if start.x < end.x { 1 } else { -1 };
    let increment_y = if start.y < end.y { 1 } else { -1 };

    let mut err = width + height;

    let mut x = start.x;
    let mut y = start.y;
    loop {
        if x == end.x && y == end.y {
            if !skip_last_point {
                action(x, y);
            }
            break;
        }

        action(x, y);

        let err_previous = 2 * err;
        if err_previous >= height {
            err += height;
            x += increment_x;
        }
        if err_previous <= width {
            err += width;
            y += increment_y;
        }
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
    #[inline]
    pub fn get_optimal_vertex_count_for_drawing(radius: f32) -> usize {
        if radius < 0.5 {
            return 4;
        }

        // Based on https://stackoverflow.com/a/11774493 with maximum error of 1/2 pixel
        let num_vertices =
            ceili(2.0 * PI / f32::acos(2.0 * (1.0 - 0.5 / radius) * (1.0 - 0.5 / radius) - 1.0));

        i32::clamp(make_even_upwards(num_vertices), 4, 128) as usize
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
    debug_assert!(vel_max_abs >= 0.0);

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
fn vel_quantized(vel: f32, ticks_per_second: f32) -> f32 {
    let result = if vel >= ticks_per_second {
        round_to_multiple_of_target(vel, ticks_per_second)
    } else if vel <= -ticks_per_second {
        round_to_multiple_of_target(vel, ticks_per_second)
    } else {
        round_to_multiple_of_target(vel, ticks_per_second / 2.0)
    };

    if result == 0.0 && !is_effectively_zero(vel) {
        vel.signum() * ticks_per_second / 2.0
    } else {
        result
    }
}

/// Adds `to_add` to the value of a given number `x`. Returns 0 if adding `to_add` to `x` changes
/// the sign of x or was already 0
///
/// Example:
/// add_or_zero_when_changing_sign(5, 3) = 8
/// add_or_zero_when_changing_sign(-2, 4) = 0
/// add_or_zero_when_changing_sign(4, -2) = 2
/// add_or_zero_when_changing_sign(4, -5) = 0
/// add_or_zero_when_changing_sign(0, 2) = 0
/// add_or_zero_when_changing_sign(0, -3) = 0
#[inline]
pub fn add_or_zero_when_changing_sign(x: f32, to_add: f32) -> f32 {
    let sum = x + to_add;
    let have_same_sign = sum * x > 0.0;

    if have_same_sign {
        // Adding did not change the sign of x
        sum
    } else {
        0.0
    }
}

#[inline]
pub fn linear_motion_integrate_with_drag(
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    acc: f32,
    drag: f32,
    vel_max_abs: f32,
    deltatime: f32,
) {
    debug_assert!(vel_max_abs >= 0.0);
    debug_assert!(drag >= 0.0);

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
        vel_halfstep = clampf_absolute(vel_halfstep, vel_max_abs);
        pos += vel_halfstep * deltatime;
        vel = add_or_zero_when_changing_sign(vel_halfstep, acc_final * deltatime / 2.0);
        vel = clampf_absolute(vel, vel_max_abs);
    } else {
        // Normal acceleration with possible velocity sign-flip
        let acc_final = acc + drag_final;
        let mut vel_halfstep = vel + (acc_final * deltatime / 2.0);
        vel_halfstep = clampf_absolute(vel_halfstep, vel_max_abs);
        pos += vel_halfstep * deltatime;
        vel = vel_halfstep + acc_final * deltatime / 2.0;
        vel = clampf_absolute(vel, vel_max_abs);
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

#[inline]
pub fn linear_motion_integrate_with_drag_v2(
    inout_pos: &mut Vec2,
    inout_vel: &mut Vec2,
    acc: Vec2,
    drag: f32,
    vel_max: f32,
    deltatime: f32,
) {
    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    linear_motion_integrate_with_drag(&mut pos.x, &mut vel.x, acc.x, drag, vel_max, deltatime);
    linear_motion_integrate_with_drag(&mut pos.y, &mut vel.y, acc.y, drag, vel_max, deltatime);
    vel = Vec2::clamped_abs(vel, vel_max);

    *inout_pos = pos;
    *inout_vel = vel;
}

/// Same as `linear_motion_integrate` but makes sure that the position is updated at either
/// 0.5 or n pixels per tick where n is an integer. This gets rid of movement stutter where objects
/// for example move 1, 1, 1, 2, 1, 1, 1, 2, 1, 1, 1, 2, ... pixels because `deltatime * vel = 1.25`
#[inline]
pub fn linear_motion_integrate_quantized_vel(
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    acc: f32,
    vel_max_abs: f32,
    deltatime: f32,
    ticks_per_second: f32,
) {
    debug_assert!(vel_max_abs >= 0.0);

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    vel = vel + acc * deltatime;
    vel = clampf_absolute(vel, vel_max_abs);
    pos += vel_quantized(vel, ticks_per_second) * deltatime;

    *inout_pos = pos;
    *inout_vel = vel;
}

/// Same as `linear_motion_integrate_with_drag` but makes sure that the position is updated at either
/// 0.5 or n pixels per tick where n is an integer. This gets rid of movement stutter where objects
/// for example move 1, 1, 1, 2, 1, 1, 1, 2, 1, 1, 1, 2, ... pixels because `deltatime * vel = 1.25`
#[inline]
pub fn linear_motion_integrate_with_drag_quantized_vel(
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    acc: f32,
    drag: f32,
    vel_max_abs: f32,
    deltatime: f32,
    ticks_per_second: f32,
) {
    debug_assert!(vel_max_abs >= 0.0);
    debug_assert!(drag >= 0.0);

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    let drag_final = -f32::signum(vel) * drag;
    let acc_final = acc + drag_final;
    if f32::abs(drag_final) > f32::abs(acc) {
        // Prevent velocity sign flips if we decelerate via drag forces
        vel = add_or_zero_when_changing_sign(vel, acc_final * deltatime);
    } else {
        // Normal acceleration with possible velocity sign-flip
        vel = vel + acc_final * deltatime;
    }
    vel = clampf_absolute(vel, vel_max_abs);
    pos += vel_quantized(vel, ticks_per_second) * deltatime;

    *inout_pos = pos;
    *inout_vel = vel;
}

#[inline]
pub fn linear_motion_integrate_with_drag_quantized_vel_v2(
    inout_pos: &mut Vec2,
    inout_vel: &mut Vec2,
    acc: Vec2,
    drag: f32,
    vel_max_abs: f32,
    deltatime: f32,
    ticks_per_second: f32,
) {
    debug_assert!(vel_max_abs >= 0.0);
    debug_assert!(drag >= 0.0);

    let mut vel = *inout_vel;
    let mut pos = *inout_pos;

    let drag_final = -Vec2::new(vel.x.signum(), vel.y.signum()) * drag;
    let acc_final = acc + drag_final;
    if f32::abs(drag_final.x) > f32::abs(acc.x) {
        // Prevent velocity sign flips if we decelerate via drag forces
        vel.x = add_or_zero_when_changing_sign(vel.x, acc_final.x * deltatime);
    } else {
        // Normal acceleration with possible velocity sign-flip
        vel.x = vel.x + acc_final.x * deltatime;
    }
    if f32::abs(drag_final.y) > f32::abs(acc.y) {
        // Prevent velocity sign flips if we decelerate via drag forces
        vel.y = add_or_zero_when_changing_sign(vel.y, acc_final.y * deltatime);
    } else {
        // Normal acceleration with possible velocity sign-flip
        vel.y = vel.y + acc_final.y * deltatime;
    }
    vel = Vec2::clamped_abs(vel, vel_max_abs);
    pos.x += vel_quantized(vel.x, ticks_per_second) * deltatime;
    pos.y += vel_quantized(vel.y, ticks_per_second) * deltatime;

    *inout_pos = pos;
    *inout_vel = vel;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sampling / interpolation

/// IMPORTANT: source_min <= source_samplepoint < source_max
///            dest_min < dest_max
#[inline]
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Intervals

use std::ops::Range;

/// This represents the half open integer interval [start, end[ or [start, end-1] respectively
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Interval {
    pub start: i64,
    pub end: i64,
}

// Conversion
impl Interval {
    #[inline]
    pub fn as_range(self) -> Range<i64> {
        self.start..self.end
    }

    #[inline]
    pub fn as_range_usize(self) -> Range<usize> {
        use std::convert::TryFrom;
        assert!(0 <= self.start && self.start <= self.end);
        let start = usize::try_from(self.start).unwrap_or_else(|error| {
            panic!(
                "Failed to create range: cannot convert start={} to usize: {}",
                self.start, error
            )
        });
        let end = usize::try_from(self.end).unwrap_or_else(|error| {
            panic!(
                "Failed to create range: cannot convert end={} to usize: {}",
                self.end, error
            )
        });
        start..end
    }
}

// Creation
impl Interval {
    #[inline]
    pub fn new(start: i64, length: usize) -> Interval {
        Interval {
            start,
            end: start + length as i64,
        }
    }

    #[inline]
    pub fn from_range(range: Range<i64>) -> Interval {
        Interval {
            start: range.start,
            end: range.end,
        }
    }

    #[inline]
    pub fn from_start_end(start: i64, end: i64) -> Interval {
        Interval { start, end }
    }
}

// Operations
impl Interval {
    #[inline]
    pub fn len(self) -> usize {
        use std::convert::TryFrom;
        let len = i64::max(0, self.end - self.start);
        usize::try_from(len).unwrap_or_else(|error| {
            panic!(
                "Failed to determine length of range: cannot convert {} to usize: {}",
                len, error
            )
        })
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }

    #[must_use]
    #[inline]
    pub fn offsetted_by(self, offset: i64) -> Interval {
        Interval {
            start: self.start + offset,
            end: self.end + offset,
        }
    }

    #[inline]
    pub fn intersect(a: Interval, b: Interval) -> Interval {
        Interval {
            start: i64::max(a.start, b.start),
            end: i64::min(a.end, b.end),
        }
    }

    /// Returns the set-theoretic difference
    ///   `a - b = a / (a intersection b)`
    /// as (left, right)
    #[inline]
    pub fn difference(a: Interval, b: Interval) -> (Interval, Interval) {
        let intersection = Interval::intersect(a, b);
        let left = Interval {
            start: a.start,
            end: intersection.start,
        };
        let right = Interval {
            start: intersection.end,
            end: a.end,
        };

        (left, right)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Easing

#[derive(Debug, Copy, Clone)]
pub enum EasingType {
    Linear,
    CubicInOut,
    StepMiddle,
    StepEnd,
}

#[inline]
pub fn ease(percent: f32, easing_type: EasingType) -> f32 {
    match easing_type {
        EasingType::Linear => percent,
        EasingType::CubicInOut => easing::cubic_inout(percent),
        EasingType::StepMiddle => easing::step_middle(percent),
        EasingType::StepEnd => easing::step_end(percent),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceil_to_multiple_of_target_i32_test() {
        assert_eq!(ceil_to_multiple_of_target_i32(-20, 10), -20);
        assert_eq!(ceil_to_multiple_of_target_i32(-19, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-18, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-17, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-16, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-15, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-14, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-13, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-12, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-11, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-10, 10), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-9, 10), -0);
        assert_eq!(ceil_to_multiple_of_target_i32(-8, 10), -0);
        assert_eq!(ceil_to_multiple_of_target_i32(-6, 10), -0);
        assert_eq!(ceil_to_multiple_of_target_i32(-5, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-4, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-3, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-2, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-1, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(0, 10), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(1, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(2, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(3, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(4, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(5, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(6, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(7, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(8, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(9, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(10, 10), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(11, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(12, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(13, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(14, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(15, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(16, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(17, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(18, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(19, 10), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(20, 10), 20);

        assert_eq!(ceil_to_multiple_of_target_i32(-20, 5), -20);
        assert_eq!(ceil_to_multiple_of_target_i32(-19, 5), -15);
        assert_eq!(ceil_to_multiple_of_target_i32(-18, 5), -15);
        assert_eq!(ceil_to_multiple_of_target_i32(-17, 5), -15);
        assert_eq!(ceil_to_multiple_of_target_i32(-16, 5), -15);
        assert_eq!(ceil_to_multiple_of_target_i32(-15, 5), -15);
        assert_eq!(ceil_to_multiple_of_target_i32(-14, 5), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-13, 5), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-12, 5), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-11, 5), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-10, 5), -10);
        assert_eq!(ceil_to_multiple_of_target_i32(-9, 5), -5);
        assert_eq!(ceil_to_multiple_of_target_i32(-8, 5), -5);
        assert_eq!(ceil_to_multiple_of_target_i32(-6, 5), -5);
        assert_eq!(ceil_to_multiple_of_target_i32(-5, 5), -5);
        assert_eq!(ceil_to_multiple_of_target_i32(-4, 5), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-3, 5), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-2, 5), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(-1, 5), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(0, 5), 0);
        assert_eq!(ceil_to_multiple_of_target_i32(1, 5), 5);
        assert_eq!(ceil_to_multiple_of_target_i32(2, 5), 5);
        assert_eq!(ceil_to_multiple_of_target_i32(3, 5), 5);
        assert_eq!(ceil_to_multiple_of_target_i32(4, 5), 5);
        assert_eq!(ceil_to_multiple_of_target_i32(5, 5), 5);
        assert_eq!(ceil_to_multiple_of_target_i32(6, 5), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(7, 5), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(8, 5), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(9, 5), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(10, 5), 10);
        assert_eq!(ceil_to_multiple_of_target_i32(11, 5), 15);
        assert_eq!(ceil_to_multiple_of_target_i32(12, 5), 15);
        assert_eq!(ceil_to_multiple_of_target_i32(13, 5), 15);
        assert_eq!(ceil_to_multiple_of_target_i32(14, 5), 15);
        assert_eq!(ceil_to_multiple_of_target_i32(15, 5), 15);
        assert_eq!(ceil_to_multiple_of_target_i32(16, 5), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(17, 5), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(18, 5), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(19, 5), 20);
        assert_eq!(ceil_to_multiple_of_target_i32(20, 5), 20);
    }

    #[test]
    fn floor_to_multiple_of_target_i32_test() {
        assert_eq!(floor_to_multiple_of_target_i32(-20, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-19, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-18, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-17, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-16, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-15, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-14, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-13, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-12, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-11, 10), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-10, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-9, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-8, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-6, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-5, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-4, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-3, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-2, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-1, 10), -10);
        assert_eq!(floor_to_multiple_of_target_i32(0, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(1, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(2, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(3, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(4, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(5, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(6, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(7, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(8, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(9, 10), 0);
        assert_eq!(floor_to_multiple_of_target_i32(10, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(11, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(12, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(13, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(14, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(15, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(16, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(17, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(18, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(19, 10), 10);
        assert_eq!(floor_to_multiple_of_target_i32(20, 10), 20);

        assert_eq!(floor_to_multiple_of_target_i32(-20, 5), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-19, 5), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-18, 5), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-17, 5), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-16, 5), -20);
        assert_eq!(floor_to_multiple_of_target_i32(-15, 5), -15);
        assert_eq!(floor_to_multiple_of_target_i32(-14, 5), -15);
        assert_eq!(floor_to_multiple_of_target_i32(-13, 5), -15);
        assert_eq!(floor_to_multiple_of_target_i32(-12, 5), -15);
        assert_eq!(floor_to_multiple_of_target_i32(-11, 5), -15);
        assert_eq!(floor_to_multiple_of_target_i32(-10, 5), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-9, 5), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-8, 5), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-6, 5), -10);
        assert_eq!(floor_to_multiple_of_target_i32(-5, 5), -5);
        assert_eq!(floor_to_multiple_of_target_i32(-4, 5), -5);
        assert_eq!(floor_to_multiple_of_target_i32(-3, 5), -5);
        assert_eq!(floor_to_multiple_of_target_i32(-2, 5), -5);
        assert_eq!(floor_to_multiple_of_target_i32(-1, 5), -5);
        assert_eq!(floor_to_multiple_of_target_i32(0, 5), 0);
        assert_eq!(floor_to_multiple_of_target_i32(1, 5), 0);
        assert_eq!(floor_to_multiple_of_target_i32(2, 5), 0);
        assert_eq!(floor_to_multiple_of_target_i32(3, 5), 0);
        assert_eq!(floor_to_multiple_of_target_i32(4, 5), 0);
        assert_eq!(floor_to_multiple_of_target_i32(5, 5), 5);
        assert_eq!(floor_to_multiple_of_target_i32(6, 5), 5);
        assert_eq!(floor_to_multiple_of_target_i32(7, 5), 5);
        assert_eq!(floor_to_multiple_of_target_i32(8, 5), 5);
        assert_eq!(floor_to_multiple_of_target_i32(9, 5), 5);
        assert_eq!(floor_to_multiple_of_target_i32(10, 5), 10);
        assert_eq!(floor_to_multiple_of_target_i32(11, 5), 10);
        assert_eq!(floor_to_multiple_of_target_i32(12, 5), 10);
        assert_eq!(floor_to_multiple_of_target_i32(13, 5), 10);
        assert_eq!(floor_to_multiple_of_target_i32(14, 5), 10);
        assert_eq!(floor_to_multiple_of_target_i32(15, 5), 15);
        assert_eq!(floor_to_multiple_of_target_i32(16, 5), 15);
        assert_eq!(floor_to_multiple_of_target_i32(17, 5), 15);
        assert_eq!(floor_to_multiple_of_target_i32(18, 5), 15);
        assert_eq!(floor_to_multiple_of_target_i32(19, 5), 15);
        assert_eq!(floor_to_multiple_of_target_i32(20, 5), 20);
    }
}
