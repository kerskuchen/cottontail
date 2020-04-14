use super::bitmap::Bitmap;
use super::color::PixelRGBA;
use super::math::Vec2i;
use serde_derive::Serialize;

use super::IndexMap;

use rect_packer;

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BitmapAtlasPosition {
    pub atlas_texture_index: u32,
    pub atlas_texture_pixel_offset: Vec2i,
}

/// An atlaspacker that can grow in size
pub struct BitmapAtlas {
    pub atlas_texture: Bitmap,
    pub rect_packer: rect_packer::DensePacker,
    pub sprite_positions: IndexMap<String, Vec2i>,
}

impl BitmapAtlas {
    pub fn new(atlas_texture_size_initial: i32) -> BitmapAtlas {
        assert!(atlas_texture_size_initial > 0);

        BitmapAtlas {
            atlas_texture: Bitmap::new(
                atlas_texture_size_initial as u32,
                atlas_texture_size_initial as u32,
            ),
            rect_packer: rect_packer::DensePacker::new(
                atlas_texture_size_initial,
                atlas_texture_size_initial,
            ),
            sprite_positions: IndexMap::new(),
        }
    }

    pub fn finish(self) -> (Bitmap, IndexMap<String, Vec2i>) {
        (self.atlas_texture, self.sprite_positions)
    }

    pub fn pack_bitmap(&mut self, name: &str, image: &Bitmap) -> Option<Vec2i> {
        if let Some(rect) = self.rect_packer.pack(image.width, image.height, false) {
            let position = Vec2i::new(rect.x, rect.y);
            image.blit_to(&mut self.atlas_texture, position, None);
            self.sprite_positions.insert(name.to_owned(), position);
            Some(position)
        } else {
            None
        }
    }

    /// NOTE: Resizes by squaring the current size
    pub fn pack_bitmap_with_resize(&mut self, name: &str, image: &Bitmap) -> Option<Vec2i> {
        if let Some(pos) = self.pack_bitmap(name, image) {
            return Some(pos);
        }

        // NOTE: At this point our image did not fit in our atlas textures, so we resize it
        let texture_size = self.atlas_texture.width;
        self.atlas_texture
            .extend(0, 0, texture_size, texture_size, PixelRGBA::transparent());
        self.rect_packer.resize(2 * texture_size, 2 * texture_size);

        self.pack_bitmap_with_resize(name, image)
    }
}

/// An atlaspacker that can have multiple fixed size atlas textures
pub struct BitmapMultiAtlas {
    pub atlas_texture_size: i32,
    pub atlas_packers: Vec<BitmapAtlas>,
    pub sprite_positions: IndexMap<String, BitmapAtlasPosition>,
}

impl BitmapMultiAtlas {
    pub fn new(atlas_texture_size: i32) -> BitmapMultiAtlas {
        assert!(atlas_texture_size > 0);

        BitmapMultiAtlas {
            atlas_texture_size,
            atlas_packers: vec![BitmapAtlas::new(atlas_texture_size)],
            sprite_positions: IndexMap::new(),
        }
    }

    pub fn pack_bitmap(&mut self, name: &str, image: &Bitmap) -> Option<BitmapAtlasPosition> {
        for (atlas_index, packer) in self.atlas_packers.iter_mut().enumerate() {
            if let Some(position) = packer.pack_bitmap(name, image) {
                let atlas_position = BitmapAtlasPosition {
                    atlas_texture_index: atlas_index as u32,
                    atlas_texture_pixel_offset: position,
                };
                self.sprite_positions
                    .insert(name.to_owned(), atlas_position);
                return Some(atlas_position);
            }
        }
        None
    }

    pub fn pack_bitmap_allow_growing(
        &mut self,
        sprite_name: &str,
        image: &Bitmap,
    ) -> BitmapAtlasPosition {
        if let Some(atlas_position) = self.pack_bitmap(sprite_name, image) {
            return atlas_position;
        }

        // NOTE: At this point our image did not fit in any of the existing atlas textures, so we
        //       create a new atlas texture and try again
        self.atlas_packers
            .push(BitmapAtlas::new(self.atlas_texture_size));
        if let Some(atlas_position) = self.pack_bitmap(sprite_name, image) {
            atlas_position
        } else {
            panic!(
                "Could not pack image with dimensions {}x{} into atlas with dimensions {}x{}",
                image.width, image.height, self.atlas_texture_size, self.atlas_texture_size
            )
        }
    }

    pub fn finish(self) -> (Vec<Bitmap>, IndexMap<String, BitmapAtlasPosition>) {
        let atlas_textures = self
            .atlas_packers
            .into_iter()
            .map(|packer| packer.atlas_texture)
            .collect();

        (atlas_textures, self.sprite_positions)
    }
}
