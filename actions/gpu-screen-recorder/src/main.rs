// specs/007_gpu_screen_recorder.md
pub mod gsr;
pub mod icon;

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex as StdMutex},
    thread,
    time::Duration,
};

use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_instance, register_action, run,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::gsr::{ActiveRecording, CaptureMode};

pub const RECORD_SCREEN_UUID: &str = "io.github.mario.gpuscreenrecorder.record-screen";
pub const RECORD_WINDOW_UUID: &str = "io.github.mario.gpuscreenrecorder.record-window";
pub const TOGGLE_DAEMON_UUID: &str = "io.github.mario.gpuscreenrecorder.toggle-daemon";

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub gsr_ui_cli_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Capture(CaptureMode),
    Daemon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderedState {
    Capture(Option<u64>),
    Daemon(bool),
}

#[derive(Clone, Debug)]
struct InstanceEntry {
    kind: ActionKind,
    last_rendered: Option<RenderedState>,
}

pub fn elapsed_for_action(
    action_mode: CaptureMode,
    active: Option<ActiveRecording>,
) -> Option<u64> {
    active.and_then(|rec| (rec.mode == action_mode).then_some(rec.elapsed_secs))
}

fn compute_rendered_state(
    kind: ActionKind,
    active: Option<ActiveRecording>,
    bg_running: bool,
) -> RenderedState {
    match kind {
        ActionKind::Capture(mode) => RenderedState::Capture(elapsed_for_action(mode, active)),
        ActionKind::Daemon => RenderedState::Daemon(bg_running),
    }
}

fn render_state_image(kind: ActionKind, state: RenderedState) -> String {
    match (kind, state) {
        (ActionKind::Capture(mode), RenderedState::Capture(elapsed)) => {
            icon::render_tile(mode, elapsed)
        }
        (ActionKind::Daemon, RenderedState::Daemon(running)) => icon::render_daemon_tile(running),
        (ActionKind::Capture(mode), _) => icon::render_tile(mode, None),
        (ActionKind::Daemon, _) => icon::render_daemon_tile(false),
    }
}

fn log_event(message: &str) {
    eprintln!("{message}");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/open-actions-gsr.log")
    {
        let _ = writeln!(file, "[pid={}] {message}", std::process::id());
    }
}

fn self_binary_was_deleted() -> bool {
    fs::read_link("/proc/self/exe")
        .ok()
        .and_then(|p| p.to_str().map(|s| s.ends_with(" (deleted)")))
        .unwrap_or(false)
}

fn write_ws_text_frame(stream: &mut TcpStream, text: &str) -> io::Result<()> {
    let payload = text.as_bytes();
    let len = payload.len();
    if len < 126 {
        stream.write_all(&[0x81, len as u8])?;
    } else if len <= 65535 {
        stream.write_all(&[0x81, 126, (len >> 8) as u8, (len & 0xff) as u8])?;
    } else {
        let mut header = [0u8; 10];
        header[0] = 0x81;
        header[1] = 127;
        header[2..10].copy_from_slice(&(len as u64).to_be_bytes());
        stream.write_all(&header)?;
    }
    stream.write_all(payload)?;
    stream.flush()
}

