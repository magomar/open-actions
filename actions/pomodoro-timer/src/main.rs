// specs/006_pomodoro_timer.md
pub mod render;
pub mod settings;
pub mod sound;
pub mod state;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_instance, register_action, run,
};
use render::{render_reset_tile, render_skip_tile, render_timer_tile};
use serde_json::Value;
use settings::PomodoroSettings;
use sound::{play_break_complete, play_focus_complete};
use state::{PomodoroState, TransitionEvent};
use tokio::sync::{Mutex, RwLock};

pub const TIMER_UUID: &str = "io.github.mario.pomodorotimer.timer";
pub const SKIP_UUID: &str = "io.github.mario.pomodorotimer.skip";
pub const RESET_UUID: &str = "io.github.mario.pomodorotimer.reset";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Timer,
    Skip,
    Reset,
}

pub struct Core {
    pub state: Mutex<PomodoroState>,
    pub settings: RwLock<PomodoroSettings>,
    pub instances: Mutex<HashMap<String, ActionKind>>,
    pub press_down: Mutex<HashMap<String, Instant>>,
    pub last_click: Mutex<HashMap<String, Instant>>,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    pub fn new() -> Self {
        let default_settings = PomodoroSettings::default();
        let initial_state = PomodoroState::new(
            default_settings.focus_duration_mins,
            default_settings.rounds,
        );
        Self {
            state: Mutex::new(initial_state),
            settings: RwLock::new(default_settings),
            instances: Mutex::default(),
            press_down: Mutex::default(),
            last_click: Mutex::default(),
        }
    }

