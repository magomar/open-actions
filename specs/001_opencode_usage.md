---
type: Feature Spec
template: feature
title: "OpenCode Usage"
description: "A device action that surfaces OpenCode Go usage telemetry on a button."
status: implemented
created: 2026-09-14
---

# Feature Spec: OpenCode Usage 🌟

An OpenAction plugin for OpenDeck devices that displays OpenCode Go telemetry on a button. The action reads the three documented Go usage metrics — last 5 hours, weekly, and monthly — from `GET https://opencode.ai/zen/go/v1/usage` using the user's existing OpenCode API key. It renders one metric at a time and supports cycling through them on button press or a timer.

OpenCode Zen balance is deliberately deferred. Zen has no key-authenticated balance endpoint today; tracking is blocked on [anomalyco/opencode#10448](https://github.com/anomalyco/opencode/issues/10448). The plugin will not scrape the authenticated dashboard or claim to show an unavailable balance.

---

## 🗺️ User Flow & Interface Design

### Button (device face)
- Each button instance shows a single metric at a time: a short label (`5h`, `Wk`, or `Mo`) and its percentage rendered as the button title.
- The displayed metric is driven by the action's configured display mode (see below).

### Display modes (property-inspector setting)
1. **Fixed metric** — the button always shows one configured metric (`5h`, `weekly`, or `monthly`).
2. **Cycling (manual)** — a single button cycles to the next metric on each button press.
3. **Cycling (periodic)** — a single button advances to the next metric automatically on a configurable interval (default 5 minutes).

### Property Inspector (Svelte)
- Configure the OpenCode Go API key.
- Select display mode and, for cycling, the interval.
- Cycling always includes all three Go usage metrics.

### Data refresh
- Metrics are refreshed on button press and on the periodic interval; the button re-renders its title when the value changes.

---

## ⚙️ Backend Models & API Endpoints

The action reads telemetry from OpenCode's documented Go endpoint using a Bearer API key. No local persistence is required.

### Metrics
| Key | Source | Unit | Label |
| :--- | :--- | :--- | :--- |
| `go_5h` | opencode go | usage | last 5 hours |
| `go_weekly` | opencode go | usage | weekly |
| `go_monthly` | opencode go | usage | monthly |

The endpoint returns `usage.rolling`, `usage.weekly`, and `usage.monthly`; each window exposes at least `percent`, `resetsAt`, and `status`.

### Deferred metric
- `zen_credits` is out of scope until OpenCode publishes a key-authenticated balance endpoint. See [anomalyco/opencode#10448](https://github.com/anomalyco/opencode/issues/10448).

### Global settings (plugin-wide shared state)
```json
{
  "api_key": "OpenCode Go API key"
}
```

### Action settings (persisted per instance)
```json
{
  "api_key": "optional instance override / legacy fallback",
  "display_mode": "fixed | cycle_manual | cycle_periodic",
  "fixed_metric": "go_5h | go_weekly | go_monthly",
  "cycle_interval_seconds": 300
}
```

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- The API key is stored in plugin-wide global settings (or optional instance settings override); it is never logged or rendered on the button.
- Failure to authenticate (invalid/expired key) surfaces a visible error state on the button (e.g. `!`) rather than silently showing stale or empty data.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Command to run plugin tests: `cargo test`
- Property-inspector tests (if any): `npm test`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Shared API key configures multiple actions without re-entry**
  - [x] **Given** multiple OpenCode Usage buttons on the device and an API key configured once in global settings
  - [x] **When** each button appears or refreshes
  - [x] **Then** every button accesses the shared API key and renders its respective metric without requiring individual key configuration

- **Scenario: Fixed metric displays the configured value**
  - [x] **Given** a configured action with `display_mode = fixed` and `fixed_metric = go_monthly`
  - [x] **When** the action is added to a button and the API key is valid
  - [x] **Then** the button shows the current monthly Go usage percentage, updated on refresh

- **Scenario: Manual cycling advances on press**
  - [x] **Given** a configured action with `display_mode = cycle_manual`
  - [x] **When** the user presses the button
  - [x] **Then** the button advances to the next metric in `cycle_metrics` (wrapping from the last back to the first)

- **Scenario: Periodic cycling advances on interval**
  - [x] **Given** a configured action with `display_mode = cycle_periodic` and `cycle_interval_seconds = 300`
  - [x] **When** 5 minutes elapse
  - [x] **Then** the button advances to the next metric without user input

- **Scenario: Invalid API key shows an error state**
  - [x] **Given** an action configured with an invalid or expired API key
  - [x] **When** the action attempts to refresh
  - [x] **Then** the button shows an error indicator and does not display stale or fabricated values

- **Scenario: Zen balance is deferred safely**
  - [x] **Given** OpenCode Zen has no key-authenticated balance endpoint
  - [x] **When** the user configures this version of the action
  - [x] **Then** no Zen-credit display option is offered and the action performs no dashboard scraping

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to the action directory `actions/opencode-usage/`.*
- [x] `src/` -> Rust OpenAction client: action registration, event handling, metric fetching.
- [x] `src/metrics.rs` -> Go metric model, fetch + display-mode/cycle logic.
- [x] `pi/` -> Svelte property inspector for API key, display mode, and interval configuration.

### Verification Assertions
- `src/metrics.rs` references `specs/001_opencode_usage.md` in its header comment.
- `cargo test` covers metric parsing and cycle advancement (wrap-around).
