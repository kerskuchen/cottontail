////////////////////////////////////////////////////////////////////////////////////////////////////
// Random
//

use super::math::*;

use rand::distributions::Distribution;
use rand::distributions::Uniform;
pub use rand::Error;
pub use rand::SeedableRng;
pub use rand::{Rng, RngCore};

#[derive(Clone)]
pub struct Random {
    pub seed: u64,
    pub generator: rand_pcg::Pcg32,
}

impl Random {
    #[inline]
    pub fn new_from_seed(seed: u64) -> Random {
        Random {
            seed,
            generator: rand_pcg::Pcg32::seed_from_u64(seed),
        }
    }
}

impl Random {
    /// Returns a uniformly distributed number in [min, max]
    #[inline]
    pub fn f32_in_range_closed(&mut self, min: f32, max: f32) -> f32 {
        Uniform::new_inclusive(min, max).sample(self)
    }

    /// Returns a uniformly distributed number in [0.0, 1.0]
    #[inline]
    pub fn f32_in_01_closed(&mut self) -> f32 {
        Uniform::new_inclusive(0.0, 1.0).sample(self)
    }

    /// Returns a uniformly distributed number in ]0.0, 1.0[
    #[inline]
    pub fn f32_in_01_open(&mut self) -> f32 {
        self.sample(rand::distributions::Open01)
    }

    /// Returns a uniformly distributed integer in [0, max]
    #[inline]
    pub fn usize_bounded(&mut self, max: usize) -> usize {
        Uniform::new_inclusive(0, max).sample(self)
    }

    /// Returns a uniformly distributed integer in [0, max - 1]
    #[inline]
    pub fn usize_bounded_exclusive(&mut self, max: usize) -> usize {
        self.gen_range(0, max)
    }

    #[inline]
    pub fn vec2_in_rect(&mut self, rect: Rect) -> Vec2 {
        Vec2 {
            x: self.f32_in_range_closed(rect.left(), rect.right()),
            y: self.f32_in_range_closed(rect.top(), rect.bottom()),
        }
    }

    /// NOTE: Returns values in [-1, 1]x[-1, 1]
    #[inline]
    pub fn vec2_in_unit_rect(&mut self) -> Vec2 {
        Vec2 {
            x: self.f32_in_range_closed(-1.0, 1.0),
            y: self.f32_in_range_closed(-1.0, 1.0),
        }
    }

    /// NOTE: This uses rejection sampling
    #[inline]
    pub fn vec2_in_unit_disk(&mut self) -> Vec2 {
        loop {
            let result = self.vec2_in_unit_rect();
            if result.magnitude_squared() <= 1.0 {
                break result;
            }
        }
    }

    #[inline]
    pub fn vec2_in_unit_circle(&mut self) -> Vec2 {
        let angle = self.f32_in_range_closed(-PI, PI);
        Vec2::from_angle_flipped_y(angle)
    }
}

impl RngCore for Random {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.generator.next_u32()
    }
    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.generator.next_u64()
    }
    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.generator.fill_bytes(dest)
    }
    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.generator.try_fill_bytes(dest)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shufflebag

/// Returns a random element from a list until the list is empty, then resets the list
/// Based on:
/// https://gamedevelopment.tutsplus.com/tutorials/shuffle-bags-making-random-feel-more-random--gamedev-1249
///
pub struct Shufflebag<ElemType>
where
    ElemType: Clone + Copy,
{
    pub elems: Vec<ElemType>,
    current_bagsize: usize,
}

impl<ElemType> Shufflebag<ElemType>
where
    ElemType: Clone + Copy,
{
    #[inline]
    pub fn new(elems: Vec<ElemType>) -> Shufflebag<ElemType> {
        let elem_count = elems.len();
        Shufflebag {
            elems,
            current_bagsize: elem_count,
        }
    }

    #[inline]
    pub fn shufflebag_get_next(&mut self, random: &mut Random) -> ElemType {
        if self.current_bagsize == 1 {
            self.current_bagsize = self.elems.len();
            self.elems[0]
        } else {
            let index = random.usize_bounded_exclusive(self.current_bagsize);
            self.current_bagsize -= 1;
            self.elems.swap(index, self.current_bagsize);
            self.elems[self.current_bagsize]
        }
    }
}
