use super::*;

use std::collections::HashMap;

#[derive(Clone)]
pub struct GameAssets {
    pub animations: HashMap<String, Animation>,
}

impl GameAssets {
    pub fn new(animations: HashMap<String, Animation>) -> GameAssets {
        GameAssets { animations }
    }

    pub fn get_anim(&self, animation_name: &str) -> Animation {
        self.animations
            .get(animation_name)
            .expect(&format!("Could not find animation '{}'", animation_name))
            .clone()
    }
}
