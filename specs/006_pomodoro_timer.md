---
type: Feature Spec
template: feature
title: "Pomodoro Timer"
description: "A native OpenAction plugin for OpenDeck that provides a Pomodoro technique workflow inspired by the Pomotroid aesthetic with custom SVG ring gauges, round tracking, and sound alerts."
status: implemented
created: 2026-09-21
generated: { by: agent/antigravity, at: 2026-09-21T10:35:56Z }
verified: { by: agent/antigravity, at: 2026-09-21T11:00:00Z }
---

# Feature Spec: Pomodoro Timer ⏱️

A native OpenAction plugin for OpenDeck and compatible control surfaces (Elgato Stream Deck, Tacto) that brings the **Pomodoro Technique** directly to the hardware pad, taking visual design and workflow inspiration from the minimalist **Pomotroid** desktop timer.

The plugin provides a unified timer core driving dynamic SVG circular dial gauges on button tiles, tracking Focus, Short Break, and Long Break intervals, multi-round progress cycles (`1/4` → `4/4`), customizable color themes, audio chimes on phase transitions, and dedicated companion actions (Play/Pause, Skip, Reset) with encoder dial support for Stream Deck + and Tacto hardware.

---

## 🗺️ User Flow & Interface Design

### 1. Button Face (Device Tile Rendering)
Each button instance dynamically renders a 144×144 SVG tile with Pomotroid-inspired visual hierarchy:
- **Circular Dial Ring**:
  - High-precision SVG circular track (`r=52`, `stroke-width=8`) with background path.
  - Active progress arc calculated from remaining fraction: `stroke-dasharray="326.7"` and dynamic `stroke-dashoffset`.
- **Phase Color Coding (Default Pomotroid Palette)**:
  - 🔴 **Focus**: Coral Red (`#ef5350` / `#f87171`)
  - 🟢 **Short Break**: Mint Green (`#4ade80` / `#22c55e`)
  - 🔵 **Long Break**: Teal / Cyan (`#22d3ee` / `#06b6d4`)
  - ⚪ **Paused / Idle State**: Muted Slate (`#64748b`) with subtle pulse or dimmed background track.
- **Hero Typography**:
  - Crisp `MM:SS` countdown timer (e.g., `25:00`, `3:25`, `0:42`) centered inside the dial ring.
- **Phase Subtitle**:
  - Uppercase phase label: `FOCUS`, `SHORT BREAK`, `LONG BREAK`.
- **Footer Telemetry**:
  - **Round Counter**: Fractional round tracker `1/4`, `2/4`, `3/4`, `4/4` positioned in the lower-left or bottom area.
  - **Playback Glyph**: Subtle Play (`▶`), Pause (`❚❚`), or Complete (`✔`) badge indicating state.

```
+---------------------------+
|                           |
|       ( 25:00 )           |  <-- Circular Ring Dial
|         FOCUS             |  <-- Phase Label
|                           |
| 1/4                  [▶]  |  <-- Round Count & Playback State
+---------------------------+
```

### 2. Hardware & Key Interactions

#### Main Timer Action (`io.github.mario.pomodorotimer.timer`)
- **Short Press (`key_up`)**:
  - When *Idle* or *Paused*: Starts / resumes countdown.
  - When *Running*: Pauses countdown.
- **Long Press (>600ms)**:
  - Skips current phase immediately to the next phase (Focus → Short Break, Short Break → Focus, or Focus → Long Break on final round).
- **Double Press (within 400ms)**:
  - Resets the current phase to its full initial duration without advancing round.

#### Companion Actions (Multi-Key Layouts)
- **Skip Action (`io.github.mario.pomodorotimer.skip`)**:
  - Single tap triggers an immediate phase advance (`>|` glyph).
- **Reset Action (`io.github.mario.pomodorotimer.reset`)**:
  - Single tap resets the active phase; long press resets the entire session back to Round 1.

#### Encoders & Touch Strips (Stream Deck + / Tacto)
- **`dial_rotate`**:
  - Rotate clockwise: Adds 1 minute to current timer (or scrubs forward).
  - Rotate counter-clockwise: Subtracts 1 minute from current timer (minimum 0:00).
- **`dial_down` / Touch Tap**:
  - Toggles Start / Pause.
- **Touch Strip (LCD Strip Display)**:
  - Renders a horizontal linear progress bar with phase badge, live digital clock readout, round indicators, and interactive touch controls.

---

### 3. Property Inspector (Svelte Frontend)

The Svelte Property Inspector exposes a clean, grouped configuration UI matching Pomotroid's settings screen:

#### Section 1: Timer Durations (Sliders & Number Inputs)
- **Focus Duration**: Range 1–90 min (default: 25 min) with coral slider accent.
- **Short Break Duration**: Range 1–30 min (default: 5 min) with mint slider accent.
- **Long Break Duration**: Range 1–60 min (default: 15 min) with teal slider accent.
- **Rounds Before Long Break**: Range 1–12 (default: 4 rounds) with round counter.
- **Reset Defaults Button**: One-click restore to standard 25 / 5 / 15 / 4 parameters.

