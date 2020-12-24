////////////////////////////////////////////////////////////////////////////////////////////////////
// Rect

use super::*;

use serde_derive::{Deserialize, Serialize};

/// Origin -> top-left
/// Grows positive towards bottom-right
///
/// Example:
/// Rect rect;
/// rect.pos = (0,0);
/// rect.dim = (4,3);
///
/// The above rectangle begins in (0,0) and stretches 4 units left and 3 units down.
/// For each point in the rect it holds:
/// p in rect <=> p in [0, 4[ x [0, 3[
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
pub struct Rect {
    pub pos: Vec2,
    pub dim: Vec2,
}

// ---------------------------------------------------------------------------------------------
// Conversion

impl From<Recti> for Rect {
    #[inline]
    fn from(rect: Recti) -> Rect {
        Rect {
            pos: Vec2::from(rect.pos),
            dim: Vec2::from(rect.dim),
        }
    }
}

impl Rect {
    /// Direct conversion, no round/floor/ceil happening
    #[inline]
    pub fn to_i32(self) -> Recti {
        Recti {
            pos: self.pos.to_i32(),
            dim: self.dim.to_i32(),
        }
    }

    #[inline]
    pub fn floor(self) -> Rect {
        Rect {
            pos: self.pos.floor(),
            dim: self.dim.floor(),
        }
    }

    #[inline]
    pub fn floori(self) -> Recti {
        Recti {
            pos: self.pos.floori(),
            dim: self.dim.floori(),
        }
    }

    #[inline]
    pub fn round(self) -> Rect {
        Rect {
            pos: self.pos.round(),
            dim: self.dim.round(),
        }
    }

    #[inline]
    pub fn roundi(self) -> Recti {
        Recti {
            pos: self.pos.roundi(),
            dim: self.dim.roundi(),
        }
    }

    #[inline]
    pub fn ceil(self) -> Rect {
        Rect {
            pos: self.pos.ceil(),
            dim: self.dim.ceil(),
        }
    }

    #[inline]
    pub fn ceili(self) -> Recti {
        Recti {
            pos: self.pos.ceili(),
            dim: self.dim.ceili(),
        }
    }
}

impl Rect {
    #[inline]
    pub fn points(self) -> [Point; 4] {
        [
            Point::new(self.left(), self.top()),
            Point::new(self.left(), self.bottom()),
            Point::new(self.right(), self.top()),
            Point::new(self.right(), self.bottom()),
        ]
    }

    #[inline]
    pub fn linestrip(self) -> [Point; 5] {
        [
            Point::new(self.left(), self.top()),
            Point::new(self.left(), self.bottom()),
            Point::new(self.right(), self.bottom()),
            Point::new(self.right(), self.top()),
            Point::new(self.left(), self.top()),
        ]
    }

    #[inline]
    pub fn left_segment(self) -> Line {
        Line::new(
            Point::new(self.left(), self.bottom()),
            Point::new(self.left(), self.top()),
        )
    }

    #[inline]
    pub fn right_segment(self) -> Line {
        Line::new(
            Point::new(self.right(), self.top()),
            Point::new(self.right(), self.bottom()),
        )
    }

    #[inline]
    pub fn top_segment(self) -> Line {
        Line::new(
            Point::new(self.left(), self.top()),
            Point::new(self.right(), self.top()),
        )
    }

    #[inline]
    pub fn bottom_segment(self) -> Line {
        Line::new(
            Point::new(self.left(), self.bottom()),
            Point::new(self.right(), self.bottom()),
        )
    }

    /// Returns line segments in the following order left, right, top, bottom
    #[inline]
    pub fn to_border_lines(self) -> [Line; 4] {
        [
            self.left_segment(),
            self.right_segment(),
            self.top_segment(),
            self.bottom_segment(),
        ]
    }
}

//--------------------------------------------------------------------------------------------------
// Creation

