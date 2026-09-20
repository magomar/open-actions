---
type: Feature Spec
template: feature
title: "Antigravity Usage"
description: "A device action that surfaces Google Antigravity model quota and credit telemetry on a button."
status: approved
created: 2026-09-20
generated: { by: agent/antigravity, at: 2026-09-20T08:40:55Z }
---

# Feature Spec: Antigravity Usage 🌟

An OpenAction plugin for OpenDeck and compatible devices (Elgato Stream Deck, Tacto) that displays Google Antigravity compute quota and credit telemetry on a button. The action connects to Antigravity's local Language Server on localhost, querying `POST /exa.language_server_pb.LanguageServerService/GetUserStatus` with CSRF authentication. It surfaces model pool quotas (Gemini Flash/Pro shared pool, Claude 4.6 Sonnet/Opus premium pool), prompt credits, and flow credits. It renders live percentages and reset countdowns on dynamic SVG tiles and supports fixed display or automatic/manual cycling.

---

## 🗺️ User Flow & Interface Design

### Button (device face)
- Each button instance displays a single metric at a time:
  - Header label (`Gemini`, `Claude`, `Credits`, `Flow`).
  - Hero value: percentage (`59%`) or count (`500`).
  - Progress bar along the bottom indicating remaining capacity (color-coded: blue > 25%, amber 10-25%, red < 10%).
  - Subtitle: reset time countdown (`1h 14m`) or status (`Ready`).
- If Antigravity is closed or unreachable, renders an "Offline" badge.
- If an error occurs, renders an "Error" indicator with device alert feedback.

### Display Modes (property-inspector setting)
1. **Fixed metric** — the button always displays one configured metric (`gemini_quota`, `claude_quota`, `prompt_credits`, `flow_credits`).
2. **Cycling (manual)** — pressing the button advances to the next metric and triggers an immediate telemetry refresh.
3. **Cycling (periodic)** — automatically advances to the next metric on a configurable interval (default 300 seconds, minimum 60 seconds).

### Property Inspector (Svelte)
- Connection mode: **Auto-detect** (default, reads `/proc` on Linux to extract port and CSRF token) with optional **Manual override** (specifying custom port and CSRF token).
- Select display mode: Fixed metric, Cycle on press, or Cycle periodically.
- For fixed mode, select target metric (`gemini_quota`, `claude_quota`, `prompt_credits`, `flow_credits`).
- For cycling mode, configure interval in minutes.
- Value preference: Display capacity as **Remaining %** (default) or **Used %**.

### Data Refresh & Caching
- Telemetry is cached across button instances with a 15-second TTL to avoid saturating the local language server.
- Key press triggers a fresh fetch bypassing the cache.

---

## ⚙️ Backend Models & API Endpoints

The action queries the local Antigravity Language Server on `127.0.0.1` via HTTPS with `Connect-Protocol-Version: 1` and `X-Codeium-Csrf-Token`.

### Metrics
| Key | Source | Unit | Label | Description |
| :--- | :--- | :--- | :--- | :--- |
| `gemini_quota` | LanguageServer `clientModelConfigs` | % + reset time | `Gemini` | Shared quota pool for Gemini 3.8 Flash, 3.7 Flash, 3.1 Pro |
| `claude_quota` | LanguageServer `clientModelConfigs` | % + reset time | `Claude` | Premium quota pool for Claude 4.6 Sonnet & Opus |
| `prompt_credits` | LanguageServer `planStatus` | count / % | `Credits` | Available prompt credits out of monthly quota |
| `flow_credits` | LanguageServer `planStatus` | count | `Flow` | Available flow credits |

### Language Server Request & Response Contract
- **Endpoint**: `POST https://127.0.0.1:<port>/exa.language_server_pb.LanguageServerService/GetUserStatus`
- **Headers**:
  - `Content-Type: application/json`
  - `Connect-Protocol-Version: 1`
  - `X-Codeium-Csrf-Token: <csrf_token>`
