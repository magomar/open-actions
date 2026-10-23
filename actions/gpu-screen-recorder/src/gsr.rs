// specs/007_gpu_screen_recorder.md
use std::{
    fs, io,
    path::Path,
    process::{Command, Stdio},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

pub const DEFAULT_BINARY: &str = "gsr-ui-cli";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Screen,
    Window,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveRecording {
    pub pid: u32,
    pub mode: CaptureMode,
    pub elapsed_secs: u64,
}

pub fn effective_path(configured: &str) -> &str {
    let path = configured.trim();
    if path.is_empty() {
        DEFAULT_BINARY
    } else {
        path
    }
}

pub fn toggle_record_command(configured: &str) -> Command {
    let mut command = Command::new(effective_path(configured));
    command.arg("toggle-record");
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command
}

pub fn toggle_record_window_command(configured: &str) -> Command {
    let mut command = Command::new(effective_path(configured));
    command.arg("toggle-record-window");
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command
}

pub fn status_command(configured: &str) -> Command {
    let mut command = Command::new(effective_path(configured));
    command.arg("status");
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command
}

fn run_checked(mut command: Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "gsr-ui-cli exited with status {status}"
        )))
    }
}

pub fn toggle_screen(configured: &str) -> io::Result<()> {
    run_checked(toggle_record_command(configured))
}

pub fn toggle_window(configured: &str) -> io::Result<()> {
    run_checked(toggle_record_window_command(configured))
}

pub fn is_daemon_running(configured: &str) -> bool {
    let Ok(output) = status_command(configured).output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim() == "running"
}

pub fn parse_cmdline_bytes(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|&b| b == 0)
        .filter(|slice| !slice.is_empty())
        .map(|slice| String::from_utf8_lossy(slice).into_owned())
        .collect()
}

fn is_stream_or_screenshot_output(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    let stream_prefixes = [
        "rtmp://", "rtmps://", "http://", "https://", "srt://", "tcp://", "udp://",
    ];
    if stream_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }
    lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
}

fn is_window_target(primary_target: &str, portal_token_path: Option<&str>) -> bool {
    if primary_target == "window"
        || primary_target == "focused"
        || primary_target.starts_with("window:")
    {
        return true;
    }

    if !primary_target.is_empty() && primary_target.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    if let Some(hex) = primary_target
        .strip_prefix("0x")
        .or_else(|| primary_target.strip_prefix("0X"))
        && !hex.is_empty()
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return true;
    }

    if primary_target == "portal"
        && let Some(token) = portal_token_path
        && token.contains("gsr-ui-window-capture-token")
    {
        return true;
    }

    false
}

pub fn classify_cmdline(args: &[String]) -> Option<CaptureMode> {
    let first = args.first()?;
    let exe_name = Path::new(first).file_name()?.to_str()?;
    if exe_name != "gpu-screen-recorder" {
        return None;
    }

    let info_flags = [
        "--info",
        "--help",
        "-h",
        "--version",
        "--list-capture-options",
        "--list-monitors",
        "--list-audio-devices",
        "--list-application-audio",
        "--list-v4l2-devices",
    ];
    if args.iter().any(|arg| info_flags.contains(&arg.as_str())) {
        return None;
    }

    let mut capture_target: Option<&str> = None;
    let mut output_path: Option<&str> = None;
    let mut has_replay_flag = false;
    let mut portal_token_path: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-w" => {
                capture_target = args.get(i + 1).map(String::as_str);
                i += 2;
            }
            "-o" => {
                output_path = args.get(i + 1).map(String::as_str);
                i += 2;
            }
            "-r" => {
                has_replay_flag = true;
                i += 2;
            }
            "-portal-session-token-filepath" => {
                portal_token_path = args.get(i + 1).map(String::as_str);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let target = capture_target?;
    let output = output_path?;

    if has_replay_flag || is_stream_or_screenshot_output(output) {
        return None;
    }

    let primary_source = target
        .split('|')
        .next()
        .unwrap_or(target)
        .split(';')
        .next()
        .unwrap_or(target);

    if is_window_target(primary_source, portal_token_path) {
        Some(CaptureMode::Window)
    } else {
        Some(CaptureMode::Screen)
    }
}

