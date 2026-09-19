//! HTTP client and process launcher for Fleet.
//!
//! Implements communication with the Fleet daemon and desktop application
//! as specified in `specs/004_fleet_monitor.md`.

use crate::state::ProjectSummaryState;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StatusResponse {
    status: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct ProjectsResponse {
    #[serde(default)]
    projects: Vec<ProjectSummaryState>,
    #[serde(rename = "activeWorkspace", default)]
    #[allow(dead_code)]
    active_workspace: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FleetClient {
    client: reqwest::Client,
    base_url: String,
}

impl FleetClient {
    /// Creates a new FleetClient with a 2000ms timeout for reliable workspace telemetry.
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(2000))
            .connect_timeout(Duration::from_millis(500))
            .build()
            .unwrap_or_default();

        Self { client, base_url }
    }

    /// Checks whether the Fleet backend is currently running and responsive.
    pub async fn is_running(&self) -> bool {
        let url = format!("{}/api/status", self.base_url.trim_end_matches('/'));
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<StatusResponse>().await {
                    data.status == "ok"
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Fetches all registered projects from Fleet's registry.
    pub async fn fetch_projects(&self) -> Result<Vec<ProjectSummaryState>, String> {
        let url = format!("{}/api/projects", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Fleet request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Fleet returned HTTP error {}", resp.status()));
        }

        let body = resp
            .json::<ProjectsResponse>()
            .await
            .map_err(|e| format!("Failed to parse projects JSON: {}", e))?;

        Ok(body.projects)
    }

    /// Sends a workspace switch request to Fleet, syncing the desktop UI.
    pub async fn switch_workspace(&self, project_id_or_path: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/projects/switch",
            self.base_url.trim_end_matches('/')
        );
        let payload = serde_json::json!({
            "projectId": project_id_or_path
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Workspace switch request failed: {}", e))?;

        Ok(resp.status().is_success())
    }
}

/// Attempts to launch Fleet application if it is currently closed.
pub fn launch_fleet() -> Result<(), String> {
    // 1. Try local desktop binary in ~/.local/bin/fleet-desktop
    if let Some(home) = std::env::var_os("HOME") {
        let local_desktop = PathBuf::from(&home).join(".local/bin/fleet-desktop");
        if local_desktop.exists() && std::process::Command::new(&local_desktop).spawn().is_ok() {
            return Ok(());
        }
        let local_cli = PathBuf::from(&home).join(".local/bin/fleet");
        if local_cli.exists()
            && std::process::Command::new(&local_cli)
                .arg("--global")
                .spawn()
                .is_ok()
        {
            return Ok(());
        }
    }

    // 2. Try binaries on PATH
    if std::process::Command::new("fleet-desktop").spawn().is_ok() {
        return Ok(());
    }

    if std::process::Command::new("fleet")
        .arg("--global")
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    Err("Could not find or launch fleet-desktop or fleet CLI".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fleet_client_live_liveness_and_fetch() {
        // Fleet is currently running in this environment at 127.0.0.1:3000
        let client = FleetClient::new("http://127.0.0.1:3000".to_string());
        if client.is_running().await {
            let projects = client.fetch_projects().await;
            assert!(projects.is_ok(), "Failed to fetch projects: {:?}", projects);
            let list = projects.unwrap();
            assert!(!list.is_empty(), "Projects list should not be empty");
        }
    }

    #[tokio::test]
    async fn test_fleet_client_offline_graceful() {
        // Connect to a closed port
        let client = FleetClient::new("http://127.0.0.1:49999".to_string());
        assert!(!client.is_running().await);
        let result = client.fetch_projects().await;
        assert!(result.is_err());
    }
}
