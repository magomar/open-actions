---
type: Feature Spec
template: feature
title: "GPU Screen Recorder"
description: "An OpenAction plugin for OpenDeck with dedicated Record Screen and Record Window actions driving GPU Screen Recorder (gsr-ui-cli) with live /proc recording status and elapsed timer icons."
status: implemented
receipt: "docs/receipts/spec-007-receipt.md"
created: 2026-10-23
generated: { by: agent/antigravity, at: 2026-10-23T19:00:41Z }
verified: { by: human:mario, at: 2026-10-26T08:37:00Z, hash: "185f124ecef6" }
---

# Feature Spec: GPU Screen Recorder 🎥

An OpenAction plugin (`actions/gpu-screen-recorder/`) for OpenDeck and compatible control surfaces that integrates with **GPU Screen Recorder** (`gpu-screen-recorder` and its ShadowPlay-style daemon `gsr-ui`). The plugin provides two dedicated hardware button actions — **Record Screen** (whole monitor) and **Record Window** (single window via XDG Desktop Portal on Wayland or XComposite window capture on X11) — driving `gsr-ui` through `gsr-ui-cli` while polling `/proc` for live `gpu-screen-recorder` process telemetry so each button's icon and `MM:SS` elapsed duration timer stay accurately synchronized with real recording state.

---

## 🗺️ User Flow & Interface Design

### 1. Actions & Hardware Key Interactions

The plugin registers three Keypad actions under the `GPU Screen Recorder` category:

1. **Record Screen (`io.github.mario.gpuscreenrecorder.record-screen`)**:
   - **Press (`key_up`)**: Starts or stops recording the primary monitor (`gpu-screen-recorder -w <monitor>`) using the user's preferences in `~/.config/gpu-screen-recorder/config_ui`.
2. **Record Window (`io.github.mario.gpuscreenrecorder.record-window`)**:
   - **Press (`key_up`)**: Automatically starts or stops recording the currently focused window (queries `org.gnome.Shell.Introspect.GetWindows` and seeds `org.freedesktop.impl.portal.PermissionStore` with a restore token on Wayland, or uses `-w focused` on X11, without requiring an interactive window picker dialog).
3. **Toggle Background Service (`io.github.mario.gpuscreenrecorder.toggle-daemon`)**:
   - **Press (`key_up`)**: Starts `gsr-ui launch-daemon` in the background when no `gsr-ui` or `gpu-screen-recorder` process is running, or stops any active recording (`SIGINT`) and terminates all background `gsr-ui` and `gpu-screen-recorder` processes (`SIGTERM` -> `SIGKILL` fallback) when running.
   - **Live Status**: Polls `/proc` at 1 Hz to show an emerald `● ON / SERVICE` tile when the background process is active and a muted `OFF / SERVICE` tile when stopped.

### 2. Button Face (144×144 Dynamic SVG Tile Rendering)

Each button renders a 144×144 SVG data-URI tile reflecting live recording state:

- **Record Screen Tile**:
  - **Idle State**: Dark zinc card (`#18181b`) with a crisp widescreen monitor bezel and stand glyph in cool slate/teal (`#38bdf8` / `#94a3b8`) and an uppercase `SCREEN` footer label.
  - **Recording State (`CaptureMode::Screen`)**: Illuminated crimson border/glow (`#ef4444`), active monitor glyph with a red recording badge (`● REC`), and a bold `MM:SS` (or `H:MM:SS` at $\ge 60\text{ min}$) elapsed duration counter.
- **Record Window Tile**:
  - **Idle State**: Dark zinc card (`#18181b`) with an application window frame glyph (title bar + window controls) in cool violet/indigo (`#a78bfa` / `#94a3b8`) and an uppercase `WINDOW` footer label.
  - **Recording State (`CaptureMode::Window`)**: Illuminated crimson border/glow (`#ef4444`), active window glyph with a red recording badge (`● REC`), and a bold `MM:SS` (or `H:MM:SS`) elapsed duration counter.
- **Error / Unreachable Daemon**:
  - If `gsr-ui-cli` fails to spawn or exits non-zero (for example, when the `gsr-ui` background daemon is not running), the button triggers `instance.show_alert()` and renders its idle state.

