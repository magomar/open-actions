// specs/006_pomodoro_timer.md
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "FOCUS",
            Self::ShortBreak => "SHORT BREAK",
            Self::LongBreak => "LONG BREAK",
        }
    }

    pub fn is_break(self) -> bool {
        matches!(self, Self::ShortBreak | Self::LongBreak)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    #[default]
    Idle,
    Running,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionEvent {
    FocusCompleted { next_phase: Phase, round: u32 },
    BreakCompleted { next_phase: Phase, round: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PomodoroState {
    pub phase: Phase,
    pub status: TimerStatus,
    pub current_round: u32,
    pub total_rounds: u32,
    pub remaining_seconds: u32,
    pub total_phase_seconds: u32,
}

impl Default for PomodoroState {
    fn default() -> Self {
        Self::new(25, 4)
    }
}

impl PomodoroState {
    pub fn new(focus_mins: u32, total_rounds: u32) -> Self {
        let total_secs = focus_mins.max(1) * 60;
        Self {
            phase: Phase::Focus,
            status: TimerStatus::Idle,
            current_round: 1,
            total_rounds: total_rounds.max(1),
            remaining_seconds: total_secs,
            total_phase_seconds: total_secs,
        }
    }

    pub fn progress_fraction(&self) -> f64 {
        if self.total_phase_seconds == 0 {
            return 0.0;
        }
        (self.remaining_seconds as f64) / (self.total_phase_seconds as f64)
    }

    pub fn format_time(&self) -> String {
        let mins = self.remaining_seconds / 60;
        let secs = self.remaining_seconds % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    pub fn format_round(&self) -> String {
        format!("{}/{}", self.current_round, self.total_rounds)
    }

    pub fn toggle_play_pause(&mut self) {
        match self.status {
            TimerStatus::Idle | TimerStatus::Paused => {
                self.status = TimerStatus::Running;
            }
            TimerStatus::Running => {
                self.status = TimerStatus::Paused;
            }
        }
    }

    pub fn tick(
        &mut self,
        auto_start_breaks: bool,
        auto_start_focus: bool,
        focus_mins: u32,
        short_break_mins: u32,
        long_break_mins: u32,
    ) -> Option<TransitionEvent> {
        if self.status != TimerStatus::Running {
            return None;
        }

        if self.remaining_seconds > 1 {
            self.remaining_seconds -= 1;
            None
        } else {
            // Timer reaches 0:00 -> Transition
            self.remaining_seconds = 0;
            let event = match self.phase {
                Phase::Focus => {
                    let is_last_round = self.current_round >= self.total_rounds;
                    if is_last_round {
                        self.phase = Phase::LongBreak;
                        let secs = long_break_mins.max(1) * 60;
                        self.total_phase_seconds = secs;
                        self.remaining_seconds = secs;
                    } else {
                        self.phase = Phase::ShortBreak;
                        let secs = short_break_mins.max(1) * 60;
                        self.total_phase_seconds = secs;
                        self.remaining_seconds = secs;
                    }

                    if auto_start_breaks {
                        self.status = TimerStatus::Running;
                    } else {
                        self.status = TimerStatus::Idle;
                    }

                    TransitionEvent::FocusCompleted {
                        next_phase: self.phase,
                        round: self.current_round,
                    }
                }
                Phase::ShortBreak => {
                    self.phase = Phase::Focus;
                    self.current_round += 1;
                    let secs = focus_mins.max(1) * 60;
                    self.total_phase_seconds = secs;
                    self.remaining_seconds = secs;

                    if auto_start_focus {
                        self.status = TimerStatus::Running;
                    } else {
                        self.status = TimerStatus::Idle;
                    }

                    TransitionEvent::BreakCompleted {
                        next_phase: self.phase,
                        round: self.current_round,
                    }
                }
                Phase::LongBreak => {
                    self.phase = Phase::Focus;
                    self.current_round = 1;
                    let secs = focus_mins.max(1) * 60;
                    self.total_phase_seconds = secs;
                    self.remaining_seconds = secs;

                    if auto_start_focus {
                        self.status = TimerStatus::Running;
                    } else {
                        self.status = TimerStatus::Idle;
                    }

                    TransitionEvent::BreakCompleted {
                        next_phase: self.phase,
                        round: self.current_round,
                    }
                }
            };
            Some(event)
        }
    }

    pub fn skip(&mut self, focus_mins: u32, short_break_mins: u32, long_break_mins: u32) {
        match self.phase {
            Phase::Focus => {
                let is_last_round = self.current_round >= self.total_rounds;
                if is_last_round {
                    self.phase = Phase::LongBreak;
                    let secs = long_break_mins.max(1) * 60;
                    self.total_phase_seconds = secs;
                    self.remaining_seconds = secs;
                } else {
                    self.phase = Phase::ShortBreak;
                    let secs = short_break_mins.max(1) * 60;
                    self.total_phase_seconds = secs;
                    self.remaining_seconds = secs;
                }
            }
            Phase::ShortBreak => {
                self.phase = Phase::Focus;
                self.current_round += 1;
                let secs = focus_mins.max(1) * 60;
                self.total_phase_seconds = secs;
                self.remaining_seconds = secs;
            }
            Phase::LongBreak => {
                self.phase = Phase::Focus;
                self.current_round = 1;
                let secs = focus_mins.max(1) * 60;
                self.total_phase_seconds = secs;
                self.remaining_seconds = secs;
            }
        }
        self.status = TimerStatus::Paused;
    }

    pub fn reset_phase(&mut self, focus_mins: u32, short_break_mins: u32, long_break_mins: u32) {
        let mins = match self.phase {
            Phase::Focus => focus_mins,
            Phase::ShortBreak => short_break_mins,
            Phase::LongBreak => long_break_mins,
        };
        let secs = mins.max(1) * 60;
        self.total_phase_seconds = secs;
        self.remaining_seconds = secs;
        self.status = TimerStatus::Idle;
    }

    pub fn reset_all(&mut self, focus_mins: u32, total_rounds: u32) {
        let secs = focus_mins.max(1) * 60;
        self.phase = Phase::Focus;
        self.status = TimerStatus::Idle;
        self.current_round = 1;
        self.total_rounds = total_rounds.max(1);
        self.remaining_seconds = secs;
        self.total_phase_seconds = secs;
    }

    pub fn adjust_seconds(&mut self, delta_secs: i32) {
        let current = self.remaining_seconds as i32;
        let next = (current + delta_secs).max(0) as u32;
        self.remaining_seconds = next;
        if next > self.total_phase_seconds {
            self.total_phase_seconds = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_initialization() {
        let state = PomodoroState::new(25, 4);
        assert_eq!(state.phase, Phase::Focus);
        assert_eq!(state.status, TimerStatus::Idle);
        assert_eq!(state.current_round, 1);
        assert_eq!(state.total_rounds, 4);
        assert_eq!(state.remaining_seconds, 1500);
        assert_eq!(state.total_phase_seconds, 1500);
        assert_eq!(state.format_time(), "25:00");
        assert_eq!(state.format_round(), "1/4");
        assert!((state.progress_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn toggle_play_pause_and_resume() {
        let mut state = PomodoroState::new(25, 4);
        assert_eq!(state.status, TimerStatus::Idle);

        state.toggle_play_pause();
        assert_eq!(state.status, TimerStatus::Running);

        state.toggle_play_pause();
        assert_eq!(state.status, TimerStatus::Paused);

        state.toggle_play_pause();
        assert_eq!(state.status, TimerStatus::Running);
    }

    #[test]
    fn tick_decrements_seconds_when_running() {
        let mut state = PomodoroState::new(25, 4);
        state.status = TimerStatus::Running;
        state.remaining_seconds = 100;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(event, None);
        assert_eq!(state.remaining_seconds, 99);
    }

    #[test]
    fn tick_does_nothing_when_paused_or_idle() {
        let mut state = PomodoroState::new(25, 4);
        state.status = TimerStatus::Paused;
        state.remaining_seconds = 100;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(event, None);
        assert_eq!(state.remaining_seconds, 100);
    }

    #[test]
    fn transitions_from_focus_to_short_break_on_intermediate_round() {
        let mut state = PomodoroState::new(25, 4);
        state.status = TimerStatus::Running;
        state.current_round = 1;
        state.remaining_seconds = 1;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(
            event,
            Some(TransitionEvent::FocusCompleted {
                next_phase: Phase::ShortBreak,
                round: 1,
            })
        );
        assert_eq!(state.phase, Phase::ShortBreak);
        assert_eq!(state.current_round, 1);
        assert_eq!(state.remaining_seconds, 300);
        assert_eq!(state.total_phase_seconds, 300);
        assert_eq!(state.status, TimerStatus::Idle);
    }

    #[test]
    fn transitions_from_focus_to_long_break_on_final_round() {
        let mut state = PomodoroState::new(25, 4);
        state.status = TimerStatus::Running;
        state.current_round = 4;
        state.remaining_seconds = 1;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(
            event,
            Some(TransitionEvent::FocusCompleted {
                next_phase: Phase::LongBreak,
                round: 4,
            })
        );
        assert_eq!(state.phase, Phase::LongBreak);
        assert_eq!(state.current_round, 4);
        assert_eq!(state.remaining_seconds, 900);
        assert_eq!(state.total_phase_seconds, 900);
        assert_eq!(state.status, TimerStatus::Idle);
    }

    #[test]
    fn transitions_from_short_break_to_next_focus_round() {
        let mut state = PomodoroState::new(25, 4);
        state.phase = Phase::ShortBreak;
        state.status = TimerStatus::Running;
        state.current_round = 1;
        state.remaining_seconds = 1;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(
            event,
            Some(TransitionEvent::BreakCompleted {
                next_phase: Phase::Focus,
                round: 2,
            })
        );
        assert_eq!(state.phase, Phase::Focus);
        assert_eq!(state.current_round, 2);
        assert_eq!(state.remaining_seconds, 1500);
    }

    #[test]
    fn transitions_from_long_break_to_round_one_focus() {
        let mut state = PomodoroState::new(25, 4);
        state.phase = Phase::LongBreak;
        state.status = TimerStatus::Running;
        state.current_round = 4;
        state.remaining_seconds = 1;

        let event = state.tick(false, false, 25, 5, 15);
        assert_eq!(
            event,
            Some(TransitionEvent::BreakCompleted {
                next_phase: Phase::Focus,
                round: 1,
            })
        );
        assert_eq!(state.phase, Phase::Focus);
        assert_eq!(state.current_round, 1);
        assert_eq!(state.remaining_seconds, 1500);
    }

    #[test]
    fn auto_start_breaks_and_focus() {
        let mut state = PomodoroState::new(25, 4);
        state.status = TimerStatus::Running;
        state.current_round = 1;
        state.remaining_seconds = 1;

        state.tick(true, false, 25, 5, 15);
        assert_eq!(state.phase, Phase::ShortBreak);
        assert_eq!(state.status, TimerStatus::Running);

        state.remaining_seconds = 1;
        state.tick(false, true, 25, 5, 15);
        assert_eq!(state.phase, Phase::Focus);
        assert_eq!(state.status, TimerStatus::Running);
    }

    #[test]
    fn manual_skip_advances_phase() {
        let mut state = PomodoroState::new(25, 4);
        state.skip(25, 5, 15);
        assert_eq!(state.phase, Phase::ShortBreak);
        assert_eq!(state.remaining_seconds, 300);
        assert_eq!(state.status, TimerStatus::Paused);

        state.skip(25, 5, 15);
        assert_eq!(state.phase, Phase::Focus);
        assert_eq!(state.current_round, 2);
        assert_eq!(state.remaining_seconds, 1500);
    }

    #[test]
    fn reset_phase_restores_initial_seconds_without_round_change() {
        let mut state = PomodoroState::new(25, 4);
        state.current_round = 3;
        state.remaining_seconds = 120;
        state.status = TimerStatus::Running;

        state.reset_phase(25, 5, 15);
        assert_eq!(state.current_round, 3);
        assert_eq!(state.remaining_seconds, 1500);
        assert_eq!(state.status, TimerStatus::Idle);
    }

    #[test]
    fn adjust_seconds_clamps_at_zero() {
        let mut state = PomodoroState::new(25, 4);
        state.remaining_seconds = 30;

        state.adjust_seconds(60);
        assert_eq!(state.remaining_seconds, 90);

        state.adjust_seconds(-150);
        assert_eq!(state.remaining_seconds, 0);
    }
}