#### Section 2: Automation & Behavior
- **Auto-start Breaks**: Checkbox toggle (default: `false`). When true, short/long breaks begin immediately when focus time expires.
- **Auto-start Focus**: Checkbox toggle (default: `false`). When true, focus sessions begin immediately when break time expires.

#### Section 3: Audio & Notifications
- **Sound Alerts**: Checkbox toggle (default: `true`). Plays pleasant notification chimes on phase completion.
- **Volume Slider**: 0% to 100% audio gain.
- **Desktop Notifications**: Checkbox toggle (default: `false`). Sends OS desktop notification when a phase ends.

#### Section 4: Themes & Color Palettes
- **Preset Selector**:
  - *Pomotroid Classic* (Coral `#ef5350` / Mint `#4ade80` / Cyan `#22d3ee`)
  - *Gruvbox* (Orange `#fe8019` / Green `#b8bb26` / Aqua `#8ec07c`)
  - *Nord* (Frost Red `#bf616a` / Aurora Green `#a3be8c` / Frost Cyan `#88c0d0`)
  - *Catppuccin Mocha* (Flamingo `#f2cdcd` / Green `#a6e3a1` / Sky `#89dceb`)
  - *Tokyo Night* (Red `#f7768e` / Green `#9ece6a` / Cyan `#7dcfff`)
  - *Custom*: Custom hex pickers for Focus, Short Break, and Long Break.

---

## ⚙️ Backend Models & API Endpoints

The plugin maintains a singleton Tokio-backed timer core that broadcasts updates at 1 Hz when active to all instantiated buttons and touch strips.

### 1. State Machine Enums & Structs

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PomodoroState {
    pub phase: Phase,
    pub status: TimerStatus,
    pub current_round: u32,
    pub total_rounds: u32,
    pub remaining_seconds: u32,
    pub total_phase_seconds: u32,
}

impl PomodoroState {
    pub fn progress_fraction(&self) -> f64 {
        if self.total_phase_seconds == 0 {
            return 0.0;
        }
        (self.remaining_seconds as f64) / (self.total_phase_seconds as f64)
    }

    pub fn format_time(&self) -> String {
        let mins = self.remaining_seconds / 60;
        let secs = self.remaining_seconds % 60;
        format!("{:02}:{:02}", mins, secs)
    }
}
```

### 2. Action Settings Schema (Persisted per Button / Global)

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PomodoroSettings {
    pub focus_duration_mins: u32,
    pub short_break_duration_mins: u32,
    pub long_break_duration_mins: u32,
    pub rounds: u32,
    pub auto_start_breaks: bool,
    pub auto_start_focus: bool,
    pub sound_enabled: bool,
    pub volume: u8,
    pub notification_enabled: bool,
    pub theme: String,
    pub custom_focus_color: Option<String>,
    pub custom_short_break_color: Option<String>,
    pub custom_long_break_color: Option<String>,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        Self {
            focus_duration_mins: 25,
            short_break_duration_mins: 5,
            long_break_duration_mins: 15,
            rounds: 4,
            auto_start_breaks: false,
            auto_start_focus: false,
            sound_enabled: true,
            volume: 80,
            notification_enabled: false,
            theme: "classic".to_string(),
            custom_focus_color: None,
            custom_short_break_color: None,
            custom_long_break_color: None,
        }
    }
}
```

### 3. State Transition Matrix

| Current Phase | Event / Trigger | Next Phase | Next Round | Next Status |
| :--- | :--- | :--- | :--- | :--- |
| `Focus` (Round `r < N`) | Timer reaches 0:00 | `ShortBreak` | `r` | `Running` (if auto-start) / `Idle` |
| `Focus` (Round `r == N`) | Timer reaches 0:00 | `LongBreak` | `r` | `Running` (if auto-start) / `Idle` |
| `ShortBreak` | Timer reaches 0:00 | `Focus` | `r + 1` | `Running` (if auto-start) / `Idle` |
| `LongBreak` | Timer reaches 0:00 | `Focus` | `1` | `Running` (if auto-start) / `Idle` |
| Any | Long Press / Skip | Next in cycle | Adjusted | `Paused` |
| Any | Double Press / Reset | Current phase | Unchanged | `Idle` (Full duration) |

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- **Local Execution Only**: The plugin operates completely locally with no external cloud communication or sensitive credential storage.
- **Resource-Efficient Ticking**: The 1 Hz Tokio timer loop sleeps when all timers are in `Idle` or `Paused` state, waking only during active `Running` countdowns.
- **Audio Safety**: Embedded chime audio assets are decoded once into memory and clamped to maximum safety limits to avoid audio glitching or excessive memory usage.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Command to run plugin test suite: `cargo test`
- Command to run linter: `cargo clippy -- -D warnings`
- Command to verify code formatting: `cargo fmt --check`
- Command to build Property Inspector: `cd pi && npm run build`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Start initial focus session on key press**
  - [x] **Given** a Pomodoro Timer action in `Idle` state with 25:00 focus duration
  - [x] **When** the user presses the action key
  - [x] **Then** the timer transitions to `Running` status, countdown ticks down every second, and the button renders a coral progress ring

