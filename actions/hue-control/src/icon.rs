// specs/003_hue_control.md
//! SVG data-URI rendering for each action's button face (Refined Colored Collection).

use std::f64::consts::PI;

/// Convert to a `data:image/svg+xml;base64,` URI the device can render.
fn data_uri(svg: &str) -> String {
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

/// The shared dark tile every action renders on.
fn tile(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="18" fill="#18181b"/>{body}</svg>"##
    )
}

/// Helper: generate 8 radial rays for the sun glyph.
fn sun_rays(cx: f64, cy: f64, r1: f64, r2: f64, stroke: &str, width: f64) -> String {
    let mut s = String::new();
    for i in 0..8 {
        let rad = (i as f64 * 45.0) * PI / 180.0;
        let x1 = cx + r1 * rad.cos();
        let y1 = cy + r1 * rad.sin();
        let x2 = cx + r2 * rad.cos();
        let y2 = cy + r2 * rad.sin();
        s.push_str(&format!(
            r##"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" stroke-linecap="round"/>"##
        ));
    }
    s
}

/// A bulb glyph, filled with `accent`.
fn bulb(accent: &str) -> String {
    format!(
        r##"<g transform="translate(72 60)"><path d="M0 -36 c-15 0 -26 11 -26 26 c0 10 6 17 11 23 c3 3 4 7 5 11 h20 c1 -4 2 -8 5 -11 c5 -6 11 -13 11 -23 c0 -15 -11 -26 -26 -26 z" fill="{accent}"/><rect x="-10" y="28" width="20" height="6" rx="2.5" fill="#9a9a9a"/><rect x="-8" y="36" width="16" height="5" rx="2" fill="#6b6b6b"/></g>"##
    )
}

/// Interpolate a warm→cool hex color for a Kelvin value (2000K–6500K) or mirek value (153–500).
pub fn temperature_color(k: u16) -> &'static str {
    let kelvin = if (153..=500).contains(&k) {
        (1_000_000 / u32::from(k)) as u16
    } else {
        k
    };
    if kelvin <= 2500 {
        "#ea580c"
    } else if kelvin <= 3200 {
        "#f97316"
    } else if kelvin <= 4000 {
        "#fbbf24"
    } else if kelvin <= 5000 {
        "#fef08a"
    } else if kelvin <= 6000 {
        "#e0f2fe"
    } else {
        "#38bdf8"
    }
}

/// Power state: a luminous glowing bulb when on, a dimmed graphite bulb when off.
pub fn power(on: bool) -> String {
    let body = if on {
        r##"<defs><radialGradient id="bulbAuraRef" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#ffd479" stop-opacity="0.4"/><stop offset="100%" stop-color="#ffd479" stop-opacity="0"/></radialGradient></defs><g transform="translate(0, -4)"><circle cx="72" cy="52" r="40" fill="url(#bulbAuraRef)"/><g transform="translate(72, 60)"><path d="M0 -36 c-15 0 -26 11 -26 26 c0 10 6 17 11 23 c3 3 4 7 5 11 h20 c1 -4 2 -8 5 -11 c5 -6 11 -13 11 -23 c0 -15 -11 -26 -26 -26 z" fill="#ffd479"/><rect x="-10" y="28" width="20" height="6" rx="2.5" fill="#9a9a9a"/><rect x="-8" y="36" width="16" height="5" rx="2" fill="#6b6b6b"/></g><circle cx="106" cy="30" r="7" fill="#34d399"/><text x="72" y="122" fill="#34d399" font-family="sans-serif" font-size="20" font-weight="700" text-anchor="middle">ON</text></g>"##
    } else {
        r##"<g transform="translate(0, -4)"><g transform="translate(72, 60)"><path d="M0 -36 c-15 0 -26 11 -26 26 c0 10 6 17 11 23 c3 3 4 7 5 11 h20 c1 -4 2 -8 5 -11 c5 -6 11 -13 11 -23 c0 -15 -11 -26 -26 -26 z" fill="#3f3f46"/><rect x="-10" y="28" width="20" height="6" rx="2.5" fill="#52525b"/><rect x="-8" y="36" width="16" height="5" rx="2" fill="#3f3f46"/></g><circle cx="106" cy="30" r="7" fill="#52525b"/><text x="72" y="122" fill="#71717a" font-family="sans-serif" font-size="20" font-weight="700" text-anchor="middle">OFF</text></g>"##
    };
    data_uri(&tile(body))
}

