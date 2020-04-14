////////////////////////////////////////////////////////////////////////////////////////////////////
// Integer Rect

use super::*;

use serde_derive::{Deserialize, Serialize};

/// Origin -> top-left
/// Grows positive towards bottom-right
///
/// Example:
/// Recti rect;
/// rect.pos = (0,0);
/// rect.dim = (4,3);
///
/// The above rectangle begins in (0,0) and stretches 4 units left and 3 units down:
/// p in rect <=> p in {0, 1, 2, 3} x {0, 1, 2}
///
///   0   1   2   3   4   5
///   |   |   |   |   |   |
/// 0-+---+---+---+---+---+---
///   |0,0|1,0|2,0|3,0|   |
/// 1-+---+---+---+-------+---
///   |0,1|1,1|2,1|3,1|   |
/// 2-+---+---+---+---+---+---
///   |0,2|1,2|2,2|3,2|   |
/// 3-+---+---+---+---+---+---
///   |   |   |   |   |   |
/// 4-+---+---+---+---+---+---
///   |   |   |   |   |   |
///
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recti {
    pub pos: Vec2i,
    pub dim: Vec2i,
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Recti {
    #[inline]
    pub fn zero() -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim: Vec2i::zero(),
        }
    }

    #[inline]
    pub fn from_xy_width_height(x: i32, y: i32, width: i32, height: i32) -> Recti {
        Recti {
            pos: Vec2i::new(x, y),
            dim: Vec2i::new(width, height),
        }
    }

    #[inline]
    pub fn from_xy_dimensions(x: i32, y: i32, dim: Vec2i) -> Recti {
        Recti {
            pos: Vec2i::new(x, y),
            dim,
        }
    }

    #[inline]
    pub fn from_width_height(width: i32, height: i32) -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim: Vec2i::new(width, height),
        }
    }

    #[inline]
    pub fn from_square(size: i32) -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim: Vec2i::new(size, size),
        }
    }

    #[inline]
    pub fn from_pos_width_height(pos: Pointi, width: i32, height: i32) -> Recti {
        Recti {
            pos,
            dim: Vec2i::new(width, height),
        }
    }

    #[inline]
    pub fn from_dimensions(dim: Vec2i) -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim,
        }
    }

    #[inline]
    pub fn from_point_dimensions(pos: Pointi, dim: Vec2i) -> Recti {
        Recti { pos, dim }
    }

    #[inline]
    pub fn from_lefttop_rightbottom(left_top: Vec2i, right_bottom: Vec2i) -> Recti {
        assert!(left_top.x <= right_bottom.x);
        assert!(left_top.y <= right_bottom.y);

        Recti {
            pos: left_top,
            dim: right_bottom - left_top,
        }
    }

    #[inline]
    pub fn from_bounds_left_top_right_bottom(
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Recti {
        assert!(left <= right);
        assert!(top <= bottom);
        Recti::from_lefttop_rightbottom(Vec2i::new(left, top), Vec2i::new(right, bottom))
    }

    #[inline]
    pub fn unit() -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim: Vec2i::ones(),
        }
    }

    #[inline]
    pub fn smallest_that_contains_both_rects(a: Recti, b: Recti) -> Recti {
        let left = i32::min(a.left(), b.left());
        let top = i32::min(a.top(), b.top());
        let right = i32::max(a.right(), b.right());
        let bottom = i32::max(a.bottom(), b.bottom());

        let left_top = Vec2i::new(left, top);
        let right_bottom = Vec2i::new(right, bottom);

        Recti::from_lefttop_rightbottom(left_top, right_bottom)
    }

    #[inline]
    pub fn from_rect_floored(rect: Rect) -> Recti {
        let pos = rect.pos.floori();
        let dim = rect.dim.floori();
        Recti::from_point_dimensions(pos, dim)
    }

    #[inline]
    pub fn from_rect_ceiled(rect: Rect) -> Recti {
        let pos = rect.pos.ceili();
        let dim = rect.dim.ceili();
        Recti::from_point_dimensions(pos, dim)
    }

    #[inline]
    pub fn from_rect_rounded(rect: Rect) -> Recti {
        let pos = rect.pos.roundi();
        let dim = rect.dim.roundi();
        Recti::from_point_dimensions(pos, dim)
    }
}

//--------------------------------------------------------------------------------------------------
// Accessors

impl Recti {
    #[inline(always)]
    pub fn center(self) -> Pointi {
        self.pos + self.dim / 2
    }

    #[inline(always)]
    pub const fn width(self) -> i32 {
        self.dim.x
    }

    #[inline(always)]
    pub const fn height(self) -> i32 {
        self.dim.y
    }

    #[inline(always)]
    pub const fn left(self) -> i32 {
        self.pos.x
    }

    #[inline(always)]
    pub const fn top(self) -> i32 {
        self.pos.y
    }