- **Scenario: Pause and resume running session**
  - [x] **Given** a Pomodoro Timer action currently in `Running` status at `21:40`
  - [x] **When** the user presses the action key
  - [x] **Then** the timer pauses at `21:40` with status `Paused` and displays a play indicator
  - [x] **And When** the user presses the key again
  - [x] **Then** the timer resumes countdown from `21:40`

- **Scenario: Transition from Focus to Short Break on intermediate round**
  - [x] **Given** a running Focus session on Round 1 of 4 at `0:01`
  - [x] **When** 1 second elapses
  - [x] **Then** the phase transitions to `ShortBreak` with duration `05:00`, the ring color switches to mint green, and the round indicator remains `1/4`

- **Scenario: Transition from Focus to Long Break after final round**
  - [x] **Given** a running Focus session on Round 4 of 4 at `0:01`
  - [x] **When** 1 second elapses
  - [x] **Then** the phase transitions to `LongBreak` with duration `15:00`, the ring color switches to cyan, and the round indicator shows `4/4`

- **Scenario: Long Break completion resets cycle to Round 1**
  - [x] **Given** a running Long Break session on Round 4 at `0:01`
  - [x] **When** 1 second elapses
  - [x] **Then** the phase transitions back to `Focus` with duration `25:00` and round counter resets to `1/4`

- **Scenario: Manual skip advances phase immediately on long press**
  - [x] **Given** a running Focus session on Round 2 at `18:30`
  - [x] **When** the user performs a long press (>600ms) on the action button
  - [x] **Then** the timer immediately skips to `ShortBreak` at `05:00` on Round 2

- **Scenario: Reset current phase on double press**
  - [x] **Given** a running Focus session at `12:15`
  - [x] **When** the user double-presses the action key within 400ms
  - [x] **Then** the remaining time resets to `25:00` in `Idle` state without advancing the round counter

- **Scenario: Auto-start breaks begins countdown immediately**
  - [x] **Given** `auto_start_breaks` is set to `true` in settings
  - [x] **When** a Focus session completes
  - [x] **Then** the Short Break or Long Break starts counting down immediately with status `Running` without requiring manual key press

- **Scenario: Encoder dial adjusts remaining duration**
  - [x] **Given** a Pomodoro Timer action bound to a Stream Deck + encoder dial
  - [x] **When** the user rotates the dial clockwise by 2 ticks
  - [x] **Then** the remaining duration increases by 2 minutes and updates the display immediately

- **Scenario: Property Inspector updates settings reactively**
  - [x] **Given** the Property Inspector is open
  - [x] **When** the user changes the Focus duration slider to 30 minutes and clicks "Reset Defaults"
  - [x] **Then** the values update immediately and synchronize with the action button

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to `actions/pomodoro-timer/`.*
- `[x]` `src/main.rs` -> Action registration, event routing (key up, long press, dial events), and Tokio timer loop.
- `[x]` `src/state.rs` -> Pomodoro state machine, phase transitions, and round progression logic.
- `[x]` `src/render.rs` -> Dynamic SVG 144×144 tile renderer (Pomotroid dial ring, typography, badges, color palettes).
- `[x]` `src/settings.rs` -> Action and global settings serialization, defaults, and theme definitions.
- `[x]` `src/sound.rs` -> Cross-platform audio notification chime player.
- `[x]` `pi/src/App.svelte` -> Svelte Property Inspector with Pomotroid sliders, theme picker, and automation switches.
- `[x]` `plugin/io.github.mario.pomodorotimer.sdPlugin/manifest.json` -> OpenAction manifest declaring timer, skip, and reset actions.
- `[x]` `scripts/package.sh` -> Build and packaging release script.
- `[x]` `README.md` -> Action overview, controls guide, and installation instructions.

### Modified Repository Files
- `[x]` `specs/constitution/ROADMAP.md` -> Register the Pomodoro Timer milestone.
- `[x]` `specs/index.md` -> Register `006_pomodoro_timer.md`.
- `[x]` `README.md` -> Add Pomodoro Timer to actions directory summary.

### Verification Assertions
- `src/state.rs` references `specs/006_pomodoro_timer.md` in its header comments.
- `cargo test` covers all state machine transitions, timer countdowns, round wrap-arounds, fraction math, and SVG generation.
