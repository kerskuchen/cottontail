use super::bitmap::Bitmap;
use super::math::Vec2i;
use serde_derive::Serialize;

use super::IndexMap;

use rect_packer;

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct AtlasPosition {
    pub atlas_texture_index: u32,
    pub atlas_texture_pixel_offset: Vec2i,
}
pub struct AtlasPacker {
    atlas_texture_size: i32,
    atlas_textures: Vec<Bitmap>,
    rect_packers: Vec<rect_packer::DensePacker>,
    sprite_positions: IndexMap<String, AtlasPosition>,
}

impl AtlasPacker {
    pub fn new(atlas_texture_size: i32) -> AtlasPacker {
        assert!(atlas_texture_size > 0);

        AtlasPacker {
            atlas_texture_size,
            atlas_textures: vec![Bitmap::new(
                atlas_texture_size as u32,
                atlas_texture_size as u32,
            )],
            rect_packers: vec![rect_packer::DensePacker::new(
                atlas_texture_size,
                atlas_texture_size,
            )],
            sprite_positions: IndexMap::new(),
        }
    }

    pub fn pack_bitmap(&mut self, sprite_name: &str, image: &Bitmap) {
        if self.try_pack_bitmap(sprite_name, image) {
            return;
        }

        // NOTE: At this point our image did not fit in any of the existing atlas textures, so we
        //       create a new atlas texture and try again
        self.rect_packers.push(rect_packer::DensePacker::new(
            self.atlas_texture_size,
            self.atlas_texture_size,
        ));
        self.atlas_textures.push(Bitmap::new(
            self.atlas_texture_size as u32,
            self.atlas_texture_size as u32,
        ));
        if !self.try_pack_bitmap(sprite_name, image) {
            panic!(
                "Could not pack image with dimensions {}x{} into atlas with dimensions {}x{}",
                image.width, image.height, self.atlas_texture_size, self.atlas_texture_size
            );
        }
    }

    pub fn finish(self) -> (Vec<Bitmap>, IndexMap<String, AtlasPosition>) {
        (self.atlas_textures, self.sprite_positions)
    }

    /// Returns true if found a spot to pack the given bitmap
    pub fn try_pack_bitmap(&mut self, name: &str, image: &Bitmap) -> bool {
        for (atlas_index, (packer, texture)) in self
            .rect_packers
            .iter_mut()
            .zip(self.atlas_textures.iter_mut())
            .enumerate()
        {
            if let Some(rect) = packer.pack(image.width, image.height, false) {
                let position = Vec2i::new(rect.x, rect.y);
                image.blit_to(texture, position, None);

                self.sprite_positions.insert(
                    name.to_owned(),
                    AtlasPosition {
                        atlas_texture_index: atlas_index as u32,
                        atlas_texture_pixel_offset: position,
                    },
                );

                return true;
            }
        }

        false
    }
}
