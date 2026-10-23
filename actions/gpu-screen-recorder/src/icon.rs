// specs/007_gpu_screen_recorder.md
use crate::gsr::{CaptureMode, format_elapsed};

pub fn render_tile(action_mode: CaptureMode, recording_elapsed_secs: Option<u64>) -> String {
    let svg = match (action_mode, recording_elapsed_secs) {
        (CaptureMode::Screen, None) => render_screen_idle(),
        (CaptureMode::Screen, Some(secs)) => render_screen_recording(secs),
        (CaptureMode::Window, None) => render_window_idle(),
        (CaptureMode::Window, Some(secs)) => render_window_recording(secs),
    };
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

fn render_screen_idle() -> String {
    concat!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">"##,
        r##"<rect width="144" height="144" rx="20" fill="#18181b"/>"##,
        r##"<rect x="3" y="3" width="138" height="138" rx="17" fill="none" stroke="#27272a" stroke-width="2"/>"##,
        // Monitor frame
        r##"<rect x="26" y="24" width="92" height="58" rx="7" fill="#0f172a" stroke="#38bdf8" stroke-width="3.5"/>"##,
        // Record-ready indicator inside monitor
        r##"<circle cx="72" cy="53" r="10" fill="none" stroke="#38bdf8" stroke-width="2.5" opacity="0.85"/>"##,
        r##"<circle cx="72" cy="53" r="4.5" fill="#38bdf8"/>"##,
        // Monitor stand
        r##"<path d="M64 82 L60 95 L84 95 L80 82 Z" fill="#64748b"/>"##,
        r##"<rect x="50" y="94" width="44" height="4" rx="2" fill="#94a3b8"/>"##,
        // Label
        r##"<text x="72" y="124" text-anchor="middle" fill="#e2e8f0" font-family="system-ui, -apple-system, sans-serif" font-size="16" font-weight="700" letter-spacing="1.5">SCREEN</text>"##,
        r##"</svg>"##
    )
    .to_string()
}

fn render_screen_recording(elapsed_secs: u64) -> String {
    let time_str = format_elapsed(elapsed_secs);
    let font_size = if time_str.len() > 5 { 18 } else { 22 };
    format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">"##,
            r##"<rect width="144" height="144" rx="20" fill="#1f1215"/>"##,
            r##"<rect x="3" y="3" width="138" height="138" rx="17" fill="none" stroke="#ef4444" stroke-width="4"/>"##,
            // Top REC badge
            r##"<circle cx="52" cy="20" r="5.5" fill="#ef4444"/>"##,
            r##"<text x="64" y="25" fill="#f87171" font-family="system-ui, -apple-system, sans-serif" font-size="14" font-weight="800" letter-spacing="1.2">REC</text>"##,
            // Monitor frame
            r##"<rect x="22" y="34" width="100" height="54" rx="7" fill="#2a1215" stroke="#f87171" stroke-width="3.5"/>"##,
            // Elapsed timer inside monitor
            r##"<text x="72" y="69" text-anchor="middle" fill="#ffffff" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="{font_size}" font-weight="800">{time_str}</text>"##,
            // Monitor stand
            r##"<path d="M64 88 L60 99 L84 99 L80 88 Z" fill="#f87171" opacity="0.75"/>"##,
            r##"<rect x="50" y="98" width="44" height="4" rx="2" fill="#fca5a5"/>"##,
            // Label
            r##"<text x="72" y="125" text-anchor="middle" fill="#fca5a5" font-family="system-ui, -apple-system, sans-serif" font-size="15" font-weight="800" letter-spacing="1.5">SCREEN</text>"##,
            r##"</svg>"##
        ),
        font_size = font_size,
        time_str = time_str,
    )
}

