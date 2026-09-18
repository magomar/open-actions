mod metrics;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use metrics::{Metric, UsageResponse, fetch, image, status};
use openaction::global_events::{
    DidReceiveGlobalSettingsEvent, GlobalEventHandler, set_global_event_handler,
};
use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_global_settings, get_instance,
    register_action, run,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

const ACTION_UUID: &str = "io.github.mario.opencodeusage.usage";
const CACHE_TTL: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DisplayMode {
    #[default]
    Fixed,
    CycleManual,
    CyclePeriodic,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct GlobalSettings {
    pub api_key: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
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

    fn resolve_api_key<'a>(&'a self, global: &'a GlobalSettings) -> &'a str {
        let instance_key = self.api_key.trim();
        if !instance_key.is_empty() {
            instance_key
        } else {
            global.api_key.trim()
        }
    }
}

fn next_metric(index: &mut usize) -> Metric {
    let metric = Metric::ALL[*index % Metric::ALL.len()];
    *index = (*index + 1) % Metric::ALL.len();
    metric
}

#[derive(Clone)]
struct CacheEntry {
    api_key: String,
    usage: UsageResponse,
    fetched_at: Instant,
}

#[derive(Default)]
struct InstanceState {
    settings: Settings,
    next_metric: usize,
    current_metric: Metric,
    generation: u64,
}

struct Core {
    global: RwLock<GlobalSettings>,
    instances: Mutex<HashMap<String, InstanceState>>,
    cache: Mutex<Option<CacheEntry>>,
}

impl Core {
    fn new() -> Self {
        Self {
            global: RwLock::new(GlobalSettings::default()),
            instances: Mutex::default(),
            cache: Mutex::default(),
        }
    }

    async fn begin(&self, instance_id: &str, settings: Settings, initial_metric: Metric) -> u64 {
        let mut instances = self.instances.lock().await;
        let state = instances.entry(instance_id.to_owned()).or_default();
        state.settings = settings;
        state.next_metric = 0;
        state.current_metric = initial_metric;
        state.generation += 1;
        state.generation
    }

    async fn next_metric(&self, instance_id: &str) -> Metric {
        let mut instances = self.instances.lock().await;
        let Some(state) = instances.get_mut(instance_id) else {
            return Metric::Go5h;
        };
        let metric = next_metric(&mut state.next_metric);
        state.current_metric = metric;
        metric
    }

    async fn get_instance_info(&self, instance_id: &str) -> Option<(Settings, u64)> {
        self.instances
            .lock()
            .await
            .get(instance_id)
            .map(|state| (state.settings.clone(), state.generation))
    }

    async fn is_current(&self, instance_id: &str, generation: u64) -> bool {
        self.instances
            .lock()
            .await
            .get(instance_id)
            .is_some_and(|state| state.generation == generation)
    }

    async fn fetch_usage(
        &self,
        api_key: &str,
        force_refresh: bool,
    ) -> Result<UsageResponse, reqwest::Error> {
        if !force_refresh {
            let cache = self.cache.lock().await;
            if let Some(entry) = &*cache
                && entry.api_key == api_key
                && entry.fetched_at.elapsed() < CACHE_TTL
            {
                return Ok(entry.usage.clone());
            }
        }

        let usage = fetch(api_key).await?;
        let mut cache = self.cache.lock().await;
        *cache = Some(CacheEntry {
            api_key: api_key.to_owned(),
            usage: usage.clone(),
            fetched_at: Instant::now(),
        });
        Ok(usage)
    }

    async fn refresh(
        &self,
        instance: &Instance,
        settings: &Settings,
        metric: Metric,
        force_refresh: bool,
    ) -> OpenActionResult<()> {
        let global = self.global.read().await;
        let api_key = settings.resolve_api_key(&global);

        if api_key.is_empty() {
            return instance
                .set_image(Some(status("Configure key")), None)
                .await;
        }

        match self.fetch_usage(api_key, force_refresh).await {
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

    async fn refresh_all(&self) {
        let targets: Vec<(String, Settings, Metric)> = {
            let instances = self.instances.lock().await;
            instances
                .iter()
                .map(|(id, state)| (id.clone(), state.settings.clone(), state.current_metric))
                .collect()
        };

        for (id, settings, metric) in targets {
            if let Some(instance) = get_instance(id).await {
                let _ = self.refresh(&instance, &settings, metric, false).await;
            }
        }
    }
}

struct UsageAction {
    core: Arc<Core>,
}

impl UsageAction {
    fn new(core: Arc<Core>) -> Self {
        Self { core }
    }

    fn spawn_periodic(&self, instance_id: String, generation: u64, interval: Duration) {
        let core = Arc::clone(&self.core);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if !core.is_current(&instance_id, generation).await {
                    return;
                }
                let Some(instance) = get_instance(instance_id.clone()).await else {
                    return;
                };
                let Some((settings, _)) = core.get_instance_info(&instance_id).await else {
                    return;
                };
                let metric = core.next_metric(&instance_id).await;
                let _ = core.refresh(&instance, &settings, metric, false).await;
            }
        });
    }
}

#[async_trait]
impl Action for UsageAction {
    const UUID: &'static str = ACTION_UUID;
    type Settings = Settings;

