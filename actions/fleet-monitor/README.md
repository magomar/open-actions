# Fleet Monitor 🧭

An OpenAction plugin for OpenDeck devices (Stream Deck, Tacto, and compatible control surfaces) that monitors your **Fleet** (`~/workspace/agentic-dev/fleet/`) multi-workspace dashboard, live Keel repositories, and Beads task graph health.

## ✨ Features

- 🌐 **Fleet Global**: Displays 4-state project breakdown (`Clean`, `Ready`, `In Progress`, `Blocked`). Pressing cycles through registered projects and synchronizes Fleet's desktop view via `POST /api/projects/switch`.
- 🧭 **Fleet Project**: Displays real-time task metrics and glowing state clues for a repository:
  - **Linked Mode** *(default)*: Cooperates with `Fleet Global`, updating dynamically to follow the currently cycled project.
  - **Fixed Mode**: Pinned to a specific project (e.g. `tdrace`).
- 🚦 **4-State Project Health Model**:
  - 🟢 **Clean**: All tasks closed, zero backlog in flight (`open == 0 && in_progress == 0`).
  - 🔵 **Ready**: Open unblocked tasks waiting for agents or developers (`open > 0 && in_progress == 0 && blocked == 0`).
  - 🟡 **In Progress**: Autonomous agents or devs actively executing tasks (`in_progress > 0 && blocked == 0`).
  - 🔴 **Blocked**: Dependency blockers or diagnostic failures needing human triage (`blocked > 0 || health.status == "fail"`).
- ⚡ **Auto-Launch**: If Fleet is closed, buttons show a dimmed monochrome helm with a launch pip; pressing any button launches `fleet-desktop` (or `fleet --global`).

## 📦 Build & Packaging

Run the packaging script from the action directory:

```bash
./scripts/package.sh
```

This compiles the release Rust binary, builds the Svelte property inspector, and produces:
- `plugin/io.github.mario.fleetmonitor.sdPlugin/`
- `plugin/fleet-monitor-0.1.0.zip`

## 🔌 Installation

Copy the plugin bundle to your OpenDeck plugins folder and restart OpenDeck:

```bash
cp -r plugin/io.github.mario.fleetmonitor.sdPlugin ~/.config/opendeck/plugins/
```
