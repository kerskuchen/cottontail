////////////////////////////////////////////////////////////////////////////////////////////////////
// Random
//

use super::*;

#[derive(Clone)]
pub struct Random {
    pub seed: u64,
    pub generator: oorandom::Rand32,
}

impl Random {
    #[inline]
    pub fn new_from_seed(seed: u64) -> Random {
        Random {
            seed,
            generator: oorandom::Rand32::new(seed),
        }
    }

    // Picks a random element from given slice
    #[inline]
    pub fn pick_from_slice<ElemType>(&mut self, slice: &[ElemType]) -> ElemType
    where
        ElemType: Copy + Clone,
    {
        assert!(
            slice.len() < std::u32::MAX as usize,
            "Shufflebag only supports u32 sized containers"
        );
        let index = self.u32_bounded_exclusive(slice.len() as u32) as usize;
        slice[index]
    }

    /// Returns a uniformly distributed number in [min, max[
    #[inline]
    pub fn f32_in_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.generator.rand_float()
    }

    /// Returns a uniformly distributed number in [0.0, 1.0[
    #[inline]
    pub fn f32(&mut self) -> f32 {
        self.generator.rand_float()
    }

    /// Returns a uniformly distributed integer in [std::i32::MIN, std::i32::MAX]
    #[inline]
    pub fn i32(&mut self) -> i32 {
        self.generator.rand_i32()
    }

    /// Returns a uniformly distributed integer in [0, std::u32::MAX]
    #[inline]
    pub fn u32(&mut self) -> u32 {
        self.generator.rand_u32()
    }

    /// Returns a uniformly distributed integer in [0, max]
    #[inline]
    pub fn u32_bounded(&mut self, max: u32) -> u32 {
        self.generator.rand_range(0..max)
    }

    /// Returns a uniformly distributed integer in [0, max - 1]
    #[inline]
    pub fn u32_bounded_exclusive(&mut self, max_exclusive: u32) -> u32 {
        self.generator.rand_range(0..(max_exclusive - 1))
    }

    #[inline]
    pub fn vec2_in_rect(&mut self, rect: Rect) -> Vec2 {
        Vec2 {
            x: self.f32_in_range(rect.left(), rect.right()),
            y: self.f32_in_range(rect.top(), rect.bottom()),
        }
    }

    #[inline]
    pub fn vec2_in_disk(&mut self, center: Vec2, radius: f32) -> Vec2 {
        center + radius * self.vec2_in_unit_disk()
    }

    /// NOTE: Returns values in [-1, 1[x[-1, 1[
    #[inline]
    pub fn vec2_in_unit_rect(&mut self) -> Vec2 {
        Vec2 {
            x: self.f32_in_range(-1.0, 1.0),
            y: self.f32_in_range(-1.0, 1.0),
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
        let angle = self.f32_in_range(-PI, PI);
        Vec2::from_angle_flipped_y(angle)
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
        assert!(
            elems.len() <= std::u32::MAX as usize,
            "Shufflebag only supports u32 sized containers"
        );
        let elem_count = elems.len();
        Shufflebag {
            elems,
            current_bagsize: elem_count,
        }
    }

    #[inline]
    pub fn new_with_counts(elems_and_counts: &[(ElemType, usize)]) -> Shufflebag<ElemType> {
        let mut elems = Vec::new();

        for (elem, count) in elems_and_counts {
            assert!(
                *count <= std::u32::MAX as usize,
                "Shufflebag only supports u32 sized containers"
            );
            for _ in 0..*count {
                elems.push(*elem);
            }
        }

        assert!(
            elems.len() <= std::u32::MAX as usize,
            "Shufflebag only supports u32 sized containers"
        );
        let elem_count = elems.len();
        Shufflebag {
            elems,
            current_bagsize: elem_count,
        }
    }

    #[inline]
    pub fn get_next(&mut self, random: &mut Random) -> ElemType {
        if self.current_bagsize == 1 {
            self.current_bagsize = self.elems.len();
            self.elems[0]
        } else {
            let index = random.u32_bounded_exclusive(self.current_bagsize as u32) as usize;
            self.current_bagsize -= 1;
            self.elems.swap(index, self.current_bagsize);
            self.elems[self.current_bagsize]
        }
    }
}
