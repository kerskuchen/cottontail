////////////////////////////////////////////////////////////////////////////////////////////////////
// Grid

use super::math::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Copy, Clone)]
pub enum GluePosition {
    LeftTop,
    LeftCenter,
    LeftBottom,

    TopLeft,
    TopCenter,
    TopRight,

    RightTop,
    RightCenter,
    RightBottom,

    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Copy, Clone)]
pub enum SearchDirection {
    LeftToRight,
    TopToBottom,
    RightToLeft,
    BottomToTop,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid<CellType>
where
    CellType: Default + Clone + Copy + PartialEq,
{
    pub width: i32,
    pub height: i32,
    pub data: Vec<CellType>,
}

impl<CellType> Default for Grid<CellType>
where
    CellType: Default + Clone + Copy + PartialEq,
{
    fn default() -> Self {
        Grid::empty()
    }
}

impl<CellType> Grid<CellType>
where
    CellType: Default + Clone + Copy + PartialEq,
{
    #[inline]
    pub fn empty() -> Grid<CellType> {
        Grid {
            width: 0,
            height: 0,
            data: Vec::with_capacity(0),
        }
    }

    #[inline]
    pub fn new(width: u32, height: u32) -> Grid<CellType> {
        Grid::new_filled(width, height, CellType::default())
    }

    #[inline]
    pub fn new_filled(width: u32, height: u32, filltype: CellType) -> Grid<CellType> {
        let cellcount = (width * height) as usize;
        let data = vec![filltype; cellcount];
        Grid::new_from_buffer(width, height, data)
    }

    #[inline]
    pub fn new_from_buffer(width: u32, height: u32, buffer: Vec<CellType>) -> Grid<CellType> {
        debug_assert!(width > 0 && height > 0);
        assert!((width * height) as usize == buffer.len());
        Grid {
            width: width as i32,
            height: height as i32,
            data: buffer,
        }
    }

    #[inline]
    pub fn dim(&self) -> Vec2i {
        Vec2i::new(self.width, self.height)
    }

    #[inline]
    pub fn rect(&self) -> Recti {
        Recti::from_width_height(self.width, self.height)
    }

    #[inline]
    pub fn clear(&mut self, clear_cell: CellType) {
        self.data.iter_mut().for_each(|cell| {
            *cell = clear_cell;
        });
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> CellType {
        self.data[(x + y * self.width) as usize]
    }
    #[inline]
    pub fn get_or_default(&self, x: i32, y: i32, default: CellType) -> CellType {
        if self.contains_coordinate(x, y) {
            self.data[(x + y * self.width) as usize]
        } else {
            default
        }
    }
    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut CellType {
        &mut self.data[(x + y * self.width) as usize]
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, value: CellType) {
        self.data[(x + y * self.width) as usize] = value
    }
    #[inline]
    pub fn set_safely(&mut self, x: i32, y: i32, value: CellType) {
        if self.contains_coordinate(x, y) {
            self.data[(x + y * self.width) as usize] = value
        }
    }

    #[inline]
    pub fn contains_coordinate(&self, x: i32, y: i32) -> bool {
        0 <= x && x < self.width && 0 <= y && y < self.height
    }

    #[inline]
    pub fn swap_cells(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let left = self.get(x1, y1);
        let right = self.get(x2, y2);
        self.set(x1, y1, right);
        self.set(x1, y1, left);
    }

    #[inline]
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        in_intervali_exlusive_max(x, 0, self.width) && in_intervali_exlusive_max(y, 0, self.height)
    }

    /// NOTE: This also returns cell indices for points that lay outside of the grid.
    #[inline]
    pub fn get_cell_index_for_pos_virtual(&self, x: i32, y: i32, cell_size: i32) -> Vec2i {
        debug_assert!(cell_size > 1); // NOTE: (cell_size == 1) => (result == point)

        Vec2i {
            x: x / cell_size,
            y: y / cell_size,
        }
    }

    #[inline]
    pub fn get_cell_index_for_pos(&self, x: i32, y: i32, cell_size: i32) -> Vec2i {
        debug_assert!(in_intervali_exlusive_max(x, 0, self.width * cell_size));
        debug_assert!(in_intervali_exlusive_max(y, 0, self.height * cell_size));

        self.get_cell_index_for_pos_virtual(x, y, cell_size)
    }

    /// NOTE: This also returns valid rects for cells that would lay outside of the grid
    #[inline]
    pub fn get_cell_rect_virtual(&self, x: i32, y: i32, cell_size: i32) -> Recti {
        assert!(cell_size > 0);
        Recti::from_xy_width_height(x * cell_size, y * cell_size, cell_size, cell_size)
    }

    #[inline]
    pub fn get_cell_rect(&self, x: i32, y: i32, cell_size: i32) -> Recti {
        debug_assert!(in_intervali_exlusive_max(x, 0, self.width * cell_size));
        debug_assert!(in_intervali_exlusive_max(y, 0, self.height * cell_size));

        self.get_cell_rect_virtual(x, y, cell_size)
    }

    pub fn copy_region(
        source_grid: &Grid<CellType>,
        source_rect: Recti,
        dest_grid: &mut Grid<CellType>,
        dest_rect: Recti,
        mask_value: Option<CellType>,
    ) {
        assert!(source_rect.dim == dest_rect.dim);

        assert!(source_rect.pos.x >= 0);
        assert!(source_rect.pos.y >= 0);
        assert!(source_rect.right() <= source_grid.width);
        assert!(source_rect.bottom() <= source_grid.height);

        assert!(dest_rect.pos.x >= 0);
        assert!(dest_rect.pos.y >= 0);
        assert!(dest_rect.right() <= dest_grid.width);
        assert!(dest_rect.bottom() <= dest_grid.height);

        if let Some(mask_color) = mask_value {
            for y in 0..source_rect.height() {
                for x in 0..source_rect.width() {
                    let source_value =
                        source_grid.get(source_rect.pos.x + x, source_rect.pos.y + y);
                    if source_value != mask_color {
                        dest_grid.set(dest_rect.pos.x + x, dest_rect.pos.y + y, source_value);
                    }
                }
            }
        } else {
            for y in 0..source_rect.height() {
                for x in 0..source_rect.width() {
                    let source_value =
                        source_grid.get(source_rect.pos.x + x, source_rect.pos.y + y);
                    dest_grid.set(dest_rect.pos.x + x, dest_rect.pos.y + y, source_value);
                }
            }
        }
    }

    pub fn blit_to(&self, other: &mut Grid<CellType>, pos: Vec2i, mask_value: Option<CellType>) {
        let rect_source = self.rect();
        let rect_dest = rect_source.translated_by(pos);
        Grid::<CellType>::copy_region(self, rect_source, other, rect_dest, mask_value);
    }

    /// Searches grid from given search direction until given condition is met.
    /// Returns coordinate of found cell
    pub fn find_first<F>(&self, search_dir: SearchDirection, mut compare: F) -> Option<Vec2i>
    where
        F: FnMut(CellType) -> bool,
    {
        match search_dir {
            SearchDirection::LeftToRight => {
                for x in 0..self.width {
                    for y in 0..self.height {
                        if compare(self.get(x, y)) {
                            return Some(Vec2i::new(x, y));
                        }
                    }
                }
            }
            SearchDirection::TopToBottom => {
                for y in 0..self.height {
                    for x in 0..self.width {
                        if compare(self.get(x, y)) {
                            return Some(Vec2i::new(x, y));
                        }
                    }
                }
            }
            SearchDirection::RightToLeft => {
                for x in (0..self.width).rev() {
                    for y in 0..self.height {
                        if compare(self.get(x, y)) {
                            return Some(Vec2i::new(x, y));
                        }
                    }
                }
            }
            SearchDirection::BottomToTop => {
                for y in (0..self.height).rev() {
                    for x in 0..self.width {
                        if compare(self.get(x, y)) {
                            return Some(Vec2i::new(x, y));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn trim(
        &mut self,
        trim_left: bool,
        trim_top: bool,
        trim_right: bool,
        trim_bottom: bool,
        trim_value: CellType,
    ) {
        *self = self.trimmed(trim_left, trim_top, trim_right, trim_bottom, trim_value);
    }

    pub fn trimmed(
        &self,
        trim_left: bool,
        trim_top: bool,
        trim_right: bool,
        trim_bottom: bool,
        trim_value: CellType,
    ) -> Grid<CellType> {
        let new_left = if trim_left {
            if let Some(coord) = self.find_first(SearchDirection::LeftToRight, |cell_value| {
                cell_value != trim_value
            }) {
                coord.x
            } else {
                return Grid::empty();
            }
        } else {
            0
        };
        let new_top = if trim_top {
            if let Some(coord) = self.find_first(SearchDirection::TopToBottom, |cell_value| {
                cell_value != trim_value
            }) {
                coord.y
            } else {
                return Grid::empty();
            }
        } else {
            0
        };
        let new_right = if trim_right {
            if let Some(coord) = self.find_first(SearchDirection::RightToLeft, |cell_value| {
                cell_value != trim_value
            }) {
                coord.x
            } else {
                return Grid::empty();
            }
        } else {
            self.width - 1
        };
        let new_bottom = if trim_bottom {
            if let Some(coord) = self.find_first(SearchDirection::BottomToTop, |cell_value| {
                cell_value != trim_value
            }) {
                coord.y
            } else {
                return Grid::empty();
            }
        } else {
            self.height - 1
        };

        let new_width = 1 + new_right - new_left;
        let new_height = 1 + new_bottom - new_top;

        let mut trimmed_result = Grid::new(new_width as u32, new_height as u32);
        Grid::copy_region(
            &self,
            Recti::from_xy_width_height(new_left, new_top, new_width, new_height),
            &mut trimmed_result,
            Recti::from_width_height(new_width, new_height),
            None,
        );

        trimmed_result
    }

    pub fn crop(&mut self, left: i32, top: i32, right: i32, bottom: i32) {
        *self = self.cropped(left, top, right, bottom);
    }

    pub fn cropped(&self, left: i32, top: i32, right: i32, bottom: i32) -> Grid<CellType> {
        let new_width = self.width - left - right;
        let new_height = self.height - top - bottom;
        if new_width <= 0 || new_height <= 0 {
            return Grid::empty();
        }

        let mut result = Grid::new(new_width as u32, new_height as u32);
        let crop_rect = Recti::from_xy_width_height(left, right, new_width, new_height);
        let result_rect = result.rect();
        Grid::copy_region(self, crop_rect, &mut result, result_rect, None);

        result
    }

    pub fn replace_cells(&mut self, to_replace: CellType, replace_with: CellType) {
        for cell in self.data.iter_mut() {
            if *cell == to_replace {
                *cell = replace_with;
            }
        }
    }

    #[must_use]
    pub fn with_replaced_cells(
        &self,
        to_replace: CellType,
        replace_with: CellType,
    ) -> Grid<CellType> {
        let mut result = self.clone();
        result.replace_cells(to_replace, replace_with);
        result
    }

    #[must_use]
    pub fn scaled(&self, scale: i32) -> Grid<CellType> {
        assert!(scale > 0);

        let mut scaled_grid = Grid::new((scale * self.width) as u32, (scale * self.height) as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.get(x, y);
                scaled_grid.draw_rect_filled(scale * x, scale * y, scale, scale, color);
            }
        }
        scaled_grid
    }

    pub fn copy_region_sample_nearest_neighbor(
        source_grid: &Grid<CellType>,
        source_rect: Recti,
        dest_grid: &mut Grid<CellType>,
        dest_rect: Recti,
    ) {
        assert!(source_rect.pos.x >= 0);
        assert!(source_rect.pos.y >= 0);
        assert!(source_rect.dim.x <= source_grid.width);
        assert!(source_rect.dim.y <= source_grid.height);

        assert!(dest_rect.pos.x >= 0);
        assert!(dest_rect.pos.y >= 0);
        assert!(dest_rect.dim.x <= dest_grid.width);
        assert!(dest_rect.dim.y <= dest_grid.height);

        for dest_y in dest_rect.top()..dest_rect.bottom() {
            for dest_x in dest_rect.left()..dest_rect.right() {
                let source_x = sample_integer_upper_exclusive_floored(
                    dest_x,
                    dest_rect.left(),
                    dest_rect.right(),
                    source_rect.left(),
                    source_rect.right(),
                );
                let source_y = sample_integer_upper_exclusive_floored(
                    dest_y,
                    dest_rect.top(),
                    dest_rect.bottom(),
                    source_rect.top(),
                    source_rect.bottom(),
                );
                let source_value = source_grid.get(source_x, source_y);
                dest_grid.set(dest_x, dest_y, source_value);
            }
        }
    }

    pub fn draw_rect_filled(
        &mut self,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
        value: CellType,
    ) {
        assert!(start_x >= 0);
        assert!(start_y >= 0);
        assert!(start_x + width <= self.width);
        assert!(start_y + height <= self.height);
        for y in start_y..(start_y + height) {
            for x in start_x..(start_x + width) {
                self.set(x, y, value);
            }
        }
    }

    pub fn draw_rect_filled_safely(
        &mut self,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
        value: CellType,
    ) {
        for y in start_y..(start_y + height) {
            for x in start_x..(start_x + width) {
                self.set_safely(x, y, value);
            }
        }
    }

    pub fn draw_rect(
        &mut self,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
        value: CellType,
    ) {
        assert!(start_x >= 0);
        assert!(start_y >= 0);
        assert!(start_x + width <= self.width);
        assert!(start_y + height <= self.height);

        for y in start_y..(start_y + height) {
            self.set(start_x, y, value);
            self.set(start_x + (width - 1), y, value);
        }
        for x in start_x..(start_x + width) {
            self.set(x, start_y, value);
            self.set(x, start_y + (height - 1), value);
        }
    }

    pub fn extend(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        padding_value: CellType,
    ) {
        let extended = self.extended(left, top, right, bottom, padding_value);
        *self = extended;
    }

    pub fn extended(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        padding_value: CellType,
    ) -> Grid<CellType> {
        let mut result = Grid::<CellType>::new_filled(
            (self.width + left + right) as u32,
            (self.height + top + bottom) as u32,
            padding_value,
        );
        self.blit_to(&mut result, Vec2i::new(left, top), None);
        result
    }

    // For [a, b, c, d, e] glues
    // (((a <- b) <- c) <- d) <- e
    pub fn glue_together_multiple(
        grids: &[Grid<CellType>],
        glue_position: GluePosition,
        padding_extra: i32,
        padding_color: CellType,
    ) -> Grid<CellType> {
        grids.iter().fold(Grid::empty(), |acc, grid| {
            Grid::glue_a_to_b(grid, &acc, glue_position, padding_extra, padding_color)
        })
    }

    // For [a, b, c, d, e] glues
    // (((a <- b) <- c) <- d) <- e
    pub fn glue_together_multiple_ref(
        grids: &[&Grid<CellType>],
        glue_position: GluePosition,
        padding_extra: i32,
        padding_color: CellType,
    ) -> Grid<CellType> {
        grids.iter().fold(Grid::empty(), |acc, grid| {
            Grid::glue_a_to_b(grid, &acc, glue_position, padding_extra, padding_color)
        })
    }

    pub fn glued_to(
        &self,
        other: &Grid<CellType>,
        glue_position: GluePosition,
        padding_extra: i32,
        padding_color: CellType,
    ) -> Grid<CellType> {
        Grid::glue_a_to_b(self, other, glue_position, padding_extra, padding_color)
    }

    /// Glues a to b at glue_position
    pub fn glue_a_to_b(
        a: &Grid<CellType>,
        b: &Grid<CellType>,
        glue_position: GluePosition,
        padding_extra: i32,
        padding_color: CellType,
    ) -> Grid<CellType> {
        if a.width == 0 || a.height == 0 {
            return b.clone();
        }
        if b.width == 0 || b.height == 0 {
            return a.clone();
        }

        let result = match glue_position {
            GluePosition::LeftTop => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::RightTop, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(a.width + padding_extra, 0, 0, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        0,
                        block_aligned_in_block(a.height, b.height, Alignment::Begin),
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::LeftCenter => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(
                        b,
                        a,
                        GluePosition::RightCenter,
                        padding_extra,
                        padding_color,
                    )
                } else {
                    let mut result = b.extended(a.width + padding_extra, 0, 0, 0, padding_color);
                    let blit_pos = Vec2i::new(0, block_centered_in_block(a.height, b.height));
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::LeftBottom => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(
                        b,
                        a,
                        GluePosition::RightBottom,
                        padding_extra,
                        padding_color,
                    )
                } else {
                    let mut result = b.extended(a.width + padding_extra, 0, 0, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        0,
                        block_aligned_in_block(a.height, b.height, Alignment::End),
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::TopLeft => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::BottomLeft, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, a.height + padding_extra, 0, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        block_aligned_in_block(a.width, b.width, Alignment::Begin),
                        0,
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::TopCenter => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(
                        b,
                        a,
                        GluePosition::BottomCenter,
                        padding_extra,
                        padding_color,
                    )
                } else {
                    let mut result = b.extended(0, a.height + padding_extra, 0, 0, padding_color);
                    let blit_pos = Vec2i::new(block_centered_in_block(a.width, b.width), 0);
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::TopRight => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(
                        b,
                        a,
                        GluePosition::BottomRight,
                        padding_extra,
                        padding_color,
                    )
                } else {
                    let mut result = b.extended(0, a.height + padding_extra, 0, 0, padding_color);
                    let blit_pos =
                        Vec2i::new(block_aligned_in_block(a.width, b.width, Alignment::End), 0);
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::RightTop => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::LeftTop, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, a.width + padding_extra, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        b.width + padding_extra,
                        block_aligned_in_block(a.height, b.height, Alignment::Begin),
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::RightCenter => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::LeftCenter, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, a.width + padding_extra, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        b.width + padding_extra,
                        block_centered_in_block(a.height, b.height),
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::RightBottom => {
                if a.height > b.height {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::LeftBottom, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, a.width + padding_extra, 0, padding_color);
                    let blit_pos = Vec2i::new(
                        b.width + padding_extra,
                        block_aligned_in_block(a.height, b.height, Alignment::End),
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::BottomLeft => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::TopLeft, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, 0, a.height + padding_extra, padding_color);
                    let blit_pos = Vec2i::new(
                        block_aligned_in_block(a.width, b.width, Alignment::Begin),
                        b.height + padding_extra,
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::BottomCenter => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::TopCenter, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, 0, a.height + padding_extra, padding_color);
                    let blit_pos = Vec2i::new(
                        block_centered_in_block(a.width, b.width),
                        b.height + padding_extra,
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
            GluePosition::BottomRight => {
                if a.width > b.width {
                    // NOTE: We use symmetry to avoid additional padding and adjusting
                    Grid::glue_a_to_b(b, a, GluePosition::TopRight, padding_extra, padding_color)
                } else {
                    let mut result = b.extended(0, 0, 0, a.height + padding_extra, padding_color);
                    let blit_pos = Vec2i::new(
                        block_aligned_in_block(a.width, b.width, Alignment::End),
                        b.height + padding_extra,
                    );
                    a.blit_to(&mut result, blit_pos, None);
                    result
                }
            }
        };

        result
    }

    pub fn floodfill(&mut self, start_x: i32, start_y: i32, fill_cell: CellType) {
        let start_cell = self.get_mut(start_x, start_y);
        if *start_cell == fill_cell {
            return;
        }
        *start_cell = fill_cell;

        let mut fill_stack = Vec::with_capacity((self.width + self.height) as usize);
        fill_stack.push(Vec2i::new(start_x, start_y));

        while let Some(center_pos) = fill_stack.pop() {
            for delta_y in -1..=1 {
                for delta_x in -1..=1 {
                    let cell_pos = center_pos + Vec2i::new(delta_x, delta_y);
                    if !self.contains_point(cell_pos.x, cell_pos.y) {
                        continue;
                    }

                    let cell = self.get_mut(cell_pos.x, cell_pos.y);
                    if *cell == fill_cell {
                        continue;
                    }

                    *cell = fill_cell;
                    fill_stack.push(cell_pos);
                }
            }
        }
    }

    /// NOTE: This may return a smaller grid than given in the rect if the rect is partly outside
    ///       the grid
    pub fn to_subgrid(&self, rect: Recti) -> Option<Grid<CellType>> {
        if let Some(intersection) = self.rect().clipped_by(rect) {
            let mut result =
                Grid::<CellType>::new(intersection.width() as u32, intersection.height() as u32);
            let dest_rect = result.rect();
            Grid::copy_region(&self, intersection, &mut result, dest_rect, None);

            Some(result)
        } else {
            None
        }
    }

    pub fn to_segments(
        &self,
        segment_width: i32,
        segment_height: i32,
    ) -> (Vec<Grid<CellType>>, Vec<Vec2i>) {
        assert!(segment_width > 0);
        assert!(segment_height > 0);

        let segment_count_x = self.width / segment_width
            + if self.width % segment_width == 0 {
                0
            } else {
                1
            };
        let segment_count_y = self.height / segment_height
            + if self.height % segment_height == 0 {
                0
            } else {
                1
            };

        let mut segment_images = Vec::new();
        let mut segment_coordinates = Vec::new();
        for y in 0..segment_count_y {
            for x in 0..segment_count_x {
                let pos = Vec2i::new(x, y);
                let subgrid = self
                    .to_subgrid(Recti::from_xy_width_height(
                        x * segment_width,
                        y * segment_height,
                        segment_width,
                        segment_height,
                    ))
                    .expect(&format!("Segment ({},{}) was empty", x, y));
                segment_images.push(subgrid);
                segment_coordinates.push(pos);
            }
        }

        (segment_images, segment_coordinates)
    }
}
