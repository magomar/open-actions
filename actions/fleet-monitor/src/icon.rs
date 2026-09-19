//! SVG data-URI icon rendering engine for Fleet Monitor.
//!
//! Implements dynamic visual clues, 4-state glowing auras, quad metrics counts,
//! and offline detection states as specified in `specs/004_fleet_monitor.md`.

use crate::state::{FleetStateCounts, ProjectDisplayMode, ProjectState, ProjectSummaryState};
use std::f64::consts::PI;

/// Convert an SVG string into a `data:image/svg+xml;base64,` URI accepted by OpenAction.
pub fn data_uri(svg: &str) -> String {
    format!("data:image/svg+xml;base64,{}", to_base64(svg.as_bytes()))
}

/// Standard 144x144 dark squircle tile.
fn tile(body: &str, border_color: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144" width="144" height="144">
  <defs>
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b"/>
      <stop offset="50%" stop-color="#0f172a"/>
      <stop offset="100%" stop-color="#020617"/>
    </linearGradient>
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="2" stdDeviation="4" flood-color="{border_color}" flood-opacity="0.45"/>
    </filter>
  </defs>
  <rect x="2" y="2" width="140" height="140" rx="28" fill="url(#bgGrad)" stroke="{border_color}" stroke-width="2.5"/>
  {body}
</svg>"##
    )
}

