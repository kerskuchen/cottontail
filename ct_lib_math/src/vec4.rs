////////////////////////////////////////////////////////////////////////////////////////////////////
// 4D Vector

use super::*;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Vec4 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
        Vec4 { x, y, z, w }
    }

    #[inline]
    pub fn zero() -> Vec4 {
        Vec4::new(0.0, 0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn ones() -> Vec4 {
        Vec4::new(1.0, 1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn unit_x() -> Vec4 {
        Vec4::new(1.0, 0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn unit_y() -> Vec4 {
        Vec4::new(0.0, 1.0, 0.0, 0.0)
    }

    #[inline]
    pub fn unit_z() -> Vec4 {
        Vec4::new(0.0, 0.0, 1.0, 0.0)
    }

    #[inline]
    pub fn unit_w() -> Vec4 {
        Vec4::new(0.0, 0.0, 0.0, 1.0)
    }

    #[inline]
    pub fn filled_x(x: f32) -> Vec4 {
        Vec4::new(x, 0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn filled_y(y: f32) -> Vec4 {
        Vec4::new(0.0, y, 0.0, 0.0)
    }

    #[inline]
    pub fn filled_z(z: f32) -> Vec4 {
        Vec4::new(0.0, 0.0, z, 0.0)
    }

    #[inline]
    pub fn filled_w(w: f32) -> Vec4 {
        Vec4::new(0.0, 0.0, 0.0, w)
    }

    #[inline]
    pub fn filled(fill: f32) -> Vec4 {
        Vec4::new(fill, fill, fill, fill)
    }

    #[inline]
    pub fn from_vec2(v: Vec2, z: f32, w: f32) -> Vec4 {
        Vec4::new(v.x, v.y, z, w)
    }

    #[inline]
    pub fn from_vec3(v: Vec3, w: f32) -> Vec4 {
        Vec4::new(v.x, v.y, v.z, w)
    }
}

//--------------------------------------------------------------------------------------------------
// Functions

impl Vec4 {
    #[inline]
    pub fn dot(v: Vec4, u: Vec4) -> f32 {
        v.x * u.x + v.y * u.y + v.z * u.z + v.w * u.w
    }

    #[must_use]
    #[inline]
    pub fn normalized(self) -> Vec4 {
        self / self.magnitude()
    }

    #[inline]
    pub fn is_normalized(self) -> bool {
        let normalized = Vec4::normalized(self);
        (normalized - self).is_effectively_zero()
    }

    #[inline]
    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn magnitude_squared(self) -> f32 {
        Vec4::dot(self, self)
    }

    #[inline]
    pub fn distance(v: Vec4, u: Vec4) -> f32 {
        (v - u).magnitude()
    }

    #[inline]
    pub fn distance_squared(v: Vec4, u: Vec4) -> f32 {
        (v - u).magnitude_squared()
    }

    #[inline]
    pub fn manhattan_distance(a: Vec4, b: Vec4) -> f32 {
        f32::abs(a.x - b.x) + f32::abs(a.y - b.y) + f32::abs(a.z - b.z) + f32::abs(a.w - b.w)
    }

    #[inline]
    /// Liner interpolation
    pub fn lerp(start: Vec4, end: Vec4, percent: f32) -> Vec4 {
        Vec4::new(
            lerp(start.x, end.x, percent),
            lerp(start.y, end.y, percent),
            lerp(start.z, end.z, percent),
            lerp(start.w, end.w, percent),
        )
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        Vec4::dot(self, self) == 0.0
    }

    #[inline]
    pub fn is_effectively_zero(self) -> bool {
        Vec4::dot(self, self) < EPSILON
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

impl Neg for Vec4 {
    type Output = Vec4;
    #[inline]
    fn neg(self) -> Vec4 {
        Vec4 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl Add<Vec4> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn add(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}
impl AddAssign<Vec4> for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: Vec4) {
        *self = Vec4 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            w: self.w + rhs.w,
        }
    }
}
impl Add<f32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn add(self, rhs: f32) -> Vec4 {
        Vec4 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
            w: self.w + rhs,
        }
    }
}
impl Add<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn add(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self + rhs.x,
            y: self + rhs.y,
            z: self + rhs.z,
            w: self + rhs.w,
        }
    }
}
impl AddAssign<f32> for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Vec4 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
            w: self.w + rhs,
        }
    }
}
impl Add<i32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn add(self, rhs: i32) -> Vec4 {
        Vec4 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
            z: self.z + rhs as f32,
            w: self.w + rhs as f32,
        }
    }
}
impl Add<Vec4> for i32 {
    type Output = Vec4;
    #[inline]
    fn add(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self as f32 + rhs.x,
            y: self as f32 + rhs.y,
            z: self as f32 + rhs.z,
            w: self as f32 + rhs.w,
        }
    }
}
impl AddAssign<i32> for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        *self = Vec4 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
            z: self.z + rhs as f32,
            w: self.w + rhs as f32,
        }
    }
}

impl Sub<Vec4> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn sub(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }
}
impl SubAssign<Vec4> for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec4) {
        *self = Vec4 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            w: self.w - rhs.w,
        }
    }
}
impl Sub<f32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn sub(self, rhs: f32) -> Vec4 {
        Vec4 {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
            w: self.w - rhs,
        }
    }
}
impl SubAssign<f32> for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Vec4 {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
            w: self.w - rhs,
        }
    }
}
impl Sub<i32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn sub(self, rhs: i32) -> Vec4 {
        Vec4 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
            z: self.z - rhs as f32,
            w: self.w - rhs as f32,
        }
    }
}
impl SubAssign<i32> for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        *self = Vec4 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
            z: self.z - rhs as f32,
            w: self.w - rhs as f32,
        }
    }
}

impl Mul<Vec4> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w,
        }
    }
}
impl MulAssign<Vec4> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec4) {
        *self = Vec4 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            w: self.w * rhs.w,
        }
    }
}
impl Mul<f32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: f32) -> Vec4 {
        Vec4 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
        }
    }
}
impl Mul<Vec4> for f32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
            w: self * rhs.w,
        }
    }
}
impl MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Vec4 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            w: self.w * rhs,
        }
    }
}
impl Mul<i32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: i32) -> Vec4 {
        Vec4 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
            z: self.z * rhs as f32,
            w: self.w * rhs as f32,
        }
    }
}
impl Mul<Vec4> for i32 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self as f32 * rhs.x,
            y: self as f32 * rhs.y,
            z: self as f32 * rhs.z,
            w: self as f32 * rhs.w,
        }
    }
}
impl MulAssign<i32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = Vec4 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
            z: self.z * rhs as f32,
            w: self.w * rhs as f32,
        }
    }
}

impl Div<Vec4> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn div(self, rhs: Vec4) -> Vec4 {
        Vec4 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
        }
    }
}
impl DivAssign<Vec4> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: Vec4) {
        *self = Vec4 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
            w: self.w / rhs.w,
        }
    }
}
impl Div<f32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn div(self, rhs: f32) -> Vec4 {
        Vec4 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
            w: self.w / rhs,
        }
    }
}
impl DivAssign<f32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Vec4 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
            w: self.w / rhs,
        }
    }
}
impl Div<i32> for Vec4 {
    type Output = Vec4;
    #[inline]
    fn div(self, rhs: i32) -> Vec4 {
        Vec4 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
            z: self.z / rhs as f32,
            w: self.w / rhs as f32,
        }
    }
}
impl DivAssign<i32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = Vec4 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
            z: self.z / rhs as f32,
            w: self.w / rhs as f32,
        }
    }
}
