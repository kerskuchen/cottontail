use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Containment

pub enum RectIntersectionResult {
    None,
    AContainsB(Rect),
    BContainsA(Rect),
    Real(Rect),
}

impl Rect {
    #[inline]
    pub fn contains_rect(&self, other: Rect) -> bool {
        self.left() <= other.left()
            && other.right() < self.right()
            && self.top() <= other.top()
            && other.bottom() < self.bottom()
    }

    #[inline]
    pub fn contains_point(&self, point: Vec2) -> bool {
        self.left() <= point.x
            && point.x < self.right()
            && self.top() <= point.y
            && point.y < self.bottom()
    }

    #[inline]
    pub fn intersect(a: Rect, b: Rect) -> RectIntersectionResult {
        if !intersects_rect_rect(a, b) {
            RectIntersectionResult::None
        } else if a.contains_rect(b) {
            RectIntersectionResult::AContainsB(b)
        } else if b.contains_rect(a) {
            RectIntersectionResult::BContainsA(a)
        } else {
            RectIntersectionResult::Real(Rect::from_bounds_left_top_right_bottom(
                f32::max(a.left(), b.left()),
                f32::max(a.top(), b.top()),
                f32::min(a.right(), b.right()),
                f32::min(a.bottom(), b.bottom()),
            ))
        }
    }

    /// Clips the given rect by another
    /// Panics if the clipping_rect does not contain our rect
    #[must_use]
    #[inline]
    pub fn clipped_by(self, clipping_rect: Rect) -> Option<Rect> {
        match Rect::intersect(clipping_rect, self) {
            RectIntersectionResult::AContainsB(intersection) => {
                // Our clipping rect contains our rect fully - no clipping needed
                Some(intersection)
            }
            RectIntersectionResult::Real(intersection) => {
                // Real intersection
                Some(intersection)
            }
            RectIntersectionResult::BContainsA(intersection) => {
                // Our rect contains the clipping rect
                Some(intersection)
            }
            RectIntersectionResult::None => {
                // Our rect has no intersection with clipping rect
                None
            }
        }
    }
}

pub enum RectiIntersectionResult {
    None,
    AContainsB(Recti),
    BContainsA(Recti),
    Real(Recti),
}

impl Recti {
    #[inline]
    pub fn contains_rect(&self, other: Recti) -> bool {
        self.left() <= other.left()
            && other.right() < self.right()
            && self.top() <= other.top()
            && other.bottom() < self.bottom()
    }

    #[inline]
    pub fn contains_point(&self, point: Vec2i) -> bool {
        (self.left() <= point.x && point.x < self.right())
            && (self.top() <= point.y && point.y < self.bottom())
    }

    #[inline]
    pub fn contains_point_exclusive_right_bottom(self, v: Vec2i) -> bool {
        (self.left() <= v.x && v.x < self.right()) && (self.top() <= v.y && v.y < self.bottom())
    }

    #[inline]
    pub fn intersect(a: Recti, b: Recti) -> RectiIntersectionResult {
        if !intersects_recti_recti(a, b) {
            RectiIntersectionResult::None
        } else if a.contains_rect(b) {
            RectiIntersectionResult::AContainsB(b)
        } else if b.contains_rect(a) {
            RectiIntersectionResult::BContainsA(a)
        } else {
            RectiIntersectionResult::Real(Recti::from_bounds_left_top_right_bottom(
                i32::max(a.left(), b.left()),
                i32::max(a.top(), b.top()),
                i32::min(a.right(), b.right()),
                i32::min(a.bottom(), b.bottom()),
            ))
        }
    }

    /// Clips the given rect by another
    #[must_use]
    #[inline]
    pub fn clipped_by(self, clipping_rect: Recti) -> Option<Recti> {
        match Recti::intersect(clipping_rect, self) {
            RectiIntersectionResult::AContainsB(intersection) => {
                // Our clipping rect contains our rect fully - no clipping needed
                Some(intersection)
            }
            RectiIntersectionResult::Real(intersection) => {
                // Real intersection
                Some(intersection)
            }
            RectiIntersectionResult::BContainsA(intersection) => {
                // Our rect contains the clipping rect
                Some(intersection)
            }
            RectiIntersectionResult::None => {
                // Our rect has no intersection with clipping rect
                None
            }
        }
    }