/// Brightness: Sun glyph with an integrated circular halo dial gauge and numerical percentage readout.
pub fn brightness(percent: u8) -> String {
    let percent = percent.min(100);
    let total_arc = 158.8_f64;
    let fill_arc = (f64::from(percent) / 100.0) * total_arc;
    let stroke_col = if percent > 70 {
        "#fde047"
    } else if percent > 35 {
        "#fbbf24"
    } else {
        "#f59e0b"
    };
    let glow_opacity = 0.2 + (f64::from(percent) / 100.0) * 0.4;
    let rays = sun_rays(72.0, 52.0, 18.0, 25.0, stroke_col, 2.5);

    let body = format!(
        r##"<defs><linearGradient id="briHaloGrad" x1="0%" y1="100%" x2="100%" y2="0%"><stop offset="0%" stop-color="#f59e0b"/><stop offset="100%" stop-color="#fde047"/></linearGradient><radialGradient id="sunAuraRefined" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#fde047" stop-opacity="{glow_opacity:.2}"/><stop offset="100%" stop-color="#f59e0b" stop-opacity="0"/></radialGradient></defs><circle cx="72" cy="52" r="35" fill="none" stroke="#27272a" stroke-width="4.5" stroke-dasharray="158.8 220" stroke-linecap="round" transform="rotate(140, 72, 52)"/><circle cx="72" cy="52" r="35" fill="none" stroke="url(#briHaloGrad)" stroke-width="4.5" stroke-dasharray="{fill_arc:.1} 220" stroke-linecap="round" transform="rotate(140, 72, 52)"/><circle cx="72" cy="52" r="26" fill="url(#sunAuraRefined)"/><circle cx="72" cy="52" r="14" fill="{stroke_col}"/>{rays}<text x="72" y="118" fill="#ffffff" font-family="sans-serif" font-size="34" font-weight="700" text-anchor="middle">{percent}%</text>"##
    );
    data_uri(&tile(&body))
}

/// Brightness cycle step: renders the refined sun with integrated halo gauge and percentage readout.
pub fn brightness_cycle(percent: u8, _index: usize, _total: usize) -> String {
    brightness(percent)
}

/// Color temperature: Thermometer whose mercury column and fill color dynamically reflect Kelvin warmth.
pub fn temperature(kelvin: u16) -> String {
    let kelvin_val = if (153..=500).contains(&kelvin) {
        (1_000_000 / u32::from(kelvin)) as u16
    } else {
        kelvin
    };
    let clamped_k = kelvin_val.clamp(2000, 6500);
    let ratio = f64::from(clamped_k - 2000) / 4500.0;
    let col = temperature_color(kelvin_val);
    let mercury_height = 12.0 + (ratio * 30.0);
    let mercury_y = 56.0 - (ratio * 30.0);

    let body = format!(
        r##"<defs><radialGradient id="bulbGlowRef" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="{col}" stop-opacity="0.45"/><stop offset="100%" stop-color="{col}" stop-opacity="0"/></radialGradient></defs><g transform="translate(0, -2)"><g stroke="#a1a1aa"><g transform="translate(72, 46) scale(0.82)"><rect x="-8" y="-42" width="16" height="42" rx="8" fill="none" stroke-width="3"/><circle cx="0" cy="12" r="16" fill="none" stroke-width="3"/><rect x="-5" y="-6" width="10" height="8" fill="#a1a1aa"/><line x1="12" y1="-32" x2="18" y2="-32" stroke-width="2.5" stroke-linecap="round"/><line x1="12" y1="-20" x2="20" y2="-20" stroke-width="2.5" stroke-linecap="round"/><line x1="12" y1="-8" x2="18" y2="-8" stroke-width="2.5" stroke-linecap="round"/></g></g><circle cx="72" cy="56" r="18" fill="url(#bulbGlowRef)"/><circle cx="72" cy="56" r="8" fill="{col}"/><rect x="69" y="{mercury_y:.1}" width="6" height="{mercury_height:.1}" rx="3" fill="{col}"/><text x="72" y="118" fill="{col}" font-family="sans-serif" font-size="30" font-weight="700" text-anchor="middle">{kelvin_val}K</text></g>"##
    );
    data_uri(&tile(&body))
}

/// Color temperature cycle step: renders the refined thermometer with Kelvin readout.
pub fn temperature_cycle(kelvin: u16, _index: usize, _total: usize) -> String {
    temperature(kelvin)
}

