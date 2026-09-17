---
type: Feature Spec
template: feature
title: "Hue Control"
description: "A native OpenAction plugin that controls Philips Hue lights over the Bridge local API — Switch, Color, Temperature, Brightness, and Scene (with fixed and cycling modes)."
status: implemented
created: 2026-09-16
generated: { by: agent/vulcan, at: 2026-09-16T18:35:00Z }
verified: { by: agent/vulcan, at: 2026-09-16T21:20:00Z }
---

# Feature Spec: Hue Control 🌈

A native OpenAction plugin for OpenDeck that controls Philips Hue lights and rooms through the Hue Bridge **local API** (`http://<bridge-ip>/api/<username>`). It provides a streamlined action set — Switch (On/Off), Color (fixed & cycling), Temperature (fixed & cycling), Brightness (fixed & cycling), and Scene (fixed & cycling) — built on the OpenAction crate and our own Svelte property inspector, ensuring every action's configuration UI actually renders and functions under OpenDeck.

The problem it solves: Mario's Hue Play lights and room groups are unreachable from the stream deck beyond a bare on/off toggle. This action makes the full Hue feature set available from the pad, with a property inspector that lists real bridges, lights, groups, and scenes discovered from the Bridge itself.

---

## 🗺️ User Flow & Interface Design

### Property Inspector (Svelte) — shared by all actions
- **Bridge section**: a `<select>` of paired bridges plus an "Add bridge…" entry. Discovery lists bridges returned by `https://discovery.meethue.com`; a manual IP field is offered as fallback.
- **Pairing flow**: after choosing a bridge, the inspector shows a "Press the bridge link button, then Pair" control. Pairing sends `POST /api` with `{"devicetype":"open-actions#hue-control"}`; on success the returned username is persisted to **global settings** and reused by every action (one credential per bridge).
- **Target section**: a `<select>` grouped into **Groups** then **Lights**, populated from the selected bridge. Groups are listed first because scenes apply to groups.
- **Action-specific controls**, revealed once a target is chosen:
  - *Switch (On/Off)* — none beyond target.
  - *Color* — a **Mode** selector (Fixed / Cycle). Fixed shows an `<input type="color">`; Cycle shows a list of color pickers with `+` / `−` buttons (2–10 colors).
  - *Temperature* — a **Mode** selector (Fixed / Cycle). Fixed shows a warmth range slider (1–100); Cycle shows a list of temperature steps with `+` / `−` buttons (2–10 steps). Both offer a "scale ticks" selector when bound to an encoder dial.
  - *Brightness* — a **Mode** selector (Fixed / Cycle). Fixed shows a brightness percentage slider (1–100); Cycle shows a list of percentage steps with `+` / `−` buttons (2–10 steps). Both offer a "scale ticks" selector when bound to an encoder dial.
  - *Scene* — a **Mode** selector (Fixed / Cycle). Fixed shows a `<select>` of scenes for the group; Cycle shows an ordered sequence of scene dropdowns with `+` / `−` buttons (2–10 scenes).
- The inspector requests bridge/target/scene data from the plugin via `sendToPlugin`; the plugin replies with `sendToPropertyInspector`, so all network access happens in Rust (the PI webview performs no cross-origin fetches).

