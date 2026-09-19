---
type: Feature Spec
template: feature
title: "Fleet Monitor"
description: "A cooperative OpenAction plugin for OpenDeck that monitors Fleet repositories, tracks 4-state project health, and provides linked cycling and independent project views."
status: implemented
created: 2026-09-18
---

# Feature Spec: Fleet Monitor 🧭

An OpenAction plugin for OpenDeck devices (Stream Deck, Tacto, and compatible hardware) that integrates directly with **Fleet** (`~/workspace/agentic-dev/fleet/`), the multi-workspace dashboard and Beads task graph visualizer for Keel Spec-Driven Development repositories.

The plugin provides at-a-glance visibility into the health and activity of an entire project fleet from hardware keys. It uses the official Fleet app icon (ship helm squircle), detects whether Fleet is running and launches it if closed, classifies projects into **4 distinct states**, and enables cooperative multi-button workflows through shared plugin state.

---

## 🗺️ User Flow & Interface Design

### 1. Action Types

The plugin provides two distinct action definitions:

| Action | ID | Role | Key Press Behavior |
| :--- | :--- | :--- | :--- |
| **Fleet Global** | `io.github.mario.fleet.global` | Displays aggregate fleet-wide health metrics across the 4 states in the 4 corners. Pure global data without tracking any project. | **If closed**: Launches Fleet.<br>**If open**: Refreshes telemetry from Fleet. |
| **Fleet Project** | `io.github.mario.fleet.project` | Dedicated monitor for a single, fixed Keel project. Displays either project status overview or bead issues breakdown. | **If closed**: Launches Fleet.<br>**If open**: Switches Fleet workspace to this project (`POST /api/projects/switch`) and toggles display mode between Status Overview and Bead Issues Breakdown. |

#### Workflow
- **Fleet Global (`io.github.mario.fleet.global`)**: Shows high-level project counts across all 4 states at a glance. It does not select or cycle individual projects. Pressing it triggers an immediate telemetry poll or launches Fleet if offline.
- **Fleet Project (`io.github.mario.fleet.project`)**: Configured to monitor a specific fixed project chosen in the Property Inspector. Supports two display modes: **Status Overview** and **Bead Issues Breakdown**. Pressing it switches Fleet's active workspace to that project and toggles between the two display modes.

---

### 2. The 4-State Project Model

Every monitored project in Fleet is categorized into one of four mutually exclusive states based on its underlying Beads (`br`/`bvr`) tasks and diagnostic health:

| State | Beads Condition | Visual Clue / Color | Meaning & Semantics |
| :--- | :--- | :--- | :--- |
| 🟢 **Clean / Done** | `open == 0 && in_progress == 0` | **Emerald Green** (`#10b981`) | **Complete / Idle**: All tasks closed, zero backlog in flight, repository clean. |
| 🔵 **Ready** | `open > 0 && in_progress == 0 && blocked == 0` | **Sky Blue** (`#38bdf8`) | **Available Backlog**: Unblocked tasks waiting for agents or developers (`br ready`). Uses Fleet brand accent. |
| 🟡 **In Progress** | `in_progress > 0 && blocked == 0` | **Pulsing Amber** (`#f59e0b`) | **Active Work**: Agents or developers are actively executing tasks. Pipeline in motion. |
| 🔴 **Blocked** | `blocked > 0 || health.status == "fail"` | **Crimson Red** (`#ef4444`) | **Needs Attention**: Tasks obstructed by dependencies, deadlocks, or failing diagnostics. Human triage required. |

---

### 3. Button Face & Graphic Rendering

Each button generates an SVG image rendered dynamically in Rust and pushed as a base64 data-URI to OpenAction.

#### Base Icon
- The official Fleet ship helm / steering wheel on a dark slate squircle (`#0f172a` to `#020617`), matching `fleet/src-tauri/icons/icon.svg`.

#### Offline / Closed State
- When the Fleet backend is unreachable (`GET /api/status` fails):
  - Helm icon is rendered in muted monochrome (40% opacity).
  - A subtle power/launch pip is displayed at the center hub.
  - Pressing the key executes `fleet-desktop` (or configured launcher) to start Fleet.

#### Fleet Global Button
- **Center**: Centered Fleet ship helm with multi-color rim glow reflecting overall fleet health (red if any project blocked, amber if any in progress, blue if ready, green if all clean).
- **Four Corners Display**: Pure fleet-wide health metrics with zero project-specific info, giving maximal breathing room for data and visual clarity:
  - **Top-Left**: 🟢 Clean project count (`#10b981` indicator + bold count)
  - **Top-Right**: 🔵 Ready project count (`#38bdf8` indicator + bold count)
  - **Bottom-Left**: 🟡 In Progress project count (`#f59e0b` indicator + bold count)
  - **Bottom-Right**: 🔴 Blocked project count (`#ef4444` indicator + bold count)
- **Offline / Closed**: Dimmed helm with `FLEET` title and `OFFLINE · TAP` prompt.

#### Fleet Project Button
Supports two toggleable display modes:

