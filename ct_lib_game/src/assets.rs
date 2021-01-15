use crate::animations_fx::Animation;

use super::*;

use indexmap::IndexMap;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub type ResourceName = String;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetLoadingStage {
    SplashStart,
    SplashProgress,
    SplashFinish,
    WaitingToStartFilesLoading,
    FilesStart,
    FilesProgress,
    FilesFinish,
    DecodingStart,
    DecodingProgress,
    DecodingFinish,
    Idle,
}

impl Default for AssetLoadingStage {
    fn default() -> Self {
        AssetLoadingStage::SplashStart
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
pub struct GraphicResources {
    pub animations: IndexMap<ResourceName, Animation<Sprite>>,
    pub animations_3d: IndexMap<ResourceName, Animation<Sprite3D>>,
    pub sprites: IndexMap<ResourceName, Sprite>,
    pub sprites_3d: IndexMap<ResourceName, Sprite3D>,
    pub fonts: IndexMap<ResourceName, SpriteFont>,

    pub textures_png_data: Vec<Vec<u8>>,
}

#[derive(Default)]
pub struct GameAssets {
    assets_folder: String,

    audio_resources: AudioResources,
    graphic_resources_splash: GraphicResources,
    graphic_resources: GraphicResources,
    content: HashMap<String, Vec<u8>>,

    decoded_audio_recordings: HashMap<ResourceName, Rc<RefCell<AudioRecording>>>,
    decoded_atlas_textures: Vec<Rc<RefCell<Bitmap>>>,
    decoded_atlas_textures_splash: Vec<Rc<RefCell<Bitmap>>>,

    files_loading_stage: AssetLoadingStage,
    files_loaders: HashMap<String, Fileloader>,

    progress_fileloads_started_count: usize,
    progress_deserialized_files_count: usize,
    progress_assets_decoded_count: usize,
}

impl Clone for GameAssets {
    fn clone(&self) -> Self {
        assert!(self.files_loading_stage == AssetLoadingStage::Idle);
        assert!(self.files_loaders.is_empty());

        let mut result = GameAssets::new(&self.assets_folder);

        result.files_loading_stage = self.files_loading_stage.clone();
        result.files_loaders = HashMap::new();

        result.audio_resources = self.audio_resources.clone();
        result.graphic_resources = self.graphic_resources.clone();
        result.content = self.content.clone();

        result
    }
}

const PROGRESS_ASSETS_TO_DECODE_COUNT: usize = 3;
const PROGRESS_FILES_TO_DESERIALIZE_COUNT: usize = 3;
const PROGRESS_FILELOADS_TO_START_COUNT: usize = 2;

impl GameAssets {
    pub fn new(assets_folder: &str) -> GameAssets {
        GameAssets {
            assets_folder: assets_folder.to_owned(),
            audio_resources: AudioResources::default(),
            graphic_resources_splash: GraphicResources::default(),
            graphic_resources: GraphicResources::default(),
            content: HashMap::new(),

            decoded_audio_recordings: HashMap::new(),
            decoded_atlas_textures: Vec::new(),
            decoded_atlas_textures_splash: Vec::new(),

            files_loading_stage: AssetLoadingStage::default(),
            files_loaders: HashMap::new(),

            progress_fileloads_started_count: 0,
            progress_deserialized_files_count: 0,
            progress_assets_decoded_count: 0,
        }
    }

    pub fn get_loading_percentage(&self) -> Option<f32> {
        let progress = self.progress_assets_decoded_count
            + self.progress_deserialized_files_count
            + self.progress_fileloads_started_count;
        let progress_target = PROGRESS_ASSETS_TO_DECODE_COUNT
            + PROGRESS_FILES_TO_DESERIALIZE_COUNT
            + PROGRESS_FILELOADS_TO_START_COUNT;
        let progress_percent = progress as f32 / progress_target as f32;

        match self.files_loading_stage {
            AssetLoadingStage::SplashStart => None,
            AssetLoadingStage::SplashProgress => None,
            AssetLoadingStage::SplashFinish => None,
            AssetLoadingStage::WaitingToStartFilesLoading => None,
            AssetLoadingStage::FilesStart => Some(progress_percent),
            AssetLoadingStage::FilesProgress => Some(progress_percent),
            AssetLoadingStage::FilesFinish => Some(progress_percent),
            AssetLoadingStage::DecodingStart => Some(progress_percent),
            AssetLoadingStage::DecodingProgress => Some(progress_percent),
            AssetLoadingStage::DecodingFinish => Some(progress_percent),
            AssetLoadingStage::Idle => Some(1.0),
        }
    }

    pub fn update(&mut self) -> AssetLoadingStage {
        match self.files_loading_stage {
            AssetLoadingStage::SplashStart => {
                let graphics_splash_filepath =
                    path_join(&self.assets_folder, "graphics_splash.data");
                self.files_loaders.insert(
                    graphics_splash_filepath.clone(),
                    Fileloader::new(&graphics_splash_filepath).unwrap(),
                );

                self.files_loading_stage = AssetLoadingStage::SplashProgress;
            }
            AssetLoadingStage::SplashProgress => {
                let mut finished_loaders = Vec::new();

                // Poll file loaders
                for (filepath, loader) in self.files_loaders.iter_mut() {
                    let poll_result = loader.poll().unwrap_or_else(|error| {
                        panic!("Failed to get file status on '{}': {}", filepath, error)
                    });

                    if let Some(content) = poll_result {
                        log::debug!("Loaded resource file '{}'", filepath);

                        if filepath == &path_join(&self.assets_folder, "graphics_splash.data") {
                            self.graphic_resources_splash = bincode::deserialize(&content).expect(
                                "Could not deserialize 'graphics_splash.data' (file corrupt?)",
                            );
                            log::info!("Loaded splash graphics resources");
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
                    self.files_loading_stage = AssetLoadingStage::SplashFinish;
                    if self.decoded_atlas_textures_splash.is_empty() {
                        self.decode_atlas_textures_splash();
                    }
                    log::info!("Finished loading splash asset files");
                }
            }
            AssetLoadingStage::SplashFinish => {
                self.files_loading_stage = AssetLoadingStage::WaitingToStartFilesLoading;
            }
            AssetLoadingStage::WaitingToStartFilesLoading => {
                // We wait here until our `start_loading_files` method is called
            }
            AssetLoadingStage::FilesStart => {
                let graphics_filepath = path_join(&self.assets_folder, "graphics.data");
                self.files_loaders.insert(
                    graphics_filepath.clone(),
                    Fileloader::new(&graphics_filepath).unwrap(),
                );
                self.progress_fileloads_started_count += 1;

                let content_filepath = path_join(&self.assets_folder, "content.data");
                self.files_loaders.insert(
                    content_filepath.clone(),
                    Fileloader::new(&content_filepath).unwrap(),
                );
                self.progress_fileloads_started_count += 1;

                #[cfg(target_arch = "wasm32")]
                let audio_filepath = path_join(&self.assets_folder, "audio_wasm.data");
                #[cfg(not(target_arch = "wasm32"))]
                let audio_filepath = path_join(&self.assets_folder, "audio.data");
                self.files_loaders.insert(
                    audio_filepath.clone(),
                    Fileloader::new(&audio_filepath).unwrap(),
                );
                self.progress_fileloads_started_count += 1;

                self.files_loading_stage = AssetLoadingStage::FilesProgress;
            }
            AssetLoadingStage::FilesProgress => {
                let mut finished_loaders = Vec::new();

                // Poll file loaders
                for (filepath, loader) in self.files_loaders.iter_mut() {
                    let poll_result = loader.poll().unwrap_or_else(|error| {
                        panic!("Failed to get file status on '{}': {}", filepath, error)
                    });

                    if let Some(bindata) = poll_result {
                        log::debug!("Loaded resource file '{}'", filepath);

                        if filepath == &path_join(&self.assets_folder, "graphics.data") {
                            self.graphic_resources = bincode::deserialize(&bindata)
                                .expect("Could not deserialize 'graphics.data' (file corrupt?)");
                            log::info!("Loaded graphics resources");
                        } else if filepath == &path_join(&self.assets_folder, "audio.data")
                            || filepath == &path_join(&self.assets_folder, "audio_wasm.data")
                        {
                            self.audio_resources = bincode::deserialize(&bindata)
                                .expect("Could not deserialize 'audio.data' (file corrupt?)");
                            log::info!("Loaded audio resources");
                        } else if filepath == &path_join(&self.assets_folder, "content.data") {
                            self.content = bincode::deserialize(&bindata)
                                .expect("Could not deserialize 'content.data' (file corrupt?)");
                            log::info!("Loaded content resources");
                        } else {
                            unreachable!("Loaded unknown file '{}'", filepath);
                        }

                        self.progress_deserialized_files_count += 1;

                        // Mark the loader for removal
                        finished_loaders.push(filepath.clone());
                    }
                }

                // Remove finished file loaders
                for path in &finished_loaders {
                    self.files_loaders.remove(path);
                }

                if self.files_loaders.is_empty() {
                    self.files_loading_stage = AssetLoadingStage::FilesFinish;
                    log::info!("Finished loading asset files");
                }
            }
            AssetLoadingStage::FilesFinish => {
                self.files_loading_stage = AssetLoadingStage::DecodingStart;
            }
            AssetLoadingStage::DecodingStart => {
                self.files_loading_stage = AssetLoadingStage::DecodingProgress;
            }
            AssetLoadingStage::DecodingProgress => {
                if self.decoded_audio_recordings.is_empty() {
                    self.decode_audiorecordings();
                    self.progress_assets_decoded_count += 1;
                    // We only want to decode one asset per frame
                    return AssetLoadingStage::DecodingProgress;
                }

                if self.decoded_atlas_textures.is_empty() {
                    self.decode_atlas_textures();
                    self.progress_assets_decoded_count += 1;
                    // We only want to decode one asset per frame
                    return AssetLoadingStage::DecodingProgress;
                }

                log::info!("Finished decoding assset files");
                self.files_loading_stage = AssetLoadingStage::DecodingFinish;
            }
            AssetLoadingStage::DecodingFinish => {
                self.files_loading_stage = AssetLoadingStage::Idle;
            }
            AssetLoadingStage::Idle => {
                // Nothing to do
            }
        }
        self.files_loading_stage
    }

    pub fn start_loading_files(&mut self) {
        assert!(self.files_loading_stage == AssetLoadingStage::WaitingToStartFilesLoading);
        self.files_loading_stage = AssetLoadingStage::FilesStart;
    }

    pub fn finished_loading_splash(&self) -> bool {
        self.files_loading_stage >= AssetLoadingStage::SplashFinish
    }

    pub fn finished_loading_assets(&self) -> bool {
        self.files_loading_stage >= AssetLoadingStage::DecodingFinish
    }

    pub fn get_atlas_textures(&self) -> &Vec<Rc<RefCell<Bitmap>>> {
        if self.files_loading_stage >= AssetLoadingStage::DecodingFinish {
            &self.decoded_atlas_textures
        } else {
            assert!(self.files_loading_stage >= AssetLoadingStage::SplashFinish);
            &self.decoded_atlas_textures_splash
        }
    }
    fn decode_atlas_textures_splash(&mut self) {
        assert!(self.files_loading_stage == AssetLoadingStage::SplashFinish);
        self.decoded_atlas_textures_splash =
            GameAssets::decode_png_images(&self.graphic_resources_splash.textures_png_data);

        log::info!("Decoded splash bitmap textures");
    }

    fn decode_atlas_textures(&mut self) {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingProgress);
        self.decoded_atlas_textures =
            GameAssets::decode_png_images(&self.graphic_resources.textures_png_data);

        // Make sprites out of the atlas pages themselves for debug purposes
        for page_index in 0..self.decoded_atlas_textures.len() {
            let bitmap_size = self.decoded_atlas_textures[page_index].borrow().width;
            let sprite_name = format!("debug_sprite_atlas_page_{}", page_index);
            self.add_sprite_for_region(
                sprite_name,
                page_index as TextureIndex,
                Recti::from_width_height(bitmap_size, bitmap_size),
                Vec2i::zero(),
                true,
            );
        }

        log::info!("Decoded bitmap textures");
    }

    pub fn get_audio_resources_sample_rate_hz(&self) -> usize {
        assert!(self.files_loading_stage >= AssetLoadingStage::FilesFinish);
        self.audio_resources.resource_sample_rate_hz
    }

    pub fn get_audiorecordings(&self) -> &HashMap<ResourceName, Rc<RefCell<AudioRecording>>> {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingFinish);
        &self.decoded_audio_recordings
    }
    fn decode_audiorecordings(&mut self) {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingProgress);
        self.decoded_audio_recordings = self
            .audio_resources
            .metadata
            .iter()
            .map(|(resource_name, metadata)| {
                let ogg_data = &self.audio_resources.recordings_ogg_data[resource_name];
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
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingProgress);
        debug_assert!(!self.graphic_resources.sprites.contains_key(&sprite_name));

        let texture_size = self.decoded_atlas_textures[atlas_texture_index as usize]
            .borrow()
            .width;
        let sprite_rect = Rect::from(sprite_rect);
        let draw_offset = Vec2::from(draw_offset);
        let uv_scale = 1.0 / texture_size as f32;
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

        self.graphic_resources
            .sprites
            .insert(sprite_name.clone(), sprite.clone());
        sprite
    }

    pub fn get_content_filedata(&self, filename: &str) -> &[u8] {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingFinish);
        self.content
            .get(filename)
            .unwrap_or_else(|| panic!("Could not find file '{}'", filename))
    }

    pub fn get_anim(&self, animation_name: &str) -> &Animation<Sprite> {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingFinish);
        self.graphic_resources
            .animations
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_anim_3d(&self, animation_name: &str) -> &Animation<Sprite3D> {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingFinish);
        self.graphic_resources
            .animations_3d
            .get(animation_name)
            .unwrap_or_else(|| panic!("Could not find animation '{}'", animation_name))
    }