fn build_profile_will_appear_events(info_json: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(info) = serde_json::from_str::<Value>(info_json) else {
        return out;
    };
    let Some(devices) = info.get("devices").and_then(Value::as_array) else {
        return out;
    };
    let Ok(home) = std::env::var("HOME") else {
        return out;
    };

    for dev in devices {
        let Some(device_id) = dev.get("id").and_then(Value::as_str) else {
            continue;
        };
        let selected_path = format!("{home}/.config/opendeck/profiles/{device_id}.json");
        let Ok(selected_raw) = fs::read_to_string(&selected_path) else {
            continue;
        };
        let Ok(selected_val) = serde_json::from_str::<Value>(&selected_raw) else {
            continue;
        };
        let Some(profile) = selected_val.get("selected_profile").and_then(Value::as_str) else {
            continue;
        };
        let profile_path = format!("{home}/.config/opendeck/profiles/{device_id}/{profile}.json");
        let Ok(profile_raw) = fs::read_to_string(&profile_path) else {
            continue;
        };
        let Ok(profile_val) = serde_json::from_str::<Value>(&profile_raw) else {
            continue;
        };
        let Some(keys) = profile_val.get("keys").and_then(Value::as_array) else {
            continue;
        };

        for (idx, key_entry) in keys.iter().enumerate() {
            let Some(key_obj) = key_entry.as_object() else {
                continue;
            };
            let Some(action_obj) = key_obj.get("action").and_then(Value::as_object) else {
                continue;
            };
            let Some(uuid) = action_obj.get("uuid").and_then(Value::as_str) else {
                continue;
            };
            if uuid != RECORD_SCREEN_UUID
                && uuid != RECORD_WINDOW_UUID
                && uuid != TOGGLE_DAEMON_UUID
            {
                continue;
            }
            let Some(short_ctx) = key_obj.get("context").and_then(Value::as_str) else {
                continue;
            };
            let full_context = format!("{device_id}.{profile}.{short_ctx}");
            let settings = key_obj
                .get("settings")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let state = key_obj
                .get("current_state")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let event_json = json!({
                "event": "willAppear",
                "action": uuid,
                "context": full_context,
                "device": device_id,
                "payload": {
                    "settings": settings,
                    "coordinates": {
                        "row": (idx / 6) as u8,
                        "column": (idx % 6) as u8,
                    },
                    "controller": "Keypad",
                    "state": state,
                    "isInMultiAction": false,
                }
            })
            .to_string();
            out.push((full_context, event_json));
        }
    }
    out
}

