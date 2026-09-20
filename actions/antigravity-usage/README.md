# Antigravity Usage

An OpenAction plugin for OpenDeck and compatible devices that surfaces Google Antigravity model quotas (Gemini shared pool, Claude/premium pool), prompt credits, flow credits, and rolling reset countdowns on a hardware key.

## Build

```sh
./scripts/package.sh
```

Copy `plugin/io.github.mario.antigravityusage.sdPlugin` into OpenDeck's `plugins` directory, then restart OpenDeck. Select the action in OpenDeck to configure its display mode: a single fixed metric, cycling on press, or cycling periodically on a timer.

By default, the plugin automatically discovers the local Antigravity Language Server port and CSRF token from the active process table on Linux, requiring zero initial configuration.
