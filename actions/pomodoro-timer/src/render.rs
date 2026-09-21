// specs/006_pomodoro_timer.md
use crate::state::{Phase, PomodoroState, TimerStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeColors {
    pub name: String,
    pub focus: String,
    pub short_break: String,
    pub long_break: String,
    pub background: String,
    pub track: String,
    pub text_primary: String,
    pub text_secondary: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::classic()
    }
}

impl ThemeColors {
    pub fn classic() -> Self {
        Self {
            name: "classic".to_string(),
            focus: "#ef5350".to_string(),
            short_break: "#4ade80".to_string(),
            long_break: "#22d3ee".to_string(),
            background: "#1e222b".to_string(),
            track: "#2e3440".to_string(),
            text_primary: "#f8fafc".to_string(),
            text_secondary: "#94a3b8".to_string(),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox".to_string(),
            focus: "#fe8019".to_string(),
            short_break: "#b8bb26".to_string(),
            long_break: "#8ec07c".to_string(),
            background: "#282828".to_string(),
            track: "#3c3836".to_string(),
            text_primary: "#ebdbb2".to_string(),
            text_secondary: "#a89984".to_string(),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "nord".to_string(),
            focus: "#bf616a".to_string(),
            short_break: "#a3be8c".to_string(),
            long_break: "#88c0d0".to_string(),
            background: "#2e3440".to_string(),
            track: "#3b4252".to_string(),
            text_primary: "#eceff4".to_string(),
            text_secondary: "#d8dee9".to_string(),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            name: "catppuccin".to_string(),
            focus: "#f38ba8".to_string(),
            short_break: "#a6e3a1".to_string(),
            long_break: "#89dceb".to_string(),
            background: "#1e1e2e".to_string(),
            track: "#313244".to_string(),
            text_primary: "#cdd6f4".to_string(),
            text_secondary: "#a6adc8".to_string(),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "tokyo_night".to_string(),
            focus: "#f7768e".to_string(),
            short_break: "#9ece6a".to_string(),
            long_break: "#7dcfff".to_string(),
            background: "#1a1b26".to_string(),
            track: "#24283b".to_string(),
            text_primary: "#c0caf5".to_string(),
            text_secondary: "#a9b1d6".to_string(),
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "monokai".to_string(),
            focus: "#ff6188".to_string(),
            short_break: "#a9dc76".to_string(),
            long_break: "#78dce8".to_string(),
            background: "#2d2a2e".to_string(),
            track: "#403e41".to_string(),
            text_primary: "#fcfcfa".to_string(),
            text_secondary: "#939293".to_string(),
        }
    }

    pub fn resolve(theme_name: &str) -> Self {
        match theme_name.to_lowercase().as_str() {
            "gruvbox" => Self::gruvbox(),
            "nord" => Self::nord(),
            "catppuccin" => Self::catppuccin(),
            "tokyo_night" | "tokyonight" => Self::tokyo_night(),
            "monokai" => Self::monokai(),
            _ => Self::classic(),
        }
    }

    pub fn active_phase_color(&self, phase: Phase) -> &str {
        match phase {
            Phase::Focus => &self.focus,
            Phase::ShortBreak => &self.short_break,
            Phase::LongBreak => &self.long_break,
        }
    }
}

