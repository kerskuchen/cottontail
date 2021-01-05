use super::*;
use indexmap::IndexMap;
use log;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
    pub metadata: HashMap<ResourceName, AudioMetadata>,
    pub metadata_original: HashMap<ResourceName, AudioMetadata>,
    pub recordings_ogg_data: HashMap<ResourceName, Vec<u8>>,
}

impl AudioResources {
    pub fn new(resource_sample_rate_hz: usize) -> AudioResources {
        AudioResources {
            resource_sample_rate_hz,
            names: Vec::new(),
            metadata: HashMap::new(),
            metadata_original: HashMap::new(),
            recordings_ogg_data: HashMap::new(),
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
        self.recordings_ogg_data.insert(name, content);
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GraphicsResources {
    pub animations: IndexMap<ResourceName, Animation<Sprite>>,
    pub animations_3d: IndexMap<ResourceName, Animation<Sprite3D>>,
    pub sprites: IndexMap<ResourceName, Sprite>,
    pub sprites_3d: IndexMap<ResourceName, Sprite3D>,
    pub fonts: IndexMap<ResourceName, SpriteFont>,

    pub textures_png_data: Vec<Vec<u8>>,
    pub textures_dimension: u32,
}

#[derive(Default)]
pub struct GameAssets {
    assets_folder: String,

    pub audio: AudioResources,
    pub graphics: GraphicsResources,
    pub content: HashMap<String, Vec<u8>>,

    decoded_audio_recordings: HashMap<ResourceName, Rc<RefCell<AudioRecording>>>,
    decoded_bitmap_textures: Vec<Rc<RefCell<Bitmap>>>,

    files_loading_stage: AssetLoadingStage,
    files_loaders: HashMap<String, Fileloader>,
}

impl Clone for GameAssets {
    fn clone(&self) -> Self {
        assert!(self.files_loading_stage != AssetLoadingStage::LoadingFiles);
        assert!(self.files_loaders.is_empty());

        let mut result = GameAssets::default();
        result.assets_folder = self.assets_folder.clone();

        result.files_loading_stage = self.files_loading_stage.clone();
        result.files_loaders = HashMap::new();

        result.audio = self.audio.clone();
        result.graphics = self.graphics.clone();
        result.content = self.content.clone();

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
                let graphics_filepath = path_join(&self.assets_folder, "graphics.data");
                self.files_loaders.insert(
                    graphics_filepath.clone(),
                    Fileloader::new(&graphics_filepath).unwrap(),
                );

                let content_filepath = path_join(&self.assets_folder, "content.data");
                self.files_loaders.insert(
                    content_filepath.clone(),
                    Fileloader::new(&content_filepath).unwrap(),
                );

                #[cfg(target_arch = "wasm32")]
                let audio_filepath = path_join(&self.assets_folder, "audio_wasm.data");
                #[cfg(not(target_arch = "wasm32"))]
                let audio_filepath = path_join(&self.assets_folder, "audio.data");
                self.files_loaders.insert(
                    audio_filepath.clone(),
                    Fileloader::new(&audio_filepath).unwrap(),
                );

                self.files_loading_stage = AssetLoadingStage::LoadingFiles;
                false
            }
            AssetLoadingStage::LoadingFiles => {
                let mut finished_loaders = Vec::new();

                // Poll file loaders
                for (filepath, loader) in self.files_loaders.iter_mut() {
                    let poll_result = loader.poll().unwrap_or_else(|error| {
                        panic!("Failed to get file status on '{}': {}", filepath, error)
                    });

                    if let Some(content) = poll_result {
                        log::debug!("Loaded resource file '{}'", filepath);

                        if filepath == &path_join(&self.assets_folder, "graphics.data") {
                            self.graphics = bincode::deserialize(&content)
                                .expect("Could not deserialize 'graphics.data' (file corrupt?)");
                            log::info!("Loaded graphics resources");
                        } else if filepath == &path_join(&self.assets_folder, "audio.data")
                            || filepath == &path_join(&self.assets_folder, "audio_wasm.data")
                        {
                            self.audio = bincode::deserialize(&content)
                                .expect("Could not deserialize 'audio.data' (file corrupt?)");
                            log::info!("Loaded audio resources");
                        } else if filepath == &path_join(&self.assets_folder, "content.data") {
                            self.content = bincode::deserialize(&content)
                                .expect("Could not deserialize 'content.data' (file corrupt?)");
                            log::info!("Loaded content resources");
                        } else {
                            unreachable!("Loaded unknown file '{}'", filepath);
                        }

                        // Mark the loader for removal
                        finished_loaders.push(filepath.clone());
                    }
                }

                // Remove finished file loaders
                for path in &finished_loaders {
                    self.files_loaders.remove(path);
                }

                if self.files_loaders.is_empty() {
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
    pub fn decode_assets(&mut self) -> bool {
        if !self.load_files() {
            return false;
        }

        if self.decoded_audio_recordings.is_empty() {
            self.decode_audiorecordings();
        }
        if self.decoded_bitmap_textures.is_empty() {
            self.decode_atlas_textures();
        }

        return true;
    }

    pub fn get_atlas_textures(&self) -> &Vec<Rc<RefCell<Bitmap>>> {
        assert!(self.files_loading_stage == AssetLoadingStage::Finished);
        &self.decoded_bitmap_textures
    }

    fn decode_atlas_textures(&mut self) {
        assert!(self.files_loading_stage == AssetLoadingStage::Finished);
        self.decoded_bitmap_textures = self
            .graphics
            .textures_png_data
            .iter()
            .enumerate()
            .map(|(index, png_data)| {
                let bitmap = Bitmap::from_png_data(png_data).unwrap_or_else(|error| {
                    panic!("Could not decode atlas texture ({}): {}", index, error)
                });
                assert!(
                    bitmap.width == bitmap.height,
                    "Loaded atlas texture ({}) needs to have same width and height - got {}x{}",
                    index,
                    bitmap.width,
                    bitmap.height,
                );
                assert!(
                    bitmap.width == self.graphics.textures_dimension as i32,
                    "Loaded atlas texture ({}) dimension does not match ours - expectet {} got {}",
                    index,
                    self.graphics.textures_dimension,
                    bitmap.width,
                );
                Rc::new(RefCell::new(bitmap))
            })
            .collect();

        // Make sprites out of the atlas pages themselves for debug purposes
        for page_index in 0..self.decoded_bitmap_textures.len() {
            let sprite_name = format!("debug_sprite_whole_page_{}", page_index);
            self.add_sprite_for_region(
                sprite_name,
                page_index as TextureIndex,
                Recti::from_width_height(
                    self.graphics.textures_dimension as i32,
                    self.graphics.textures_dimension as i32,
                ),
                Vec2i::zero(),
                true,
            );
        }

        log::info!("Decoded bitmap textures");
    }

    pub fn get_audiorecordings(&self) -> &HashMap<ResourceName, Rc<RefCell<AudioRecording>>> {
        assert!(self.files_loading_stage == AssetLoadingStage::Finished);
        &self.decoded_audio_recordings
    }

    fn decode_audiorecordings(&mut self) {
        assert!(self.files_loading_stage == AssetLoadingStage::Finished);
        self.decoded_audio_recordings = self
            .audio
            .metadata
            .iter()
            .map(|(resource_name, metadata)| {
                let ogg_data = &self.audio.recordings_ogg_data[resource_name];
                let recording = AudioRecording::new_from_ogg_stream_with_loopsection(
                    resource_name.clone(),
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
                (resource_name.clone(), Rc::new(RefCell::new(recording)))
            })
            .collect();

        log::info!("Decoded audio recordings");
    }

    /// This does not change the atlas bitmap
    pub fn add_sprite_for_region(
        &mut self,
        sprite_name: String,
        atlas_texture_index: TextureIndex,
        sprite_rect: Recti,
        draw_offset: Vec2i,
        has_translucency: bool,
    ) -> Sprite {
        debug_assert!(!self.graphics.sprites.contains_key(&sprite_name));

        let sprite_rect = Rect::from(sprite_rect);
        let draw_offset = Vec2::from(draw_offset);
        let uv_scale = 1.0 / self.graphics.textures_dimension as f32;
        let sprite = Sprite {
            name: sprite_name.clone(),
            atlas_texture_index: atlas_texture_index,
            has_translucency,
            pivot_offset: Vec2::zero(),
            attachment_points: [Vec2::zero(); SPRITE_ATTACHMENT_POINTS_MAX_COUNT],
            untrimmed_dimensions: sprite_rect.dim,
            trimmed_rect: sprite_rect.translated_by(draw_offset),
            trimmed_uvs: AAQuad::from_rect(sprite_rect.scaled_from_origin(Vec2::filled(uv_scale))),
        };

        self.graphics
            .sprites
            .insert(sprite_name.clone(), sprite.clone());
        sprite
    }

    pub fn get_content_filedata(&self, filename: &str) -> &[u8] {
        self.content
            .get(filename)
            .unwrap_or_else(|| panic!("Could not find file '{}'", filename))
    }

    pub fn get_anim(&self, animation_name: &str) -> &Animation<Sprite> {
        self.graphics
            .animations
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_anim_3d(&self, animation_name: &str) -> &Animation<Sprite3D> {
        self.graphics
            .animations_3d
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_font(&self, font_name: &str) -> &SpriteFont {
        self.graphics
            .fonts
            .get(font_name)
            .unwrap_or_else(|| panic!("Could not find font '{}'", font_name))
    }

    pub fn get_sprite(&self, sprite_name: &str) -> &Sprite {
        if let Some(result) = self.graphics.sprites.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            self.graphics
                .sprites
                .get(&format!("{}.0", sprite_name))
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
        }
    }

    pub fn get_sprite_3d(&self, sprite_name: &str) -> &Sprite3D {
        if let Some(result) = self.graphics.sprites_3d.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            self.graphics
                .sprites_3d
                .get(&format!("{}.0", sprite_name))
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
        }
    }
}
