// specs/006_pomodoro_timer.md
use crate::render::ThemeColors;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroSettings {
    pub focus_duration_mins: u32,
    pub short_break_duration_mins: u32,
    pub long_break_duration_mins: u32,
    pub rounds: u32,
    pub auto_start_breaks: bool,
    pub auto_start_focus: bool,
    pub sound_enabled: bool,
    pub volume: u8,
    pub theme: String,
    pub custom_focus_color: Option<String>,
    pub custom_short_break_color: Option<String>,
    pub custom_long_break_color: Option<String>,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        Self {
            focus_duration_mins: 25,
            short_break_duration_mins: 5,
            long_break_duration_mins: 15,
            rounds: 4,
            auto_start_breaks: false,
            auto_start_focus: false,
            sound_enabled: true,
            volume: 80,
            theme: "classic".to_string(),
            custom_focus_color: None,
            custom_short_break_color: None,
            custom_long_break_color: None,
        }
    }
}

impl PomodoroSettings {
    pub fn resolve_theme(&self) -> ThemeColors {
        if self.theme.to_lowercase() == "custom" {
            let base = ThemeColors::classic();
            ThemeColors {
                name: "custom".to_string(),
                focus: self
                    .custom_focus_color
                    .clone()
                    .unwrap_or(base.focus),
                short_break: self
                    .custom_short_break_color
                    .clone()
                    .unwrap_or(base.short_break),
                long_break: self
                    .custom_long_break_color
                    .clone()
                    .unwrap_or(base.long_break),
                background: base.background,
                track: base.track,
                text_primary: base.text_primary,
                text_secondary: base.text_secondary,
            }
        } else {
            ThemeColors::resolve(&self.theme)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_values() {
        let s = PomodoroSettings::default();
        assert_eq!(s.focus_duration_mins, 25);
        assert_eq!(s.short_break_duration_mins, 5);
        assert_eq!(s.long_break_duration_mins, 15);
        assert_eq!(s.rounds, 4);
        assert!(!s.auto_start_breaks);
        assert!(!s.auto_start_focus);
        assert!(s.sound_enabled);
        assert_eq!(s.volume, 80);
        assert_eq!(s.theme, "classic");
    }

    #[test]
    fn deserialization_with_missing_fields_uses_defaults() {
        let raw = r#"{"focus_duration_mins": 30}"#;
        let s: PomodoroSettings = serde_json::from_str(raw).expect("valid JSON");
        assert_eq!(s.focus_duration_mins, 30);
        assert_eq!(s.short_break_duration_mins, 5);
        assert_eq!(s.rounds, 4);
    }

    #[test]
    fn custom_theme_resolution() {
        let s = PomodoroSettings {
            theme: "custom".to_string(),
            custom_focus_color: Some("#ff007f".to_string()),
            custom_short_break_color: Some("#00ff7f".to_string()),
            custom_long_break_color: Some("#7f00ff".to_string()),
            ..Default::default()
        };
        let theme = s.resolve_theme();
        assert_eq!(theme.name, "custom");
        assert_eq!(theme.focus, "#ff007f");
        assert_eq!(theme.short_break, "#00ff7f");
        assert_eq!(theme.long_break, "#7f00ff");
    }
}
