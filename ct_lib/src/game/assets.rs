use super::*;

use std::{collections::HashMap, rc::Rc};

#[derive(Default, Clone)]
pub struct GameAssets {
    pub assets_folder: String,
    animations: HashMap<String, Animation<Sprite>>,
    animations_3d: HashMap<String, Animation<Sprite3D>>,
    sprites_3d: HashMap<String, Sprite3D>,
    fonts: HashMap<String, SpriteFont>,
    atlas: SpriteAtlas,
}

impl GameAssets {
    pub fn new(assets_folder: &str) -> GameAssets {
        let mut result = GameAssets::default();
        result.assets_folder = assets_folder.to_string();
        result
    }

    pub fn load_graphics(&mut self) {
        self.atlas = load_atlas(&self.assets_folder);
        self.animations = load_animations(&self.assets_folder);
        self.fonts = load_fonts(&self.assets_folder);
        self.animations_3d = load_animations_3d(&self.assets_folder);
        self.sprites_3d = load_sprites_3d(&self.assets_folder);
    }

    pub fn get_atlas_textures(&self) -> &[Bitmap] {
        &self.atlas.textures
    }

    pub fn get_anim(&self, animation_name: &str) -> &Animation<Sprite> {
        self.animations
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
    }

    pub fn get_anim_3d(&self, animation_name: &str) -> &Animation<Sprite3D> {
        self.animations_3d
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
    }

    pub fn get_font(&self, font_name: &str) -> &SpriteFont {
        self.fonts
            .get(font_name)
            .expect(&format!("Could not find font '{}'", font_name))
    }

    pub fn get_sprite(&self, sprite_name: &str) -> &Sprite {
        if let Some(result) = self.atlas.sprites.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            self.atlas
                .sprites
                .get(&format!("{}.0", sprite_name))
                .expect(&format!(
                    "Sprite with name '{}' does not exist",
                    sprite_name
                ))
        }
    }

    pub fn get_sprite_3d(&self, sprite_name: &str) -> &Sprite3D {
        if let Some(result) = self.sprites_3d.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            self.sprites_3d
                .get(&format!("{}.0", sprite_name))
                .expect(&format!(
                    "Sprite with name '{}' does not exist",
                    sprite_name
                ))
        }
    }

    pub fn debug_get_sprite_as_bitmap(&self, sprite_name: &str) -> Bitmap {
        self.atlas.debug_get_bitmap_for_sprite(sprite_name)
    }

    pub fn debug_save_sprite_as_png(&self, sprite_name: &str, filepath: &str) {
        let sprite_bitmap = self.debug_get_sprite_as_bitmap(sprite_name);
        Bitmap::write_to_png_file(&sprite_bitmap, filepath);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Asset loading

fn load_sprites(assets_folder: &str) -> HashMap<String, Sprite> {
    let sprites_filepath = system::path_join(assets_folder, "sprites.data");
    super::deserialize_from_file_binary(&sprites_filepath)
}

fn load_sprites_3d(assets_folder: &str) -> HashMap<String, Sprite3D> {
    let sprites_filepath = system::path_join(assets_folder, "sprites_3d.data");
    super::deserialize_from_file_binary(&sprites_filepath)
}

fn load_animations(assets_folder: &str) -> HashMap<String, Animation<Sprite>> {
    let animations_filepath = system::path_join(assets_folder, "animations.data");
    super::deserialize_from_file_binary(&animations_filepath)
}

fn load_animations_3d(assets_folder: &str) -> HashMap<String, Animation<Sprite3D>> {
    let animations_filepath = system::path_join(assets_folder, "animations_3d.data");
    super::deserialize_from_file_binary(&animations_filepath)
}

fn load_atlas(assets_folder: &str) -> SpriteAtlas {
    let textures_list_filepath = system::path_join(assets_folder, "atlas.data");
    let textures_list: Vec<String> = super::deserialize_from_file_binary(&textures_list_filepath);

    let mut textures = Vec::new();
    for texture_filepath_relative in &textures_list {
        let texture_filepath = system::path_join(assets_folder, texture_filepath_relative);
        textures.push(Bitmap::from_png_file_or_panic(&texture_filepath));
    }

    let sprites = load_sprites(assets_folder);

    let mut atlas = SpriteAtlas::new(textures, sprites);

    // Make sprites out of the atlas pages themselves for debug purposes
    for page_index in 0..atlas.textures.len() {
        let sprite_name = format!("debug_sprite_whole_page_{}", page_index);
        atlas.add_sprite_for_region(
            sprite_name,
            page_index as TextureIndex,
            Recti::from_width_height(atlas.textures_size as i32, atlas.textures_size as i32),
            Vec2i::zero(),
            true,
        );
    }

    atlas
}

fn load_fonts(assets_folder: &str) -> HashMap<String, SpriteFont> {
    let fonts_filepath = system::path_join(assets_folder, "fonts.data");
    super::deserialize_from_file_binary(&fonts_filepath)
}

pub fn load_audiorecordings_mono(assets_folder: &str) -> HashMap<String, AudioBufferMono> {
    let mut audiorecordings = HashMap::new();

    let wav_filepaths = system::collect_files_by_extension_recursive(assets_folder, ".wav");
    for wav_filepath in &wav_filepaths {
        let mut wav_file = audrey::open(&wav_filepath).expect(&format!(
            "Could not open audio file for reading: '{}'",
            wav_filepath
        ));
        let name = system::path_to_filename_without_extension(wav_filepath);
        let sample_rate_hz = wav_file.description().sample_rate() as usize;
        let samples: Vec<AudioSample> = wav_file.samples().map(Result::unwrap).collect();
        let samplecount = samples.len();
        let recording = AudioBufferMono {
            name: name.clone(),
            sample_rate_hz,
            samples,
            loop_start_sampleindex: 0,
            loop_end_sampleindex: samplecount - 1,
        };
        audiorecordings.insert(name, recording);
    }

    audiorecordings
}

pub fn load_audiorecordings_stereo(assets_folder: &str) -> HashMap<String, AudioBufferStereo> {
    // TODO
    HashMap::new()
}
