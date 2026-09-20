// specs/005_antigravity_usage.md
pub mod discovery;
pub mod metrics;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use discovery::{ConnectionInfo, discover_connection};
use metrics::{DisplayFractionAs, Metric, UserStatusResponse, fetch, render_image, render_status};
use openaction::global_events::{
    DidReceiveGlobalSettingsEvent, GlobalEventHandler, set_global_event_handler,
};
use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_global_settings, get_instance,
    register_action, run,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

pub const ACTION_UUID: &str = "io.github.mario.antigravityusage.usage";
pub const CACHE_TTL: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    #[default]
    Fixed,
    CycleManual,
    CyclePeriodic,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct GlobalSettings {
    pub override_port: Option<u16>,
    pub override_csrf_token: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub display_mode: DisplayMode,
    pub fixed_metric: Metric,
    pub cycle_interval_seconds: u64,
    pub display_fraction_as: DisplayFractionAs,
    pub override_port: Option<u16>,
    pub override_csrf_token: String,
}

impl Settings {
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.cycle_interval_seconds.max(60))
    }

    pub fn resolve_connection(&self, global: &GlobalSettings) -> Option<ConnectionInfo> {
        let port = self.override_port.or(global.override_port);
        let token = if !self.override_csrf_token.trim().is_empty() {
            self.override_csrf_token.trim()
        } else {
            global.override_csrf_token.trim()
        };

        if let Some(p) = port
            && !token.is_empty()
        {
            return Some(ConnectionInfo {
                port: p,
                csrf_token: token.to_string(),
            });
        }
        None
    }
}

pub fn next_metric(index: &mut usize) -> Metric {
    let metric = Metric::ALL[*index % Metric::ALL.len()];
    *index = (*index + 1) % Metric::ALL.len();
    metric
}

#[derive(Clone)]
struct CacheEntry {
    conn: ConnectionInfo,
    response: UserStatusResponse,
    fetched_at: Instant,
}

#[derive(Default)]
struct InstanceState {
    settings: Settings,
    next_metric: usize,
    current_metric: Metric,
    generation: u64,
}

pub struct Core {
    global: RwLock<GlobalSettings>,
    instances: Mutex<HashMap<String, InstanceState>>,
    cache: Mutex<Option<CacheEntry>>,
    discovered_conn: Mutex<Option<ConnectionInfo>>,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    pub fn new() -> Self {
        Self {
            global: RwLock::new(GlobalSettings::default()),
            instances: Mutex::default(),
            cache: Mutex::default(),
            discovered_conn: Mutex::default(),
        }
    }

    pub async fn begin(
        &self,
        instance_id: &str,
        settings: Settings,
        initial_metric: Metric,
    ) -> u64 {
        let mut instances = self.instances.lock().await;
        let state = instances.entry(instance_id.to_owned()).or_default();
        state.settings = settings;
        state.next_metric = 0;
        state.current_metric = initial_metric;
        state.generation += 1;
        state.generation
    }

    pub async fn next_metric(&self, instance_id: &str) -> Metric {
        let mut instances = self.instances.lock().await;
        let Some(state) = instances.get_mut(instance_id) else {
            return Metric::GeminiQuota;
        };
        let metric = next_metric(&mut state.next_metric);
        state.current_metric = metric;
        metric
    }

    pub async fn get_instance_info(&self, instance_id: &str) -> Option<(Settings, u64)> {
        self.instances
            .lock()
            .await
            .get(instance_id)
            .map(|state| (state.settings.clone(), state.generation))
    }

    pub async fn is_current(&self, instance_id: &str, generation: u64) -> bool {
        self.instances
            .lock()
            .await
            .get(instance_id)
            .is_some_and(|state| state.generation == generation)
    }

    async fn get_connection(&self, settings: &Settings) -> Result<ConnectionInfo, &'static str> {
        let global = self.global.read().await;
        if let Some(conn) = settings.resolve_connection(&global) {
            return Ok(conn);
        }

        let mut disc = self.discovered_conn.lock().await;
        if let Some(conn) = &*disc
            && discovery::verify_port(conn.port, &conn.csrf_token).await
        {
            return Ok(conn.clone());
        }

