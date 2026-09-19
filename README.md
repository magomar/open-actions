# open-actions

A monorepo of custom [OpenAction](https://github.com/OpenActionAPI) plugins ("actions") for OpenDeck devices and compatible control surfaces (Elgato Stream Deck, Tacto, and similar). Each action is a self-contained plugin — a Rust backend plus a Svelte property inspector — that installs cleanly and reports honest, up-to-date data on a device button.

## Actions

| Action | Directory | Description |
| :--- | :--- | :--- |
| **OpenCode Usage** | [`actions/opencode-usage/`](actions/opencode-usage/) | Surface OpenCode Go 5-hour, weekly, or monthly usage on a button. |
| **Handy Transcribe** | [`actions/handy-transcribe/`](actions/handy-transcribe/) | Toggle [Handy](https://handy.computer) dictation from a button via its remote-control CLI, with an idle/recording icon state. |
| **Hue Control** | [`actions/hue-control/`](actions/hue-control/) | Control Philips Hue lights and rooms over the Bridge local API: switch, color, brightness, temperature, and scenes (fixed and cycling modes). |
| **Fleet Monitor** | [`actions/fleet-monitor/`](actions/fleet-monitor/) | Monitor multi-workspace Keel fleets, 4-state project health, cooperative cycling, and independent project views on device keys. |

## Repository Layout

```
open-actions/
├── actions/                # one directory per action (self-contained OpenAction plugin)
│   └── <action-name>/      # Rust crate + Svelte property inspector + plugin bundle
├── specs/                  # numbered feature specs + the project constitution
├── BACKLOG.md              # un-started ideas, promoted to specs/ on selection
└── AGENTS.md               # agent working instructions
```

## Building an action

Each action builds independently. From its directory:

```sh
./scripts/package.sh
```

Then copy the resulting `plugin/*.sdPlugin/` bundle into OpenDeck's `plugins` directory and restart OpenDeck. See each action's own `README.md` for specifics.

## Governance

This repository follows the Keel Spec-Driven Development (SDD) methodology. The constitution lives in [`specs/constitution/`](specs/constitution/) — see `MISSION.md` for the development laws and `TECH_STACK.md` for the approved toolchain. Feature work is driven by numbered specs under `specs/`.
