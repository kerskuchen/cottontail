use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Animations

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AnimationFrame<FrameType: Clone> {
    pub duration_seconds: f32,
    #[serde(bound(deserialize = "FrameType: serde::de::DeserializeOwned"))]
    pub value: FrameType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Animation<FrameType: Clone> {
    pub name: String,
    #[serde(bound(deserialize = "FrameType: serde::de::DeserializeOwned"))]
    pub frames: Vec<AnimationFrame<FrameType>>,
    pub length: f32,
}

impl<FrameType: Clone> Animation<FrameType> {
    pub fn new_empty(name: String) -> Animation<FrameType> {
        Animation {
            name,
            frames: Vec::with_capacity(32),
            length: 0.0,
        }
    }

    pub fn add_frame(&mut self, duration_seconds: f32, value: FrameType) {
        assert!(duration_seconds > 0.0);

        self.length += duration_seconds;
        self.frames.push(AnimationFrame {
            duration_seconds,
            value,
        });
    }

    fn frame_index_and_percent_at_time(&self, time: f32, wrap_around: bool) -> (usize, f32) {
        assert!(!self.frames.is_empty());

        let time = if wrap_around {
            wrap_value_in_range(time, self.length)
        } else {
            clampf(time, 0.0, self.length)
        };

        let mut frame_start = 0.0;

        for (index, frame) in self.frames.iter().enumerate() {
            let frame_end = frame_start + frame.duration_seconds;

            if time < frame_end {
                let percent = (time - frame_start) / frame.duration_seconds;
                return (index, percent);
            }

            frame_start = frame_end;
        }

        (self.frames.len() - 1, 1.0)
    }

    pub fn frame_at_time(&self, time: f32, wrap_around: bool) -> &FrameType {
        let (index, _percent) = self.frame_index_and_percent_at_time(time, wrap_around);
        &self.frames[index].value
    }

    pub fn frame_at_percentage(&self, percentage: f32) -> &FrameType {
        debug_assert!(0.0 <= percentage && percentage <= 1.0);
        let time = percentage * self.length;
        self.frame_at_time(time, false)
    }
}

impl Animation<f32> {
    pub fn value_at_time_interpolated_linear(&self, time: f32, wrap_around: bool) -> f32 {
        let (frame_index, frametime_percent) =
            self.frame_index_and_percent_at_time(time, wrap_around);
        let next_frame_index = if wrap_around {
            (frame_index + 1) % self.frames.len()
        } else {
            usize::min(frame_index + 1, self.frames.len() - 1)
        };

        let value_start = self.frames[frame_index].value;
        let value_end = self.frames[next_frame_index].value;

        lerp(value_start, value_end, frametime_percent)
    }
}

#[derive(Clone)]
pub struct AnimationPlayer<FrameType: Clone> {
    pub current_frametime: f32,
    pub playback_speed: f32,
    pub looping: bool,
    pub animation: Animation<FrameType>,
    pub has_finished: bool,
}

impl<FrameType: Clone> AnimationPlayer<FrameType> {
    pub fn new_from_beginning(
        animation: Animation<FrameType>,
        playback_speed: f32,
        looping: bool,
    ) -> AnimationPlayer<FrameType> {
        assert!(animation.length > 0.0);

        AnimationPlayer {
            current_frametime: 0.0,
            playback_speed,
            looping,
            animation,
            has_finished: false,
        }
    }

    pub fn new_from_end(
        animation: Animation<FrameType>,
        playback_speed: f32,
        looping: bool,
    ) -> AnimationPlayer<FrameType> {
        let mut result = AnimationPlayer::new_from_beginning(animation, playback_speed, looping);
        result.restart_from_end();
        result
    }

    pub fn restart_from_beginning(&mut self) {
        self.current_frametime = 0.0;
        self.has_finished = false;
    }

    pub fn restart_from_end(&mut self) {
        self.current_frametime = self.animation.length;
        self.has_finished = false;
    }

    pub fn update(&mut self, deltatime: f32) {
        if self.playback_speed == 0.0 {
            return;
        }

        let new_frametime = self.current_frametime + self.playback_speed * deltatime;
        if self.looping {
            self.current_frametime = wrap_value_in_range(new_frametime, self.animation.length);
        } else {
            self.current_frametime = clampf(new_frametime, 0.0, self.animation.length);
            if self.current_frametime == self.animation.length && self.playback_speed > 0.0 {
                self.has_finished = true;
            }
            if self.current_frametime == 0.0 && self.playback_speed < 0.0 {
                self.has_finished = true;
            }
        }
    }

    pub fn frame_at_percentage(&self, percentage: f32) -> &FrameType {
        self.animation.frame_at_percentage(percentage)
    }

    pub fn current_frame(&self) -> &FrameType {
        self.animation
            .frame_at_time(self.current_frametime, self.looping)
    }
}

impl AnimationPlayer<f32> {
    pub fn value_current_interpolated_linear(&self) -> f32 {
        self.animation
            .value_at_time_interpolated_linear(self.current_frametime, self.looping)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Particles

#[derive(Copy, Clone, Default)]
pub struct ParticleSystemParams {
    pub gravity: Vec2,
    pub vel_start: Vec2,
    pub vel_max: f32,
    pub scale_start: f32,
    pub scale_end: f32,
    pub spawn_radius: f32,
    pub lifetime: f32,
    pub additivity_start: f32,
    pub additivity_end: f32,
    pub color_start: Color,
    pub color_end: Color,
}

#[derive(Clone)]
pub struct ParticleSystem {
    count_max: usize,
    root_pos: Vec2,

    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    age: Vec<f32>,

    pub params: ParticleSystemParams,

    time_since_last_spawn: f32,
}

impl ParticleSystem {
    pub fn new(params: ParticleSystemParams, count_max: usize, root_pos: Vec2) -> ParticleSystem {
        ParticleSystem {
            count_max,
            root_pos,
            pos: Vec::with_capacity(count_max),
            vel: Vec::with_capacity(count_max),
            age: Vec::with_capacity(count_max),
            params,
            time_since_last_spawn: 0.0,
        }
    }

    pub fn pos(&self) -> Vec2 {
        self.root_pos
    }

    pub fn count(&self) -> usize {
        self.pos.len()
    }

    pub fn set_count_max(&mut self, count_max: usize) {
        self.count_max = count_max;
    }

    pub fn move_to(&mut self, pos: Vec2) {
        self.root_pos = pos;
    }

    pub fn update_and_draw(
        &mut self,
        random: &mut Random,
        deltatime: f32,
        depth: f32,
        drawspace: Drawspace,
    ) {
        // Update
        for index in 0..self.pos.len() {
            linear_motion_integrate_v2(
                &mut self.pos[index],
                &mut self.vel[index],
                self.params.gravity,
                self.params.vel_max,
                deltatime,
            );
        }

        // Draw
        for index in 0..self.pos.len() {
            let age_percentage = self.age[index] / self.params.lifetime;
            let scale = lerp(
                self.params.scale_start,
                self.params.scale_end,
                age_percentage,
            );
            let additivity = lerp(
                self.params.additivity_start,
                self.params.additivity_end,
                age_percentage,
            );
            let color = Color::mix(
                self.params.color_start,
                self.params.color_end,
                age_percentage,
            );
            let pos = self.pos[index].pixel_snapped();
            let drawparams = Drawparams::new(depth, color, additivity, drawspace);
            if scale > 1.0 {
                draw_rect_transformed(
                    Vec2::ones(),
                    true,
                    true,
                    Vec2::zero(),
                    Transform::from_pos_scale_uniform(pos, scale),
                    drawparams,
                );
            } else {
                draw_pixel(pos, drawparams);
            }
        }

        // Remove old
        for index in (0..self.pos.len()).rev() {
            self.age[index] += deltatime;
            if self.age[index] > self.params.lifetime {
                self.pos.swap_remove(index);
                self.vel.swap_remove(index);
                self.age.swap_remove(index);
            }
        }

        self.time_since_last_spawn += deltatime;

        // Spawn new
        if self.count_max > 0 {
            let time_between_spawns = self.params.lifetime / self.count_max as f32;
            if self.pos.len() < self.count_max && self.time_since_last_spawn >= time_between_spawns
            {
                self.time_since_last_spawn -= time_between_spawns;
                let pos = self.root_pos + self.params.spawn_radius * random.vec2_in_unit_disk();
                let vel = self.params.vel_start;

                self.pos.push(pos);
                self.vel.push(vel);
                self.age.push(0.0);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Afterimage

#[derive(Clone)]
pub struct Afterimage {
    count_max: usize,

    lifetime: f32,
    additivity_modulate_start: f32,
    additivity_modulate_end: f32,
    color_modulate_start: Color,
    color_modulate_end: Color,

    sprite: Vec<Sprite>,
    age: Vec<f32>,
    xform: Vec<Transform>,
    flip_horizontally: Vec<bool>,
    flip_vertically: Vec<bool>,
    color_modulate: Vec<Color>,
    additivity: Vec<f32>,

    time_since_last_spawn: f32,
}

impl Afterimage {
    pub fn new(
        lifetime: f32,
        additivity_modulate_start: f32,
        additivity_modulate_end: f32,
        color_modulate_start: Color,
        color_modulate_end: Color,
        count_max: usize,
    ) -> Afterimage {
        Afterimage {
            count_max,

            lifetime,
            additivity_modulate_start,
            additivity_modulate_end,
            color_modulate_start,
            color_modulate_end,

            sprite: Vec::with_capacity(count_max),
            age: Vec::with_capacity(count_max),
            xform: Vec::with_capacity(count_max),
            flip_horizontally: Vec::with_capacity(count_max),
            flip_vertically: Vec::with_capacity(count_max),
            color_modulate: Vec::with_capacity(count_max),
            additivity: Vec::with_capacity(count_max),

            time_since_last_spawn: 0.0,
        }
    }

    pub fn set_count_max(&mut self, count_max: usize) {
        self.count_max = count_max;
    }

    pub fn add_afterimage_image_if_needed(
        &mut self,
        deltatime: f32,
        newimage_sprite: Sprite,
        newimage_xform: Transform,
        newimage_flip_horizontally: bool,
        newimage_flip_vertically: bool,
        newimage_color_modulate: Color,
        newimage_additivity: f32,
    ) {
        self.time_since_last_spawn += deltatime;

        if self.count_max > 0 {
            let time_between_spawns = self.lifetime / self.count_max as f32;
            if self.xform.len() < self.count_max
                && self.time_since_last_spawn >= time_between_spawns
            {
                self.time_since_last_spawn -= time_between_spawns;

                self.sprite.push(newimage_sprite);
                self.age.push(0.0);
                self.xform.push(newimage_xform);
                self.flip_horizontally.push(newimage_flip_horizontally);
                self.flip_vertically.push(newimage_flip_vertically);
                self.color_modulate.push(newimage_color_modulate);
                self.additivity.push(newimage_additivity);
            }
        }
    }

    pub fn update_and_draw(&mut self, deltatime: f32, draw_depth: f32, drawspace: Drawspace) {
        for index in 0..self.sprite.len() {
            let age_percentage = self.age[index] / self.lifetime;
            let additivity = lerp(
                self.additivity_modulate_start,
                self.additivity_modulate_end,
                age_percentage,
            );
            let color = Color::mix(
                self.color_modulate_start,
                self.color_modulate_end,
                age_percentage,
            );

            draw_sprite(
                &self.sprite[index],
                self.xform[index],
                self.flip_horizontally[index],
                self.flip_vertically[index],
                Drawparams::new(
                    draw_depth,
                    color * self.color_modulate[index],
                    additivity * self.additivity[index],
                    drawspace,
                ),
            );
        }

        for index in (0..self.xform.len()).rev() {
            self.age[index] += deltatime;
            if self.age[index] > self.lifetime {
                self.sprite.swap_remove(index);
                self.age.swap_remove(index);
                self.xform.swap_remove(index);
                self.flip_horizontally.swap_remove(index);
                self.flip_vertically.swap_remove(index);
                self.color_modulate.swap_remove(index);
                self.additivity.swap_remove(index);
            }
        }
    }
}