    #[inline(always)]
    pub const fn right(self) -> i32 {
        self.pos.x + self.dim.x
    }

    #[inline(always)]
    pub const fn bottom(self) -> i32 {
        self.pos.y + self.dim.y
    }
}

//--------------------------------------------------------------------------------------------------
// Modify geomerty

impl Recti {
    #[must_use]
    #[inline]
    pub fn with_new_width(self, new_width: i32, alignment: AlignmentHorizontal) -> Recti {
        let mut result = self;
        result.dim.x = new_width;
        match alignment {
            AlignmentHorizontal::Left => result,
            AlignmentHorizontal::Center => {
                let new_left = block_centered_in_point(new_width, self.center().x);
                result.pos.x = new_left;
                result
            }
            AlignmentHorizontal::Right => {
                let new_left = self.right() - new_width;
                result.pos.x = new_left;
                result
            }
        }
    }

    #[must_use]
    #[inline]
    pub fn with_new_height(self, new_height: i32, alignment: AlignmentVertical) -> Recti {
        let mut result = self;
        result.dim.y = new_height;
        match alignment {
            AlignmentVertical::Top => result,
            AlignmentVertical::Center => {
                let new_top = block_centered_in_point(new_height, self.center().y);
                result.pos.y = new_top;
                result
            }
            AlignmentVertical::Bottom => {
                let new_top = self.bottom() - new_height;
                result.pos.y = new_top;
                result
            }
        }
    }

    #[must_use]
    #[inline]
    pub fn translated_by(self, translation: Vec2i) -> Recti {
        Recti {
            pos: self.pos + translation,
            dim: self.dim,
        }
    }

    #[must_use]
    #[inline]
    pub fn translated_to_origin(self) -> Recti {
        Recti {
            pos: Vec2i::zero(),
            dim: self.dim,
        }
    }

    #[must_use]
    #[inline]
    pub fn translated_to_pos(self, pos: Pointi) -> Recti {
        Recti { pos, dim: self.dim }
    }

    #[must_use]
    #[inline]
    pub fn centered(self) -> Recti {
        let half_dim = self.dim / 2;
        self.translated_by(-half_dim)
    }

    #[must_use]
    #[inline]
    pub fn centered_in_origin(self) -> Recti {
        Recti::centered(self.translated_to_origin())
    }

    #[must_use]
    #[inline]
    pub fn centered_in_position(self, pos: Pointi) -> Recti {
        Recti::centered(self.translated_to_pos(pos))
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_origin(self, scale: Vec2i) -> Recti {
        debug_assert!(scale.x >= 0);
        debug_assert!(scale.y >= 0);

        let left = self.left() * scale.x;
        let top = self.top() * scale.y;
        let right = self.right() * scale.x;
        let bottom = self.bottom() * scale.y;
        Recti::from_lefttop_rightbottom(Vec2i::new(left, top), Vec2i::new(right, bottom))
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_left_top(self, scale: Vec2i) -> Recti {
        debug_assert!(scale.x > 0);
        debug_assert!(scale.y > 0);

        Recti::from_point_dimensions(self.pos, self.dim * scale)
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_center(self, scale: Vec2i) -> Recti {
        let previous_center = self.center();
        let origin_rect = self.centered_in_origin();
        let scaled_rect = origin_rect.scaled_from_origin(scale);
        scaled_rect.centered_in_position(previous_center)
    }

    #[must_use]
    #[inline]
    pub fn extended_uniformly_by(self, extension: i32) -> Recti {
        let left = self.left() - extension;
        let top = self.top() - extension;
        let right = self.right() + extension;
        let bottom = self.bottom() + extension;
        Recti::from_lefttop_rightbottom(Vec2i::new(left, top), Vec2i::new(right, bottom))
    }

    /// Returns a version of the rectangle that is centered in a given rect
    #[must_use]
    #[inline]
    pub fn centered_in_rect(self, target: Recti) -> Recti {
        self.centered_in_position(target.center())
    }

    /// Returns a version of the rectangle that is centered horizontally in a given rect, leaving
    /// the original vertical position intact
    #[must_use]
    #[inline]
    pub fn centered_in_rect_horizontally(self, target: Recti) -> Recti {
        let pos_x = block_centered_in_point(self.dim.x, target.center().x);
        Recti::from_xy_dimensions(pos_x, self.pos.y, self.dim)
    }

    /// Returns a version of the rectangle that is centered vertically in a given rect, leaving
    /// the original horizontal position intact
    #[must_use]
    #[inline]
    pub fn centered_in_rect_vertically(self, target: Recti) -> Recti {
        let pos_y = block_centered_in_point(self.dim.y, target.center().y);
        Recti::from_xy_dimensions(self.pos.x, pos_y, self.dim)
    }
}