    /// Returns the closest point the given rect needs to be moved to, to not overlap with any of
    /// the other given rects
    #[inline]
    pub fn get_closest_position_without_overlapping(self, others: &[Recti]) -> Vec2i {
        if !others
            .iter()
            .any(|&other| intersects_recti_recti(self, other))
        {
            // No intersections (also when `others` is empty)
            return self.pos;
        }

        let search_directions = [
            Vec2i::new(1, 0),
            Vec2i::new(0, 1),
            Vec2i::new(-1, 0),
            Vec2i::new(0, -1),
            Vec2i::new(1, 1),
            Vec2i::new(-1, 1),
            Vec2i::new(-1, -1),
            Vec2i::new(1, -1),
        ];

        let mut multiplier = 1;
        loop {
            for &dir in &search_directions {
                let test_rect = self.translated_by(multiplier * dir);
                if !others
                    .iter()
                    .any(|&other| intersects_recti_recti(test_rect, other))
                {
                    // No intersections
                    return test_rect.pos;
                }
            }
            multiplier += 1;
        }
    }
}
////////////////////////////////////////////////////////////////////////////////////////////////////
// Intersection existence queries
//

#[inline]
pub fn intersects_circle_circle(a: Circle, b: Circle) -> bool {
    squared(a.radius + b.radius) < Vec2::distance_squared(a.center, b.center)
}

#[inline]
pub fn intersects_rect_rect(a: Rect, b: Rect) -> bool {
    !((a.right() < b.left())
        || (a.left() >= b.right())
        || (a.bottom() < b.top())
        || (a.top() >= b.bottom()))
}

#[inline]
pub fn intersects_recti_recti(a: Recti, b: Recti) -> bool {
    !((a.right() < b.left())
        || (a.left() >= b.right())
        || (a.bottom() < b.top())
        || (a.top() >= b.bottom()))
}

#[inline]
pub fn intersects_point_rect(rect: Rect, point: Point) -> bool {
    !((point.x < rect.left())
        || (point.x >= rect.right())
        || (point.y < rect.top())
        || (point.y >= rect.bottom()))
}

#[inline]
pub fn intersects_pointi_recti(rect: Recti, point: Pointi) -> bool {
    !((point.x < rect.left())
        || (point.x >= rect.right())
        || (point.y < rect.top())
        || (point.y >= rect.bottom()))
}

impl Point {
    #[inline]
    pub fn intersects_line(self, line: Line, line_thickness: f32) -> bool {
        let distance_to_start = self - line.start;
        let distance_to_line = f32::abs(Vec2::dot(distance_to_start, line.normal()));
        distance_to_line <= line_thickness
    }

    #[inline]
    pub fn intersects_circle(self, circle: Circle, line_thickness: f32) -> bool {
        let squared_distance_to_center = Vec2::distance_squared(self, circle.center);
        let circle_radius_min = circle.radius - line_thickness;
        let circle_radius_max = circle.radius + line_thickness;
        debug_assert!(circle_radius_min >= 0.0);

        in_interval(
            squared_distance_to_center,
            circle_radius_min * circle_radius_min,
            circle_radius_max * circle_radius_max,
        )
    }

    #[inline]
    pub fn intersects_sphere(self, sphere: Circle) -> bool {
        Vec2::distance_squared(self, sphere.center) <= sphere.radius * sphere.radius
    }

    #[inline]
    pub fn intersects_rect(self, rect: Rect) -> bool {
        rect.left() <= self.x
            && self.x <= rect.right()
            && rect.top() <= self.y
            && self.y <= rect.bottom()
    }
}

impl Line {
    #[inline]
    pub fn intersects_line(self, other: Line) -> bool {
        intersection_line_line(self, other).is_some()
    }

    #[inline]
    pub fn intersects_circle(self, circle: Circle) -> bool {
        let (intersection_near, intersection_far) = intersections_line_circle(self, circle);
        intersection_near.is_some() || intersection_far.is_some()
    }

    #[inline]
    pub fn intersects_sphere(self, sphere: Circle) -> bool {
        self.intersects_circle(sphere) || self.start.intersects_sphere(sphere)
    }

    #[inline]
    pub fn intersects_rect(self, rect: Rect) -> bool {
        self.start.intersects_rect(rect)
            || intersections_line_rect(self, rect)
                .iter()
                .any(|intersection| intersection.is_some())
    }
}

impl Circle {
    #[inline]
    pub fn intersects_line(self, line: Line, line_thickness: f32) -> bool {
        let distance_to_start = self.center - line.start;
        let distance_to_line = f32::abs(Vec2::dot(distance_to_start, line.normal()));
        distance_to_line <= line_thickness + self.radius
    }

    #[inline]
    pub fn intersects_circle(self, other: Circle) -> bool {
        let radius_sum = self.radius + other.radius;
        Vec2::distance_squared(self.center, other.center) <= radius_sum * radius_sum
    }