        // Run auto-discovery
        if let Some(conn) = discover_connection().await {
            *disc = Some(conn.clone());
            Ok(conn)
        } else {
            *disc = None;
            Err("Antigravity offline")
        }
    }

    pub async fn fetch_telemetry(
        &self,
        settings: &Settings,
        force_refresh: bool,
    ) -> Result<UserStatusResponse, String> {
        let conn = self
            .get_connection(settings)
            .await
            .map_err(|e| e.to_string())?;

        if !force_refresh {
            let cache = self.cache.lock().await;
            if let Some(entry) = &*cache
                && entry.conn == conn
                && entry.fetched_at.elapsed() < CACHE_TTL
            {
                return Ok(entry.response.clone());
            }
        }

        match fetch(conn.port, &conn.csrf_token).await {
            Ok(resp) => {
                let mut cache = self.cache.lock().await;
                *cache = Some(CacheEntry {
                    conn,
                    response: resp.clone(),
                    fetched_at: Instant::now(),
                });
                Ok(resp)
            }
            Err(e) => {
                // Clear cached connection on failure
                *self.discovered_conn.lock().await = None;
                Err(format!("Fetch failed: {e}"))
            }
        }
    }

    pub async fn refresh(
        &self,
        instance: &Instance,
        settings: &Settings,
        metric: Metric,
        force_refresh: bool,
    ) -> OpenActionResult<()> {
        match self.fetch_telemetry(settings, force_refresh).await {
            Ok(status) => {
                let display = status.display(metric, settings.display_fraction_as);
                let img = render_image(&display);
                instance.set_image(Some(img), None).await
            }
            Err(err) => {
                let (title, sub) = if err.contains("offline") {
                    ("Offline", "Start AGY")
                } else {
                    ("Error", "Check AGY")
                };
                instance
                    .set_image(Some(render_status(title, sub)), None)
                    .await?;
                instance.show_alert().await
            }
        }
    }

    pub async fn refresh_all(&self) {
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

pub struct UsageAction {
    core: Arc<Core>,
}

impl UsageAction {
    pub fn new(core: Arc<Core>) -> Self {
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
            Metric::GeminiQuota
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
            Metric::GeminiQuota
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

pub struct GlobalHandler {
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
    fn cycle_wraps_around_all_metrics() {
        let mut idx = 0;
        assert_eq!(next_metric(&mut idx), Metric::GeminiQuota);
        assert_eq!(next_metric(&mut idx), Metric::ClaudeQuota);
        assert_eq!(next_metric(&mut idx), Metric::PromptCredits);
        assert_eq!(next_metric(&mut idx), Metric::FlowCredits);
        assert_eq!(next_metric(&mut idx), Metric::GeminiQuota);
    }

    #[test]
    fn settings_connection_hierarchy() {
        let global = GlobalSettings {
            override_port: Some(41719),
            override_csrf_token: "global-csrf".to_string(),
        };

        // When no instance overrides, falls back to global
        let default_settings = Settings::default();
        let conn = default_settings.resolve_connection(&global).unwrap();
        assert_eq!(conn.port, 41719);
        assert_eq!(conn.csrf_token, "global-csrf");

        // When instance overrides, uses instance
        let custom_settings = Settings {
            override_port: Some(50000),
            override_csrf_token: "instance-csrf".to_string(),
            ..Settings::default()
        };
        let custom_conn = custom_settings.resolve_connection(&global).unwrap();
        assert_eq!(custom_conn.port, 50000);
        assert_eq!(custom_conn.csrf_token, "instance-csrf");

        // When neither has port/token, returns None (falls back to auto-discovery)
        let empty_global = GlobalSettings::default();
        assert!(default_settings.resolve_connection(&empty_global).is_none());
    }

    #[tokio::test]
    async fn core_begin_and_cycle_advancement() {
        let core = Core::new();
        let settings = Settings {
            display_mode: DisplayMode::CycleManual,
            ..Settings::default()
        };
        let generation = core.begin("btn-1", settings, Metric::GeminiQuota).await;
        assert_eq!(generation, 1);
        assert!(core.is_current("btn-1", 1).await);
        assert!(!core.is_current("btn-1", 2).await);

        assert_eq!(core.next_metric("btn-1").await, Metric::GeminiQuota);
        assert_eq!(core.next_metric("btn-1").await, Metric::ClaudeQuota);
        assert_eq!(core.next_metric("btn-1").await, Metric::PromptCredits);
        assert_eq!(core.next_metric("btn-1").await, Metric::FlowCredits);
        assert_eq!(core.next_metric("btn-1").await, Metric::GeminiQuota);
    }

    #[tokio::test]
    async fn cache_hit_avoids_refetch() {
        let core = Core::new();
        let conn = ConnectionInfo {
            port: 12345,
            csrf_token: "token".to_string(),
        };
        let sample = UserStatusResponse {
            user_status: metrics::UserStatus {
                plan_status: None,
                cascade_model_config_data: None,
            },
        };

        *core.cache.lock().await = Some(CacheEntry {
            conn: conn.clone(),
            response: sample.clone(),
            fetched_at: Instant::now(),
        });

        let settings = Settings {
            override_port: Some(12345),
            override_csrf_token: "token".to_string(),
            ..Settings::default()
        };

        let fetched = core.fetch_telemetry(&settings, false).await.unwrap();
        assert_eq!(fetched, sample);
    }
}
