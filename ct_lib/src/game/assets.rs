use super::*;

use std::collections::HashMap;

#[derive(Clone)]
pub struct GameAssets {
    pub animations: HashMap<String, Animation<SpriteIndex>>,
    pub animations_3d: HashMap<String, Animation<Sprite3D>>,
}

impl GameAssets {
    pub fn new(assets_folder: &str) -> GameAssets {
        let animations = GameAssets::load_animations(assets_folder);
        let animations_3d = GameAssets::load_animations_3d(assets_folder);
        GameAssets {
            animations,
            animations_3d,
        }
    }

    pub fn get_anim(&self, animation_name: &str) -> &Animation<SpriteIndex> {
        self.animations
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
    }

    pub fn get_anim_3d(&self, animation_name: &str) -> &Animation<Sprite3D> {
        self.animations_3d
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
    }

    fn load_animations(assets_folder: &str) -> HashMap<String, Animation<SpriteIndex>> {
        let animations_filepath = system::path_join(assets_folder, "animations.data");
        let animations =
            bincode::deserialize(&std::fs::read(&animations_filepath).expect(&format!(
                "Could not read '{}' - Gamedata corrupt?",
                animations_filepath
            )))
            .expect(&format!(
                "Could not deserialize '{}' - Gamedata corrupt?",
                animations_filepath
            ));
        animations
    }

    fn load_animations_3d(assets_folder: &str) -> HashMap<String, Animation<Sprite3D>> {
        /*
        let animations_filepath = system::path_join(assets_folder, "animations_3d.data");
        let animations =
            bincode::deserialize(&std::fs::read(&animations_filepath).expect(&format!(
                "Could not read '{}' - Gamedata corrupt?",
                animations_filepath
            )))
            .expect(&format!(
                "Could not deserialize '{}' - Gamedata corrupt?",
                animations_filepath
            ));
        animations
        */

        HashMap::new()
    }
}
