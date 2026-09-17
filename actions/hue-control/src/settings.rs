// specs/003_hue_control.md
//! Bridge credentials, per-action settings, target encoding, and unit conversions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Credentials for one paired Hue Bridge, shared by every action instance.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BridgeConfig {
    pub ip: String,
    pub username: String,
}

/// Plugin-wide settings: one entry per paired bridge, keyed by bridge id.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GlobalSettings {
    #[serde(default)]
    pub bridges: HashMap<String, BridgeConfig>,
}

impl GlobalSettings {
    /// Resolve the bridge for an instance: the named one, or the sole configured one.
    pub fn pick(&self, bridge_id: &str) -> Option<&BridgeConfig> {
        if let Some(config) = self.bridges.get(bridge_id) {
            return Some(config);
        }
        if bridge_id.is_empty() && self.bridges.len() == 1 {
            return self.bridges.values().next();
        }
        None
    }
}

/// Default cycle palette, matching the Elgato plugin's initial triad.
pub const DEFAULT_COLORS: [&str; 3] = ["#ff0000", "#00ff00", "#0000ff"];

/// Default temperature cycle steps in warmth (1..100): ~2900K, ~4300K, ~5600K.
pub const DEFAULT_TEMPERATURES: [u16; 3] = [20, 50, 80];

/// Default brightness cycle steps in percent (1..100).
pub const DEFAULT_BRIGHTNESSES: [u8; 4] = [25, 50, 75, 100];

/// Per-instance settings; all actions share one shape, as Elgato's plugin does.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub bridge: String,
    pub target: String,
    pub mode: String,
    pub color: String,
    pub colors: Vec<String>,
    pub brightness: u8,
    pub brightnesses: Vec<u8>,
    pub scale_ticks: u8,
    pub temperature: u16,
    pub temperatures: Vec<u16>,
    pub brightness_rel: i16,
    pub scene: String,
    pub scenes: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bridge: String::new(),
            target: String::new(),
            mode: "fixed".to_owned(),
            color: "#ffcc66".to_owned(),
            colors: DEFAULT_COLORS
                .iter()
                .map(|color| (*color).to_owned())
                .collect(),
            brightness: 100,
            brightnesses: DEFAULT_BRIGHTNESSES.to_vec(),
            scale_ticks: 1,
            temperature: 50,
            temperatures: DEFAULT_TEMPERATURES.to_vec(),
            brightness_rel: 10,
            scene: String::new(),
            scenes: Vec::new(),
        }
    }
}

impl Settings {
    /// Whether the action instance is operating in cycling mode.
    pub fn is_cycle(&self) -> bool {
        self.mode.eq_ignore_ascii_case("cycle")
    }

    /// The configured cycle palette, ignoring invalid entries, with a default triad fallback.
    pub fn effective_colors(&self) -> Vec<String> {
        let colors: Vec<String> = self
            .colors
            .iter()
            .filter(|color| hex_to_xy(color).is_some())
            .cloned()
            .collect();
        if colors.is_empty() {
            DEFAULT_COLORS
                .iter()
                .map(|color| (*color).to_owned())
                .collect()
        } else {
            colors
        }
    }

    /// The configured temperature steps (warmth 1..100), with defaults fallback.
    pub fn effective_temperatures(&self) -> Vec<u16> {
        let temps: Vec<u16> = self.temperatures.iter().map(|&t| t.clamp(1, 100)).collect();
        if temps.is_empty() {
            DEFAULT_TEMPERATURES.to_vec()
        } else {
            temps
        }
    }

    /// The configured brightness steps (1..100%), with defaults fallback.
    pub fn effective_brightnesses(&self) -> Vec<u8> {
        let bris: Vec<u8> = self.brightnesses.iter().map(|&b| b.clamp(1, 100)).collect();
        if bris.is_empty() {
            DEFAULT_BRIGHTNESSES.to_vec()
        } else {
            bris
        }
    }

    /// The configured scene sequence for cycling, filtering out empty IDs.
    pub fn effective_scenes(&self) -> Vec<String> {
        let filtered: Vec<String> = self
            .scenes
            .iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if filtered.is_empty() && !self.scene.trim().is_empty() {
            vec![self.scene.trim().to_owned()]
        } else {
            filtered
        }
    }

    /// Dial sensitivity: at least one brightness step per detent.
    pub fn tick_scale(&self) -> i16 {
        i16::from(self.scale_ticks.max(1))
    }
}

/// A control target: a group (`g-<id>`) or a single light (`l-<id>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Group(String),
    Light(String),
}

impl Target {
    pub fn decode(encoded: &str) -> Option<Self> {
        let (prefix, id) = encoded.split_once('-')?;
        if id.is_empty() {
            return None;
        }
        match prefix {
            "g" => Some(Self::Group(id.to_owned())),
            "l" => Some(Self::Light(id.to_owned())),
            _ => None,
        }
    }

