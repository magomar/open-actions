//! State management and 4-state project classification for Fleet Monitor.
//!
//! Implements the core models and classification rules defined in
//! `specs/004_fleet_monitor.md`.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Individual registered project metadata from Fleet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredProject {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "registeredAt", default)]
    pub registered_at: Option<String>,
    #[serde(rename = "lastAccessedAt", default)]
    pub last_accessed_at: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Beads summary metrics for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BeadsSummary {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub open: usize,
    #[serde(rename = "inProgress", default)]
    pub in_progress: usize,
    #[serde(default)]
    pub closed: usize,
    #[serde(default)]
    pub ready: usize,
    #[serde(default)]
    pub blocked: usize,
    #[serde(rename = "primaryState", default)]
    pub primary_state: Option<String>,
}

/// Keel health diagnostics summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HealthSummary {
    #[serde(default)]
    pub status: String, // "pass" | "warn" | "fail" | "unknown"
    #[serde(rename = "passCount", default)]
    pub pass_count: usize,
    #[serde(rename = "warnCount", default)]
    pub warn_count: usize,
    #[serde(rename = "failCount", default)]
    pub fail_count: usize,
}

/// Git status summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GitSummary {
    #[serde(default)]
    pub branch: String,
    #[serde(rename = "isClean", default)]
    pub is_clean: bool,
    #[serde(rename = "uncommittedCount", default)]
    pub uncommitted_count: usize,
    #[serde(rename = "lastCommitMessage", default)]
    pub last_commit_message: Option<String>,
    #[serde(rename = "lastCommitDate", default)]
    pub last_commit_date: Option<String>,
}

/// Comprehensive evaluated state of a project from Fleet's `/api/projects`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummaryState {
    pub project: RegisteredProject,
    #[serde(default = "default_true")]
    pub exists: bool,
    #[serde(rename = "isKeel", default = "default_true")]
    pub is_keel: bool,
    #[serde(default)]
    pub beads: BeadsSummary,
    #[serde(default)]
    pub health: HealthSummary,
    #[serde(default)]
    pub git: GitSummary,
}

fn default_true() -> bool {
    true
}

/// The 4 distinct project health/activity states specified in Spec 004.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectState {
    /// No open issues and none in progress (`open == 0 && in_progress == 0`).
    Clean,
    /// Open issues exist, but none blocked and none in progress.
    Ready,
    /// Active work underway (`in_progress > 0` and not blocked).
    InProgress,
    /// Tasks blocked by dependencies or failing diagnostics.
    Blocked,
}

impl ProjectState {
    /// Evaluates the 4-state classification for a given project summary.
    pub fn classify(summary: &ProjectSummaryState) -> Self {
        if !summary.exists || summary.health.status == "fail" || summary.beads.blocked > 0 {
            ProjectState::Blocked
        } else if summary.beads.in_progress > 0 {
            ProjectState::InProgress
        } else if summary.beads.open > 0 {
            ProjectState::Ready
        } else {
            ProjectState::Clean
        }
    }

    /// Returns the primary color hex associated with this state.
    pub fn color_hex(&self) -> &'static str {
        match self {
            ProjectState::Clean => "#10b981",      // Emerald green
            ProjectState::Ready => "#38bdf8",      // Fleet Sky Blue
            ProjectState::InProgress => "#f59e0b", // Amber / Warm yellow
            ProjectState::Blocked => "#ef4444",    // Crimson red
        }
    }

    /// Human-readable label for UI titles.
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            ProjectState::Clean => "Clean",
            ProjectState::Ready => "Ready",
            ProjectState::InProgress => "In Progress",
            ProjectState::Blocked => "Blocked",
        }
    }

    /// Single-character badge symbol for tight layouts.
    #[allow(dead_code)]
    pub fn symbol(&self) -> &'static str {
        match self {
            ProjectState::Clean => "✓",
            ProjectState::Ready => "●",
            ProjectState::InProgress => "⚡",
            ProjectState::Blocked => "⛔",
        }
    }
}