    pub fn get_font(&self, font_name: &str) -> &SpriteFont {
        let fonts = if self.files_loading_stage >= AssetLoadingStage::DecodingFinish {
            &self.graphic_resources.fonts
        } else {
            assert!(self.files_loading_stage >= AssetLoadingStage::SplashFinish);
            &self.graphic_resources_splash.fonts
        };

        fonts
            .get(font_name)
            .unwrap_or_else(|| panic!("Could not find font '{}'", font_name))
    }

    pub fn get_sprite(&self, sprite_name: &str) -> &Sprite {
        let sprites = if self.files_loading_stage >= AssetLoadingStage::DecodingFinish {
            &self.graphic_resources.sprites
        } else {
            assert!(self.files_loading_stage >= AssetLoadingStage::SplashFinish);
            &self.graphic_resources_splash.sprites
        };

        if let Some(result) = sprites.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            sprites
                .get(&format!("{}.0", sprite_name))
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
        }
    }

    pub fn get_sprite_3d(&self, sprite_name: &str) -> &Sprite3D {
        assert!(self.files_loading_stage >= AssetLoadingStage::DecodingFinish);
        if let Some(result) = self.graphic_resources.sprites_3d.get(sprite_name) {
            result
        } else {
            // NOTE: By adding ".0" automatically we can conveniently call the first (or only) frame
            //       of a sprite without the ".0" suffix
            self.graphic_resources
                .sprites_3d
                .get(&format!("{}.0", sprite_name))
                .unwrap_or_else(|| panic!("Sprite with name '{}' does not exist", sprite_name))
        }
    }

    fn decode_png_images(textures_png_data: &[Vec<u8>]) -> Vec<Rc<RefCell<Bitmap>>> {
        textures_png_data
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
                Rc::new(RefCell::new(bitmap))
            })
            .collect()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn hotreload_assets(&mut self) -> bool {
        // Maybe someday
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn hotreload_assets(&mut self) -> bool {
        let path_graphics = path_join(&self.assets_folder, "graphics.data");
        let path_content = path_join(&self.assets_folder, "content.data");
        let path_audio = path_join(&self.assets_folder, "audio.data");

        if path_exists("resources/assetbaker.lock")
            || !path_exists(&path_graphics)
            || !path_exists(&path_content)
            || !path_exists(&path_audio)
        {
            // The resource folder is probably currently baking
            return false;
        }

        let last_write_time_graphics = path_last_modified_time(&path_graphics);
        let last_write_time_content = path_last_modified_time(&path_content);
        let last_write_time_audio = path_last_modified_time(&path_audio);

        static mut LAST_WRITE_TIME_GRAPHICS: f64 = 0.0;
        static mut LAST_WRITE_TIME_CONTENT: f64 = 0.0;
        static mut LAST_WRITE_TIME_AUDIO: f64 = 0.0;

        let mut reload_happened = false;
        unsafe {
            if LAST_WRITE_TIME_GRAPHICS == 0.0 {
                LAST_WRITE_TIME_GRAPHICS = last_write_time_graphics;
            }
            if LAST_WRITE_TIME_CONTENT == 0.0 {
                LAST_WRITE_TIME_CONTENT = last_write_time_content;
            }
            if LAST_WRITE_TIME_AUDIO == 0.0 {
                LAST_WRITE_TIME_AUDIO = last_write_time_audio;
            }

            if LAST_WRITE_TIME_GRAPHICS != last_write_time_graphics {
                self.graphic_resources = deserialize_from_binary_file(&path_graphics);
                self.decode_atlas_textures();
                LAST_WRITE_TIME_GRAPHICS = last_write_time_graphics;
                reload_happened = true;
            }
            if LAST_WRITE_TIME_CONTENT != last_write_time_content {
                self.content = deserialize_from_binary_file(&path_content);
                LAST_WRITE_TIME_CONTENT = last_write_time_content;
                reload_happened = true;
            }
            if LAST_WRITE_TIME_AUDIO != last_write_time_audio {
                self.audio_resources = deserialize_from_binary_file(&path_audio);
                self.decode_audiorecordings();
                LAST_WRITE_TIME_AUDIO = last_write_time_audio;
                reload_happened = true;
            }
        }
        reload_happened
    }
}