fn render_window_idle() -> String {
    concat!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">"##,
        r##"<rect width="144" height="144" rx="20" fill="#18181b"/>"##,
        r##"<rect x="3" y="3" width="138" height="138" rx="17" fill="none" stroke="#27272a" stroke-width="2"/>"##,
        // Application window frame
        r##"<rect x="26" y="26" width="92" height="66" rx="8" fill="#0f172a" stroke="#a78bfa" stroke-width="3.5"/>"##,
        r##"<line x1="26" y1="42" x2="118" y2="42" stroke="#a78bfa" stroke-width="2.5"/>"##,
        // Titlebar traffic-light dots
        r##"<circle cx="37" cy="34" r="2.8" fill="#f87171"/>"##,
        r##"<circle cx="46" cy="34" r="2.8" fill="#fbbf24"/>"##,
        r##"<circle cx="55" cy="34" r="2.8" fill="#4ade80"/>"##,
        // Record-ready indicator inside window body
        r##"<circle cx="72" cy="67" r="10" fill="none" stroke="#a78bfa" stroke-width="2.5" opacity="0.85"/>"##,
        r##"<circle cx="72" cy="67" r="4.5" fill="#a78bfa"/>"##,
        // Label
        r##"<text x="72" y="124" text-anchor="middle" fill="#e2e8f0" font-family="system-ui, -apple-system, sans-serif" font-size="16" font-weight="700" letter-spacing="1.5">WINDOW</text>"##,
        r##"</svg>"##
    )
    .to_string()
}

fn render_window_recording(elapsed_secs: u64) -> String {
    let time_str = format_elapsed(elapsed_secs);
    let font_size = if time_str.len() > 5 { 18 } else { 22 };
    format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">"##,
            r##"<rect width="144" height="144" rx="20" fill="#1f1215"/>"##,
            r##"<rect x="3" y="3" width="138" height="138" rx="17" fill="none" stroke="#ef4444" stroke-width="4"/>"##,
            // Top REC badge
            r##"<circle cx="52" cy="20" r="5.5" fill="#ef4444"/>"##,
            r##"<text x="64" y="25" fill="#f87171" font-family="system-ui, -apple-system, sans-serif" font-size="14" font-weight="800" letter-spacing="1.2">REC</text>"##,
            // Application window frame
            r##"<rect x="22" y="34" width="100" height="64" rx="8" fill="#2a1215" stroke="#f87171" stroke-width="3.5"/>"##,
            r##"<line x1="22" y1="48" x2="122" y2="48" stroke="#f87171" stroke-width="2.5"/>"##,
            r##"<circle cx="33" cy="41" r="2.5" fill="#f87171"/>"##,
            r##"<circle cx="41" cy="41" r="2.5" fill="#fbbf24"/>"##,
            r##"<circle cx="49" cy="41" r="2.5" fill="#4ade80"/>"##,
            // Elapsed timer inside window body
            r##"<text x="72" y="81" text-anchor="middle" fill="#ffffff" font-family="ui-monospace, SFMono-Regular, Menlo, monospace" font-size="{font_size}" font-weight="800">{time_str}</text>"##,
            // Label
            r##"<text x="72" y="125" text-anchor="middle" fill="#fca5a5" font-family="system-ui, -apple-system, sans-serif" font-size="15" font-weight="800" letter-spacing="1.5">WINDOW</text>"##,
            r##"</svg>"##
        ),
        font_size = font_size,
        time_str = time_str,
    )
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
    fn renders_all_states_as_svg_data_uris() {
        let screen_idle = render_tile(CaptureMode::Screen, None);
        let screen_rec = render_tile(CaptureMode::Screen, Some(84));
        let window_idle = render_tile(CaptureMode::Window, None);
        let window_rec = render_tile(CaptureMode::Window, Some(42));

        for uri in [&screen_idle, &screen_rec, &window_idle, &window_rec] {
            assert!(uri.starts_with("data:image/svg+xml;base64,"));
        }

        assert_ne!(screen_idle, screen_rec);
        assert_ne!(window_idle, window_rec);
        assert_ne!(screen_idle, window_idle);
        assert_ne!(screen_rec, window_rec);
    }

    #[test]
    fn recording_svgs_include_formatted_time_and_rec_badge() {
        let svg_screen = render_screen_recording(125);
        assert!(svg_screen.contains("02:05"));
        assert!(svg_screen.contains("REC"));
        assert!(svg_screen.contains("SCREEN"));

        let svg_window = render_window_recording(3661);
        assert!(svg_window.contains("1:01:01"));
        assert!(svg_window.contains("REC"));
        assert!(svg_window.contains("WINDOW"));
    }
}