impl Rect {
    #[inline]
    pub fn zero() -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim: Vec2::zero(),
        }
    }

    #[inline]
    pub fn from_xy_width_height(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            pos: Vec2::new(x, y),
            dim: Vec2::new(width, height),
        }
    }

    #[inline]
    pub fn from_xy_dim(x: f32, y: f32, dim: Vec2) -> Rect {
        Rect {
            pos: Vec2::new(x, y),
            dim,
        }
    }

    #[inline]
    pub fn from_width_height(width: f32, height: f32) -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim: Vec2::new(width, height),
        }
    }

    #[inline]
    pub fn from_square(size: f32) -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim: Vec2::new(size, size),
        }
    }

    #[inline]
    pub fn from_pos_width_height(pos: Point, width: f32, height: f32) -> Rect {
        Rect {
            pos,
            dim: Vec2::new(width, height),
        }
    }

    #[inline]
    pub fn from_dim(dim: Vec2) -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim,
        }
    }

    #[inline]
    pub fn from_pos_dim(pos: Point, dim: Vec2) -> Rect {
        Rect { pos, dim }
    }

    #[inline]
    pub fn from_diagonal(left_top: Vec2, right_bottom: Vec2) -> Rect {
        assert!(left_top.x <= right_bottom.x);
        assert!(left_top.y <= right_bottom.y);

        Rect {
            pos: left_top,
            dim: right_bottom - left_top,
        }
    }

    #[inline]
    pub fn from_bounds_left_top_right_bottom(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        assert!(left <= right);
        assert!(top <= bottom);
        Rect::from_diagonal(Vec2::new(left, top), Vec2::new(right, bottom))
    }

    #[inline]
    pub fn unit() -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim: Vec2::ones(),
        }
    }

    #[inline]
    pub fn unit_centered() -> Rect {
        Rect {
            pos: -Vec2::ones() / 2.0,
            dim: Vec2::ones(),
        }
    }

    #[inline]
    pub fn smallest_that_contains_both_rects(a: Rect, b: Rect) -> Rect {
        let left = f32::min(a.left(), b.left());
        let top = f32::min(a.top(), b.top());
        let right = f32::max(a.right(), b.right());
        let bottom = f32::max(a.bottom(), b.bottom());

        let left_top = Vec2::new(left, top);
        let right_bottom = Vec2::new(right, bottom);

        Rect::from_diagonal(left_top, right_bottom)
    }
}

//--------------------------------------------------------------------------------------------------
// Accessors

impl Rect {
    #[inline(always)]
    pub fn center(self) -> Point {
        self.pos + self.dim / 2.0
    }

    #[inline(always)]
    pub fn width(self) -> f32 {
        self.dim.x
    }

    #[inline(always)]
    pub fn height(self) -> f32 {
        self.dim.y
    }

    #[inline(always)]
    pub fn left(self) -> f32 {
        self.pos.x
    }

    #[inline(always)]
    pub fn top(self) -> f32 {
        self.pos.y
    }

    #[inline(always)]
    pub fn right(self) -> f32 {
        self.pos.x + self.dim.x
    }

    #[inline(always)]
    pub fn bottom(self) -> f32 {
        self.pos.y + self.dim.y
    }
}

//--------------------------------------------------------------------------------------------------
// Modify geometry

