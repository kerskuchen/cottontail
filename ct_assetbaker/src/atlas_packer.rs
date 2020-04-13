use super::{AssetAtlas, SpritePosition, Spritename, TextureIndex};

use ct_lib::bitmap::*;
use ct_lib::math::*;
use ct_lib::system;

use indexmap::IndexMap;

use rect_packer;
struct AtlasPacker {
    atlas_texture_size: i32,
    atlas_textures: Vec<Bitmap>,
    rect_packers: Vec<rect_packer::DensePacker>,
    sprite_positions: IndexMap<Spritename, SpritePosition>,
}

impl AtlasPacker {
    fn new(atlas_texture_size: i32) -> AtlasPacker {
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

    fn pack_bitmap(&mut self, sprite_name: &str, image: &Bitmap, trim_bottom_right: bool) {
        if self.try_pack_bitmap(sprite_name, image, trim_bottom_right) {
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
        if !self.try_pack_bitmap(sprite_name, image, trim_bottom_right) {
            panic!(
                "Could not pack image with dimensions {}x{} into atlas with dimensions {}x{}",
                image.width, image.height, self.atlas_texture_size, self.atlas_texture_size
            );
        }
    }

    fn finish(self) -> (Vec<Bitmap>, IndexMap<Spritename, SpritePosition>) {
        (self.atlas_textures, self.sprite_positions)
    }

    /// Returns true if found a spot to pack the given bitmap
    fn try_pack_bitmap(
        &mut self,
        sprite_name: &str,
        image: &Bitmap,
        trim_bottom_right: bool,
    ) -> bool {
        for (atlas_index, (packer, texture)) in self
            .rect_packers
            .iter_mut()
            .zip(self.atlas_textures.iter_mut())
            .enumerate()
        {
            if let Some(rect) = packer.pack(image.width, image.height, false) {
                let position = Vec2i::new(rect.x, rect.y);

                if trim_bottom_right == true {
                    image
                        .trimmed(false, false, true, true, PixelRGBA::transparent())
                        .blit_to(texture, position, None);
                } else {
                    image.blit_to(texture, position, None);
                };

                self.sprite_positions.insert(
                    sprite_name.to_owned(),
                    SpritePosition {
                        atlas_texture_index: atlas_index as TextureIndex,
                        atlas_texture_pixel_offset: position,
                    },
                );

                return true;
            }
        }

        false
    }
}

pub fn atlas_create_from_pngs(
    source_dir: &str,
    output_dir: &str,
    atlas_texture_size: u32,
) -> AssetAtlas {
    let sprite_imagepaths = system::collect_files_by_extension_recursive(source_dir, ".png");

    // Pack sprites
    let (atlas_textures, result_sprite_positions) = {
        let mut packer = AtlasPacker::new(atlas_texture_size as i32);
        for image_path in sprite_imagepaths.into_iter() {
            let image = Bitmap::create_from_png_file(&image_path);
            let sprite_name = system::path_without_extension(&image_path)
                .replace(&format!("{}/", source_dir), "");
            packer.pack_bitmap(&sprite_name, &image, true);
        }
        packer.finish()
    };

    // Write textures to disk
    let result_texture_imagepaths = {
        let atlas_path_without_extension = system::path_join(output_dir, "atlas");
        let mut texture_imagepaths = Vec::new();
        for (index, atlas_texture) in atlas_textures.iter().enumerate() {
            let texture_path = format!("{}-{}.png", atlas_path_without_extension, index);
            Bitmap::write_to_png_file(&atlas_texture.to_premultiplied(), &texture_path);
            texture_imagepaths.push(texture_path);
        }
        texture_imagepaths
    };

    AssetAtlas {
        texture_size: atlas_texture_size,
        texture_count: result_texture_imagepaths.len() as u32,
        texture_imagepaths: result_texture_imagepaths,
        sprite_positions: result_sprite_positions,
    }
}
