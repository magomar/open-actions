mod handy;

use std::{collections::HashMap, sync::Arc};

use openaction::{Action, Instance, OpenActionResult, async_trait, register_action, run};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const ACTION_UUID: &str = "io.github.mario.handytranscribe.transcribe";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    handy_binary_path: String,
}

#[derive(Default)]
struct InstanceState {
    recording: bool,
}

struct HandyAction {
    instances: Arc<Mutex<HashMap<String, InstanceState>>>,
}

impl HandyAction {
    fn new() -> Self {
        Self {
            instances: Arc::default(),
        }
    }

    async fn render(&self, instance: &Instance, recording: bool) -> OpenActionResult<()> {
        instance.set_image(Some(handy::icon(recording)), None).await
    }

    async fn toggle(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        if handy::toggle(&settings.handy_binary_path).is_err() {
            instance.show_alert().await?;
            return self.render(instance, false).await;
        }

        let recording = {
            let mut instances = self.instances.lock().await;
            let state = instances.entry(instance.instance_id.clone()).or_default();
            state.recording = !state.recording;
            state.recording
        };
        self.render(instance, recording).await
    }
}

#[async_trait]
impl Action for HandyAction {
    const UUID: &'static str = ACTION_UUID;
    type Settings = Settings;

    async fn will_appear(&self, instance: &Instance, _settings: &Settings) -> OpenActionResult<()> {
        self.render(instance, false).await
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Settings,
    ) -> OpenActionResult<()> {
        self.instances.lock().await.remove(&instance.instance_id);
        Ok(())
    }

    async fn key_up(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        self.toggle(instance, settings).await
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    register_action(HandyAction::new()).await;
    run(std::env::args().collect()).await
}