impl Rect {
    #[must_use]
    #[inline]
    pub fn with_new_width(self, new_width: f32, alignment: AlignmentHorizontal) -> Rect {
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
    pub fn with_new_height(self, new_height: f32, alignment: AlignmentVertical) -> Rect {
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
    pub fn translated_by(self, translation: Vec2) -> Rect {
        Rect {
            pos: self.pos + translation,
            dim: self.dim,
        }
    }

    #[must_use]
    #[inline]
    pub fn translated_to_origin(self) -> Rect {
        Rect {
            pos: Vec2::zero(),
            dim: self.dim,
        }
    }

    #[must_use]
    #[inline]
    pub fn translated_to_pos(self, pos: Point) -> Rect {
        Rect { pos, dim: self.dim }
    }

    #[must_use]
    #[inline]
    pub fn mirrored_horizontally_on_axis(self, axis_x: f32) -> Rect {
        let axis_diff = self.pos.x - axis_x;
        let new_pos_x = self.pos.x - 2.0 * axis_diff - self.dim.x;
        Rect::from_xy_dim(new_pos_x, self.pos.y, self.dim)
    }

    #[must_use]
    #[inline]
    pub fn mirrored_vertically_on_axis(self, axis_y: f32) -> Rect {
        let axis_diff = self.pos.y - axis_y;
        let new_pos_y = self.pos.y - 2.0 * axis_diff - self.dim.y;
        Rect::from_xy_dim(self.pos.x, new_pos_y, self.dim)
    }

    #[must_use]
    #[inline]
    pub fn centered(self) -> Rect {
        let half_dim = self.dim / 2.0;
        self.translated_by(-half_dim)
    }

    #[must_use]
    #[inline]
    pub fn centered_in_origin(self) -> Rect {
        Rect::centered(self.translated_to_origin())
    }

    #[must_use]
    #[inline]
    pub fn centered_in_position(self, pos: Point) -> Rect {
        Rect::centered(self.translated_to_pos(pos))
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_origin(self, scale: Vec2) -> Rect {
        debug_assert!(scale.x >= 0.0);
        debug_assert!(scale.y >= 0.0);

        let left = self.left() * scale.x;
        let top = self.top() * scale.y;
        let right = self.right() * scale.x;
        let bottom = self.bottom() * scale.y;
        Rect::from_diagonal(Vec2::new(left, top), Vec2::new(right, bottom))
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_left_top(self, scale: Vec2) -> Rect {
        debug_assert!(scale.x > 0.0);
        debug_assert!(scale.y > 0.0);

        Rect::from_pos_dim(self.pos, self.dim * scale)
    }

    #[must_use]
    #[inline]
    pub fn scaled_from_center(self, scale: Vec2) -> Rect {
        let previous_center = self.center();
        let origin_rect = self.centered_in_origin();
        let scaled_rect = origin_rect.scaled_from_origin(scale);
        scaled_rect.centered_in_position(previous_center)
    }

    #[must_use]
    #[inline]
    pub fn extended_uniformly_by(self, extension: f32) -> Rect {
        let left = self.left() - extension;
        let top = self.top() - extension;
        let right = self.right() + extension;
        let bottom = self.bottom() + extension;
        Rect::from_diagonal(Vec2::new(left, top), Vec2::new(right, bottom))
    }

    /// Returns a version of the rectangle that is centered in a given rect
    #[must_use]
    #[inline]
    pub fn centered_in_rect(self, target: Rect) -> Rect {
        self.centered_in_position(target.center())
    }

    /// Returns a version of the rectangle that is centered horizontally in a given rect, leaving
    /// the original vertical position intact
    #[must_use]
    #[inline]
    pub fn centered_in_rect_horizontally(self, target: Rect) -> Rect {
        let pos_x = block_centered_in_point(self.dim.x, target.center().x);
        Rect::from_xy_dim(pos_x, self.pos.y, self.dim)
    }

    /// Returns a version of the rectangle that is centered vertically in a given rect, leaving
    /// the original horizontal position intact
    #[must_use]
    #[inline]
    pub fn centered_in_rect_vertically(self, target: Rect) -> Rect {
        let pos_y = block_centered_in_point(self.dim.y, target.center().y);
        Rect::from_xy_dim(self.pos.x, pos_y, self.dim)
    }

    /// Returns the biggest proportionally stretched version of the rectangle that can fit
    /// into `target`.
    #[must_use]
    #[inline]
    pub fn rect_stretched_to_fit(rect: Rect, target: Rect) -> Rect {
        debug_assert!(!is_effectively_zero(rect.height()));
        debug_assert!(!is_effectively_zero(target.height()));

        let source_aspect_ratio = rect.width() / rect.height();
        let target_aspect_ratio = target.width() / target.height();

        let scale_factor = if source_aspect_ratio < target_aspect_ratio {
            // Target rect is 'wider' than ours -> height is our limit when stretching
            target.height() / rect.height()
        } else {
            // Target rect is 'narrower' than ours -> width is our limit when stretching
            target.width() / rect.width()
        };

        let stretched_width = rect.width() * scale_factor;
        let stretched_height = rect.height() * scale_factor;

        return Rect::from_pos_width_height(rect.pos, stretched_width, stretched_height);
    }
}
