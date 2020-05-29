////////////////////////////////////////////////////////////////////////////////////////////////////
// 2D Vector

use super::*;

use serde_derive::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

//--------------------------------------------------------------------------------------------------
// Conversion

impl From<Vec2i> for Vec2 {
    #[inline]
    fn from(v: Vec2i) -> Self {
        Vec2 {
            x: v.x as f32,
            y: v.y as f32,
        }
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from(pair: (f32, f32)) -> Self {
        Vec2 {
            x: pair.0,
            y: pair.1,
        }
    }
}

impl From<(i32, i32)> for Vec2 {
    #[inline]
    fn from(pair: (i32, i32)) -> Self {
        Vec2 {
            x: pair.0 as f32,
            y: pair.1 as f32,
        }
    }
}

impl Vec2 {
    #[inline]
    pub fn floor(self) -> Vec2 {
        Vec2 {
            x: f32::floor(self.x),
            y: f32::floor(self.y),
        }
    }

    #[inline]
    pub fn floori(self) -> Vec2i {
        Vec2i {
            x: floori(self.x),
            y: floori(self.y),
        }
    }

    #[inline]
    pub fn round(self) -> Vec2 {
        Vec2 {
            x: f32::round(self.x),
            y: f32::round(self.y),
        }
    }

    #[inline]
    pub fn roundi(self) -> Vec2i {
        Vec2i {
            x: roundi(self.x),
            y: roundi(self.y),
        }
    }

    #[inline]
    pub fn ceil(self) -> Vec2 {
        Vec2 {
            x: f32::ceil(self.x),
            y: f32::ceil(self.y),
        }
    }