    /// Path for state changes, relative to `/api/<username>/`.
    pub fn action_path(&self) -> String {
        match self {
            Self::Group(id) => format!("groups/{id}/action"),
            Self::Light(id) => format!("lights/{id}/state"),
        }
    }

    /// Path for reading current state, relative to `/api/<username>/`.
    pub fn resource_path(&self) -> String {
        match self {
            Self::Group(id) => format!("groups/{id}"),
            Self::Light(id) => format!("lights/{id}"),
        }
    }

    /// The key holding the state object: groups report `action`, lights report `state`.
    pub fn state_key(&self) -> &'static str {
        match self {
            Self::Group(_) => "action",
            Self::Light(_) => "state",
        }
    }
}

/// Convert a 0–100 percentage to the bridge's 1–254 brightness scale.
pub fn percent_to_bri(percent: u8) -> u8 {
    let scaled = (u32::from(percent.min(100)) * 254 + 50) / 100;
    scaled.clamp(1, 254) as u8
}

/// Convert a `#rrggbb` color to the bridge's CIE xy chromaticity point.
pub fn hex_to_xy(hex: &str) -> Option<[f32; 2]> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let channel = |offset: usize| {
        let raw = u8::from_str_radix(&digits[offset..offset + 2], 16).unwrap_or(0) as f32 / 255.0;
        if raw > 0.040_45 {
            ((raw + 0.055) / 1.055).powf(2.4)
        } else {
            raw / 12.92
        }
    };

    let (r, g, b) = (channel(0), channel(2), channel(4));
    let x = r * 0.664_511 + g * 0.154_324 + b * 0.162_028;
    let y = r * 0.283_881 + g * 0.668_433 + b * 0.047_685;
    let z = r * 0.000_088 + g * 0.072_310 + b * 0.986_039;
    let sum = x + y + z;
    if sum <= 0.0 {
        return Some([0.0, 0.0]);
    }
    Some([x / sum, y / sum])
}

/// Normalize a user-entered color to `#rrggbb`, falling back to a warm default.
pub fn normalize_hex(hex: &str) -> String {
    let digits: String = hex
        .trim()
        .trim_start_matches('#')
        .chars()
        .filter(char::is_ascii_hexdigit)
        .take(6)
        .collect::<String>()
        .to_lowercase();
    if digits.len() == 6 {
        format!("#{digits}")
    } else {
        "#ffcc66".to_owned()
    }
}

/// Map a 1–100 warmth slider to the bridge's 153–500 mired scale.
pub fn warmth_to_ct(warmth: u16) -> u16 {
    (153 + u32::from(warmth.clamp(1, 100) - 1) * 347 / 99) as u16
}

/// Map a 1–100 warmth slider to a Kelvin readout, for display only.
pub fn warmth_to_kelvin(warmth: u16) -> u16 {
    (2000 + u32::from(warmth.clamp(1, 100) - 1) * 4500 / 99) as u16
}

