////////////////////////////////////////////////////////////////////////////////////////////////////
// Integer 2D Vector

use super::*;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

//--------------------------------------------------------------------------------------------------
// Conversion

impl From<(i32, i32)> for Vec2i {
    #[inline]
    fn from(pair: (i32, i32)) -> Self {
        Vec2i {
            x: pair.0,
            y: pair.1,
        }
    }
}

impl Vec2i {
    #[inline]
    pub fn to_vec2(self) -> Vec2 {
        self.into()
    }
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Vec2i {
    #[inline]
    pub fn new(x: i32, y: i32) -> Vec2i {
        Vec2i { x, y }
    }

    #[inline]
    pub fn zero() -> Vec2i {
        Vec2i::new(0, 0)
    }

    #[inline]
    pub fn ones() -> Vec2i {
        Vec2i::new(1, 1)
    }

    #[inline]
    pub fn unit_x() -> Vec2i {
        Vec2i::new(1, 0)
    }

    #[inline]
    pub fn unit_y() -> Vec2i {
        Vec2i::new(0, 1)
    }

    #[inline]
    pub fn filled_x(x: i32) -> Vec2i {
        Vec2i::new(x, 0)
    }

    #[inline]
    pub fn filled_y(y: i32) -> Vec2i {
        Vec2i::new(0, y)
    }

    #[inline]
    pub fn filled(fill: i32) -> Vec2i {
        Vec2i::new(fill, fill)
    }
}

//--------------------------------------------------------------------------------------------------
// Creation Special

impl Vec2i {
    pub fn from_vec2_floored(v: Vec2) -> Vec2i {
        Vec2i::new(v.x as i32, v.y as i32)
    }

    pub fn from_vec2_ceiled(v: Vec2) -> Vec2i {
        Vec2i::new(ceili(v.x), ceili(v.y))
    }

    pub fn from_vec2_rounded(v: Vec2) -> Vec2i {
        Vec2i::new(roundi(v.x), roundi(v.y))
    }
}

//--------------------------------------------------------------------------------------------------
// Functions

impl Vec2i {
    #[inline]
    pub fn dot(v: Vec2i, u: Vec2i) -> i32 {
        v.x * u.x + v.y * u.y
    }

    #[inline]
    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn magnitude_squared(self) -> f32 {
        Vec2i::dot(self, self) as f32
    }

    #[inline]
    pub fn distance(v: Vec2i, u: Vec2i) -> f32 {
        (v - u).magnitude()
    }

    #[inline]
    pub fn distance_squared(v: Vec2i, u: Vec2i) -> f32 {
        (v - u).magnitude_squared()
    }

    #[inline]
    pub fn manhattan_distance(a: Vec2i, b: Vec2i) -> i32 {
        i32::abs(a.x - b.x) + i32::abs(a.y - b.y)
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        Vec2i::dot(self, self) == 0
    }
}

//--------------------------------------------------------------------------------------------------
// Functions Special

impl Vec2i {
    /// Returns the given vector clamped to a given rect
    #[must_use]
    pub fn clamped_to_recti(self, rect: Recti) -> Vec2i {
        Vec2i::new(
            clampi(self.x, rect.left(), rect.right()),
            clampi(self.y, rect.top(), rect.bottom()),
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

impl Neg for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn neg(self) -> Vec2i {
        Vec2i {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Add<Vec2i> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn add(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl AddAssign<Vec2i> for Vec2i {
    #[inline]
    fn add_assign(&mut self, rhs: Vec2i) {
        *self = Vec2i {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Add<i32> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn add(self, rhs: i32) -> Vec2i {
        Vec2i {
            x: self.x + rhs as i32,
            y: self.y + rhs as i32,
        }
    }
}
impl Add<Vec2i> for i32 {
    type Output = Vec2i;
    #[inline]
    fn add(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self as i32 + rhs.x,
            y: self as i32 + rhs.y,
        }
    }
}
impl AddAssign<i32> for Vec2i {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        *self = Vec2i {
            x: self.x + rhs as i32,
            y: self.y + rhs as i32,
        }
    }
}

impl Sub<Vec2i> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn sub(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl SubAssign<Vec2i> for Vec2i {
    #[inline]
    fn sub_assign(&mut self, rhs: Vec2i) {
        *self = Vec2i {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl Sub<i32> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn sub(self, rhs: i32) -> Vec2i {
        Vec2i {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}
impl SubAssign<i32> for Vec2i {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        *self = Vec2i {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl Mul<Vec2i> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn mul(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
impl MulAssign<Vec2i> for Vec2i {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec2i) {
        *self = Vec2i {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}
impl Mul<i32> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn mul(self, rhs: i32) -> Vec2i {
        Vec2i {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
impl Mul<Vec2i> for i32 {
    type Output = Vec2i;
    #[inline]
    fn mul(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self * rhs.x,
            y: self * rhs.y,
        }
    }
}
impl MulAssign<i32> for Vec2i {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        *self = Vec2i {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<Vec2i> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn div(self, rhs: Vec2i) -> Vec2i {
        Vec2i {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}
impl DivAssign<Vec2i> for Vec2i {
    #[inline]
    fn div_assign(&mut self, rhs: Vec2i) {
        *self = Vec2i {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}
impl Div<i32> for Vec2i {
    type Output = Vec2i;
    #[inline]
    fn div(self, rhs: i32) -> Vec2i {
        Vec2i {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}
impl DivAssign<i32> for Vec2i {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        *self = Vec2i {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// 2D Vector