fn setup_ws_bridge(mut args: Vec<String>) -> Vec<String> {
    let Some(port_idx) = args.iter().position(|x| x.eq_ignore_ascii_case("-port")) else {
        return args;
    };
    let Some(real_port) = args.get(port_idx + 1).cloned() else {
        return args;
    };
    let info_json = args
        .iter()
        .position(|x| x.eq_ignore_ascii_case("-info"))
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_default();

    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        return args;
    };
    let Ok(local_addr) = listener.local_addr() else {
        return args;
    };
    args[port_idx + 1] = local_addr.port().to_string();

    thread::spawn(move || {
        let Ok((mut client_read, _)) = listener.accept() else {
            return;
        };
        let Ok(client_write_stream) = client_read.try_clone() else {
            return;
        };
        let Ok(mut server_read) = TcpStream::connect(format!("127.0.0.1:{real_port}")) else {
            return;
        };
        let Ok(mut server_write) = server_read.try_clone() else {
            return;
        };

        thread::spawn(move || {
            let _ = io::copy(&mut client_read, &mut server_write);
        });

        // Forward HTTP 101 Switching Protocols response header until \r\n\r\n
        let mut header_buf = Vec::new();
        let mut byte = [0u8; 1];
        while server_read.read_exact(&mut byte).is_ok() {
            header_buf.push(byte[0]);
            if header_buf.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        let client_writer = Arc::new(StdMutex::new(client_write_stream));
        if let Ok(mut w) = client_writer.lock()
            && (w.write_all(&header_buf).is_err() || w.flush().is_err())
        {
            return;
        }

        let seen_contexts: Arc<StdMutex<HashSet<String>>> = Arc::new(StdMutex::new(HashSet::new()));

        let synth_writer = Arc::clone(&client_writer);
        let synth_seen = Arc::clone(&seen_contexts);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            let events = build_profile_will_appear_events(&info_json);
            for (ctx, ev_json) in events {
                let should_send = synth_seen
                    .lock()
                    .map(|mut s| s.insert(ctx.clone()))
                    .unwrap_or(false);
                if should_send && let Ok(mut w) = synth_writer.lock() {
                    log_event(&format!("synthesizing willAppear for {ctx}"));
                    let _ = write_ws_text_frame(&mut w, &ev_json);
                }
            }
        });

        loop {
            let mut hdr = [0u8; 2];
            if server_read.read_exact(&mut hdr).is_err() {
                break;
            }
            let opcode = hdr[0] & 0x0f;
            let masked = (hdr[1] & 0x80) != 0;
            let len7 = (hdr[1] & 0x7f) as usize;

            let mut frame_bytes = vec![hdr[0], hdr[1]];
            let payload_len = if len7 == 126 {
                let mut ext = [0u8; 2];
                if server_read.read_exact(&mut ext).is_err() {
                    break;
                }
                frame_bytes.extend_from_slice(&ext);
                u16::from_be_bytes(ext) as usize
            } else if len7 == 127 {
                let mut ext = [0u8; 8];
                if server_read.read_exact(&mut ext).is_err() {
                    break;
                }
                frame_bytes.extend_from_slice(&ext);
                u64::from_be_bytes(ext) as usize
            } else {
                len7
            };

            if masked {
                let mut mask = [0u8; 4];
                if server_read.read_exact(&mut mask).is_err() {
                    break;
                }
                frame_bytes.extend_from_slice(&mask);
            }

            let payload_offset = frame_bytes.len();
            frame_bytes.resize(payload_offset + payload_len, 0);
            if server_read
                .read_exact(&mut frame_bytes[payload_offset..])
                .is_err()
            {
                break;
            }

            let mut synth_before: Option<String> = None;
            if opcode == 1
                && !masked
                && let Ok(text) = std::str::from_utf8(&frame_bytes[payload_offset..])
                && let Ok(val) = serde_json::from_str::<Value>(text)
                && let Some(ev) = val.get("event").and_then(Value::as_str)
            {
                if ev == "willAppear" {
                    if let Some(ctx) = val.get("context").and_then(Value::as_str)
                        && let Ok(mut s) = seen_contexts.lock()
                    {
                        s.insert(ctx.to_string());
                    }
                } else if (ev == "keyDown" || ev == "keyUp" || ev == "didReceiveSettings")
                    && let Some(ctx) = val.get("context").and_then(Value::as_str)
                {
                    let is_new = seen_contexts
                        .lock()
                        .map(|mut s| s.insert(ctx.to_string()))
                        .unwrap_or(false);
                    if is_new {
                        let mut appear = val.clone();
                        if let Some(obj) = appear.as_object_mut() {
                            obj.insert("event".to_string(), json!("willAppear"));
                        }
                        synth_before = Some(appear.to_string());
                    }
                }
            }

            if let Ok(mut w) = client_writer.lock() {
                if let Some(appear_json) = synth_before {
                    log_event("synthesizing on-demand willAppear before key event");
                    let _ = write_ws_text_frame(&mut w, &appear_json);
                }
                if w.write_all(&frame_bytes).is_err() || w.flush().is_err() {
                    break;
                }
            }
        }
    });

    args
}

struct GsrCore {
    instances: Mutex<HashMap<String, InstanceEntry>>,
    toggle_lock: Mutex<()>,
}