### Button (device face)
- Each action renders an SVG data-URI image (matching the sibling actions' convention of generating SVG in Rust and base64-encoding it).
- *Switch (On/Off)*: luminous glowing bulb with ambient aura and green indicator when on, dimmed graphite bulb when off.
- *Brightness*: radiant sun glyph with an integrated circular halo dial gauge (0–100%) and percentage readout below.
- *Temperature*: thermometer glyph with dynamic mercury height and Kelvin-matched color glow, with Kelvin readout below.
- *Color*: artist palette glyph featuring a luminous active color gem with ambient glow, and hex readout below.
- *Scene*: ambient violet/fuchsia sparkles constellation with scene label below.
- Errors (bridge unreachable, unknown target, unauthorized username) show `showAlert` and a descriptive on-button label.

### Encoder / dial
- *Brightness* and *Temperature* declare `Controllers: ["Keypad", "Encoder"]`. `dial_rotate` adjusts the value by `ticks × scale`; `dial_down`/`touch_tap` toggles power.

### State management
- Global settings hold the bridge credentials (shared across instances). Per-instance settings hold the selected bridge id, target, and action parameters.
- A short-lived bridge cache (lights/groups/scenes) is refreshed when the property inspector opens and after each successful state change, so the PI lists current names.

---

## ⚙️ Backend Models & API Endpoints

The action performs HTTP calls against the Hue Bridge on the local network only. No cloud service is contacted except `https://discovery.meethue.com` for optional bridge discovery.

### 1. Bridge API (Philips Hue v1 local API, verified against BSB002 / API 1.78.0)

| Purpose | Method & path |
| :--- | :--- |
| Discover bridge IPs | `GET https://discovery.meethue.com` → `[{"id":"001788fffe7a9abf","internalipaddress":"192.168.1.74"}]` |
| Verify bridge reachable | `GET http://<ip>/api/config` → object containing `bridgeid` |
| Pair (create username) | `POST http://<ip>/api` body `{"devicetype":"open-actions#hue-control"}` → `[{"success":{"username":"<user>"}}]` (link button must be pressed first; otherwise `[{"error":{"type":101,...}}]`) |
| List lights | `GET http://<ip>/api/<user>/lights` |
| List groups | `GET http://<ip>/api/<user>/groups` |
| List scenes | `GET http://<ip>/api/<user>/scenes` |
| Set light state | `PUT http://<ip>/api/<user>/lights/<id>/state` |
| Set group state | `PUT http://<ip>/api/<user>/groups/<id>/action` |
| Apply scene | `PUT http://<ip>/api/<user>/groups/<group>/action` body `{"scene":"<scene-id>"}` |

State payloads used: `{"on":bool}`, `{"bri":1..254}`, `{"xy":[x,y]}`, `{"ct":153..500}`, `{"scene":"<id>"}`.

### 2. Persisted settings

Global settings (one entry per paired bridge, shared by all instances):
```json
{
  "bridges": {
    "001788fffe7a9abf": { "ip": "192.168.1.74", "username": "<bridge-username>" }
  }
}
```

Per-action settings (one object per instance):
```json
{
  "bridge": "001788fffe7a9abf",
  "target": "g-81",
  "mode": "fixed",
  "color": "#ff0000",
  "colors": ["#ff0000", "#00ff00", "#0000ff"],
  "brightness": 100,
  "brightnesses": [25, 50, 75, 100],
  "scale_ticks": 1,
  "temperature": 50,
  "temperatures": [20, 50, 80],
  "brightness_rel": 10,
  "scene": "<scene-id>",
  "scenes": ["<scene-id-1>", "<scene-id-2>"]
}
```

Target encoding: `g-<group-id>` selects a group; `l-<light-id>` selects an individual light.

### 3. Dependencies
- `reqwest` (json, rustls-tls) + `serde` / `serde_json` for bridge HTTP and JSON — both already approved in `TECH_STACK.md`.
- The local API is plain HTTP; `rustls-tls` remains enabled for the HTTPS discovery endpoint.

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- **No cloud credentials.** The only secret is the Hue Bridge username, which grants control of lights on the local network. It is stored in plugin global settings (the same store the Elgato plugin uses) and never logged or echoed to the property inspector in full.
- **Pairing requires physical access**: the Hue link button must be pressed, so a plugin cannot silently mint a username.
- **Local network only for control.** Bridge control requests are issued against a user-supplied bridge IP over HTTP; discovery is the sole outbound internet call and is optional.
- **Failure is surfaced, never silent**: an unreachable bridge, an unauthorized username (Hue error type 1), or an unknown target triggers `showAlert` plus an explanatory image rather than a false success.
- No RBAC or subscription gating: this is a local, single-user utility plugin.

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Plugin tests: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt --check`
- Property inspector build: `cd pi && npm run build`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Pair a bridge from the property inspector**
  - [ ] **Given** the Hue Bridge is on the local network and its link button has just been pressed
  - [ ] **When** the user selects the discovered bridge and clicks "Pair" in the property inspector
  - [ ] **Then** the plugin stores `{ip, username}` in global settings and the inspector lists the bridge's groups and lights

- **Scenario: Pairing waits for the link button instead of failing**
  - [x] **Given** a reachable Hue Bridge with no pending link-button press
  - [x] **When** the user clicks "Pair" in the property inspector
  - [x] **Then** the plugin reports the press as *pending* (Hue error 101) rather than a failure, the inspector shows "press the round link button on top of the bridge" with a countdown, and re-sends the request every 3 s for up to 120 s
  - [ ] **And** once the link button is pressed, the plugin stores `{ip, username}` in global settings and the inspector lists the bridge's groups and lights

- **Scenario: Toggle a group on and off**
  - [x] **Given** a Switch action configured to a reachable group
  - [x] **When** the user presses the button
  - [x] **Then** the group's lights change state and the button image reflects the new on/off state

- **Scenario: Color cycle advances one step per press**
  - [x] **Given** a Color action in cycle mode configured to a group with colors `[red, green, blue]`
  - [x] **When** the user presses the button three times
  - [x] **Then** the group is set to red, then green, then blue, and the fourth press wraps back to red

- **Scenario: Temperature cycle advances one step per press**
  - [x] **Given** a Temperature action in cycle mode configured to a group with temperatures `[20, 50, 80]`
  - [x] **When** the user presses the button three times
  - [x] **Then** the group is set to temperature 20, then 50, then 80, and the fourth press wraps back to 20

- **Scenario: Brightness sets an absolute level**
  - [x] **Given** a Brightness action in fixed mode configured to 40%
  - [x] **When** the user presses the button
  - [x] **Then** the target is turned on and set to brightness 102/254 (`40% × 2.54`)

- **Scenario: Brightness cycle advances one step per press**
  - [x] **Given** a Brightness action in cycle mode configured to levels `[25, 50, 75, 100]`
  - [x] **When** the user presses the button four times
  - [x] **Then** the target cycles through each level and wraps back to 25%

- **Scenario: Scene applies to its group**
  - [x] **Given** a Scene action in fixed mode configured to a group and one of its scenes
  - [x] **When** the user presses the button
  - [x] **Then** the plugin applies that scene to the group via `groups/<id>/action`

- **Scenario: Scene cycle advances one step per press**
  - [x] **Given** a Scene action in cycle mode configured to a group and scene sequence `[sceneA, sceneB]`
  - [x] **When** the user presses the button twice
  - [x] **Then** sceneA is applied, then sceneB, and the third press wraps back to sceneA

- **Scenario: Unreachable bridge surfaces an alert**
  - [x] **Given** an action whose configured bridge IP is unreachable
  - [x] **When** the user presses the button
  - [x] **Then** the button shows an alert and an error label, and no state change is reported as successful

- **Scenario: Unknown target is rejected**
  - [x] **Given** an action whose configured target no longer exists on the bridge
  - [x] **When** the user presses the button
  - [x] **Then** the plugin shows an alert instead of sending a request to a non-existent light or group

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to the action directory `actions/hue-control/`.*
- `[x]` `src/main.rs` → registers the consolidated actions and dispatches key/dial events.
- `[x]` `src/bridge.rs` → Hue Bridge client: discovery, pairing, target listing, and state changes over the local API.
- `[x]` `src/settings.rs` → global (bridge credentials) and per-action settings types, plus target encoding.
- `[x]` `src/icon.rs` → SVG data-URI rendering for each action's button state.
- `[x]` `pi/` → Svelte property inspector (bridge pairing, target picker, action-specific controls).
- `[x]` `plugin/io.github.mario.huecontrol.sdPlugin/` → manifest, plugin icon, and per-action assets.
- `[x]` `scripts/package.sh`, `README.md` → build/install instructions.

### Modified repository files
- `[x]` `specs/constitution/ROADMAP.md` → add the Hue Control milestone.
- `[x]` `specs/index.md` → register spec 003.
- `[x]` `README.md` → add Hue Control to the actions table.

### Verification Assertions
- `src/bridge.rs` references `specs/003_hue_control.md` in its header comment.
- `cargo test` covers target encoding/decoding, percent↔bri conversion, hex→xy conversion, cycle stepping, and SVG icon rendering.
- Protocol-level verification (18 checks) drives the packaged binary against both a mock bridge and the live bridge at `192.168.1.74`.