1. **Status Overview Mode (`status`)**:
   - **Top**: Project Name (e.g., `tdrace` or `quant-trade`).
   - **Center**:
     - **Custom Project Icon Detection**: If the project contains an icon asset (`assets/icons/icon.svg`, `icon.svg`, `icon.png`, `src-tauri/icons/icon.svg`, or `project.icon` from API), the custom icon is rendered directly.
     - **Fallback**: If no custom icon exists, the Fleet ship helm is rendered illuminated in the active state color (🟢 Green, 🔵 Blue, 🟡 Amber, or 🔴 Red).
   - **Bottom Status Display**: A clean status pill using the state color and a small, legible vector icon (eliminating tiny, unreadable text labels):
     - 🟢 **Clean**: Emerald Green pill with checkmark `✓` (`#10b981`)
     - 🔵 **Ready**: Sky Blue pill with dot `●` (`#38bdf8`)
     - 🟡 **In Progress**: Amber pill with lightning bolt `⚡` (`#f59e0b`)
     - 🔴 **Blocked**: Crimson Red pill with exclamation alert `!` (`#ef4444`)

2. **Bead Issues Breakdown Mode (`issues`)**:
   - Uses the identical 4-corner layout as Fleet Global to display the project's bead tasks:
     - **Top-Left**: 🟢 Closed bead issues count (`#34d399`)
     - **Top-Right**: 🔵 Open bead issues count (`#38bdf8`)
     - **Bottom-Left**: 🟡 In Progress bead issues count (`#fbbf24`)
     - **Bottom-Right**: 🔴 Blocked bead issues count (`#f87171`)
   - **Center**: Scaled custom project icon (or state-illuminated ship helm).
   - **Border Glow**: State color glow matching the project's primary health state.

---

### 4. Property Inspector (Svelte)

- **Connection Indicator**: Live status pill showing Fleet connectivity (`● Online (N)` / `○ Offline`).
- **Fleet URL**: Configurable host/port (defaults to `http://127.0.0.1:3000`).
- **For `Fleet Global`**:
  - **Refresh Frequency**: Configurable polling interval in seconds (`refreshIntervalSecs`, defaults to 10s, minimum 1s).
  - Explanatory hint describing the 4-corner state layout.
  - Refresh Telemetry button.
- **For `Fleet Project`**:
  - **Project Selector**: Dropdown listing all registered projects fetched dynamically from Fleet (`/api/projects`).
  - **Display Mode**: Select dropdown between "Project Status (Overview)" and "Bead Issues (4-Corner Breakdown)".
  - Refresh Telemetry button.

---

## ⚙️ Backend Models & API Endpoints

The plugin communicates with the local Fleet daemon over HTTP and Server-Sent Events (SSE). No external cloud endpoints are contacted.

### 1. Fleet Endpoints Used

| Purpose | Method & Path | Description |
| :--- | :--- | :--- |
| **Liveness Check** | `GET /api/status` | Returns `{"status":"ok","name":"fleet",...}`. Used to detect if Fleet is open. |
| **Projects Telemetry** | `GET /api/projects` | Returns list of all registered projects with `beads`, `specs`, `health`, and `git` metrics. |
| **Switch Workspace** | `POST /api/projects/switch` | Body: `{"projectId":"<id>"}`. Changes Fleet's active workspace and notifies file watchers. |
| **Instant Live Events** | `GET /api/events` | SSE stream broadcasting real-time filesystem updates (debounced at 150ms). |

### 2. State Classification Logic (Rust)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectState {
    Clean,      // open == 0 && in_progress == 0
    Ready,      // open > 0 && in_progress == 0 && blocked == 0
    InProgress, // in_progress > 0 && blocked == 0
    Blocked,    // blocked > 0 || health_fail
}