/// Tally of projects across all 4 states in the fleet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetStateCounts {
    pub clean: usize,
    pub ready: usize,
    pub in_progress: usize,
    pub blocked: usize,
}

impl FleetStateCounts {
    pub fn from_projects(projects: &[ProjectSummaryState]) -> Self {
        let mut counts = Self::default();
        for p in projects {
            match ProjectState::classify(p) {
                ProjectState::Clean => counts.clean += 1,
                ProjectState::Ready => counts.ready += 1,
                ProjectState::InProgress => counts.in_progress += 1,
                ProjectState::Blocked => counts.blocked += 1,
            }
        }
        counts
    }

    pub fn total(&self) -> usize {
        self.clean + self.ready + self.in_progress + self.blocked
    }
}

/// Thread-safe shared state holding the live fleet registry.
#[derive(Debug, Default)]
pub struct SharedFleetState {
    pub is_running: bool,
    pub projects: Vec<ProjectSummaryState>,
    pub last_updated: Option<Instant>,
}

impl SharedFleetState {
    /// Finds a specific project by its slug ID or path.
    pub fn find_project(&self, id_or_path: &str) -> Option<&ProjectSummaryState> {
        self.projects
            .iter()
            .find(|p| p.project.id == id_or_path || p.project.path == id_or_path)
    }

    /// Computes the aggregate 4-state counts across all registered projects.
    pub fn state_counts(&self) -> FleetStateCounts {
        FleetStateCounts::from_projects(&self.projects)
    }

    /// Updates the registered project list.
    pub fn update_projects(&mut self, new_projects: Vec<ProjectSummaryState>) {
        self.projects = new_projects;
        self.last_updated = Some(Instant::now());
    }
}

fn default_refresh_interval_secs() -> u64 {
    10
}

/// Persisted settings for the Fleet Global action.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GlobalActionSettings {
    #[serde(rename = "apiUrl", default = "default_api_url")]
    pub api_url: String,
    #[serde(
        rename = "refreshIntervalSecs",
        default = "default_refresh_interval_secs"
    )]
    pub refresh_interval_secs: u64,
}

impl Default for GlobalActionSettings {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            refresh_interval_secs: default_refresh_interval_secs(),
        }
    }
}

impl<'de> Deserialize<'de> for GlobalActionSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde_json::Value;
        let v = Value::deserialize(deserializer)?;
        let api_url = v
            .get("apiUrl")
            .or_else(|| v.get("api_url"))
            .and_then(Value::as_str)
            .unwrap_or("http://127.0.0.1:3000")
            .to_string();

        let refresh_interval_secs = v
            .get("refreshIntervalSecs")
            .or_else(|| v.get("refresh_interval_secs"))
            .or_else(|| v.get("refreshInterval"))
            .and_then(|val| {
                if let Some(n) = val.as_u64() {
                    Some(n)
                } else if let Some(s) = val.as_str() {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(default_refresh_interval_secs)
            .max(1);

        Ok(Self {
            api_url,
            refresh_interval_secs,
        })
    }
}

/// Display mode for the Fleet Project action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectDisplayMode {
    /// Overview mode showing project name, icon, and status badge pill.
    #[default]
    Status,
    /// Metrics mode showing 4-corner breakdown of bead issues (closed, open, in_progress, blocked).
    Issues,
}

impl ProjectDisplayMode {
    /// Toggles between Status overview and Issues breakdown modes.
    pub fn toggle(&self) -> Self {
        match self {
            Self::Status => Self::Issues,
            Self::Issues => Self::Status,
        }
    }
}

/// Persisted settings for the Fleet Project action (fixed project mode).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectActionSettings {
    #[serde(rename = "apiUrl", default = "default_api_url")]
    pub api_url: String,
    #[serde(rename = "projectId", default)]
    pub project_id: Option<String>,
    #[serde(rename = "displayMode", default)]
    pub display_mode: ProjectDisplayMode,
    #[serde(rename = "metricsTimeoutSecs", skip_serializing_if = "Option::is_none")]
    pub metrics_timeout_secs: Option<u64>,
}

