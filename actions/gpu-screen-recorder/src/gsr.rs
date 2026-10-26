// specs/007_gpu_screen_recorder.md
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub const DEFAULT_BINARY: &str = "gsr-ui-cli";
pub const DEFAULT_DAEMON_BINARY: &str = "gsr-ui";
pub const DEFAULT_RECORDER_BINARY: &str = "gpu-screen-recorder";
pub const DEFAULT_NOTIFY_BINARY: &str = "gsr-notify";
pub const OPEN_ACTIONS_WINDOW_TOKEN_FILE: &str = "/tmp/gsr-open-actions-window-token";

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct AudioTrackItem {
    pub audio_inputs: Vec<String>,
}

impl Default for AudioTrackItem {
    fn default() -> Self {
        Self {
            audio_inputs: vec!["device:default_output".to_string()],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct RecordOptions {
    pub audio_codec: String,
    pub audio_track_item: Vec<AudioTrackItem>,
    pub codec: String,
    pub color_range: String,
    pub fps: u32,
    pub framerate_mode: String,
    pub record_cursor: bool,
    pub show_notifications: bool,
    pub video_bitrate: u32,
    pub video_quality: String,
}

impl Default for RecordOptions {
    fn default() -> Self {
        Self {
            audio_codec: "opus".to_string(),
            audio_track_item: vec![AudioTrackItem::default()],
            codec: "auto".to_string(),
            color_range: "limited".to_string(),
            fps: 60,
            framerate_mode: "auto".to_string(),
            record_cursor: true,
            show_notifications: true,
            video_bitrate: 40000,
            video_quality: "very_high".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct RecordSection {
    pub container: String,
    pub save_directory: String,
    pub record_options: RecordOptions,
}

impl Default for RecordSection {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            container: "mp4".to_string(),
            save_directory: format!("{home}/Videos"),
            record_options: RecordOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct GsrUiConfig {
    pub record: RecordSection,
}

pub fn effective_path(configured: &str) -> &str {
    let path = configured.trim();
    if path.is_empty() {
        DEFAULT_BINARY
    } else {
        path
    }
}

pub fn effective_daemon_path(configured_cli: &str) -> String {
    let cli = effective_path(configured_cli);
    let path = Path::new(cli);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        return parent
            .join(DEFAULT_DAEMON_BINARY)
            .to_string_lossy()
            .into_owned();
    }
    DEFAULT_DAEMON_BINARY.to_string()
}

pub fn effective_recorder_path(configured_cli: &str) -> String {
    let trimmed = configured_cli.trim();
    if trimmed.is_empty() {
        return DEFAULT_RECORDER_BINARY.to_string();
    }
    let path = Path::new(trimmed);
    if path.file_name().and_then(|s| s.to_str()) == Some(DEFAULT_BINARY) {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            return parent
                .join(DEFAULT_RECORDER_BINARY)
                .to_string_lossy()
                .into_owned();
        }
        return DEFAULT_RECORDER_BINARY.to_string();
    }
    trimmed.to_string()
}

pub fn launch_daemon_command(configured_cli: &str) -> Command {
    let mut command = Command::new(effective_daemon_path(configured_cli));
    command.arg("launch-daemon");
    command.current_dir("/tmp");
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command
}

pub fn status_command(configured: &str) -> Command {
    let mut command = Command::new(effective_path(configured));
    command.arg("status");
    command.current_dir("/tmp");
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command
}

pub fn ensure_daemon_running(configured: &str) -> io::Result<()> {
    if is_daemon_running(configured) {
        return Ok(());
    }

    let mut child = launch_daemon_command(configured).spawn()?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(100));
        if is_daemon_running(configured) {
            return Ok(());
        }
    }

    Err(io::Error::other(
        "timed out waiting for gsr-ui daemon to start",
    ))
}

pub fn is_daemon_running(configured: &str) -> bool {
    let trimmed = configured.trim();
    if (trimmed.is_empty() || trimmed == DEFAULT_BINARY) && is_background_process_running() {
        return true;
    }
    let Ok(output) = status_command(configured).output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim() == "running"
}

pub fn parse_ui_config(json_str: &str) -> GsrUiConfig {
    let mut cfg: GsrUiConfig = serde_json::from_str(json_str).unwrap_or_default();
    if cfg.record.container.trim().is_empty() {
        cfg.record.container = "mp4".to_string();
    }
    if cfg.record.save_directory.trim().is_empty() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        cfg.record.save_directory = format!("{home}/Videos");
    }
    if cfg.record.record_options.fps == 0 {
        cfg.record.record_options.fps = 60;
    }
    cfg
}

pub fn load_ui_config() -> GsrUiConfig {
    let config_path = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .map(|base| base.join("gpu-screen-recorder/config_ui"));

    if let Some(path) = config_path
        && let Ok(contents) = fs::read_to_string(path)
    {
        return parse_ui_config(&contents);
    }
    GsrUiConfig::default()
}

pub fn detect_primary_monitor(recorder_bin: &str) -> String {
    let output = Command::new(recorder_bin)
        .arg("--list-monitors")
        .current_dir("/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    if let Ok(out) = output
        && out.status.success()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let name = trimmed.split('|').next().unwrap_or(trimmed).trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "screen".to_string()
}

fn current_timestamp_string() -> String {
    if let Ok(out) = Command::new("date")
        .arg("+%Y-%m-%d_%H-%M-%S")
        .current_dir("/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

pub fn build_output_filepath(config: &GsrUiConfig) -> io::Result<PathBuf> {
    let raw_dir = config.record.save_directory.trim();
    let expanded_dir = if let Some(rest) = raw_dir.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(rest)
    } else if raw_dir == "~" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
    } else {
        PathBuf::from(raw_dir)
    };

    fs::create_dir_all(&expanded_dir)?;
    let ext = config.record.container.trim().trim_start_matches('.');
    let timestamp = current_timestamp_string();
    Ok(expanded_dir.join(format!("Video_{timestamp}.{ext}")))
}

pub fn build_recorder_command(
    recorder_bin: &str,
    target: &str,
    portal_token_path: Option<&Path>,
    is_x11_focused: bool,
    config: &GsrUiConfig,
    output_file: &Path,
) -> Command {
    let opts = &config.record.record_options;
    let mut command = Command::new(recorder_bin);
    command.arg("-w").arg(target);

    if is_x11_focused {
        command.arg("-s").arg("1920x1080");
    }

    command.arg("-c").arg(config.record.container.trim());
    command.arg("-f").arg(opts.fps.to_string());

    if opts.video_quality == "custom" {
        let bitrate = if opts.video_bitrate > 0 {
            opts.video_bitrate
        } else {
            40000
        };
        command.arg("-bm").arg("cbr");
        command.arg("-q").arg(format!("{bitrate}k"));
    } else {
        let quality = if opts.video_quality.trim().is_empty() {
            "very_high"
        } else {
            opts.video_quality.trim()
        };
        command.arg("-bm").arg("auto");
        command.arg("-q").arg(quality);
    }

    let codec = if opts.codec.trim().is_empty() {
        "auto"
    } else {
        opts.codec.trim()
    };
    command.arg("-k").arg(codec);

    let audio_codec = if opts.audio_codec.trim().is_empty() {
        "opus"
    } else {
        opts.audio_codec.trim()
    };
    command.arg("-ac").arg(audio_codec);

    let color_range = if opts.color_range.trim().is_empty() {
        "limited"
    } else {
        opts.color_range.trim()
    };
    command.arg("-cr").arg(color_range);

    let fm = match opts.framerate_mode.trim() {
        "cfr" => "cfr",
        "content" => "content",
        _ => "vfr",
    };
    command.arg("-fm").arg(fm);

    command
        .arg("-cursor")
        .arg(if opts.record_cursor { "yes" } else { "no" });

    let mut added_audio = false;
    for track in &opts.audio_track_item {
        let inputs: Vec<&str> = track
            .audio_inputs
            .iter()
            .map(String::as_str)
            .filter(|s| !s.trim().is_empty())
            .collect();
        if !inputs.is_empty() {
            command.arg("-a").arg(inputs.join("|"));
            added_audio = true;
        }
    }
    if !added_audio {
        command.arg("-a").arg("device:default_output");
    }

    command.arg("-v").arg("no");

    if let Some(token_path) = portal_token_path {
        command.arg("-restore-portal-session").arg("yes");
        command
            .arg("-portal-session-token-filepath")
            .arg(token_path);
    }

    command.arg("-o").arg(output_file);
    command.current_dir("/tmp");
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    command
}

const WAYLAND_FOCUSED_WINDOW_HELPER_PY: &str = r#"
import sys, time, uuid, gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

token_path = sys.argv[1]
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

bus.call_sync(
    'org.freedesktop.DBus',
    '/org/freedesktop/DBus',
    'org.freedesktop.DBus',
    'RequestName',
    GLib.Variant('(su)', ('org.freedesktop.impl.portal.desktop.gtk', 7)),
    GLib.VariantType('(u)'),
    Gio.DBusCallFlags.NONE,
    -1,
    None,
)
try:
    res = bus.call_sync(
        'org.gnome.Shell.Introspect',
        '/org/gnome/Shell/Introspect',
        'org.gnome.Shell.Introspect',
        'GetWindows',
        None,
        GLib.VariantType('(a{ta{sv}})'),
        Gio.DBusCallFlags.NONE,
        -1,
        None,
    )
finally:
    bus.call_sync(
        'org.freedesktop.DBus',
        '/org/freedesktop/DBus',
        'org.freedesktop.DBus',
        'ReleaseName',
        GLib.Variant('(s)', ('org.freedesktop.impl.portal.desktop.gtk',)),
        GLib.VariantType('(u)'),
        Gio.DBusCallFlags.NONE,
        -1,
        None,
    )

windows = res.unpack()[0]
focused = None
fallback = None
for _wid, props in windows.items():
    app = str(props.get('app-id', ''))
    if app == 'gsr-ui':
        continue
    if props.get('has-focus'):
        focused = (app, str(props.get('title', '')))
        break
    if fallback is None and props.get('window-type', 0) == 0 and not props.get('is-hidden', False):
        fallback = (app, str(props.get('title', '')))

target = focused or fallback
if not target:
    sys.exit('no window found to record')

app_id, title = target
token = str(uuid.uuid4())
now_us = int(time.time() * 1_000_000)

stream_info = GLib.Variant('(ss)', (app_id, title))
inner = GLib.Variant('(xxa(uuv))', (now_us, now_us, [(0, 2, stream_info)]))
outer_data = GLib.Variant('(suv)', ('GNOME', 1, inner))

perms = {
    '': ['yes'],
    'gpu-screen-recorder': ['yes'],
    'gsr-ui': ['yes'],
    'opendeck': ['yes'],
    'gnome-opendeck': ['yes'],
    'antigravity': ['yes'],
}
try:
    with open('/proc/self/cgroup', 'r', encoding='utf-8') as f:
        for line in f:
            unit = line.strip().split('/')[-1]
            if unit.startswith('app-') and (unit.endswith('.scope') or unit.endswith('.service')):
                body = unit[4:].rsplit('.', 1)[0]
                if '-' in body and body.rsplit('-', 1)[-1].isdigit():
                    body = body.rsplit('-', 1)[0]
                body_unesc = body.replace('\\x2d', '-')
                perms[body] = ['yes']
                perms[body_unesc] = ['yes']
                for prefix in ('gnome-', 'flatpak-', 'dbus-'):
                    if body_unesc.startswith(prefix):
                        perms[body_unesc[len(prefix):]] = ['yes']
except Exception:
    pass

bus.call_sync(
    'org.freedesktop.impl.portal.PermissionStore',
    '/org/freedesktop/impl/portal/PermissionStore',
    'org.freedesktop.impl.portal.PermissionStore',
    'Set',
    GLib.Variant('(sbsa{sas}v)', ('screencast', True, token, perms, outer_data)),
    None,
    Gio.DBusCallFlags.NONE,
    -1,
    None,
)

with open(token_path, 'w', encoding='utf-8') as f:
    f.write(token)

print(title or app_id)
"#;

pub fn prepare_wayland_focused_window_token(token_path: &Path) -> io::Result<String> {
    let output = Command::new("python3")
        .arg("-P")
        .arg("-c")
        .arg(WAYLAND_FOCUSED_WINDOW_HELPER_PY)
        .arg(token_path)
        .current_dir("/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "failed to prepare Wayland focused window token: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_wayland_session() -> bool {
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE")
        && session_type.eq_ignore_ascii_case("wayland")
    {
        return true;
    }
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn notify_recording_event(config: &GsrUiConfig, text: &str) {
    if !config.record.record_options.show_notifications {
        return;
    }
    let _ = Command::new(DEFAULT_NOTIFY_BINARY)
        .args(["--text", text, "--timeout", "2.0", "--icon", "record"])
        .current_dir("/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn spawn_and_verify(mut command: Command) -> io::Result<u32> {
    let mut child = command.spawn()?;
    let pid = child.id();

    for _ in 0..7 {
        thread::sleep(Duration::from_millis(50));
        if let Some(status) = child.try_wait()? {
            return Err(io::Error::other(format!(
                "gpu-screen-recorder exited prematurely with status {status}"
            )));
        }
    }

    thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(pid)
}

pub fn stop_active_recording_in(proc_root: &Path, pid: u32) -> io::Result<()> {
    let status = Command::new("kill")
        .args(["-s", "INT", &pid.to_string()])
        .current_dir("/tmp")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "kill -s INT {pid} exited with {status}"
        )));
    }

    let cmdline_path = proc_root.join(pid.to_string()).join("cmdline");
    for _ in 0..50 {
        let still_alive = fs::read(&cmdline_path)
            .map(|b| !b.is_empty())
            .unwrap_or(false);
        if !still_alive {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

pub fn stop_active_recording(pid: u32) -> io::Result<()> {
    stop_active_recording_in(Path::new("/proc"), pid)
}

pub fn is_background_process_cmdline(args: &[String]) -> bool {
    let Some(first) = args.first() else {
        return false;
    };
    let Some(exe_name) = Path::new(first).file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if exe_name != "gsr-ui" && exe_name != "gpu-screen-recorder" {
        return false;
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
    !args.iter().any(|arg| info_flags.contains(&arg.as_str()))
}

pub fn detect_background_pids_in(proc_root: &Path) -> Vec<u32> {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return Vec::new();
    };
    let mut pids = Vec::new();

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
        if is_background_process_cmdline(&args) {
            pids.push(pid);
        }
    }

    pids.sort_unstable();
    pids
}

pub fn is_background_process_running_in(proc_root: &Path) -> bool {
    !detect_background_pids_in(proc_root).is_empty()
}

pub fn is_background_process_running() -> bool {
    is_background_process_running_in(Path::new("/proc"))
}

pub fn kill_background_processes_in(proc_root: &Path) -> io::Result<()> {
    if let Some(active) = detect_active_recording_in(proc_root, SystemTime::now()) {
        let _ = stop_active_recording_in(proc_root, active.pid);
    }

    let pids = detect_background_pids_in(proc_root);
    if pids.is_empty() {
        return Ok(());
    }

    for pid in &pids {
        let _ = Command::new("kill")
            .args(["-s", "TERM", &pid.to_string()])
            .current_dir("/tmp")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    for _ in 0..20 {
        if !is_background_process_running_in(proc_root) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    for pid in detect_background_pids_in(proc_root) {
        let _ = Command::new("kill")
            .args(["-s", "KILL", &pid.to_string()])
            .current_dir("/tmp")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    for _ in 0..6 {
        if !is_background_process_running_in(proc_root) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

pub fn kill_background_processes() -> io::Result<()> {
    kill_background_processes_in(Path::new("/proc"))
}

pub fn toggle_daemon(configured: &str) -> io::Result<bool> {
    let config = load_ui_config();
    let trimmed = configured.trim();
    let use_proc = trimmed.is_empty() || trimmed == DEFAULT_BINARY;
    let running = if use_proc {
        is_background_process_running() || is_daemon_running(configured)
    } else {
        is_daemon_running(configured)
    };

    if running {
        kill_background_processes()?;
        notify_recording_event(&config, "Stopped GPU Screen Recorder");
        Ok(false)
    } else {
        ensure_daemon_running(configured)?;
        notify_recording_event(&config, "Started GPU Screen Recorder");
        Ok(true)
    }
}

pub fn start_screen_recording(configured: &str, config: &GsrUiConfig) -> io::Result<u32> {
    let recorder_bin = effective_recorder_path(configured);
    let monitor = detect_primary_monitor(&recorder_bin);
    let output_file = build_output_filepath(config)?;
    let command =
        build_recorder_command(&recorder_bin, &monitor, None, false, config, &output_file);
    let pid = spawn_and_verify(command)?;
    notify_recording_event(config, "Started recording screen");
    Ok(pid)
}

pub fn start_window_recording(configured: &str, config: &GsrUiConfig) -> io::Result<u32> {
    let recorder_bin = effective_recorder_path(configured);
    let output_file = build_output_filepath(config)?;

    let (command, label) = if is_wayland_session() {
        let token_path = Path::new(OPEN_ACTIONS_WINDOW_TOKEN_FILE);
        let window_name = prepare_wayland_focused_window_token(token_path)?;
        let cmd = build_recorder_command(
            &recorder_bin,
            "portal",
            Some(token_path),
            false,
            config,
            &output_file,
        );
        let msg = if window_name.is_empty() {
            "Started recording focused window".to_string()
        } else {
            format!("Started recording \"{window_name}\"")
        };
        (cmd, msg)
    } else {
        let cmd =
            build_recorder_command(&recorder_bin, "focused", None, true, config, &output_file);
        (cmd, "Started recording focused window".to_string())
    };

    let pid = spawn_and_verify(command)?;
    notify_recording_event(config, &label);
    Ok(pid)
}

pub fn toggle_screen(configured: &str) -> io::Result<()> {
    let config = load_ui_config();
    if let Some(active) = detect_active_recording() {
        stop_active_recording(active.pid)?;
        if active.mode == CaptureMode::Screen {
            notify_recording_event(&config, "Stopped recording and saved video");
            return Ok(());
        }
    }
    start_screen_recording(configured, &config).map(|_| ())
}

pub fn toggle_window(configured: &str) -> io::Result<()> {
    let config = load_ui_config();
    if let Some(active) = detect_active_recording() {
        stop_active_recording(active.pid)?;
        if active.mode == CaptureMode::Window {
            notify_recording_event(&config, "Stopped recording and saved video");
            return Ok(());
        }
    }
    start_window_recording(configured, &config).map(|_| ())
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
        && (token.contains("gsr-ui-window-capture-token")
            || token.contains("gsr-open-actions-window-token"))
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
        assert_eq!(effective_recorder_path(""), "gpu-screen-recorder");
        assert_eq!(effective_recorder_path("gsr-ui-cli"), "gpu-screen-recorder");
        assert_eq!(
            effective_recorder_path("/opt/gsr/bin/gsr-ui-cli"),
            "/opt/gsr/bin/gpu-screen-recorder"
        );
    }

    #[test]
    fn preserves_configured_custom_path() {
        assert_eq!(
            effective_path("/usr/local/bin/gsr-ui-cli"),
            "/usr/local/bin/gsr-ui-cli"
        );
        assert_eq!(effective_path("  /opt/gsr-ui-cli  "), "/opt/gsr-ui-cli");
        assert_eq!(
            effective_recorder_path("/custom/bin/gpu-screen-recorder"),
            "/custom/bin/gpu-screen-recorder"
        );
    }

    #[test]
    fn constructs_status_command() {
        let status_cmd = status_command("");
        assert_eq!(status_cmd.get_program(), "gsr-ui-cli");
        assert_eq!(status_cmd.get_args().collect::<Vec<_>>(), vec!["status"]);
    }

    #[test]
    fn parses_ui_config_and_builds_recorder_command() {
        let json = r#"{
            "record": {
                "container": "mkv",
                "save_directory": "/tmp/videos",
                "record_options": {
                    "fps": 120,
                    "video_quality": "ultra",
                    "audio_codec": "aac",
                    "color_range": "full",
                    "framerate_mode": "cfr",
                    "record_cursor": false,
                    "show_notifications": false,
                    "audio_track_item": [
                        { "audio_inputs": ["device:default_output", "device:default_input"] }
                    ]
                }
            }
        }"#;
        let cfg = parse_ui_config(json);
        assert_eq!(cfg.record.container, "mkv");
        assert_eq!(cfg.record.save_directory, "/tmp/videos");
        assert_eq!(cfg.record.record_options.fps, 120);

        let cmd = build_recorder_command(
            "gpu-screen-recorder",
            "portal",
            Some(Path::new("/tmp/gsr-open-actions-window-token")),
            false,
            &cfg,
            Path::new("/tmp/videos/Video_2026-10-25.mkv"),
        );
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "-w",
                "portal",
                "-c",
                "mkv",
                "-f",
                "120",
                "-bm",
                "auto",
                "-q",
                "ultra",
                "-k",
                "auto",
                "-ac",
                "aac",
                "-cr",
                "full",
                "-fm",
                "cfr",
                "-cursor",
                "no",
                "-a",
                "device:default_output|device:default_input",
                "-v",
                "no",
                "-restore-portal-session",
                "yes",
                "-portal-session-token-filepath",
                "/tmp/gsr-open-actions-window-token",
                "-o",
                "/tmp/videos/Video_2026-10-25.mkv",
            ]
        );
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

        let open_actions_wayland_window = to_args(&[
            "/usr/bin/gpu-screen-recorder",
            "-w",
            "portal",
            "-restore-portal-session",
            "yes",
            "-portal-session-token-filepath",
            "/tmp/gsr-open-actions-window-token",
            "-o",
            "/home/mario/Videos/Video_2026-10-25_14-00-00.mp4",
        ]);
        assert_eq!(
            classify_cmdline(&open_actions_wayland_window),
            Some(CaptureMode::Window)
        );

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
            b"/usr/bin/gpu-screen-recorder\0-w\0portal\0-portal-session-token-filepath\0/tmp/gsr-open-actions-window-token\0-o\0/home/mario/Videos/out.mp4\0",
        )
        .unwrap();

        let detected = detect_active_recording_in(&temp, SystemTime::now());
        let _ = fs::remove_dir_all(&temp);

        let rec = detected.expect("expected active recording to be detected");
        assert_eq!(rec.pid, 2042);
        assert_eq!(rec.mode, CaptureMode::Window);
    }

    #[test]
    fn missing_or_failing_binary_returns_error_and_offline_status() {
        let missing = "/nonexistent/path/to/gsr-ui-cli";
        let mut cfg = GsrUiConfig::default();
        cfg.record.record_options.show_notifications = false;
        assert!(start_screen_recording(missing, &cfg).is_err());
        assert!(!is_daemon_running(missing));
        assert!(!is_daemon_running("false"));
    }

    #[test]
    fn resolves_daemon_binary_path_and_launch_command() {
        assert_eq!(effective_daemon_path(""), "gsr-ui");
        assert_eq!(effective_daemon_path("gsr-ui-cli"), "gsr-ui");
        assert_eq!(
            effective_daemon_path("/opt/gsr/bin/gsr-ui-cli"),
            "/opt/gsr/bin/gsr-ui"
        );

        let cmd = launch_daemon_command("/opt/gsr/bin/gsr-ui-cli");
        assert_eq!(cmd.get_program(), "/opt/gsr/bin/gsr-ui");
        assert_eq!(cmd.get_args().collect::<Vec<_>>(), vec!["launch-daemon"]);
    }

    #[test]
    fn auto_launches_daemon_and_starts_and_stops_recorder_process() {
        use std::os::unix::fs::PermissionsExt;

        let temp = std::env::temp_dir().join(format!("gsr_autolaunch_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let state_file = temp.join("daemon_running");
        let log_file = temp.join("invocations.log");
        let daemon_bin = temp.join("gsr-ui");
        let cli_bin = temp.join("gsr-ui-cli");
        let recorder_bin = temp.join("gpu-screen-recorder");

        let daemon_script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"launch-daemon\" ]; then\n  echo \"launched\" >> \"{}\"\n  touch \"{}\"\n  exit 0\nfi\nexit 1\n",
            log_file.display(),
            state_file.display()
        );
        fs::write(&daemon_bin, daemon_script).unwrap();
        fs::set_permissions(&daemon_bin, fs::Permissions::from_mode(0o755)).unwrap();

        let cli_script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"status\" ]; then\n  if [ -f \"{}\" ]; then\n    echo \"running\"\n    exit 0\n  else\n    echo \"not running\"\n    exit 1\n  fi\nfi\nexit 0\n",
            state_file.display()
        );
        fs::write(&cli_bin, cli_script).unwrap();
        fs::set_permissions(&cli_bin, fs::Permissions::from_mode(0o755)).unwrap();

        let recorder_script = format!(
            "#!/bin/sh\nif [ \"$1\" = \"--list-monitors\" ]; then\n  echo \"HDMI-A-1|1920x1080\"\n  exit 0\nfi\necho \"recorder:$1:$2\" >> \"{}\"\ntrap 'exit 0' INT TERM\nwhile :; do sleep 0.05; done\n",
            log_file.display()
        );
        fs::write(&recorder_bin, recorder_script).unwrap();
        fs::set_permissions(&recorder_bin, fs::Permissions::from_mode(0o755)).unwrap();

        let cli_str = cli_bin.to_str().unwrap();
        assert!(!is_daemon_running(cli_str));
        assert!(ensure_daemon_running(cli_str).is_ok());
        assert!(is_daemon_running(cli_str));

        let mut cfg = GsrUiConfig::default();
        cfg.record.save_directory = temp.to_string_lossy().into_owned();
        cfg.record.record_options.show_notifications = false;

        let pid =
            start_screen_recording(cli_str, &cfg).expect("expected screen recording to start");
        assert!(stop_active_recording(pid).is_ok());

        let log = fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines, vec!["launched", "recorder:-w:HDMI-A-1"]);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn detects_and_kills_background_processes_in_mock_proc() {
        let temp = std::env::temp_dir().join(format!("gsr_bg_proc_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("3001")).unwrap();
        fs::create_dir_all(temp.join("3002")).unwrap();
        fs::create_dir_all(temp.join("3003")).unwrap();

        fs::write(
            temp.join("3001/cmdline"),
            b"/usr/bin/gsr-ui\0launch-daemon\0",
        )
        .unwrap();
        fs::write(
            temp.join("3002/cmdline"),
            b"/usr/bin/gpu-screen-recorder\0--list-monitors\0",
        )
        .unwrap();
        fs::write(
            temp.join("3003/cmdline"),
            b"/home/mario/.config/opendeck/plugins/open-actions-gsr\0",
        )
        .unwrap();

        let pids = detect_background_pids_in(&temp);
        assert_eq!(pids, vec![3001]);
        assert!(is_background_process_running_in(&temp));

        let _ = fs::remove_dir_all(&temp);
    }
}