pub fn detect_active_recording_in(proc_root: &Path, now: SystemTime) -> Option<ActiveRecording> {
    let entries = fs::read_dir(proc_root).ok()?;
    let mut best: Option<ActiveRecording> = None;

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        let Ok(pid) = name_str.parse::<u32>() else {
            continue;
        };

        let proc_dir = entry.path();
        let cmdline_bytes = match fs::read(proc_dir.join("cmdline")) {
            Ok(b) if !b.is_empty() => b,
            _ => continue,
        };

        let args = parse_cmdline_bytes(&cmdline_bytes);
        let Some(mode) = classify_cmdline(&args) else {
            continue;
        };

        let elapsed_secs = fs::metadata(&proc_dir)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|started| now.duration_since(started).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let candidate = ActiveRecording {
            pid,
            mode,
            elapsed_secs,
        };

        if best.is_none_or(|prev| candidate.pid > prev.pid) {
            best = Some(candidate);
        }
    }

    best
}

pub fn detect_active_recording() -> Option<ActiveRecording> {
    detect_active_recording_in(Path::new("/proc"), SystemTime::now())
}

pub fn format_elapsed(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let rem_secs = secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{rem_secs:02}")
    } else {
        format!("{mins:02}:{rem_secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn resolves_empty_path_to_default_binary() {
        assert_eq!(effective_path(""), "gsr-ui-cli");
        assert_eq!(effective_path("   "), "gsr-ui-cli");
    }

    #[test]
    fn preserves_configured_custom_path() {
        assert_eq!(
            effective_path("/usr/local/bin/gsr-ui-cli"),
            "/usr/local/bin/gsr-ui-cli"
        );
        assert_eq!(effective_path("  /opt/gsr-ui-cli  "), "/opt/gsr-ui-cli");
    }

    #[test]
    fn constructs_toggle_record_and_window_commands() {
        let screen_cmd = toggle_record_command("gsr-ui-cli");
        assert_eq!(screen_cmd.get_program(), "gsr-ui-cli");
        assert_eq!(
            screen_cmd.get_args().collect::<Vec<_>>(),
            vec!["toggle-record"]
        );

        let window_cmd = toggle_record_window_command("/custom/gsr-ui-cli");
        assert_eq!(window_cmd.get_program(), "/custom/gsr-ui-cli");
        assert_eq!(
            window_cmd.get_args().collect::<Vec<_>>(),
            vec!["toggle-record-window"]
        );

        let status_cmd = status_command("");
        assert_eq!(status_cmd.get_program(), "gsr-ui-cli");
        assert_eq!(status_cmd.get_args().collect::<Vec<_>>(), vec!["status"]);
    }

    #[test]
    fn parses_nul_separated_cmdline_bytes() {
        let raw = b"/usr/bin/gpu-screen-recorder\0-w\0HDMI-A-1\0-o\0/home/mario/Videos/Video.mp4\0";
        assert_eq!(
            parse_cmdline_bytes(raw),
            vec![
                "/usr/bin/gpu-screen-recorder",
                "-w",
                "HDMI-A-1",
                "-o",
                "/home/mario/Videos/Video.mp4"
            ]
        );
    }

    #[test]
    fn classifies_monitor_and_screen_targets_as_screen_mode() {
        let args = to_args(&[
            "/usr/bin/gpu-screen-recorder",
            "-w",
            "HDMI-A-1",
            "-f",
            "60",
            "-o",
            "/home/mario/Videos/Video_2026-10-23_21-00-00.mp4",
        ]);
        assert_eq!(classify_cmdline(&args), Some(CaptureMode::Screen));

        let screen_with_webcam = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "screen|/dev/video0;halign=end",
            "-o",
            "/tmp/out.mkv",
        ]);
        assert_eq!(
            classify_cmdline(&screen_with_webcam),
            Some(CaptureMode::Screen)
        );
    }

    #[test]
    fn classifies_wayland_portal_window_token_and_x11_window_as_window_mode() {
        let wayland_window = to_args(&[
            "/usr/bin/gpu-screen-recorder",
            "-w",
            "portal",
            "-portal-session-token-filepath",
            "/tmp/gsr-ui-window-capture-token",
            "-o",
            "/home/mario/Videos/Video_2026-10-23_21-00-00.mp4",
        ]);
        assert_eq!(classify_cmdline(&wayland_window), Some(CaptureMode::Window));

        let x11_window_id = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "62914571",
            "-o",
            "/home/mario/Videos/window.mp4",
        ]);
        assert_eq!(classify_cmdline(&x11_window_id), Some(CaptureMode::Window));

        let x11_focused = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "focused",
            "-s",
            "1920x1080",
            "-o",
            "/home/mario/Videos/focused.mp4",
        ]);
        assert_eq!(classify_cmdline(&x11_focused), Some(CaptureMode::Window));
    }

    #[test]
    fn ignores_replay_buffer_streaming_screenshots_and_info_commands() {
        let replay = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "HDMI-A-1",
            "-r",
            "60",
            "-o",
            "/home/mario/Videos",
        ]);
        assert_eq!(classify_cmdline(&replay), None);

        let stream = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "HDMI-A-1",
            "-o",
            "rtmp://live.twitch.tv/app/key",
        ]);
        assert_eq!(classify_cmdline(&stream), None);

        let screenshot = to_args(&[
            "gpu-screen-recorder",
            "-w",
            "HDMI-A-1",
            "-o",
            "/home/mario/Pictures/Screenshot.jpg",
        ]);
        assert_eq!(classify_cmdline(&screenshot), None);

        let info = to_args(&["gpu-screen-recorder", "--info"]);
        assert_eq!(classify_cmdline(&info), None);
    }

    #[test]
    fn formats_elapsed_duration_as_mm_ss_and_h_mm_ss() {
        assert_eq!(format_elapsed(0), "00:00");
        assert_eq!(format_elapsed(9), "00:09");
        assert_eq!(format_elapsed(84), "01:24");
        assert_eq!(format_elapsed(3599), "59:59");
        assert_eq!(format_elapsed(3600), "1:00:00");
        assert_eq!(format_elapsed(3665), "1:01:05");
    }

    #[test]
    fn detects_active_recording_from_mock_proc_directory() {
        let temp = std::env::temp_dir().join(format!("gsr_proc_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("1001")).unwrap();
        fs::create_dir_all(temp.join("2042")).unwrap();
        fs::create_dir_all(temp.join("self")).unwrap();

        // 1001 is a replay buffer (should be ignored)
        fs::write(
            temp.join("1001/cmdline"),
            b"/usr/bin/gpu-screen-recorder\0-w\0HDMI-A-1\0-r\060\0-o\0/home/mario/Videos\0",
        )
        .unwrap();

        // 2042 is an active window recording
        fs::write(
            temp.join("2042/cmdline"),
            b"/usr/bin/gpu-screen-recorder\0-w\0portal\0-portal-session-token-filepath\0/tmp/gsr-ui-window-capture-token\0-o\0/home/mario/Videos/out.mp4\0",
        )
        .unwrap();

        let detected = detect_active_recording_in(&temp, SystemTime::now());
        let _ = fs::remove_dir_all(&temp);

        let rec = detected.expect("expected active recording to be detected");
        assert_eq!(rec.pid, 2042);
        assert_eq!(rec.mode, CaptureMode::Window);
    }
}