/// Renders the 8-spoke Fleet ship helm at given coordinates and radius.
fn render_helm(cx: f64, cy: f64, r: f64, accent: &str, is_offline: bool) -> String {
    let opacity = if is_offline { 0.4 } else { 1.0 };
    let rim_width = r * 0.16;
    let spoke_width = r * 0.13;
    let hub_r = r * 0.32;
    let inner_r = r * 0.14;
    let compass_r = r * 0.65;

    let mut spokes = String::new();
    for i in 0..8 {
        let rad = (i as f64 * 45.0) * PI / 180.0;
        let x1 = cx + (r * 0.44) * rad.cos();
        let y1 = cy + (r * 0.44) * rad.sin();
        let x2 = cx + (r * 1.35) * rad.cos();
        let y2 = cy + (r * 1.35) * rad.sin();
        spokes.push_str(&format!(
            r##"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{accent}" stroke-width="{spoke_width:.1}" stroke-linecap="round"/>"##
        ));
    }

    format!(
        r##"<g opacity="{opacity}">
  <!-- Outer Wheel Rim -->
  <circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" stroke="{accent}" stroke-width="{rim_width:.1}" fill="none" filter="url(#glow)"/>
  <!-- Compass Dash Ring -->
  <circle cx="{cx:.1}" cy="{cy:.1}" r="{compass_r:.1}" stroke="{accent}" stroke-width="1.5" stroke-dasharray="3 4" opacity="0.6" fill="none"/>
  <!-- Spokes -->
  {spokes}
  <!-- Hub -->
  <circle cx="{cx:.1}" cy="{cy:.1}" r="{hub_r:.1}" fill="#0f172a" stroke="{accent}" stroke-width="{spoke_width:.1}"/>
  <circle cx="{cx:.1}" cy="{cy:.1}" r="{inner_r:.1}" fill="{accent}"/>
</g>"##
    )
}

/// Renders the button face for the Fleet Global action.
pub fn global_icon(counts: FleetStateCounts, is_running: bool) -> String {
    if !is_running {
        let helm = render_helm(72.0, 52.0, 24.0, "#64748b", true);
        let body = format!(
            r##"{helm}
  <text x="72" y="98" fill="#94a3b8" font-size="13" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">FLEET</text>
  <rect x="24" y="112" width="96" height="20" rx="10" fill="#334155" opacity="0.8"/>
  <text x="72" y="126" fill="#f8fafc" font-size="10" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">OFFLINE · TAP</text>"##
        );
        return data_uri(&tile(&body, "#475569"));
    }

    // Determine primary glow color based on fleet urgency
    let primary_color = if counts.blocked > 0 {
        ProjectState::Blocked.color_hex()
    } else if counts.in_progress > 0 {
        ProjectState::InProgress.color_hex()
    } else if counts.ready > 0 {
        ProjectState::Ready.color_hex()
    } else {
        ProjectState::Clean.color_hex()
    };

    let helm = render_helm(72.0, 72.0, 20.0, primary_color, false);

    let body = format!(
        r##"{helm}
  <!-- Top-Left: Clean -->
  <text x="26" y="37" fill="#34d399" font-size="26" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{clean}</text>
  <!-- Top-Right: Ready -->
  <text x="118" y="37" fill="#38bdf8" font-size="26" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{ready}</text>
  <!-- Bottom-Left: In Progress -->
  <text x="26" y="124" fill="#fbbf24" font-size="26" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{in_prog}</text>
  <!-- Bottom-Right: Blocked -->
  <text x="118" y="124" fill="#f87171" font-size="26" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{blocked}</text>"##,
        clean = counts.clean,
        ready = counts.ready,
        in_prog = counts.in_progress,
        blocked = counts.blocked
    );

    data_uri(&tile(&body, primary_color))
}

/// Attempts to locate a project's custom icon, either from the project model
/// or by probing candidate paths in the project directory.
/// Returns a base64 `data:image/...` URI if found.
pub fn find_project_icon(summary: &ProjectSummaryState) -> Option<String> {
    if let Some(ref custom) = summary.project.icon {
        let trimmed = custom.trim();
        if trimmed.starts_with("data:") {
            return Some(trimmed.to_string());
        }
        if trimmed.starts_with("<svg") {
            return Some(format!(
                "data:image/svg+xml;base64,{}",
                to_base64(trimmed.as_bytes())
            ));
        }
        let custom_path = std::path::Path::new(trimmed);
        let resolved = if custom_path.is_absolute() {
            custom_path.to_path_buf()
        } else {
            std::path::Path::new(&summary.project.path).join(custom_path)
        };
        if let Some(uri) = read_icon_file(&resolved) {
            return Some(uri);
        }
    }

    let base = std::path::Path::new(&summary.project.path);
    let candidates = [
        "assets/icons/icon.svg",
        "assets/icons/icon.png",
        "assets/icon.svg",
        "assets/icon.png",
        "icon.svg",
        "icon.png",
        "logo.svg",
        "logo.png",
        ".keel/icon.svg",
        ".keel/icon.png",
        ".keel/logo.svg",
        "src-tauri/icons/icon.svg",
        "src-tauri/icons/128x128.png",
        "frontend/public/icon.svg",
        "public/icon.svg",
    ];

    for rel in candidates {
        let path = base.join(rel);
        if let Some(uri) = read_icon_file(&path) {
            return Some(uri);
        }
    }

    None
}

fn read_icon_file(path: &std::path::Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let path_str = path.to_string_lossy();
    if path_str.ends_with(".svg") {
        Some(format!("data:image/svg+xml;base64,{}", to_base64(&bytes)))
    } else if path_str.ends_with(".png") {
        Some(format!("data:image/png;base64,{}", to_base64(&bytes)))
    } else {
        None
    }
}

/// Renders the button face for the Fleet Project action.
pub fn project_icon(
    summary: Option<&ProjectSummaryState>,
    is_running: bool,
    mode: ProjectDisplayMode,
) -> String {
    if !is_running {
        let custom_icon = summary.and_then(find_project_icon);
        let center_visual = if let Some(ref uri) = custom_icon {
            format!(r##"<image href="{uri}" x="38" y="32" width="68" height="68" opacity="0.4"/>"##)
        } else {
            render_helm(72.0, 64.0, 26.0, "#64748b", true)
        };
        let name = summary.map(|s| s.project.name.as_str()).unwrap_or("Fleet");
        let body = format!(
            r##"<text x="72" y="23" fill="#94a3b8" font-size="13" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">{name}</text>
  {center_visual}
  <rect x="24" y="112" width="96" height="20" rx="10" fill="#334155" opacity="0.8"/>
  <text x="72" y="126" fill="#f8fafc" font-size="10" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">OFFLINE · TAP</text>"##
        );
        return data_uri(&tile(&body, "#475569"));
    }

    let Some(summary) = summary else {
        let helm = render_helm(72.0, 64.0, 26.0, "#38bdf8", false);
        let body = format!(
            r##"<text x="72" y="23" fill="#94a3b8" font-size="13" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">Empty</text>
  {helm}
  <text x="72" y="126" fill="#94a3b8" font-size="10" font-family="system-ui, sans-serif" text-anchor="middle">No Projects</text>"##
        );
        return data_uri(&tile(&body, "#38bdf8"));
    };

    let state = ProjectState::classify(summary);
    let color = state.color_hex();
    let custom_icon = find_project_icon(summary);

    if mode == ProjectDisplayMode::Issues {
        let center_visual = if let Some(ref uri) = custom_icon {
            format!(r##"<image href="{uri}" x="52" y="52" width="40" height="40"/>"##)
        } else {
            render_helm(72.0, 72.0, 20.0, color, false)
        };

        let fsz = |val: usize| if val > 99 { 20 } else { 26 };

        let body = format!(
            r##"{center_visual}
  <!-- Top-Left: Closed -->
  <text x="26" y="37" fill="#34d399" font-size="{fsz_closed}" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{closed}</text>
  <!-- Top-Right: Open -->
  <text x="118" y="37" fill="#38bdf8" font-size="{fsz_open}" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{open}</text>
  <!-- Bottom-Left: In Progress -->
  <text x="26" y="124" fill="#fbbf24" font-size="{fsz_in_prog}" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{in_prog}</text>
  <!-- Bottom-Right: Blocked -->
  <text x="118" y="124" fill="#f87171" font-size="{fsz_blocked}" font-weight="900" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" text-anchor="middle">{blocked}</text>"##,
            center_visual = center_visual,
            closed = summary.beads.closed,
            open = summary.beads.open,
            in_prog = summary.beads.in_progress,
            blocked = summary.beads.blocked,
            fsz_closed = fsz(summary.beads.closed),
            fsz_open = fsz(summary.beads.open),
            fsz_in_prog = fsz(summary.beads.in_progress),
            fsz_blocked = fsz(summary.beads.blocked),
        );

        return data_uri(&tile(&body, color));
    }

    // Default Status Overview mode
    let center_visual = if let Some(ref uri) = custom_icon {
        format!(r##"<image href="{uri}" x="38" y="32" width="68" height="68"/>"##)
    } else {
        render_helm(72.0, 66.0, 25.0, color, false)
    };

    // Status indicator: subtle pill with iconic symbol in state color (no illegible text)
    let status_glyph = match state {
        ProjectState::Clean => {
            r##"<path d="M66 122 l4 4 l8 -8" stroke="#10b981" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##
        }
        ProjectState::Ready => r##"<circle cx="72" cy="122" r="5" fill="#38bdf8"/>"##,
        ProjectState::InProgress => {
            r##"<path d="M73 115 l-4 7 h5 l-2 7 l6 -8 h-5 z" fill="#f59e0b"/>"##
        }
        ProjectState::Blocked => {
            r##"<rect x="71" y="116" width="2.2" height="7" rx="1" fill="#ef4444"/><circle cx="72.1" cy="126" r="1.3" fill="#ef4444"/>"##
        }
    };

    let proj_name = if summary.project.name.len() > 14 {
        format!("{}…", &summary.project.name[..13])
    } else {
        summary.project.name.clone()
    };

    let body = format!(
        r##"<text x="72" y="23" fill="#f8fafc" font-size="13" font-weight="bold" font-family="system-ui, sans-serif" text-anchor="middle">{name}</text>
  {center_visual}
  <!-- Status pill -->
  <rect x="52" y="112" width="40" height="20" rx="10" fill="#0f172a" stroke="{color}" stroke-width="1.8"/>
  {status_glyph}"##,
        name = proj_name,
        center_visual = center_visual,
        color = color,
        status_glyph = status_glyph
    );

    data_uri(&tile(&body, color))
}

/// Minimal RFC-4648 standard base64 encoder with zero external dependencies.
fn to_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::*;

    fn make_test_project(
        id: &str,
        open: usize,
        in_prog: usize,
        blocked: usize,
    ) -> ProjectSummaryState {
        ProjectSummaryState {
            project: RegisteredProject {
                id: id.to_string(),
                name: id.to_string(),
                path: format!("/path/{}", id),
                registered_at: None,
                last_accessed_at: None,
                tags: vec![],
                icon: None,
            },
            exists: true,
            is_keel: true,
            beads: BeadsSummary {
                total: open + 10,
                open,
                in_progress: in_prog,
                closed: 10,
                ready: open.saturating_sub(blocked),
                blocked,
                primary_state: None,
            },
            health: HealthSummary {
                status: "pass".to_string(),
                pass_count: 4,
                warn_count: 0,
                fail_count: 0,
            },
            git: GitSummary::default(),
        }
    }

    #[test]
    fn test_global_icon_renders_valid_data_uri() {
        let counts = FleetStateCounts {
            clean: 2,
            ready: 1,
            in_progress: 1,
            blocked: 1,
        };
        let uri = global_icon(counts, true);
        assert!(uri.starts_with("data:image/svg+xml;base64,"));

        let offline_uri = global_icon(counts, false);
        assert!(offline_uri.starts_with("data:image/svg+xml;base64,"));
        assert_ne!(uri, offline_uri);
    }

    #[test]
    fn test_project_icon_renders_all_states() {
        let clean = make_test_project("keel", 0, 0, 0);
        let ready = make_test_project("tdrace", 5, 0, 0);
        let in_prog = make_test_project("quant-trade", 10, 1, 0);
        let blocked = make_test_project("fleet", 5, 0, 3);

        let u_clean = project_icon(Some(&clean), true, ProjectDisplayMode::Status);
        let u_ready = project_icon(Some(&ready), true, ProjectDisplayMode::Status);
        let u_in_prog = project_icon(Some(&in_prog), true, ProjectDisplayMode::Status);
        let u_blocked = project_icon(Some(&blocked), true, ProjectDisplayMode::Status);

        assert!(u_clean.starts_with("data:image/svg+xml;base64,"));
        assert!(u_ready.starts_with("data:image/svg+xml;base64,"));
        assert!(u_in_prog.starts_with("data:image/svg+xml;base64,"));
        assert!(u_blocked.starts_with("data:image/svg+xml;base64,"));

        // All 4 render different SVG payloads
        assert_ne!(u_clean, u_ready);
        assert_ne!(u_ready, u_in_prog);
        assert_ne!(u_in_prog, u_blocked);
    }

    #[test]
    fn test_project_icon_renders_issues_mode() {
        let mut proj = make_test_project("tdrace", 12, 3, 2);
        proj.beads.closed = 25;

        let icon_uri = project_icon(Some(&proj), true, ProjectDisplayMode::Issues);
        assert!(icon_uri.starts_with("data:image/svg+xml;base64,"));

        let raw = String::from_utf8(to_base64_decode(
            icon_uri.trim_start_matches("data:image/svg+xml;base64,"),
        ))
        .unwrap();

        // Check 4-corner metric numbers are present
        assert!(raw.contains(">25<"), "Top-left closed count missing");
        assert!(raw.contains(">12<"), "Top-right open count missing");
        assert!(raw.contains(">3<"), "Bottom-left in-progress count missing");
        assert!(raw.contains(">2<"), "Bottom-right blocked count missing");
    }

    #[test]
    fn test_custom_icon_renders_image_tag() {
        let mut proj = make_test_project("custom-proj", 0, 0, 0);
        proj.project.icon = Some("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=".to_string());

        let icon_uri = project_icon(Some(&proj), true, ProjectDisplayMode::Status);
        assert!(icon_uri.starts_with("data:image/svg+xml;base64,"));
        // The base64-encoded SVG contains `<image href="data:image/svg+xml;base64,...`
        let raw = String::from_utf8(to_base64_decode(
            icon_uri.trim_start_matches("data:image/svg+xml;base64,"),
        ))
        .unwrap();
        assert!(raw.contains("<image href=\"data:image/svg+xml;base64,"));

        // Also in issues mode
        let issues_uri = project_icon(Some(&proj), true, ProjectDisplayMode::Issues);
        let raw_issues = String::from_utf8(to_base64_decode(
            issues_uri.trim_start_matches("data:image/svg+xml;base64,"),
        ))
        .unwrap();
        assert!(raw_issues.contains("<image href=\"data:image/svg+xml;base64,"));
    }

    fn to_base64_decode(input: &str) -> Vec<u8> {
        // Minimal decoder for tests
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut map = [255u8; 256];
        for (i, &b) in T.iter().enumerate() {
            map[b as usize] = i as u8;
        }
        let clean: Vec<u8> = input
            .bytes()
            .filter(|&b| b != b'=' && b != b'\n' && b != b'\r')
            .collect();
        let mut out = Vec::new();
        for chunk in clean.chunks(4) {
            let b0 = map[chunk[0] as usize];
            let b1 = map
                .get(1)
                .map(|&_idx| map[chunk.get(1).copied().unwrap_or(0) as usize])
                .unwrap_or(0);
            let b2 = chunk.get(2).map(|&b| map[b as usize]).unwrap_or(0);
            let b3 = chunk.get(3).map(|&b| map[b as usize]).unwrap_or(0);

            out.push((b0 << 2) | (b1 >> 4));
            if chunk.len() > 2 {
                out.push((b1 << 4) | (b2 >> 2));
            }
            if chunk.len() > 3 {
                out.push((b2 << 6) | b3);
            }
        }
        out
    }

    #[test]
    fn test_base64_roundtrip() {
        assert_eq!(to_base64(b""), "");
        assert_eq!(to_base64(b"f"), "Zg==");
        assert_eq!(to_base64(b"fo"), "Zm8=");
        assert_eq!(to_base64(b"foo"), "Zm9v");
    }
}