    pub fn start_engine(core: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let (event, sound_enabled, volume) = {
                    let settings = core.settings.read().await;
                    let mut state = core.state.lock().await;
                    let ev = state.tick(
                        settings.auto_start_breaks,
                        settings.auto_start_focus,
                        settings.focus_duration_mins,
                        settings.short_break_duration_mins,
                        settings.long_break_duration_mins,
                    );
                    (ev, settings.sound_enabled, settings.volume)
                };

                if let Some(ev) = event {
                    if sound_enabled {
                        match ev {
                            TransitionEvent::FocusCompleted { .. } => {
                                play_focus_complete(volume);
                            }
                            TransitionEvent::BreakCompleted { .. } => {
                                play_break_complete(volume);
                            }
                        }
                    }
                }

                core.refresh_all().await;
            }
        });
    }

    pub async fn register_instance(&self, id: String, kind: ActionKind) {
        let mut instances = self.instances.lock().await;
        instances.insert(id, kind);
    }

    pub async fn unregister_instance(&self, id: &str) {
        let mut instances = self.instances.lock().await;
        instances.remove(id);
        let mut downs = self.press_down.lock().await;
        downs.remove(id);
        let mut clicks = self.last_click.lock().await;
        clicks.remove(id);
    }

    pub async fn refresh_instance(&self, instance: &Instance, kind: ActionKind) -> OpenActionResult<()> {
        let theme = {
            let s = self.settings.read().await;
            s.resolve_theme()
        };

        let image_data = match kind {
            ActionKind::Timer => {
                let st = self.state.lock().await;
                render_timer_tile(&st, &theme)
            }
            ActionKind::Skip => render_skip_tile(&theme),
            ActionKind::Reset => render_reset_tile(&theme),
        };

        instance.set_image(Some(image_data), None).await
    }

    pub async fn refresh_all(&self) {
        let list: Vec<(String, ActionKind)> = {
            let instances = self.instances.lock().await;
            instances.iter().map(|(k, v)| (k.clone(), *v)).collect()
        };

        for (id, kind) in list {
            if let Some(inst) = get_instance(id).await {
                let _ = self.refresh_instance(&inst, kind).await;
            }
        }
    }

    pub async fn handle_key_down(&self, id: &str) {
        let mut downs = self.press_down.lock().await;
        downs.insert(id.to_string(), Instant::now());
    }

    pub async fn handle_key_up(&self, id: &str, kind: ActionKind) {
        let elapsed = {
            let mut downs = self.press_down.lock().await;
            downs.remove(id).map(|start| start.elapsed())
        };

        let is_long_press = elapsed.is_some_and(|dur| dur >= Duration::from_millis(600));

        let s = self.settings.read().await.clone();
        let mut state = self.state.lock().await;

        match kind {
            ActionKind::Timer => {
                if is_long_press {
                    state.skip(
                        s.focus_duration_mins,
                        s.short_break_duration_mins,
                        s.long_break_duration_mins,
                    );
                } else {
                    let is_double_click = {
                        let mut clicks = self.last_click.lock().await;
                        let now = Instant::now();
                        if let Some(prev) = clicks.get(id) {
                            if now.duration_since(*prev) < Duration::from_millis(400) {
                                clicks.remove(id);
                                true
                            } else {
                                clicks.insert(id.to_string(), now);
                                false
                            }
                        } else {
                            clicks.insert(id.to_string(), now);
                            false
                        }
                    };

                    if is_double_click {
                        state.reset_phase(
                            s.focus_duration_mins,
                            s.short_break_duration_mins,
                            s.long_break_duration_mins,
                        );
                    } else {
                        state.toggle_play_pause();
                    }
                }
            }
            ActionKind::Skip => {
                state.skip(
                    s.focus_duration_mins,
                    s.short_break_duration_mins,
                    s.long_break_duration_mins,
                );
            }
            ActionKind::Reset => {
                if is_long_press {
                    state.reset_all(s.focus_duration_mins, s.rounds);
                } else {
                    state.reset_phase(
                        s.focus_duration_mins,
                        s.short_break_duration_mins,
                        s.long_break_duration_mins,
                    );
                }
            }
        }
        drop(state);
        self.refresh_all().await;
    }

    pub async fn handle_dial_rotate(&self, ticks: i16) {
        let delta_secs = (ticks as i32) * 60;
        let mut state = self.state.lock().await;
        state.adjust_seconds(delta_secs);
        drop(state);
        self.refresh_all().await;
    }

    pub async fn handle_dial_down(&self) {
        let mut state = self.state.lock().await;
        state.toggle_play_pause();
        drop(state);
        self.refresh_all().await;
    }

    pub async fn apply_settings(&self, new_settings: PomodoroSettings) {
        let mut s = self.settings.write().await;
        let mut state = self.state.lock().await;

        if state.status == state::TimerStatus::Idle && state.phase == state::Phase::Focus {
            state.total_rounds = new_settings.rounds.max(1);
            let total_secs = new_settings.focus_duration_mins.max(1) * 60;
            state.total_phase_seconds = total_secs;
            state.remaining_seconds = total_secs;
        }
        *s = new_settings;
        drop(state);
        drop(s);
        self.refresh_all().await;
    }
}

pub struct TimerAction(pub Arc<Core>);
#[async_trait]
impl Action for TimerAction {
    const UUID: &'static str = TIMER_UUID;
    type Settings = PomodoroSettings;
    async fn will_appear(&self, i: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.register_instance(i.instance_id.clone(), ActionKind::Timer).await;
        self.0.apply_settings(s.clone()).await;
        self.0.refresh_instance(i, ActionKind::Timer).await
    }
    async fn will_disappear(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.unregister_instance(&i.instance_id).await;
        Ok(())
    }
    async fn key_down(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_down(&i.instance_id).await;
        Ok(())
    }
    async fn key_up(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_up(&i.instance_id, ActionKind::Timer).await;
        Ok(())
    }
    async fn dial_rotate(&self, _: &Instance, _: &PomodoroSettings, ticks: i16, _: bool) -> OpenActionResult<()> {
        self.0.handle_dial_rotate(ticks).await;
        Ok(())
    }
    async fn dial_down(&self, _: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_dial_down().await;
        Ok(())
    }
    async fn did_receive_settings(&self, _: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.apply_settings(s.clone()).await;
        Ok(())
    }
    async fn send_to_plugin(&self, i: &Instance, _: &PomodoroSettings, payload: &Value) -> OpenActionResult<()> {
        if let Some(event) = payload.get("event").and_then(Value::as_str) {
            if event == "reset_defaults" {
                let defaults = PomodoroSettings::default();
                i.set_settings(&defaults).await?;
                self.0.apply_settings(defaults).await;
            }
        }
        Ok(())
    }
}

