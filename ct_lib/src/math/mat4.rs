////////////////////////////////////////////////////////////////////////////////////////////////////
// 4x4 Matrix

use super::*;

use serde_derive::{Deserialize, Serialize};

/// Matrix is indexed column major in compliance with OpenGL shaders
/// (like a transposed matrix in mathematical literature)
/// |a00 a10 a20 a30|
/// |a01 a11 a21 a31|
/// |a02 a12 a22 a32|
/// |a03 a13 a23 a33|
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mat4 {
    pub cols: [Vec4; 4],
}

//--------------------------------------------------------------------------------------------------
// Conversion

impl Mat4 {
    #[inline]
    pub fn into_column_array(self) -> [f32; 16] {
        [
            self.cols[0].x,
            self.cols[0].y,
            self.cols[0].z,
            self.cols[0].w,
            self.cols[1].x,
            self.cols[1].y,
            self.cols[1].z,
            self.cols[1].w,
            self.cols[2].x,
            self.cols[2].y,
            self.cols[2].z,
            self.cols[2].w,
            self.cols[3].x,
            self.cols[3].y,
            self.cols[3].z,
            self.cols[3].w,
        ]
    }
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Mat4 {
    #[inline]
    pub fn identity() -> Mat4 {
        Mat4 {
            cols: [
                Vec4::unit_x(),
                Vec4::unit_y(),
                Vec4::unit_z(),
                Vec4::unit_w(),
            ],
        }
    }

    #[inline]
    pub fn translation(x: f32, y: f32, z: f32) -> Mat4 {
        let mut result = Mat4::identity();
        result.cols[3].x = x;
        result.cols[3].y = y;
        result.cols[3].z = z;
        result
    }

    #[inline]
    pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        let mut result = Mat4::identity();
        result.cols[0].x = x;
        result.cols[1].y = y;
        result.cols[2].z = z;
        result
    }

    #[inline]
    pub fn rotation_around_z(angle_rad: f32) -> Mat4 {
        let cos_angle = f32::cos(angle_rad);
        let sin_angle = f32::sin(angle_rad);

        let mut result = Mat4::identity();
        result.cols[0].x = cos_angle;
        result.cols[1].y = cos_angle;
        result.cols[1].x = -sin_angle;
        result.cols[0].y = sin_angle;
        result
    }

    #[inline]
    pub fn rotation_around_z_flipped_y(angle_rad: f32) -> Mat4 {
        let cos_angle = f32::cos(-angle_rad);
        let sin_angle = f32::sin(-angle_rad);

        let mut result = Mat4::identity();
        result.cols[0].x = cos_angle;
        result.cols[1].y = cos_angle;
        result.cols[1].x = -sin_angle;
        result.cols[0].y = sin_angle;
        result
    }

    /// See http://www.songho.ca/opengl/gl_projectionmatrix.html for derivation
    ///
    #[inline]
    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
        let mut result = Mat4::default();

        result.cols[0].x = 2.0 / (right - left);
        result.cols[1].y = 2.0 / (top - bottom);
        result.cols[2].z = -2.0 / (far - near);
        result.cols[3].x = -(right + left) / (right - left);
        result.cols[3].y = -(top + bottom) / (top - bottom);
        result.cols[3].z = -(far + near) / (far - near);
        result.cols[3].w = 1.0;
        result
    }

    #[inline]
    pub fn ortho_origin_center(width: f32, height: f32, near: f32, far: f32) -> Mat4 {
        Mat4::ortho(
            -0.5 * width,
            0.5 * width,
            -0.5 * height,
            0.5 * height,
            near,
            far,
        )
    }

    #[inline]
    pub fn ortho_origin_center_flipped_y(width: f32, height: f32, near: f32, far: f32) -> Mat4 {
        Mat4::ortho(
            -0.5 * width,
            0.5 * width,
            0.5 * height,
            -0.5 * height,
            near,
            far,
        )
    }

    #[inline]
    pub fn ortho_origin_left_top(width: f32, height: f32, near: f32, far: f32) -> Mat4 {
        Mat4::ortho(0.0, width, height, 0.0, near, far)
    }

    #[inline]
    pub fn ortho_origin_left_bottom(width: f32, height: f32, near: f32, far: f32) -> Mat4 {
        Mat4::ortho(0.0, width, 0.0, height, near, far)
    }
}

//--------------------------------------------------------------------------------------------------
// Functions

impl Mat4 {
    #[must_use]
    #[inline]
    pub fn transposed(self) -> Mat4 {
        Mat4 {
            cols: [
                Vec4::new(
                    self.cols[0].x,
                    self.cols[1].x,
                    self.cols[2].x,
                    self.cols[3].x,
                ),
                Vec4::new(
                    self.cols[0].y,
                    self.cols[1].y,
                    self.cols[2].y,
                    self.cols[3].y,
                ),
                Vec4::new(
                    self.cols[0].z,
                    self.cols[1].z,
                    self.cols[2].z,
                    self.cols[3].z,
                ),
                Vec4::new(
                    self.cols[0].w,
                    self.cols[1].w,
                    self.cols[2].w,
                    self.cols[3].w,
                ),
            ],
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Operators

use std::ops::Mul;
use std::ops::MulAssign;

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4::new(
            self.cols[0].x * rhs.x
                + self.cols[1].x * rhs.y
                + self.cols[2].x * rhs.z
                + self.cols[3].x * rhs.w,
            self.cols[0].y * rhs.x
                + self.cols[1].y * rhs.y
                + self.cols[2].y * rhs.z
                + self.cols[3].y * rhs.w,
            self.cols[0].z * rhs.x
                + self.cols[1].z * rhs.y
                + self.cols[2].z * rhs.z
                + self.cols[3].z * rhs.w,
            self.cols[0].w * rhs.x
                + self.cols[1].w * rhs.y
                + self.cols[2].w * rhs.z
                + self.cols[3].w * rhs.w,
        )
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4;
    #[must_use]
    #[inline]
    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4 {
            cols: [
                self * rhs.cols[0],
                self * rhs.cols[1],
                self * rhs.cols[2],
                self * rhs.cols[3],
            ],
        }
    }
}
impl MulAssign<Mat4> for Mat4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Mat4) {
        *self = *self * rhs;
    }
}

impl Mul<f32> for Mat4 {
    type Output = Mat4;
    #[inline]
    fn mul(self, rhs: f32) -> Mat4 {
        Mat4 {
            cols: [
                self.cols[0] * rhs,
                self.cols[1] * rhs,
                self.cols[2] * rhs,
                self.cols[3] * rhs,
            ],
        }
    }
}
impl Mul<Mat4> for f32 {
    type Output = Mat4;
    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4 {
            cols: [
                self * rhs.cols[0],
                self * rhs.cols[1],
                self * rhs.cols[2],
                self * rhs.cols[3],
            ],
        }
    }
}
impl MulAssign<f32> for Mat4 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}
