// specs/003_hue_control.md
//! SVG data-URI rendering for each action's button face.

/// Convert to a `data:image/svg+xml;base64,` URI the device can render.
fn data_uri(svg: &str) -> String {
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

/// A bulb glyph, filled with `accent`.
fn bulb(accent: &str) -> String {
    format!(
        r##"<g transform="translate(72 66)"><path d="M0 -40c-17 0-30 13-30 30 0 12 7 20 13 27 3 4 5 8 6 13h22c1-5 3-9 6-13 6-7 13-15 13-27 0-17-13-30-30-30z" fill="{accent}"/><rect x="-11" y="33" width="22" height="7" rx="3" fill="#9a9a9a"/><rect x="-9" y="43" width="18" height="6" rx="3" fill="#6b6b6b"/></g>"##
    )
}

/// The shared dark tile every action renders on.
fn tile(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="18" fill="#18181b"/>{body}</svg>"##
    )
}

/// Power state: a lit bulb when on, a hollow one when off.
pub fn power(on: bool) -> String {
    let body = if on {
        format!(
            "{}{}",
            bulb("#ffd479"),
            r##"<circle cx="106" cy="38" r="9" fill="#34d399"/>"##
        )
    } else {
        format!(
            "{}{}",
            bulb("#4b4b52"),
            r##"<circle cx="106" cy="38" r="9" fill="#3f3f46"/>"##
        )
    };
    data_uri(&tile(&body))
}

/// Absolute brightness: a percentage readout with a progress bar.
pub fn brightness(percent: u8) -> String {
    let percent = percent.min(100);
    let width = 96 * u16::from(percent) / 100;
    let body = format!(
        r##"<text x="72" y="62" fill="#ffd479" font-family="sans-serif" font-size="20" font-weight="700" text-anchor="middle">Bri</text><text x="72" y="106" fill="#fff" font-family="sans-serif" font-size="48" font-weight="700" text-anchor="middle">{percent}%</text><rect x="24" y="120" width="96" height="8" rx="4" fill="#3f3f46"/><rect x="24" y="120" width="{width}" height="8" rx="4" fill="#ffd479"/>"##
    );
    data_uri(&tile(&body))
}

/// Relative brightness: a signed step readout.
pub fn brightness_relative(steps: i16) -> String {
    let sign = if steps >= 0 { "+" } else { "" };
    let body = format!(
        r##"<text x="72" y="58" fill="#ffd479" font-family="sans-serif" font-size="18" font-weight="700" text-anchor="middle">Steps</text><text x="72" y="112" fill="#fff" font-family="sans-serif" font-size="52" font-weight="700" text-anchor="middle">{sign}{steps}</text>"##
    );
    data_uri(&tile(&body))
}

/// Color temperature: a swatch interpolated between warm and cool.
pub fn temperature(kelvin: u16) -> String {
    let accent = temperature_color(kelvin);
    let body = format!(
        r##"<text x="72" y="58" fill="{accent}" font-family="sans-serif" font-size="18" font-weight="700" text-anchor="middle">Temp</text><text x="72" y="106" fill="#fff" font-family="sans-serif" font-size="40" font-weight="700" text-anchor="middle">{kelvin}K</text><rect x="24" y="120" width="96" height="8" rx="4" fill="url(#g)"/><defs><linearGradient id="g"><stop offset="0%" stop-color="#faa04e"/><stop offset="100%" stop-color="#86c6e8"/></linearGradient></defs>"##
    );
    data_uri(&tile(&body))
}

/// A color swatch, named by its hex value.
pub fn color(hex: &str) -> String {
    let body = format!(
        r##"<rect x="30" y="30" width="84" height="72" rx="10" fill="{hex}" stroke="#3f3f46" stroke-width="2"/><text x="72" y="124" fill="#d8d8d8" font-family="sans-serif" font-size="16" font-weight="700" text-anchor="middle">{hex}</text>"##
    );
    data_uri(&tile(&body))
}

/// A cycle step: the current swatch plus its position in the palette.
pub fn cycle(hex: &str, index: usize, total: usize) -> String {
    let body = format!(
        r##"<rect x="30" y="26" width="84" height="68" rx="10" fill="{hex}" stroke="#3f3f46" stroke-width="2"/><text x="72" y="118" fill="#d8d8d8" font-family="sans-serif" font-size="16" font-weight="700" text-anchor="middle">Cycle {}/{} </text>"##,
        index + 1,
        total,
    );
    data_uri(&tile(&body))
}

/// A scene button.
pub fn scene() -> String {
    let body = r##"<g fill="#c084fc"><circle cx="46" cy="52" r="13"/><circle cx="98" cy="44" r="10"/><circle cx="72" cy="98" r="15"/></g><text x="72" y="132" fill="#c084fc" font-family="sans-serif" font-size="16" font-weight="700" text-anchor="middle">Scene</text>"##;
    data_uri(&tile(body))
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
        r##"{}<text x="72" y="134" fill="#ffd479" font-family="sans-serif" font-size="16" font-weight="700" text-anchor="middle">{text}</text>"##,
        bulb("#ffd479")
    );
    data_uri(&tile(&body))
}

/// Interpolate a warm→cool hex color for a Kelvin value in the Hue 153–500 range.
fn temperature_color(kelvin: u16) -> &'static str {
    if kelvin <= 250 {
        "#86c6e8"
    } else if kelvin >= 430 {
        "#faa04e"
    } else {
        "#d9b779"
    }
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
        assert!(brightness_relative(10).starts_with("data:image/svg+xml;base64,"));
        assert!(temperature(366).starts_with("data:image/svg+xml;base64,"));
        assert!(color("#ff0000").starts_with("data:image/svg+xml;base64,"));
        assert!(cycle("#ff0000", 0, 3).starts_with("data:image/svg+xml;base64,"));
        assert!(scene().starts_with("data:image/svg+xml;base64,"));
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
    fn arc_axis_renders_a_signed_step() {
        assert_ne!(brightness_relative(-10), brightness_relative(10));
    }

    #[test]
    fn temperature_colors_follow_the_warm_cool_axis() {
        assert_ne!(temperature_color(153), temperature_color(500));
    }
}
