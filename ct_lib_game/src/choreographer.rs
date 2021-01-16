use std::collections::HashMap;

use super::*;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Gate

pub struct Gate {
    pub is_open: bool,
}

impl Gate {
    pub fn new_opened() -> Gate {
        Gate { is_open: true }
    }

    pub fn new_closed() -> Gate {
        Gate { is_open: false }
    }

    pub fn open(&mut self) -> bool {
        let was_opened = self.is_open;
        self.is_open = true;
        was_opened
    }

    pub fn close(&mut self) -> bool {
        let was_opened = self.is_open;
        self.is_open = false;
        was_opened
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Simple timer

#[derive(Debug, Clone, Copy)]
pub struct TimerSimple {
    pub time_cur: f32,
    pub time_end: f32,
}

impl Default for TimerSimple {
    fn default() -> Self {
        TimerSimple::new_started(1.0)
    }
}

impl TimerSimple {
    pub fn new_started(end_time: f32) -> TimerSimple {
        TimerSimple {
            time_cur: 0.0,
            time_end: end_time,
        }
    }

    pub fn new_stopped(end_time: f32) -> TimerSimple {
        TimerSimple {
            time_cur: end_time,
            time_end: end_time,
        }
    }

    pub fn update(&mut self, deltatime: f32) {
        self.time_cur = f32::min(self.time_cur + deltatime, self.time_end);
    }

    pub fn update_and_check_if_triggered(&mut self, deltatime: f32) -> bool {
        let time_previous = self.time_cur;
        self.time_cur = f32::min(self.time_cur + deltatime, self.time_end);

        self.time_cur == self.time_end && time_previous != self.time_end
    }

    pub fn is_running(&self) -> bool {
        self.time_cur < self.time_end
    }

    pub fn is_finished(&self) -> bool {
        !self.is_running()
    }

    pub fn completion_ratio(&self) -> f32 {
        self.time_cur / self.time_end
    }

    pub fn stop(&mut self) {
        self.time_cur = self.time_end;
    }

    pub fn restart(&mut self) {
        self.time_cur = 0.0;
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Timer

#[derive(Debug, Clone, Copy)]
pub enum Timerstate {
    Running {
        completion_ratio: f32,
    },
    Triggered {
        trigger_count: u64,
        remaining_delta: f32,
    },
    Paused,
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    time_cur: f32,
    time_end: f32,
    trigger_count: u64,
    trigger_count_max: u64,
    pub is_paused: bool,
}

impl Timer {
    pub fn new_started(trigger_time: f32) -> Timer {
        Timer {
            time_cur: 0.0,
            time_end: trigger_time,
            trigger_count: 0,
            trigger_count_max: 1,
            is_paused: false,
        }
    }

    pub fn new_stopped(trigger_time: f32) -> Timer {
        Timer {
            time_cur: trigger_time,
            time_end: trigger_time,
            trigger_count: 1,
            trigger_count_max: 1,
            is_paused: false,
        }
    }

    pub fn new_repeating_started(trigger_time: f32) -> Timer {
        Timer {
            time_cur: 0.0,
            time_end: trigger_time,
            trigger_count: 0,
            trigger_count_max: std::u64::MAX,
            is_paused: false,
        }
    }

    pub fn new_repeating_stopped(trigger_time: f32) -> Timer {
        Timer {
            time_cur: trigger_time,
            time_end: trigger_time,
            trigger_count: std::u64::MAX,
            trigger_count_max: std::u64::MAX,
            is_paused: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.trigger_count == self.trigger_count_max
    }

    pub fn is_running(&self) -> bool {
        !self.is_finished()
    }

    pub fn completion_ratio(&self) -> f32 {
        (self.time_cur % self.time_end) / self.time_end
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = true;
    }

    pub fn stop(&mut self) {
        self.time_cur = self.time_end;
        self.trigger_count = self.trigger_count_max;
    }

    pub fn restart(&mut self) {
        self.time_cur = 0.0;
        self.trigger_count = 0;
    }

    pub fn update(&mut self, deltatime: f32) -> Timerstate {
        if self.trigger_count >= self.trigger_count_max {
            return Timerstate::Done;
        }
        if self.is_paused {
            return Timerstate::Paused;
        }

        self.time_cur += deltatime;

        if self.time_cur > self.time_end {
            self.time_cur -= self.time_end;
            self.trigger_count += 1;

            let remaining_delta = if self.trigger_count == self.trigger_count_max {
                // NOTE: This was the last possible trigger event so we also return any
                //       remaining time we accumulated and set the current time to its max so that
                //       the completion ratio is still correct.
                let remainder = self.time_cur;
                self.time_cur = self.time_end;
                remainder
            } else {
                0.0
            };

            return Timerstate::Triggered {
                trigger_count: self.trigger_count,
                remaining_delta,
            };
        }

        Timerstate::Running {
            completion_ratio: self.completion_ratio(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Special timers

#[derive(Debug, Clone, Copy)]
pub struct TriggerRepeating {
    timer: Timer,
    triggertime_initial: f32,
    triggertime_repeating: f32,
}

impl TriggerRepeating {
    #[inline]
    pub fn new(trigger_time: f32) -> TriggerRepeating {
        TriggerRepeating {
            timer: Timer::new_repeating_started(trigger_time),
            triggertime_initial: trigger_time,
            triggertime_repeating: trigger_time,
        }
    }

    #[inline]
    pub fn new_with_distinct_triggertimes(
        trigger_time_initial: f32,
        trigger_time_repeat: f32,
    ) -> TriggerRepeating {
        TriggerRepeating {
            timer: Timer::new_repeating_started(trigger_time_initial),
            triggertime_initial: trigger_time_initial,
            triggertime_repeating: trigger_time_repeat,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.timer = Timer::new_repeating_started(self.triggertime_initial);
    }

    #[inline]
    pub fn completion_ratio(&self) -> f32 {
        self.timer.completion_ratio()
    }

    /// Returns true if actually triggered
    #[inline]
    pub fn update_and_check(&mut self, deltatime: f32) -> bool {
        match self.timer.update(deltatime) {
            Timerstate::Triggered { trigger_count, .. } => {
                if trigger_count == 1 {
                    self.timer.time_end = self.triggertime_repeating;
                }
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TimerStateSwitchBinary {
    pub repeat_timer: TriggerRepeating,
    pub active: bool,
}

impl TimerStateSwitchBinary {
    pub fn new(start_active: bool, start_time: f32, phase_duration: f32) -> TimerStateSwitchBinary {
        TimerStateSwitchBinary {
            repeat_timer: TriggerRepeating::new_with_distinct_triggertimes(
                start_time,
                phase_duration,
            ),
            active: start_active,
        }
    }
    pub fn update_and_check(&mut self, deltatime: f32) -> bool {
        if self.repeat_timer.update_and_check(deltatime) {
            self.active = !self.active;
        }
        self.active
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Choreographer

#[derive(Debug, Clone)]
pub struct Choreographer {
    current_stage: usize,
    stages: Vec<Timer>,
    specials: HashMap<String, usize>,
    pub time_accumulator: f32,
}

impl Choreographer {
    pub fn new() -> Choreographer {
        Choreographer {
            current_stage: 0,
            stages: Vec::new(),
            specials: HashMap::new(),
            time_accumulator: 0.0,
        }
    }

    pub fn restart(&mut self) {
        self.current_stage = 0;
        self.stages.clear();
        self.time_accumulator = 0.0;
    }

    pub fn update(&mut self, deltatime: f32) -> &mut Self {
        self.current_stage = 0;
        self.time_accumulator += deltatime;
        self
    }

    pub fn get_previous_triggercount(&self) -> u64 {
        assert!(self.current_stage > 0);
        self.stages[self.current_stage - 1].trigger_count
    }

    /// NOTE: This only resets the last `current_time` and `trigger_time` but NOT
    ///       the `trigger_count`
    pub fn reset_previous(&mut self, new_delay: f32) {
        assert!(self.current_stage > 0);
        self.stages[self.current_stage - 1].time_cur = 0.0;
        self.stages[self.current_stage - 1].time_end = new_delay;
    }

    pub fn wait(&mut self, delay: f32) -> bool {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_started(delay));
        }
        let timer = &mut self.stages[current_stage];

        match timer.update(self.time_accumulator) {
            Timerstate::Triggered {
                remaining_delta, ..
            } => {
                self.time_accumulator = remaining_delta;
                true
            }
            Timerstate::Done => true,
            Timerstate::Running { .. } => {
                self.time_accumulator = 0.0;
                false
            }
            Timerstate::Paused => unreachable!(),
        }
    }

    pub fn tween(&mut self, tween_time: f32) -> (f32, bool) {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_started(tween_time));
        }
        let timer = &mut self.stages[current_stage];

        match timer.update(self.time_accumulator) {
            Timerstate::Triggered {
                remaining_delta, ..
            } => {
                self.time_accumulator = remaining_delta;
                (1.0, true)
            }
            Timerstate::Done => (1.0, true),
            Timerstate::Running { completion_ratio } => {
                self.time_accumulator = 0.0;
                (completion_ratio, false)
            }
            Timerstate::Paused => unreachable!(),
        }
    }

    pub fn once(&mut self) -> bool {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_stopped(1.0));
            return true;
        }

        false
    }

    pub fn hitcount(&mut self) -> u64 {
        let current_stage = self.current_stage;
        self.current_stage += 1;

        if self.stages.len() <= current_stage {
            self.stages.push(Timer::new_repeating_started(0.0));
        }

        let timer = &mut self.stages[current_stage];
        timer.trigger_count
    }
}

/*
// ALTERNATIVE CHOREOGRAPHER EXPERIMENT
let mut choreo_test = ChoreographerTest::new();
choreo_test
    .update(time_deltatime())
    .then_tween(1.0, &mut |tween_percent| {
        self.circle_radius = lerp(20.0, 50.0, easing::cubic_inout(tween_percent))
    })
    .then_wait(1.0)
    .then_subroutine(&mut |choreo| {
        choreo
            .then_start_repeat(5)
            .then_tween(1.0, &mut |tween_percent| {
                self.circle_radius = lerp(20.0, 50.0, easing::cubic_inout(tween_percent))
            })
            .then_wait(1.0)
            .then_stop_repeat();
    })
    .then_tween(1.0, &mut |tween_percent| {
        self.circle_radius = lerp(50.0, 20.0, easing::cubic_inout(tween_percent));
    })
    .then_restart();

#[derive(Debug, Clone)]
pub struct ChoreographerTest {
    current_stage: usize,
    stages: Vec<Timer>,
    specials: HashMap<String, usize>,
    pub time_accumulator: f32,
}

impl ChoreographerTest {
    pub fn new() -> ChoreographerTest {
        ChoreographerTest {
            current_stage: 0,
            stages: Vec::new(),
            specials: HashMap::new(),
            time_accumulator: 0.0,
        }
    }

    pub fn update(&mut self, deltatime: f32) -> &mut Self {
        self.current_stage = 0;
        self.time_accumulator += deltatime;
        self
    }

    pub fn then_restart(&mut self) {
        self.current_stage = 0;
        self.stages.clear();
        self.time_accumulator = 0.0;
    }

    pub fn then_wait(&mut self, time: f32) -> &mut Self {
        todo!();
        self
    }

    pub fn then_subroutine(
        &mut self,
        function: &mut impl FnMut(&mut ChoreographerTest),
    ) -> &mut Self {
        function(self);
        todo!();
        self
    }

    pub fn then_tween(&mut self, time: f32, function: &mut impl FnMut(f32)) -> &mut Self {
        let percent = time;
        function(percent);
        todo!();
        self
    }

    pub fn then_start_repeat(&mut self, repeatcount: usize) -> &mut Self {
        todo!();
        self
    }
    pub fn then_stop_repeat(&mut self) -> &mut Self {
        todo!();
        self
    }
}
*/

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fader

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Fadestate {
    FadedIn,
    FadedOut,
    FadingIn,
    FadingOut,
}

#[derive(Clone)]
pub struct Fader {
    pub timer: TimerSimple,
    pub state: Fadestate,
}

impl Fader {
    pub fn new_faded_out() -> Fader {
        Fader {
            timer: TimerSimple::new_stopped(1.0),
            state: Fadestate::FadedOut,
        }
    }
    pub fn new_faded_in() -> Fader {
        Fader {
            timer: TimerSimple::new_stopped(1.0),
            state: Fadestate::FadedIn,
        }
    }

    pub fn start_fading_out(&mut self, fade_out_time: f32) {
        self.state = Fadestate::FadingOut;
        self.timer = TimerSimple::new_started(fade_out_time);
    }

    pub fn start_fading_in(&mut self, fade_in_time: f32) {
        self.state = Fadestate::FadingIn;
        self.timer = TimerSimple::new_started(fade_in_time);
    }

    pub fn opacity(&self) -> f32 {
        match self.state {
            Fadestate::FadedIn => 1.0,
            Fadestate::FadedOut => 0.0,
            Fadestate::FadingIn => self.timer.completion_ratio(),
            Fadestate::FadingOut => 1.0 - self.timer.completion_ratio(),
        }
    }

    pub fn update(&mut self, deltatime: f32) {
        if self.state == Fadestate::FadedIn || self.state == Fadestate::FadedOut {
            return;
        }

        self.timer.update(deltatime);

        if self.timer.is_finished() {
            if self.state == Fadestate::FadingIn {
                self.state = Fadestate::FadedIn;
            } else {
                self.state = Fadestate::FadedOut;
            }
        }
    }

    pub fn is_fading(self) -> bool {
        self.state == Fadestate::FadingIn || self.state == Fadestate::FadingOut
    }

    pub fn is_finished(self) -> bool {
        self.state == Fadestate::FadedIn || self.state == Fadestate::FadedOut
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ScreenFader

#[derive(Clone)]
pub struct CanvasFader {
    pub color_start: Color,
    pub color_end: Color,
    pub timer: TimerSimple,
}

impl CanvasFader {
    pub fn new(color_start: Color, color_end: Color, fade_time_seconds: f32) -> CanvasFader {
        CanvasFader {
            color_start,
            color_end,
            timer: TimerSimple::new_started(fade_time_seconds),
        }
    }

    pub fn completion_ratio(&self) -> f32 {
        self.timer.completion_ratio()
    }

    pub fn update_and_draw(&mut self, deltatime: f32, canvas_width: u32, canvas_height: u32) {
        self.timer.update(deltatime);

        let percent = self.timer.completion_ratio();
        let color = Color::mix(self.color_start, self.color_end, percent);
        if color.a > 0.0 {
            draw_rect(
                Rect::from_width_height(canvas_width as f32, canvas_height as f32),
                true,
                Drawparams::new(
                    DEPTH_SCREEN_FADER,
                    color,
                    ADDITIVITY_NONE,
                    Drawspace::Canvas,
                ),
            );
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Splashscreen

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadingScreenState {
    Idle,
    StartedFadingIn,
    IsFadingIn,
    FinishedFadingIn,
    Sustain,
    StartedFadingOut,
    IsFadingOut,
    FinishedFadingOut,
    IsDone,
}

#[derive(Clone)]
pub struct LoadingScreen {
    time_fade_in: f32,
    time_fade_out: f32,
    color_progressbar: Color,

    fader: CanvasFader,
    state: LoadingScreenState,
}

impl LoadingScreen {
    pub fn new(time_fade_in: f32, time_fade_out: f32, color_progressbar: Color) -> LoadingScreen {
        LoadingScreen {
            time_fade_in,
            time_fade_out,
            color_progressbar,
            fader: CanvasFader::new(Color::black(), Color::white(), time_fade_in),
            state: LoadingScreenState::StartedFadingIn,
        }
    }

    pub fn start_fading_in(&mut self) {
        self.state = LoadingScreenState::StartedFadingIn;
        self.fader = CanvasFader::new(Color::black(), Color::white(), self.time_fade_in);
    }

    pub fn start_fading_out(&mut self) {
        self.state = LoadingScreenState::StartedFadingOut;
        self.fader = CanvasFader::new(Color::white(), Color::transparent(), self.time_fade_out);
    }

    pub fn is_faded_in(&self) -> bool {
        self.state == LoadingScreenState::Sustain
    }

    pub fn is_faded_out(&self) -> bool {
        self.state == LoadingScreenState::IsDone
    }

    pub fn update_and_draw(
        &mut self,
        deltatime: f32,
        canvas_width: u32,
        canvas_height: u32,
        sprite: &Sprite,
        progress_percentage: Option<f32>,
    ) {
        if self.state == LoadingScreenState::IsDone || self.state == LoadingScreenState::Idle {
            return;
        }

        self.fader
            .update_and_draw(deltatime, canvas_width, canvas_height);

        let opacity = if self.state <= LoadingScreenState::Sustain {
            self.fader.completion_ratio()
        } else {
            1.0 - self.fader.completion_ratio()
        };

        let (splash_rect, letterbox_rects) = letterbox_rects_create(
            sprite.untrimmed_dimensions.x as i32,
            sprite.untrimmed_dimensions.y as i32,
            canvas_width as i32,
            canvas_height as i32,
        );
        draw_sprite(
            &sprite,
            Transform::from_pos(Vec2::new(
                splash_rect.left() as f32,
                splash_rect.top() as f32,
            )),
            false,
            false,
            Drawparams::new(
                DEPTH_SPLASH,
                opacity * Color::white(),
                ADDITIVITY_NONE,
                Drawspace::Canvas,
            ),
        );

        for letterbox_rect in &letterbox_rects {
            draw_rect(
                Rect::from(*letterbox_rect),
                true,
                Drawparams::new(
                    DEPTH_SCREEN_FADER,
                    opacity * Color::white(),
                    ADDITIVITY_NONE,
                    Drawspace::Canvas,
                ),
            );
        }

        // Draw progress bar
        if let Some(progress_percentage) = progress_percentage {
            let progress_bar_height = (canvas_height as f32 / 25.0).round();
            draw_rect(
                Rect::from_bounds_left_top_right_bottom(
                    0.0,
                    canvas_height as f32 - progress_bar_height,
                    canvas_width as f32 * progress_percentage,
                    canvas_height as f32,
                ),
                true,
                Drawparams::new(
                    DEPTH_SCREEN_FADER,
                    opacity * self.color_progressbar,
                    ADDITIVITY_NONE,
                    Drawspace::Canvas,
                ),
            );
        }

        match self.state {
            LoadingScreenState::Idle => {}
            LoadingScreenState::StartedFadingIn => {
                self.state = LoadingScreenState::IsFadingIn;
            }
            LoadingScreenState::IsFadingIn => {
                if self.fader.completion_ratio() == 1.0 {
                    self.state = LoadingScreenState::FinishedFadingIn;
                }
            }
            LoadingScreenState::FinishedFadingIn => {
                self.state = LoadingScreenState::Sustain;
            }
            LoadingScreenState::Sustain => {}
            LoadingScreenState::StartedFadingOut => {
                self.state = LoadingScreenState::IsFadingOut;
            }
            LoadingScreenState::IsFadingOut => {
                if self.fader.completion_ratio() == 1.0 {
                    self.state = LoadingScreenState::FinishedFadingOut;
                }
            }
            LoadingScreenState::FinishedFadingOut => {
                self.state = LoadingScreenState::IsDone;
            }
            LoadingScreenState::IsDone => {}
        }
    }
}
