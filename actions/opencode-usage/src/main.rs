mod metrics;

use std::{collections::HashMap, sync::Arc, time::Duration};

use metrics::{Metric, fetch, image, status};
use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_instance, register_action, run,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const ACTION_UUID: &str = "io.github.mario.opencodeusage.usage";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum DisplayMode {
    #[default]
    Fixed,
    CycleManual,
    CyclePeriodic,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    api_key: String,
    display_mode: DisplayMode,
    fixed_metric: Metric,
    cycle_interval_seconds: u64,
}

impl Settings {
    fn interval(&self) -> Duration {
        Duration::from_secs(self.cycle_interval_seconds.max(60))
    }
}

fn next_metric(index: &mut usize) -> Metric {
    let metric = Metric::ALL[*index % Metric::ALL.len()];
    *index = (*index + 1) % Metric::ALL.len();
    metric
}

#[derive(Default)]
struct InstanceState {
    next_metric: usize,
    generation: u64,
}

struct UsageAction {
    instances: Arc<Mutex<HashMap<String, InstanceState>>>,
}

impl UsageAction {
    fn new() -> Self {
        Self {
            instances: Arc::default(),
        }
    }

    async fn begin(&self, instance_id: &str) -> u64 {
        let mut instances = self.instances.lock().await;
        let state = instances.entry(instance_id.to_owned()).or_default();
        state.next_metric = 0;
        state.generation += 1;
        state.generation
    }

    async fn next_metric(&self, instance_id: &str) -> Metric {
        let mut instances = self.instances.lock().await;
        let state = instances.entry(instance_id.to_owned()).or_default();
        next_metric(&mut state.next_metric)
    }

    async fn is_current(&self, instance_id: &str, generation: u64) -> bool {
        self.instances
            .lock()
            .await
            .get(instance_id)
            .is_some_and(|state| state.generation == generation)
    }

    async fn refresh(
        &self,
        instance: &Instance,
        settings: &Settings,
        metric: Metric,
    ) -> OpenActionResult<()> {
        if settings.api_key.trim().is_empty() {
            return instance
                .set_image(Some(status("Configure key")), None)
                .await;
        }

        match fetch(settings.api_key.trim()).await {
            Ok(usage) => {
                instance
                    .set_image(Some(image(metric, usage.percent(metric))), None)
                    .await
            }
            Err(_) => {
                instance.set_image(Some(status("Error")), None).await?;
                instance.show_alert().await
            }
        }
    }

    fn spawn_periodic(&self, instance_id: String, settings: Settings, generation: u64) {
        if settings.display_mode != DisplayMode::CyclePeriodic {
            return;
        }

        let action = Self {
            instances: self.instances.clone(),
        };
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(settings.interval()).await;
                if !action.is_current(&instance_id, generation).await {
                    return;
                }
                let Some(instance) = get_instance(instance_id.clone()).await else {
                    return;
                };
                let metric = action.next_metric(&instance_id).await;
                let _ = action.refresh(&instance, &settings, metric).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_after_monthly_usage() {
        let mut index = 0;
        assert_eq!(next_metric(&mut index), Metric::Go5h);
        assert_eq!(next_metric(&mut index), Metric::GoWeekly);
        assert_eq!(next_metric(&mut index), Metric::GoMonthly);
        assert_eq!(next_metric(&mut index), Metric::Go5h);
    }
}

#[async_trait]
impl Action for UsageAction {
    const UUID: &'static str = ACTION_UUID;
    type Settings = Settings;

    async fn will_appear(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        let generation = self.begin(&instance.instance_id).await;
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.next_metric(&instance.instance_id).await
        };
        self.refresh(instance, settings, metric).await?;
        self.spawn_periodic(instance.instance_id.clone(), settings.clone(), generation);
        Ok(())
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
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.next_metric(&instance.instance_id).await
        };
        self.refresh(instance, settings, metric).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        let generation = self.begin(&instance.instance_id).await;
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.next_metric(&instance.instance_id).await
        };
        self.refresh(instance, settings, metric).await?;
        self.spawn_periodic(instance.instance_id.clone(), settings.clone(), generation);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    register_action(UsageAction::new()).await;
    run(std::env::args().collect()).await
}
