---
type: Product Roadmap
title: "Product Roadmap & Feature Backlog"
description: "Living roadmap tracking upcoming developmental milestones and technical debt."
status: active
tags: [constitution, roadmap, milestones, planning]
---

# Product Roadmap & Feature Backlog 🚀

## 📅 Living Milestones: [Phase 1]

Each milestone maps to one action directory under `actions/<name>/` and one numbered spec under `specs/`.

### Phase 1: Core Essentials (Priority: High)
- `[ ]` **[OpenCode Usage](../001_opencode_usage.md)** (`actions/opencode-usage/`): Display OpenCode Go 5-hour, weekly, or monthly usage on a device button — fixed metric or cycling (manual/periodic).

### Phase 2: Power Utilities (Priority: Medium)
- `[ ]` **[Handy Transcribe](../002_handy_transcribe.md)** (`actions/handy-transcribe/`): Toggle Handy dictation from a button via its remote-control CLI, with optimistic recording feedback on the icon.

### Phase 3: Growth & Engagement (Priority: Low)
- *(None scheduled.)*

---

## 🛠 Technical Debt & Maintenance

- **Zen balance**: OpenCode Go usage is available from `GET /zen/go/v1/usage`; Zen balance has no key-authenticated endpoint. Defer it until [anomalyco/opencode#10448](https://github.com/anomalyco/opencode/issues/10448) is fulfilled.

---

## 🗄️ Combined Feature Backlog

### 1. Additional opencode actions
- **Proposed exploration**: Run/stop opencode or surface opencode status as additional actions on the device — each landing in its own `actions/<name>/` directory.

### 2. Distribution
- **Proposed exploration**: Package and publish the plugin to the OpenAction Marketplace.

---

## ✅ Completed Milestones

- *(None yet.)*
