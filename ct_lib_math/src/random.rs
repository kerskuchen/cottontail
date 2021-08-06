////////////////////////////////////////////////////////////////////////////////////////////////////
// Random
//

use super::*;

#[derive(Clone)]
pub struct Random {
    pub seed: u64,
    generator: oorandom::Rand32,
}

impl Random {
    #[inline]
    pub fn new_from_seed(seed: u64) -> Random {
        Random {
            seed,
            generator: oorandom::Rand32::new(seed),
        }
    }

    #[inline]
    pub fn new_from_seed_multiple(seed: u64, count: usize) -> Vec<Random> {
        (0..count)
            .into_iter()
            .map(|index| Random {
                seed,
                generator: oorandom::Rand32::new_inc(seed, index as u64),
            })
            .collect()
    }

    /// Returns a uniformly distributed integer in [std::i32::MIN, std::i32::MAX]
    #[inline]
    pub fn i32(&mut self) -> i32 {
        self.generator.rand_u32() as i32
    }

    /// Returns a uniformly distributed integer in [0, std::u32::MAX]
    #[inline]
    pub fn u32(&mut self) -> u32 {
        self.generator.rand_u32()
    }

    /// Returns a uniformly distributed integer in [0, max]
    #[inline]
    pub fn u32_bounded(&mut self, max: u32) -> u32 {
        self.u32_bounded_exclusive(max + 1)
    }

    /// Returns a uniformly distributed integer in [0, max - 1]
    #[inline]
    pub fn u32_bounded_exclusive(&mut self, max_exclusive: u32) -> u32 {
        assert!(max_exclusive > 0);
        let remainder = std::u32::MAX % max_exclusive;
        let rejection_bound = std::u32::MAX - remainder;

        loop {
            let result = self.u32();
            if result <= rejection_bound {
                return result % max_exclusive;
            }
        }
    }

    /// Returns a uniformly distributed integer in [0, max]
    #[inline]
    pub fn u32_in_range(&mut self, min: u32, max: u32) -> u32 {
        self.u32_in_range_exclusive(min, max + 1)
    }

    /// Returns a uniformly distributed integer in [0, max - 1]
    #[inline]
    pub fn u32_in_range_exclusive(&mut self, min: u32, max_exclusive: u32) -> u32 {
        assert!(min < max_exclusive);
        let length = max_exclusive - min;
        let random = self.u32_bounded_exclusive(length);
        random + min
    }

    /// Returns a uniformly distributed number in [0.0, 1.0[
    pub fn f32(&mut self) -> f32 {
        // This `swap_bytes` part is not strictly necessary but MSB is usually more 'random' so we swap LSB and MSB
        let random = self.generator.rand_u32().swap_bytes();

        // |sign (1-bit) | exponent (8-bits) | mantissa (23-bit) |
        // https://www.h-schmidt.net/FloatConverter/IEEE754de.html
        // 0x3FFF_FFFF = 0 0111 1111 1111 ... = (-1)^0 * (2^0) * mantissa = 1.0 + [0.0, 1.0[
        // This is a number in [1.0, 2.0[ - therefore we subtract -1
        const AND_PART: u32 = 0b0000_0000_0111_1111_1111_1111_1111_1111;
        const OR_PART_: u32 = 0b0011_1111_1000_0000_0000_0000_0000_00001;
        let bytes = (random & AND_PART) | OR_PART_;

        f32::from_ne_bytes(bytes.to_ne_bytes()) - 1.0
    }

    /// Returns a uniformly distributed number in [min, max[
    #[inline]
    pub fn f32_in_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.f32()
    }

    /// Returns a uniformly distributed number in [rect.left, rect.right[ x [rect.top, rect.bottom[
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

    /// Picks a random element from given slice
    #[inline]
    pub fn pick_from_slice<ElemType>(&mut self, slice: &[ElemType]) -> ElemType
    where
        ElemType: Copy + Clone,
    {
        assert!(
            slice.len() < std::u32::MAX as usize,
            "Only u32 sized containers supported"
        );
        let index = self.u32_bounded_exclusive(slice.len() as u32) as usize;
        slice[index]
    }

    /// Based on https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
    #[inline]
    pub fn shuffle_slice<ElemType>(&mut self, slice: &mut [ElemType]) {
        assert!(
            slice.len() < std::u32::MAX as usize,
            "Only u32 sized containers supported"
        );
        let mut last_index = slice.len() - 1;
        while last_index > 0 {
            let random_index = self.u32_in_range(0, last_index as u32) as usize;
            slice.swap(random_index, last_index);
            last_index -= 1;
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shufflebag

/// Returns a random element from a list until the list is empty, then resets the list
/// Based on:
/// https://gamedevelopment.tutsplus.com/tutorials/shuffle-bags-making-random-feel-more-random--gamedev-1249
/// https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
///
pub struct Shufflebag<ElemType: Clone> {
    pub elems: Vec<ElemType>,
    current_bagsize: usize,
}

impl<ElemType: Clone> Shufflebag<ElemType> {
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
                elems.push(elem.clone());
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
    pub fn reset(&mut self) {
        self.current_bagsize = self.elems.len();
    }

    #[inline]
    pub fn get_next(&mut self, random: &mut Random) -> ElemType {
        if self.current_bagsize == 1 {
            self.current_bagsize = self.elems.len();
            self.elems[0].clone()
        } else {
            let index = random.u32_bounded_exclusive(self.current_bagsize as u32) as usize;
            self.current_bagsize -= 1;
            self.elems.swap(index, self.current_bagsize);
            self.elems[self.current_bagsize].clone()
        }
    }
}
