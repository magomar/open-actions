---
type: Feature Spec
template: feature
title: "Handy Transcribe"
description: "A device action that toggles Handy transcription on a button."
status: draft
created: 2026-09-15
---

# Feature Spec: Handy Transcribe 🖐️

An OpenAction plugin for OpenDeck devices that toggles [Handy](https://handy.computer) dictation from a button. The action drives Handy through its remote-control CLI (`handy --toggle-transcription`), which relays to the running instance via Handy's single-instance plugin. Pressing the button starts or stops recording; the button icon reflects the toggle state.

---

## 🗺️ User Flow & Interface Design

### Button (device face)
- The button shows a pink hand when idle.
- On press it toggles Handy transcription: the icon flips to a complementary teal hand with a red "recording" dot while recording, and back to pink on the next press.
- If the Handy binary cannot be launched, the button shows an alert and stays idle.

### Property Inspector (Svelte)
- Configure the Handy binary path. Leave empty to auto-detect: the macOS app bundle (`/Applications/Handy.app/Contents/MacOS/Handy`) or `handy` on PATH elsewhere.

### Trigger semantics
- The button is a toggle: one press starts recording, the next press stops it, matching Handy's `--toggle-transcription` CLI. Handy's own configured keyboard shortcut (e.g. Shift+Ctrl+Space) remains an independent, equivalent trigger.

---

## ⚙️ Backend Models & API Endpoints

The action performs no network I/O and no API calls. It drives Handy through one external command:

```
handy --toggle-transcription
```

The binary path is resolved as: configured value, if non-empty; otherwise a per-OS default (macOS app bundle path, or `handy` on PATH).

### Action settings (persisted)
```json
{
  "handy_binary_path": "handy"
}
```

---

## 🛡️ Security & Role-Based Access Controls (RBAC)

- No secrets are stored or logged; only a binary path is persisted.
- Failure to launch the Handy binary surfaces an alert on the button rather than silently reporting a successful toggle.
- The recording indicator is **optimistic**: it reflects the button's own toggle, not Handy's true state. Handy exposes no live status API, so the icon can drift if transcription is started or stopped elsewhere (hotkey, `Escape`).

---

## 🧪 Verification & Acceptance Criteria

### Automated Tests
- Command to run plugin tests: `cargo test`
- Property-inspector tests (if any): `npm test`

### Manual Acceptance Criteria (Pseudo-Gherkin)

- **Scenario: Button toggles Handy transcription**
  - [ ] **Given** Handy is installed and running with `handy` on PATH
  - [ ] **When** the user presses the button
  - [ ] **Then** Handy starts recording and the icon shows the teal recording state

- **Scenario: Second press stops recording**
  - [ ] **Given** the button is in the recording state
  - [ ] **When** the user presses the button again
  - [ ] **Then** Handy stops recording and the icon returns to the idle pink state

- **Scenario: Missing binary surfaces an error**
  - [ ] **Given** the configured Handy binary path does not exist
  - [ ] **When** the user presses the button
  - [ ] **Then** the button shows an alert and remains in the idle state

- **Scenario: Empty path auto-detects the binary**
  - [ ] **Given** `handy_binary_path` is empty
  - [ ] **When** the action resolves the binary
  - [ ] **Then** it uses the macOS app bundle path on macOS, or `handy` on PATH elsewhere

---

## 🔗 Traceability & Codebase Mapping

### Created/Modified Files
*Paths below are relative to the action directory `actions/handy-transcribe/`.*
- `[ ]` `src/main.rs` -> Rust OpenAction client: action registration, event handling, optimistic toggle state.
- `[ ]` `src/handy.rs` -> binary-path resolution, toggle command construction, idle/recording icon rendering.
- `[ ]` `pi/` -> Svelte property inspector for the Handy binary path.
- `[ ]` `plugin/` -> manifest, pink-hand icon, and action asset.
- `[ ]` `scripts/package.sh`, `README.md` -> build/install instructions.

### Verification Assertions
- `src/handy.rs` references `specs/002_handy_transcribe.md` in its header comment.
- `cargo test` covers binary-path resolution, toggle command arguments, and idle/recording icon rendering.