- **Request Body**:
  ```json
  {
    "metadata": {
      "ideName": "antigravity",
      "extensionName": "antigravity",
      "locale": "en"
    }
  }
  ```
- **Response Schema** (extract):
  ```json
  {
    "userStatus": {
      "planStatus": {
        "planInfo": {
          "monthlyPromptCredits": 50000,
          "monthlyFlowCredits": 150000
        },
        "availablePromptCredits": 500,
        "availableFlowCredits": 100
      },
      "cascadeModelConfigData": {
        "clientModelConfigs": [
          {
            "label": "Gemini 3.8 Flash (High)",
            "quotaInfo": {
              "remainingFraction": 0.59487,
              "resetTime": "2026-09-20T11:46:16Z"
            }
          }
        ]
      }
    }
  }
  ```

### Action Settings Schema (persisted per button)
```json
{
  "display_mode": "fixed | cycle_manual | cycle_periodic",
  "fixed_metric": "gemini_quota | claude_quota | prompt_credits | flow_credits",
  "cycle_interval_seconds": 300,
  "display_fraction_as": "remaining | used",
  "override_port": null,
  "override_csrf_token": ""
}
```

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- The plugin only communicates with localhost (`127.0.0.1`) over TLS.
- CSRF token extracted from the process table or configured by user is strictly passed via HTTP header and never leaked or rendered on button faces.
- If Antigravity is not running or the token is invalid, the plugin displays "Offline" or "Error" rather than stale data.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Command to run plugin tests: `cargo test`
- Command to build Property Inspector: `npm run build`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Zero-config auto-discovery retrieves live quota**
  - [ ] **Given** Antigravity is running with its Language Server on localhost
  - [ ] **When** the Antigravity Usage action is added to a button
  - [ ] **Then** the plugin automatically detects the port and CSRF token, queries `GetUserStatus`, and renders Gemini quota on the button tile

- **Scenario: Fixed metric displays configured pool**
  - [ ] **Given** an action configured with `display_mode = fixed` and `fixed_metric = claude_quota`
  - [ ] **When** telemetry is fetched
  - [ ] **Then** the button shows the Claude pool label, remaining percentage, progress bar, and reset countdown

- **Scenario: Manual cycling advances and refreshes on press**
  - [ ] **Given** an action configured with `display_mode = cycle_manual`
  - [ ] **When** the user presses the button
  - [ ] **Then** the displayed metric cycles to the next item (Gemini -> Claude -> Credits -> Flow -> Gemini) and performs a fresh telemetry poll

- **Scenario: Periodic cycling advances automatically**
  - [ ] **Given** an action configured with `display_mode = cycle_periodic` and `cycle_interval_seconds = 300`
  - [ ] **When** 5 minutes elapse
  - [ ] **Then** the button advances to the next metric in sequence without user intervention

- **Scenario: Offline handling when Antigravity is closed**
  - [ ] **Given** Antigravity is not running on the system
  - [ ] **When** the plugin attempts discovery or refresh
  - [ ] **Then** the button displays an "Offline" badge and gracefully retries on subsequent polls without crashing

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths relative to `actions/antigravity-usage/`.*
- [ ] `src/discovery.rs` -> Process finder and port/CSRF detection for Antigravity language server.
- [ ] `src/metrics.rs` -> Data models, quota computation, and SVG button rendering.
- [ ] `src/main.rs` -> OpenAction client registration, settings handling, cycling loops.
- [ ] `pi/src/App.svelte` -> Svelte Property Inspector for display modes, metrics, and overrides.
- [ ] `plugin/io.github.mario.antigravityusage.sdPlugin/manifest.json` -> Action definition and manifest.
- [ ] `scripts/package.sh` -> Release packaging script.

### Verification Assertions
- `src/metrics.rs` references `specs/005_antigravity_usage.md` in its header comment.
- `cargo test` covers model response parsing, percentage calculations, reset time formatting, and cycle advancement.
