//! The Fleet Monitor OpenAction plugin entrypoint.
//!
//! Registers Fleet Global and Fleet Project actions, coordinates shared
//! state, background polling, and device rendering as specified in
//! `specs/004_fleet_monitor.md`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use openaction::{
    Action, Instance, OpenActionResult, async_trait, get_instance, register_action, run,
};
use serde_json::{Value, json};
use tokio::sync::{Mutex, Notify, RwLock};

mod client;
mod icon;
mod state;

use client::{FleetClient, launch_fleet};
use state::{GlobalActionSettings, ProjectActionSettings, ProjectDisplayMode, SharedFleetState};

/// Core coordinator shared across all actions and instances.
pub struct Core {
    pub state: Arc<RwLock<SharedFleetState>>,
    pub client: Arc<RwLock<FleetClient>>,
    pub global_instances: Mutex<HashMap<String, GlobalActionSettings>>,
    pub project_instances: Mutex<HashMap<String, ProjectActionSettings>>,
    pub project_generations: Mutex<HashMap<String, u64>>,
    pub refresh_interval_secs: Arc<AtomicU64>,
    pub refresh_notifier: Arc<Notify>,
}

impl Core {
    pub fn new(default_url: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(SharedFleetState::default())),
            client: Arc::new(RwLock::new(FleetClient::new(default_url))),
            global_instances: Mutex::new(HashMap::new()),
            project_instances: Mutex::new(HashMap::new()),
            project_generations: Mutex::new(HashMap::new()),
            refresh_interval_secs: Arc::new(AtomicU64::new(10)),
            refresh_notifier: Arc::new(Notify::new()),
        }
    }

    /// Updates the configured Fleet API URL if changed.
    pub async fn update_api_url(&self, url: &str) {
        let mut client = self.client.write().await;
        *client = FleetClient::new(url.to_string());
    }

    /// Updates the background telemetry polling frequency.
    pub async fn update_refresh_interval(&self, secs: u64) {
        let valid_secs = secs.max(1);
        let old = self
            .refresh_interval_secs
            .swap(valid_secs, Ordering::SeqCst);
        if old != valid_secs {
            eprintln!(
                "[fleet-monitor] updated refresh interval from {}s to {}s",
                old, valid_secs
            );
            self.refresh_notifier.notify_one();
        }
    }

    /// Refreshes telemetry from Fleet and updates all active buttons on the deck.
    pub async fn refresh(&self) {
        let (is_running, new_projects) = {
            let client = self.client.read().await;
            if client.is_running().await {
                match client.fetch_projects().await {
                    Ok(projects) => (true, Some(projects)),
                    Err(_) => (true, None),
                }
            } else {
                (false, None)
            }
        };

        {
            let mut state = self.state.write().await;
            state.is_running = is_running;
            if let Some(projects) = new_projects {
                state.update_projects(projects);
            }
        }

        self.render_all().await;
    }

    /// Renders the face for every active Global and Project button instance.
    pub async fn render_all(&self) {
        let (is_running, counts) = {
            let state = self.state.read().await;
            (state.is_running, state.state_counts())
        };

        // Render Global action instances
        let globals: Vec<_> = {
            let map = self.global_instances.lock().await;
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (instance_id, _settings) in globals {
            if let Some(instance) = get_instance(instance_id.clone()).await {
                eprintln!("[fleet-monitor] RENDER GLOBAL instance={}", instance_id);
                let img = icon::global_icon(counts, is_running);
                let _ = instance.set_image(Some(img), None).await;
            }
        }

        // Render Project action instances
        let projects: Vec<_> = {
            let map = self.project_instances.lock().await;
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let state = self.state.read().await;
        for (instance_id, settings) in projects {
            if let Some(instance) = get_instance(instance_id.clone()).await {
                let target_project = settings
                    .project_id
                    .as_deref()
                    .and_then(|id| state.find_project(id));

                eprintln!(
                    "[fleet-monitor] RENDER instance={} pid={:?} found={}",
                    instance_id,
                    settings.project_id,
                    target_project.is_some()
                );

                let img = icon::project_icon(target_project, is_running, settings.display_mode);
                let _ = instance.set_image(Some(img), None).await;
            }
        }
    }

    /// Handles press on a Fleet Global key.
    pub async fn global_press(
        &self,
        instance: &Instance,
        _settings: &GlobalActionSettings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] global_press received for instance={}",
            instance.instance_id
        );
        let is_running = self.state.read().await.is_running;

        if !is_running {
            // Fleet is closed -> launch it
            eprintln!("[fleet-monitor] Fleet is not running -> triggering launch_fleet()");
            if let Err(e) = launch_fleet() {
                eprintln!("[fleet-monitor] launch failed: {e}");
                let _ = instance.show_alert().await;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            self.refresh().await;
            return Ok(());
        }

        // Fleet is running -> refresh telemetry
        eprintln!("[fleet-monitor] Fleet is running -> refreshing telemetry");
        self.refresh().await;
        Ok(())
    }

    /// Handles press on a Fleet Project key (Fixed Project).
    pub async fn project_press(
        self: &Arc<Self>,
        instance: &Instance,
        settings: &ProjectActionSettings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] project_press received for instance={}",
            instance.instance_id
        );
        let is_running = self.state.read().await.is_running;

        if !is_running {
            eprintln!("[fleet-monitor] Fleet is not running -> triggering launch_fleet()");
            if let Err(e) = launch_fleet() {
                eprintln!("[fleet-monitor] launch failed: {e}");
                let _ = instance.show_alert().await;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            self.refresh().await;
            return Ok(());
        }

        // Retrieve current settings from in-memory map or fallback to passed settings
        let current_settings = {
            let map = self.project_instances.lock().await;
            map.get(&instance.instance_id)
                .cloned()
                .unwrap_or_else(|| settings.clone())
        };

        // Toggle display mode and persist
        let mut next_settings = current_settings.clone();
        next_settings.display_mode = current_settings.display_mode.toggle();
        let _ = instance.set_settings(&next_settings).await;

        {
            let mut projects = self.project_instances.lock().await;
            projects.insert(instance.instance_id.clone(), next_settings.clone());
        }

        // Advance generation for this instance to invalidate any previous timeout
        let generation = {
            let mut gens = self.project_generations.lock().await;
            let current_gen = gens.entry(instance.instance_id.clone()).or_insert(0);
            *current_gen = current_gen.wrapping_add(1);
            *current_gen
        };

        // Switch workspace if target project is set
        if let Some(ref id) = next_settings.project_id {
            let client = self.client.read().await;
            let _ = client.switch_workspace(id).await;
        }

        self.render_all().await;

        // If toggled to Issues (metrics view) and an auto-revert timeout is configured, spawn revert timer
        if next_settings.display_mode == ProjectDisplayMode::Issues
            && let Some(timeout_secs) = next_settings.metrics_timeout_secs
            && timeout_secs > 0
        {
            let core = Arc::clone(self);
            let instance_id = instance.instance_id.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(timeout_secs)).await;

                // Check if generation still matches
                {
                    let gens = core.project_generations.lock().await;
                    if gens.get(&instance_id).copied() != Some(generation) {
                        return;
                    }
                }

                // Revert to Status overview
                let mut reverted_settings = None;
                {
                    let mut projects = core.project_instances.lock().await;
                    if let Some(proj_settings) = projects.get_mut(&instance_id)
                        && proj_settings.display_mode == ProjectDisplayMode::Issues
                    {
                        proj_settings.display_mode = ProjectDisplayMode::Status;
                        reverted_settings = Some(proj_settings.clone());
                    }
                }

                if let Some(reverted) = reverted_settings {
                    eprintln!(
                        "[fleet-monitor] auto-reverting instance={} back to Status overview after {}s",
                        instance_id, timeout_secs
                    );
                    if let Some(inst) = get_instance(instance_id).await {
                        let _ = inst.set_settings(&reverted).await;
                    }
                    core.render_all().await;
                }
            });
        }

        Ok(())
    }

    /// Handles messages from the Property Inspector (fetching projects list, status).
    pub async fn handle_pi_message(
        &self,
        instance: &Instance,
        payload: &Value,
    ) -> OpenActionResult<()> {
        let action = payload.get("action").and_then(Value::as_str).unwrap_or("");

        match action {
            "getProjects" => {
                let state = self.state.read().await;
                let projects: Vec<_> = state
                    .projects
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.project.id,
                            "name": p.project.name,
                            "path": p.project.path,
                        })
                    })
                    .collect();

                instance
                    .send_to_property_inspector(json!({
                        "event": "projectsList",
                        "projects": projects,
                        "isRunning": state.is_running,
                    }))
                    .await
            }
            "refresh" => {
                self.refresh().await;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Action Definitions
// ---------------------------------------------------------------------------

struct FleetGlobalAction {
    core: Arc<Core>,
}

#[async_trait]
impl Action for FleetGlobalAction {
    const UUID: &'static str = "io.github.mario.fleet.global";
    type Settings = GlobalActionSettings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] will_appear GLOBAL instance={} settings={:?}",
            instance.instance_id, settings
        );
        self.core.update_api_url(&settings.api_url).await;
        self.core
            .update_refresh_interval(settings.refresh_interval_secs)
            .await;
        {
            let mut globals = self.core.global_instances.lock().await;
            globals.insert(instance.instance_id.clone(), settings.clone());
        }
        self.core.refresh().await;
        Ok(())
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        let mut globals = self.core.global_instances.lock().await;
        globals.remove(&instance.instance_id);
        Ok(())
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.core.global_press(instance, settings).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        self.core.update_api_url(&settings.api_url).await;
        self.core
            .update_refresh_interval(settings.refresh_interval_secs)
            .await;
        {
            let mut globals = self.core.global_instances.lock().await;
            globals.insert(instance.instance_id.clone(), settings.clone());
        }
        self.core.render_all().await;
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        self.core.handle_pi_message(instance, payload).await
    }
}