impl GsrCore {
    fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
            toggle_lock: Mutex::new(()),
        }
    }

    async fn upsert_instance(
        &self,
        instance: &Instance,
        kind: ActionKind,
        _settings: &Settings,
    ) -> OpenActionResult<()> {
        log_event(&format!(
            "upsert_instance id={} kind={kind:?}",
            instance.instance_id
        ));
        let active = gsr::detect_active_recording();
        let bg_running = active.is_some() || gsr::is_background_process_running();
        let state = compute_rendered_state(kind, active, bg_running);
        {
            let mut map = self.instances.lock().await;
            map.insert(
                instance.instance_id.clone(),
                InstanceEntry {
                    kind,
                    last_rendered: Some(state),
                },
            );
        }
        instance
            .set_image(Some(render_state_image(kind, state)), None)
            .await
    }

    async fn remove_instance(&self, instance_id: &str) {
        self.instances.lock().await.remove(instance_id);
    }

    async fn sync_all(&self, force: bool) {
        let is_empty = self.instances.lock().await.is_empty();
        if is_empty {
            return;
        }

        let active = gsr::detect_active_recording();
        let bg_running = active.is_some() || gsr::is_background_process_running();
        let mut updates: Vec<(String, ActionKind, RenderedState)> = Vec::new();

        {
            let mut map = self.instances.lock().await;
            for (instance_id, entry) in map.iter_mut() {
                let target_state = compute_rendered_state(entry.kind, active, bg_running);
                if force || entry.last_rendered != Some(target_state) {
                    entry.last_rendered = Some(target_state);
                    updates.push((instance_id.clone(), entry.kind, target_state));
                }
            }
        }

        for (instance_id, kind, state) in updates {
            if let Some(instance) = get_instance(instance_id).await {
                let _ = instance
                    .set_image(Some(render_state_image(kind, state)), None)
                    .await;
            }
        }
    }

    async fn toggle(
        &self,
        instance: &Instance,
        mode: CaptureMode,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        let _guard = self.toggle_lock.lock().await;
        let path = settings.gsr_ui_cli_path.clone();
        let result = tokio::task::spawn_blocking(move || match mode {
            CaptureMode::Screen => gsr::toggle_screen(&path),
            CaptureMode::Window => gsr::toggle_window(&path),
        })
        .await;

        let succeeded = match &result {
            Ok(Ok(())) => {
                log_event(&format!("toggle({mode:?}) succeeded"));
                true
            }
            Ok(Err(err)) => {
                log_event(&format!("toggle({mode:?}) error: {err}"));
                false
            }
            Err(err) => {
                log_event(&format!("toggle({mode:?}) join error: {err}"));
                false
            }
        };
        if !succeeded {
            instance.show_alert().await?;
            {
                let mut map = self.instances.lock().await;
                if let Some(entry) = map.get_mut(&instance.instance_id) {
                    entry.last_rendered = Some(RenderedState::Capture(None));
                }
            }
            return instance
                .set_image(Some(icon::render_tile(mode, None)), None)
                .await;
        }

        tokio::time::sleep(Duration::from_millis(120)).await;
        self.sync_all(true).await;
        Ok(())
    }

    async fn toggle_daemon(
        &self,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        let _guard = self.toggle_lock.lock().await;
        let path = settings.gsr_ui_cli_path.clone();
        let result = tokio::task::spawn_blocking(move || gsr::toggle_daemon(&path)).await;

        let succeeded = match &result {
            Ok(Ok(running)) => {
                log_event(&format!("toggle_daemon succeeded (running={running})"));
                true
            }
            Ok(Err(err)) => {
                log_event(&format!("toggle_daemon error: {err}"));
                false
            }
            Err(err) => {
                log_event(&format!("toggle_daemon join error: {err}"));
                false
            }
        };
        if !succeeded {
            instance.show_alert().await?;
        }

        self.sync_all(true).await;
        Ok(())
    }

    async fn handle_pi_message(
        &self,
        instance: &Instance,
        settings: &Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let event = payload
            .get("event")
            .or_else(|| payload.get("action"))
            .and_then(Value::as_str)
            .unwrap_or("");

        if event == "getStatus" {
            let path = settings.gsr_ui_cli_path.clone();
            let daemon_running = tokio::task::spawn_blocking(move || gsr::is_daemon_running(&path))
                .await
                .unwrap_or(false);
            let active = gsr::detect_active_recording();

            instance
                .send_to_property_inspector(json!({
                    "event": "status",
                    "daemonRunning": daemon_running,
                    "recording": active.is_some(),
                    "captureMode": active.map(|a| a.mode),
                    "elapsedSecs": active.map(|a| a.elapsed_secs),
                }))
                .await?;
        }
        Ok(())
    }
}

struct RecordScreenAction {
    core: Arc<GsrCore>,
}

