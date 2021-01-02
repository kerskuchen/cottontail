use super::*;
use log;

use std::collections::HashMap;

pub type ResourceName = String;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub resource_name: ResourceName,
    pub original_filepath: String,
    pub samplerate_hz: usize,
    pub framecount: usize,
    pub channelcount: usize,
    pub compression_quality: usize,
    pub loopsection_start_frameindex: Option<usize>,
    pub loopsection_framecount: Option<usize>,
}

impl AudioMetadata {
    pub fn clone_with_new_sample_rate(&self, audio_sample_rate_hz: usize) -> AudioMetadata {
        if self.samplerate_hz == audio_sample_rate_hz {
            return self.clone();
        }

        let sample_rate_ratio = audio_sample_rate_hz as f64 / self.samplerate_hz as f64;

        let mut result = self.clone();
        result.samplerate_hz = audio_sample_rate_hz;
        result.framecount = (self.framecount as f64 * sample_rate_ratio).ceil() as usize;
        if result.loopsection_framecount.is_some() {
            result.loopsection_start_frameindex = Some(
                (self.loopsection_start_frameindex.unwrap() as f64 * sample_rate_ratio).ceil()
                    as usize,
            );
        }
        if result.loopsection_framecount.is_some() {
            result.loopsection_framecount = Some(
                (self.loopsection_framecount.unwrap() as f64 * sample_rate_ratio).ceil() as usize,
            );
        }

        result
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AudioResources {
    pub resource_sample_rate_hz: usize,
    pub names: Vec<ResourceName>,
    pub metadata: HashMap<String, AudioMetadata>,
    pub metadata_original: HashMap<String, AudioMetadata>,
    pub content: HashMap<String, Vec<u8>>,
}

impl AudioResources {
    pub fn new(resource_sample_rate_hz: usize) -> AudioResources {
        AudioResources {
            resource_sample_rate_hz,
            names: Vec::new(),
            metadata: HashMap::new(),
            metadata_original: HashMap::new(),
            content: HashMap::new(),
        }
    }

    pub fn add_audio_resource(
        &mut self,
        name: ResourceName,
        metadata_original: AudioMetadata,
        metadata: AudioMetadata,
        content: Vec<u8>,
    ) {
        assert!(!self.metadata.contains_key(&name));
        self.names.push(name.clone());
        self.metadata.insert(name.clone(), metadata);
        self.metadata_original
            .insert(name.clone(), metadata_original);
        self.content.insert(name, content);
    }
}
#[derive(Default)]
pub struct GameAssets {
    assets_folder: String,
    animations: HashMap<String, Animation<Sprite>>,
    animations_3d: HashMap<String, Animation<Sprite3D>>,
    sprites_3d: HashMap<String, Sprite3D>,
    fonts: HashMap<String, SpriteFont>,
    atlas: SpriteAtlas,

    pub audio: AudioResources,

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
        result.audio = self.audio.clone();

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
                let index_filepath = path_join(&self.assets_folder, "index.txt");
                let index_loader = self
                    .files_loaders
                    .entry(index_filepath.clone())
                    .or_insert(Fileloader::new(&index_filepath).unwrap());

                if let Some(index_content) = index_loader
                    .poll()
                    .expect("Could not load resource index file")
                {
                    log::debug!("Loaded index file '{}'", index_filepath);
                    self.files_list = String::from_utf8_lossy(&index_content)
                        .lines()
                        .filter(|&filepath| !filepath.is_empty())
                        .map(|filepath| filepath.to_owned())
                        .collect();

                    self.files_loaders.clear();
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
                        log::debug!("Loaded resource file '{}'", filepath);
                    }
                }

                if self.files_loaders.is_empty() {
                    assert!(self.files_bindata.len() == self.files_list.len());

                    self.audio = self.load_audio();
                    log::info!("Loaded audio resources");

                    self.files_loading_stage = AssetLoadingStage::Finished;
                    log::info!("Finished loading asset files");
                    true
                } else {
                    false
                }
            }
            AssetLoadingStage::Finished => true,
        }
    }

    #[must_use]
    pub fn load_graphics(&mut self) -> bool {
        if !self.load_files() {
            return false;
        }

        if self.atlas.textures.is_empty() {
            self.atlas = self.load_atlas();
            self.animations = self.load_animations();
            self.fonts = self.load_fonts();
            self.animations_3d = self.load_animations_3d();
            self.sprites_3d = self.load_sprites_3d();
            log::info!("Loaded graphic resources");
        }

        return true;
    }

    pub fn load_audiorecordings(&self) -> HashMap<String, AudioRecording> {
        assert!(self.files_loading_stage == AssetLoadingStage::Finished);

        let mut audiorecordings = HashMap::new();

        for (resource_name, metadata) in self.audio.metadata.iter() {
            let ogg_data = &self.audio.content[resource_name];
            let recording = AudioRecording::new_from_ogg_stream_with_loopsection(
                metadata.resource_name.clone(),
                metadata.framecount,
                ogg_data.clone(),
                metadata.loopsection_start_frameindex.unwrap_or(0),
                metadata
                    .loopsection_framecount
                    .unwrap_or(metadata.framecount),
            )
            .unwrap_or_else(|error| {
                panic!("Cannot create Audiorecording from resource '{}'", error)
            });
            audiorecordings.insert(resource_name.clone(), recording);
        }

        log::info!("Loaded audio recordings");
        audiorecordings
    }

    pub fn get_atlas_textures(&self) -> &[Bitmap] {
        &self.atlas.textures
    }

    pub fn get_anim(&self, animation_name: &str) -> &Animation<Sprite> {
        self.animations
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_anim_3d(&self, animation_name: &str) -> &Animation<Sprite3D> {
        self.animations_3d
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_font(&self, font_name: &str) -> &SpriteFont {
        self.fonts
            .get(font_name)
            .unwrap_or_else(|| panic!("Could not find font '{}'", font_name))
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
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
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
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_audio(&self) -> AudioResources {
        let filepath = path_join(&self.assets_folder, "audio_wasm.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_audio(&self) -> AudioResources {
        let filepath = path_join(&self.assets_folder, "audio.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_sprites(&self) -> HashMap<String, Sprite> {
        let filepath = path_join(&self.assets_folder, "sprites.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_sprites_3d(&self) -> HashMap<String, Sprite3D> {
        let filepath = path_join(&self.assets_folder, "sprites_3d.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_animations(&self) -> HashMap<String, Animation<Sprite>> {
        let filepath = path_join(&self.assets_folder, "animations.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_animations_3d(&self) -> HashMap<String, Animation<Sprite3D>> {
        let filepath = path_join(&self.assets_folder, "animations_3d.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_fonts(&self) -> HashMap<String, SpriteFont> {
        let filepath = path_join(&self.assets_folder, "fonts.data");
        deserialize_from_binary(&self.files_bindata[&filepath])
    }

    fn load_atlas(&self) -> SpriteAtlas {
        let textures_list_filepath = path_join(&self.assets_folder, "atlas.data");
        let textures_list: Vec<String> =
            deserialize_from_binary(&self.files_bindata[&textures_list_filepath]);

        let mut textures = Vec::new();
        for texture_filepath_relative in &textures_list {
            let texture_filepath = path_join(&self.assets_folder, texture_filepath_relative);
            textures.push(
                Bitmap::from_png_data(&self.files_bindata[&texture_filepath])
                    .unwrap_or_else(|error| panic!("Could load texture '{}'", texture_filepath)),
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