    #[inline]
    pub fn intersects_rect(self, rect: Rect) -> bool {
        let rect_point_that_is_nearest_to_circle = Point::new(
            f32::max(rect.left(), f32::min(self.center.x, rect.right())),
            f32::max(rect.bottom(), f32::min(self.center.y, rect.top())),
        );
        rect_point_that_is_nearest_to_circle.intersects_sphere(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Intersection with intersection-point information
//

#[derive(Debug, Clone, Copy)]
pub struct Intersection {
    pub point: Point,
    pub normal: Vec2,
    pub time: f32,
}

/// Returns the intersections for segments in the following order left, right, top, bottom
#[inline]
pub fn intersections_line_rect(line: Line, rect: Rect) -> [Option<Intersection>; 4] {
    let left = intersection_line_line(line, rect.left_segment());
    let right = intersection_line_line(line, rect.right_segment());
    let top = intersection_line_line(line, rect.top_segment());
    let bottom = intersection_line_line(line, rect.bottom_segment());
    [left, right, top, bottom]
}

#[inline]
pub fn intersections_line_circle(
    line: Line,
    circle: Circle,
) -> (Option<Intersection>, Option<Intersection>) {
    // We need to find the `t` values for the intersection points
    // `p = line.start + t * line_dir` such that `||p - circle.center||^2 == circle.radius^2`.
    // Substituting `p` in the above equation leads to solving the quadratic equation
    // `at^2 + bt + c = 0`
    // with
    // `a = ||line.end - line.start||^2`
    // `b = 2 dot(line.start - circle.center, line.end - line.start)`
    // `c = ||line.end - line.start||^2 - circle.radius^2`
    //
    // which solution is `t = (-b +- sqrt(b^2 - 4ac)) / 2a`
    let line_dir = line.end - line.start;
    let center_to_line_start = line.start - circle.center;
    let a = Vec2::dot(line_dir, line_dir);
    if is_effectively_zero(a) {
        // line.start == line.end
        debug_assert!(false);
        return (None, None);
    }
    let b = 2.0 * Vec2::dot(center_to_line_start, line_dir);
    let c = Vec2::dot(center_to_line_start, center_to_line_start) - circle.radius * circle.radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        // No intersection with circle
        return (None, None);
    }

    let discriminant = f32::sqrt(discriminant);
    let recip_a = f32::recip(2.0 * a);
    // NOTE: t_min <= t_max because a > 0 and discriminant >= 0
    let t_min = (-b - discriminant) * recip_a;
    let t_max = (-b + discriminant) * recip_a;

    let t_min_result = if 0.0 <= t_min && t_min <= 1.0 {
        let point = line.start + t_min * line_dir;
        let normal = (point - circle.center).normalized();
        Some(Intersection {
            point,
            normal,
            time: t_min,
        })
    } else {
        None
    };

    let t_max_result = if 0.0 <= t_max && t_max <= 1.0 {
        let point = line.start + t_max * line_dir;
        let normal = (point - circle.center).normalized();
        Some(Intersection {
            point,
            normal,
            time: t_max,
        })
    } else {
        None
    };

    (t_min_result, t_max_result)
}

// Checks intersection of a line with multiple lines.
#[inline]
pub fn intersections_line_lines(line: Line, line_segments: &[Line]) -> Vec<Option<Intersection>> {
    line_segments
        .iter()
        .map(|segment| intersection_line_line(line, *segment))
        .collect()
}

// Checks whether two line segments intersect. If so returns the intersection point `point`
// and the time of intersection `time_a` with `point = a.start + time_a * (a.end - a.start)`.
// See https://stackoverflow.com/a/565282 for derivation
// with p = self.start, r = self_dir, q = line.start, s = line_dir.
// NOTE: We treat colinear line segments as non-intersecting
#[inline]
pub fn intersection_line_line(a: Line, b: Line) -> Option<Intersection> {
    let dir_a = a.end - a.start;
    let dir_b = b.end - b.start;
    let dir_a_x_dir_b = Vec2::cross_z(dir_a, dir_b);

    if !is_effectively_zero(dir_a_x_dir_b) {
        let diff_start_b_a = b.start - a.start;
        let time_a = Vec2::cross_z(diff_start_b_a, dir_b) / dir_a_x_dir_b;
        let time_b = Vec2::cross_z(diff_start_b_a, dir_a) / dir_a_x_dir_b;

        // Check if t in [0, 1] and u in [0, 1]
        if time_a >= 0.0 && time_a <= 1.0 && time_b >= 0.0 && time_b <= 1.0 {
            let intersection = Intersection {
                point: a.start + time_a * dir_a,
                normal: dir_b.perpendicular().normalized(),
                time: time_a,
            };
            return Some(intersection);
        }
    }
    None
}
