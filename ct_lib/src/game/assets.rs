use system::Fileloader;

use super::*;

use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AssetLoadingStage {
    Start,
    LoadingFiles,
    Finished,
}

impl Default for AssetLoadingStage {
    fn default() -> Self {
        AssetLoadingStage::Start
    }
}

#[derive(Default)]
pub struct GameAssets {
    pub assets_folder: String,
    animations: HashMap<String, Animation<Sprite>>,
    animations_3d: HashMap<String, Animation<Sprite3D>>,
    sprites_3d: HashMap<String, Sprite3D>,
    fonts: HashMap<String, SpriteFont>,
    atlas: SpriteAtlas,

    files_loading_stage: AssetLoadingStage,
    files_list: Vec<String>,
    files_bindata: HashMap<String, Vec<u8>>,
    files_loaders: HashMap<String, Fileloader>,
}

impl Clone for GameAssets {
    fn clone(&self) -> Self {
        assert!(self.files_loading_stage != AssetLoadingStage::LoadingFiles);
        assert!(self.files_loaders.is_empty());

        let mut result = GameAssets::default();
        result.assets_folder = self.assets_folder.clone();
        result.animations = self.animations.clone();
        result.animations_3d = self.animations_3d.clone();
        result.sprites_3d = self.sprites_3d.clone();
        result.fonts = self.fonts.clone();
        result.atlas = self.atlas.clone();
        result.files_loading_stage = self.files_loading_stage.clone();
        result.files_list = self.files_list.clone();
        result.files_bindata = self.files_bindata.clone();

        result
    }
}

impl GameAssets {
    pub fn new(assets_folder: &str) -> GameAssets {
        let mut result = GameAssets::default();
        result.assets_folder = assets_folder.to_string();
        result
    }

    pub fn load_files(&mut self) -> bool {
        match self.files_loading_stage {
            AssetLoadingStage::Start => {
                let index_filepath = system::path_join(&self.assets_folder, "index.txt");
                let index_loader = self
                    .files_loaders
                    .entry(index_filepath.clone())
                    .or_insert(Fileloader::new(&index_filepath).unwrap());

                if let Some(index_content) = index_loader
                    .poll()
                    .expect("Could not load resource index file")
                {
                    self.files_list = String::from_utf8_lossy(&index_content)
                        .lines()
                        .filter(|&filepath| !filepath.is_empty())
                        .map(|filepath| filepath.to_owned())
                        .collect();

                    for filepath in &self.files_list {
                        self.files_loaders
                            .insert(filepath.clone(), Fileloader::new(&filepath).unwrap());
                    }
                    self.files_loading_stage = AssetLoadingStage::LoadingFiles;
                }

                false
            }
            AssetLoadingStage::LoadingFiles => {
                // Remove loaders for which we already saved the bindata
                for filepath in self.files_bindata.keys() {
                    self.files_loaders.remove(filepath);
                }

                // Poll loaders
                for (filepath, loader) in self.files_loaders.iter_mut() {
                    if let Some(content) = loader.poll().unwrap() {
                        self.files_bindata.insert(filepath.clone(), content);
                    }
                }

                if self.files_loaders.is_empty() {
                    assert!(self.files_bindata.len() == self.files_list.len());
                    self.files_loading_stage = AssetLoadingStage::Finished;
                    true
                } else {
                    false
                }
            }
            AssetLoadingStage::Finished => true,
        }
    }

    pub fn load_graphics(&mut self) -> bool {
        if !self.load_files() {
            return false;
        }

        self.atlas = self.load_atlas();
        self.animations = self.load_animations();
        self.fonts = self.load_fonts();
        self.animations_3d = self.load_animations_3d();
        self.sprites_3d = self.load_sprites_3d();

        return true;
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

    fn load_sprites(&self) -> HashMap<String, Sprite> {
        let filepath = system::path_join(&self.assets_folder, "sprites.data");
        super::deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_sprites_3d(&self) -> HashMap<String, Sprite3D> {
        let filepath = system::path_join(&self.assets_folder, "sprites_3d.data");
        super::deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_animations(&self) -> HashMap<String, Animation<Sprite>> {
        let filepath = system::path_join(&self.assets_folder, "animations.data");
        super::deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_animations_3d(&self) -> HashMap<String, Animation<Sprite3D>> {
        let filepath = system::path_join(&self.assets_folder, "animations_3d.data");
        super::deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_fonts(&self) -> HashMap<String, SpriteFont> {
        let filepath = system::path_join(&self.assets_folder, "fonts.data");
        super::deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_atlas(&self) -> SpriteAtlas {
        let textures_list_filepath = system::path_join(&self.assets_folder, "atlas.data");
        let textures_list: Vec<String> =
            super::deserialize_from_binary(&self.files_bindata[&textures_list_filepath]);

        let mut textures = Vec::new();
        for texture_filepath_relative in &textures_list {
            let texture_filepath =
                system::path_join(&self.assets_folder, texture_filepath_relative);
            textures.push(
                Bitmap::from_png_data(&self.files_bindata[&texture_filepath])
                    .expect(&format!("Could load texture '{}'", texture_filepath)),
            );
        }

        let sprites = self.load_sprites();
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

    pub fn debug_get_sprite_as_bitmap(&self, sprite_name: &str) -> Bitmap {
        self.atlas.debug_get_bitmap_for_sprite(sprite_name)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn debug_save_sprite_as_png(&self, sprite_name: &str, filepath: &str) {
        let sprite_bitmap = self.debug_get_sprite_as_bitmap(sprite_name);
        Bitmap::write_to_png_file(&sprite_bitmap, filepath);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Asset loading

#[cfg(target_arch = "wasm32")]
pub fn load_audiorecordings_mono(assets_folder: &str) -> HashMap<String, AudioBufferMono> {
    todo!()
}

#[cfg(not(target_arch = "wasm32"))]
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
            loopsection_start_sampleindex: 0,
            loopsection_samplecount: samplecount,
        };
        audiorecordings.insert(name, recording);
    }

    audiorecordings
}

pub fn load_audiorecordings_stereo(_assets_folder: &str) -> HashMap<String, AudioBufferStereo> {
    let TODO = true;
    HashMap::new()
}
