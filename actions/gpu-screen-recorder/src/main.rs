// specs/007_gpu_screen_recorder.md
pub mod gsr;
pub mod icon;

use std::{collections::HashMap, sync::Arc, time::Duration};

use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_instance, register_action, run,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::gsr::{ActiveRecording, CaptureMode};

pub const RECORD_SCREEN_UUID: &str = "io.github.mario.gpuscreenrecorder.record-screen";
pub const RECORD_WINDOW_UUID: &str = "io.github.mario.gpuscreenrecorder.record-window";

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub gsr_ui_cli_path: String,
}

#[derive(Clone, Debug)]
struct InstanceEntry {
    mode: CaptureMode,
    last_rendered_elapsed: Option<Option<u64>>,
}

pub fn elapsed_for_action(
    action_mode: CaptureMode,
    active: Option<ActiveRecording>,
) -> Option<u64> {
    active.and_then(|rec| (rec.mode == action_mode).then_some(rec.elapsed_secs))
}

struct GsrCore {
    instances: Mutex<HashMap<String, InstanceEntry>>,
}

impl GsrCore {
    fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    async fn upsert_instance(
        &self,
        instance: &Instance,
        mode: CaptureMode,
    ) -> OpenActionResult<()> {
        {
            let mut map = self.instances.lock().await;
            map.insert(
                instance.instance_id.clone(),
                InstanceEntry {
                    mode,
                    last_rendered_elapsed: None,
                },
            );
        }
        let active = gsr::detect_active_recording();
        let elapsed = elapsed_for_action(mode, active);
        {
            let mut map = self.instances.lock().await;
            if let Some(entry) = map.get_mut(&instance.instance_id) {
                entry.last_rendered_elapsed = Some(elapsed);
            }
        }
        instance
            .set_image(Some(icon::render_tile(mode, elapsed)), None)
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
        let mut updates: Vec<(String, CaptureMode, Option<u64>)> = Vec::new();

        {
            let mut map = self.instances.lock().await;
            for (instance_id, entry) in map.iter_mut() {
                let target_elapsed = elapsed_for_action(entry.mode, active);
                if force || entry.last_rendered_elapsed != Some(target_elapsed) {
                    entry.last_rendered_elapsed = Some(target_elapsed);
                    updates.push((instance_id.clone(), entry.mode, target_elapsed));
                }
            }
        }

        for (instance_id, mode, elapsed) in updates {
            if let Some(instance) = get_instance(instance_id).await {
                let _ = instance
                    .set_image(Some(icon::render_tile(mode, elapsed)), None)
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
        let path = settings.gsr_ui_cli_path.clone();
        let result = tokio::task::spawn_blocking(move || match mode {
            CaptureMode::Screen => gsr::toggle_screen(&path),
            CaptureMode::Window => gsr::toggle_window(&path),
        })
        .await;

        let succeeded = matches!(result, Ok(Ok(())));
        if !succeeded {
            instance.show_alert().await?;
            {
                let mut map = self.instances.lock().await;
                if let Some(entry) = map.get_mut(&instance.instance_id) {
                    entry.last_rendered_elapsed = Some(None);
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
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, CaptureMode::Screen)
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
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, CaptureMode::Screen)
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
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, CaptureMode::Window)
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
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core
            .upsert_instance(instance, CaptureMode::Window)
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

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let core = Arc::new(GsrCore::new());

    let watcher_core = Arc::clone(&core);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(500));
        loop {
            ticker.tick().await;
            watcher_core.sync_all(false).await;
        }
    });

    register_action(RecordScreenAction {
        core: Arc::clone(&core),
    })
    .await;
    register_action(RecordWindowAction { core }).await;

    run(std::env::args().collect()).await
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
    fn settings_deserialize_with_default_empty_path() {
        let parsed: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.gsr_ui_cli_path, "");
    }
}