/// Color: Artist palette with a luminous active color gem and hex readout.
pub fn color(hex: &str) -> String {
    let hex_clean = if hex.starts_with('#') {
        hex.to_string()
    } else {
        format!("#{hex}")
    };
    let hex_upper = hex_clean.to_uppercase();

    let body = format!(
        r##"<defs><filter id="gemGlow" x="-30%" y="-30%" width="160%" height="160%"><feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="{hex_clean}" flood-opacity="0.6"/></filter></defs><g transform="translate(0, -2)"><g transform="translate(72, 50) scale(0.98)"><path d="M-36 -4 C-44 -26 -28 -46 -4 -46 C20 -46 38 -30 38 -10 C38 6 26 20 12 20 C6 20 2 16 0 10 C-2 4 -8 0 -14 0 C-22 0 -28 8 -32 8 C-35 8 -36 4 -36 -4 Z" fill="#27272a" stroke="#e4e4e7" stroke-width="3.5" stroke-linejoin="round"/><circle cx="16" cy="0" r="5" fill="#e4e4e7"/></g><circle cx="58" cy="30" r="4.5" fill="#ef4444"/><circle cx="72" cy="25" r="4.5" fill="#eab308"/><circle cx="86" cy="32" r="4.5" fill="#3b82f6"/><circle cx="58" cy="46" r="8" fill="{hex_clean}" stroke="#ffffff" stroke-width="2" filter="url(#gemGlow)"/><text x="72" y="118" fill="{hex_clean}" font-family="sans-serif" font-size="24" font-weight="700" text-anchor="middle">{hex_upper}</text></g>"##
    );
    data_uri(&tile(&body))
}

/// Color cycle step: renders the refined artist palette with active color gem and hex readout.
pub fn cycle(hex: &str, _index: usize, _total: usize) -> String {
    color(hex)
}

/// A scene button with glowing ambient constellation.
pub fn scene() -> String {
    let body = r##"<defs><radialGradient id="sceneGlowRef" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#c084fc" stop-opacity="0.5"/><stop offset="100%" stop-color="#a855f7" stop-opacity="0"/></radialGradient></defs><circle cx="72" cy="50" r="34" fill="url(#sceneGlowRef)"/><g transform="translate(0, -4)"><path d="M72 26 Q72 44 54 44 Q72 44 72 62 Q72 44 90 44 Q72 44 72 26 Z" fill="#e879f9" stroke="#c084fc" stroke-width="2"/><path d="M96 52 Q96 62 86 62 Q96 62 96 72 Q96 62 106 62 Q96 62 96 52 Z" fill="#e879f9" stroke="#c084fc" stroke-width="1.5"/><path d="M46 60 Q46 68 38 68 Q46 68 46 76 Q46 68 54 68 Q46 68 46 60 Z" fill="#e879f9" stroke="#c084fc" stroke-width="1.5"/><text x="72" y="120" fill="#c084fc" font-family="sans-serif" font-size="20" font-weight="700" text-anchor="middle">Scene</text></g>"##;
    data_uri(&tile(body))
}

/// A scene cycle step with glowing ambient constellation.
pub fn scene_cycle(_index: usize, _total: usize) -> String {
    scene()
}

/// An error state with an explanatory label.
pub fn error(text: &str) -> String {
    let body = format!(
        r##"<text x="72" y="70" fill="#ef4444" font-family="sans-serif" font-size="46" font-weight="700" text-anchor="middle">!</text><text x="72" y="106" fill="#ef4444" font-family="sans-serif" font-size="17" font-weight="700" text-anchor="middle">{text}</text>"##
    );
    data_uri(&tile(&body))
}

/// The plugin icon, also used before an action is configured.
pub fn status(text: &str) -> String {
    let body = format!(
        r##"{}<text x="72" y="130" fill="#ffd479" font-family="sans-serif" font-size="16" font-weight="700" text-anchor="middle">{text}</text>"##,
        bulb("#ffd479")
    );
    data_uri(&tile(&body))
}

fn base64(input: &[u8]) -> String {
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
    fn every_icon_renders_as_an_svg_data_image() {
        assert!(power(true).starts_with("data:image/svg+xml;base64,"));
        assert!(power(false).starts_with("data:image/svg+xml;base64,"));
        assert!(brightness(40).starts_with("data:image/svg+xml;base64,"));
        assert!(brightness_cycle(40, 0, 4).starts_with("data:image/svg+xml;base64,"));
        assert!(temperature(2700).starts_with("data:image/svg+xml;base64,"));
        assert!(temperature_cycle(2700, 0, 3).starts_with("data:image/svg+xml;base64,"));
        assert!(color("#ff0000").starts_with("data:image/svg+xml;base64,"));
        assert!(cycle("#ff0000", 0, 3).starts_with("data:image/svg+xml;base64,"));
        assert!(scene().starts_with("data:image/svg+xml;base64,"));
        assert!(scene_cycle(0, 2).starts_with("data:image/svg+xml;base64,"));
        assert!(error("Offline").starts_with("data:image/svg+xml;base64,"));
        assert!(status("Hue").starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn power_states_render_differently() {
        assert_ne!(power(true), power(false));
    }

    #[test]
    fn brightness_clamps_above_one_hundred() {
        assert_eq!(brightness(140), brightness(100));
    }

    #[test]
    fn temperature_colors_follow_the_warm_cool_axis() {
        assert_ne!(temperature_color(153), temperature_color(500));
        assert_ne!(temperature_color(2200), temperature_color(6500));
    }
}