pub fn classify_project(summary: &ProjectSummaryState) -> ProjectState {
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
```

### 3. Shared In-Memory Store

```rust
pub struct SharedFleetState {
    pub is_running: bool,
    pub projects: Vec<ProjectSummaryState>,
    pub last_updated: Option<Instant>,
}
```
All action instances share an `Arc<RwLock<SharedFleetState>>`. When telemetry refreshes, all active contexts are notified to re-render.

### 4. Persisted Settings Schema

#### Global Action (`io.github.mario.fleet.global`):
```json
{
  "api_url": "http://127.0.0.1:3000"
}
```

#### Project Action (`io.github.mario.fleet.project`):
```json
{
  "api_url": "http://127.0.0.1:3000",
  "project_id": "tdrace",
  "display_mode": "status"
}
```

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- **Loopback Only**: All network traffic is strictly confined to `127.0.0.1`.
- **Safe Process Spawning**: Fleet launching uses direct binary execution (`fleet-desktop` from `~/.local/bin/fleet-desktop` or `PATH`) without passing user-controlled strings to a raw shell.
- **Graceful Degradation**: If Fleet is closed, network timeouts are handled gracefully so the button remains immediately responsive and prompts to launch the app.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- `cargo test --manifest-path actions/fleet-monitor/Cargo.toml`:
  - Verify 4-state classification logic with mock beads summaries.
  - Verify SVG rendering for all 4 states, quad metrics corners, and offline state.
  - Verify display mode toggling and serialization/deserialization.
  - Verify bead issues 4-corner breakdown rendering.
- `cargo clippy --manifest-path actions/fleet-monitor/Cargo.toml -- -D warnings`
- `cargo fmt --check --manifest-path actions/fleet-monitor/Cargo.toml`
- `cd actions/fleet-monitor/pi && bun run build`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Fleet Global displays 4-state breakdown in corners**
  - [x] **Given** Fleet is running with registered projects
  - [x] **When** the Fleet Global action is loaded on a button
  - [x] **Then** the button displays the aggregate 4-state metrics in the four corners (🟢 Top-Left Clean, 🔵 Top-Right Ready, 🟡 Bottom-Left In Progress, 🔴 Bottom-Right Blocked) around the centered ship helm without project-specific details

- **Scenario: Fleet Global refreshes telemetry on press**
  - [x] **Given** Fleet is running
  - [x] **When** the user presses the Fleet Global button
  - [x] **Then** the plugin polls Fleet telemetry and refreshes all active keys on the device without tracking or cycling any single project

- **Scenario: Fixed Project action switches workspace and toggles mode on press**
  - [x] **Given** a Fleet Project action configured with a target `project_id` and currently in `Status` mode
  - [x] **When** the user presses the Fleet Project button
  - [x] **Then** Fleet receives `POST /api/projects/switch` to switch the active workspace to that project, and the button toggles to `Issues` mode

- **Scenario: Fleet Project displays bead issues breakdown in 4 corners**
  - [x] **Given** a Fleet Project action in `Issues` display mode
  - [x] **When** its button face is rendered
  - [x] **Then** the button displays the project's bead counts in the four corners (🟢 Top-Left Closed, 🔵 Top-Right Open, 🟡 Bottom-Left In Progress, 🔴 Bottom-Right Blocked) with the center visual and state color glow

- **Scenario: Launch Fleet when closed**
  - [x] **Given** Fleet is closed and `http://127.0.0.1:3000` is offline
  - [x] **When** the user presses any Fleet action key
  - [x] **Then** the plugin launches `fleet-desktop` and transitions the buttons to live state once ready

- **Scenario: Accurate 4-state color rendering**
  - [x] **Given** projects in Clean, Ready, InProgress, and Blocked states
  - [x] **When** their button images are generated
  - [x] **Then** Clean renders Emerald Green (`#10b981`), Ready renders Sky Blue (`#38bdf8`), InProgress renders Amber (`#f59e0b`), and Blocked renders Crimson Red (`#ef4444`)

- **Scenario: Custom project icon displayed when available**
  - [x] **Given** a target project containing an icon file (e.g. `tdrace/assets/icons/icon.svg` or `quant-trade/frontend/public/icon.svg`)
  - [x] **When** its button image is generated
  - [x] **Then** the button renders the project's custom icon in the center instead of the Fleet helm

- **Scenario: Clean status pill with state vector glyph**
  - [x] **Given** a Fleet Project action in `Status` mode in any of the 4 states
  - [x] **When** its button is displayed
  - [x] **Then** it renders a compact status pill with a crisp vector symbol (`✓`, `●`, `⚡`, `!`) in the matching state color rather than illegible text labels

- **Scenario: Configurable telemetry polling frequency**
  - [x] **Given** a Fleet Global action instance
  - [x] **When** the user configures the refresh frequency (e.g. 15 seconds) in the Property Inspector
  - [x] **Then** the plugin dynamically updates its background polling interval to that duration and persists the setting

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to `actions/fleet-monitor/`.*
- `[x]` `Cargo.toml` → Independent Rust crate configuration.
- `[x]` `src/main.rs` → OpenAction client initialization and event dispatch.
- `[x]` `src/state.rs` → Shared cooperative state store and 4-state classification.
- `[x]` `src/client.rs` → Fleet HTTP client, SSE subscriber, and desktop app launcher.
- `[x]` `src/icon.rs` → Dynamic SVG data-URI generation (Fleet ship helm, auras, quad metrics).
- `[x]` `pi/` → Svelte property inspector with linked/fixed toggle and live project dropdown.
- `[x]` `plugin/io.github.mario.fleetmonitor.sdPlugin/` → Manifest and assets.
- `[x]` `scripts/package.sh`, `README.md` → Build and packaging scripts.

### Repository Files
- `[x]` `specs/004_fleet_monitor.md` → Feature specification.
- `[x]` `specs/index.md` → Spec 004 registration.
- `[x]` `specs/constitution/ROADMAP.md` → Fleet Monitor living milestone.
- `[x]` `README.md` → Action table update.

### Verification Assertions
- `src/state.rs` references `specs/004_fleet_monitor.md` in its header comment.
- `cargo test` validates 4-state classification, cycling logic, and SVG rendering.