    #[inline]
    pub fn ceili(self) -> Vec2i {
        Vec2i {
            x: ceili(self.x),
            y: ceili(self.y),
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    #[inline]
    pub const fn zero() -> Vec2 {
        Vec2::new(0.0, 0.0)
    }

    #[inline]
    pub const fn ones() -> Vec2 {
        Vec2::new(1.0, 1.0)
    }

    #[inline]
    pub const fn unit_x() -> Vec2 {
        Vec2::new(1.0, 0.0)
    }

    #[inline]
    pub const fn unit_y() -> Vec2 {
        Vec2::new(0.0, 1.0)
    }

    #[inline]
    pub const fn filled_x(x: f32) -> Vec2 {
        Vec2::new(x, 0.0)
    }

    #[inline]
    pub const fn filled_y(y: f32) -> Vec2 {
        Vec2::new(0.0, y)
    }

    #[inline]
    pub const fn filled(fill: f32) -> Vec2 {
        Vec2::new(fill, fill)
    }
}

//--------------------------------------------------------------------------------------------------
// Creation Special

impl Vec2 {
    /// Returns a unit vector constructed from a given angle in range [-180, 180]
    /// which represents the angle between the resulting vector and the vector (1,0) in the
    /// 2D cartesian coordinate system.
    pub fn from_angle(angle_deg: f32) -> Vec2 {
        let angle_rad = DEGREE_TO_RADIANS * angle_deg;
        Vec2::new(f32::cos(angle_rad), f32::sin(angle_rad))
    }

    /// Returns a unit vector constructed from a given angle in range [-180, 180]
    /// which represents the angle between the resulting vector and the vector (1,0) in a y-flipped
    /// 2D cartesian coordinate system.
    pub fn from_angle_flipped_y(angle_deg: f32) -> Vec2 {
        let angle_rad = DEGREE_TO_RADIANS * angle_deg;
        Vec2::new(f32::cos(-angle_rad), f32::sin(-angle_rad))
    }

    /// Returns a vector constructed from a given magnitude and an angle in range [-180, 180]
    /// which represents the angle between the resulting vector and the vector (1,0) in the
    /// 2D cartesian coordinate system.
    pub fn from_angle_magnitude(angle_deg: f32, magnitude: f32) -> Vec2 {
        let angle_rad = DEGREE_TO_RADIANS * angle_deg;
        Vec2::new(
            magnitude * f32::cos(angle_rad),
            magnitude * f32::sin(angle_rad),
        )
    }
}

//--------------------------------------------------------------------------------------------------
// Functions

impl Lerp for Vec2 {
    #[inline]
    fn lerp_value(start: Vec2, end: Vec2, percent: f32) -> Vec2 {
        Vec2::lerp(start, end, percent)
    }
}

impl Vec2 {
    #[inline]
    pub fn dot(v: Vec2, u: Vec2) -> f32 {
        v.x * u.x + v.y * u.y
    }

    #[must_use]
    #[inline]
    pub fn normalized(self) -> Vec2 {
        self / self.magnitude()
    }

    #[inline]
    pub fn is_normalized(self) -> bool {
        let normalized = Vec2::normalized(self);
        (normalized - self).is_effectively_zero()
    }

    #[inline]
    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn magnitude_squared(self) -> f32 {
        Vec2::dot(self, self)
    }

    #[inline]
    pub fn distance(v: Vec2, u: Vec2) -> f32 {
        (v - u).magnitude()
    }

    #[inline]
    pub fn distance_squared(v: Vec2, u: Vec2) -> f32 {
        (v - u).magnitude_squared()
    }

    #[inline]
    pub fn manhattan_distance(a: Vec2, b: Vec2) -> f32 {
        f32::abs(a.x - b.x) + f32::abs(a.y - b.y)
    }

    #[inline]
    /// Liner interpolation
    pub fn lerp(start: Vec2, end: Vec2, percent: f32) -> Vec2 {
        Vec2::new(lerp(start.x, end.x, percent), lerp(start.y, end.y, percent))
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        Vec2::dot(self, self) == 0.0
    }

    #[inline]
    pub fn is_effectively_zero(self) -> bool {
        Vec2::dot(self, self) < EPSILON
    }
}

//--------------------------------------------------------------------------------------------------
// Functions Special

impl Vec2 {
    /// Returns the given vector clamped to a given rect
    #[must_use]
    #[inline]
    pub fn clamped_to_recti(self, rect: Recti) -> Vec2 {
        Vec2::new(
            clampf(self.x, rect.left() as f32, rect.right() as f32),
            clampf(self.y, rect.top() as f32, rect.bottom() as f32),
        )
    }

    /// Returns the given vector clamped to a given rect
    #[must_use]
    #[inline]
    pub fn clamped_to_rect(self, rect: Rect) -> Vec2 {
        Vec2::new(
            clampf(self.x, rect.left(), rect.right()),
            clampf(self.y, rect.top(), rect.bottom()),
        )
    }

    /// Returns the "right" vector that is perpendicular to the given vector
    ///
    /// Example: Given the up vector (0,1) we get the right vector (1,0)
    ///
    /// NOTE: That in a flipped-y coordinate system we get the "left" vector
    ///
    /// It holds:
    /// dot(v2_perpendicular(v), v) = 0
    /// v x (0,0,1) = v2_perpendicular(v)
    #[must_use]
    #[inline]
    pub fn perpendicular(self) -> Vec2 {
        Vec2::new(self.y, -self.x)
    }

    /// Slides a given vector along a given normal
    #[must_use]
    #[inline]
    pub fn slided(self, normal: Vec2) -> Vec2 {
        self - Vec2::dot(self, normal) * normal
    }

    /// Reflects a given vector off a given normal
    #[must_use]
    #[inline]
    pub fn reflected(self, normal: Vec2) -> Vec2 {
        self - 2.0 * Vec2::dot(self, normal) * normal
    }

    /// Returns the z-component of a 3D cross-product of v and u as if they were 3D-vectors
    #[inline]
    pub fn cross_z(v: Vec2, u: Vec2) -> f32 {
        v.x * u.y - v.y * u.x
    }

    /// Example (0,1)x1 = (1,0)
    #[inline]
    pub fn cross_right(v: Vec2, z: f32) -> Vec2 {
        Vec2::new(z * v.y, -z * v.x)
    }

    /// Example 1x(0,1) = -((0,1)x1) = -(1,0)
    #[inline]
    pub fn cross_left(z: f32, v: Vec2) -> Vec2 {
        Vec2::new(-z * v.y, z * v.x)
    }

    /// Returns an angle in [-180, 180] with the following properties:
    /// If v and u are parallel and pointing into the same direction it returns zero
    /// If v is "left" of u then a positive value is returned, otherwise a negative
    /// If v and u are almost parallel and pointing into different directions it returns almost PI
    /// or -PI depending on if v is more to the "left" or to the "right" of u respectively
    ///
    /// NOTE: This function does not need for v and u to be normalized
    ///       (see https://stackoverflow.com/a/21486462)
    #[inline]
    pub fn signed_angle_between(v: Vec2, u: Vec2) -> f32 {
        let dot = Vec2::dot(v, u);
        let cross = -Vec2::cross_z(v, u);
        RADIANS_TO_DEGREE * f32::atan2(cross, dot)
    }

    /// Returns an angle in [-180, 180] which represents the angle between the given vector and
    /// the vector (1,0) in the 2D cartesian coordinate system.
    /// NOTE: This function does not need for v and u to be normalized
    #[inline]
    pub fn to_angle(self) -> f32 {
        Vec2::signed_angle_between(self, Vec2::unit_x())
    }

    /// Returns an angle in [-180, 180] which represents the angle between the given vector and
    /// the vector (1,0) in the y-flipped 2D cartesian coordinate system.
    /// NOTE: This function does not need for v and u to be normalized
    #[inline]
    pub fn to_angle_flipped_y(self) -> f32 {
        -Vec2::signed_angle_between(self, Vec2::unit_x())
    }

    /// Returns an angle in [0, 180] with the following properties:
    /// If u and v are parallel and pointing into the same direction it returns zero
    /// If u and v are parallel and pointing into different directions it returns PI
    /// NOTE: This function does not need for v and u to be normalized
    #[inline]
    pub fn abs_angle_between(v: Vec2, u: Vec2) -> f32 {
        f32::abs(Vec2::signed_angle_between(v, u))
    }

    /// Same as v2_abs_angle_between but faster and needs normalized v and u to work
    #[inline]
    pub fn abs_angle_between_fast(v_normalized: Vec2, u_normalized: Vec2) -> f32 {
        RADIANS_TO_DEGREE * f32::acos(Vec2::dot(v_normalized, u_normalized))
    }

    /// For a given vector this returns a rotated vector by given angle
    #[must_use]
    #[inline]
    pub fn rotated(self, angle_deg: f32) -> Vec2 {
        let angle_rad = DEGREE_TO_RADIANS * angle_deg;

        let cos_angle = f32::cos(angle_rad);
        let sin_angle = f32::sin(angle_rad);
        Vec2::new(
            self.x * cos_angle - self.y * sin_angle,
            self.x * sin_angle + self.y * cos_angle,
        )
    }

    /// For a given vector this returns a rotated vector by given angle in a y-flipped coordinate system
    #[must_use]
    #[inline]
    pub fn rotated_flipped_y(self, angle_deg: f32) -> Vec2 {
        let angle_rad = DEGREE_TO_RADIANS * angle_deg;

        let cos_angle = f32::cos(-angle_rad);
        let sin_angle = f32::sin(-angle_rad);
        Vec2::new(
            self.x * cos_angle - self.y * sin_angle,
            self.x * sin_angle + self.y * cos_angle,
        )
    }

    #[must_use]
    #[inline]
    pub fn mirrored_horizontally(self, pivot_x: f32) -> Vec2 {
        Vec2::new(2.0 * pivot_x - self.x, self.y)
    }

    #[must_use]
    #[inline]
    pub fn mirrored_vertically(self, pivot_y: f32) -> Vec2 {
        Vec2::new(self.x, 2.0 * pivot_y - self.y)
    }

    #[must_use]
    #[inline]
    pub fn mirrored_in_origin(self) -> Vec2 {
        -self
    }

    #[must_use]
    #[inline]
    pub fn transformed(self, pivot: Vec2, xform: Transform) -> Vec2 {
        let offsetted_scaled = xform.scale * (self - pivot);
        let rotation_dir = xform.rotation_dir();

        // NOTE:
        // This describes a matrix multiplication with the rotation matrix
        // | cos(a) -sin(a) |  = | rotation_dir.x  -rotation_dir.y |
        // | sin(a)  cos(a) |    | rotation_dir.y   rotation_dir.x |
        //                     = | rotation_dir     v2_perpendicular(rotation_dir) |
        //
        let rotated_scaled = Vec2::new(
            offsetted_scaled.x * rotation_dir.x - offsetted_scaled.y * rotation_dir.y,
            offsetted_scaled.x * rotation_dir.y + offsetted_scaled.y * rotation_dir.x,
        );

        rotated_scaled + xform.pos
    }

    #[inline]
    pub fn multi_transform(coordinates: &mut [Vec2], pivot: Vec2, xform: Transform) {
        for point in coordinates {
            *point = point.transformed(pivot, xform);
        }
    }

    #[must_use]
    #[inline]
    pub fn multi_transformed<CoordType>(
        coordinates: &[CoordType],
        pivot: Vec2,
        xform: Transform,
    ) -> Vec<Vec2>
    where
        CoordType: Into<Vec2> + Copy + Clone,
    {
        coordinates
            .iter()
            .map(|&point| Vec2::from(point.into()).transformed(pivot, xform))
            .collect()
    }

    /// Spherical interpolation
    #[inline]
    pub fn slerp(start: Vec2, end: Vec2, percent: f32) -> Vec2 {
        let angle_between = Vec2::signed_angle_between(start, end);
        start.rotated(percent * angle_between)
    }

    /// Normalized linear interpolation (good approximation to expensive slerp)
    #[inline]
    pub fn nlerp(start: Vec2, end: Vec2, percent: f32) -> Vec2 {
        Vec2::lerp(start, end, percent).normalized()
    }

    /// Returns the given vector clamped to a given maximum length
    #[must_use]
    #[inline]
    pub fn clamped_abs(self, max_length: f32) -> Vec2 {
        assert!(max_length > EPSILON);
        let length = self.magnitude();
        if length <= max_length {
            return self;
        }
        let clamped = f32::min(length, max_length);
        self * (clamped / length)
    }

    /// Convert square-shaped input into disk-shaped output
    /// See docs/mapping-square-to-circle.html for derivation
    /// (Originally from http://mathproofs.blogspot.de/2005/07/mapping-square-to-circle.html)
    ///
    /// NOTE: This assumes v in [-1, 1]x[-1, 1]
    #[must_use]
    #[inline]
    pub fn square_to_disk_transform(self) -> Vec2 {
        Vec2::new(
            self.x * f32::sqrt(1.0 - 0.5 * self.y * self.y),
            self.y * f32::sqrt(1.0 - 0.5 * self.x * self.x),
        )
    }
}

//--------------------------------------------------------------------------------------------------
// Operators

use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;

impl Neg for Vec2 {
    type Output = Vec2;
    #[inline]
    fn neg(self) -> Vec2 {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Add<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl AddAssign<Vec2> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Add<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: f32) -> Vec2 {
        Vec2 {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}
impl Add<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self + rhs.x,
            y: self + rhs.y,
        }
    }
}
impl AddAssign<f32> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Vec2 {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}
impl Add<i32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: i32) -> Vec2 {
        Vec2 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
        }
    }
}
impl Add<Vec2> for i32 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self as f32 + rhs.x,
            y: self as f32 + rhs.y,
        }
    }
}
impl AddAssign<i32> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        *self = Vec2 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
        }
    }
}

impl Sub<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl SubAssign<Vec2> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl Sub<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: f32) -> Vec2 {
        Vec2 {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}
impl SubAssign<f32> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Vec2 {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}
impl Sub<i32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: i32) -> Vec2 {
        Vec2 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
        }
    }
}
impl SubAssign<i32> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        *self = Vec2 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
        }
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
impl MulAssign<Vec2> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}
impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
impl Mul<i32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: i32) -> Vec2 {
        Vec2 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
        }
    }
}
impl Mul<Vec2> for i32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self as f32 * rhs.x,
            y: self as f32 * rhs.y,
        }
    }
}
impl MulAssign<i32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = Vec2 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
        }
    }
}

impl Div<Vec2> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}
impl DivAssign<Vec2> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Vec2) {
        *self = Vec2 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}
impl Div<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: f32) -> Vec2 {
        Vec2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}
impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Vec2 {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}
impl Div<i32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: i32) -> Vec2 {
        Vec2 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
        }
    }
}
impl DivAssign<i32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = Vec2 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// 2D Vector
