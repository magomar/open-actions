# GPU Screen Recorder (`actions/gpu-screen-recorder/`)

An OpenAction plugin for OpenDeck that controls [GPU Screen Recorder](https://git.dec05eba.com/gpu-screen-recorder/about/) (`gsr-ui`) via `gsr-ui-cli` and displays live recording status and elapsed `MM:SS` duration by inspecting `/proc`.

## Actions

- **Record Screen** (`io.github.mario.gpuscreenrecorder.record-screen`): Toggles whole-screen / monitor recording (`gsr-ui-cli toggle-record`).
- **Record Window** (`io.github.mario.gpuscreenrecorder.record-window`): Toggles window recording (`gsr-ui-cli toggle-record-window` — opens the XDG Desktop Portal window picker on Wayland, or selects a window on X11).

## Prerequisites

- `gpu-screen-recorder` and `gpu-screen-recorder-ui` (`gsr-ui` + `gsr-ui-cli`) installed. If `gsr-ui` is not already running, the plugin automatically launches `gsr-ui launch-daemon` in the background.

## Building & Packaging

```sh
cd pi && npm install
cd ..
./scripts/package.sh
```

Copy `plugin/io.github.mario.gpuscreenrecorder.sdPlugin` into OpenDeck's `plugins` directory (`~/.config/opendeck/plugins/`) or install the generated `plugin/gpu-screen-recorder-0.1.0.zip`.