impl Default for ProjectActionSettings {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            project_id: None,
            display_mode: ProjectDisplayMode::default(),
            metrics_timeout_secs: None,
        }
    }
}

impl<'de> Deserialize<'de> for ProjectActionSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde_json::Value;
        let v = Value::deserialize(deserializer)?;
        let api_url = v
            .get("apiUrl")
            .or_else(|| v.get("api_url"))
            .and_then(Value::as_str)
            .unwrap_or("http://127.0.0.1:3000")
            .to_string();

        let project_id = v
            .get("projectId")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| {
                v.get("fixedProjectId")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                v.get("project_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
            })
            .map(|s| s.to_string());

        let display_mode = v
            .get("displayMode")
            .or_else(|| v.get("display_mode"))
            .or_else(|| v.get("mode"))
            .and_then(Value::as_str)
            .map(|s| match s {
                "issues" | "beads" => ProjectDisplayMode::Issues,
                _ => ProjectDisplayMode::Status,
            })
            .unwrap_or_default();

        let metrics_timeout_secs = v
            .get("metricsTimeoutSecs")
            .or_else(|| v.get("metrics_timeout_secs"))
            .or_else(|| v.get("autoRevertSecs"))
            .or_else(|| v.get("auto_revert_secs"))
            .and_then(|val| {
                if let Some(n) = val.as_u64() {
                    Some(n)
                } else if let Some(s) = val.as_str() {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .filter(|&secs| secs > 0);

        Ok(Self {
            api_url,
            project_id,
            display_mode,
            metrics_timeout_secs,
        })
    }
}

fn default_api_url() -> String {
    "http://127.0.0.1:3000".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mock_project(
        id: &str,
        open: usize,
        in_progress: usize,
        blocked: usize,
    ) -> ProjectSummaryState {
        ProjectSummaryState {
            project: RegisteredProject {
                id: id.to_string(),
                name: id.to_string(),
                path: format!("/workspace/{}", id),
                registered_at: None,
                last_accessed_at: None,
                tags: vec![],
                icon: None,
            },
            exists: true,
            is_keel: true,
            beads: BeadsSummary {
                total: open + 10,
                open,
                in_progress,
                closed: 10,
                ready: open.saturating_sub(blocked),
                blocked,
                primary_state: None,
            },
            health: HealthSummary {
                status: "pass".to_string(),
                pass_count: 4,
                warn_count: 0,
                fail_count: 0,
            },
            git: GitSummary::default(),
        }
    }

    #[test]
    fn test_classify_clean_project() {
        let p = make_mock_project("keel", 0, 0, 0);
        assert_eq!(ProjectState::classify(&p), ProjectState::Clean);
        assert_eq!(ProjectState::classify(&p).color_hex(), "#10b981");
    }

    #[test]
    fn test_classify_ready_project() {
        let p = make_mock_project("tdrace", 5, 0, 0);
        assert_eq!(ProjectState::classify(&p), ProjectState::Ready);
        assert_eq!(ProjectState::classify(&p).color_hex(), "#38bdf8");
    }

    #[test]
    fn test_classify_in_progress_project() {
        let p = make_mock_project("quant-trade", 5, 2, 0);
        assert_eq!(ProjectState::classify(&p), ProjectState::InProgress);
        assert_eq!(ProjectState::classify(&p).color_hex(), "#f59e0b");
    }

    #[test]
    fn test_classify_blocked_project() {
        let p = make_mock_project("fleet", 5, 1, 2);
        assert_eq!(ProjectState::classify(&p), ProjectState::Blocked);
        assert_eq!(ProjectState::classify(&p).color_hex(), "#ef4444");

        // Failing health also marks project as Blocked
        let mut healthy = make_mock_project("clean-fail", 0, 0, 0);
        healthy.health.status = "fail".to_string();
        assert_eq!(ProjectState::classify(&healthy), ProjectState::Blocked);
    }

    #[test]
    fn test_fleet_state_counts_matching_actual_workspace() {
        let projects = vec![
            make_mock_project("agora", 0, 0, 0),        // Clean
            make_mock_project("keel", 0, 0, 0),         // Clean
            make_mock_project("open-actions", 0, 0, 0), // Clean
            make_mock_project("quant-trade", 11, 0, 0), // Ready
            make_mock_project("tdrace", 6, 0, 0),       // Ready
            make_mock_project("fleet", 5, 0, 3),        // Blocked
        ];

        let counts = FleetStateCounts::from_projects(&projects);
        assert_eq!(counts.clean, 3);
        assert_eq!(counts.ready, 2);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 1);
        assert_eq!(counts.total(), 6);
    }

    #[test]
    fn test_deserialize_project_action_settings_with_both_fields() {
        let json =
            r#"{"apiUrl":"http://127.0.0.1:3000","fixedProjectId":"fleet","projectId":"fleet"}"#;
        let res: Result<ProjectActionSettings, _> = serde_json::from_str(json);
        assert!(res.is_ok(), "Deserialization failed: {:?}", res);
    }

    #[test]
    fn test_deserialize_global_action_settings() {
        // Default empty object
        let res: GlobalActionSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(res.api_url, "http://127.0.0.1:3000");
        assert_eq!(res.refresh_interval_secs, 10);

        // Numeric interval
        let res: GlobalActionSettings =
            serde_json::from_str(r#"{"refreshIntervalSecs": 15}"#).unwrap();
        assert_eq!(res.refresh_interval_secs, 15);

        // String interval from HTML input
        let res: GlobalActionSettings =
            serde_json::from_str(r#"{"refreshIntervalSecs": "20"}"#).unwrap();
        assert_eq!(res.refresh_interval_secs, 20);

        // Clamped minimum (zero is clamped to 1)
        let res: GlobalActionSettings =
            serde_json::from_str(r#"{"refreshIntervalSecs": 0}"#).unwrap();
        assert_eq!(res.refresh_interval_secs, 1);
    }

    #[test]
    fn test_project_display_mode_toggle() {
        assert_eq!(
            ProjectDisplayMode::Status.toggle(),
            ProjectDisplayMode::Issues
        );
        assert_eq!(
            ProjectDisplayMode::Issues.toggle(),
            ProjectDisplayMode::Status
        );
    }

    #[test]
    fn test_deserialize_project_action_settings_display_mode() {
        // Default
        let s: ProjectActionSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.display_mode, ProjectDisplayMode::Status);

        // Explicit status
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"displayMode": "status"}"#).unwrap();
        assert_eq!(s.display_mode, ProjectDisplayMode::Status);

        // Explicit issues
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"displayMode": "issues"}"#).unwrap();
        assert_eq!(s.display_mode, ProjectDisplayMode::Issues);

        // Legacy/alias beads
        let s: ProjectActionSettings = serde_json::from_str(r#"{"mode": "beads"}"#).unwrap();
        assert_eq!(s.display_mode, ProjectDisplayMode::Issues);
    }

    #[test]
    fn test_deserialize_project_action_settings_metrics_timeout() {
        // Default / empty -> None (stay until clicked again)
        let s: ProjectActionSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.metrics_timeout_secs, None);

        // Numeric timeout
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"metricsTimeoutSecs": 5}"#).unwrap();
        assert_eq!(s.metrics_timeout_secs, Some(5));

        // String timeout from input
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"metricsTimeoutSecs": "10"}"#).unwrap();
        assert_eq!(s.metrics_timeout_secs, Some(10));

        // 0 -> None (disabled)
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"metricsTimeoutSecs": 0}"#).unwrap();
        assert_eq!(s.metrics_timeout_secs, None);

        // Snake case alias
        let s: ProjectActionSettings =
            serde_json::from_str(r#"{"metrics_timeout_secs": 15}"#).unwrap();
        assert_eq!(s.metrics_timeout_secs, Some(15));

        // autoRevertSecs alias
        let s: ProjectActionSettings = serde_json::from_str(r#"{"autoRevertSecs": 20}"#).unwrap();
        assert_eq!(s.metrics_timeout_secs, Some(20));
    }
}