#[async_trait]
impl Action for RecordScreenAction {
    const UUID: &'static str = RECORD_SCREEN_UUID;
    type Settings = Settings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Capture(CaptureMode::Screen), settings)
            .await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core.remove_instance(&instance.instance_id).await;
        Ok(())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Capture(CaptureMode::Screen), settings)
            .await
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.core
            .toggle(instance, CaptureMode::Screen, settings)
            .await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        self.core
            .handle_pi_message(instance, settings, payload)
            .await
    }
}

struct RecordWindowAction {
    core: Arc<GsrCore>,
}

#[async_trait]
impl Action for RecordWindowAction {
    const UUID: &'static str = RECORD_WINDOW_UUID;
    type Settings = Settings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Capture(CaptureMode::Window), settings)
            .await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core.remove_instance(&instance.instance_id).await;
        Ok(())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Capture(CaptureMode::Window), settings)
            .await
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.core
            .toggle(instance, CaptureMode::Window, settings)
            .await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        self.core
            .handle_pi_message(instance, settings, payload)
            .await
    }
}

struct ToggleDaemonAction {
    core: Arc<GsrCore>,
}

#[async_trait]
impl Action for ToggleDaemonAction {
    const UUID: &'static str = TOGGLE_DAEMON_UUID;
    type Settings = Settings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Daemon, settings)
            .await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core.remove_instance(&instance.instance_id).await;
        Ok(())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, ActionKind::Daemon, settings)
            .await
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.core.toggle_daemon(instance, settings).await
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        self.core
            .handle_pi_message(instance, settings, payload)
            .await
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let _ = std::env::set_current_dir("/tmp");
    log_event("open-actions-gsr starting");
    let core = Arc::new(GsrCore::new());

    let watcher_core = Arc::clone(&core);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(1000));
        loop {
            ticker.tick().await;
            if self_binary_was_deleted() {
                log_event("binary replaced on disk; exiting so new instance can register");
                std::process::exit(0);
            }
            watcher_core.sync_all(false).await;
        }
    });

    register_action(RecordScreenAction {
        core: Arc::clone(&core),
    })
    .await;
    register_action(RecordWindowAction {
        core: Arc::clone(&core),
    })
    .await;
    register_action(ToggleDaemonAction { core }).await;

    let args = setup_ws_bridge(std::env::args().collect());
    run(args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_for_action_matches_only_matching_capture_mode() {
        let screen_rec = Some(ActiveRecording {
            pid: 1234,
            mode: CaptureMode::Screen,
            elapsed_secs: 45,
        });
        assert_eq!(
            elapsed_for_action(CaptureMode::Screen, screen_rec),
            Some(45)
        );
        assert_eq!(elapsed_for_action(CaptureMode::Window, screen_rec), None);

        let window_rec = Some(ActiveRecording {
            pid: 5678,
            mode: CaptureMode::Window,
            elapsed_secs: 12,
        });
        assert_eq!(elapsed_for_action(CaptureMode::Screen, window_rec), None);
        assert_eq!(
            elapsed_for_action(CaptureMode::Window, window_rec),
            Some(12)
        );

        assert_eq!(elapsed_for_action(CaptureMode::Screen, None), None);
        assert_eq!(elapsed_for_action(CaptureMode::Window, None), None);
    }

    #[test]
    fn computes_rendered_state_for_capture_and_daemon_actions() {
        let screen_rec = Some(ActiveRecording {
            pid: 100,
            mode: CaptureMode::Screen,
            elapsed_secs: 10,
        });
        assert_eq!(
            compute_rendered_state(ActionKind::Capture(CaptureMode::Screen), screen_rec, true),
            RenderedState::Capture(Some(10))
        );
        assert_eq!(
            compute_rendered_state(ActionKind::Daemon, screen_rec, true),
            RenderedState::Daemon(true)
        );
        assert_eq!(
            compute_rendered_state(ActionKind::Daemon, None, false),
            RenderedState::Daemon(false)
        );
    }

    #[test]
    fn settings_deserialize_with_default_empty_path() {
        let parsed: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.gsr_ui_cli_path, "");
    }
}