```
   Idle (Record Screen)           Recording (Record Screen)
+-------------------------+     +-------------------------+
|                         |     |  ● REC                  |
|      +-----------+      |     |      +-----------+      |
|      |           |      |     |      |   01:24   |      |
|      +-----+-----+      |     |      +-----+-----+      |
|          --+--          |     |          --+--          |
|         SCREEN          |     |         SCREEN          |
+-------------------------+     +-------------------------+

   Idle (Record Window)           Recording (Record Window)
+-------------------------+     +-------------------------+
|                         |     |  ● REC                  |
|      +-----------+      |     |      +-----------+      |
|      | o o o     |      |     |      | o o o     |      |
|      |           |      |     |      |   00:42   |      |
|      +-----------+      |     |      +-----------+      |
|         WINDOW          |     |         WINDOW          |
+-------------------------+     +-------------------------+
```

### 3. Property Inspector (Svelte Frontend)

Both actions share a clean Svelte Property Inspector (`pi/`):
- **`gsr-ui-cli` Binary Path**: Text input field bound to `settings.gsr_ui_cli_path`. Leaving it empty uses `gsr-ui-cli` on `PATH`.
- **Daemon & Capture Status Readout**: On open (and via a "Check Status" refresh button), the PI sends `{"event": "getStatus"}` to the Rust plugin via `sendToPlugin`. The plugin checks `gsr-ui-cli status` and current `/proc` recording state, replying via `sendToPropertyInspector` with whether `gsr-ui` is running (`running` / `not running`) and whether a screen or window recording is currently active.

---

## ⚙️ Backend Models & API Endpoints

The plugin performs no network I/O; it interacts locally with `gsr-ui-cli` and the Linux `/proc` filesystem.

### 1. External CLI Commands (`gpu-screen-recorder`, `gsr-ui-cli`, & `gsr-ui`)

| Operation | Command | Expected Behavior |
| :--- | :--- | :--- |
| Check daemon health | `<gsr-ui-cli> status` | Prints `running` and exits `0` when `gsr-ui` is listening on its socket; prints `not running` and exits `1` otherwise. |
| Auto-start daemon | `<gsr-ui> launch-daemon` | Starts `gsr-ui` in background daemon mode (hidden until `Alt+Z`) when `gsr-ui-cli status` reports not running, polling until ready (up to 3 s). |
| Toggle whole-screen recording | `<gpu-screen-recorder> -w <monitor> ... -o <file>` | Starts whole-screen recording using settings from `~/.config/gpu-screen-recorder/config_ui`, or stops an active recording via `SIGINT`. |
| Toggle focused-window recording | `<gpu-screen-recorder> -w portal -restore-portal-session yes -portal-session-token-filepath /tmp/gsr-open-actions-window-token ...` | Automatically captures the currently focused window on Wayland (via GNOME Shell Introspect + `PermissionStore` token) or X11 (`-w focused`), or stops an active recording via `SIGINT`. |

### 2. Live Recording Detection (`/proc` Inspection)

Rather than relying on an optimistic boolean that drifts when recordings are started/stopped via `Alt+Z`, global hotkeys, or a canceled Wayland portal dialog, the plugin inspects running `gpu-screen-recorder` processes in `/proc`:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Screen,
    Window,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveRecording {
    pub pid: u32,
    pub mode: CaptureMode,
    pub elapsed_secs: u64,
}
```

#### Classification Rules for `/proc/<pid>/cmdline`:
1. **Executable Match**: The argv0 basename is `gpu-screen-recorder`.
2. **Active Regular Recording Filter**:
   - Must contain `-w <source>` and `-o <output>`.
   - Must **not** contain `-r` (which denotes a background replay-buffer session), unless `-ro` is also active.
   - `<output>` must **not** be a live stream URL (`rtmp://`, `rtmps://`, `http://`, `https://`, `srt://`, `tcp://`, `udp://`).
3. **Capture Mode Classification (`CaptureMode`)**:
   - **`CaptureMode::Window`** when:
     - `-w` is `window`, `focused`, or a numeric X11 window ID, **or**
     - `-w` is `portal` and argv contains `-portal-session-token-filepath` pointing to a `gsr-ui-window-capture-token` path.
   - **`CaptureMode::Screen`** for all other regular recording targets (`-w screen`, `-w screen-direct`, `-w <monitor_name>` such as `HDMI-A-1` or `DP-1`, `-w region`, or `-w portal` without the window-capture token).