struct FleetProjectAction {
    core: Arc<Core>,
}

#[async_trait]
impl Action for FleetProjectAction {
    const UUID: &'static str = "io.github.mario.fleet.project";
    type Settings = ProjectActionSettings;

    async fn will_appear(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] will_appear instance={} settings={:?}",
            instance.instance_id, settings
        );
        self.core.update_api_url(&settings.api_url).await;
        {
            let mut projects = self.core.project_instances.lock().await;
            projects.insert(instance.instance_id.clone(), settings.clone());
        }
        {
            let mut gens = self.core.project_generations.lock().await;
            gens.insert(instance.instance_id.clone(), 0);
        }
        self.core.refresh().await;
        Ok(())
    }

    async fn will_disappear(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] will_disappear instance={}",
            instance.instance_id
        );
        let mut projects = self.core.project_instances.lock().await;
        projects.remove(&instance.instance_id);
        let mut gens = self.core.project_generations.lock().await;
        gens.remove(&instance.instance_id);
        Ok(())
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.core.project_press(instance, settings).await
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        eprintln!(
            "[fleet-monitor] did_receive_settings instance={} settings={:?}",
            instance.instance_id, settings
        );
        self.core.update_api_url(&settings.api_url).await;
        {
            let mut projects = self.core.project_instances.lock().await;
            projects.insert(instance.instance_id.clone(), settings.clone());
        }
        // Advance generation to cancel any pending auto-revert timer
        {
            let mut gens = self.core.project_generations.lock().await;
            let current_gen = gens.entry(instance.instance_id.clone()).or_insert(0);
            *current_gen = current_gen.wrapping_add(1);
        }
        self.core.render_all().await;
        Ok(())
    }

    async fn send_to_plugin(
        &self,
        instance: &Instance,
        _settings: &Self::Settings,
        payload: &Value,
    ) -> OpenActionResult<()> {
        self.core.handle_pi_message(instance, payload).await
    }
}

// ---------------------------------------------------------------------------
// Main Entrypoint & Background Poller
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    let core = Arc::new(Core::new("http://127.0.0.1:3000".to_string()));

    // Spawn background poller to keep button metrics up-to-date
    let poller_core = Arc::clone(&core);
    tokio::spawn(async move {
        loop {
            let secs = poller_core.refresh_interval_secs.load(Ordering::SeqCst);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(secs)) => {
                    poller_core.refresh().await;
                }
                _ = poller_core.refresh_notifier.notified() => {
                    poller_core.refresh().await;
                }
            }
        }
    });

    register_action(FleetGlobalAction {
        core: Arc::clone(&core),
    })
    .await;

    register_action(FleetProjectAction {
        core: Arc::clone(&core),
    })
    .await;

    run(std::env::args().collect()).await
}
