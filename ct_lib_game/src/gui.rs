/// Immediate mode gui that is heavily inspired by the tutorials of
/// Jari Komppa of http://sol.gfxile.net/imgui/index.html
///
use crate::draw::Canvaspoint;
use crate::draw::{Drawspace, Drawstate};
use crate::math;
use crate::math::Rect;
use crate::*;

const GUI_ELEM_ID_UNAVAILABLE: GuiElemId = GuiElemId {
    name: "__unavailable",
    counter: std::usize::MAX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuiElemId {
    name: &'static str,
    counter: usize,
}

impl GuiElemId {
    pub fn new(name: &'static str) -> GuiElemId {
        GuiElemId { name, counter: 0 }
    }
    pub fn new_with_counter(name: &'static str, counter: usize) -> GuiElemId {
        GuiElemId { name, counter }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiAction {
    Next,
    Previous,
    Accept,
    Left,
    Right,
    Up,
    Down,
    PageDown,
    PageUp,
    Increase,
    Decrease,
}

#[derive(Debug)]
pub struct GuiState {
    mouse_canvas_pos: Canvaspoint,
    mouse_is_down: bool,
    mouse_recently_pressed: bool,
    mouse_recently_released: bool,
    mouse_canvas_delta: Vec2,
    mouse_wheel_delta: i32,
    mouse_highlighted_item: Option<GuiElemId>,

    finger_pos_canvas: Option<Canvaspoint>,
    finger_pos_canvas_previous: Vec2,

    keyboard_highlighted_item: Option<GuiElemId>,
    active_item: Option<GuiElemId>,

    current_action: Option<GuiAction>,
    last_widget: Option<GuiElemId>,
}

impl GuiState {
    #[inline]
    pub fn new() -> GuiState {
        GuiState {
            mouse_canvas_pos: Vec2::zero(),
            mouse_is_down: false,
            mouse_recently_pressed: false,
            mouse_recently_released: false,
            mouse_canvas_delta: Vec2::zero(),
            mouse_wheel_delta: 0,

            mouse_highlighted_item: None,

            finger_pos_canvas: None,
            finger_pos_canvas_previous: -Vec2::ones(),

            keyboard_highlighted_item: None,
            active_item: None,
            current_action: None,
            last_widget: None,
        }
    }

    #[inline]
    pub fn begin_frame(&mut self, cursors: &Cursors, input: &InputState) {
        self.finger_pos_canvas = cursors.finger_primary.map(|coords| coords.pos_canvas);
        self.mouse_is_down = input.mouse.button_left.is_pressed;
        self.mouse_recently_pressed = input.mouse.button_left.recently_pressed();
        self.mouse_recently_released = input.mouse.button_left.recently_released();
        self.mouse_canvas_delta = cursors.mouse.delta_canvas;
        self.mouse_canvas_pos = cursors.mouse.pos_canvas;
        self.mouse_wheel_delta = input.mouse.wheel_delta;

        self.mouse_highlighted_item = None;

        self.current_action = if (input.keyboard.is_down(Scancode::ShiftLeft)
            || input.keyboard.is_down(Scancode::ShiftRight))
            && input.keyboard.recently_pressed(Scancode::Tab)
        {
            Some(GuiAction::Previous)
        } else if input.keyboard.recently_pressed(Scancode::Tab) {
            Some(GuiAction::Next)
        } else if input.keyboard.recently_pressed(Scancode::Enter) {
            Some(GuiAction::Accept)
        } else if input.keyboard.recently_pressed(Scancode::ArrowDown) {
            Some(GuiAction::Down)
        } else if input.keyboard.recently_pressed(Scancode::ArrowUp) {
            Some(GuiAction::Up)
        } else if input.keyboard.recently_pressed(Scancode::ArrowLeft) {
            Some(GuiAction::Left)
        } else if input.keyboard.recently_pressed(Scancode::ArrowRight) {
            Some(GuiAction::Right)
        } else if input.keyboard.recently_pressed(Scancode::NumpadAdd) {
            Some(GuiAction::Increase)
        } else if input.keyboard.recently_pressed(Scancode::NumpadSubtract) {
            Some(GuiAction::Decrease)
        } else if input.keyboard.recently_pressed(Scancode::PageDown) {
            Some(GuiAction::PageDown)
        } else if input.keyboard.recently_pressed(Scancode::PageUp) {
            Some(GuiAction::PageUp)
        } else {
            None
        };
    }

    #[inline]
    pub fn end_frame(&mut self) {
        if let Some(finger_pos_canvas) = self.finger_pos_canvas {
            self.finger_pos_canvas_previous = finger_pos_canvas;
        }
        if self.mouse_is_down || self.finger_pos_canvas.is_some() {
            // From http://sol.gfxile.net/imgui/ch03.html
            // "If the mouse is pressed, but no widget is active, we need to mark the active item
            // unavailable so that we won't activate the next widget we drag the cursor onto."
            if self.active_item.is_none() {
                self.active_item = Some(GUI_ELEM_ID_UNAVAILABLE);
            }
        } else {
            self.active_item = None;
        }

        if self.current_action == Some(GuiAction::Next) {
            self.keyboard_highlighted_item = None;
        }
        self.current_action = None;
    }

    /// Returns true if the button was clicked
    #[inline]
    #[must_use = "It returns whether the button was clicked or not"]
    pub fn button(
        &mut self,
        draw: &mut Drawstate,
        id: GuiElemId,
        button_rect: Rect,
        label: &str,
        label_font: &SpriteFont,
        color_label: Color,
        color_background: Color,
        drawparams: Drawparams,
    ) -> bool {
        if self.mouse_canvas_pos.intersects_rect(button_rect) {
            self.mouse_highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        let finger_intersects_rect_current = self
            .finger_pos_canvas
            .map(|finger_pos_canvas| finger_pos_canvas.intersects_rect(button_rect))
            .unwrap_or(false);
        let finger_intersects_rect_previous =
            self.finger_pos_canvas_previous.intersects_rect(button_rect);

        if finger_intersects_rect_current {
            if self.active_item.is_none() {
                self.active_item = Some(id);
            }
        }

        if self.keyboard_highlighted_item.is_none() {
            self.keyboard_highlighted_item = Some(id);
        }

        let color_highlight = if self.mouse_highlighted_item == Some(id)
            || finger_intersects_rect_current
            || self.keyboard_highlighted_item == Some(id)
        {
            if self.active_item == Some(id) {
                Color::red()
            } else {
                Color::magenta()
            }
        } else {
            Color::blue()
        };

        // Draw buttons with outlines
        draw.draw_rect(
            button_rect,
            true,
            Drawparams {
                color_modulate: color_background,
                ..drawparams
            },
        );
        draw.draw_rect(
            button_rect,
            false,
            Drawparams {
                color_modulate: color_highlight,
                ..drawparams
            },
        );

        // Draw button text
        draw.draw_text(
            label,
            label_font,
            1.0,
            button_rect.center(),
            Vec2::zero(),
            Some(TextAlignment::centered(false, false)),
            None,
            Drawparams::without_additivity(drawparams.depth, color_label, drawparams.drawspace),
        );

        // Keyboard input
        if self.keyboard_highlighted_item == Some(id) {
            if let Some(key) = self.current_action {
                match key {
                    GuiAction::Accept => return true,
                    GuiAction::Previous => self.keyboard_highlighted_item = self.last_widget,
                    GuiAction::Next => self.keyboard_highlighted_item = None,
                    _ => {}
                }
                self.current_action = None;
            }
        }
        self.last_widget = Some(id);

        let button_clicked_mouse = self.active_item == Some(id)
            && self.mouse_highlighted_item == Some(id)
            && !self.mouse_is_down;

        let button_clicked_finger = self.active_item == Some(id)
            && self.finger_pos_canvas.is_none()
            && finger_intersects_rect_previous;

        button_clicked_finger || button_clicked_mouse
    }

    #[inline]
    #[must_use = "It returns a new percentage value if the slider was mutated"]
    pub fn horizontal_slider(
        &mut self,
        draw: &mut Drawstate,
        id: GuiElemId,
        slider_rect: Rect,
        cur_value: f32,
        depth: f32,
    ) -> Option<f32> {
        let knob_size = slider_rect.dim.y;
        let x_pos = (slider_rect.dim.x - knob_size) * cur_value;
        let knob_rect = Rect::from_xy_width_height(
            slider_rect.pos.x + x_pos,
            slider_rect.pos.y,
            knob_size,
            knob_size,
        );

        if self.mouse_canvas_pos.intersects_rect(slider_rect) {
            self.mouse_highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        // If no widget has keyboard focus, take it
        if self.keyboard_highlighted_item.is_none() {
            self.keyboard_highlighted_item = Some(id);
        }

        if self.keyboard_highlighted_item == Some(id) {
            draw.draw_rect(
                slider_rect.extended_uniformly_by(2.0),
                true,
                Drawparams::without_additivity(depth, Color::cyan(), Drawspace::Canvas),
            );
        }

        let color = if self.mouse_highlighted_item == Some(id) {
            if self.active_item == Some(id) {
                Color::red()
            } else {
                Color::magenta()
            }
        } else {
            Color::blue()
        };

        draw.draw_rect(
            slider_rect,
            true,
            Drawparams::without_additivity(depth, Color::white(), Drawspace::Canvas),
        );
        draw.draw_rect(
            knob_rect,
            true,
            Drawparams::without_additivity(depth, color, Drawspace::Canvas),
        );

        if self.keyboard_highlighted_item == Some(id) {
            if let Some(key) = self.current_action {
                match key {
                    GuiAction::Previous => self.keyboard_highlighted_item = self.last_widget,
                    GuiAction::Next => self.keyboard_highlighted_item = None,
                    GuiAction::Decrease | GuiAction::Left => {
                        return Some(math::clampf(cur_value - 0.1, 0.0, 1.0))
                    }
                    GuiAction::Increase | GuiAction::Right => {
                        return Some(math::clampf(cur_value + 0.1, 0.0, 1.0))
                    }
                    _ => {}
                }
                self.current_action = None;
            }
        }
        self.last_widget = Some(id);

        if self.active_item == Some(id) {
            let mouse_x = math::clampf(
                self.mouse_canvas_pos.x - (slider_rect.pos.x),
                0.0,
                slider_rect.dim.x,
            );

            let value = mouse_x / slider_rect.dim.x;
            if value != cur_value {
                return Some(value);
            }
        }
        None
    }

    #[inline]
    pub fn text_scroller(
        &mut self,
        draw: &mut Drawstate,
        id: GuiElemId,
        dt: f32,
        rect: Rect,
        font: &SpriteFont,
        font_scale: f32,
        text_color: Color,
        text: &str,
        linecount: usize,
        inout_pos: &mut f32,
        inout_vel: &mut f32,
        inout_acc: &mut f32,
        depth: f32,
    ) {
        if self.mouse_canvas_pos.intersects_rect(rect) {
            self.mouse_highlighted_item = Some(id);
            if self.active_item.is_none() && self.mouse_is_down {
                self.active_item = Some(id);
            }
        }

        // If no widget has keyboard focus, take it
        if self.keyboard_highlighted_item.is_none() {
            self.keyboard_highlighted_item = Some(id);
        }

        if self.keyboard_highlighted_item == Some(id) {
            draw.draw_rect(
                rect.extended_uniformly_by(2.0),
                true,
                Drawparams::without_additivity(depth, Color::cyan(), Drawspace::Canvas),
            );
        }

        let mut new_pos = *inout_pos;
        let mut new_vel = *inout_vel;
        let mut new_acc = *inout_acc;

        let line_height = font.vertical_advance() as f32;

        // Mouse scroll
        if self.active_item == Some(id) {
            if self.mouse_recently_pressed {
                // We want to stop previous scrolling movement
                new_vel = 0.0;
                new_acc = 0.0;
            } else if self.mouse_recently_released {
                // We want to start autoscrolling after releasing
                let cursor_vel = self.mouse_canvas_delta.y / dt;
                let mut _distance_dont_care = 0.0;

                new_vel = cursor_vel;
                linear_motion_get_start_acc_and_final_resting_distance(
                    cursor_vel,
                    1.5,
                    &mut _distance_dont_care,
                    &mut new_acc,
                );
            } else {
                // We are just holding down the mouse - drag content
                new_pos += self.mouse_canvas_delta.y;
            }
        }
        if self.mouse_canvas_pos.intersects_rect(rect) {
            if self.mouse_wheel_delta != 0 {
                linear_motion_get_start_vel_and_start_acc(
                    5.0 * self.mouse_wheel_delta as f32 * line_height,
                    0.1,
                    &mut new_vel,
                    &mut new_acc,
                );
            }
        }

        // Keyboard scroll
        if self.keyboard_highlighted_item == Some(id) {
            if let Some(action) = self.current_action {
                match action {
                    GuiAction::Previous => self.keyboard_highlighted_item = self.last_widget,
                    GuiAction::Next => self.keyboard_highlighted_item = None,
                    GuiAction::Up => {
                        linear_motion_get_start_vel_and_start_acc(
                            line_height,
                            0.1,
                            &mut new_vel,
                            &mut new_acc,
                        );
                    }
                    GuiAction::Down => {
                        linear_motion_get_start_vel_and_start_acc(
                            -line_height,
                            0.1,
                            &mut new_vel,
                            &mut new_acc,
                        );
                    }
                    GuiAction::PageDown => {
                        linear_motion_get_start_vel_and_start_acc(
                            -20.0 * line_height,
                            0.2,
                            &mut new_vel,
                            &mut new_acc,
                        );
                    }
                    GuiAction::PageUp => {
                        linear_motion_get_start_vel_and_start_acc(
                            20.0 * line_height,
                            0.2,
                            &mut new_vel,
                            &mut new_acc,
                        );
                    }
                    _ => {}
                }
            }
        }

        // This uses velocity-verlet integration as euler integration has a huge error even for fixed dt
        // https://jdickinsongames.wordpress.com/2015/01/22/numerical-integration-in-games-development-2/
        let vel_halfstep = add_or_zero_when_changing_sign(new_vel, new_acc * dt / 2.0);
        new_pos += vel_halfstep * dt;
        if vel_halfstep == 0.0 {
            new_acc = 0.0;
        }
        new_vel = add_or_zero_when_changing_sign(vel_halfstep, new_acc * dt / 2.0);
        if new_vel == 0.0 {
            new_acc = 0.0;
        }

        let text_height = linecount as f32 * font_scale as f32 * font.vertical_advance() as f32;
        let max_pos = -(text_height - rect.height());
        new_pos = clampf(new_pos, max_pos, 0.0);
        if (new_pos == 0.0) || (new_pos == -max_pos) {
            new_vel = 0.0;
            new_acc = 0.0;
        }

        *inout_pos = new_pos;
        *inout_vel = new_vel;
        *inout_acc = new_acc;

        // DEBUG
        {
            if self.keyboard_highlighted_item == Some(id) {
                {
                    // TODO: Keyboard focus here
                    draw.draw_rect(
                        rect.extended_uniformly_by(1.0),
                        true,
                        Drawparams::without_additivity(
                            depth,
                            Color::green().with_multiplied_color(0.5),
                            Drawspace::Canvas,
                        ),
                    );
                }

                // Draw background
                if self.active_item == Some(id) || self.mouse_highlighted_item == Some(id) {
                    draw.draw_rect(
                        rect,
                        true,
                        Drawparams::without_additivity(depth, Color::red(), Drawspace::Canvas),
                    );
                } else {
                    draw.draw_rect(
                        rect,
                        true,
                        Drawparams::without_additivity(depth, Color::black(), Drawspace::Canvas),
                    );
                }
            }
        }

        // Draw text
        draw.draw_text_clipped(
            text,
            font,
            font_scale,
            rect.pos + Vec2::filled_y(new_pos),
            Vec2::zero(),
            false,
            rect,
            Drawparams::without_additivity(depth, text_color, Drawspace::Canvas),
        );

        self.last_widget = Some(id);

        let has_clicked = self.mouse_highlighted_item == Some(id)
            && self.active_item == Some(id)
            && self.mouse_recently_released;
        if has_clicked {
            self.keyboard_highlighted_item = Some(id);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Easy API

static mut GUI_STATE: Option<GuiState> = None;

#[inline]
fn gui_get() -> &'static mut GuiState {
    unsafe {
        if let Some(gui) = GUI_STATE.as_mut() {
            gui
        } else {
            GUI_STATE = Some(GuiState::new());
            GUI_STATE.as_mut().unwrap()
        }
    }
}

#[inline]
pub fn gui_begin_frame(cursors: &Cursors, input: &InputState) {
    gui_get().begin_frame(cursors, input)
}

#[inline]
pub fn gui_end_frame() {
    gui_get().end_frame()
}

#[inline]
#[must_use = "It returns whether the button was clicked or not"]
pub fn gui_button(
    draw: &mut Drawstate,
    id: GuiElemId,
    button_rect: Rect,
    label: &str,
    label_font: &SpriteFont,
    color_label: Color,
    color_background: Color,
    drawparams: Drawparams,
) -> bool {
    gui_get().button(
        draw,
        id,
        button_rect,
        label,
        label_font,
        color_label,
        color_background,
        drawparams,
    )
}

#[inline]
#[must_use = "It returns a new percentage value if the slider was mutated"]
pub fn gui_horizontal_slider(
    draw: &mut Drawstate,
    id: GuiElemId,
    slider_rect: Rect,
    cur_value: f32,
    depth: f32,
) -> Option<f32> {
    gui_get().horizontal_slider(draw, id, slider_rect, cur_value, depth)
}

pub fn gui_text_scroller(
    draw: &mut Drawstate,
    id: GuiElemId,
    dt: f32,
    rect: Rect,
    font: &SpriteFont,
    font_scale: f32,
    text_color: Color,
    text: &str,
    linecount: usize,
    inout_pos: &mut f32,
    inout_vel: &mut f32,
    inout_acc: &mut f32,
    depth: f32,
) {
    gui_get().text_scroller(
        draw, id, dt, rect, font, font_scale, text_color, text, linecount, inout_pos, inout_vel,
        inout_acc, depth,
    )
}