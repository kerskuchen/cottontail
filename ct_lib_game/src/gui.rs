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
    Increase,
    Decrease,
}

#[derive(Debug)]
pub struct GuiState {
    mouse_pos_canvas: Canvaspoint,
    mouse_is_down: bool,
    mouse_highlighted_item: Option<GuiElemId>,

    finger_pos_canvas: Option<Canvaspoint>,
    finger_pos_canvas_previous: Vec2,

    keyboard_highlighted_item: Option<GuiElemId>,
    active_item: Option<GuiElemId>,

    current_action: Option<GuiAction>,
    last_widget: Option<GuiElemId>,
}

impl GuiState {
    pub fn new() -> GuiState {
        GuiState {
            mouse_pos_canvas: Vec2::zero(),
            mouse_is_down: false,
            mouse_highlighted_item: None,

            finger_pos_canvas: None,
            finger_pos_canvas_previous: -Vec2::ones(),

            keyboard_highlighted_item: None,
            active_item: None,

            current_action: None,
            last_widget: None,
        }
    }
    pub fn begin_frame(&mut self, cursors: &Cursors, input: &InputState) {
        self.finger_pos_canvas = cursors.finger_primary.map(|coords| coords.pos_canvas);
        self.mouse_is_down = input.mouse.button_left.is_pressed;
        self.mouse_pos_canvas = cursors.mouse.pos_canvas;

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
        } else {
            None
        };
    }

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
        if self.mouse_pos_canvas.intersects_rect(button_rect) {
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

        if self.mouse_pos_canvas.intersects_rect(slider_rect) {
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
                self.mouse_pos_canvas.x - (slider_rect.pos.x),
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
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Easy API

static mut GUI_STATE: Option<GuiState> = None;

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

pub fn gui_begin_frame(cursors: &Cursors, input: &InputState) {
    gui_get().begin_frame(cursors, input)
}
pub fn gui_end_frame() {
    gui_get().end_frame()
}
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
