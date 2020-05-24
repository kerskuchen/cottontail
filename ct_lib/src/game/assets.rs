use super::*;

use std::collections::HashMap;

#[derive(Clone)]
pub struct GameAssets {
    pub animations: HashMap<String, Animation<SpriteIndex>>,
}

impl GameAssets {
    pub fn new(animations: HashMap<String, Animation<SpriteIndex>>) -> GameAssets {
        GameAssets { animations }
    }

    pub fn get_anim(&self, animation_name: &str) -> Animation<SpriteIndex> {
        self.animations
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
            .clone()
    }
}
