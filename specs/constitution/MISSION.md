---
type: Project Constitution
title: "Project Mission & Agent Constitution"
description: "Constitutional governance, coding laws, and architectural boundaries."
status: active
tags: [constitution, governance, principles]
---

# Project Mission & Agent Constitution 📜

## 🎯 Primary Mission

open-actions is a **monorepo of custom OpenAction plugins ("actions")** for OpenDeck devices and compatible control surfaces (Elgato Stream Deck, Tacto, and similar). Each action is a self-contained plugin — a Rust backend plus a Svelte property inspector — that installs cleanly and reports honest, up-to-date data on a device button.

The first action, **OpenCode Usage** (`actions/opencode-usage/`), surfaces live opencode usage directly on a button, giving developers at-a-glance telemetry on their coding-agent spend without leaving their workflow. Further actions are added as sibling directories under `actions/`.

**Target audience:** Developers who use opencode and stream-controller hardware, and want custom actions beyond what off-the-shelf plugins provide.

**Standard of polish:** Small, reliable, single-purpose actions that install cleanly and report honest, up-to-date data on the button.

---

## 📜 Core Laws of Development

All developers and AI coding agents must unconditionally adhere to the following principles:

### 1. Hybrid Pragmatic Keel Methodology (Spec-Driven Development)
- **Major Features & Architectural Changes**: Must be plan-driven. A formal specification (or modification to an existing spec) must be drafted in `/specs/` and approved before code changes.
- **Minor Fixes & Local Adjustments**: Bug fixes, style tweaks, and test cases can be updated directly in code without requiring a formal spec update.
- **Strict Major Boundaries**: A formal specification MUST be created or modified if a change:
  1. Adds, renames, or removes a database field, collection, or schema.
  2. Modifies an API signature (query parameters, path parameters, or request/response payloads).
  3. Adds or removes an external dependency or library.
  4. Aligns with a milestone checklist item in `roadmap.md`.
- **File Naming & Casing Standard**: To establish a clear visual hierarchy and maintain cross-platform filesystem compatibility:
  - **UPPERCASE** is strictly used for high-priority constitutional and policy sheets (e.g., `MISSION.md`, `TECH_STACK.md`, `ROADMAP.md`, `BACKLOG.md`).
  - **lowercase_snake_case** is strictly used for active, numbered feature or technical specifications (e.g., `001_opencode_usage.md`).
- **Implementation Plans**: Always draft a detailed, approved implementation plan before making complex code changes.

### 2. Simplicity First (KISS/DRY)
- Write the minimum amount of code required to resolve the problem. No speculative abstractions or unrequested features.
- Avoid introducing redundant external packages or library dependencies when standard tools exist.

### 3. Surgical Changes
- Touch only what is required to satisfy the goal. Match surrounding code conventions, and do not make unrelated refactors or cleanups.
- Remove any imports, variables, or functions that are made unused by your modifications.

### 4. Goal-Driven Verification
- Enforce the Plan-Build-Verify loop: know your testing and validation criteria before writing any code.
- Ensure all quality checks (linting, tests) compile and pass with zero warnings prior to merge.

---

## 🗂️ Repository Layout

```
open-actions/
├── actions/                # one directory per action (self-contained OpenAction plugin)
│   └── <action-name>/      # e.g. opencode-usage
│       ├── src/            # Rust action logic (backend)
│       ├── pi/             # Svelte property inspector (frontend)
│       ├── plugin/         # plugin bundle (*.sdPlugin) + manifest + assets
│       ├── scripts/        # packaging scripts
│       └── README.md       # per-action build/install instructions
├── specs/                  # numbered feature specs + this constitution
├── BACKLOG.md              # un-started ideas, promoted to specs/ on selection
└── AGENTS.md               # agent working instructions
```

### Adding a new action
1. Create `actions/<action-name>/` mirroring the structure of an existing action (Rust crate + `pi/` + `plugin/` + `scripts/`).
2. Draft a numbered spec at `specs/NNN_<action-name>.md` and register it in `specs/index.md`.
3. Record the milestone in `specs/constitution/ROADMAP.md` (promote from `BACKLOG.md` if listed there).
4. Each action is an independent Rust crate with its own `Cargo.toml` and `Cargo.lock` — no shared Cargo workspace.
5. Its compiled binary and built property inspector are gitignored; only `manifest.json`, `icon.svg`, and `actions/` assets are tracked inside `plugin/*.sdPlugin/`.

---

## 🤝 Codebase Architecture & Ownership

- **Actions Directory** (`actions/<name>/`): One self-contained plugin per action — the single source of truth for that action's logic, device communication, and data fetching.
- **Plugin Layer (Rust)**: Each action's OpenAction client, built on the official OpenAction crate (`OpenActionAPI/rust`).
- **Property Inspector Layer (Svelte)**: Each action's configuration UI, using the official OpenAction Svelte library.
- **Specs Directory** (`specs/`): Location of active design, database, and feature contracts. Constitutional documents live in `specs/constitution/`.
