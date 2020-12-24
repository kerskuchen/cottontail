pub use ezing::*;

#[inline]
pub fn step_middle(percent: f32) -> f32 {
    if percent < 0.5 {
        0.0
    } else {
        1.0
    }
}

#[inline]
pub fn step_end(percent: f32) -> f32 {
    if percent < 1.0 {
        0.0
    } else {
        1.0
    }
}