    async fn will_appear(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        let initial_metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            Metric::Go5h
        };
        let generation = self
            .core
            .begin(&instance.instance_id, settings.clone(), initial_metric)
            .await;
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.core.next_metric(&instance.instance_id).await
        };
        self.core.refresh(instance, settings, metric, false).await?;
        if settings.display_mode == DisplayMode::CyclePeriodic {
            self.spawn_periodic(
                instance.instance_id.clone(),
                generation,
                settings.interval(),
            );
        }
        Ok(())
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Settings,
    ) -> OpenActionResult<()> {
        self.core
            .instances
            .lock()
            .await
            .remove(&instance.instance_id);
        Ok(())
    }

    async fn key_up(&self, instance: &Instance, settings: &Settings) -> OpenActionResult<()> {
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.core.next_metric(&instance.instance_id).await
        };
        self.core.refresh(instance, settings, metric, true).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Settings,
    ) -> OpenActionResult<()> {
        let initial_metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            Metric::Go5h
        };
        let generation = self
            .core
            .begin(&instance.instance_id, settings.clone(), initial_metric)
            .await;
        let metric = if settings.display_mode == DisplayMode::Fixed {
            settings.fixed_metric
        } else {
            self.core.next_metric(&instance.instance_id).await
        };
        self.core.refresh(instance, settings, metric, false).await?;
        if settings.display_mode == DisplayMode::CyclePeriodic {
            self.spawn_periodic(
                instance.instance_id.clone(),
                generation,
                settings.interval(),
            );
        }
        Ok(())
    }

    async fn property_inspector_did_appear(
        &self,
        _instance: &Instance,
        _settings: &Settings,
    ) -> OpenActionResult<()> {
        get_global_settings().await
    }
}

struct GlobalHandler {
    core: Arc<Core>,
}

#[async_trait]
impl GlobalEventHandler for GlobalHandler {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        get_global_settings().await
    }

    async fn did_receive_global_settings(
        &self,
        event: DidReceiveGlobalSettingsEvent,
    ) -> OpenActionResult<()> {
        if let Ok(settings) = serde_json::from_value::<GlobalSettings>(event.payload.settings) {
            *self.core.global.write().await = settings;
            self.core.refresh_all().await;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let core = Arc::new(Core::new());
    let handler: &'static GlobalHandler = Box::leak(Box::new(GlobalHandler {
        core: Arc::clone(&core),
    }));
    set_global_event_handler(handler);
    register_action(UsageAction::new(core)).await;
    run(std::env::args().collect()).await
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

    #[test]
    fn deserializes_global_settings() {
        let global: GlobalSettings = serde_json::from_str(r#"{"api_key":"test-key-123"}"#).unwrap();
        assert_eq!(global.api_key, "test-key-123");
    }

    #[test]
    fn resolves_api_key_hierarchy() {
        let global = GlobalSettings {
            api_key: "global-secret".to_owned(),
        };

        // When instance has no key, fallback to global
        let settings_no_key = Settings::default();
        assert_eq!(settings_no_key.resolve_api_key(&global), "global-secret");

        // When instance has whitespace, fallback to global
        let settings_whitespace = Settings {
            api_key: "   ".to_owned(),
            ..Settings::default()
        };
        assert_eq!(
            settings_whitespace.resolve_api_key(&global),
            "global-secret"
        );

        // When instance has an override key, use instance key
        let settings_override = Settings {
            api_key: "instance-override".to_owned(),
            ..Settings::default()
        };
        assert_eq!(
            settings_override.resolve_api_key(&global),
            "instance-override"
        );

        // When both are empty, returns empty string
        let empty_global = GlobalSettings::default();
        assert_eq!(settings_no_key.resolve_api_key(&empty_global), "");
    }

    #[tokio::test]
    async fn core_begin_and_cycle_advancement() {
        let core = Core::new();
        let settings = Settings {
            display_mode: DisplayMode::CycleManual,
            ..Settings::default()
        };
        let generation = core.begin("btn-1", settings, Metric::Go5h).await;
        assert_eq!(generation, 1);
        assert!(core.is_current("btn-1", 1).await);
        assert!(!core.is_current("btn-1", 2).await);

        assert_eq!(core.next_metric("btn-1").await, Metric::Go5h);
        assert_eq!(core.next_metric("btn-1").await, Metric::GoWeekly);
        assert_eq!(core.next_metric("btn-1").await, Metric::GoMonthly);
        assert_eq!(core.next_metric("btn-1").await, Metric::Go5h);
    }

    #[tokio::test]
    async fn cache_hit_avoids_network() {
        let core = Core::new();
        let usage = metrics::UsageResponse {
            usage: metrics::UsageWindows {
                rolling: metrics::UsageWindow { percent: 15 },
                weekly: metrics::UsageWindow { percent: 30 },
                monthly: metrics::UsageWindow { percent: 45 },
            },
        };
        *core.cache.lock().await = Some(CacheEntry {
            api_key: "cached-key".to_owned(),
            usage: usage.clone(),
            fetched_at: Instant::now(),
        });

        // Cache hit within TTL
        let fetched = core.fetch_usage("cached-key", false).await.unwrap();
        assert_eq!(fetched, usage);
    }
}