/// Advance a cycle cursor, returning the index to apply now.
pub fn take_index(cursor: &mut usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let index = *cursor % len;
    *cursor = (index + 1) % len;
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_group_and_light_targets() {
        assert_eq!(Target::decode("g-81"), Some(Target::Group("81".into())));
        assert_eq!(Target::decode("l-8"), Some(Target::Light("8".into())));
    }

    #[test]
    fn rejects_malformed_targets() {
        assert_eq!(Target::decode("81"), None);
        assert_eq!(Target::decode("g-"), None);
        assert_eq!(Target::decode("x-1"), None);
        assert_eq!(Target::decode(""), None);
    }

    #[test]
    fn builds_bridge_paths_per_target_kind() {
        assert_eq!(Target::Group("81".into()).action_path(), "groups/81/action");
        assert_eq!(Target::Light("8".into()).action_path(), "lights/8/state");
        assert_eq!(Target::Group("81".into()).resource_path(), "groups/81");
        assert_eq!(Target::Light("8".into()).resource_path(), "lights/8");
        assert_eq!(Target::Group("81".into()).state_key(), "action");
        assert_eq!(Target::Light("8".into()).state_key(), "state");
    }

    #[test]
    fn converts_percent_to_bridge_brightness() {
        assert_eq!(percent_to_bri(40), 102);
        assert_eq!(percent_to_bri(100), 254);
        assert_eq!(percent_to_bri(0), 1);
        assert_eq!(percent_to_bri(200), 254);
    }

    #[test]
    fn converts_red_to_the_bridge_xy_point() {
        let [x, y] = hex_to_xy("#ff0000").expect("red is valid");
        assert!((x - 0.7006).abs() < 0.001, "x was {x}");
        assert!((y - 0.2993).abs() < 0.001, "y was {y}");
    }

    #[test]
    fn accepts_hex_with_or_without_the_hash() {
        assert_eq!(hex_to_xy("ff0000"), hex_to_xy("#ff0000"));
    }

    #[test]
    fn rejects_malformed_colors() {
        assert_eq!(hex_to_xy("#ff00"), None);
        assert_eq!(hex_to_xy("#gggggg"), None);
        assert_eq!(hex_to_xy(""), None);
    }

    #[test]
    fn cycle_advances_and_wraps() {
        let mut cursor = 0;
        assert_eq!(take_index(&mut cursor, 3), 0);
        assert_eq!(take_index(&mut cursor, 3), 1);
        assert_eq!(take_index(&mut cursor, 3), 2);
        assert_eq!(take_index(&mut cursor, 3), 0);
    }

    #[test]
    fn cycle_handles_an_empty_palette() {
        let mut cursor = 0;
        assert_eq!(take_index(&mut cursor, 0), 0);
    }

    #[test]
    fn defaults_are_usable_without_inspector_configuration() {
        let settings = Settings::default();
        assert_eq!(settings.effective_colors().len(), 3);
        assert_eq!(settings.effective_temperatures().len(), 3);
        assert_eq!(settings.effective_brightnesses().len(), 4);
        assert_eq!(settings.effective_scenes().len(), 0);
        assert_eq!(settings.tick_scale(), 1);
        assert_eq!(settings.brightness, 100);
        assert!(!settings.is_cycle());
    }

    #[test]
    fn cycle_helpers_filter_and_fallback_properly() {
        let mut settings = Settings {
            mode: "cycle".to_owned(),
            temperatures: vec![],
            brightnesses: vec![],
            scenes: vec!["  ".to_owned()],
            scene: "scene123".to_owned(),
            ..Settings::default()
        };
        assert!(settings.is_cycle());
        assert_eq!(
            settings.effective_temperatures(),
            DEFAULT_TEMPERATURES.to_vec()
        );
        assert_eq!(
            settings.effective_brightnesses(),
            DEFAULT_BRIGHTNESSES.to_vec()
        );
        assert_eq!(settings.effective_scenes(), vec!["scene123".to_owned()]);

        settings.temperatures = vec![150, 0, 45];
        assert_eq!(settings.effective_temperatures(), vec![100, 1, 45]);

        settings.brightnesses = vec![200, 0, 50];
        assert_eq!(settings.effective_brightnesses(), vec![100, 1, 50]);

        settings.scenes = vec!["s1".into(), "".into(), "s2".into()];
        assert_eq!(
            settings.effective_scenes(),
            vec!["s1".to_owned(), "s2".to_owned()]
        );
    }

    #[test]
    fn invalid_palette_entries_fall_back_to_the_default_triad() {
        let settings = Settings {
            colors: vec!["not-a-color".to_owned()],
            ..Settings::default()
        };
        assert_eq!(
            settings.effective_colors(),
            DEFAULT_COLORS.map(str::to_owned).to_vec()
        );
    }

    #[test]
    fn picks_the_only_configured_bridge_when_unnamed() {
        let mut global = GlobalSettings::default();
        assert!(global.pick("").is_none());

        global.bridges.insert(
            "001788fffe7a9abf".to_owned(),
            BridgeConfig {
                ip: "192.168.1.74".to_owned(),
                username: "user".to_owned(),
            },
        );
        assert_eq!(
            global.pick("").map(|config| config.ip.as_str()),
            Some("192.168.1.74")
        );
        assert_eq!(
            global
                .pick("001788fffe7a9abf")
                .map(|config| config.ip.as_str()),
            Some("192.168.1.74")
        );
        assert!(global.pick("unknown").is_none());
    }

    #[test]
    fn normalizes_hex_colors() {
        assert_eq!(normalize_hex("#FF0000"), "#ff0000");
        assert_eq!(normalize_hex("00ff00"), "#00ff00");
        assert_eq!(normalize_hex("  #00Ff00  "), "#00ff00");
    }

    #[test]
    fn malformed_colors_fall_back_to_a_default() {
        assert_eq!(normalize_hex("nope"), "#ffcc66");
        assert_eq!(normalize_hex(""), "#ffcc66");
    }

    #[test]
    fn warmth_spans_the_bridge_mired_range() {
        assert_eq!(warmth_to_ct(1), 153);
        assert_eq!(warmth_to_ct(100), 500);
        assert_eq!(warmth_to_kelvin(1), 2000);
        assert_eq!(warmth_to_kelvin(100), 6500);
    }

    #[test]
    fn deserializes_settings_from_a_partial_inspector_payload() {
        let settings: Settings =
            serde_json::from_str(r##"{"target":"g-81","color":"#00ff00"}"##).unwrap();
        assert_eq!(settings.target, "g-81");
        assert_eq!(settings.color, "#00ff00");
        assert_eq!(settings.brightness, 100);
    }
}
