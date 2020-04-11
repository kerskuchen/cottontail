////////////////////////////////////////////////////////////////////////////////////////////////////
// 3D Vector

use super::*;

use serde_derive::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

//--------------------------------------------------------------------------------------------------
// Conversion

impl Vec3 {
    #[inline]
    #[must_use]
    pub fn dropped_z(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Vec3 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    #[inline]
    pub fn zero() -> Vec3 {
        Vec3::new(0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn ones() -> Vec3 {
        Vec3::new(1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn unit_x() -> Vec3 {
        Vec3::new(1.0, 0.0, 0.0)
    }

    #[inline]
    pub fn unit_y() -> Vec3 {
        Vec3::new(0.0, 1.0, 0.0)
    }

    #[inline]
    pub fn unit_z() -> Vec3 {
        Vec3::new(0.0, 0.0, 1.0)
    }

    #[inline]
    pub fn filled_x(x: f32) -> Vec3 {
        Vec3::new(x, 0.0, 0.0)
    }

    #[inline]
    pub fn filled_y(y: f32) -> Vec3 {
        Vec3::new(0.0, y, 0.0)
    }

    #[inline]
    pub fn filled_z(z: f32) -> Vec3 {
        Vec3::new(0.0, 0.0, z)
    }

    #[inline]
    pub fn filled(fill: f32) -> Vec3 {
        Vec3::new(fill, fill, fill)
    }

    #[inline]
    pub fn from_vec2(v: Vec2, z: f32) -> Vec3 {
        Vec3::new(v.x, v.y, z)
    }
}

//--------------------------------------------------------------------------------------------------
// Functions

impl Vec3 {
    #[inline]
    pub fn dot(v: Vec3, u: Vec3) -> f32 {
        v.x * u.x + v.y * u.y + v.z * u.z
    }

    #[must_use]
    #[inline]
    pub fn normalized(self) -> Vec3 {
        self / self.magnitude()
    }

    #[inline]
    pub fn is_normalized(self) -> bool {
        let normalized = Vec3::normalized(self);
        (normalized - self).is_effectively_zero()
    }

    #[inline]
    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn magnitude_squared(self) -> f32 {
        Vec3::dot(self, self)
    }

    #[inline]
    pub fn distance(v: Vec3, u: Vec3) -> f32 {
        (v - u).magnitude()
    }

    #[inline]
    pub fn distance_squared(v: Vec3, u: Vec3) -> f32 {
        (v - u).magnitude_squared()
    }

    #[inline]
    pub fn manhattan_distance(a: Vec3, b: Vec3) -> f32 {
        f32::abs(a.x - b.x) + f32::abs(a.y - b.y) + f32::abs(a.z - b.z)
    }

    #[inline]
    /// Liner interpolation
    pub fn lerp(start: Vec3, end: Vec3, percent: f32) -> Vec3 {
        Vec3::new(
            lerp(start.x, end.x, percent),
            lerp(start.y, end.y, percent),
            lerp(start.z, end.z, percent),
        )
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        Vec3::dot(self, self) == 0.0
    }

    #[inline]
    pub fn is_effectively_zero(self) -> bool {
        Vec3::dot(self, self) < EPSILON
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

impl Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl AddAssign<Vec3> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Vec3) {
        *self = Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl Add<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }
}
impl Add<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self + rhs.x,
            y: self + rhs.y,
            z: self + rhs.z,
        }
    }
}
impl AddAssign<f32> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Vec3 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }
}
impl Add<i32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: i32) -> Vec3 {
        Vec3 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
            z: self.z + rhs as f32,
        }
    }
}
impl Add<Vec3> for i32 {
    type Output = Vec3;
    #[inline]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self as f32 + rhs.x,
            y: self as f32 + rhs.y,
            z: self as f32 + rhs.z,
        }
    }
}
impl AddAssign<i32> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        *self = Vec3 {
            x: self.x + rhs as f32,
            y: self.y + rhs as f32,
            z: self.z + rhs as f32,
        }
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}
impl SubAssign<Vec3> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}
impl Sub<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }
}
impl SubAssign<f32> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Vec3 {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }
}
impl Sub<i32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, rhs: i32) -> Vec3 {
        Vec3 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
            z: self.z - rhs as f32,
        }
    }
}
impl SubAssign<i32> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        *self = Vec3 {
            x: self.x - rhs as f32,
            y: self.y - rhs as f32,
            z: self.z - rhs as f32,
        }
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}
impl MulAssign<Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec3) {
        *self = Vec3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self * rhs.x,
            y: self * rhs.y,
            z: self * rhs.z,
        }
    }
}
impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}
impl Mul<i32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: i32) -> Vec3 {
        Vec3 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
            z: self.z * rhs as f32,
        }
    }
}
impl Mul<Vec3> for i32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self as f32 * rhs.x,
            y: self as f32 * rhs.y,
            z: self as f32 * rhs.z,
        }
    }
}
impl MulAssign<i32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = Vec3 {
            x: self.x * rhs as f32,
            y: self.y * rhs as f32,
            z: self.z * rhs as f32,
        }
    }
}

impl Div<Vec3> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}
impl DivAssign<Vec3> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: Vec3) {
        *self = Vec3 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}
impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}
impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}
impl Div<i32> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, rhs: i32) -> Vec3 {
        Vec3 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
            z: self.z / rhs as f32,
        }
    }
}
impl DivAssign<i32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = Vec3 {
            x: self.x / rhs as f32,
            y: self.y / rhs as f32,
            z: self.z / rhs as f32,
        }
    }
}