pub fn render_timer_tile(state: &PomodoroState, theme: &ThemeColors) -> String {
    let circumference = 314.159; // 2 * PI * 50
    let fraction = state.progress_fraction().clamp(0.0, 1.0);
    let offset = circumference * (1.0 - fraction);

    let phase_color = theme.active_phase_color(state.phase);
    let time_str = state.format_time();
    let round_str = state.format_round();
    let phase_label = state.phase.label();

    let status_glyph = match state.status {
        TimerStatus::Running => r##"<circle cx="120" cy="120" r="4" fill="#22c55e"/>"##,
        TimerStatus::Paused => {
            r##"<text x="120" y="125" fill="#f59e0b" font-family="system-ui,sans-serif" font-size="12" font-weight="800" text-anchor="middle">❚❚</text>"##
        }
        TimerStatus::Idle => {
            r##"<text x="120" y="125" fill="#94a3b8" font-family="system-ui,sans-serif" font-size="12" font-weight="800" text-anchor="middle">▶</text>"##
        }
    };

    let font_size = if time_str.len() > 5 { 22 } else { 28 };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="20" fill="{bg}"/><circle cx="72" cy="72" r="50" fill="none" stroke="{track}" stroke-width="8"/><circle cx="72" cy="72" r="50" fill="none" stroke="{color}" stroke-width="8" stroke-dasharray="314.16" stroke-dashoffset="{offset:.2}" stroke-linecap="round" transform="rotate(-90 72 72)"/><text x="72" y="74" fill="{text_p}" font-family="system-ui,sans-serif" font-size="{font_size}" font-weight="700" text-anchor="middle">{time}</text><text x="72" y="93" fill="{color}" font-family="system-ui,sans-serif" font-size="11" font-weight="700" letter-spacing="1" text-anchor="middle">{phase}</text><text x="24" y="125" fill="{text_s}" font-family="system-ui,sans-serif" font-size="12" font-weight="700">{round}</text>{glyph}</svg>"##,
        bg = theme.background,
        track = theme.track,
        color = phase_color,
        offset = offset,
        text_p = theme.text_primary,
        font_size = font_size,
        time = time_str,
        phase = phase_label,
        text_s = theme.text_secondary,
        round = round_str,
        glyph = status_glyph,
    );

    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

pub fn render_skip_tile(theme: &ThemeColors) -> String {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="20" fill="{bg}"/><path d="M48 44 L88 72 L48 100 Z M92 44 L102 44 L102 100 L92 100 Z" fill="{color}"/><text x="72" y="122" fill="{text_s}" font-family="system-ui,sans-serif" font-size="13" font-weight="700" letter-spacing="1" text-anchor="middle">SKIP</text></svg>"##,
        bg = theme.background,
        color = theme.short_break,
        text_s = theme.text_secondary,
    );
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

pub fn render_reset_tile(theme: &ThemeColors) -> String {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="20" fill="{bg}"/><path d="M72 40 A32 32 0 1 0 104 72 L112 72 A40 40 0 1 1 72 32 L72 24 L88 36 L72 48 Z" fill="{color}"/><text x="72" y="122" fill="{text_s}" font-family="system-ui,sans-serif" font-size="13" font-weight="700" letter-spacing="1" text-anchor="middle">RESET</text></svg>"##,
        bg = theme.background,
        color = theme.long_break,
        text_s = theme.text_secondary,
    );
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

pub fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        output.push(TABLE[(bytes[0] >> 2) as usize] as char);
        output.push(TABLE[(((bytes[0] & 0b11) << 4) | (bytes[1] >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((bytes[1] & 0b1111) << 2) | (bytes[2] >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(bytes[2] & 0b11_1111) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_timer_tile_generates_valid_data_uri() {
        let state = PomodoroState::new(25, 4);
        let theme = ThemeColors::classic();
        let uri = render_timer_tile(&state, &theme);
        assert!(uri.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn theme_resolution_works_for_all_presets() {
        assert_eq!(ThemeColors::resolve("classic").focus, "#ef5350");
        assert_eq!(ThemeColors::resolve("gruvbox").focus, "#fe8019");
        assert_eq!(ThemeColors::resolve("nord").focus, "#bf616a");
        assert_eq!(ThemeColors::resolve("catppuccin").focus, "#f38ba8");
        assert_eq!(ThemeColors::resolve("tokyo_night").focus, "#f7768e");
        assert_eq!(ThemeColors::resolve("monokai").focus, "#ff6188");
        assert_eq!(ThemeColors::resolve("unknown").focus, "#ef5350");
    }

    #[test]
    fn companion_tiles_generate_valid_data_uris() {
        let theme = ThemeColors::classic();
        let skip_uri = render_skip_tile(&theme);
        let reset_uri = render_reset_tile(&theme);
        assert!(skip_uri.starts_with("data:image/svg+xml;base64,"));
        assert!(reset_uri.starts_with("data:image/svg+xml;base64,"));
    }
}
