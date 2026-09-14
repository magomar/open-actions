---
type: Tech Stack
title: "Tech Stack & Styling Guidelines"
description: "Approved technical stack, framework runtimes, styling guidelines, and commands."
status: active
tags: [constitution, tech-stack, tooling]
---

# Tech Stack & Styling Guidelines 🛠️

> This stack applies to every action under `actions/<name>/`. Each action is an independent Rust crate with its own Svelte property inspector; commands below are run from the action's directory. See `MISSION.md` → *Repository Layout* for the overall structure.

## 💻 Tech Stack Specification

### Plugin (Backend)
| Technology | Version | Purpose / Description |
| :--- | :--- | :--- |
| **Rust** | latest stable | Main plugin runtime and action logic |
| **OpenAction crate** (`OpenActionAPI/rust`) | latest | Official OpenAction client library (WebSocket, cross-platform) |
| **reqwest + serde** | latest compatible | HTTPS client and JSON decoding for the OpenCode Go usage API |

### Property Inspector (Frontend)
| Technology | Version | Purpose / Description |
| :--- | :--- | :--- |
| **Svelte** | latest | Property inspector UI framework |
| **OpenAction Svelte library** (`OpenActionAPI/svelte-pi`) | latest | Official PI bindings and components |

### Storage
| Technology | Version | Purpose / Description |
| :--- | :--- | :--- |
| **OpenCode Go Usage API** | undocumented schema | `GET https://opencode.ai/zen/go/v1/usage`; provides the Go usage windows with Bearer authentication. |

---

## 🎨 Design & Styling Principles (Aesthetic Excellence)

- **Property Inspector**: Minimal, reactive Svelte UI that matches OpenDeck/OpenAction conventions. No dedicated design system yet — keep it plain and functional.
- **Button Feedback**: Prefer the device's native image/title rendering for the action state (usage figures, credit counts) over custom UI chrome.

---

## 🚀 Execution & Command Enforcements

### Plugin (Rust / cargo)
- Command to build: `cargo build`
- Command to run tests: `cargo test`
- Command to lint: `cargo clippy -- -D warnings`
- Command to format: `cargo fmt --check`

### Property Inspector (Svelte / npm or bun)
- Command to install: `npm install` (or `bun install`)
- Command to run development server: `npm run dev`
- Command to build: `npm run build`

---

## 🛡️ Coding & Schema Constraints

- **Type Safety**: Rely on the OpenAction crate's typed event contracts; avoid `unwrap()`/`expect()` on live event paths (prefer `Result`/error handling).
- **Validation Schemas**: Property-inspector input should be validated before being persisted to action settings.
- **Dependency Auditing**: Use the official OpenAction crates/library; do not add redundant third-party libraries when standard Rust/stdlib equivalents exist.