pub struct SkipAction(pub Arc<Core>);
#[async_trait]
impl Action for SkipAction {
    const UUID: &'static str = SKIP_UUID;
    type Settings = PomodoroSettings;
    async fn will_appear(&self, i: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.register_instance(i.instance_id.clone(), ActionKind::Skip).await;
        self.0.apply_settings(s.clone()).await;
        self.0.refresh_instance(i, ActionKind::Skip).await
    }
    async fn will_disappear(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.unregister_instance(&i.instance_id).await;
        Ok(())
    }
    async fn key_down(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_down(&i.instance_id).await;
        Ok(())
    }
    async fn key_up(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_up(&i.instance_id, ActionKind::Skip).await;
        Ok(())
    }
    async fn did_receive_settings(&self, _: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.apply_settings(s.clone()).await;
        Ok(())
    }
}

pub struct ResetAction(pub Arc<Core>);
#[async_trait]
impl Action for ResetAction {
    const UUID: &'static str = RESET_UUID;
    type Settings = PomodoroSettings;
    async fn will_appear(&self, i: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.register_instance(i.instance_id.clone(), ActionKind::Reset).await;
        self.0.apply_settings(s.clone()).await;
        self.0.refresh_instance(i, ActionKind::Reset).await
    }
    async fn will_disappear(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.unregister_instance(&i.instance_id).await;
        Ok(())
    }
    async fn key_down(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_down(&i.instance_id).await;
        Ok(())
    }
    async fn key_up(&self, i: &Instance, _: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.handle_key_up(&i.instance_id, ActionKind::Reset).await;
        Ok(())
    }
    async fn did_receive_settings(&self, _: &Instance, s: &PomodoroSettings) -> OpenActionResult<()> {
        self.0.apply_settings(s.clone()).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let core = Arc::new(Core::new());
    Core::start_engine(Arc::clone(&core));

    register_action(TimerAction(Arc::clone(&core))).await;
    register_action(SkipAction(Arc::clone(&core))).await;
    register_action(ResetAction(Arc::clone(&core))).await;

    run(std::env::args().collect()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn core_lifecycle_and_key_handling() {
        let core = Arc::new(Core::new());
        core.register_instance("btn-1".to_string(), ActionKind::Timer).await;

        // Start countdown
        core.handle_key_down("btn-1").await;
        core.handle_key_up("btn-1", ActionKind::Timer).await;

        {
            let st = core.state.lock().await;
            assert_eq!(st.status, state::TimerStatus::Running);
        }

        // Wait past double-click threshold to test pause
        tokio::time::sleep(Duration::from_millis(450)).await;

        // Pause countdown
        core.handle_key_down("btn-1").await;
        core.handle_key_up("btn-1", ActionKind::Timer).await;

        {
            let st = core.state.lock().await;
            assert_eq!(st.status, state::TimerStatus::Paused);
        }
    }

    #[tokio::test]
    async fn core_dial_adjustment() {
        let core = Arc::new(Core::new());
        {
            let st = core.state.lock().await;
            assert_eq!(st.remaining_seconds, 1500); // 25 min
        }

        core.handle_dial_rotate(2).await;
        {
            let st = core.state.lock().await;
            assert_eq!(st.remaining_seconds, 1620); // +2 min = 27 min
        }
    }
}