4. **Elapsed Duration Calculation**:
   - Computed from the `/proc/<pid>` directory modification timestamp (`std::fs::metadata("/proc/<pid>").modified()`) relative to `SystemTime::now()`, formatted as `MM:SS` under 1 hour or `H:MM:SS` at 1 hour and above.
5. **1 Hz Background Polling & Deduplicated Rendering**:
   - A background Tokio task ticks every 500 ms while at least one button instance is visible, scanning `/proc` and pushing `instance.set_image(...)` only to instances whose visual state (`Idle` vs `Recording`, `mode`, or `elapsed_secs`) has changed since the last render.

### 3. Persisted Action Settings Schema

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gsr_ui_cli_path: String,
}
```

Persisted JSON payload:
```json
{
  "gsr_ui_cli_path": ""
}
```

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- **Zero Network / Zero Secrets**: The plugin executes local commands and reads `/proc` metadata only; no credentials, tokens, or network requests are used.
- **Process Ownership & Safe Argument Handling**: `Command::new(effective_path)` passes subcommands (`toggle-record`, `toggle-record-window`, `status`) as discrete arguments without a shell (`sh -c`), preventing command injection from custom binary paths.
- **Honest Failure Reporting**: Any failure to invoke `gsr-ui-cli` or a non-zero exit code (such as `gsr-ui` not running) immediately triggers `instance.show_alert()` on the hardware key rather than displaying a false recording state.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Command to run plugin unit tests: `cargo test`
- Command to run linter: `cargo clippy -- -D warnings`
- Command to check formatting: `cargo fmt --check`
- Command to build Property Inspector: `cd pi && npm run build`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Record Screen button starts and stops whole-screen recording**
  - [x] **Given** `gsr-ui` is running in the background and no recording is active
  - [x] **When** the user presses the **Record Screen** button
  - [x] **Then** the plugin runs `gsr-ui-cli toggle-record`, `gpu-screen-recorder` starts capturing the monitor, and the **Record Screen** button switches to the crimson recording icon with a live `MM:SS` elapsed timer
  - [x] **And When** the user presses the **Record Screen** button a second time
  - [x] **Then** `gsr-ui-cli toggle-record` stops and saves the recording, and the button returns to the idle screen icon

- **Scenario: Record Window button starts and stops window recording**
  - [x] **Given** `gsr-ui` is running in the background and no recording is active
  - [x] **When** the user presses the **Record Window** button and selects a window in the portal/window picker
  - [x] **Then** the plugin runs `gsr-ui-cli toggle-record-window`, `gpu-screen-recorder` captures the selected window, and the **Record Window** button displays the crimson window recording icon with a live `MM:SS` elapsed timer
  - [x] **And When** the user presses the **Record Window** button again
  - [x] **Then** the window recording stops and saves, and the button returns to the idle window icon

- **Scenario: Canceling the Wayland desktop portal picker returns button to idle**
  - [x] **Given** the user presses the **Record Window** button on Wayland and the XDG Desktop Portal dialog opens
  - [x] **When** the user cancels/dismisses the portal dialog without selecting a window
  - [x] **Then** `gpu-screen-recorder` exits and the **Record Window** button automatically returns to the idle state

- **Scenario: External recording control via hotkey syncs button state**
  - [x] **Given** the **Record Screen** and **Record Window** buttons are visible on the device
  - [x] **When** the user starts or stops a recording externally via `gsr-ui` (`Alt+Z` overlay or global hotkey)
  - [x] **Then** the `/proc` watcher detects the `gpu-screen-recorder` process within 1 second and updates the corresponding button icon and `MM:SS` timer automatically

- **Scenario: Replay buffer process is not mistaken for an active recording**
  - [x] **Given** `gpu-screen-recorder` is running in replay-buffer mode (`-r 60`) without an active regular recording
  - [x] **When** the `/proc` watcher evaluates running processes
  - [x] **Then** both **Record Screen** and **Record Window** buttons remain in their `Idle` state

- **Scenario: Auto-launches gsr-ui daemon when not already running**
  - [x] **Given** `gsr-ui` is not currently running (`gsr-ui-cli status` reports `not running`)
  - [x] **When** the user presses **Record Screen** or **Record Window**
  - [x] **Then** the plugin automatically spawns `gsr-ui launch-daemon`, waits until `gsr-ui-cli status` reports `running`, and completes the requested recording toggle

- **Scenario: Toggle Background Service button starts and kills background process**
  - [x] **Given** the **Toggle Background Service** button is visible on the device
  - [x] **When** the user presses the button while `gsr-ui` is not running
  - [x] **Then** `gsr-ui launch-daemon` starts in the background and the button icon switches to the emerald `● ON / SERVICE` tile
  - [x] **And When** the user presses the button again while `gsr-ui` (or `gpu-screen-recorder`) is running in the background
  - [x] **Then** any active recording is cleanly stopped (`SIGINT`), the background `gsr-ui` / `gpu-screen-recorder` processes are terminated, and the button icon switches to the muted `OFF / SERVICE` tile

- **Scenario: Missing binary surfaces an alert**
  - [x] **Given** `gsr_ui_cli_path` points to a non-existent binary or `gsr-ui` cannot be launched
  - [x] **When** the user presses either **Record Screen** or **Record Window**
  - [x] **Then** the button displays an alert (`show_alert`) and remains in the `Idle` state

- **Scenario: Empty binary path resolves to default gsr-ui-cli**
  - [x] **Given** `gsr_ui_cli_path` is empty or whitespace in action settings
  - [x] **When** the plugin resolves the executable path
  - [x] **Then** it defaults to `gsr-ui-cli` on `PATH`

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to `actions/gpu-screen-recorder/`.*
- `[x]` `Cargo.toml` -> Independent Rust crate manifest (`openaction`, `serde`, `serde_json`, `tokio`, `log`, `simplelog`).
- `[x]` `src/main.rs` -> OpenAction plugin entrypoint, registration of `RecordScreenAction` and `RecordWindowAction`, 1 Hz `/proc` watcher loop, and Property Inspector status messaging.
- `[x]` `src/gsr.rs` -> `gsr-ui-cli` path resolution, command execution (`toggle-record`, `toggle-record-window`, `status`), `/proc` cmdline parsing, `CaptureMode` classification, and elapsed duration formatting.
- `[x]` `src/icon.rs` -> Dynamic 144×144 SVG data-URI renderer for Screen and Window tiles (idle and active recording states with `MM:SS` / `H:MM:SS` timer readout).
- `[x]` `pi/` -> Svelte Property Inspector (`package.json`, `vite.config.ts`, `src/App.svelte`) for configuring `gsr_ui_cli_path` and viewing live `gsr-ui` daemon status.
- `[x]` `plugin/io.github.mario.gpuscreenrecorder.sdPlugin/manifest.json` -> OpenAction manifest declaring the `Record Screen`, `Record Window`, and `Toggle Background Service` actions.
- `[x]` `plugin/io.github.mario.gpuscreenrecorder.sdPlugin/icon.svg` -> Plugin category icon.
- `[x]` `plugin/io.github.mario.gpuscreenrecorder.sdPlugin/actions/` -> Static action list icons (`record-screen.svg`, `record-window.svg`, `toggle-daemon.svg`).
- `[x]` `scripts/package.sh`, `README.md` -> Build, packaging, and installation instructions.

### Modified Repository Files
- `[x]` `specs/constitution/ROADMAP.md` -> Add the GPU Screen Recorder milestone under Phase 2: Power Utilities.
- `[x]` `specs/index.md` -> Register `007_gpu_screen_recorder.md`.
- `[x]` `README.md` -> Add GPU Screen Recorder to the root actions table.

### Verification Assertions
- `src/gsr.rs` and `src/icon.rs` reference `specs/007_gpu_screen_recorder.md` in their header comments.
- `cargo test` covers binary path resolution, `gsr-ui-cli` command construction, `/proc` cmdline classification (screen vs. window vs. replay buffer vs. livestream), elapsed time formatting (`MM:SS` and `H:MM:SS`), and SVG tile rendering for both actions in idle and recording states.
